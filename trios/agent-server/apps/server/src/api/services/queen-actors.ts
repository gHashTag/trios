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
 *
 * INCARNATIONS AND THE FENCE (specs/queen/netlink.t27, trios#1712). A linked
 * node takes a new incarnation at every start, and every pid it hands out
 * carries it (inc_next_gen), so a restarted node never reuses a pid of its
 * last life. A node-down names the incarnation that went down, and ends only
 * what waited on it (lost_with). A node the link says must fence (must_fence)
 * stops every actor at once, quietly, and from then on sends nothing and
 * starts nothing: its peers are about to see it down and start its children
 * elsewhere.
 *
 * TELEMETRY (specs/queen/telemetry.t27), with `options.telemetry` only: every
 * hook below is guarded by `tel !== undefined`, reads the clock it is given,
 * and never schedules. queen-actors-telemetry.ts says what is counted.
 */

import {
  D_DROPPED_DEAD,
  D_DROPPED_FULL,
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
import {
  type ActorTelemetry,
  createActorTelemetry,
  type KindStats,
  type TelemetryOptions,
} from './queen-actors-telemetry'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { END_CRASH, END_KILL, END_OK } from './queen-telemetry-card.gen'
import { escalation, turnRunner } from './queen-turn-stop'
import {
  ESC_ABANDON,
  ESC_KILL_GROUP,
  TURN_STOP_GRACE_MS,
} from './queen-turn-stop-card.gen'

export const ACTORS_CARD = 'queen/actors.wasm'
export const NETLINK_CARD = 'queen/netlink.wasm'

const card = () => loadCardWasm(ACTORS_CARD)
const netlink = () => loadCardWasm(NETLINK_CARD)
const big = (v: number | bigint) => BigInt.asUintN(64, BigInt(v))
/** A card function over u64s; its u64 answer comes back unsigned. */
const c64 = (name: string, ...a: Array<number | bigint>) =>
  big(card().call64(name, ...a))
const c = (name: string, ...a: number[]) => card().call(name, ...a)

export type Pid = bigint
export const slotOf = (pid: Pid): bigint => c64('slot_of', pid)
export const nodeOf = (pid: Pid): number => Number(c64('node_of', pid))
/** The incarnation a pid was born in (netlink.t27 inc_of); 0 with no store. */
export const incOf = (pid: Pid): number =>
  Number(big(netlink().call64('inc_of', pid)))
/** Whether a node-down of (downNode, downInc) ends what waits on (node, inc). */
const lostWith = (
  node: number,
  inc: number,
  downNode: number,
  downInc: number,
) =>
  netlink().call64(
    'lost_with',
    BigInt(node),
    BigInt(inc),
    BigInt(downNode),
    BigInt(downInc),
  ) !== 0
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
  /** What its telemetry counts it under; its name when unset. */
  kind?: string
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
  /**
   * The monitor's reference, not enumerable: a demonitor with flush finds its
   * own DOWN by it, and a DOWN still reads as { kind, pid, reason }.
   */
  readonly ref?: number
}

const downOf = (pid: Pid, reason: number, ref: number): Down => {
  const down = { kind: 'DOWN', pid, reason } as Down
  Object.defineProperty(down, 'ref', { value: ref })
  return down
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
      /** The asker's incarnation: the answer is written for it alone. */
      fromInc: number
    }
  | { kind: 'spawned'; ref: number; pid: Pid | null }

/**
 * One node's view of the others. `up` reads the node's lease through the card
 * (node_up). `carry` delivers one mail to a node, or loses it if that node is
 * gone, as a socket to a dead host does. `toInc` is the receiver's incarnation
 * the mail is written for (netlink.t27 section 1): mail for any other one is
 * expired, never delivered.
 */
export interface NodeLink {
  node: number
  /** This node's incarnation, taken from the store at start; 0 with none. */
  incarnation: number
  up(node: number): boolean
  /** The incarnation of `node` as this node last read it; 0 if unknown. */
  incarnationOf(node: number): number
  carry(toNode: number, mail: Mail, toInc: number): void
  onMail(handler: (mail: Mail) => void): void
  /** A node-down names the incarnation that went down. */
  onNodeDown(handler: (node: number, inc: number) => void): void
  /** Whether this node must stop now (netlink.t27 must_fence). */
  fenced(): boolean
  /** Told once, when the link fences this node. */
  onFenced(handler: () => void): void
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
  // telemetry's: its kind, its depth alarm, its kill bound, its letter's
  // wait, and the ms from which its turn is long
  k?: KindStats
  alarm: boolean
  bound: number
  waitMs: number
  longAt: number
}

/**
 * A sampled letter carries its arrival time. step() unwraps it, and so must
 * anything else that reads a mailbox (`mailbox`) before its owner does.
 */
class Stamped {
  constructor(
    readonly msg: unknown,
    readonly at: number,
  ) {}
}

/**
 * The message a letter in a mailbox holds. WHY: with telemetry on, a sampled
 * letter waits wrapped in its arrival time, and a reader that matched the
 * wrapper by `kind` missed it: demonitor's flush left a sampled DOWN queued,
 * and a call's timeout won over a sampled reply already queued
 * (tests/api/queen-actors-queued-letters.test.ts, gHashTag/trios#1731).
 */
export const letterOf = (m: unknown): unknown =>
  m instanceof Stamped ? m.msg : m

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
  /** Actors stopped because their node fenced itself (netlink.t27). */
  fencedStops: number
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
  /** Counts, histograms and the decision log (telemetry.t27). Off when unset. */
  telemetry?: TelemetryOptions
}

export function createActorSystem(
  clock: Clock = realClock,
  options: ActorSystemOptions = {},
) {
  const node = options.node ?? options.link?.node ?? 0
  const link = options.link
  const slicesOn = options.slices ?? true
  const turnStop = options.turnStop ?? false
  const inc = link?.incarnation ?? 0
  // a linked node's pids carry its incarnation; a lone one keeps section 1's
  const nextGen = link
    ? (gen: bigint) => big(netlink().call64('inc_next_gen', BigInt(inc), gen))
    : (gen: bigint) => c64('next_gen', gen)
  const gens = new Map<bigint, bigint>()
  let nextLocal = 1
  const live = new Map<bigint, Proc>()
  // One entry per monitor call, never merged: two monitors on one target are
  // two DOWNs (monitors_after_monitor). A Set here gave one.
  const watchers = new Map<Pid, Array<{ watcher: Pid; ref: number }>>()
  let nextMonitorRef = 1
  // Told of every local exit, after the DOWNs: links (queen-actors-links.ts)
  const exitListeners = new Set<(pid: Pid, reason: number) => void>()
  // monitors this node holds on pids elsewhere, by the node they live on
  const remoteWatches = new Map<number, Array<{ watcher: Pid; target: Pid }>>()
  const kinds = new Map<string, (arg: unknown) => ActorSpec<unknown>>()
  // a remote start waits on one incarnation of one node
  const spawnWaits = new Map<
    number,
    { on: number; onInc: number; resolve: (pid: Pid | null) => void }
  >()
  let nextRef = 1
  let fenced = false
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
    fencedStops: 0,
  }
  const tel: ActorTelemetry | undefined = options.telemetry
    ? createActorTelemetry(clock, options.telemetry)
    : undefined
  tel?.watch({
    procs: () => live.values(),
    slot: (pid) => Number(slotOf(pid)),
    yields: () => stats.yields,
  })

  const procOf = (pid: Pid): Proc | undefined => {
    const p = live.get(slotOf(pid))
    return p && card().call64('reaches', pid, p.pid) !== 0 ? p : undefined
  }
  const current = (p: Proc) => live.get(slotOf(p.pid)) === p
  const remote = (pid: Pid) =>
    card().call64('is_remote', pid, BigInt(node)) !== 0

  // THE FENCE (netlink.t27 section 3). The link says when: a renewal or a
  // write the store refused, or no renewal confirmed for SELF_FENCE_SECONDS.
  // Every actor stops where it is, quietly: a supervisor here must not bring
  // back what a peer is about to start elsewhere. A running turn is stopped
  // as a kill stops it: with turnStop, its abort, grace and escalation;
  // without, isolated work a kill stops (kill_effect). A remote start still
  // waiting resolves to no pid.
  const fence = () => {
    if (fenced) return
    fenced = true
    for (const p of live.values()) {
      p.cancelKill?.()
      const t = p.turn
      if (t && !t.ended) {
        if (turnStop) stopTurn(p, t, X_NOCONNECTION)
        else if (c('kill_effect', p.isolation) === KILL_STOPS && t.stopWork) {
          t.stopWork()
          stats.stopped++
        }
      }
      stats.deadLetters += p.box.length
      tel?.exited(p)
      stats.fencedStops++
    }
    live.clear()
    watchers.clear()
    remoteWatches.clear()
    for (const w of spawnWaits.values()) w.resolve(null)
    spawnWaits.clear()
  }
  const isFenced = () => {
    if (!fenced && link?.fenced()) fence()
    return fenced
  }
  link?.onFenced(fence)

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
    // a fenced node starts nothing: NO_PID reaches no one
    if (isFenced()) return 0n
    let s = slot
    if (s === undefined) {
      s = slotOf(c64('node_pid', BigInt(node), BigInt(nextLocal++), 0n))
    }
    const gen = nextGen(gens.get(s) ?? 0n)
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
      alarm: false,
      bound: spec.turnMaxSeconds || TURN_MAX_SECONDS,
      waitMs: -1,
      longAt: 0,
    }
    if (tel !== undefined) {
      p.k = tel.kind(spec.kind ?? spec.name)
      tel.spawned(p)
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
      tel?.dropped(undefined, false)
      return D_DROPPED_DEAD
    }
    if (remote(pid)) {
      const to = nodeOf(pid)
      // a fenced node reaches no other node
      const d = c('send_remote', flag(!!link && !isFenced() && link.up(to)))
      if (d !== D_QUEUED || !link) {
        stats.deadLetters++
        tel?.dropped(undefined, false)
        return d
      }
      stats.remoteSent++
      link.carry(to, { kind: 'send', to: pid, msg, from }, incOf(pid))
      return d
    }
    const p = procOf(pid)
    const d = c('deliver', flag(!!p), u32(p?.box.length ?? 0), MAILBOX_CAP)
    if (d !== D_QUEUED || !p) {
      stats.deadLetters++
      tel?.dropped(p?.k, d === D_DROPPED_FULL)
      return d
    }
    if (tel !== undefined) {
      const k = p.k as KindStats
      p.box.push(tel.sample(k) ? new Stamped(msg, clock.now()) : msg)
      tel.queued(p)
    } else p.box.push(msg)
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

  /**
   * A new monitor reference on a local target. The card's count after a
   * monitor is the count before plus one (monitors_after_monitor), whoever
   * watches: a second monitor by the same watcher is a second entry.
   */
  const watch = (target: Pid, watcher: Pid): number => {
    const list = watchers.get(target) ?? []
    const ref = nextMonitorRef++
    const count = c('monitors_after_monitor', u32(list.length))
    list.push({ watcher, ref })
    if (list.length !== count)
      throw new Error(`monitor count ${list.length}, the card says ${count}`)
    watchers.set(target, list)
    return ref
  }

  /**
   * Monitor `target` from `watcher`. Returns the monitor's reference for
   * `unwatch` (0 when the DOWN was sent at once, or the target is remote).
   */
  function monitor(watcher: Pid, target: Pid): number {
    if (remote(target)) {
      const on = nodeOf(target)
      if (!link || isFenced() || !link.up(on)) {
        send(watcher, {
          kind: 'DOWN',
          pid: target,
          reason: c('remote_down', 0, 0, X_NOPROC),
        } as Down)
        return 0
      }
      const list = remoteWatches.get(on) ?? []
      list.push({ watcher, target })
      remoteWatches.set(on, list)
      link.carry(on, { kind: 'monitor', target, watcher }, incOf(target))
      return 0
    }
    if (!procOf(target)) {
      send(watcher, { kind: 'DOWN', pid: target, reason: X_NOPROC } as Down)
      return 0
    }
    return watch(target, watcher)
  }

  /** Drop one monitor by its reference. The flush is queen-actors-links.ts. */
  function unwatch(ref: number): void {
    for (const [target, list] of watchers) {
      const i = list.findIndex((w) => w.ref === ref)
      if (i < 0) continue
      list.splice(i, 1)
      if (list.length === 0) watchers.delete(target)
      return
    }
  }

  /** The end of a process. `quiet` is a stop its supervisor asked for. */
  function exit(pid: Pid, reason: number, quiet = false): void {
    if (remote(pid)) {
      if (!isFenced())
        link?.carry(
          nodeOf(pid),
          { kind: 'exit', target: pid, reason },
          incOf(pid),
        )
      return
    }
    const p = procOf(pid)
    if (!p) return
    live.delete(slotOf(p.pid))
    p.cancelKill?.()
    if (turnStop && p.turn && !p.turn.ended) stopTurn(p, p.turn, reason)
    stats.deadLetters += p.box.length
    tel?.exited(p)
    const downs = watchers.get(p.pid) ?? []
    watchers.delete(p.pid)
    for (const w of downs) send(w.watcher, downOf(p.pid, reason, w.ref))
    for (const told of exitListeners) told(p.pid, reason)
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

  // with telemetry, a step that has a turn to start runs as its actor's kind,
  // and so does every await of that turn: the decision log names the kind
  // behind each call. A step with nothing queued starts no turn.
  function step(p: Proc): void {
    if (tel === undefined || (p.box.length === 0 && p.lane === 0n)) stepOn(p)
    else tel.als.run((p.k as KindStats).kind, stepOn, p)
  }

  function stepOn(p: Proc): void {
    // a node that must fence runs no further turn (netlink.t27 section 3)
    if (link && isFenced()) return
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
        tel?.controlTurn(p, true)
        exit(p.pid, X_CRASH)
        return
      }
      tel?.controlTurn(p, false)
      schedule(p)
      return
    }
    if (lane !== LANE_DATA) return
    let msg = p.box.shift()
    let stampedAt = -1
    if (msg instanceof Stamped) {
      stampedAt = msg.at
      msg = msg.msg
    }
    const bound = p.spec.turnMaxSeconds ?? 0
    const t: Turn = { ended: false, held: false }
    const signal = p.abort?.signal ?? NEVER_ABORTED
    p.turn = t
    p.busy = true
    p.turnAt = clock.now()
    tel?.turnStart(p, stampedAt)
    p.cancelKill = clock.after((bound || TURN_MAX_SECONDS) * 1000, () => {
      const age = Math.ceil((clock.now() - p.turnAt) / 1000)
      if (current(p) && c('turn_signal', u32(age), u32(bound)) === X_KILL) {
        stats.killed++
        // Without turnStop: an isolated turn's work is stopped for real, a
        // loop turn's is not. With it, `exit` starts the turn's stop.
        let stopped = false
        if (
          !turnStop &&
          c('kill_effect', p.isolation) === KILL_STOPS &&
          t.stopWork
        ) {
          t.stopWork()
          stats.stopped++
          stopped = true
        }
        tel?.turnEnd(p, END_KILL, clock.now(), stopped)
        exit(p.pid, c('death_reason', X_KILL))
      }
    })
    // A TURN RUNS ONLY FOR A LIVE PROCESS. The message is taken here and the
    // turn runs a microtask later, or once isolated work returns. A stop in
    // between - a supervisor that gives up stops every child at once - left
    // the process dead and its receive still ran. Found by the simulation gate
    // (tests/sim, seed 3600507402, trios#1712 item 5).
    const run = () =>
      p.isolation !== ISO_LOOP && p.spec.isolated
        ? (() => {
            const work = p.spec.isolated.start(msg, signal)
            t.stopWork = work.stop
            return work.result.then((r) =>
              current(p) ? p.spec.receive(msg, p.pid, r, signal) : undefined,
            )
          })()
        : Promise.resolve().then(() =>
            current(p)
              ? p.spec.receive(msg, p.pid, undefined, signal)
              : undefined,
          )
    // with turnStop, everything the turn starts, down to a git command, sees
    // its signal; without it a turn costs what it did
    const turn = p.inTurn ? p.inTurn(run) : run()
    turn.then(
      () => {
        turnEnded(p, t)
        if (!current(p)) return
        tel?.turnEnd(p, END_OK, clock.now())
        p.cancelKill?.()
        p.turn = undefined
        p.busy = false
        schedule(p)
      },
      () => {
        turnEnded(p, t)
        if (!current(p)) return
        tel?.turnEnd(p, END_CRASH, clock.now())
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
      else watch(mail.target, mail.watcher)
    } else if (mail.kind === 'exit') {
      exit(mail.target, mail.reason)
    } else if (mail.kind === 'spawn') {
      if (isFenced()) return
      const make = kinds.get(mail.spawnKind)
      const pid = make ? spawn(make(mail.arg)) : null
      link.carry(
        mail.from,
        { kind: 'spawned', ref: mail.ref, pid },
        mail.fromInc,
      )
    } else if (mail.kind === 'spawned') {
      spawnWaits.get(mail.ref)?.resolve(mail.pid)
      spawnWaits.delete(mail.ref)
    }
  })
  // an incarnation that went down: what waited on it, and only that, ends
  // (netlink.t27 lost_with). Every monitor of a pid born in it hears
  // noconnection; every remote start sent to it resolves to no pid.
  link?.onNodeDown((down, downInc) => {
    const kept: Array<{ watcher: Pid; target: Pid }> = []
    for (const w of remoteWatches.get(down) ?? []) {
      if (!lostWith(nodeOf(w.target), incOf(w.target), down, downInc)) {
        kept.push(w)
        continue
      }
      send(w.watcher, {
        kind: 'DOWN',
        pid: w.target,
        reason: c('remote_down', 0, 0, X_NOPROC),
      } as Down)
    }
    if (kept.length > 0) remoteWatches.set(down, kept)
    else remoteWatches.delete(down)
    for (const [ref, w] of spawnWaits)
      if (lostWith(w.on, w.onInc, down, downInc)) {
        w.resolve(null)
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
      return Promise.resolve(make && !isFenced() ? spawn(make(arg)) : null)
    }
    if (!link || isFenced() || !link.up(on)) return Promise.resolve(null)
    const ref = nextRef++
    const onInc = link.incarnationOf(on)
    return new Promise((resolve) => {
      spawnWaits.set(ref, { on, onInc, resolve })
      link.carry(
        on,
        { kind: 'spawn', spawnKind, arg, ref, from: node, fromInc: inc },
        onInc,
      )
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
    unwatch,
    exit,
    alive: (pid: Pid) => !isFenced() && !!procOf(pid),
    /** A live process's mailbox, oldest first, or undefined. */
    mailbox: (pid: Pid): unknown[] | undefined => procOf(pid)?.box,
    /** Whether a live process is in a data turn. */
    busy: (pid: Pid): boolean => !!procOf(pid)?.busy,
    /** Be told of every local exit, after its DOWNs. Returns the unsubscribe. */
    onExit: (told: (pid: Pid, reason: number) => void): (() => void) => {
      exitListeners.add(told)
      return () => exitListeners.delete(told)
    },
    /** Whether a node is up as this node sees it; this node is, unless fenced. */
    up: (n: number) =>
      n === node ? !isFenced() : !isFenced() && !!link && link.up(n),
    /** This node's incarnation (netlink.t27 section 1); 0 with no link. */
    incarnation: inc,
    /** Whether this node fenced itself; it never comes back (FENCE_REJOINS). */
    fenced: isFenced,
    stats,
    clock,
    /** Whether a dying pid's turn is stopped (turn_stop.t27) or abandoned. */
    turnStop,
    /** Set with options.telemetry; GET /queen/actors/metrics reads it. */
    telemetry: tel,
  }
}

export type ActorSystem = ReturnType<typeof createActorSystem>

/** What a supervisor starts: an actor, or a supervisor below it. */
export interface Child {
  name: string
  /** What telemetry counts its restarts under; its name when unset. */
  kind?: string
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
  kind: spec.kind ?? spec.name,
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
      const inPeriod = () => {
        const now = clock.now()
        return restarts.filter(
          (t) =>
            c('in_period', u32((now - t) / 1000), u32(opts.periodSeconds)) !==
            0,
        ).length
      }
      const tel = sys.telemetry
      const sup = tel?.supervisor(opts.name, opts.maxRestarts, inPeriod)

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
      // with telemetry, the supervisor's decisions are logged under its name
      const exited = (i: number, reason: number) => {
        if (tel === undefined) exitedOn(i, reason)
        else tel.als.run(opts.name, exitedOn, i, reason)
      }
      const exitedOn = (i: number, reason: number) => {
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
          if (sup) tel?.gaveUp(sup)
          down = true
          stopAll()
          onExit(GIVE_UP_REASON)
          return
        }
        if (decision !== SUP_RESTART) return
        restarts.push(now)
        if (sup) tel?.restarted(sup, children[i].kind ?? children[i].name)
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
