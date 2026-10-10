/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE STOP OF ONE TURN (gHashTag/t27 specs/queen/turn_stop.t27, epic
 * trios#1712). A killed turn used to be abandoned: its model call ran on to
 * its 120 s timeout and its commands to theirs, holding the row and the key
 * lane while its worker came back and took another row.
 *
 * Now a turn carries an AbortSignal. The runtime aborts it when the turn's
 * pid dies (the turn timer or any `exit`): actors.t27 stop_signal's
 * X_SHUTDOWN. Everything the turn waits on obeys it. A model call's fetch
 * rejects at once, and a command's process group is killed. After
 * TURN_STOP_GRACE_MS the runtime escalates as the card's `escalation` says.
 *
 * The signal reaches the commands without a new parameter on every git
 * helper: the runtime enters each turn with `withTurnSignal`, and the helpers
 * that spawn (`run` in queen-dispatch.ts, `defaultExec` in
 * queen-criteria-run.ts) ask `stepMayStart` and `stepSignal` here. A region of
 * code marks what its steps are with `asStep`. A step that writes the shared
 * checkout runs to its end once started, and a finalizer always runs. The
 * card decides each case (step_on_abort); this file only carries the answer.
 * Outside a turn there is no signal, and nothing changes.
 */

import { AsyncLocalStorage } from 'node:async_hooks'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { STEP_CUT } from './queen-turn-stop-card.gen'

export const TURN_STOP_CARD = 'queen/turn_stop.wasm'

const card = () => loadCardWasm(TURN_STOP_CARD)

/** step_on_abort: STEP_CUT or STEP_RUN for one step of a stopped turn. */
export const stepOnAbort = (
  started: boolean,
  writesShared: boolean,
  finalizer: boolean,
): number =>
  card().call(
    'step_on_abort',
    flag(started),
    flag(writesShared),
    flag(finalizer),
  )

/** escalation: ESC_NOTHING, ESC_KILL_GROUP or ESC_ABANDON past the grace. */
export const escalation = (killStops: boolean, workEnded: boolean): number =>
  card().call('escalation', flag(killStops), flag(workEnded))

/** in_flight: what a concurrency bound counts, held turns included. */
export const inFlight = (running: number, heldAfterKill: number): number =>
  card().call('in_flight', u32(running), u32(heldAfterKill)) >>> 0

interface Step {
  signal: AbortSignal
  writesShared: boolean
  finalizer: boolean
  /**
   * Inside an `asStep` region that has started: its commands are parts of one
   * step under way. Outside one, each command is a step of its own.
   */
  started: boolean
}

const steps = new AsyncLocalStorage<Step | undefined>()

/** Run `fn` as one turn: what it starts obeys `signal`. */
export function withTurnSignal<T>(
  signal: AbortSignal | undefined,
  fn: () => T,
): T {
  return steps.run(
    signal
      ? { signal, writesShared: false, finalizer: false, started: false }
      : undefined,
    fn,
  )
}

/**
 * withTurnSignal for every turn of one pid, its context made once: the
 * runtime runs each turn through it, so a turn costs no new allocation.
 */
export function turnRunner(signal: AbortSignal): <T>(fn: () => T) => T {
  const step: Step = {
    signal,
    writesShared: false,
    finalizer: false,
    started: false,
  }
  return (fn) => steps.run(step, fn)
}

/**
 * Run `fn` as one step of a kind: one that writes what other turns share, or
 * a finalizer. The card decides at its start whether it starts at all in a
 * stopped turn (it throws the abort if not), and every command inside it is
 * then a part of a step under way. Outside a turn this only runs `fn`.
 */
export function asStep<T>(
  kind: { writesShared?: boolean; finalizer?: boolean },
  fn: () => T,
): T {
  const outer = steps.getStore()
  if (!outer) return fn()
  const writesShared = kind.writesShared ?? outer.writesShared
  const finalizer = kind.finalizer ?? outer.finalizer
  // a region inside a step already under way is a part of that step
  if (
    outer.signal.aborted &&
    stepOnAbort(outer.started, writesShared, finalizer) === STEP_CUT
  )
    throw abortError(outer.signal)
  return steps.run(
    { signal: outer.signal, writesShared, finalizer, started: true },
    fn,
  )
}

/** The signal of the turn this code runs in, if any. */
export function turnSignal(): AbortSignal | undefined {
  return steps.getStore()?.signal
}

/**
 * Whether a command may start now. False only inside a turn that was
 * stopped, for a kind of step the card cuts before it starts.
 */
export function stepMayStart(): boolean {
  const step = steps.getStore()
  if (!step?.signal.aborted) return true
  return (
    stepOnAbort(step.started, step.writesShared, step.finalizer) !== STEP_CUT
  )
}

/**
 * The signal that cuts a started step of this kind, or undefined when the
 * card lets such a step run to its end (or there is no turn).
 */
export function stepSignal(): AbortSignal | undefined {
  const step = steps.getStore()
  if (!step) return undefined
  return stepOnAbort(true, step.writesShared, step.finalizer) === STEP_CUT
    ? step.signal
    : undefined
}

/** What an aborted wait throws: the reason the turn was stopped. */
export function abortError(signal: AbortSignal): Error {
  const reason: unknown = signal.reason
  return reason instanceof Error
    ? reason
    : new DOMException(String(reason ?? 'the turn was stopped'), 'AbortError')
}
