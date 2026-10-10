/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTOR RUNTIME'S TELEMETRY (gHashTag/t27 specs/queen/telemetry.t27,
 * trios#1712 item 1). The card runs as the wasm runs it, under a virtual
 * clock, except in the cost benchmark at the end, which uses the real one.
 *
 * Four claims are checked here:
 *   1. every count the card's rules define reaches the endpoint (depth alarm,
 *      long turn, kill, crash, restart storm, dead letters);
 *   2. telemetry changes no scheduling: the reviewer on the seeded bench input
 *      starts the same reviews at the same times with it on and off;
 *   3. every decision it logs replays to the same answer on fresh instances
 *      of the pinned cards, and a corrupted log does not;
 *   4. what it costs per message, off and on, on the ring benchmark's input.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { createQueenActorsRoute } from '../../src/api/routes/queen-actors-metrics'
import {
  ACTORS_CARD,
  actorChild,
  createActorSystem,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  MAILBOX_CAP,
  RESTART_PERMANENT,
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
} from '../../src/api/services/queen-actors-card.gen'
import {
  type DecisionRecord,
  replayDecisions,
  setLiveActorTelemetry,
  TELEMETRY_CARD,
  type TelemetryOptions,
} from '../../src/api/services/queen-actors-telemetry'
import { reviewerTree } from '../../src/api/services/queen-review-actors'
import {
  drainReviewerRound,
  REVIEWER_CARD,
} from '../../src/api/services/queen-review-loop'
import { REVIEW_ROW_SECONDS } from '../../src/api/services/queen-reviewer-card.gen'
import {
  DLOG_FIELDS,
  DLOG_PER_FN_PER_WINDOW,
  MT_CRASHES,
  MT_DEAD_LETTERS,
  MT_DROPPED_FULL,
  MT_KILLS,
  MT_MAILBOX_DEPTH,
  MT_RESTARTS,
  MT_TURN_TIME,
  MT_TURNS,
  RV_EMPTY,
  RV_MISMATCH,
  RV_PASS,
  RV_UNKNOWN_FN,
  RV_UNSAMPLED,
  TEL_EVENT_NAMES,
} from '../../src/api/services/queen-telemetry-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { writeDecisionSpecs } from './queen-decision-spec'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

type Snap = {
  kinds: Record<string, Record<string, unknown>>
  supervisors: Record<string, Record<string, unknown>>
  top: Array<{ kind: string; depth: number }>
  events: Array<{ event: string; kind: string }>
}
const eventsOf = (s: Snap, name: string) =>
  s.events.filter((e) => e.event === name)

// The counting tests run with slices off. A slice reads the host's real
// microseconds, so on a loaded host a turn chain can yield past the end of a
// virtual-clock run and stop there; without slices it never reads real time.

describe('the vendored telemetry card is the one PIN names', () => {
  it('telemetry.t27 and telemetry.wasm match, and the wasm exports every fn', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/telemetry.t27 sha256 ${sha('queen/telemetry.t27')}`,
    )
    expect(pin).toContain(
      `queen/telemetry.wasm sha256 ${sha('queen/telemetry.wasm')}`,
    )
    const spec = readFileSync(
      join(DEFAULT_SPECS_ROOT, 'queen/telemetry.t27'),
      'utf8',
    )
    const fns = [...spec.matchAll(/^pub fn (\w+)\(/gm)].map((m) => m[1])
    const exported = WebAssembly.Module.exports(
      new WebAssembly.Module(
        readFileSync(join(DEFAULT_SPECS_ROOT, 'queen/telemetry.wasm')),
      ),
    ).map((e) => e.name)
    expect(fns.length).toBeGreaterThan(15)
    for (const fn of fns) expect(exported).toContain(fn)
  })
})

describe('what the runtime counts', () => {
  it('a mailbox that fills raises one event, drops past the cap, and clears once drained', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    const pid = sys.spawn<number>({
      name: 'slow-0',
      kind: 'slow',
      receive: () => new Promise<void>((r) => clock.after(1000, r)),
    })
    // every send lands before the first turn: MAILBOX_CAP wait, the rest drop
    for (let i = 0; i < MAILBOX_CAP + 44; i++) sys.send(pid, i)
    let s = tel.snapshot() as unknown as Snap
    expect(eventsOf(s, TEL_EVENT_NAMES[1]).length).toBe(1)
    expect(s.kinds.slow[MT_DROPPED_FULL]).toBe(44)
    expect(s.kinds.slow[MT_DEAD_LETTERS]).toBe(44)
    expect(s.kinds.slow[MT_MAILBOX_DEPTH]).toEqual({
      max: MAILBOX_CAP,
      alarmsOn: 1,
    })
    expect(s.top).toEqual([
      expect.objectContaining({ kind: 'slow', depth: MAILBOX_CAP }),
    ])
    await clock.runUntil(400_000)
    s = tel.snapshot() as unknown as Snap
    expect(eventsOf(s, TEL_EVENT_NAMES[1]).length).toBe(1)
    expect(eventsOf(s, TEL_EVENT_NAMES[2]).length).toBe(1)
    expect(s.kinds.slow[MT_TURNS]).toBe(MAILBOX_CAP)
    expect(s.kinds.slow[MT_MAILBOX_DEPTH]).toEqual({
      max: MAILBOX_CAP,
      alarmsOn: 0,
    })
    expect(s.top).toEqual([])
    tel.close()
  })

  it('a turn past half its bound is long, a hung one is killed, a crash is a crash', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    const seconds = [100, 160, 4000]
    const pid = sys.spawn<number>({
      name: 'reviewer-like',
      turnMaxSeconds: REVIEW_ROW_SECONDS,
      receive: (s) => new Promise<void>((r) => clock.after(s * 1000, r)),
    })
    for (const s of seconds) sys.send(pid, s)
    const crasher = sys.spawn<number>({
      name: 'crasher',
      receive: () => {
        throw new Error('boom')
      },
    })
    sys.send(crasher, 1)
    await clock.runUntil(1_000_000)
    const s = tel.snapshot() as unknown as Snap
    const k = s.kinds['reviewer-like']
    // 100 s is short; 160 s is long; the 4000 s one is killed at 300 s, and long
    expect(eventsOf(s, TEL_EVENT_NAMES[3]).length).toBe(2)
    expect(k[MT_KILLS]).toBe(1)
    expect(k[MT_TURNS]).toBe(3)
    expect((k[MT_TURN_TIME] as { maxMs: number }).maxMs).toBe(300_000)
    expect(s.kinds.crasher[MT_CRASHES]).toBe(1)
    tel.close()
  })

  it('three restarts in the period are a storm, and the storm ends with the period', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    let crashes = 0
    const child = actorChild(
      sys,
      {
        name: 'flaky-0',
        kind: 'flaky',
        init: (self) => {
          if (crashes < 3) clock.after(1000, () => sys.send(self, 'crash'))
        },
        receive: () => {
          crashes++
          throw new Error('crash')
        },
      },
      RESTART_PERMANENT,
    )
    supervisor(
      sys,
      {
        name: 'flaky-domain',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [child],
    ).start(() => {})
    await clock.runUntil(250_000)
    let s = tel.snapshot() as unknown as Snap
    expect(crashes).toBe(3)
    expect(s.kinds.flaky[MT_RESTARTS]).toBe(3)
    expect(s.supervisors['flaky-domain']).toMatchObject({ storm: true })
    expect(eventsOf(s, TEL_EVENT_NAMES[5]).length).toBe(1)
    await clock.runUntil(1_000_000)
    s = tel.snapshot() as unknown as Snap
    expect(s.supervisors['flaky-domain']).toMatchObject({
      storm: false,
      inPeriod: 0,
    })
    expect(eventsOf(s, TEL_EVENT_NAMES[6]).length).toBe(1)
    tel.close()
  })

  it('the summary line comes at the first turn after its period, per kind', async () => {
    const clock = new VirtualClock()
    const lines: Array<Record<string, unknown>> = []
    const sys = createActorSystem(clock, {
      slices: false,
      telemetry: { summary: (l) => lines.push(l) },
    })
    const pid = sys.spawn<number>({
      name: 'worker-0',
      kind: 'worker',
      receive: () => new Promise<void>((r) => clock.after(30_000, r)),
    })
    const beat = () => {
      sys.send(pid, 1)
      clock.after(60_000, beat)
    }
    beat()
    await clock.runUntil(301_000)
    expect(lines.length).toBe(1)
    const worker = (lines[0].kinds as Record<string, Record<string, unknown>>)
      .worker
    // five 30 s turns in 300 s on one actor: half its time
    expect(worker.turns).toBe(5)
    expect(worker.busyPermille).toBe(500)
    expect(lines[0].windowSeconds).toBe(300)
    sys.telemetry?.close()
  })

  it('GET /queen/actors/metrics answers the live system, and says so when there is none', async () => {
    const app = createQueenActorsRoute()
    setLiveActorTelemetry(undefined)
    expect(await (await app.request('/metrics')).json()).toMatchObject({
      enabled: false,
    })
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    setLiveActorTelemetry(sys.telemetry)
    const pid = sys.spawn<number>({ name: 'echo', receive: () => {} })
    sys.send(pid, 1)
    await clock.runUntil(10)
    const body = (await (await app.request('/metrics')).json()) as Snap & {
      enabled: boolean
    }
    expect(body.enabled).toBe(true)
    expect(body.kinds.echo[MT_TURNS]).toBe(1)
    const log = (await (await app.request('/decisions?limit=5')).json()) as {
      fields: string[]
      records: Array<Record<string, unknown>>
    }
    expect(log.fields).toEqual(DLOG_FIELDS)
    expect(log.records.length).toBe(5)
    for (const r of log.records)
      expect(Object.keys(r).sort()).toEqual([...DLOG_FIELDS].sort())
    sys.telemetry?.close()
    setLiveActorTelemetry(undefined)
  })
})

// THE SEEDED BENCH INPUT. A copy of queen-actors-bench.test.ts `workload`
// (same generator, same seeds 7851..7855), so these runs see the draws the
// bench table is built from. A test file cannot import another without
// running its tests.
const HOUR = 3_600_000
const STALL_SECONDS = 1800
type Fate = 'ok' | 'crash' | 'stall'
interface Attempt {
  fate: Fate
  seconds: number
}
interface Workload {
  arrivals: Array<{ issue: number; at: number }>
  attempts: Map<number, Attempt[]>
  horizonMs: number
}
const mulberry32 = (seed: number) => {
  let a = seed
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
function workload(
  seed: number,
  o: {
    perHour: number
    hours: number
    medianSeconds: number
    crash: number
    stall: number
    burst?: number
  },
): Workload {
  const rnd = mulberry32(seed)
  const normal = () =>
    Math.sqrt(-2 * Math.log(1 - rnd())) * Math.cos(2 * Math.PI * rnd())
  const arrivals: Workload['arrivals'] = []
  let issue = 1000
  for (let i = 0; i < (o.burst ?? 0); i++)
    arrivals.push({ issue: issue++, at: Math.floor(rnd() * 600_000) })
  let t = 0
  for (;;) {
    t += (-Math.log(1 - rnd()) / o.perHour) * HOUR
    if (t >= o.hours * HOUR) break
    arrivals.push({ issue: issue++, at: Math.floor(t) })
  }
  arrivals.sort((a, b) => a.at - b.at)
  const attempts = new Map<number, Attempt[]>()
  for (const a of arrivals) {
    const list: Attempt[] = []
    for (let k = 0; k < 8; k++) {
      const u = rnd()
      const fate: Fate =
        u < o.crash ? 'crash' : u < o.crash + o.stall ? 'stall' : 'ok'
      const seconds = Math.min(
        Math.round(REVIEW_ROW_SECONDS * 0.9),
        Math.max(5, Math.round(o.medianSeconds * Math.exp(0.7 * normal()))),
      )
      list.push({ fate, seconds })
    }
    attempts.set(a.issue, list)
  }
  return { arrivals, attempts, horizonMs: (o.hours + 1) * HOUR }
}
const FAULTS_180 = workload(7852, {
  perHour: 45,
  hours: 6,
  medianSeconds: 180,
  crash: 0.05,
  stall: 0.02,
})
const BURST = workload(7855, {
  perHour: 45,
  hours: 6,
  medianSeconds: 180,
  crash: 0.05,
  stall: 0.02,
  burst: 120,
})

/** The bench's actors run: the reviewer tree under a root, woken per arrival. */
async function runReviewer(
  w: Workload,
  o: {
    telemetry?: TelemetryOptions
    slices?: boolean
    turnStop?: boolean
  } = {},
) {
  const clock = new VirtualClock()
  const arrived = new Map<number, number>()
  const completed = new Map<number, number>()
  const attemptOf = new Map<number, number>()
  const fence = new Map<number, number>()
  const starts: string[] = []
  const injected = { crash: 0, stall: 0 }
  const reviewOne = (issue: number) => {
    starts.push(`${issue}@${clock.now()}`)
    const k = attemptOf.get(issue) ?? 0
    attemptOf.set(issue, k + 1)
    const token = (fence.get(issue) ?? 0) + 1
    fence.set(issue, token)
    const list = w.attempts.get(issue) as Attempt[]
    const a = list[k % list.length]
    if (a.fate !== 'ok') injected[a.fate]++
    const ms =
      a.fate === 'stall'
        ? STALL_SECONDS * 1000
        : a.fate === 'crash'
          ? a.seconds * 300
          : a.seconds * 1000
    return new Promise<{ acted: string[]; strays: string[]; tally: string[] }>(
      (resolve, reject) =>
        clock.after(ms, () => {
          if (a.fate === 'crash') return reject(new Error('review crashed'))
          if (fence.get(issue) === token && !completed.has(issue))
            completed.set(issue, clock.now())
          resolve({ acted: [], strays: [], tally: [] })
        }),
    )
  }
  const sys = createActorSystem(clock, {
    slices: o.slices ?? true,
    turnStop: o.turnStop ?? false,
    ...(o.telemetry
      ? { telemetry: { summary: () => {}, ...o.telemetry } }
      : {}),
  })
  const r = reviewerTree(sys, {
    holdsLease: async () => true,
    waiting: async () => [...arrived.keys()].filter((n) => !completed.has(n)),
    reviewOne: (issue) => reviewOne(issue),
  })
  supervisor(
    sys,
    {
      name: 'root',
      strategy: STRAT_ONE_FOR_ONE,
      maxRestarts: ROOT_MAX_RESTARTS,
      periodSeconds: ROOT_PERIOD_SECONDS,
    },
    [r.tree],
  ).start(() => {})
  for (const a of w.arrivals)
    clock.after(a.at, () => {
      arrived.set(a.issue, a.at)
      r.wake()
    })
  await clock.runUntil(w.horizonMs)
  drainReviewerRound()
  return {
    sys,
    starts,
    completed: [...completed],
    stats: { ...sys.stats },
    injected,
  }
}

// Every decision the runtime takes is a card's, on the virtual clock, except
// one: whether a slice is spent reads the host's real microseconds
// (actors.t27 section 8). Telemetry takes real time, as a slower host or a GC
// pause does, so with slices on it can move a yield, and a yield moves what is
// in flight when the horizon ends. With slices off nothing reads real time,
// and the run is the same to the last counter.
describe('telemetry changes no scheduling', () => {
  for (const [name, w] of [
    ['steady 45/h, 180 s, faults', FAULTS_180],
    ['burst 120 in 10 min + 45/h', BURST],
  ] as const)
    it(`the reviewer starts the same reviews at the same times with it off and on: ${name}`, async () => {
      const telemetry = { cards: [ACTORS_CARD, REVIEWER_CARD] }
      const off = await runReviewer(w, { slices: false })
      const on = await runReviewer(w, { slices: false, telemetry })
      on.sys.telemetry?.close()
      expect(on.starts.length).toBeGreaterThan(w.arrivals.length / 2)
      expect(on.starts).toEqual(off.starts)
      expect(on.completed).toEqual(off.completed)
      expect(on.stats).toEqual(off.stats)
      // with slices on, as in production: reported, not asserted, beside two
      // runs with telemetry off, which can differ from each other the same way
      const offS = await runReviewer(w)
      const offS2 = await runReviewer(w)
      const onS = await runReviewer(w, { telemetry })
      onS.sys.telemetry?.close()
      const same = (a: { starts: string[] }, b: { starts: string[] }) =>
        JSON.stringify(a.starts) === JSON.stringify(b.starts)
          ? 'same'
          : 'differ'
      console.log(
        `\n## scheduling, slices on (${name}): starts off/off ${same(offS, offS2)}, off/on ${same(offS, onS)}; yields ${offS.stats.yields}, ${offS2.stats.yields} off, ${onS.stats.yields} on`,
      )
    }, 120_000)
})

// lane 2's stop (turn_stop.t27): a killed turn is aborted, then escalated
it('with turnStop on, telemetry still changes no scheduling, and counts each stop', async () => {
  const telemetry = { cards: [ACTORS_CARD, REVIEWER_CARD] }
  const off = await runReviewer(FAULTS_180, { slices: false, turnStop: true })
  const on = await runReviewer(FAULTS_180, {
    slices: false,
    turnStop: true,
    telemetry,
  })
  const s = on.sys.telemetry?.snapshot() as unknown as Snap
  on.sys.telemetry?.close()
  expect(on.starts).toEqual(off.starts)
  expect(on.completed).toEqual(off.completed)
  expect(on.stats).toEqual(off.stats)
  expect(s.kinds['reviewer-worker'][MT_KILLS]).toBe(on.stats.killed)
}, 120_000)

describe('the decision log replays against the pinned cards', () => {
  it('every record of the seeded reviewer run replays to the same answer', async () => {
    const all: DecisionRecord[] = []
    const run = await runReviewer(FAULTS_180, {
      slices: false,
      telemetry: {
        cards: [ACTORS_CARD, REVIEWER_CARD],
        onRecord: (r) => all.push(r),
      },
    })
    const tel = run.sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    tel.close()
    const replay = replayDecisions(all)
    const cards = new Set(all.map((r) => r.card))
    const fns = new Set(all.map((r) => `${r.card}:${r.fn}`))
    const kinds = new Set(all.map((r) => r.kind))
    console.log(
      `\n## decision log replay (steady 45/h, 180 s, faults)\n${all.length} records, ${fns.size} functions on ${cards.size} cards, kinds ${[...kinds].sort().join(', ')}: verdict ${replay.verdict} (${RV_PASS} = pass), mismatches ${replay.mismatches}, unknown ${replay.unknownFns}, unsampled ${replay.unsampled}`,
    )
    expect(replay.verdict).toBe(RV_PASS)
    expect(replay.records).toBe(all.length)
    expect(cards).toEqual(new Set([ACTORS_CARD, REVIEWER_CARD, TELEMETRY_CARD]))
    // the decisions that matter are in it, not only the hot path
    for (const fn of [
      'queen/actors.wasm:on_child_exit',
      'queen/actors.wasm:turn_signal',
      'queen/actors.wasm:deliver',
      'queen/reviewer.wasm:review_slots',
      'queen/reviewer.wasm:visit_first',
      'queen/telemetry.wasm:turn_event',
    ])
      expect(fns).toContain(fn)
    // the kind follows a turn across its awaits: the intake's sort after its
    // reads, and the supervisor's own decisions under its name
    expect(
      all.some((r) => r.fn === 'visit_first' && r.kind === 'reviewer-intake'),
    ).toBe(true)
    expect(
      all.some((r) => r.fn === 'on_child_exit' && r.kind === 'reviewer-domain'),
    ).toBe(true)
    // the injected faults are seen, and none is invented: a crash or a stall
    // whose worker a rest_for_one restart already replaced ends unseen, as
    // its turn is no longer current
    const s = tel.snapshot() as unknown as Snap
    const w = s.kinds['reviewer-worker']
    console.log(
      `injected: ${run.injected.crash} crashes, ${run.injected.stall} stalls; counted for reviewer-worker: ${w[MT_CRASHES]} crashes, ${w[MT_KILLS]} kills, ${w[MT_RESTARTS]} restarts; events ${JSON.stringify(Object.fromEntries(Object.entries(s.kinds).map(([k, v]) => [k, v.events])))}`,
    )
    expect(w[MT_CRASHES]).toBeGreaterThan(0)
    expect(w[MT_CRASHES] as number).toBeLessThanOrEqual(run.injected.crash)
    expect(w[MT_KILLS]).toBeGreaterThan(0)
    expect(w[MT_KILLS] as number).toBeLessThanOrEqual(run.injected.stall)
    // the ring keeps the newest; the stream saw them all
    expect(tel.records().length).toBeLessThanOrEqual(all.length)
    // QUEEN_DECISION_SPEC_OUT=<t27 checkout>: this log becomes the three-way
    // conformance spec, specs/queen/replay/<card>_decisions.t27 there
    const out = process.env.QUEEN_DECISION_SPEC_OUT
    if (out) {
      const spec = writeDecisionSpecs(
        all,
        out,
        process.env.QUEEN_DECISION_SPEC_FROM ?? 'unrecorded',
        'the seeded reviewer run (seed 7852: 45 arrivals an hour for 6 h, 180 s median review, 5% crashes, 2% stalls; slices off)',
        process.env.QUEEN_DECISION_LOG_OUT,
      )
      console.log(
        `\n## decision log as t27 asserts\n${spec.logged} records, ${spec.unique} distinct (card, fn, args); conflicts ${spec.conflicts.length}, unwritable ${spec.unwritable.length}\n| card | fn | logged | distinct |\n|---|---|---|---|\n${spec.perFn.map((f) => `| ${f.card} | ${f.fn} | ${f.logged} | ${f.unique} |`).join('\n')}`,
      )
      expect(spec.conflicts).toEqual([])
      expect(spec.unwritable).toEqual([])
    }
  }, 120_000)

  it('the log becomes t27 asserts typed by the card, and a call answered two ways or a value its type cannot hold is refused', () => {
    const dir = mkdtempSync(join(tmpdir(), 'queen-decision-spec-'))
    const spec = join(dir, 'specs/queen/replay/reviewer_decisions.t27')
    const call: DecisionRecord = {
      card: REVIEWER_CARD,
      fn: 'review_slots',
      args: [1, 4],
      result: 3,
      kind: 'reviewer-intake',
      at: 0,
      n: 0,
    }
    const write = (rs: DecisionRecord[]) =>
      writeDecisionSpecs(rs, dir, 'x', 'a test')
    const once = write([call, call])
    expect([
      once.unique,
      once.conflicts.length,
      once.unwritable.length,
    ]).toEqual([1, 0, 0])
    expect(readFileSync(spec, 'utf8')).toContain(
      'assert review_slots(1, 4) == 3;',
    )
    // a u32 read back signed from the wasm is written unsigned, a bool as a bool
    write([
      { ...call, result: -1 },
      { ...call, fn: 'visit_first', args: [1, 0, 0, 24], result: 1 },
    ])
    expect(readFileSync(spec, 'utf8')).toContain(
      'assert review_slots(1, 4) == 4294967295;',
    )
    expect(readFileSync(spec, 'utf8')).toContain(
      'assert visit_first(true, 0, false, 24) == true;',
    )
    expect(write([call, { ...call, result: 4 }]).conflicts.length).toBe(1)
    expect(write([{ ...call, args: [-1, 4] }]).unwritable.length).toBe(1)
    expect(
      write([{ ...call, fn: 'visit_first', args: [2, 0, 0, 24] }]).unwritable
        .length,
    ).toBe(1)
    rmSync(dir, { recursive: true })
  })

  it('a corrupted, empty, foreign or unsampled log does not pass', async () => {
    const all: DecisionRecord[] = []
    const run = await runReviewer(FAULTS_180, {
      slices: false,
      telemetry: {
        cards: [ACTORS_CARD, REVIEWER_CARD],
        onRecord: (r) => all.push(r),
      },
    })
    run.sys.telemetry?.close()
    const some = all.slice(0, 200)
    expect(replayDecisions(some).verdict).toBe(RV_PASS)
    expect(replayDecisions([]).verdict).toBe(RV_EMPTY)
    const i = some.findIndex((r) => r.fn === 'review_slots')
    const wrong = some.map((r, j) =>
      j === i ? { ...r, result: Number(r.result) + 1 } : r,
    )
    expect(replayDecisions(wrong).verdict).toBe(RV_MISMATCH)
    const foreign = [...some, { ...some[0], fn: 'no_such_fn' }]
    expect(replayDecisions(foreign).verdict).toBe(RV_UNKNOWN_FN)
    const unsampled = [...some, { ...some[0], n: DLOG_PER_FN_PER_WINDOW }]
    expect(replayDecisions(unsampled).verdict).toBe(RV_UNSAMPLED)
    // and through JSON, as GET /queen/actors/decisions serves it
    const wire = JSON.parse(
      JSON.stringify(
        some.map((r) => ({
          ...r,
          args: r.args.map((a) => (typeof a === 'bigint' ? String(a) : a)),
          result: typeof r.result === 'bigint' ? String(r.result) : r.result,
        })),
      ),
    )
    expect(replayDecisions(wire).verdict).toBe(RV_PASS)
  }, 120_000)
})

describe('the cost per message, telemetry off and on', () => {
  // The ring of queen-actors-bench.test.ts: 1000 actors pass 100 000 messages,
  // every decision asked of the wasm card. Rounds alternate off and on, so
  // load on the host falls on both, and the medians are the result. Wall time
  // is what the bench has always printed; CPU time (process.cpuUsage) is the
  // same run read by the time this process ran, which other processes on a
  // busy host disturb less.
  const ringOnce = async (slices: boolean, telemetry: boolean) => {
    const sys = createActorSystem(undefined, {
      slices,
      ...(telemetry
        ? { telemetry: { cards: [ACTORS_CARD], summary: () => {} } }
        : {}),
    })
    const N = 1000
    const HOPS = 100_000
    const ring: bigint[] = []
    let left = HOPS
    let finish: () => void = () => {}
    const done = new Promise<void>((r) => {
      finish = r
    })
    for (let i = 0; i < N; i++)
      ring.push(
        sys.spawn<number>({
          name: `r${i}`,
          kind: 'ring',
          receive: (hop, self) => {
            left--
            if (left === 0) finish()
            else sys.send(ring[(i + 1) % N], hop + 1, self)
          },
        }),
      )
    const cpu0 = process.cpuUsage()
    const t0 = performance.now()
    sys.send(ring[0], 0)
    await done
    const wall = ((performance.now() - t0) * 1000) / HOPS
    const used = process.cpuUsage(cpu0)
    const cpu = (used.user + used.system) / HOPS
    expect(sys.stats.delivered).toBe(HOPS)
    sys.telemetry?.close()
    return { wall, cpu }
  }
  const median = (xs: number[]) => {
    const s = [...xs].sort((a, b) => a - b)
    return s[Math.floor(s.length / 2)]
  }

  it('a ring of 1000 actors passes 100 000 messages, off and on, slices off and on', async () => {
    const ROUNDS = Number(process.env.QUEEN_TELEMETRY_BENCH_ROUNDS ?? 9)
    const lines = [
      '',
      `## telemetry cost per message, us (${ROUNDS} rounds, each off then on)`,
      '| slices | wall off | wall on | cpu off | cpu on | cpu on/off per round: median [p25, p75] | cpu min off / on |',
      '|---|---|---|---|---|---|---|',
    ]
    for (const slices of [false, true]) {
      await ringOnce(slices, false)
      await ringOnce(slices, true)
      const off: Array<{ wall: number; cpu: number }> = []
      const on: Array<{ wall: number; cpu: number }> = []
      for (let k = 0; k < ROUNDS; k++) {
        off.push(await ringOnce(slices, false))
        on.push(await ringOnce(slices, true))
      }
      const ratios = on.map((r, k) => r.cpu / off[k].cpu).sort((a, b) => a - b)
      const q = (p: number) =>
        ratios[Math.min(ratios.length - 1, Math.floor(p * ratios.length))]
      const plus = (r: number) => `${((r - 1) * 100).toFixed(1)}%`
      const f = (xs: number[]) => median(xs).toFixed(2)
      const minOff = Math.min(...off.map((r) => r.cpu))
      const minOn = Math.min(...on.map((r) => r.cpu))
      lines.push(
        `| ${slices ? 'on' : 'off'} | ${f(off.map((r) => r.wall))} | ${f(on.map((r) => r.wall))} | ${f(off.map((r) => r.cpu))} | ${f(on.map((r) => r.cpu))} | ${plus(q(0.5))} [${plus(q(0.25))}, ${plus(q(0.75))}] | ${minOff.toFixed(2)} / ${minOn.toFixed(2)} (${plus(minOn / minOff)}) |`,
      )
    }
    console.log(lines.join('\n'))
  }, 600_000)
})
