/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BENCHMARK OF LONG WAITS AS ROWS (gHashTag/t27 specs/queen/waits.t27,
 * trios#1712 item 4), old against new on the same scripted job, against a real
 * PostgreSQL, on a virtual clock.
 *
 * The job: release-t27c, as job #3 ran it on 2026-10-09. The release is
 * published at T0; its release.yml run waits 90 minutes for a runner, then
 * succeeds and the crate is on the registry. A round runs every 30 s (the live
 * Queen's measured pace), and with waits on, a wake asks for a round at once,
 * as gate.request does in queen-tick.ts.
 *   old: TRIOS_QUEEN_WAITS off - every round asks the WAIT step again.
 *   new: TRIOS_QUEEN_WAITS=rows - the step parks on a queen_wait row, the
 *        round skips the job, the scheduler checks the run by its id.
 * Measured during the 90-minute wait: GitHub reads, entries appended to the
 * job's log, and the wall-clock milliseconds this process spent on the wait
 * (rounds' job advance + the scheduler's passes). Then the same job with a
 * forced restart 45 minutes in, and the cost of one step in ms.
 * GitHub here answers at once: a real read adds its own latency to every read
 * counted, so the read counts, not the ms, carry that part.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import type { Pool } from 'pg'
import {
  advanceJobs,
  getJob,
  startJob,
} from '../../src/api/services/queen-jobs'
import { J_DONE } from '../../src/api/services/queen-jobs-rules'
import {
  claimDue,
  createWait,
  EV_RESOLVE,
  endClaimed,
  githubRunResolver,
} from '../../src/api/services/queen-waits'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool } from '../../src/lib/db/queen-pool'
import { VirtualClock } from '../api/queen-virtual-clock'
import {
  offlineRequested,
  ReleaseWorld,
  type Scheduler,
  scratchDatabase,
  startScheduler,
  walk,
} from './queen-waits-world'

const T0 = Date.UTC(2026, 9, 9, 7, 41)
const MIN = 60_000
// the run ends 90 minutes in, at a moment no round and no check is aligned to
const CI_MS = 90 * MIN + 7_000
const ROUND_MS = 30_000
const HORIZON = 100 * MIN

interface Run {
  design: 'old' | 'new'
  restart: boolean
  reads: number
  listReads: number
  runReads: number
  logEntries: number
  roundMs: number
  schedulerMs: number
  passes: number
  /** Seconds from the run's end to the job leaving the step. */
  detectSeconds: number | null
  done: boolean
}

const pct = (xs: number[], p: number): number => {
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]
}
const f = (n: number, d = 1) => n.toFixed(d)

describe('long waits: rows against rounds, on the same release', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const previousUrl = process.env.DATABASE_URL
  const pool = (): Pool => {
    const p = createQueenPool((scratch as { url: string }).url)
    pools.push(p)
    return p
  }

  beforeEach(async () => {
    scratch = await scratchDatabase('queen_waits_bench')
    if (!scratch) return
    process.env.DATABASE_URL = scratch.url
    await runPgMigrations()
  })

  afterEach(async () => {
    for (const p of pools.splice(0)) await p.end().catch(() => undefined)
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
  })

  async function release(
    design: 'old' | 'new',
    restart: boolean,
  ): Promise<Run> {
    const waits = design === 'new'
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    let p = pool()
    const world = new ReleaseWorld()
    const started = await startJob(
      p,
      'release-t27c',
      { version: '0.6.0' },
      false,
      'bench',
    )
    if (!started.ok) throw new Error(started.error)
    const id = started.job.id
    let roundMs = 0
    let wanted = false
    let sched: Scheduler | null = null
    let schedulerMsBefore = 0
    let passesBefore = 0
    const startHere = () => {
      sched = startScheduler(
        p,
        clock,
        { 'gh-run': githubRunResolver(world.get.bind(world)) },
        () => {
          wanted = true
        },
      )
    }
    const round = async () => {
      const t = performance.now()
      await advanceJobs(p, world, clock.now, waits)
      if (clock.now() < T0 + CI_MS) roundMs += performance.now() - t
    }
    // the first round walks the checks and the publish, and reaches the WAIT
    await round()
    if (waits) startHere()
    const first = await getJob(p, id)
    const at = {
      reads: world.reads,
      listReads: world.listReads,
      runReads: world.runReads,
      log: first?.log.length ?? 0,
    }
    let atCi = { ...at }
    const schedNow = () => ({
      ms:
        schedulerMsBefore + ((sched as Scheduler | null)?.w.stats.passMs ?? 0),
      passes: passesBefore + ((sched as Scheduler | null)?.w.stats.passes ?? 0),
    })
    let schedAtCi = { ms: 0, passes: 0 }
    let ciSeen = false
    let leftAt: number | null = null
    let restarted = false
    const current = () => (sched ? [sched as Scheduler] : [])
    // a forced restart: the process ends, a new one starts on the same database
    const restartNow = async () => {
      restarted = true
      if (sched) {
        const s = sched as Scheduler
        schedulerMsBefore += s.w.stats.passMs
        passesBefore += s.w.stats.passes
        s.stop()
        sched = null
      }
      await p.end()
      pools.splice(pools.indexOf(p), 1)
      p = pool()
      if (waits) startHere()
    }
    // the run ends: snapshot the wait's costs, then the world moves on
    const ciEnds = async () => {
      ciSeen = true
      schedAtCi = schedNow()
      const job = await getJob(p, id)
      atCi = {
        reads: world.reads,
        listReads: world.listReads,
        runReads: world.runReads,
        log: job?.log.length ?? 0,
      }
      world.run = { status: 'completed', conclusion: 'success' }
      world.crates.add('t27c@0.6.0')
    }
    const roundNow = async (now: number) => {
      wanted = false
      await round()
      if (leftAt !== null || !ciSeen) return
      const job = await getJob(p, id)
      if ((job?.step ?? 0) > 3 || job?.state === J_DONE) leftAt = now
    }
    await walk(clock, T0 + HORIZON, current, 1000, async (now) => {
      if (restart && !restarted && now >= T0 + 45 * MIN) await restartNow()
      if (!ciSeen && now >= T0 + CI_MS) await ciEnds()
      if ((now - T0) % ROUND_MS === 0 || wanted) await roundNow(now)
    })
    const s = sched as Scheduler | null
    s?.stop()
    const job = await getJob(p, id)
    return {
      design,
      restart,
      reads: atCi.reads - at.reads,
      listReads: atCi.listReads - at.listReads,
      runReads: atCi.runReads - at.runReads,
      logEntries: atCi.log - at.log,
      roundMs,
      schedulerMs: schedAtCi.ms,
      passes: schedAtCi.passes,
      detectSeconds: leftAt === null ? null : (leftAt - (T0 + CI_MS)) / 1000,
      done: job?.state === J_DONE,
    }
  }

  it('runs the 90-minute release wait both ways, and through a restart', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const runs: Run[] = []
    for (const restart of [false, true])
      for (const design of ['old', 'new'] as const)
        runs.push(await release(design, restart))
    const rows = runs.map(
      (r) =>
        `| ${r.design} | ${r.restart ? 'yes' : 'no'} | ${r.reads} (${r.listReads} list, ${r.runReads} by id) | ${r.logEntries} | ${f(r.roundMs)} | ${f(r.schedulerMs)} (${r.passes} passes) | ${f(r.roundMs + r.schedulerMs)} | ${r.detectSeconds ?? '-'} | ${r.done ? 0 : 1} |`,
    )
    console.log(
      [
        '',
        '| design | restart at 45 min | GitHub reads during the wait | job log entries | round ms | scheduler ms | process ms held | detect s | waits lost |',
        '|---|---|---|---|---|---|---|---|---|',
        ...rows,
      ].join('\n'),
    )
    for (const r of runs) expect(r.done).toBe(true)
    const [oldRun, newRun] = runs
    expect(newRun.reads).toBeLessThan(oldRun.reads)
    expect(newRun.logEntries).toBe(0)
  }, 600_000)

  it('costs a few ms per step', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const N = 300
    const create: number[] = []
    const claim: number[] = []
    const end: number[] = []
    let now = T0
    for (let i = 0; i < N; i++) {
      now += 1000
      let t = performance.now()
      await createWait(p, { owner: `job:${i}`, wakeInSeconds: 0 }, now)
      create.push(performance.now() - t)
      t = performance.now()
      const [row] = await claimDue(p, now)
      claim.push(performance.now() - t)
      t = performance.now()
      const r = await endClaimed(p, row, EV_RESOLVE, null, now)
      end.push(performance.now() - t)
      expect(r.landed).toBe(true)
    }
    // the old step: one round's visit of a job waiting at release-workflow
    const world = new ReleaseWorld()
    const started = await startJob(
      p,
      'release-t27c',
      { version: '0.6.0' },
      false,
      'bench',
    )
    if (!started.ok) throw new Error(started.error)
    await advanceJobs(p, world, () => now, false)
    const oldVisit: number[] = []
    for (let i = 0; i < N; i++) {
      const t = performance.now()
      await advanceJobs(p, world, () => now, false)
      oldVisit.push(performance.now() - t)
    }
    // the new round over the same job, parked: the skip
    await p.query(`DELETE FROM queen_job`)
    const again = await startJob(
      p,
      'release-t27c',
      { version: '0.7.0' },
      false,
      'bench',
    )
    if (!again.ok) throw new Error(again.error)
    world.tags.clear()
    world.manifests = { cargo: '0.7.0', zenodo: '0.7.0' }
    await advanceJobs(p, world, () => now, true)
    const newVisit: number[] = []
    for (let i = 0; i < N; i++) {
      const t = performance.now()
      await advanceJobs(p, world, () => now, true)
      newVisit.push(performance.now() - t)
    }
    const line = (name: string, xs: number[]) =>
      `| ${name} | ${f(pct(xs, 50), 2)} | ${f(pct(xs, 95), 2)} |`
    console.log(
      [
        '',
        `| step (N=${N}, local PostgreSQL 16) | p50 ms | p95 ms |`,
        '|---|---|---|',
        line('new: create a wait (INSERT + NOTIFY)', create),
        line('new: claim one due row (SKIP LOCKED + epoch)', claim),
        line('new: end it at its epoch (lock, card, UPDATE)', end),
        line(
          'new: a whole wait (create + claim + end)',
          create.map((x, i) => x + claim[i] + end[i]),
        ),
        line('old: a round visits the waiting job', oldVisit),
        line('new: a round skips the parked job', newVisit),
      ].join('\n'),
    )
  }, 600_000)
})
