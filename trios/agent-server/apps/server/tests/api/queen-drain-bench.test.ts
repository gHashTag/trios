/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A SIMULATED DEPLOY, TODAY AGAINST drain.t27 (trios#1729 item 2, numbers on
 * gHashTag/t27#7851).
 *
 * The swarm is the bee dispatcher's seeded world (queen-bee-sim.ts
 * `workload`: Poisson arrivals, lognormal bees around a 25 min median capped
 * at 0.9 x TURN_MAX_SECONDS, 8% fail and 4% crash at 0.3 of their length, 3%
 * hang until the turn bound kills them at TURN_MAX_SECONDS). Bees start in
 * arrival order on a free lane picked at random, and QUEEN_SLOTS of the lanes
 * are the runner inside the Queen's own container. Reviews follow the bees
 * that end: 4 workers, lognormal around 60 s, capped at 270 s, as the
 * reviewer benchmark draws them.
 *
 * A deploy is SIGTERM at an instant drawn uniformly over hours 2..8. The same
 * instants, with the same bees and reviews in flight, go through:
 *   today    the real drainBeeRunner (1800 s, sequential salvage), then the
 *            server's 2 s close; a bee cut at the deadline keeps its row
 *            claimed until its task lease lapses.
 *   bounded  the real drainBounded on the drain card (cap 1800 inside 1830,
 *            hand-backs side by side, row ended at once), then the 2 s close.
 * plus what the owner could choose, on the same card: a 600 s or 300 s runner
 * drain, and no bee in the Queen's container at all.
 *
 * Each salvage takes SALVAGE_MS and each bundle STORE_MS: assumptions, not
 * measurements (the salvage is up to seven git commands, the bundle one).
 * Everything runs on the virtual clock.
 */

import { describe, expect, it } from 'bun:test'
import type { Pool } from 'pg'
import { TURN_MAX_SECONDS } from '../../src/api/services/queen-actors-card.gen'
import {
  beeFreeAfterSeconds,
  drainBounded,
  drainCapSeconds,
} from '../../src/api/services/queen-drain'
import { LANE_FREE_MS } from '../../src/api/services/queen-drain-card.gen'
import {
  type BeeOrder,
  drainBeeRunner,
} from '../../src/api/services/queen-runner'
import { HOUR, workload } from './queen-bee-sim'
import { VirtualClock } from './queen-virtual-clock'

const SALVAGE_MS = 3_000
const STORE_MS = 2_000
const CLOSE_MS = 2_000
const PLATFORM_SECONDS = 1830
const RUNNER_DRAIN_SECONDS = 1800
const HEARTBEAT_MS = 60_000
const DEPLOYS = 300
const REVIEW_WORKERS = 4

const mulberry32 = (seed: number) => {
  let a = seed
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

interface Span {
  issue: number
  start: number
  end: number
}

interface Swarm {
  queenBees: Span[]
  reviews: Span[]
}

/** The swarm over 8 h: which bees ran in the Queen's own slots, and every review. */
function swarm(
  seed: number,
  o: { perHour: number; lanes: number; queenSlots: number },
): Swarm {
  const w = workload('drain', seed, {
    perHour: o.perHour,
    hours: 8,
    medianMinutes: 25,
    lanes: o.lanes,
    fail: 0.08,
    crash: 0.04,
    hang: 0.03,
  })
  const rnd = mulberry32(seed ^ 0x5eed)
  const freeAt = new Array<number>(o.lanes).fill(0)
  const queenBees: Span[] = []
  const ends: number[] = []
  for (const a of w.arrivals) {
    const first = w.attempts.get(a.issue)?.[0]
    if (!first) continue
    const len = first.seconds * 1000
    const ms =
      first.fate === 'ok'
        ? len
        : first.fate === 'hang'
          ? TURN_MAX_SECONDS * 1000
          : 0.3 * len
    const t = Math.max(a.at, Math.min(...freeAt))
    const free = freeAt.flatMap((f, i) => (f <= t ? [i] : []))
    const lane = free[Math.floor(rnd() * free.length)]
    freeAt[lane] = t + ms
    ends.push(t + ms)
    if (lane < o.queenSlots)
      queenBees.push({ issue: a.issue, start: t, end: t + ms })
  }
  // reviews of the bees that ended, 4 at a time, first come first served
  const normal = () =>
    Math.sqrt(-2 * Math.log(1 - rnd())) * Math.cos(2 * Math.PI * rnd())
  const workers = new Array<number>(REVIEW_WORKERS).fill(0)
  const reviews: Span[] = []
  ends.sort((x, y) => x - y)
  for (const [i, at] of ends.entries()) {
    const len =
      Math.min(270, Math.max(5, Math.round(60 * Math.exp(0.7 * normal())))) *
      1000
    const k = workers.indexOf(Math.min(...workers))
    const start = Math.max(at, workers[k])
    workers[k] = start + len
    reviews.push({ issue: i, start, end: start + len })
  }
  return { queenBees, reviews }
}

interface Outcome {
  exitMs: number
  cut: Array<{ issue: number; ageMs: number; freeAfterS: number }>
  reviewLostMs: number
  idleLaneMs: number
}

const order = (issue: number): BeeOrder => ({
  issue,
  branch: `queen/${issue}`,
  brief: '',
  ownedPaths: [],
  conversationId: `c${issue}`,
  keyIndex: 1001,
})

/** What both policies share once the exit time is known. */
function settle(
  s: Swarm,
  t0: number,
  exitMs: number,
  queenSlots: number,
): Pick<Outcome, 'reviewLostMs' | 'idleLaneMs'> {
  const exitAt = t0 + exitMs
  let reviewLostMs = 0
  for (const r of s.reviews)
    if (r.start <= exitAt && exitAt < r.end) reviewLostMs += exitAt - r.start
  let busy = 0
  for (const b of s.queenBees)
    if (b.start <= t0 && t0 < b.end) busy += Math.min(b.end, exitAt) - t0
  return { reviewLostMs, idleLaneMs: queenSlots * exitMs - busy }
}

async function today(s: Swarm, t0: number, queenSlots: number) {
  const clock = new VirtualClock()
  const vsleep = (ms: number) =>
    new Promise<void>((resolve) => clock.after(ms, resolve))
  const orders = new Map<number, BeeOrder>()
  const live = s.queenBees.filter((b) => b.start <= t0 && t0 < b.end)
  for (const b of live) {
    orders.set(b.issue, order(b.issue))
    clock.after(b.end - t0, () => orders.delete(b.issue))
  }
  const cut: Outcome['cut'] = []
  let exitMs = -1
  void drainBeeRunner({} as Pool, 'queen', {
    seconds: RUNNER_DRAIN_SECONDS,
    now: clock.now,
    sleep: vsleep,
    orders,
    salvage: async (_pool, issue) => {
      const b = live.find((x) => x.issue === issue) as Span
      cut.push({ issue, ageMs: t0 + clock.now() - b.start, freeAfterS: 0 })
      await vsleep(SALVAGE_MS)
      return { committed: true, files: [], left: [], sha: null, detail: '' }
    },
    store: async () => {
      await vsleep(STORE_MS)
    },
  }).then(() => {
    exitMs = clock.now() + CLOSE_MS
  })
  await clock.runUntil(3 * HOUR)
  // left claimed: free when its task lease lapses, counted from its last
  // heartbeat (every 60 s from its claim, until the process ended)
  for (const c of cut) {
    const b = live.find((x) => x.issue === c.issue) as Span
    const sinceBeat = Math.floor(
      ((t0 + exitMs - b.start) % HEARTBEAT_MS) / 1000,
    )
    c.freeAfterS = beeFreeAfterSeconds(false, sinceBeat)
  }
  return { exitMs, cut, ...settle(s, t0, exitMs, queenSlots) }
}

async function bounded(
  s: Swarm,
  t0: number,
  queenSlots: number,
  runnerDrainSeconds: number,
) {
  const clock = new VirtualClock()
  const vsleep = (ms: number) =>
    new Promise<void>((resolve) => clock.after(ms, resolve))
  const running = new Map<number, Span>()
  for (const b of s.queenBees)
    if (b.start <= t0 && t0 < b.end) {
      running.set(b.issue, b)
      clock.after(b.end - t0, () => running.delete(b.issue))
    }
  const cut: Outcome['cut'] = []
  let reviewsLive = s.reviews.filter((r) => r.start <= t0 && t0 < r.end).length
  let exitMs = -1
  void drainBounded({
    now: clock.now,
    sleep: vsleep,
    capSeconds: drainCapSeconds(runnerDrainSeconds, PLATFORM_SECONDS),
    roundsStartBeesHere: false,
    stopClaims: () => {},
    bees: () =>
      [...running.values()].map((b) => ({
        issue: b.issue,
        conversationId: `c${b.issue}`,
      })),
    handBack: async (bee) => {
      const b = running.get(bee.issue) as Span
      const ageMs = t0 + clock.now() - b.start
      await vsleep(SALVAGE_MS + STORE_MS)
      running.delete(bee.issue)
      // ended now: free at once
      cut.push({
        issue: bee.issue,
        ageMs,
        freeAfterS: beeFreeAfterSeconds(true, 0),
      })
    },
    reviews: () => reviewsLive,
    stopRest: async () => {
      // the reviews in flight then are aborted; a model call answers its
      // abort in under 8 ms (turn_stop.t27's benchmark), 100 ms here
      const now = t0 + clock.now()
      reviewsLive = s.reviews.filter(
        (r) => r.start <= now && now < r.end,
      ).length
      if (reviewsLive > 0)
        clock.after(100, () => {
          reviewsLive = 0
        })
    },
  }).then(() => {
    exitMs = clock.now() + CLOSE_MS
  })
  await clock.runUntil(3 * HOUR)
  return { exitMs, cut, ...settle(s, t0, exitMs, queenSlots) }
}

const pct = (xs: number[], p: number) => {
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]
}
const mean = (xs: number[]) =>
  xs.length === 0 ? 0 : xs.reduce((a, b) => a + b, 0) / xs.length

function row(label: string, os: Outcome[], cap: number) {
  const exits = os.map((o) => o.exitMs / 1000)
  const cuts = os.flatMap((o) => o.cut)
  return {
    label,
    p50: pct(exits, 50),
    p95: pct(exits, 95),
    max: Math.max(...exits),
    atCap: os.filter((o) => o.exitMs / 1000 >= cap).length / os.length,
    killed: os.filter((o) => o.exitMs / 1000 > PLATFORM_SECONDS).length,
    beesCut: cuts.length / os.length,
    cutAgeMin: mean(cuts.map((c) => c.ageMs / 60000)),
    freeAfterS: mean(cuts.map((c) => c.freeAfterS)),
    reviewLostS: mean(os.map((o) => o.reviewLostMs / 1000)),
    idleLaneMin: mean(os.map((o) => o.idleLaneMs / 60000)),
  }
}

const fmt = (r: ReturnType<typeof row>) =>
  `| ${r.label} | ${r.p50.toFixed(0)} | ${r.p95.toFixed(0)} | ${r.max.toFixed(0)} | ${(r.atCap * 100).toFixed(0)}% | ${r.killed} | ${r.beesCut.toFixed(2)} | ${r.cutAgeMin.toFixed(1)} | ${r.freeAfterS.toFixed(0)} | ${r.reviewLostS.toFixed(0)} | ${r.idleLaneMin.toFixed(1)} |`

const SCENARIOS = [
  { name: 'quiet: 6 bees/h, 70 lanes', perHour: 6, lanes: 70 },
  { name: 'busy (measured): 58 bees/h, 70 lanes', perHour: 58, lanes: 70 },
  { name: 'saturated: 58 bees/h, 16 lanes', perHour: 58, lanes: 16 },
]
// TRIOS_QUEEN_RUNNER_SLOTS is not known here: 4 by default, and the
// sensitivity is one variable away (QUEEN_DRAIN_BENCH_SLOTS=8 bun test ...)
const QUEEN_SLOTS = Number(process.env.QUEEN_DRAIN_BENCH_SLOTS ?? 4)

describe('a simulated deploy: the drain today against drain.t27', () => {
  for (const sc of SCENARIOS) {
    it(sc.name, async () => {
      const s = swarm(1729, { ...sc, queenSlots: QUEEN_SLOTS })
      const none = swarm(1729, { ...sc, queenSlots: 0 })
      const rnd = mulberry32(8575)
      const instants = Array.from(
        { length: DEPLOYS },
        () => 2 * HOUR + Math.floor(rnd() * 6 * HOUR),
      )
      const t: Outcome[] = []
      const b: Outcome[] = []
      const b600: Outcome[] = []
      const b300: Outcome[] = []
      const b0: Outcome[] = []
      for (const t0 of instants) {
        t.push(await today(s, t0, QUEEN_SLOTS))
        b.push(await bounded(s, t0, QUEEN_SLOTS, RUNNER_DRAIN_SECONDS))
        b600.push(await bounded(s, t0, QUEEN_SLOTS, 600))
        b300.push(await bounded(s, t0, QUEEN_SLOTS, 300))
        b0.push(await bounded(none, t0, 0, RUNNER_DRAIN_SECONDS))
      }
      const rows = [
        row('today (runner drain 1800)', t, 1800),
        row('bounded, card defaults (cap 1800)', b, 1800),
        row('bounded, owner sets runner drain 600', b600, 600),
        row('bounded, owner sets runner drain 300', b300, 300),
        row('bounded, Queen container runs no bee', b0, 1800),
      ]
      console.log(
        [
          `\n## ${sc.name}, ${QUEEN_SLOTS} of its lanes in the Queen's container, ${DEPLOYS} deploys`,
          '| policy | exit p50 s | exit p95 s | exit max s | at cap | SIGKILLed | bees cut / deploy | cut bee age min | issue free after s | review s lost / deploy | Queen lane-min idle / deploy |',
          '|---|---|---|---|---|---|---|---|---|---|---|',
          ...rows.map(fmt),
        ].join('\n'),
      )

      // the card's defaults cut no bee today would not cut; they exit no
      // later than today but for the wait on stopped reviews (LANE_FREE_MS
      // at most), which today cuts by exiting
      for (const [i, o] of b.entries()) {
        expect(o.cut.map((c) => c.issue).sort()).toEqual(
          t[i].cut.map((c) => c.issue).sort(),
        )
        expect(o.exitMs).toBeLessThanOrEqual(t[i].exitMs + LANE_FREE_MS)
        expect(o.exitMs / 1000).toBeLessThanOrEqual(PLATFORM_SECONDS)
        for (const c of o.cut) expect(c.freeAfterS).toBe(0)
      }
      // a Queen container that runs no bee leaves within a few seconds
      for (const o of b0) expect(o.exitMs).toBeLessThanOrEqual(5_000 + CLOSE_MS)
    }, 600_000)
  }
})
