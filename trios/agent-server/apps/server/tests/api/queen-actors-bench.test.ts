/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BENCHMARK BEFORE THE SWAP (t27#7851). The owner's rule: a loop is
 * replaced only after the actor runtime runs the same workload and the
 * numbers say it is at least as good.
 *
 * Each scenario is drawn once from a fixed seed: arrivals (a bee finished,
 * its row waits for a review), and for each row the fate of each attempt
 * (ok, crash, stall) and its length. The same draw is then run three ways:
 *   loop            queen-review-loop.ts createReviewer, one pass every
 *                   REVIEWER_EVERY_SECONDS (what runs today)
 *   actors          queen-review-actors.ts under a root supervisor, woken by
 *                   the bus on each arrival and by its heartbeat
 *   actors-hb-only  the same tree with no bus wake, heartbeat only. This
 *                   separates what the runtime does from what the bus does.
 * All three run on a virtual clock, so seven hours of Queen time take
 * milliseconds and every run of the same input sees the same order.
 *
 * A stall is a review step that hangs for STALL_SECONDS with no timeout of its
 * own. The loop cannot end a promise, so the slot is held until it returns.
 * The actor runtime kills the turn at REVIEW_ROW_SECONDS, and the row goes to
 * another worker. A late result from an abandoned attempt is fenced, as
 * control.t27's lease fences it: only the newest attempt on a row may finish
 * it.
 *
 * The table is printed. Set QUEEN_BENCH_OUT to write it as JSON as well. The
 * asserts check only that every run accounts for its input. Which runtime
 * wins is the output, not a gate.
 */

import { describe, expect, it } from 'bun:test'
import { writeFileSync } from 'node:fs'
import {
  createActorSystem,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
} from '../../src/api/services/queen-actors-card.gen'
import { reviewerTree } from '../../src/api/services/queen-review-actors'
import {
  createReviewer,
  drainReviewerRound,
} from '../../src/api/services/queen-review-loop'
import {
  REVIEW_ROW_SECONDS,
  REVIEWER_EVERY_SECONDS,
} from '../../src/api/services/queen-reviewer-card.gen'
import { VirtualClock } from './queen-virtual-clock'

const STALL_SECONDS = 1800
const HOUR = 3_600_000

type Fate = 'ok' | 'crash' | 'stall'
interface Attempt {
  fate: Fate
  seconds: number
}
interface Workload {
  name: string
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
  name: string,
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
      // a review that works ends inside its row deadline: the sweep stops starting steps at
      // REVIEW_ROW_SECONDS and a model call times out at 120 s. Only a stall runs past it.
      const seconds = Math.min(
        Math.round(REVIEW_ROW_SECONDS * 0.9),
        Math.max(5, Math.round(o.medianSeconds * Math.exp(0.7 * normal()))),
      )
      list.push({ fate, seconds })
    }
    attempts.set(a.issue, list)
  }
  return { name, arrivals, attempts, horizonMs: (o.hours + 1) * HOUR }
}

type Runtime = 'loop' | 'actors' | 'actors-hb-only'

interface Result {
  scenario: string
  runtime: Runtime
  arrived: number
  done: number
  open: number
  perHour: number
  waitP50: number
  waitP95: number
  doneP50: number
  doneP95: number
  recoveryP50: number
  recoveryMax: number
  stallsStarted: number
  killed: number
  deadLetters: number
  gaveUp: number
}

const pct = (xs: number[], p: number) => {
  if (xs.length === 0) return 0
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]
}
const sec = (ms: number) => Math.round(ms / 1000)
const judged = () => ({ acted: [], strays: [], tally: [] })

async function simulate(w: Workload, runtime: Runtime): Promise<Result> {
  const clock = new VirtualClock()
  const arrived = new Map<number, number>()
  const completed = new Map<number, number>()
  const firstStart = new Map<number, number>()
  const attemptOf = new Map<number, number>()
  const fence = new Map<number, number>()
  const starts: Array<{ issue: number; at: number }> = []
  const crashes: Array<{ issue: number; at: number }> = []
  let stallsStarted = 0

  const waiting = async () =>
    [...arrived.keys()].filter((n) => !completed.has(n))
  const reviewOne = (issue: number) => {
    const now = clock.now()
    if (!firstStart.has(issue)) firstStart.set(issue, now)
    starts.push({ issue, at: now })
    const k = attemptOf.get(issue) ?? 0
    attemptOf.set(issue, k + 1)
    const token = (fence.get(issue) ?? 0) + 1
    fence.set(issue, token)
    const list = w.attempts.get(issue) as Attempt[]
    const a = list[k % list.length]
    if (a.fate === 'stall') stallsStarted++
    const ms =
      a.fate === 'stall'
        ? STALL_SECONDS * 1000
        : a.fate === 'crash'
          ? a.seconds * 300
          : a.seconds * 1000
    return new Promise<ReturnType<typeof judged>>((resolve, reject) =>
      clock.after(ms, () => {
        if (a.fate === 'crash') {
          // a crash counts when the attempt still holds its row: an abandoned one is fenced
          if (fence.get(issue) === token && !completed.has(issue))
            crashes.push({ issue, at: clock.now() })
          reject(new Error('review crashed'))
          return
        }
        if (fence.get(issue) === token && !completed.has(issue))
          completed.set(issue, clock.now())
        resolve(judged())
      }),
    )
  }

  let killed = 0
  let deadLetters = 0
  let gaveUp = 0
  let wake = () => {}
  if (runtime === 'loop') {
    const reviewer = createReviewer({
      holdsLease: async () => true,
      waiting,
      reviewOne: (issue) => reviewOne(issue),
      now: clock.now,
    })
    const tick = () => {
      void reviewer.pass()
      clock.after(REVIEWER_EVERY_SECONDS * 1000, tick)
    }
    clock.after(REVIEWER_EVERY_SECONDS * 1000, tick)
  } else {
    const sys = createActorSystem(clock)
    const r = reviewerTree(sys, {
      holdsLease: async () => true,
      waiting,
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
    ).start(() => {
      gaveUp++
    })
    if (runtime === 'actors') wake = r.wake
    const stats = sys.stats
    const read = () => {
      killed = stats.killed
      deadLetters = stats.deadLetters
    }
    clock.after(w.horizonMs, read)
  }
  for (const a of w.arrivals)
    clock.after(a.at, () => {
      arrived.set(a.issue, a.at)
      wake()
    })

  await clock.runUntil(w.horizonMs)
  drainReviewerRound()

  const wait = [...firstStart].map(([n, at]) => at - (arrived.get(n) as number))
  const done = [...completed].map(([n, at]) => at - (arrived.get(n) as number))
  const recovery = crashes.map((c) => {
    const next = starts.find((s) => s.issue === c.issue && s.at >= c.at)
    return next ? next.at - c.at : w.horizonMs - c.at
  })
  return {
    scenario: w.name,
    runtime,
    arrived: arrived.size,
    done: completed.size,
    open: arrived.size - completed.size,
    perHour: Math.round((completed.size / (w.horizonMs / HOUR)) * 10) / 10,
    waitP50: sec(pct(wait, 50)),
    waitP95: sec(pct(wait, 95)),
    doneP50: sec(pct(done, 50)),
    doneP95: sec(pct(done, 95)),
    recoveryP50: sec(pct(recovery, 50)),
    recoveryMax: sec(Math.max(0, ...recovery)),
    stallsStarted,
    killed,
    deadLetters,
    gaveUp,
  }
}

const SCENARIOS: Workload[] = [
  workload('steady 45/h, 60 s reviews, faults', 7851, {
    perHour: 45,
    hours: 6,
    medianSeconds: 60,
    crash: 0.05,
    stall: 0.02,
  }),
  workload('steady 45/h, 180 s reviews, faults', 7852, {
    perHour: 45,
    hours: 6,
    medianSeconds: 180,
    crash: 0.05,
    stall: 0.02,
  }),
  workload('steady 45/h, 180 s reviews, no faults', 7853, {
    perHour: 45,
    hours: 6,
    medianSeconds: 180,
    crash: 0,
    stall: 0,
  }),
  workload('overload 120/h, 180 s reviews, faults', 7854, {
    perHour: 120,
    hours: 6,
    medianSeconds: 180,
    crash: 0.05,
    stall: 0.02,
  }),
  workload('burst 120 in 10 min + 45/h, 180 s, faults', 7855, {
    perHour: 45,
    hours: 6,
    medianSeconds: 180,
    crash: 0.05,
    stall: 0.02,
    burst: 120,
  }),
]

describe('the reviewer: loop against actors, same input', () => {
  it('runs every scenario three ways and accounts for every row', async () => {
    const results: Result[] = []
    for (const w of SCENARIOS)
      for (const runtime of ['loop', 'actors', 'actors-hb-only'] as Runtime[]) {
        const r = await simulate(w, runtime)
        expect(r.arrived).toBe(w.arrivals.length)
        expect(r.done + r.open).toBe(r.arrived)
        results.push(r)
      }
    const cols: Array<keyof Result> = [
      'runtime',
      'done',
      'open',
      'perHour',
      'waitP50',
      'waitP95',
      'doneP50',
      'doneP95',
      'recoveryP50',
      'recoveryMax',
      'stallsStarted',
      'killed',
      'gaveUp',
    ]
    const lines: string[] = []
    for (const w of SCENARIOS) {
      lines.push(
        `\n## ${w.name} (${w.arrivals.length} rows, horizon ${w.horizonMs / HOUR} h)`,
      )
      lines.push(`| ${cols.join(' | ')} |`)
      lines.push(`|${cols.map(() => '---').join('|')}|`)
      for (const r of results.filter((x) => x.scenario === w.name))
        lines.push(`| ${cols.map((c) => String(r[c])).join(' | ')} |`)
    }
    console.log(lines.join('\n'))
    const out = process.env.QUEEN_BENCH_OUT
    if (out) writeFileSync(out, JSON.stringify(results, null, 2))
  }, 600_000)
})

describe('the runtime cost per message', () => {
  it('a ring of 1000 actors passes 100 000 messages, every decision asked of the wasm card', async () => {
    const sys = createActorSystem()
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
          receive: (hop, self) => {
            left--
            if (left === 0) finish()
            else sys.send(ring[(i + 1) % N], hop + 1, self)
          },
        }),
      )
    const t0 = performance.now()
    sys.send(ring[0], 0)
    await done
    const ms = performance.now() - t0
    const perSecond = Math.round(HOPS / (ms / 1000))
    console.log(
      `\n## runtime cost\n${HOPS} messages through ${N} actors in ${Math.round(ms)} ms: ${perSecond} messages/s, ${((ms * 1000) / HOPS).toFixed(1)} us per message`,
    )
    expect(sys.stats.delivered).toBe(HOPS)
  }, 600_000)
})
