/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTOR RUNTIME, MVP (gHashTag/t27 specs/queen/actors.t27, epic t27#7851).
 *
 * Owner's rule, 2026-10-08: whatever runs at the same time in the Queen is an
 * actor, with a pid, a bounded mailbox and a supervisor. The card makes the
 * decisions:
 *   - whether a send reaches its target (reaches, deliver, deliver_control);
 *   - which lane goes first (next_lane, ctl_*);
 *   - whether a dead child comes back, and after how long (on_child_exit,
 *     unstable_streak, with_backoff, backoff_seconds, jittered_seconds);
 *   - when a turn has run too long (turn_signal).
 * This file holds the state, runs the turns, and keeps the clock it is given.
 *
 * A JavaScript promise cannot be killed. A turn the card kills is abandoned
 * instead. Its pid is dead, so whatever it sends with `from` is dropped, and
 * its result is ignored. Its work is fenced by control.t27's lease. Whatever
 * it had already started outside keeps running until it ends on its own.
 *
 * Nothing here is wired into the Queen yet. The swap waits for the benchmark
 * (t27#7851).
 */

import {
  D_DROPPED_DEAD,
  D_QUEUED,
  GIVE_UP_REASON,
  LANE_CONTROL,
  LANE_DATA,
  MAILBOX_CAP,
  RESTART_PERMANENT,
  SUP_GIVE_UP,
  SUP_RESTART,
  TURN_MAX_SECONDS,
  X_CRASH,
  X_KILL,
  X_SHUTDOWN,
} from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'

export const ACTORS_CARD = 'queen/actors.wasm'

const card = () => loadCardWasm(ACTORS_CARD)
const big = (v: number | bigint) => BigInt.asUintN(64, BigInt(v))
/** A card function over u64s; its u64 answer comes back unsigned. */
const c64 = (name: string, ...a: Array<number | bigint>) =>
  big(card().call64(name, ...a))
const c = (name: string, ...a: number[]) => card().call(name, ...a)

export type Pid = bigint
export const slotOf = (pid: Pid): bigint => c64('slot_of', pid)

/** The runtime's clock. A test hands in a virtual one. */
export interface Clock {
  now(): number
  after(ms: number, fn: () => void): () => void
}
export const realClock: Clock = {
  now: () => Date.now(),
  after: (ms, fn) => {
    const t = setTimeout(fn, ms)
    return () => clearTimeout(t)
  },
}

export interface ActorSpec<M> {
  name: string
  /** Called once, after the pid exists and before any message. */
  init?: (self: Pid) => void
  /** One turn per message. A throw or a rejection is a crash. */
  receive: (msg: M, self: Pid) => Promise<void> | void
  /** A control message (tags 1..63), taken before any queued data message. */
  control?: (tag: number, self: Pid) => void
  /** The domain's bound on one turn, in seconds. 0 is the card's default. */
  turnMaxSeconds?: number
}

export interface Down {
  kind: 'DOWN'
  pid: Pid
  reason: number
}

interface Proc {
  pid: Pid
  spec: ActorSpec<unknown>
  box: unknown[]
  lane: bigint
  busy: boolean
  turnAt: number
  cancelKill?: () => void
  onExit: (reason: number) => void
}

export interface ActorStats {
  delivered: number
  deadLetters: number
  crashed: number
  killed: number
}

export function createActorSystem(clock: Clock = realClock) {
  const gens: bigint[] = [0n]
  const live = new Map<bigint, Proc>()
  const watchers = new Map<Pid, Set<Pid>>()
  const stats: ActorStats = {
    delivered: 0,
    deadLetters: 0,
    crashed: 0,
    killed: 0,
  }

  const procOf = (pid: Pid): Proc | undefined => {
    const p = live.get(slotOf(pid))
    return p && card().call64('reaches', pid, p.pid) !== 0 ? p : undefined
  }
  const current = (p: Proc) => live.get(slotOf(p.pid)) === p
  const schedule = (p: Proc) => queueMicrotask(() => step(p))

  function spawn<M>(
    spec: ActorSpec<M>,
    onExit: (reason: number) => void = () => {},
    slot?: bigint,
  ): Pid {
    const s = slot ?? BigInt(gens.push(0n) - 1)
    const gen = c64('next_gen', gens[Number(s)])
    gens[Number(s)] = gen
    const pid = c64('pid_of', s, gen)
    const p: Proc = {
      pid,
      spec: spec as ActorSpec<unknown>,
      box: [],
      lane: 0n,
      busy: false,
      turnAt: 0,
      onExit,
    }
    live.set(s, p)
    spec.init?.(pid)
    return pid
  }

  /**
   * `from` names the sender. A send from a pid that is no longer alive is
   * dropped: an abandoned turn keeps running, but it is dead and reaches no one.
   */
  function send(pid: Pid, msg: unknown, from?: Pid): number {
    if (from !== undefined && !procOf(from)) {
      stats.deadLetters++
      return D_DROPPED_DEAD
    }
    const p = procOf(pid)
    const d = c('deliver', flag(!!p), u32(p?.box.length ?? 0), MAILBOX_CAP)
    if (d !== D_QUEUED || !p) {
      stats.deadLetters++
      return d
    }
    p.box.push(msg)
    stats.delivered++
    schedule(p)
    return d
  }

  function post(pid: Pid, tag: number): number {
    const p = procOf(pid)
    const pending =
      !!p && card().call64('ctl_pending', p.lane, BigInt(tag)) !== 0
    const d = c('deliver_control', flag(!!p), flag(pending))
    if (d !== D_QUEUED || !p) return d
    p.lane = c64('ctl_post', p.lane, BigInt(tag))
    schedule(p)
    return d
  }

  function monitor(watcher: Pid, target: Pid): void {
    if (!procOf(target)) {
      send(watcher, { kind: 'DOWN', pid: target, reason: X_SHUTDOWN } as Down)
      return
    }
    const set = watchers.get(target) ?? new Set<Pid>()
    set.add(watcher)
    watchers.set(target, set)
  }

  /** The end of a process. `quiet` is a stop its supervisor asked for. */
  function exit(pid: Pid, reason: number, quiet = false): void {
    const p = procOf(pid)
    if (!p) return
    live.delete(slotOf(p.pid))
    p.cancelKill?.()
    stats.deadLetters += p.box.length
    for (const w of watchers.get(p.pid) ?? [])
      send(w, { kind: 'DOWN', pid: p.pid, reason } as Down)
    watchers.delete(p.pid)
    if (!quiet) p.onExit(reason)
  }

  function step(p: Proc): void {
    if (p.busy || !current(p)) return
    const lane = c('next_lane', flag(p.lane !== 0n), flag(p.box.length > 0))
    if (lane === LANE_CONTROL) {
      const tag = Number(c64('ctl_next', p.lane))
      p.lane = c64('ctl_take', p.lane)
      try {
        p.spec.control?.(tag, p.pid)
      } catch {
        stats.crashed++
        exit(p.pid, X_CRASH)
        return
      }
      schedule(p)
      return
    }
    if (lane !== LANE_DATA) return
    const msg = p.box.shift()
    const bound = p.spec.turnMaxSeconds ?? 0
    p.busy = true
    p.turnAt = clock.now()
    p.cancelKill = clock.after((bound || TURN_MAX_SECONDS) * 1000, () => {
      const age = Math.ceil((clock.now() - p.turnAt) / 1000)
      if (current(p) && c('turn_signal', u32(age), u32(bound)) === X_KILL) {
        stats.killed++
        exit(p.pid, c('death_reason', X_KILL))
      }
    })
    Promise.resolve()
      .then(() => p.spec.receive(msg, p.pid))
      .then(
        () => {
          if (!current(p)) return
          p.cancelKill?.()
          p.busy = false
          schedule(p)
        },
        () => {
          if (!current(p)) return
          stats.crashed++
          exit(p.pid, X_CRASH)
        },
      )
  }

  return {
    spawn,
    send,
    post,
    monitor,
    exit,
    alive: (pid: Pid) => !!procOf(pid),
    stats,
    clock,
  }
}

export type ActorSystem = ReturnType<typeof createActorSystem>

/** What a supervisor starts: an actor, or a supervisor below it. */
export interface Child {
  name: string
  restart?: number
  start: (
    onExit: (reason: number) => void,
    slot?: bigint,
  ) => {
    pid?: Pid
    stop: () => void
  }
}

export const actorChild = <M>(
  sys: ActorSystem,
  spec: ActorSpec<M>,
  restart: number = RESTART_PERMANENT,
): Child => ({
  name: spec.name,
  restart,
  start: (onExit, slot) => {
    const pid = sys.spawn(spec, onExit, slot)
    return { pid, stop: () => sys.exit(pid, X_SHUTDOWN, true) }
  },
})

export interface SupervisorOptions {
  name: string
  strategy: number
  maxRestarts: number
  periodSeconds: number
}

/**
 * A supervisor over `children`, in start order. The card decides each exit:
 * restart or give up, how long to wait, and which siblings go down and come
 * back with it. Giving up stops every child and reports GIVE_UP_REASON to the
 * parent through `onExit`.
 */
export function supervisor(
  sys: ActorSystem,
  opts: SupervisorOptions,
  children: Child[],
): Child {
  return {
    name: opts.name,
    restart: RESTART_PERMANENT,
    start: (onExit) => {
      const clock = sys.clock
      const running: Array<{ pid?: Pid; stop: () => void } | undefined> = []
      const slots: Array<bigint | undefined> = []
      const startedAt: number[] = []
      const streak: number[] = children.map(() => 0)
      let restarts: number[] = []
      let down = false
      const pending = new Set<() => void>()

      const start = (i: number) => {
        startedAt[i] = clock.now()
        const h = children[i].start((reason) => exited(i, reason), slots[i])
        running[i] = h
        if (h.pid !== undefined) slots[i] = slotOf(h.pid)
      }
      const stopAll = () => {
        for (const cancel of pending) cancel()
        pending.clear()
        for (let j = running.length - 1; j >= 0; j--) running[j]?.stop()
      }
      const exited = (i: number, reason: number) => {
        if (down) return
        running[i] = undefined
        const now = clock.now()
        restarts = restarts.filter(
          (t) =>
            c('in_period', u32((now - t) / 1000), u32(opts.periodSeconds)) !==
            0,
        )
        const rtype = children[i].restart ?? RESTART_PERMANENT
        streak[i] = c(
          'unstable_streak',
          u32(streak[i]),
          u32((now - startedAt[i]) / 1000),
        )
        const decision = c(
          'with_backoff',
          c(
            'on_child_exit',
            rtype,
            reason,
            u32(restarts.length),
            u32(opts.maxRestarts),
          ),
          u32(streak[i]),
        )
        if (decision === SUP_GIVE_UP) {
          down = true
          stopAll()
          onExit(GIVE_UP_REASON)
          return
        }
        if (decision !== SUP_RESTART) return
        restarts.push(now)
        for (let j = children.length - 1; j >= 0; j--)
          if (
            c('stopped_by_restart', opts.strategy, i, j, 1) !== 0 &&
            running[j]
          ) {
            running[j]?.stop()
            running[j] = undefined
          }
        const wait = Number(
          card().call64(
            'jittered_seconds',
            c('backoff_seconds', u32(streak[i])),
            slots[i] ?? BigInt(i + 1),
          ),
        )
        const cancel = clock.after(wait * 1000, () => {
          pending.delete(cancel)
          if (down) return
          for (let j = 0; j < children.length; j++)
            if (
              c(
                'restarted',
                opts.strategy,
                i,
                j,
                children[j].restart ?? RESTART_PERMANENT,
                1,
              ) !== 0 &&
              !running[j]
            )
              start(j)
        })
        pending.add(cancel)
      }

      for (let i = 0; i < children.length; i++) start(i)
      return {
        stop: () => {
          down = true
          stopAll()
        },
      }
    },
  }
}
