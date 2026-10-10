/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A SEEDED WORLD FOR THE BEE DISPATCHER (gHashTag/trios#1712 item 7,
 * gHashTag/t27#7851). One draw of issues and bee fates, run three ways on a
 * virtual clock:
 *   loop          the round as queen-tick.ts runs it: the real round gate
 *                 (createRoundGate), asked by a timer and by every bee that
 *                 reports its end. A round reads the issues, reclaims leases
 *                 that lapsed, reaps runner tasks past RUNNER_CAP_MINUTES,
 *                 and then dispatches one bee after another (one queend call
 *                 and one dispatch each) until no lane is free.
 *   actors-round  queen-bee-actors.ts, the MVP: the same round still reads
 *                 the issues and hands them over where it used to dispatch;
 *                 the actors start the bees, and everything after a start
 *                 (an end, a retry, a freed lane, a kill) is their own event.
 *   actors-event  the same actors, told of an issue the moment it is ready
 *                 (an issues webhook, which is not wired): what the round's
 *                 cadence still costs the MVP.
 *
 * The world is ground truth. A bee is a runner's process: it works, fails,
 * crashes with its runner (no word, no more heartbeats), or hangs (alive,
 * vouched for, no progress). A stopped bee ends at its runner's next poll,
 * as waitForEnding polls every 15 s. Leases follow control.t27 claim_lands:
 * one row per issue, a holder, a fence, renewed every 60 s while the bee's
 * runner lives, expired 180 s after the last renewal. A DUPLICATE PICKUP is a
 * bee started on an issue while another bee on that issue still runs. The
 * world counts it from the bees themselves, not from either dispatcher's
 * books.
 *
 * Round costs are assumptions, the same for every runtime: a tick every
 * 60 s, 2 s to read the issues, 25 s from the round's start to its dispatch
 * step, 0.5 s per queend call, 1 s per dispatch (claim, row, offer).
 */

import { createActorSystem } from '../../src/api/services/queen-actors'
import { TURN_MAX_SECONDS } from '../../src/api/services/queen-actors-card.gen'
import {
  type BeeEnd,
  type BeeStart,
  beeDispatcher,
} from '../../src/api/services/queen-bee-actors'
import { loadCardWasm, u32 } from '../../src/api/services/queen-card-wasm'
import {
  EF_CANCEL,
  EF_NONE,
  EF_PROVIDER,
  LEASE_TTL_SECONDS,
  RD_GIVE_UP,
} from '../../src/api/services/queen-dispatch-exit-card.gen'
import { RUNNER_CAP_MINUTES } from '../../src/api/services/queen-runner-work'
import { createRoundGate } from '../../src/api/services/queen-tick'
import { VirtualClock } from './queen-virtual-clock'

export const HOUR = 3_600_000
export const TICK_MS = 60_000
export const READ_MS = 2_000
export const PREPARE_MS = 25_000
export const QUEEND_MS = 500
export const DISPATCH_MS = 1_000
export const REPORT_MS = 2_000
export const RUNNER_POLL_MS = 15_000
export const HEARTBEAT_MS = 60_000
const TTL_MS = LEASE_TTL_SECONDS * 1000
const SAMPLE_MS = 10_000

export type Fate = 'ok' | 'fail' | 'crash' | 'hang'
export interface Attempt {
  fate: Fate
  seconds: number
}
export interface Workload {
  name: string
  arrivals: Array<{ issue: number; at: number }>
  attempts: Map<number, Attempt[]>
  lanes: number
  prepareMs: number
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

export function workload(
  name: string,
  seed: number,
  o: {
    perHour: number
    hours: number
    medianMinutes: number
    fail: number
    crash: number
    hang: number
    lanes: number
    burst?: number
    prepareSeconds?: number
    drainHours?: number
  },
): Workload {
  const rnd = mulberry32(seed)
  const normal = () =>
    Math.sqrt(-2 * Math.log(1 - rnd())) * Math.cos(2 * Math.PI * rnd())
  const arrivals: Workload['arrivals'] = []
  let issue = 10_000
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
        u < o.fail
          ? 'fail'
          : u < o.fail + o.crash
            ? 'crash'
            : u < o.fail + o.crash + o.hang
              ? 'hang'
              : 'ok'
      // a bee that works ends inside its turn bound (actors.t27
      // TURN_MAX_SECONDS); only a hang runs past it
      const seconds = Math.min(
        Math.round(TURN_MAX_SECONDS * 0.9),
        Math.max(
          60,
          Math.round(o.medianMinutes * 60 * Math.exp(0.7 * normal())),
        ),
      )
      list.push({ fate, seconds })
    }
    attempts.set(a.issue, list)
  }
  return {
    name,
    arrivals,
    attempts,
    lanes: o.lanes,
    prepareMs: (o.prepareSeconds ?? PREPARE_MS / 1000) * 1000,
    horizonMs: (o.hours + (o.drainHours ?? 4)) * HOUR,
  }
}

/**
 * `loop-cap60` is the loop with its runner cap cut from RUNNER_CAP_MINUTES to
 * the actors' turn bound, TURN_MAX_SECONDS: it separates what the bound does
 * from what the actors do.
 */
export type Runtime = 'loop' | 'loop-cap60' | 'actors-round' | 'actors-event'

export interface Result {
  scenario: string
  runtime: Runtime
  lanes: number
  arrived: number
  done: number
  perHour: number
  neverStarted: number
  duplicates: number
  waitP50: number
  waitP95: number
  readyToStartP95: number
  crashRecoveryP50: number
  crashRecoveryMax: number
  hangRecoveryP50: number
  failRetryP50: number
  effective: number
  occupied: number
  nominal: number
  stops: number
  gaveUp: number
  standDowns: number
  rounds: number
}

type How = 'ok' | 'fail' | 'stopped'
interface Bee {
  issue: number
  fate: Fate
  start: number
  /** when it stops making progress */
  stallAt: number
  /** when its process is gone: finished, failed, crashed, or stopped */
  endAt?: number
  /** its runner reports the end (not for a crash) */
  onReport: Array<(how: How) => void>
}
interface Lease {
  holder: string
  fence: number
  claimedAt: number
  released: boolean
  bee?: Bee
}

const keyed = () => loadCardWasm('queen/keyed.wasm')
const laneFree = (running: number, lanes: number) =>
  keyed().call('lane_free', u32(running), u32(lanes)) !== 0

const pct = (xs: number[], p: number) => {
  if (xs.length === 0) return 0
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]
}
const sec = (ms: number) => Math.round(ms / 1000)

export interface Options {
  /** Negative control: every actor claims as the process, not as its pid. */
  processHolder?: boolean
  /** Negative control: two dispatcher nodes over one lease table. */
  nodes?: number
}

export async function simulate(
  w: Workload,
  runtime: Runtime,
  opt: Options = {},
): Promise<Result> {
  const clock = new VirtualClock()
  const now = () => clock.now()
  const sleep = (ms: number) =>
    new Promise<void>((resolve) => clock.after(ms, resolve))
  const arrived = new Map<number, number>()
  const completed = new Map<number, number>()
  const used = new Map<number, number>()
  const live = new Map<number, Set<Bee>>()
  const bees: Bee[] = []
  const starts: Array<{ issue: number; at: number; readyAt: number }> = []
  const lastStop = new Map<number, number>()
  const crashes: Array<{ issue: number; at: number }> = []
  const hangs: Array<{ issue: number; at: number }> = []
  const fails: Array<{ issue: number; at: number }> = []
  const leases = new Map<number, Lease>()
  let duplicates = 0
  let stops = 0
  let occupiedSum = 0
  let samples = 0

  // ---- the bees, as their runners see them
  const end = (b: Bee, how: How | 'crash') => {
    if (b.endAt !== undefined) return
    b.endAt = now()
    lastStop.set(b.issue, now())
    live.get(b.issue)?.delete(b)
    if (how === 'ok' && !completed.has(b.issue)) completed.set(b.issue, now())
    if (how === 'crash') return
    // the row closes, so the claim goes with it
    const l = leases.get(b.issue)
    if (l?.bee === b) l.released = true
    for (const told of b.onReport) told(how)
  }
  const startBee = (issue: number): Bee => {
    const k = used.get(issue) ?? 0
    used.set(issue, k + 1)
    const list = w.attempts.get(issue) as Attempt[]
    const a = list[k % list.length]
    const set = live.get(issue) ?? new Set<Bee>()
    if (set.size > 0) duplicates++
    const t = now()
    const len = a.seconds * 1000
    const b: Bee = {
      issue,
      fate: a.fate,
      start: t,
      stallAt: Number.POSITIVE_INFINITY,
      onReport: [],
    }
    set.add(b)
    live.set(issue, set)
    bees.push(b)
    starts.push({
      issue,
      at: t,
      readyAt: lastStop.get(issue) ?? (arrived.get(issue) as number),
    })
    if (a.fate === 'ok') {
      b.stallAt = t + len
      clock.after(len, () => end(b, 'ok'))
    } else if (a.fate === 'fail') {
      b.stallAt = t + 0.3 * len
      clock.after(0.3 * len, () => {
        fails.push({ issue, at: now() })
        end(b, 'fail')
      })
    } else if (a.fate === 'crash') {
      b.stallAt = t + 0.5 * len
      clock.after(0.5 * len, () => {
        crashes.push({ issue, at: now() })
        end(b, 'crash')
      })
    } else {
      b.stallAt = t + 0.2 * len
      clock.after(0.2 * len, () => hangs.push({ issue, at: now() }))
    }
    return b
  }
  /** Ask a bee to stop: its runner aborts it at its next poll. */
  const stop = (b: Bee) => {
    if (b.endAt !== undefined) return
    stops++
    const polls = Math.floor((now() - b.start) / RUNNER_POLL_MS) + 1
    clock.after(b.start + polls * RUNNER_POLL_MS - now(), () =>
      end(b, 'stopped'),
    )
  }

  // ---- leases (control.t27 section 2)
  const renewedAt = (l: Lease): number => {
    const b = l.bee
    if (!b) return l.claimedAt
    const until = Math.min(now(), b.endAt ?? now())
    return Math.max(
      l.claimedAt,
      b.start + Math.floor((until - b.start) / HEARTBEAT_MS) * HEARTBEAT_MS,
    )
  }
  const expired = (issue: number) => {
    const l = leases.get(issue)
    return !l || l.released || now() - renewedAt(l) >= TTL_MS
  }
  const claim = (issue: number, holder: string): boolean => {
    const l = leases.get(issue)
    if (l && !expired(issue) && l.holder !== holder) return false
    leases.set(issue, {
      holder,
      fence: (l?.fence ?? 0) + 1,
      claimedAt: now(),
      released: false,
    })
    return true
  }

  const open = () =>
    [...arrived.keys()]
      .filter((n) => !completed.has(n))
      .sort((a, b) => (arrived.get(a) as number) - (arrived.get(b) as number))

  let rounds = 0
  let occupied = () => 0
  let gaveUp = () => 0
  let standDowns = () => 0
  let onArrival = (_issue: number) => {}

  if (runtime === 'loop' || runtime === 'loop-cap60') {
    const capMs =
      runtime === 'loop' ? RUNNER_CAP_MINUTES * 60_000 : TURN_MAX_SECONDS * 1000
    const rows = new Map<number, { bee: Bee; at: number }>()
    occupied = () => rows.size
    let gate: ReturnType<typeof createRoundGate>
    /** The round's housekeeping before it chooses. */
    const reclaimAndReap = () => {
      // control pass: a runner silent for a whole TTL loses its task
      for (const [issue, row] of rows)
        if (expired(issue) && row.bee.endAt !== undefined) rows.delete(issue)
      // reapSilentRunners: no runner holds an issue past RUNNER_CAP_MINUTES
      for (const [issue, row] of rows) {
        if (now() - row.at < capMs) continue
        rows.delete(issue)
        const l = leases.get(issue)
        if (l) l.released = true
        stop(row.bee)
      }
    }
    /** One dispatch, as dispatchBee does it: claim, row, start. */
    const dispatch = (pick: number) => {
      // the Queen claims as herself: a claim she holds lands again
      if (!claim(pick, 'queen')) return
      const bee = startBee(pick)
      ;(leases.get(pick) as Lease).bee = bee
      rows.set(pick, { bee, at: now() })
      bee.onReport.push(() => {
        if (rows.get(pick)?.bee === bee) rows.delete(pick)
        gate.request('a bee ended')
      })
    }
    const round = async () => {
      rounds++
      await sleep(READ_MS)
      const seen = open()
      reclaimAndReap()
      await sleep(w.prepareMs - READ_MS)
      for (;;) {
        await sleep(QUEEND_MS)
        if (!laneFree(rows.size, w.lanes)) break
        const pick = seen.find((n) => !rows.has(n) && !completed.has(n))
        if (pick === undefined) break
        await sleep(DISPATCH_MS)
        dispatch(pick)
      }
      await sleep(REPORT_MS)
    }
    gate = createRoundGate(round)
    const tick = () => {
      gate.request('periodic tick')
      clock.after(TICK_MS, tick)
    }
    tick()
  } else {
    const nodes = opt.nodes ?? 1
    const givenUp = new Set<number>()
    let wakeRound = () => {}
    const dispatchers = Array.from({ length: nodes }, (_, node) => {
      const sys = createActorSystem(clock, { slices: false, node })
      return beeDispatcher(sys, {
        lanes: () => Math.ceil(w.lanes / nodes),
        // the store remembers what an incarnation forgets: done and given up
        admit: async (issue) => {
          await sleep(QUEEND_MS)
          return !completed.has(issue) && !givenUp.has(issue)
        },
        observe: (e) => {
          if (e.kind !== 'ended') return
          if (e.decision === RD_GIVE_UP) givenUp.add(e.issue)
          wakeRound()
        },
        order: (waiting) =>
          [...waiting].sort(
            (a, b) => (arrived.get(a) as number) - (arrived.get(b) as number),
          ),
        holderPrefix: `node${node}`,
        start: async (issue, holder): Promise<BeeStart> => {
          await sleep(DISPATCH_MS)
          const who = opt.processHolder ? 'queen' : holder
          if (completed.has(issue) || !claim(issue, who)) {
            const l = leases.get(issue)
            return {
              claim: false,
              work: null,
              holderSinceRenewal: l ? sec(now() - renewedAt(l)) : 0,
            }
          }
          const bee = startBee(issue)
          ;(leases.get(issue) as Lease).bee = bee
          const ended = new Promise<BeeEnd>((resolve) => {
            bee.onReport.push((how) =>
              resolve({
                completion: how === 'ok',
                errorFrame:
                  how === 'fail'
                    ? EF_PROVIDER
                    : how === 'stopped'
                      ? EF_CANCEL
                      : EF_NONE,
                http: how === 'fail' ? 503 : how === 'ok' ? 200 : 0,
                heartbeatAge: 0,
                leaseHeld: true,
              }),
            )
            // a crashed runner says nothing: the runtime's own check of the
            // lease, once a heartbeat, finds it lapsed
            const check = () => {
              if (
                bee.endAt !== undefined &&
                bee.fate === 'crash' &&
                expired(issue)
              ) {
                const l = leases.get(issue) as Lease
                resolve({
                  completion: false,
                  errorFrame: EF_NONE,
                  http: 0,
                  heartbeatAge: sec(now() - renewedAt(l)),
                  leaseHeld: false,
                })
                return
              }
              if (bee.endAt === undefined || bee.fate === 'crash')
                clock.after(HEARTBEAT_MS, check)
            }
            clock.after(HEARTBEAT_MS, check)
          })
          return { claim: true, work: { ended, cancel: () => stop(bee) } }
        },
      })
    })
    occupied = () => dispatchers.reduce((n, d) => n + d.lanesInUse(), 0)
    gaveUp = () => givenUp.size
    standDowns = () => dispatchers.reduce((n, d) => n + d.stats.standDowns, 0)
    const readyAll = (issues: number[]) => {
      for (const issue of issues) for (const d of dispatchers) d.ready(issue)
    }
    if (runtime === 'actors-round') {
      // the round still reads and still runs on its clock; at its dispatch
      // step it hands the issues over and dispatches nothing itself
      let gate: ReturnType<typeof createRoundGate>
      const round = async () => {
        rounds++
        await sleep(READ_MS)
        const seen = open()
        await sleep(w.prepareMs - READ_MS)
        readyAll(seen)
        await sleep(REPORT_MS)
      }
      gate = createRoundGate(round)
      // as in production, a bee's end also asks for a round
      wakeRound = () => gate.request('a bee ended')
      const tick = () => {
        gate.request('periodic tick')
        clock.after(TICK_MS, tick)
      }
      tick()
    } else {
      onArrival = (issue) => readyAll([issue])
    }
  }

  for (const a of w.arrivals)
    clock.after(a.at, () => {
      arrived.set(a.issue, a.at)
      onArrival(a.issue)
    })
  const sample = () => {
    occupiedSum += occupied()
    samples++
    clock.after(SAMPLE_MS, sample)
  }
  sample()

  await clock.runUntil(w.horizonMs)

  const firstStart = new Map<number, number>()
  for (const s of starts)
    if (!firstStart.has(s.issue)) firstStart.set(s.issue, s.at)
  const wait = [...firstStart].map(([n, at]) => at - (arrived.get(n) as number))
  const nextStart = (issue: number, after: number) =>
    starts.find((s) => s.issue === issue && s.at >= after)?.at ??
    (completed.has(issue) ? undefined : w.horizonMs)
  const recover = (xs: Array<{ issue: number; at: number }>) =>
    xs
      .map((x) => {
        const n = nextStart(x.issue, x.at)
        return n === undefined ? undefined : n - x.at
      })
      .filter((v): v is number => v !== undefined)
  const progress = bees.reduce(
    (ms, b) =>
      ms +
      Math.max(
        0,
        Math.min(b.stallAt, b.endAt ?? w.horizonMs, w.horizonMs) - b.start,
      ),
    0,
  )
  const crashRec = recover(crashes)
  return {
    scenario: w.name,
    runtime,
    lanes: w.lanes,
    arrived: arrived.size,
    done: completed.size,
    perHour: Math.round((completed.size / (w.horizonMs / HOUR)) * 10) / 10,
    neverStarted: arrived.size - firstStart.size,
    duplicates,
    waitP50: sec(pct(wait, 50)),
    waitP95: sec(pct(wait, 95)),
    readyToStartP95: sec(
      pct(
        starts.map((s) => s.at - s.readyAt),
        95,
      ),
    ),
    crashRecoveryP50: sec(pct(crashRec, 50)),
    crashRecoveryMax: sec(Math.max(0, ...crashRec)),
    hangRecoveryP50: sec(pct(recover(hangs), 50)),
    failRetryP50: sec(pct(recover(fails), 50)),
    effective: Math.round((progress / w.horizonMs) * 10) / 10,
    occupied: Math.round((occupiedSum / Math.max(1, samples)) * 10) / 10,
    nominal: w.lanes,
    stops,
    gaveUp: gaveUp(),
    standDowns: standDowns(),
    rounds,
  }
}
