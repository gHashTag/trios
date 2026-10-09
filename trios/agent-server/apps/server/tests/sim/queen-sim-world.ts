/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE SIMULATION WORLD (gHashTag/t27 specs/queen/simulation.t27, trios#1712
 * item 5).
 *
 * WHY. The benchmark that cleared the actor reviewer ran on a virtual clock
 * and a seed, and still missed the go-live hot loop: it had no unreadable
 * row, so ~184 visits a minute against 22 such rows were found in production
 * (t27#7851). This world draws the work, the interleaving and the faults from
 * one seed, checks the invariants after every event, and is run twice per
 * seed by the gate so that a run which logs differently is caught too.
 *
 * WHAT RUNS IS THE REAL RUNTIME: createActorSystem, supervisor, remoteChild,
 * createMemoryNet and the reviewer actors (reviewerTree, withWaitBackoff), on
 * the virtual clock. What is simulated is only what the runtime talks to: the
 * store (which rows wait, the lease), the review itself (its length and
 * fate), the bus, and the nodes of the pool.
 *
 * WHAT DECIDES. The simulation card (queen/simulation.wasm): every random
 * number is its sim_roll(seed, stream, index); each step's event is its
 * step_event, the time that passes its step_ms; which invariant fails the
 * gate, the hot-loop bound, the cap and the intensity checks are its
 * functions. performance.now, the one wall clock the runtime reads (the
 * slice), is replaced for the run by a counter the card drives. This file
 * holds the world's state, applies events and watches.
 *
 * HOW IT WATCHES. The reviewer and the pool are handed an observed system: a
 * copy of the runtime's whose spawn, send, exit and monitor log and check
 * before they delegate. Every spec spawned through it, and every kind started
 * on a node, gets a receive that checks its process is alive. Every mail that
 * crosses nodes carries a wire id, so mail still on the wire is known when a
 * pid is given out. The actors and reviewer cards are tapped, so their
 * decisions go into the run's log, and the supervisors' restart decisions are
 * counted on the harness's own clock.
 */

import {
  type ActorSpec,
  type ActorSystem,
  createActorSystem,
  type Down,
  type Mail,
  type NodeLink,
  type Pid,
  remoteChild,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  D_DROPPED_FULL,
  DOMAIN_MAX_RESTARTS,
  DOMAIN_PERIOD_SECONDS,
  MAILBOX_CAP,
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
  SUP_GIVE_UP,
  SUP_RESTART,
  X_KILLED,
} from '../../src/api/services/queen-actors-card.gen'
import { createMemoryNet } from '../../src/api/services/queen-actors-net'
import {
  type CardWasm,
  loadCardWasm,
} from '../../src/api/services/queen-card-wasm'
import { reviewerTree } from '../../src/api/services/queen-review-actors'
import {
  type Judged,
  type ReviewerDeps,
  withWaitBackoff,
} from '../../src/api/services/queen-review-loop'
import {
  REVIEWER_CONCURRENCY,
  WAIT_REVISIT_BASE_SECONDS,
} from '../../src/api/services/queen-reviewer-card.gen'
import * as S from '../../src/api/services/queen-simulation-card.gen'
import { VirtualClock } from '../api/queen-virtual-clock'

export const SIMULATION_CARD = 'queen/simulation.wasm'
const ACTORS_CARD = 'queen/actors.wasm'
const REVIEWER_CARD = 'queen/reviewer.wasm'

const sim = () => loadCardWasm(SIMULATION_CARD)
const u32 = (n: number) => n >>> 0
export const simRoll = (seed: number, stream: number, index: number) =>
  u32(sim().call('sim_roll', u32(seed), stream, u32(index)))
export const runSeed = (base: number, i: number) =>
  u32(sim().call('run_seed', u32(base), u32(i)))
export const pickBetween = (roll: number, lo: number, hi: number) =>
  u32(sim().call('pick_between', u32(roll), u32(lo), u32(hi)))
const stepEvent = (roll: number) => sim().call('step_event', u32(roll))
const stepMs = (roll: number) => u32(sim().call('step_ms', u32(roll)))
const mailboxBurst = (cap: number) => u32(sim().call('mailbox_burst', cap))
const hotLoop = (visits: number, base: number) =>
  sim().call('hot_loop', visits, base) !== 0
const overCap = (atOnce: number, cap: number) =>
  sim().call('over_cap', atOnce, cap) !== 0
const intensityKept = (inPeriod: number, max: number) =>
  sim().call('intensity_kept', inPeriod, max) !== 0
export const answerLost = (roll: number) =>
  sim().call('answer_lost', u32(roll)) !== 0
export const failsGate = (inv: number) => sim().call('fails_gate', inv) !== 0
export const mustReach = (state: number) =>
  sim().call('must_reach', state) !== 0

export const INVARIANT_NAMES: Record<number, string> = {
  [S.INV_PID_REUSED]: 'a pid was reused while mail to it was on the wire',
  [S.INV_DELIVERED_AFTER_EXIT]: 'a turn ran for a process that had exited',
  [S.INV_INTENSITY]: 'a supervisor restarted past its intensity',
  [S.INV_OVER_CAP]: 'concurrency exceeded its cap',
  [S.INV_RESTART_WITHOUT_EXIT]:
    'a child started again before its previous incarnation ended',
  [S.INV_HOT_LOOP]: 'a row was visited in a hot loop',
  [S.INV_RUNS_DIFFER]: 'two runs of one seed logged differently',
  [S.INV_RARE_STATE_MISSED]: 'a rare state was never reached',
  [S.INV_TAKEN_NOT_HANDLED]:
    'a node took mail off the wire and never handled it',
}
export const RARE_NAMES: Record<number, string> = {
  [S.RARE_MAILBOX_FULL]: 'mailbox full',
  [S.RARE_INTENSITY_SPENT]: 'intensity spent (a supervisor gave up)',
  [S.RARE_TURN_KILLED]: 'turn killed at its bound',
  [S.RARE_MAIL_TO_LOST_NODE]: 'mail lost on the wire to a lost node',
  [S.RARE_WAIT_BACKED_OFF]: 'a row held back by the wait backoff',
  [S.RARE_SLICE_YIELDED]: 'a scheduler yielded its slice',
  [S.RARE_MOVED_NODE]: 'a child restarted on another node',
}

/** A node link the world can crash and bring back, over some wire. */
export interface Transport {
  /** The link a (re)started node `n` runs on. */
  link(n: number): NodeLink | Promise<NodeLink>
  /** The node stops: no heartbeat, no mail in or out. */
  crash(n: number): void
  /** Called at the end of each step: the transport settles what it owes. */
  settle?(): Promise<void>
  stop?(): Promise<void>
}

export interface WorldOptions {
  /** Re-create the go-live defect: the reviewer without the wait backoff. */
  noWaitBackoff?: boolean
  /** A transport other than the memory net (the PgLink store model). */
  transport?: (world: SimWorld) => Transport
  /** Keep every log line of this step, for a determinism diff. */
  captureStep?: number
  /**
   * Invariants this run logs and counts but does not stop on: a known
   * defect, so a run can look past it for another. Never set by the gate.
   */
  known?: number[]
}

export interface Violation {
  inv: number
  step: number
  t: number
  text: string
  subjects: string[]
}

export interface RunResult {
  seed: number
  steps: number
  violation: Violation | null
  stepHashes: number[]
  hash: number
  rare: Set<number>
  captured: string[]
  /** Card calls made while the world quiesced after its steps. */
  afterTail: string[]
  /** The last log lines of the run, for a divergence the replay does not repeat. */
  lines: string[]
  counts: Record<string, number>
  tail: string[]
  ms: number
}

interface Incarnation {
  id: number
  name: string
  node: number
  pid: Pid
  /** The pool child an echo worker runs for. */
  child?: string
  exited: boolean
  inTurn: number
}

type Fate = 'ok' | 'wait' | 'refused' | 'stall' | 'crash'

interface Row {
  issue: number
  unreadableUntil: number
  decided: boolean
}

interface LogLine {
  n: number
  t: number
  step: number
  text: string
  subjects: string[]
}

const RING = 4000
const fmtPid = (pid: Pid) => {
  const slot = pid >> 32n
  return `${pid}(n${Number(slot >> 20n)}.${Number(slot & 0xfffffn)}.g${Number(pid & 0xffffffffn)})`
}

/**
 * One seed's world. `run` plays STEPS_PER_RUN steps (or `steps`) and the
 * quiet tail, and stops at the first violation.
 */
export class SimWorld {
  readonly clock = new VirtualClock()
  step = -1
  violation: Violation | null = null
  readonly rare = new Set<number>()
  readonly counts: Record<string, number> = {}
  private hash = 0x811c9dc5
  private finalHash = 0
  private lineNo = 0
  /** Set when the run has ended: what still runs afterwards logs nothing. */
  private closed = false
  private readonly stepHashes: number[] = []
  private readonly ring: LogLine[] = []
  private readonly captured: string[] = []
  private readonly names = new Map<string, number>()
  private tapping = false
  private quiescing = false
  /** What ran while the world quiesced, for a divergence report. */
  private readonly afterTail: string[] = []

  // the runtime, as the world sees it
  private readonly systems = new Map<number, ActorSystem>()
  private readonly rawSystems: ActorSystem[] = []
  private readonly crashed = new Set<number>()
  private transport!: Transport
  private nextInc = 1
  private readonly live = new Set<Incarnation>()
  private readonly byPid = new Map<Pid, Incarnation>()
  private readonly lastByName = new Map<string, Incarnation>()
  private readonly lastByChild = new Map<string, Incarnation>()
  // mail on the wire, by the pid it is addressed to
  private nextWire = 1
  private readonly wire = new Map<
    number,
    { pid?: Pid; intended?: number; toNode: number }
  >()
  private readonly wireByPid = new Map<Pid, Set<number>>()

  // the reviewer's world
  private readonly rows = new Map<number, Row>()
  private nextIssue = 1000
  private attempts = 0
  private dbCalls = 0
  private pendingHung = 0
  private pendingCrash = 0
  private refusedUntil = 0
  private leaseLostUntil = 0
  private readonly reviewsOf = new Map<number, number>()
  private readonly keysHeld = new Map<number, number>()
  private readonly visits = new Map<number, number[]>()
  private wake: () => void = () => {}

  // the pool's world
  private registrar: Pid = 0n
  private readonly known = new Map<string, Pid>()
  private readonly svc = new Map<number, Pid>()
  private nextJob = 1

  // restart decisions per supervisor, by its maximum
  private readonly decisions = new Map<number, number[]>()
  private lastExitMax = -1
  private readonly periods = new Map<number, number>([
    [DOMAIN_MAX_RESTARTS, DOMAIN_PERIOD_SECONDS],
    [ROOT_MAX_RESTARTS, ROOT_PERIOD_SECONDS],
    [S.POOL_MAX_RESTARTS, S.POOL_PERIOD_SECONDS],
  ])

  constructor(
    readonly seed: number,
    readonly options: WorldOptions = {},
  ) {}

  roll(stream: number, index: number): number {
    return simRoll(this.seed, stream, index)
  }

  // --- the log ------------------------------------------------------------

  private mix(n: number) {
    if (this.closed) return
    this.hash = Math.imul(this.hash ^ (n | 0), 16777619)
  }
  private mixText(s: string) {
    for (let i = 0; i < s.length; i++) this.mix(s.charCodeAt(i))
  }
  private nameId(name: string): number {
    let id = this.names.get(name)
    if (id === undefined) {
      id = this.names.size + 1
      this.names.set(name, id)
    }
    return id
  }
  log(text: string, ...subjects: string[]) {
    if (this.closed) return
    this.mixText(text)
    const line = {
      n: this.lineNo++,
      t: this.clock.now(),
      step: this.step,
      text,
      subjects,
    }
    this.ring.push(line)
    if (this.ring.length > RING) this.ring.splice(0, this.ring.length - RING)
    if (this.options.captureStep === this.step)
      this.captured.push(`t=${line.t} ${text}`)
  }

  fail(inv: number, text: string, ...subjects: string[]) {
    if (this.closed || this.violation || !failsGate(inv)) return
    if (this.options.known?.includes(inv)) {
      this.count(`known${inv}`)
      this.log(`known defect ${INVARIANT_NAMES[inv]}: ${text}`, ...subjects)
      return
    }
    this.violation = {
      inv,
      step: this.step,
      t: this.clock.now(),
      text,
      subjects,
    }
    this.log(`VIOLATION ${INVARIANT_NAMES[inv]}: ${text}`, ...subjects)
  }

  count(key: string) {
    this.counts[key] = (this.counts[key] ?? 0) + 1
  }

  /** The last lines that name a subject of the violation: the minimal tail. */
  tail(): string[] {
    const v = this.violation
    const fmt = (l: LogLine) =>
      `  step ${l.step} t=${(l.t / 1000).toFixed(3)}s ${l.text}`
    if (!v) return this.ring.slice(-S.LOG_TAIL_LINES).map(fmt)
    const named = this.ring.filter((l) =>
      l.subjects.some((s) => v.subjects.includes(s)),
    )
    return named.slice(-S.LOG_TAIL_LINES).map(fmt)
  }

  // --- the card tap: every decision of the actors and reviewer cards -------

  private tap(
    file: string,
    onCall: (name: string, a: unknown[], r: unknown) => void,
  ) {
    const card = loadCardWasm(file) as CardWasm
    const call = card.call
    const call64 = card.call64
    card.call = (name: string, ...a: number[]) => {
      const r = call(name, ...a)
      if (!this.tapping) onCall(name, a, r)
      return r
    }
    card.call64 = (name: string, ...a: Array<number | bigint>) => {
      const r = call64(name, ...a)
      if (!this.tapping) onCall(name, a, r)
      return r
    }
    return () => {
      card.call = call
      card.call64 = call64
    }
  }

  private journal(name: string, a: unknown[], r: unknown) {
    this.mix(this.nameId(name))
    for (const v of a) {
      if (typeof v === 'bigint') {
        this.mix(Number(v & 0xffffffffn))
        this.mix(Number((v >> 32n) & 0xffffffffn))
      } else this.mix(v as number)
    }
    if (typeof r === 'bigint') this.mix(Number(r & 0xffffffffn))
    else this.mix(r as number)
    if (this.options.captureStep === this.step)
      this.captured.push(`card ${name}(${a.join(',')}) = ${r}`)
    if (this.quiescing && this.afterTail.length < 200)
      this.afterTail.push(`card ${name}(${a.join(',')}) = ${r}`)
  }

  private onActorsCall(name: string, a: unknown[], r: unknown) {
    this.journal(name, a, r)
    if (name === 'on_child_exit') this.lastExitMax = a[3] as number
    else if (name === 'with_backoff') this.decided(r as number)
  }

  private onReviewerCall(name: string, a: unknown[], r: unknown) {
    this.journal(name, a, r)
    if (name === 'due_again' && r === 0) this.rare.add(S.RARE_WAIT_BACKED_OFF)
  }

  /** A supervisor decided; count its restarts in its period on our clock. */
  private decided(decision: number) {
    const max = this.lastExitMax
    const period = this.periods.get(max)
    if (period === undefined) return
    const now = this.clock.now()
    const list = this.decisions.get(max) ?? []
    if (decision === SUP_GIVE_UP) {
      this.rare.add(S.RARE_INTENSITY_SPENT)
      this.log(`supervisor (max ${max}) gave up`)
      // that supervisor is gone; the one that replaces it counts afresh
      this.decisions.set(max, [])
      return
    }
    if (decision !== SUP_RESTART) return
    this.tapping = true
    const kept = [...list, now].filter(
      (t) =>
        loadCardWasm(ACTORS_CARD).call(
          'in_period',
          u32((now - t) / 1000),
          period,
        ) !== 0,
    )
    this.tapping = false
    this.decisions.set(max, kept)
    this.log(`supervisor (max ${max}) restarts: ${kept.length} in ${period} s`)
    if (!intensityKept(kept.length, max))
      this.fail(
        S.INV_INTENSITY,
        `supervisor with max ${max} in ${period} s decided ${kept.length} restarts in its period`,
      )
  }

  // --- the observed system --------------------------------------------------

  private incOf(pid: Pid): Incarnation | undefined {
    return this.byPid.get(pid)
  }

  private alive(inc: Incarnation): boolean {
    if (inc.exited || this.crashed.has(inc.node)) return false
    return this.systems.get(inc.node)?.alive(inc.pid) ?? false
  }

  private exited(inc: Incarnation | undefined, why: string) {
    if (!inc || inc.exited) return
    inc.exited = true
    this.live.delete(inc)
    this.log(`exit ${inc.name} ${fmtPid(inc.pid)} (${why})`, String(inc.pid))
  }

  /** A spec, watched: its turns check that their process lives. */
  private watch<M>(
    spec: ActorSpec<M>,
    node: number,
    child?: string,
  ): { spec: ActorSpec<M>; inc: Incarnation } {
    const inc: Incarnation = {
      id: this.nextInc++,
      name: spec.name,
      node,
      pid: 0n,
      child,
      exited: false,
      inTurn: 0,
    }
    const turnCheck = (self: Pid, what: string) => {
      if (inc.exited || !this.systems.get(node)?.alive(self))
        this.fail(
          S.INV_DELIVERED_AFTER_EXIT,
          `${what} ran for ${inc.name} ${fmtPid(self)} after it exited`,
          String(self),
        )
    }
    const watched: ActorSpec<M> = {
      ...spec,
      init: (self) => {
        inc.pid = self
        this.born(inc)
        spec.init?.(self)
      },
      receive: async (msg, self, result) => {
        turnCheck(self, 'a turn')
        const m = msg as { intended?: number; kind?: string }
        if (
          m &&
          typeof m === 'object' &&
          m.intended !== undefined &&
          m.intended !== inc.id
        )
          this.fail(
            S.INV_PID_REUSED,
            `mail for incarnation ${m.intended} of ${fmtPid(self)} reached incarnation ${inc.id} (${inc.name})`,
            String(self),
          )
        inc.inTurn++
        try {
          return await spec.receive(msg, self, result)
        } finally {
          inc.inTurn--
        }
      },
      control: spec.control
        ? (tag, self) => {
            turnCheck(self, 'a control turn')
            spec.control?.(tag, self)
          }
        : undefined,
    }
    return { spec: watched, inc }
  }

  /** A process got its pid: no mail to an earlier holder may be on the wire. */
  private born(inc: Incarnation) {
    const flying = this.wireByPid.get(inc.pid)
    if (flying)
      for (const id of flying) {
        const w = this.wire.get(id)
        if (w && w.intended !== undefined && w.intended !== inc.id)
          this.fail(
            S.INV_PID_REUSED,
            `${inc.name} got pid ${fmtPid(inc.pid)} while mail ${id} to incarnation ${w.intended} was on the wire`,
            String(inc.pid),
          )
      }
    // a child starts again only once the previous one ended
    const key = inc.child ?? `${inc.node}:${inc.name}`
    const prev = inc.child
      ? this.lastByChild.get(key)
      : this.lastByName.get(key)
    if (prev && prev !== inc) {
      if (!prev.exited && this.alive(prev))
        this.fail(
          S.INV_RESTART_WITHOUT_EXIT,
          `${inc.name} started as ${fmtPid(inc.pid)} while ${fmtPid(prev.pid)} still lived`,
          String(inc.pid),
          String(prev.pid),
        )
      else if (!prev.exited && !inc.child)
        this.fail(
          S.INV_RESTART_WITHOUT_EXIT,
          `${inc.name} started as ${fmtPid(inc.pid)} before the exit of ${fmtPid(prev.pid)} was seen`,
          String(inc.pid),
          String(prev.pid),
        )
      if (inc.child && prev.node !== inc.node) this.rare.add(S.RARE_MOVED_NODE)
    }
    if (inc.child) this.lastByChild.set(key, inc)
    else this.lastByName.set(key, inc)
    this.byPid.set(inc.pid, inc)
    this.live.add(inc)
    this.log(
      `spawn ${inc.name} ${fmtPid(inc.pid)} on node ${inc.node} (incarnation ${inc.id})`,
      String(inc.pid),
    )
  }

  /** The runtime of node `n`, observed. */
  private observe(raw: ActorSystem): ActorSystem {
    const node = raw.node
    return {
      ...raw,
      spawn: <M>(
        spec: ActorSpec<M>,
        onExit: (reason: number) => void = () => {},
        slot?: bigint,
      ): Pid => {
        const { spec: watched, inc } = this.watch(spec, node)
        return raw.spawn(
          watched,
          (reason) => {
            if (reason === X_KILLED) this.rare.add(S.RARE_TURN_KILLED)
            this.exited(inc, `reason ${reason}`)
            onExit(reason)
          },
          slot,
        )
      },
      send: (pid: Pid, msg: unknown, from?: Pid): number => {
        const d = raw.send(pid, msg, from)
        const kind = (msg as { kind?: string })?.kind ?? 'msg'
        if (d === D_DROPPED_FULL) {
          this.rare.add(S.RARE_MAILBOX_FULL)
          this.count('droppedFull')
        }
        this.log(`send ${kind} -> ${fmtPid(pid)} = ${d}`, String(pid))
        return d
      },
      exit: (pid: Pid, reason: number, quiet = false) => {
        const inc = this.incOf(pid)
        const was = inc ? this.alive(inc) : false
        raw.exit(pid, reason, quiet)
        if (inc && was && !raw.alive(pid) && inc.node === node)
          this.exited(inc, `stopped, reason ${reason}`)
      },
      monitor: (watcher: Pid, target: Pid) => {
        this.log(
          `monitor ${fmtPid(watcher)} -> ${fmtPid(target)}`,
          String(target),
        )
        raw.monitor(watcher, target)
      },
      spawnOn: (on: number, kind: string, arg: unknown) => {
        this.log(`spawnOn node ${on} ${kind}`)
        return raw.spawnOn(on, kind, arg)
      },
    }
  }

  /** A node's link, with every crossing mail on the wire until it lands. */
  private wired(link: NodeLink, settleMs: number | null): NodeLink {
    return {
      ...link,
      node: link.node,
      up: (n) => link.up(n),
      carry: (to, mail) => {
        const id = this.nextWire++
        const pid =
          mail.kind === 'send'
            ? mail.to
            : mail.kind === 'monitor' || mail.kind === 'exit'
              ? mail.target
              : undefined
        const inc = pid === undefined ? undefined : this.incOf(pid)
        this.wire.set(id, { pid, intended: inc?.id, toNode: to })
        if (pid !== undefined) {
          const set = this.wireByPid.get(pid) ?? new Set<number>()
          set.add(id)
          this.wireByPid.set(pid, set)
        }
        this.log(
          `wire ${id} ${mail.kind} node ${link.node} -> ${to}${pid === undefined ? '' : ` for ${fmtPid(pid)}`}`,
          `wire ${id}`,
          ...(pid === undefined ? [] : [String(pid)]),
        )
        // the wire id rides along; the runtime reads only the fields it knows
        link.carry(to, { ...mail, simWire: id } as unknown as Mail)
        // the memory net lands or loses a mail after its latency; a store
        // model settles its own wire
        if (settleMs !== null)
          this.clock.after(settleMs, () => this.landed(id, false))
      },
      onMail: (h) =>
        link.onMail((mail) => {
          this.landed((mail as { simWire?: number }).simWire, true)
          h(mail)
        }),
      onNodeDown: (h) => link.onNodeDown(h),
    }
  }

  /** Whether a mail is still on the wire: neither handed to a node nor lost. */
  onWire(id: number): boolean {
    return this.wire.has(id)
  }

  /** A mail left the wire: handed to its node, or lost. */
  landed(id: number | undefined, arrived: boolean) {
    if (id === undefined) return
    const w = this.wire.get(id)
    if (!w) return
    this.wire.delete(id)
    if (w.pid !== undefined) {
      const set = this.wireByPid.get(w.pid)
      set?.delete(id)
      if (set && set.size === 0) this.wireByPid.delete(w.pid)
    }
    if (!arrived) {
      if (this.crashed.has(w.toNode)) this.rare.add(S.RARE_MAIL_TO_LOST_NODE)
      this.count('lostOnWire')
      this.log(`wire ${id} lost`, `wire ${id}`)
    }
  }

  // --- the nodes ------------------------------------------------------------

  private async bootNode(n: number) {
    const link = this.wired(
      await this.transport.link(n),
      this.options.transport ? null : S.NET_LATENCY_MS,
    )
    const raw = createActorSystem(this.clock, { node: n, link })
    this.rawSystems.push(raw)
    const sys = this.observe(raw)
    this.systems.set(n, sys)
    this.crashed.delete(n)
    this.log(`node ${n} up`)
    if (n === 0) return sys
    raw.registerKind('echo', (arg: { registrar: Pid; child: string }) => {
      const { spec } = this.watch(
        {
          name: `echo-${arg.child}`,
          init: (self: Pid) => {
            sys.send(
              arg.registrar,
              { kind: 'up', pid: self, child: arg.child },
              self,
            )
          },
          receive: async (m: { kind: string; id?: number }, self: Pid) => {
            if (m.kind !== 'job') return
            const ms = pickBetween(
              this.roll(S.STREAM_DURATION, m.id ?? 0),
              0,
              S.JOB_MAX_MS,
            )
            await new Promise<void>((r) => this.clock.after(ms, r))
            sys.send(arg.registrar, { kind: 'done', id: m.id }, self)
          },
        },
        n,
        arg.child,
      )
      return spec as ActorSpec<unknown>
    })
    // every node starts its own service at boot: the first local slot
    this.svc.set(
      n,
      sys.spawn<{ kind: string }>({ name: `svc-${n}`, receive: () => {} }),
    )
    return sys
  }

  private crashNode(n: number) {
    if (this.crashed.has(n) || n === 0) return
    const sys = this.systems.get(n)
    this.crashed.add(n)
    this.transport.crash(n)
    this.log(`node ${n} lost`)
    // the process is gone: its actors with it, quietly (nothing leaves the node)
    for (const inc of [...this.live])
      if (inc.node === n) {
        sys?.exit(inc.pid, X_KILLED, true)
        this.exited(inc, 'node lost')
      }
    const back = pickBetween(
      this.roll(S.STREAM_DURATION, this.step),
      S.NODE_RESTART_MIN_SECONDS,
      S.NODE_RESTART_MAX_SECONDS,
    )
    this.clock.after(back * 1000, () => {
      void this.bootNode(n)
    })
  }

  // --- the reviewer ---------------------------------------------------------

  private db(): Promise<void> {
    const ms = pickBetween(
      this.roll(S.STREAM_LATENCY, this.dbCalls++),
      0,
      S.DB_MAX_MS,
    )
    return new Promise((r) => this.clock.after(ms, r))
  }

  private reviewerDeps(): ReviewerDeps {
    const base: ReviewerDeps = {
      holdsLease: async () => {
        await this.db()
        return this.clock.now() >= this.leaseLostUntil
      },
      waiting: async () => {
        await this.db()
        return [...this.rows.values()]
          .filter((r) => !r.decided)
          .map((r) => r.issue)
      },
      reviewOne: (issue, reservedKeys, onLane) =>
        this.reviewOne(issue, reservedKeys, onLane),
    }
    return this.options.noWaitBackoff
      ? base
      : withWaitBackoff(base, this.clock.now)
  }

  private liveWorkerTurns(): number {
    let n = 0
    for (const inc of this.live)
      if (
        inc.node === 0 &&
        inc.name.startsWith('reviewer-worker-') &&
        inc.inTurn > 0
      )
        n++
    return n
  }

  /** The hot loop: this row's visits in the last minute, against the card's bound. */
  private visit(issue: number, now: number) {
    const seen = (this.visits.get(issue) ?? []).filter((t) => now - t < 60_000)
    seen.push(now)
    this.visits.set(issue, seen)
    this.count('visits')
    this.counts.maxRowVisitsPerMinute = Math.max(
      this.counts.maxRowVisitsPerMinute ?? 0,
      seen.length,
    )
    if (hotLoop(seen.length, WAIT_REVISIT_BASE_SECONDS))
      this.fail(
        S.INV_HOT_LOOP,
        `row #${issue} visited ${seen.length} times in a minute`,
        `#${issue}`,
      )
  }

  /** Concurrency: live review turns against the cap, and one row once. */
  private startReview(issue: number) {
    const turns = this.liveWorkerTurns()
    if (overCap(turns, REVIEWER_CONCURRENCY))
      this.fail(S.INV_OVER_CAP, `${turns} review turns at once`, `#${issue}`)
    const sameRow = (this.reviewsOf.get(issue) ?? 0) + 1
    this.reviewsOf.set(issue, sameRow)
    if (overCap(sameRow, 1))
      this.fail(
        S.INV_OVER_CAP,
        `row #${issue} reviewed ${sameRow} times at once`,
        `#${issue}`,
      )
  }

  /** What this attempt will do: a fault queued for it, the provider, or the row. */
  private fateOf(row: Row | undefined, now: number): Fate {
    if (this.pendingHung > 0) {
      this.pendingHung--
      return 'stall'
    }
    if (this.pendingCrash > 0) {
      this.pendingCrash--
      return 'crash'
    }
    if (now < this.refusedUntil) return 'refused'
    if (!row || row.unreadableUntil > now) return 'wait'
    return 'ok'
  }

  /**
   * The lane a review takes: one the runtime does not hold for a sibling
   * (reservedKeys), as the review sweep picks it. Whether it is really free
   * is the invariant's question, not the picker's.
   */
  private bookLane(
    issue: number,
    reservedKeys: () => number[],
    onLane: (keyIndex: number | undefined) => void,
  ): number | undefined {
    const taken = new Set(reservedKeys())
    let key: number | undefined
    for (let k = 0; k < S.REVIEW_KEYS && key === undefined; k++)
      if (!taken.has(k)) key = k
    if (key === undefined) return undefined
    const held = (this.keysHeld.get(key) ?? 0) + 1
    this.keysHeld.set(key, held)
    if (overCap(held, 1))
      this.fail(
        S.INV_OVER_CAP,
        `lane ${key} booked ${held} times at once`,
        `#${issue}`,
      )
    onLane(key)
    return key
  }

  private lengthOf(fate: Fate, attempt: number): number {
    if (fate === 'stall') return S.STALL_SECONDS * 1000
    if (fate === 'refused') return S.REFUSED_REVIEW_MS
    if (fate === 'wait') return S.WAIT_ANSWER_MS
    const ms = pickBetween(
      this.roll(S.STREAM_ATTEMPT, attempt),
      S.REVIEW_MIN_SECONDS * 1000,
      S.REVIEW_MAX_SECONDS * 1000,
    )
    // a crash comes 30% of the way into the review, as in the benchmark
    return fate === 'crash' ? Math.floor((ms * 3) / 10) : ms
  }

  private endReview(issue: number, key: number | undefined) {
    this.reviewsOf.set(issue, (this.reviewsOf.get(issue) ?? 1) - 1)
    if (key === undefined) return
    const left = (this.keysHeld.get(key) ?? 1) - 1
    if (left > 0) this.keysHeld.set(key, left)
    else this.keysHeld.delete(key)
  }

  private async reviewOne(
    issue: number,
    reservedKeys: () => number[],
    onLane: (keyIndex: number | undefined) => void,
  ): Promise<Judged> {
    const now = this.clock.now()
    const row = this.rows.get(issue)
    this.visit(issue, now)
    this.startReview(issue)
    const a = this.attempts++
    let fate = this.fateOf(row, now)
    // an unreadable row answers before any lane is taken; no free lane is a refusal
    const key =
      fate === 'wait' ? undefined : this.bookLane(issue, reservedKeys, onLane)
    if (fate !== 'wait' && key === undefined) fate = 'refused'
    const ms = this.lengthOf(fate, a)
    this.log(
      `review #${issue} attempt ${a}: ${fate}, ${ms} ms, lane ${key ?? '-'}`,
      `#${issue}`,
    )
    try {
      await new Promise<void>((r) => this.clock.after(ms, r))
    } finally {
      this.endReview(issue, key)
    }
    if (fate === 'crash') {
      this.log(`review #${issue} attempt ${a} threw`, `#${issue}`)
      throw new Error('review crashed (simulated)')
    }
    if (fate === 'ok' && row) row.decided = true
    const verdict = fate === 'ok' ? 'accept' : 'wait'
    this.log(`review #${issue} attempt ${a} ended: ${verdict}`, `#${issue}`)
    return { acted: [`#${issue}:${verdict}`], strays: [], tally: [] }
  }

  private startReviewer(sys0: ActorSystem) {
    const r = reviewerTree(sys0, this.reviewerDeps())
    this.wake = r.wake
    supervisor(
      sys0,
      {
        name: 'queen-root',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: ROOT_MAX_RESTARTS,
        periodSeconds: ROOT_PERIOD_SECONDS,
      },
      [r.tree],
    ).start(() => {
      // reviewing goes back to the round; the next deploy starts it again
      this.wake = () => {}
      this.count('reviewerGaveUp')
      this.log('reviewer root gave up')
      this.clock.after(S.GIVE_UP_REARM_SECONDS * 1000, () =>
        this.startReviewer(sys0),
      )
    })
    r.wake()
  }

  // --- the pool -------------------------------------------------------------

  private startPool(sys0: ActorSystem) {
    if (this.registrar === 0n)
      this.registrar = sys0.spawn<
        { kind: string; pid?: Pid; child?: string } | Down
      >({
        name: 'registrar',
        receive: (m, self) => {
          if (m.kind === 'up') {
            const up = m as { pid: Pid; child: string }
            this.known.set(up.child, up.pid)
            sys0.monitor(self, up.pid)
          } else if (m.kind === 'DOWN') {
            for (const [c, p] of this.known)
              if (p === (m as Down).pid) this.known.delete(c)
          } else if (m.kind === 'done') this.count('jobsDone')
        },
      })
    // the placer's count: where each child's latest start went
    const assigned = new Map<string, number | null>()
    const placedOn = (n: number) => {
      let k = 0
      for (const at of assigned.values()) if (at === n) k++
      return k
    }
    const nodes = Array.from({ length: S.POOL_NODES }, (_, i) => i + 1)
    supervisor(
      sys0,
      {
        name: 'pool',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: S.POOL_MAX_RESTARTS,
        periodSeconds: S.POOL_PERIOD_SECONDS,
      },
      Array.from({ length: S.POOL_WORKERS }, (_, i) =>
        remoteChild(sys0, {
          name: `w${i}`,
          kind: 'echo',
          arg: { registrar: this.registrar, child: `w${i}` },
          nodes: () => nodes,
          room: (n) => S.POOL_WORKERS - placedOn(n),
          placed: (n) => assigned.set(`w${i}`, n),
        }),
      ),
    ).start(() => {
      this.count('poolGaveUp')
      this.log('pool gave up')
      this.clock.after(S.GIVE_UP_REARM_SECONDS * 1000, () =>
        this.startPool(sys0),
      )
    })
  }

  // --- the steps --------------------------------------------------------------

  private apply(event: number, step: number, sys0: ActorSystem) {
    const now = this.clock.now()
    const r = () => this.roll(S.STREAM_TARGET, step)
    const span = (lo: number, hi: number) =>
      pickBetween(this.roll(S.STREAM_DURATION, step), lo, hi) * 1000
    this.count(`event${event}`)
    switch (event) {
      case S.E_ARRIVAL: {
        const issue = this.nextIssue++
        this.rows.set(issue, { issue, unreadableUntil: 0, decided: false })
        this.log(`arrival #${issue}`, `#${issue}`)
        this.wake()
        return
      }
      case S.F_UNREADABLE_ROW: {
        const issue = this.nextIssue++
        const until =
          now + span(S.UNREADABLE_MIN_SECONDS, S.UNREADABLE_MAX_SECONDS)
        this.rows.set(issue, { issue, unreadableUntil: until, decided: false })
        this.log(`arrival #${issue}, unreadable until ${until}`, `#${issue}`)
        this.wake()
        return
      }
      case S.E_JOB: {
        const targets = [...this.known.values(), ...this.svc.values()]
        if (targets.length === 0) return
        const to = targets[r() % targets.length]
        const id = this.nextJob++
        sys0.send(to, { kind: 'job', id, intended: this.incOf(to)?.id })
        return
      }
      case S.F_NODE_LOSS:
        this.crashNode(1 + (r() % S.POOL_NODES))
        return
      case S.F_HUNG_TURN:
        this.pendingHung++
        this.log('the next review hangs')
        return
      case S.F_CRASH:
        this.pendingCrash++
        this.log('the next review throws')
        return
      case S.F_MAILBOX_OVERFLOW: {
        const burst = mailboxBurst(MAILBOX_CAP)
        this.log(`bus burst of ${burst} wake-ups`)
        for (let i = 0; i < burst; i++) this.wake()
        return
      }
      case S.F_PROVIDER_429:
        this.refusedUntil = Math.max(
          this.refusedUntil,
          now + span(S.PROVIDER_429_MIN_SECONDS, S.PROVIDER_429_MAX_SECONDS),
        )
        this.log(`provider 429 until ${this.refusedUntil}`)
        return
      case S.F_STALE_LEASE:
        this.leaseLostUntil = Math.max(
          this.leaseLostUntil,
          now + span(S.STALE_LEASE_MIN_SECONDS, S.STALE_LEASE_MAX_SECONDS),
        )
        this.log(`lease lost until ${this.leaseLostUntil}`)
        return
      default:
        return
    }
  }

  /** Run the clock until `p` settles: a store's answers come on the clock. */
  private async drive<T>(p: Promise<T>): Promise<T> {
    let done = false
    const out = p.finally(() => {
      done = true
    })
    for (let ms = 0; !done && ms < 60_000; ms++)
      await this.clock.runUntil(this.clock.now() + 1)
    return out
  }

  /**
   * Let the world's own scheduled work - a scheduler that yielded its slice,
   * promises a step left settling - finish before the run closes, so none of
   * it runs while the next world's taps are in place.
   */
  private async quiesce() {
    let last = -1
    this.quiescing = true
    for (let i = 0; i < 1000 && (this.hash | 0) !== last; i++) {
      last = this.hash | 0
      this.count('quiesceRounds')
      for (let k = 0; k < 3; k++)
        await new Promise<void>((r) => setImmediate(r))
    }
    this.quiescing = false
  }

  /** After a step: processes that ended without a word are marked ended. */
  private sweep() {
    for (const inc of [...this.live])
      if (!this.alive(inc)) this.exited(inc, 'gone')
  }

  async run(steps: number = S.STEPS_PER_RUN): Promise<RunResult> {
    const t0 = performance.now()
    const perf = globalThis.performance
    const realNow = perf.now
    let micros = 0
    let reads = 0
    perf.now = () => {
      micros += pickBetween(
        this.roll(S.STREAM_CLOCK, reads++),
        0,
        S.CLOCK_TICK_MAX_MICROS,
      )
      return micros / 1000
    }
    const untapActors = this.tap(ACTORS_CARD, (n, a, r) =>
      this.onActorsCall(n, a, r),
    )
    const untapReviewer = this.tap(REVIEWER_CARD, (n, a, r) =>
      this.onReviewerCall(n, a, r),
    )
    let ran = 0
    try {
      if (this.options.transport) this.transport = this.options.transport(this)
      else {
        const net = createMemoryNet(this.clock, { latencyMs: S.NET_LATENCY_MS })
        this.transport = {
          link: (n) => net.link(n),
          crash: (n) => net.crash(n),
        }
      }
      const sys0 = await this.drive(this.bootNode(0))
      for (let n = 1; n <= S.POOL_NODES; n++) await this.drive(this.bootNode(n))
      this.startReviewer(sys0)
      this.startPool(sys0)
      for (let step = 0; step < steps && !this.violation; step++) {
        this.step = step
        this.mix(step)
        const event = stepEvent(this.roll(S.STREAM_STEP, step))
        this.apply(event, step, sys0)
        await this.clock.runUntil(
          this.clock.now() + stepMs(this.roll(S.STREAM_DELAY, step)),
        )
        await this.transport.settle?.()
        this.sweep()
        // the clock moves in whole milliseconds; a fraction is a hidden input
        if (!Number.isInteger(this.clock.now())) this.count('fractionalClock')
        this.stepHashes.push(this.hash >>> 0)
        ran = step + 1
      }
      // the quiet tail: nothing new, every window and stall runs out
      this.step = steps
      const start = this.clock.now()
      this.counts.tailStartMs = start
      // one hash a virtual minute, so two runs that part are placed; a fixed
      // count of minutes from a fixed start, never a test against the clock
      for (let m = 1; m * 60 <= S.TAIL_SECONDS && !this.violation; m++) {
        await this.clock.runUntil(start + m * 60_000)
        await this.transport.settle?.()
        this.sweep()
        this.stepHashes.push(this.hash >>> 0)
      }
      this.counts.tailEndMs = this.clock.now()
      await this.quiesce()
      this.stepHashes.push(this.hash >>> 0)
      for (const s of this.rawSystems)
        if (s.stats.yields > 0) this.rare.add(S.RARE_SLICE_YIELDED)
      for (const s of this.rawSystems) {
        this.counts.killed = (this.counts.killed ?? 0) + s.stats.killed
        this.counts.crashed = (this.counts.crashed ?? 0) + s.stats.crashed
        this.counts.yields = (this.counts.yields ?? 0) + s.stats.yields
        this.counts.deadLetters =
          (this.counts.deadLetters ?? 0) + s.stats.deadLetters
      }
      this.counts.rows = this.rows.size
      this.counts.decided = [...this.rows.values()].filter(
        (r) => r.decided,
      ).length
      this.counts.virtualSeconds = Math.round(this.clock.now() / 1000)
      this.counts.clockReads = reads
    } finally {
      // a run that stopped at a violation still lets its own work finish here
      await this.quiesce()
      const hash = this.hash >>> 0
      this.closed = true
      untapActors()
      untapReviewer()
      perf.now = realNow
      await this.transport?.stop?.()
      this.finalHash = hash
    }
    return {
      seed: this.seed,
      steps: ran,
      violation: this.violation,
      stepHashes: this.stepHashes,
      hash: this.finalHash,
      rare: this.rare,
      captured: this.captured,
      afterTail: this.afterTail,
      lines: this.ring.map(
        (l) => `#${l.n} step ${l.step} t=${(l.t / 1000).toFixed(3)}s ${l.text}`,
      ),
      counts: this.counts,
      tail: this.tail(),
      ms: performance.now() - t0,
    }
  }
}

/** The report of a failed run: the seed, how to replay it, the minimal tail. */
export function report(r: RunResult, file: string): string {
  const v = r.violation
  if (!v) return `seed ${r.seed}: ok`
  return [
    `SIMULATION FAILED: ${INVARIANT_NAMES[v.inv]} (invariant ${v.inv})`,
    `  seed ${r.seed}, step ${v.step}, t=${(v.t / 1000).toFixed(3)} s`,
    `  ${v.text}`,
    `  replay: SIM_SEED=${r.seed} bun test ${file}`,
    `  log tail (the last lines that name ${v.subjects.join(', ') || 'anything'}):`,
    ...r.tail,
  ].join('\n')
}
