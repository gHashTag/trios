/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTOR RUNTIME'S TELEMETRY (gHashTag/t27 specs/queen/telemetry.t27,
 * epic gHashTag/trios#1712 item 1). Production could not say how deep a
 * mailbox is, how long a turn takes, how often a supervisor restarts or how
 * many letters die, and two readings of the reviewer disagree: 23% of its
 * worker-seconds busy on 2026-10-08, 11 reviewed an hour against 58 finished
 * on 2026-10-09.
 *
 * The card decides:
 *   - when a mailbox raises its event and when it clears it, with a gap
 *     between the two, as OTP 27's long_message_queue (depth_event);
 *   - when a turn or a message's wait is long (turn_event, wait_event);
 *   - when a supervisor's restarts are a storm (storm_event);
 *   - the bucket of a time and the rank of a percentile (bucket_of, rank_of);
 *   - a kind's busy share (busy_permille);
 *   - which messages and card calls are recorded (window_over, kept);
 *   - the order of the deepest mailboxes (ranked, deeper_first);
 *   - when the summary line is due (summary_due);
 *   - whether a replay of the decision log passes (replay_verdict).
 * This file counts, keeps the rings and answers the endpoint. It never
 * schedules: no timer, no send, no turn of its own. With it on or off the
 * runtime takes the same turns in the same order.
 *
 * KINDS, NOT PIDS. Every count is per actor kind (ActorSpec.kind, else its
 * name), as Akka groups its metrics. The top-N list is the one place a pid
 * appears, and it says how deep a mailbox is, never what it holds (recon).
 *
 * THE DECISION LOG. Every call into the runtime's cards goes through
 * queen-card-wasm.ts, whose tap hands the sampled ones here. A record holds
 * the card, fn, args, result, the kind whose turn made the call, the UTC time
 * and the call's number in its window. The kind follows a turn across its
 * awaits (AsyncLocalStorage), so a decision made after an await in a receive
 * body is still that kind's. `replayDecisions` runs records again on fresh
 * instances of the pinned cards.
 */

import { AsyncLocalStorage } from 'node:async_hooks'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { DEFAULT_SPECS_ROOT } from '../../inngest/spec-catalog'
import { logger } from '../../lib/logger'
import type { Clock } from './queen-actors'
import { MAILBOX_CAP } from './queen-actors-card.gen'
import {
  type CardWasm,
  flag,
  loadCardWasm,
  reopenCardGates,
  setCardTap,
  u32,
} from './queen-card-wasm'
import {
  DLOG_FIELDS,
  DLOG_PER_FN_PER_WINDOW,
  DLOG_RING,
  END_CRASH,
  END_KILL,
  EV_DEPTH_HIGH,
  EV_NONE,
  EV_STORM,
  EVENT_KINDS,
  EVENT_RING,
  HIST_BUCKETS,
  MSG_RING,
  MSG_SAMPLES_PER_WINDOW,
  MT_BUSY,
  MT_CONTROL,
  MT_CRASHES,
  MT_DEAD_LETTERS,
  MT_DROPPED_FULL,
  MT_GIVE_UPS,
  MT_KILLS,
  MT_KILLS_STOPPED,
  MT_LIVE,
  MT_MAILBOX_DEPTH,
  MT_MAILBOX_TIME,
  MT_RESTARTS,
  MT_SENT,
  MT_TURN_TIME,
  MT_TURNS,
  MT_YIELDS,
  NO_KIND,
  SAMPLE_WINDOW_MS,
  TEL_EVENT_NAMES,
  TOP_N,
} from './queen-telemetry-card.gen'

export const TELEMETRY_CARD = 'queen/telemetry.wasm'

let loaded: CardWasm | undefined
const card = () => {
  loaded ??= loadCardWasm(TELEMETRY_CARD)
  return loaded
}
const c = (name: string, ...a: number[]) => card().call(name, ...a)

// A time in ms below this has its bucket remembered: bucket_of is pure, so
// the card's answer for one ms is its answer for good.
const BUCKET_MEMO_MS = 4096
const bucketMemo: number[] = []
const bucketOf = (ms: number): number => {
  if (ms >= BUCKET_MEMO_MS) return c('bucket_of', u32(ms))
  let b = bucketMemo[ms]
  if (b === undefined) {
    b = c('bucket_of', ms)
    bucketMemo[ms] = b
  }
  return b
}

interface Counts {
  sent: number
  deadLetters: number
  droppedFull: number
  control: number
  turns: number
  crashes: number
  kills: number
  killsStopped: number
  restarts: number
  busyMs: number
}
const zero = (): Counts => ({
  sent: 0,
  deadLetters: 0,
  droppedFull: 0,
  control: 0,
  turns: 0,
  crashes: 0,
  kills: 0,
  killsStopped: 0,
  restarts: 0,
  busyMs: 0,
})
const countsOf = (k: Counts): Counts => ({
  sent: k.sent,
  deadLetters: k.deadLetters,
  droppedFull: k.droppedFull,
  control: k.control,
  turns: k.turns,
  crashes: k.crashes,
  kills: k.kills,
  killsStopped: k.killsStopped,
  restarts: k.restarts,
  busyMs: k.busyMs,
})

export interface KindStats extends Counts {
  kind: string
  live: number
  /** Deepest mailbox since start, and since the last summary line. */
  depthMax: number
  depthWin: number
  /** Actors of this kind whose depth alarm is on. */
  depthAlarms: number
  turnMs: number[]
  turnWin: number[]
  waitMs: number[]
  turnMaxMs: number
  events: number[]
  /** This window's sampled messages, and whether `kept` still says yes. */
  samples: number
  sampleOpen: boolean
  last: Counts & { events: number[] }
}

/** What telemetry reads and writes on a process. */
export interface TelProc {
  pid: bigint
  k?: KindStats
  alarm: boolean
  /** Seconds: the bound its kill timer uses. */
  bound: number
  /** The current message's time in its mailbox, -1 when not sampled. */
  waitMs: number
  /** Ms from which a turn is long under its bound (long_at_ms), set at spawn. */
  longAt: number
  box: unknown[]
  busy: boolean
  turnAt: number
}

export interface DecisionRecord {
  card: string
  fn: string
  args: Array<number | bigint>
  result: number | bigint
  kind: string
  at: number
  n: number
}

export interface SupervisorStats {
  name: string
  maxRestarts: number
  /** The restarts its period holds now (actors.t27 in_period, its own count). */
  inPeriod: () => number
  restarts: number
  giveUps: number
  storm: boolean
}

export interface TelemetryOptions {
  /** The cards whose calls go to the decision log; this card is added. */
  cards?: string[]
  /** Every record as it is written, beyond the ring (a replay test). */
  onRecord?: (r: DecisionRecord) => void
  /** Where the summary line goes; the server log when unset. */
  summary?: (line: Record<string, unknown>) => void
}

interface Ring<T> {
  items: T[]
  next: number
  written: number
}
const ring = <T>(): Ring<T> => ({ items: [], next: 0, written: 0 })
const put = <T>(r: Ring<T>, size: number, item: T) => {
  r.items[r.next] = item
  r.next = (r.next + 1) % size
  r.written++
}
/** Oldest first. */
const read = <T>(r: Ring<T>, size: number): T[] =>
  r.written <= size
    ? r.items.slice(0, r.written)
    : [...r.items.slice(r.next), ...r.items.slice(0, r.next)]

const wide = (v: number | bigint) => (typeof v === 'bigint' ? String(v) : v)

export function createActorTelemetry(
  clock: Clock,
  options: TelemetryOptions = {},
) {
  const als = new AsyncLocalStorage<string>()
  const kinds = new Map<string, KindStats>()
  const sups = new Map<string, SupervisorStats>()
  const decisionRing = ring<DecisionRecord>()
  const eventRing = ring<Record<string, unknown>>()
  const msgRing = ring<Record<string, unknown>>()
  const since = clock.now()
  let windowAt = since
  // the host's arithmetic for when to ask the card (window_over), as the
  // kill timer's arithmetic decides when to ask turn_signal
  let rollAt = since + SAMPLE_WINDOW_MS
  let summaryAt = since
  let procs: () => Iterable<TelProc> = () => []
  let slot: (pid: bigint) => number = () => 0
  let yields: () => number = () => 0
  let yieldsAtSummary = 0
  let closed = false

  const kind = (name: string): KindStats => {
    let k = kinds.get(name)
    if (k === undefined) {
      k = {
        ...zero(),
        kind: name,
        live: 0,
        depthMax: 0,
        depthWin: 0,
        depthAlarms: 0,
        turnMs: new Array(HIST_BUCKETS).fill(0),
        turnWin: new Array(HIST_BUCKETS).fill(0),
        waitMs: new Array(HIST_BUCKETS).fill(0),
        turnMaxMs: 0,
        events: new Array(EVENT_KINDS).fill(0),
        samples: 0,
        sampleOpen: true,
        last: { ...zero(), events: new Array(EVENT_KINDS).fill(0) },
      }
      kinds.set(name, k)
    }
    return k
  }
  const deadKind = kind(NO_KIND)

  // WHERE AN ANSWER CAN CHANGE. The card's own thresholds, asked once: below
  // depth_on_at an off alarm stays off, and below long_at_ms a turn is not
  // long (telemetry.t27 a_falling_depth_never_raises_and_a_rising_one_never_clears,
  // a_turn_is_long_exactly_from_its_threshold). So the host asks the card only
  // at or past them, as it asks turn_signal only once the kill timer fires.
  const raiseAt = c('depth_on_at', MAILBOX_CAP)
  const longAt = new Map<number, number>()
  const longAtMs = (bound: number) => {
    let at = longAt.get(bound)
    if (at === undefined) {
      at = Number(card().call64('long_at_ms', u32(bound)))
      longAt.set(bound, at)
    }
    return at
  }

  const event = (e: number, k: KindStats, value: number) => {
    k.events[e]++
    const record = {
      event: TEL_EVENT_NAMES[e],
      kind: k.kind,
      value,
      at: clock.now(),
    }
    put(eventRing, EVENT_RING, record)
    if (e === EV_DEPTH_HIGH || e === EV_STORM)
      logger.warn('Queen actor event', record)
  }

  const alarm = (p: TelProc, depth: number) => {
    const k = p.k as KindStats
    const e = c('depth_event', flag(p.alarm), u32(depth), MAILBOX_CAP)
    if (e === EV_NONE) return
    p.alarm = e === EV_DEPTH_HIGH
    k.depthAlarms += p.alarm ? 1 : -1
    event(e, k, depth)
  }

  const cards = new Set([...(options.cards ?? []), TELEMETRY_CARD])
  setCardTap({
    files: cards,
    keep: (n) => c('kept', u32(n), DLOG_PER_FN_PER_WINDOW) !== 0,
    record: (file, fn, args, result, n) => {
      const r: DecisionRecord = {
        card: file,
        fn,
        args,
        result,
        kind: als.getStore() ?? NO_KIND,
        at: clock.now(),
        n,
      }
      put(decisionRing, DLOG_RING, r)
      options.onRecord?.(r)
    },
  })

  /** p50 and p95 of a histogram, each as its bucket's [floor, next floor). */
  const quantiles = (hist: number[]) => {
    const count = hist.reduce((a, b) => a + b, 0)
    const at = (percent: number) => {
      const rank = c('rank_of', u32(count), percent)
      if (rank === 0) return null
      let seen = 0
      for (let b = 0; b < hist.length; b++) {
        seen += hist[b]
        if (seen >= rank)
          return b + 1 < HIST_BUCKETS
            ? [c('bucket_floor_ms', b), c('bucket_floor_ms', b + 1)]
            : [c('bucket_floor_ms', b), null]
      }
      return null
    }
    return { count, p50: at(50), p95: at(95) }
  }
  const histogram = (hist: number[]) => ({
    ...quantiles(hist),
    buckets: hist.flatMap((n, b) =>
      n > 0 ? [[c('bucket_floor_ms', b), n] as const] : [],
    ),
  })

  const storms = (s: SupervisorStats) => {
    const n = s.inPeriod()
    const e = c('storm_event', flag(s.storm), u32(n), u32(s.maxRestarts))
    if (e === EV_NONE) return
    s.storm = e === EV_STORM
    event(e, kind(s.name), n)
  }

  const summarize = (now: number) => {
    const windowMs = now - summaryAt
    const out: Record<string, unknown> = {}
    for (const k of kinds.values()) {
      const turns = k.turns - k.last.turns
      const sent = k.sent - k.last.sent
      const dead = k.deadLetters - k.last.deadLetters
      const restarts = k.restarts - k.last.restarts
      const events = k.events.reduce((a, n, e) => a + n - k.last.events[e], 0)
      const quiet = turns + sent + dead + restarts + events === 0
      if (quiet && k.live === 0) continue
      const q = quantiles(k.turnWin)
      out[k.kind] = {
        live: k.live,
        turns,
        busyPermille: c(
          'busy_permille',
          u32(k.busyMs - k.last.busyMs),
          u32(k.live),
          u32(windowMs),
        ),
        turnP50Ms: q.p50,
        turnP95Ms: q.p95,
        sent,
        depthMax: k.depthWin,
        deadLetters: dead,
        droppedFull: k.droppedFull - k.last.droppedFull,
        crashes: k.crashes - k.last.crashes,
        kills: k.kills - k.last.kills,
        restarts,
        events,
      }
      k.turnWin.fill(0)
      k.depthWin = 0
      k.last = { ...countsOf(k), events: [...k.events] }
    }
    const line = {
      windowSeconds: Math.round(windowMs / 1000),
      kinds: out,
      storms: [...sups.values()].filter((s) => s.storm).map((s) => s.name),
      yields: yields() - yieldsAtSummary,
      decisionsLogged: decisionRing.written,
    }
    yieldsAtSummary = yields()
    summaryAt = now
    if (options.summary) options.summary(line)
    else logger.info('Queen actors measured', line)
  }

  const roll = (now: number) => {
    if (c('window_over', u32(now - windowAt)) === 0) return
    windowAt = now
    rollAt = now + SAMPLE_WINDOW_MS
    reopenCardGates()
    for (const k of kinds.values()) {
      k.samples = 0
      k.sampleOpen = true
    }
    if (c('summary_due', u32((now - summaryAt) / 1000)) !== 0) {
      for (const s of sups.values()) storms(s)
      summarize(now)
    }
  }

  const self = {
    als,
    kind,
    /**
     * The system's live processes, a pid's slot (actors.t27 slot_of) and its
     * yield count, read when asked.
     */
    watch(o: {
      procs: () => Iterable<TelProc>
      slot: (pid: bigint) => number
      yields: () => number
    }) {
      procs = o.procs
      slot = o.slot
      yields = o.yields
    },
    spawned(p: TelProc) {
      ;(p.k as KindStats).live++
      p.longAt = longAtMs(p.bound)
    },
    exited(p: TelProc) {
      const k = p.k as KindStats
      k.live--
      k.deadLetters += p.box.length
      if (p.alarm) alarm(p, 0)
    },
    dropped(k: KindStats | undefined, full: boolean) {
      const to = k ?? deadKind
      to.deadLetters++
      if (full) to.droppedFull++
    },
    /** Whether the next message to a kind carries a record (kept). */
    sample(k: KindStats): boolean {
      if (!k.sampleOpen) return false
      if (c('kept', u32(k.samples), MSG_SAMPLES_PER_WINDOW) !== 0) {
        k.samples++
        return true
      }
      k.sampleOpen = false
      return false
    },
    /** A letter is in the mailbox: count it and read the depth. */
    queued(p: TelProc) {
      const k = p.k as KindStats
      k.sent++
      const depth = p.box.length
      if (depth > k.depthMax) k.depthMax = depth
      if (depth > k.depthWin) k.depthWin = depth
      if (depth >= raiseAt || p.alarm) alarm(p, depth)
    },
    controlTurn(p: TelProc, crashed: boolean) {
      const k = p.k as KindStats
      k.control++
      if (crashed) k.crashes++
    },
    /** A data turn started at p.turnAt; `stampedAt` is its letter's arrival, or -1. */
    turnStart(p: TelProc, stampedAt: number) {
      const now = p.turnAt
      if (now >= rollAt) roll(now)
      // a take only lowers the depth, so an off alarm cannot come on here
      // (telemetry.t27 a_falling_depth_never_raises_and_a_rising_one_never_clears)
      if (p.alarm) alarm(p, p.box.length)
      if (stampedAt < 0) {
        p.waitMs = -1
        return
      }
      const k = p.k as KindStats
      const wait = now - stampedAt
      p.waitMs = wait
      k.waitMs[bucketOf(wait)]++
      const e = c('wait_event', u32(wait), u32(p.bound))
      if (e !== EV_NONE) event(e, k, wait)
    },
    /** A data turn ended at `now`: done, crashed or killed (END_*). */
    turnEnd(p: TelProc, end: number, now: number, stopped = false) {
      const k = p.k as KindStats
      const ms = Math.max(0, now - p.turnAt)
      k.turns++
      k.busyMs += ms
      const b = bucketOf(ms)
      k.turnMs[b]++
      k.turnWin[b]++
      if (ms > k.turnMaxMs) k.turnMaxMs = ms
      if (end === END_CRASH) k.crashes++
      if (end === END_KILL) {
        k.kills++
        if (stopped) k.killsStopped++
      }
      if (ms >= p.longAt) {
        const e = c('turn_event', u32(ms), u32(p.bound))
        if (e !== EV_NONE) event(e, k, ms)
      }
      if (p.waitMs >= 0) {
        put(msgRing, MSG_RING, {
          kind: k.kind,
          wait_ms: p.waitMs,
          turn_ms: ms,
          end,
          at: now,
        })
        p.waitMs = -1
      }
    },
    supervisor(
      name: string,
      maxRestarts: number,
      inPeriod: () => number,
    ): SupervisorStats {
      const had = sups.get(name)
      const s: SupervisorStats = had ?? {
        name,
        maxRestarts,
        inPeriod,
        restarts: 0,
        giveUps: 0,
        storm: false,
      }
      // a supervisor its parent started again counts on under its name
      s.inPeriod = inPeriod
      s.maxRestarts = maxRestarts
      sups.set(name, s)
      return s
    },
    restarted(s: SupervisorStats, childKind: string) {
      s.restarts++
      kind(childKind).restarts++
      storms(s)
    },
    gaveUp(s: SupervisorStats) {
      s.giveUps++
    },
    /** Everything counted, for GET /queen/actors/metrics. */
    snapshot() {
      const now = clock.now()
      for (const s of sups.values()) storms(s)
      const top: TelProc[] = []
      for (const p of procs()) {
        const depth = p.box.length
        if (c('ranked', u32(depth)) === 0) continue
        let at = top.length
        while (
          at > 0 &&
          c(
            'deeper_first',
            u32(depth),
            u32(slot(p.pid)),
            u32(top[at - 1].box.length),
            u32(slot(top[at - 1].pid)),
          ) !== 0
        )
          at--
        if (at < TOP_N) {
          top.splice(at, 0, p)
          if (top.length > TOP_N) top.pop()
        }
      }
      const out: Record<string, unknown> = {}
      for (const k of kinds.values())
        out[k.kind] = {
          [MT_LIVE]: k.live,
          [MT_SENT]: k.sent,
          [MT_DEAD_LETTERS]: k.deadLetters,
          [MT_DROPPED_FULL]: k.droppedFull,
          [MT_CONTROL]: k.control,
          [MT_TURNS]: k.turns,
          [MT_CRASHES]: k.crashes,
          [MT_KILLS]: k.kills,
          [MT_KILLS_STOPPED]: k.killsStopped,
          [MT_RESTARTS]: k.restarts,
          [MT_MAILBOX_DEPTH]: { max: k.depthMax, alarmsOn: k.depthAlarms },
          [MT_BUSY]: c(
            'busy_permille',
            u32(k.busyMs),
            u32(k.live),
            u32(now - since),
          ),
          [MT_TURN_TIME]: { ...histogram(k.turnMs), maxMs: k.turnMaxMs },
          [MT_MAILBOX_TIME]: histogram(k.waitMs),
          events: Object.fromEntries(
            k.events.flatMap((n, e) =>
              e > 0 && n > 0 ? [[TEL_EVENT_NAMES[e], n]] : [],
            ),
          ),
        }
      return {
        enabled: true,
        at: now,
        since,
        kinds: out,
        supervisors: Object.fromEntries(
          [...sups.values()].map((s) => [
            s.name,
            {
              [MT_RESTARTS]: s.restarts,
              [MT_GIVE_UPS]: s.giveUps,
              inPeriod: s.inPeriod(),
              storm: s.storm,
            },
          ]),
        ),
        top: top.map((p) => ({
          pid: String(p.pid),
          kind: p.k?.kind ?? NO_KIND,
          depth: p.box.length,
          busy: p.busy,
          turnAgeMs: p.busy ? now - p.turnAt : 0,
        })),
        [MT_YIELDS]: yields(),
        events: read(eventRing, EVENT_RING),
        messages: read(msgRing, MSG_RING),
        decisionLog: {
          fields: DLOG_FIELDS,
          ring: DLOG_RING,
          written: decisionRing.written,
        },
      }
    },
    /** The newest `limit` decision records, oldest first; u64s as strings. */
    decisions(limit: number = DLOG_RING): Array<Record<string, unknown>> {
      const all = read(decisionRing, DLOG_RING)
      return all
        .slice(Math.max(0, all.length - Math.max(0, limit)))
        .map((r) => ({ ...r, args: r.args.map(wide), result: wide(r.result) }))
    },
    /** The records as written, for a replay in this process. */
    records: (): DecisionRecord[] => read(decisionRing, DLOG_RING),
    close() {
      if (closed) return
      closed = true
      setCardTap(undefined)
      if (live === self) live = undefined
    },
  }
  return self
}

export type ActorTelemetry = ReturnType<typeof createActorTelemetry>

// the one telemetry the endpoint reads: the production actor system's
let live: ActorTelemetry | undefined
export function setLiveActorTelemetry(t: ActorTelemetry | undefined): void {
  live = t
}
export const liveActorTelemetry = (): ActorTelemetry | undefined => live

export interface ReplayResult {
  verdict: number
  records: number
  unknownFns: number
  mismatches: number
  unsampled: number
  firstBad?: Record<string, unknown>
}

/**
 * Run every record again on a fresh instance of its card, read from the
 * pinned specs, and let the card judge (replay_verdict). Records may come
 * from this process or from GET /queen/actors/decisions (u64s as strings).
 */
export function replayDecisions(
  records: Array<DecisionRecord | Record<string, unknown>>,
  root: string = DEFAULT_SPECS_ROOT,
): ReplayResult {
  const fresh = new Map<string, Record<string, unknown>>()
  const exportsOf = (file: string) => {
    let x = fresh.get(file)
    if (x === undefined) {
      const m = new WebAssembly.Module(readFileSync(join(root, file)))
      x = new WebAssembly.Instance(m, {}).exports as Record<string, unknown>
      ;(x._initialize as (() => void) | undefined)?.()
      fresh.set(file, x)
    }
    return x
  }
  type Fn = (...a: Array<number | bigint>) => number | bigint
  const judge = exportsOf(TELEMETRY_CARD)
  const tel = (name: string) => judge[name] as Fn
  const back = (v: unknown): number | bigint =>
    typeof v === 'string' ? BigInt(v) : (v as number | bigint)
  let unknownFns = 0
  let mismatches = 0
  let unsampled = 0
  let firstBad: Record<string, unknown> | undefined
  for (const r of records) {
    const fn = exportsOf(String(r.card))[String(r.fn)] as Fn | undefined
    let bad = ''
    if (typeof fn !== 'function') {
      unknownFns++
      bad = 'unknown fn'
    } else {
      let got: number | bigint | string
      try {
        got = fn(...(r.args as unknown[]).map(back))
      } catch (error) {
        got = `threw ${error instanceof Error ? error.message : String(error)}`
      }
      if (String(got) !== String(back(r.result))) {
        mismatches++
        bad = `replayed ${String(got)}`
      }
    }
    if (tel('kept')(u32(Number(r.n)), DLOG_PER_FN_PER_WINDOW) === 0) {
      unsampled++
      bad ||= 'not kept'
    }
    if (bad && !firstBad) firstBad = { ...r, bad }
  }
  return {
    verdict: Number(
      tel('replay_verdict')(
        u32(records.length),
        u32(unknownFns),
        u32(mismatches),
        u32(unsampled),
      ),
    ),
    records: records.length,
    unknownFns,
    mismatches,
    unsampled,
    firstBad,
  }
}
