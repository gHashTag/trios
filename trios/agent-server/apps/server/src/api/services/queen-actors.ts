/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTOR RUNTIME (gHashTag/t27 specs/queen/actors.t27, epic t27#7851).
 *
 * Owner's rule, 2026-10-08: whatever runs at the same time in the Queen is an
 * actor, with a pid, a bounded mailbox and a supervisor. The card makes the
 * decisions:
 *   - whether a send reaches its target (reaches, deliver, deliver_control,
 *     and across nodes is_remote, send_remote);
 *   - which lane goes first (next_lane, ctl_*);
 *   - whether a dead child comes back, and after how long (on_child_exit,
 *     unstable_streak, with_backoff, backoff_seconds, jittered_seconds);
 *   - when a turn has run too long (turn_signal), and what killing it does
 *     (turn_isolation, kill_effect, stop_signal, holds_after_kill, and
 *     turn_stop.t27's escalation);
 *   - when a scheduler must yield (slice_spent);
 *   - what a monitor across nodes hears (node_up, remote_down).
 * This file holds the state, runs the turns, and keeps the clock it is given.
 *
 * PREEMPTION (section 8). Between turns, a slice: at most SLICE_TURNS turns
 * or SLICE_MICROS, then the scheduler yields to the host's timers and I/O.
 * Within a turn, isolation: a turn with `isolated` work runs it in an OS
 * thread or an OS process, which the OS preempts, so it never holds the host's
 * thread. Only a process is certain to stop when killed: a terminated worker
 * thread kept spinning (measured). A kill of any other turn abandons it. Its
 * pid is dead, so what it sends with `from` is dropped, and its late result
 * is ignored and fenced by control.t27's lease.
 *
 * STOP (turn_stop.t27, trios#1712). With `turnStop`, a turn whose pid dies is
 * stopped, not only abandoned. Each data turn has its pid's AbortSignal,
 * handed to `receive` and to its isolated work, and held for everything the
 * turn starts (queen-turn-stop.ts turnRunner). When the pid dies - its timer
 * or any `exit` - stop_signal says X_SHUTDOWN and the signal is aborted: a
 * model call's fetch rejects, a command's process group is killed. At
 * TURN_STOP_GRACE_MS stop_signal says X_KILL, and the card's `escalation` says
 * what that does to work still running: SIGKILL to the turn's process group,
 * or, for a turn on the loop or in a thread that ignored its abort, an
 * abandon. A stopped turn holds its row and lane until holds_after_kill says
 * it does not. Without `turnStop` a kill abandons the turn, as before.
 *
 * NODES (section 9). A pid names its node. A send to another node goes over
 * the `link`, a NodeLink. A node that stops renewing its lease is down, and
 * every monitor across it hears X_NOCONNECTION. No code moves between nodes:
 * a remote start names a kind registered on that node.
 */

import {
  D_DROPPED_DEAD,
  D_QUEUED,
  GIVE_UP_REASON,
  ISO_LOOP,
  KILL_STOPS,
  LANE_CONTROL,
  LANE_DATA,
  MAILBOX_CAP,
  RESTART_PERMANENT,
  SUP_GIVE_UP,
  SUP_RESTART,
  TURN_MAX_SECONDS,
  X_CRASH,
  X_KILL,
  X_NOCONNECTION,
  X_NOPROC,
  X_SHUTDOWN,
} from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { escalation, turnRunner } from './queen-turn-stop'
import {
  ESC_ABANDON,
  ESC_KILL_GROUP,
  TURN_STOP_GRACE_MS,
} from './queen-turn-stop-card.gen'

export const ACTORS_CARD = 'queen/actors.wasm'

const card = () => loadCardWasm(ACTORS_CARD)
const big = (v: number | bigint) => BigInt.asUintN(64, BigInt(v))
/** A card function over u64s; its u64 answer comes back unsigned. */
const c64 = (name: string, ...a: Array<number | bigint>) =>
  big(card().call64(name, ...a))
const c = (name: string, ...a: number[]) => card().call(name, ...a)

export type Pid = bigint
export const slotOf = (pid: Pid): bigint => c64('slot_of', pid)
export const nodeOf = (pid: Pid): number => Number(c64('node_of', pid))
export const turnIsolation = (
  cpuBound: boolean,
  foreignCode: boolean,
  mustStop: boolean,
): number =>
  c('turn_isolation', flag(cpuBound), flag(foreignCode), flag(mustStop))
export const holdsAfterKill = (
  isolation: number,
  workEnded: boolean,
): boolean => c('holds_after_kill', isolation, flag(workEnded)) !== 0

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

/**
 * A turn's work, started off the host's thread: its result, and a stop.
 * `stop` is the escalation's kill. The abort signal handed to `start` is the
 * request that comes before it.
 */
export interface IsolatedWork {
  result: Promise<unknown>
  stop: () => void
}

export interface Isolated<M> {
  /** Burns CPU in this build's own code: an OS thread. */
  cpuBound: boolean
  /** Runs code a bee wrote, or a compiler on its work: an OS process. */
  foreignCode: boolean
  /**
   * Must end when its turn is killed: an OS process. A terminated worker
   * thread can keep running (measured, actors.t27 section 8).
   */
  mustStop?: boolean
  start: (msg: M, signal: AbortSignal) => IsolatedWork
}

export interface ActorSpec<M> {
  name: string
  /** Called once, after the pid exists and before any message. */
  init?: (self: Pid) => void
  /**
   * One turn per message. A throw or a rejection is a crash. With `isolated`,
   * the isolated work runs first and its result is the third argument. The
   * fourth is the turn's abort signal: with `turnStop` it is aborted when the
   * pid dies, so whatever the turn waits on can stop with it.
   */
  receive: (
    msg: M,
    self: Pid,
    result: unknown,
    signal: AbortSignal,
  ) => Promise<void> | void
  /** A control message (tags 1..63), taken before any queued data message. */
  control?: (tag: number, self: Pid) => void
  /** The domain's bound on one turn, in seconds. 0 is the card's default. */
  turnMaxSeconds?: number
  isolated?: Isolated<M>
}

export interface Down {
  kind: 'DOWN'
  pid: Pid
  reason: number
}

/** What crosses between nodes. Everything in it must survive structuredClone. */
export type Mail =
  | { kind: 'send'; to: Pid; msg: unknown; from?: Pid }
  | { kind: 'monitor'; target: Pid; watcher: Pid }
  | { kind: 'exit'; target: Pid; reason: number }
  | {
      kind: 'spawn'
      spawnKind: string
      arg: unknown
      ref: number
      from: number
    }
  | { kind: 'spawned'; ref: number; pid: Pid | null }

/**
 * One node's view of the others. `up` reads the node's lease through the card
 * (node_up). `carry` delivers one mail to a node, or loses it if that node is
 * gone, as a socket to a dead host does.
 */
export interface NodeLink {
  node: number
  up(node: number): boolean
  carry(toNode: number, mail: Mail): void
  onMail(handler: (mail: Mail) => void): void
  onNodeDown(handler: (node: number) => void): void
}

/** The signal of a turn that is never stopped: without `turnStop`. */
const NEVER_ABORTED = new AbortController().signal

/** One data turn, from its message to the end of its work. */
interface Turn {
  stopWork?: () => void
  ended: boolean
  /** When its stop began: its pid died while the work ran. */
  stoppedAt?: number
  /** Stopped, and still holding its row and lane (holds_after_kill). */
  held: boolean
  cancelGrace?: () => void
}

interface Proc {
  pid: Pid
  spec: ActorSpec<unknown>
  box: unknown[]
  lane: bigint
  busy: boolean
  turnAt: number
  isolation: number
  cancelKill?: () => void
  turn?: Turn
  /**
   * With `turnStop`: one controller for the pid's life. Its turns run one at
   * a time and it is aborted only when the pid dies, so every turn can share
   * it: a new AbortController per turn cost about 1.1 us of a 13 us message
   * (measured). A turn that adds a listener to it removes it when done.
   */
  abort?: AbortController
  inTurn?: <T>(fn: () => T) => T
  onExit: (reason: number) => void
}

export interface ActorStats {
  delivered: number
  deadLetters: number
  crashed: number
  killed: number
  /**
   * Kills that stopped the work, not only the turn. Without `turnStop`, a
   * process turn killed (kill_effect). With it, a stopped turn whose work then
   * ended, by its abort or by the escalation.
   */
  stopped: number
  /** Stopped turns past their grace whose process group was killed. */
  escalated: number
  /** Stopped turns past their grace that nothing could stop: loop or thread. */
  abandoned: number
  /** Stopped turns that still hold their row and lane, now. */
  held: number
  /** Times a scheduler spent its slice and yielded to the host. */
  yields: number
  remoteSent: number
}

export interface ActorSystemOptions {
  /** This system's node (section 9). 0 alone is section 1 unchanged. */
  node?: number
  link?: NodeLink
  /** False turns the slice off: every turn chained as a microtask, as before. */
  slices?: boolean
  /**
   * True stops a turn whose pid dies (turn_stop.t27): its abort, the grace,
   * the escalation. False, the default, abandons it as before. A flag until
   * the benchmark posted on t27#7851 is accepted.
   */
  turnStop?: boolean
}

export function createActorSystem(
  clock: Clock = realClock,
  options: ActorSystemOptions = {},
) {
  const node = options.node ?? options.link?.node ?? 0
  const link = options.link
  const slicesOn = options.slices ?? true
  const turnStop = options.turnStop ?? false
  const gens = new Map<bigint, bigint>()
  let nextLocal = 1
  const live = new Map<bigint, Proc>()
  const watchers = new Map<Pid, Set<Pid>>()
  // monitors this node holds on pids elsewhere, by the node they live on
  const remoteWatches = new Map<number, Array<{ watcher: Pid; target: Pid }>>()
  const kinds = new Map<string, (arg: unknown) => ActorSpec<unknown>>()
  const spawnWaits = new Map<number, (pid: Pid | null) => void>()
  let nextRef = 1
  const stats: ActorStats = {
    delivered: 0,
    deadLetters: 0,
    crashed: 0,
    killed: 0,
    stopped: 0,
    escalated: 0,
    abandoned: 0,
    held: 0,
    yields: 0,
    remoteSent: 0,
  }

  const procOf = (pid: Pid): Proc | undefined => {
    const p = live.get(slotOf(pid))
    return p && card().call64('reaches', pid, p.pid) !== 0 ? p : undefined
  }
  const current = (p: Proc) => live.get(slotOf(p.pid)) === p
  const remote = (pid: Pid) =>
    card().call64('is_remote', pid, BigInt(node)) !== 0

  // THE SLICE (section 8). Ready processes wait in one queue. A slice lasts
  // until the event loop turns: it counts turns across every drain the
  // microtask queue starts, and once slice_spent says so, the next drain
  // waits for setImmediate, so timers and sockets run between slices. Before
  // this, one message chain ran 200 000 turns as microtasks, and nothing else
  // in the process could run.
  const ready: Proc[] = []
  let head = 0
  const queued = new Set<Proc>()
  let draining = false
  let sliceOpen = false
  let sliceTurns = 0
  let sliceT0 = 0
  const turnStarted = () => {
    if (!sliceOpen) {
      sliceOpen = true
      sliceTurns = 0
      sliceT0 = performance.now()
      // the loop turned: the next turn opens a new slice
      setImmediate(() => {
        sliceOpen = false
      })
    }
    sliceTurns++
  }
  const spent = () =>
    sliceOpen &&
    c(
      'slice_spent',
      u32(sliceTurns),
      u32((performance.now() - sliceT0) * 1000),
    ) !== 0
  const compact = () => {
    if (head > 0) {
      ready.splice(0, head)
      head = 0
    }
  }
  const yieldThenDrain = () => {
    stats.yields++
    compact()
    setImmediate(drain)
  }
  const drain = () => {
    while (head < ready.length) {
      if (spent()) return yieldThenDrain()
      const p = ready[head++]
      queued.delete(p)
      step(p)
    }
    compact()
    draining = false
  }
  const schedule = (p: Proc) => {
    if (!slicesOn) {
      queueMicrotask(() => step(p))
      return
    }
    if (queued.has(p)) return
    queued.add(p)
    ready.push(p)
    if (!draining) {
      draining = true
      if (spent()) yieldThenDrain()
      else queueMicrotask(drain)
    }
  }

  function spawn<M>(
    spec: ActorSpec<M>,
    onExit: (reason: number) => void = () => {},
    slot?: bigint,
  ): Pid {
    let s = slot
    if (s === undefined) {
      s = slotOf(c64('node_pid', BigInt(node), BigInt(nextLocal++), 0n))
    }
    const gen = c64('next_gen', gens.get(s) ?? 0n)
    gens.set(s, gen)
    const pid = c64('pid_of', s, gen)
    const isolation = spec.isolated
      ? turnIsolation(
          spec.isolated.cpuBound,
          spec.isolated.foreignCode,
          spec.isolated.mustStop ?? false,
        )
      : ISO_LOOP
    const p: Proc = {
      pid,
      spec: spec as ActorSpec<unknown>,
      box: [],
      lane: 0n,
      busy: false,
      turnAt: 0,
      isolation,
      onExit,
    }
    if (turnStop) {
      p.abort = new AbortController()
      p.inTurn = turnRunner(p.abort.signal)
    }
    live.set(s, p)
    spec.init?.(pid)
    return pid
  }

  /**
   * `from` names the sender. A send from a pid that is no longer alive is
   * dropped: an abandoned turn keeps running, but it is dead and reaches no one.
   * A send to a pid on another node goes over the link, if the card says that
   * node is up (send_remote).
   */
  function send(pid: Pid, msg: unknown, from?: Pid): number {
    if (from !== undefined && !procOf(from)) {
      stats.deadLetters++
      return D_DROPPED_DEAD
    }
    if (remote(pid)) {
      const to = nodeOf(pid)
      const d = c('send_remote', flag(!!link && link.up(to)))
      if (d !== D_QUEUED || !link) {
        stats.deadLetters++
        return d
      }
      stats.remoteSent++
      link.carry(to, { kind: 'send', to: pid, msg, from })
      return d
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
    if (remote(target)) {
      const on = nodeOf(target)
      if (!link || !link.up(on)) {
        send(watcher, {
          kind: 'DOWN',
          pid: target,
          reason: c('remote_down', 0, 0, X_NOPROC),
        } as Down)
        return
      }
      const list = remoteWatches.get(on) ?? []
      list.push({ watcher, target })
      remoteWatches.set(on, list)
      link.carry(on, { kind: 'monitor', target, watcher })
      return
    }
    if (!procOf(target)) {
      send(watcher, { kind: 'DOWN', pid: target, reason: X_NOPROC } as Down)
      return
    }
    const set = watchers.get(target) ?? new Set<Pid>()
    set.add(watcher)
    watchers.set(target, set)
  }

  /** The end of a process. `quiet` is a stop its supervisor asked for. */
  function exit(pid: Pid, reason: number, quiet = false): void {
    if (remote(pid)) {
      link?.carry(nodeOf(pid), { kind: 'exit', target: pid, reason })
      return
    }
    const p = procOf(pid)
    if (!p) return
    live.delete(slotOf(p.pid))
    p.cancelKill?.()
    if (turnStop && p.turn && !p.turn.ended) stopTurn(p, p.turn, reason)
    stats.deadLetters += p.box.length
    for (const w of watchers.get(p.pid) ?? [])
      send(w, { kind: 'DOWN', pid: p.pid, reason } as Down)
    watchers.delete(p.pid)
    if (!quiet) p.onExit(reason)
  }

  // A stopped turn gives back its row and lane when holds_after_kill says so:
  // once its work ended or, in an OS process, once the kill landed.
  const release = (p: Proc, t: Turn) => {
    if (!t.held || holdsAfterKill(p.isolation, t.ended)) return
    t.held = false
    stats.held--
  }

  /**
   * The stop of a turn whose pid just died (turn_stop.t27). stop_signal at
   * 0 ms is X_SHUTDOWN: the abort. At TURN_STOP_GRACE_MS it is X_KILL, and
   * the card's `escalation` says what that does to work still running.
   */
  function stopTurn(p: Proc, t: Turn, reason: number): void {
    const at = clock.now()
    t.stoppedAt = at
    t.held = true
    stats.held++
    if (c('stop_signal', 0, TURN_STOP_GRACE_MS) === X_SHUTDOWN)
      p.abort?.abort(
        new DOMException(
          `aborted: the turn was stopped (exit ${reason})`,
          'AbortError',
        ),
      )
    t.cancelGrace = clock.after(TURN_STOP_GRACE_MS, () => {
      const waited = u32(clock.now() - at)
      if (c('stop_signal', waited, TURN_STOP_GRACE_MS) !== X_KILL) return
      const esc = escalation(
        c('kill_effect', p.isolation) === KILL_STOPS,
        t.ended,
      )
      if (esc === ESC_KILL_GROUP && t.stopWork) {
        stats.escalated++
        t.stopWork()
        release(p, t)
      } else if (esc === ESC_ABANDON) {
        stats.abandoned++
      }
    })
  }

  /** The end of a turn's work, whatever ended it. */
  const turnEnded = (p: Proc, t: Turn) => {
    if (t.ended) return
    t.ended = true
    if (t.stoppedAt === undefined) return
    t.cancelGrace?.()
    stats.stopped++
    release(p, t)
  }

  function step(p: Proc): void {
    if (p.busy || !current(p)) return
    const lane = c('next_lane', flag(p.lane !== 0n), flag(p.box.length > 0))
    if (slicesOn && (lane === LANE_CONTROL || lane === LANE_DATA)) turnStarted()
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
    const t: Turn = { ended: false, held: false }
    const signal = p.abort?.signal ?? NEVER_ABORTED
    p.turn = t
    p.busy = true
    p.turnAt = clock.now()
    p.cancelKill = clock.after((bound || TURN_MAX_SECONDS) * 1000, () => {
      const age = Math.ceil((clock.now() - p.turnAt) / 1000)
      if (current(p) && c('turn_signal', u32(age), u32(bound)) === X_KILL) {
        stats.killed++
        // Without turnStop: an isolated turn's work is stopped for real, a
        // loop turn's is not. With it, `exit` starts the turn's stop.
        if (
          !turnStop &&
          c('kill_effect', p.isolation) === KILL_STOPS &&
          t.stopWork
        ) {
          t.stopWork()
          stats.stopped++
        }
        exit(p.pid, c('death_reason', X_KILL))
      }
    })
    const run = () =>
      p.isolation !== ISO_LOOP && p.spec.isolated
        ? (() => {
            const work = p.spec.isolated.start(msg, signal)
            t.stopWork = work.stop
            return work.result.then((r) =>
              p.spec.receive(msg, p.pid, r, signal),
            )
          })()
        : Promise.resolve().then(() =>
            p.spec.receive(msg, p.pid, undefined, signal),
          )
    // with turnStop, everything the turn starts, down to a git command, sees
    // its signal; without it a turn costs what it did
    const turn = p.inTurn ? p.inTurn(run) : run()
    turn.then(
      () => {
        turnEnded(p, t)
        if (!current(p)) return
        p.cancelKill?.()
        p.turn = undefined
        p.busy = false
        schedule(p)
      },
      () => {
        turnEnded(p, t)
        if (!current(p)) return
        p.turn = undefined
        stats.crashed++
        exit(p.pid, X_CRASH)
      },
    )
  }

  // what arrives from other nodes
  link?.onMail((mail) => {
    if (mail.kind === 'send') {
      send(mail.to, mail.msg)
    } else if (mail.kind === 'monitor') {
      if (!procOf(mail.target))
        send(mail.watcher, {
          kind: 'DOWN',
          pid: mail.target,
          reason: X_NOPROC,
        } as Down)
      else {
        const set = watchers.get(mail.target) ?? new Set<Pid>()
        set.add(mail.watcher)
        watchers.set(mail.target, set)
      }
    } else if (mail.kind === 'exit') {
      exit(mail.target, mail.reason)
    } else if (mail.kind === 'spawn') {
      const make = kinds.get(mail.spawnKind)
      const pid = make ? spawn(make(mail.arg)) : null
      link.carry(mail.from, { kind: 'spawned', ref: mail.ref, pid })
    } else if (mail.kind === 'spawned') {
      spawnWaits.get(mail.ref)?.(mail.pid)
      spawnWaits.delete(mail.ref)
    }
  })
  // a node whose lease lapsed: every monitor across it hears noconnection
  link?.onNodeDown((down) => {
    for (const { watcher, target } of remoteWatches.get(down) ?? [])
      send(watcher, {
        kind: 'DOWN',
        pid: target,
        reason: c('remote_down', 0, 0, X_NOPROC),
      } as Down)
    remoteWatches.delete(down)
    for (const [ref, resolve] of spawnWaits) {
      resolve(null)
      spawnWaits.delete(ref)
    }
  })

  /** A kind another node may start here by name (no code crosses nodes). */
  function registerKind<A>(name: string, make: (arg: A) => ActorSpec<unknown>) {
    kinds.set(name, make as (arg: unknown) => ActorSpec<unknown>)
  }

  /** Start a registered kind on `on`. Resolves null if it could not start. */
  function spawnOn(
    on: number,
    spawnKind: string,
    arg: unknown,
  ): Promise<Pid | null> {
    if (on === node) {
      const make = kinds.get(spawnKind)
      return Promise.resolve(make ? spawn(make(arg)) : null)
    }
    if (!link || !link.up(on)) return Promise.resolve(null)
    const ref = nextRef++
    return new Promise((resolve) => {
      spawnWaits.set(ref, resolve)
      link.carry(on, { kind: 'spawn', spawnKind, arg, ref, from: node })
    })
  }

  return {
    node,
    spawn,
    spawnOn,
    registerKind,
    send,
    post,
    monitor,
    exit,
    alive: (pid: Pid) => !!procOf(pid),
    /** Whether a node is up as this node sees it; this node always is. */
    up: (n: number) => n === node || (!!link && link.up(n)),
    stats,
    clock,
    /** Whether a dying pid's turn is stopped (turn_stop.t27) or abandoned. */
    turnStop,
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

export interface RemoteChildOptions {
  name: string
  /** A kind registered on every candidate node (registerKind). */
  kind: string
  arg: unknown
  /** The nodes it may run on. */
  nodes: () => number[]
  /** Free room for this kind on a node, as the placer counts it. */
  room: (node: number) => number
  /** Told where each start landed, so the placer's count can follow. */
  placed?: (node: number | null) => void
  restart?: number
}

const placeBefore = (freeA: number, a: number, freeB: number, b: number) =>
  card().call64(
    'place_before',
    u32(freeA),
    BigInt(a),
    u32(freeB),
    BigInt(b),
  ) !== 0

/**
 * A child started on another node (actors.t27 section 9). Each start goes to
 * the roomiest node that is up (place_before), so a restart after its node
 * went down lands elsewhere. It is watched from here: its DOWN - X_NOCONNECTION
 * once its node's lease lapses - is the supervisor's onExit.
 */
export function remoteChild(sys: ActorSystem, opts: RemoteChildOptions): Child {
  return {
    name: opts.name,
    restart: opts.restart ?? RESTART_PERMANENT,
    start: (onExit) => {
      let stopped = false
      let pid: Pid | undefined
      let best: number | null = null
      for (const n of opts.nodes().filter((m) => sys.up(m)))
        if (
          best === null ||
          placeBefore(opts.room(n), n, opts.room(best), best)
        )
          best = n
      opts.placed?.(best)
      const watcher: Pid = sys.spawn<Down>({
        name: `${opts.name}-watch`,
        receive: (m) => {
          if (m.kind !== 'DOWN' || m.pid !== pid) return
          sys.exit(watcher, X_SHUTDOWN, true)
          if (!stopped) onExit(m.reason)
        },
      })
      const failed = () => {
        sys.exit(watcher, X_SHUTDOWN, true)
        if (!stopped) onExit(X_NOCONNECTION)
      }
      if (best === null) queueMicrotask(failed)
      else
        void sys.spawnOn(best, opts.kind, opts.arg).then((p) => {
          if (stopped) {
            if (p !== null) sys.exit(p, X_SHUTDOWN)
            return
          }
          if (p === null) return failed()
          pid = p
          sys.monitor(watcher, p)
        })
      return {
        stop: () => {
          stopped = true
          if (pid !== undefined) sys.exit(pid, X_SHUTDOWN)
          sys.exit(watcher, X_SHUTDOWN, true)
        },
      }
    },
  }
}
