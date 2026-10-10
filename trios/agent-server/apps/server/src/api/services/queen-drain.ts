/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A DRAIN BOUNDED BY WHAT IS IN FLIGHT (gHashTag/t27 specs/queen/drain.t27,
 * trios#1729 item 2). Behind TRIOS_QUEEN_DRAIN=bounded; off, the drain is the
 * runner's own (queen-runner.ts drainBeeRunner) exactly as before.
 *
 * WHAT THE DRAIN WAS. On SIGTERM the server waited while any bee its runner
 * carried still ran, up to TRIOS_BEE_RUNNER_DRAIN_SECONDS (1800), salvaged
 * the bees still running one after another, and exited about 2 s later.
 * Rounds, reviews and waits ran on through that wait and were cut by the
 * exit. A bee cut at the deadline kept its row claimed until its task lease
 * lapsed, and the salvages had the 30 s between 1800 and Railway's 1830 to
 * finish before SIGKILL, however many bees there were.
 *
 * WHAT THE CARD DECIDES, per piece of work: DONE, LEAVE (a row keeps it),
 * STOP (a review: stopped at the exit through turn_stop.t27), HOLD (a bee,
 * until it ends) or HAND_BACK (a bee still running at the cap). Also the cap
 * itself (drain_cap_seconds), whether a source starts anything new
 * (takes_new_work) and when the exit may go (exit_step). This file holds the
 * clock and the loop; every threshold and every choice is the card's.
 */

import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import {
  DV_HAND_BACK,
  DV_HOLD,
  DV_STOP,
  EXIT_WAIT,
  HAND_BACK_SECONDS,
  WK_BEE,
  WK_REVIEW,
} from './queen-drain-card.gen'

export const DRAIN_CARD = 'queen/drain.wasm'

const card = () => loadCardWasm(DRAIN_CARD)

/** drain_verdict: DV_DONE, DV_LEAVE, DV_STOP, DV_HOLD or DV_HAND_BACK. */
export const drainVerdict = (
  kind: number,
  ended: boolean,
  waitedSeconds: number,
  capSeconds: number,
): number =>
  card().call(
    'drain_verdict',
    kind,
    flag(ended),
    u32(waitedSeconds),
    u32(capSeconds),
  )

/** drain_cap_seconds: the runner's drain, inside the platform's SIGKILL. */
export const drainCapSeconds = (
  runnerDrainSeconds: number,
  platformSeconds: number,
): number =>
  card().call(
    'drain_cap_seconds',
    u32(runnerDrainSeconds),
    u32(platformSeconds),
  ) >>> 0

/** takes_new_work: whether a source starts anything new while draining. */
export const takesNewWork = (
  startsABeeHere: boolean,
  holding: number,
): boolean =>
  card().call('takes_new_work', flag(startsABeeHere), u32(holding)) !== 0

/** exit_step: EXIT_NOW or EXIT_WAIT. */
export const exitStep = (
  holding: number,
  stopping: number,
  sinceStopMs: number,
): number =>
  card().call('exit_step', u32(holding), u32(stopping), u32(sinceStopMs))

/** bee_free_after_seconds: when another process may take a stopped bee's issue. */
export const beeFreeAfterSeconds = (
  handedBack: boolean,
  sinceRenewal: number,
): number =>
  card().call('bee_free_after_seconds', flag(handedBack), u32(sinceRenewal)) >>>
  0

/** TRIOS_QUEEN_DRAIN=bounded turns the card's drain on; anything else is the runner's. */
export function drainBoundedEnabled(
  env: Record<string, string | undefined> = process.env,
): boolean {
  return (env.TRIOS_QUEEN_DRAIN ?? '').trim().toLowerCase() === 'bounded'
}

/**
 * The platform's drain, RAILWAY_DEPLOYMENT_DRAINING_SECONDS, which Railway
 * hands the process as a service variable. Unset or unreadable is 0, which
 * the card reads as "unknown" and leaves the runner's drain alone.
 */
export function platformDrainSeconds(
  env: Record<string, string | undefined> = process.env,
): number {
  const raw = env.RAILWAY_DEPLOYMENT_DRAINING_SECONDS?.trim() ?? ''
  if (!/^\d+$/.test(raw)) return 0
  return Math.min(Number(raw), 4294967295)
}

export interface DrainBee {
  issue: number
  conversationId: string
}

export interface DrainDeps {
  now: () => number
  sleep: (ms: number) => Promise<void>
  /** drain_cap_seconds, already asked. */
  capSeconds: number
  /** A round here would start a bee in this process (bees do not run elsewhere). */
  roundsStartBeesHere: boolean
  /** The runner takes no new order. */
  stopClaims: () => void
  /** The bees this process still runs. A bee that ended is no longer listed. */
  bees: () => DrainBee[]
  /** Salvage, store, end the row, abort: a bee still running at the cap. */
  handBack: (bee: DrainBee) => Promise<void>
  /** Reviews still running in this process. */
  reviews: () => number
  /** Rounds, reviewer and waits stop (queen-tick.ts handover). */
  stopRest: () => Promise<void>
  /** How often the loop looks again while something holds. */
  pollMs?: number
}

export interface DrainResult {
  /** From SIGTERM to the moment the process may exit. */
  tookMs: number
  /** The bees that held the drain at its start. */
  heldAtStart: number
  /** The bees handed back at the cap, by issue. */
  handedBack: number[]
  /** The reviews running when the rest was stopped. */
  reviewsStopped: number
}

/**
 * The drain, as the card says. Resolves when the process may exit.
 *
 * The bees are asked about every pollMs (2 s, the runner's own drain step):
 * a bee that ends leaves the list, and at the cap the card answers
 * DV_HAND_BACK for every bee still there. The hand-backs run side by side
 * under one HAND_BACK_SECONDS deadline (the card's), because the platform's
 * SIGKILL does not wait for them one by one. While anything holds, rounds and
 * reviews go on (takes_new_work): the next container cannot start before this
 * one ends, so stopping them would only idle the swarm.
 */
export async function drainBounded(deps: DrainDeps): Promise<DrainResult> {
  const pollMs = deps.pollMs ?? 2000
  const start = deps.now()
  const waited = () => Math.floor((deps.now() - start) / 1000)
  const handedBack: number[] = []
  const handing = new Set<number>()
  let restStopped = false
  let reviewsStopped = 0

  // a new bee here would hold the drain for up to its hour
  if (!takesNewWork(true, 1)) deps.stopClaims()

  const holdingNow = (): { holding: number; due: DrainBee[] } => {
    let holding = 0
    const due: DrainBee[] = []
    for (const bee of deps.bees()) {
      if (handing.has(bee.issue)) continue
      const verdict = drainVerdict(WK_BEE, false, waited(), deps.capSeconds)
      if (verdict === DV_HOLD) holding++
      else if (verdict === DV_HAND_BACK) due.push(bee)
    }
    return { holding, due }
  }

  const stopRest = async (holding: number): Promise<void> => {
    if (restStopped) return
    if (takesNewWork(deps.roundsStartBeesHere, holding)) return
    restStopped = true
    // the reviews the card stops rather than waits for (DV_STOP)
    for (let i = 0; i < deps.reviews(); i++)
      if (drainVerdict(WK_REVIEW, false, waited(), deps.capSeconds) === DV_STOP)
        reviewsStopped++
    await deps.stopRest().catch(() => {})
  }

  const heldAtStart = holdingNow().holding
  for (;;) {
    const { holding, due } = holdingNow()
    await stopRest(holding)
    if (due.length > 0) {
      for (const bee of due) handing.add(bee.issue)
      const deadline = new Promise<void>((resolve) =>
        deps.sleep(HAND_BACK_SECONDS * 1000).then(resolve),
      )
      await Promise.race([
        Promise.allSettled(due.map((bee) => deps.handBack(bee))),
        deadline,
      ])
      handedBack.push(...due.map((bee) => bee.issue))
      continue
    }
    if (holding === 0) break
    const left = deps.capSeconds * 1000 - (deps.now() - start)
    await deps.sleep(Math.max(1, Math.min(pollMs, left)))
  }

  // nothing holds: whatever runs on is stopped, and the exit waits for the
  // stops to land (turn_stop.t27: abort, grace, escalation)
  await stopRest(0)
  const stoppedAt = deps.now()
  while (exitStep(0, deps.reviews(), deps.now() - stoppedAt) === EXIT_WAIT)
    await deps.sleep(100)
  return {
    tookMs: deps.now() - start,
    heldAtStart,
    handedBack,
    reviewsStopped,
  }
}
