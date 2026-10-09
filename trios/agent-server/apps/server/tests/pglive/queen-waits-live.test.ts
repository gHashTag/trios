/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * LONG WAITS AS ROWS (gHashTag/t27 specs/queen/waits.t27), against a real
 * PostgreSQL:
 *   - a wait outlives its process: a new runtime on the same database wakes it;
 *   - a write at a stale epoch is refused, and wakes nobody;
 *   - two schedulers on one table never wake the same row twice;
 *   - the release job parks on a row, the round leaves it alone, and the
 *     scheduler's check of the run wakes it;
 *   - a cancelled job cancels its row; a NOTIFY brings a pass forward.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import type { Pool } from 'pg'
import {
  createActorSystem,
  realClock,
} from '../../src/api/services/queen-actors'
import {
  advanceJobs,
  cancelJob,
  getJob,
  startJob,
} from '../../src/api/services/queen-jobs'
import { J_DONE, J_RUNNING } from '../../src/api/services/queen-jobs-rules'
import {
  CLAIM_TTL_SECONDS,
  claimDue,
  createWait,
  EV_RESOLVE,
  endClaimed,
  getWait,
  ghRunKey,
  githubRunResolver,
  jobOwner,
  type Resolver,
  rearmClaimed,
  resolveWaitByKey,
  startWaitsActors,
  W_CANCELLED,
  W_RESOLVED,
  W_WAITING,
  type WaitRow,
  waitOf,
} from '../../src/api/services/queen-waits'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool } from '../../src/lib/db/queen-pool'
import { VirtualClock } from '../api/queen-virtual-clock'
import {
  idle,
  offlineRequested,
  ReleaseWorld,
  RUN_ID,
  type Scheduler,
  scratchDatabase,
  startScheduler,
  walk,
} from './queen-waits-world'

/** 2026-10-09 07:41Z, when job #3 published its release. */
const T0 = Date.UTC(2026, 9, 9, 7, 41)
const MIN = 60_000
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms))

describe('long waits as rows, against PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const running: Array<{ stop: () => void | Promise<void> }> = []
  const previousUrl = process.env.DATABASE_URL

  const pool = (): Pool => {
    const p = createQueenPool((scratch as { url: string }).url)
    pools.push(p)
    return p
  }

  beforeEach(async () => {
    scratch = await scratchDatabase('queen_waits')
    if (!scratch) return
    process.env.DATABASE_URL = scratch.url
    await runPgMigrations()
  })

  afterEach(async () => {
    for (const r of running.splice(0)) await r.stop()
    for (const p of pools.splice(0)) await p.end().catch(() => undefined)
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
  })

  it('outlives its process: a new runtime on the same database wakes it', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    // a CI run that completes 90 minutes in, and a one-hour timer
    let ciDone = false
    const resolvers: Record<string, Resolver> = {
      'gh-run': async () => ({
        completed: ciDone,
        resolution: { conclusion: ciDone ? 'success' : null },
      }),
    }
    const a = pool()
    const keyed = await createWait(
      a,
      {
        owner: 'job:1',
        key: ghRunKey('gHashTag/t27', RUN_ID),
        wakeInSeconds: 30,
      },
      clock.now(),
    )
    const timer = await createWait(
      a,
      { owner: 'job:2', wakeInSeconds: 3600 },
      clock.now(),
    )
    const woken: Array<{ by: string; id: number }> = []
    let first: Scheduler | null = startScheduler(a, clock, resolvers, (row) =>
      woken.push({ by: 'first', id: row.id }),
    )
    await walk(clock, T0 + 45 * MIN, () => (first ? [first] : []))
    expect(first.w.stats.checks).toBeGreaterThan(0)
    expect(woken).toEqual([])
    // the process ends: its actor stops and its pool closes; nothing is written
    const before = await getWait(a, keyed.id)
    first.stop()
    first = null
    await a.end()
    pools.splice(pools.indexOf(a), 1)

    const b = pool()
    const after = await getWait(b, keyed.id)
    expect(after?.state).toBe(W_WAITING)
    expect(after?.epoch).toBe(before?.epoch)
    expect(after?.checks).toBe(before?.checks)
    const second = startScheduler(b, clock, resolvers, (row) =>
      woken.push({ by: 'second', id: row.id }),
    )
    running.push(second)
    await walk(clock, T0 + 90 * MIN, () => [second])
    ciDone = true
    await walk(clock, T0 + 100 * MIN, () => [second])
    expect(woken.map((x) => x.id).sort()).toEqual([keyed.id, timer.id].sort())
    expect(woken.every((x) => x.by === 'second')).toBe(true)
    expect((await getWait(b, keyed.id))?.state).toBe(W_RESOLVED)
    expect((await getWait(b, timer.id))?.state).toBe(W_RESOLVED)
  }, 120_000)

  it('refuses a write at a stale epoch, and wakes nobody with it', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const t = T0
    const row = await createWait(p, { owner: 'job:7', wakeInSeconds: 0 }, t)
    const [mine] = await claimDue(p, t)
    expect(mine.id).toBe(row.id)
    expect(mine.epoch).toBe(row.epoch + 1n)
    // inside the claim nobody else sees the row
    expect(await claimDue(p, t + 30_000)).toEqual([])
    // this claimer stalls past its claim; another takes the row
    const [theirs] = await claimDue(p, t + (CLAIM_TTL_SECONDS + 1) * 1000)
    expect(theirs.id).toBe(row.id)
    expect(theirs.epoch).toBe(mine.epoch + 1n)
    const late = await endClaimed(p, mine, EV_RESOLVE, null, t + 62_000)
    expect(late.landed).toBe(false)
    expect(await rearmClaimed(p, mine, true, t + 62_000)).toBe(false)
    const still = await getWait(p, row.id)
    expect(still?.state).toBe(W_WAITING)
    expect(still?.checks).toBe(0)
    const won = await endClaimed(p, theirs, EV_RESOLVE, { ok: 1 }, t + 63_000)
    expect(won).toEqual({ landed: true, to: W_RESOLVED, resolution: { ok: 1 } })
    // and once ended, no epoch writes again
    expect(
      (await endClaimed(p, theirs, EV_RESOLVE, null, t + 64_000)).landed,
    ).toBe(false)
  }, 120_000)

  it('a scheduler that slept past its claim wakes nobody; the next one does', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    const a = pool()
    const b = pool()
    const row = await createWait(
      a,
      { owner: 'job:8', key: 'gh-run:a/b:1', wakeInSeconds: 30 },
      clock.now(),
    )
    let release: () => void = () => {}
    const stalled = new Promise<void>((r) => {
      release = r
    })
    const woken: string[] = []
    // the first scheduler's check hangs (a GitHub read that never answers)
    const first = startScheduler(
      a,
      clock,
      {
        'gh-run': async () => {
          await stalled
          return { completed: true, resolution: { by: 'first' } }
        },
      },
      () => woken.push('first'),
    )
    running.push(first)
    await walk(clock, T0 + 29_000, () => [first])
    // the check comes due at 30 s and never answers: no waiting for idle now
    await clock.runUntil(T0 + 31_000)
    for (let i = 0; i < 500 && first.w.stats.checks === 0; i++) await sleep(2)
    expect(first.w.stats.checks).toBe(1)
    // the second takes the row once the first one's claim has run out
    const second = startScheduler(
      b,
      clock,
      {
        'gh-run': async () => ({
          completed: true,
          resolution: { by: 'second' },
        }),
      },
      () => woken.push('second'),
    )
    running.push(second)
    await walk(clock, T0 + 31_000 + (CLAIM_TTL_SECONDS + 16) * 1000, () => [
      second,
    ])
    expect(woken).toEqual(['second'])
    release()
    for (let i = 0; i < 500 && first.w.stats.refused === 0; i++) await sleep(2)
    expect(first.w.stats.refused).toBe(1)
    expect(woken).toEqual(['second'])
    const ended = await getWait(a, row.id)
    expect(ended?.state).toBe(W_RESOLVED)
    expect(ended?.resolution).toEqual({ by: 'second' })
  }, 120_000)

  it('two schedulers on one table never wake the same row twice', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const setup = pool()
    const now = Date.now()
    const ids: number[] = []
    for (let i = 0; i < 200; i++)
      ids.push(
        (await createWait(setup, { owner: `job:${i}`, wakeInSeconds: 0 }, now))
          .id,
      )
    for (let i = 0; i < 200; i++) {
      const w = await createWait(
        setup,
        { owner: `job:k${i}`, key: `approval:${i}` },
        now,
      )
      ids.push(w.id)
      await resolveWaitByKey(setup, `approval:${i}`, { yes: i }, now)
    }
    const wakes = new Map<number, number>()
    const count = (row: WaitRow) =>
      wakes.set(row.id, (wakes.get(row.id) ?? 0) + 1)
    const one = startScheduler(pool(), realClock, {}, count)
    const two = startScheduler(pool(), realClock, {}, count)
    running.push(one, two)
    const end = Date.now() + 30_000
    while (Date.now() < end) {
      const left = await setup.query(
        `SELECT count(*)::int AS n FROM queen_wait WHERE state = ${W_WAITING}`,
      )
      if (left.rows[0].n === 0) break
      await sleep(20)
    }
    await idle([one, two])
    expect(wakes.size).toBe(400)
    expect([...wakes.values()].every((n) => n === 1)).toBe(true)
    expect(one.w.stats.woken + two.w.stats.woken).toBe(400)
    expect(one.w.stats.claimed + two.w.stats.claimed).toBe(400)
  }, 120_000)

  it('parks the release job on a row; the round leaves it, the check wakes it', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    const world = new ReleaseWorld()
    const started = await startJob(
      p,
      'release-t27c',
      { version: '0.6.0' },
      false,
      'test',
    )
    if (!started.ok) throw new Error(started.error)
    const id = started.job.id
    await advanceJobs(p, world, clock.now, true)
    let job = await getJob(p, id)
    expect(job?.step).toBe(3)
    expect(String(job?.note)).toContain('parked on wait #')
    const row = await waitOf(p, jobOwner(id), 3)
    expect(row?.key).toBe(ghRunKey('gHashTag/t27', RUN_ID))
    expect(row?.state).toBe(W_WAITING)
    const logAtPark = job?.log.length ?? 0
    const listAtPark = world.listReads
    let wakes = 0
    const s = startScheduler(
      p,
      clock,
      { 'gh-run': githubRunResolver(world.get.bind(world)) },
      () => {
        wakes += 1
      },
    )
    running.push(s)
    // rounds every 30 s; the run waits 90 minutes for a runner
    await walk(
      clock,
      T0 + 90 * MIN,
      () => [s],
      1000,
      async (now) => {
        if ((now - T0) % 30_000 === 0)
          await advanceJobs(p, world, clock.now, true)
      },
    )
    job = await getJob(p, id)
    expect(job?.step).toBe(3)
    expect(job?.log.length).toBe(logAtPark)
    expect(world.listReads).toBe(listAtPark)
    world.run = { status: 'completed', conclusion: 'success' }
    world.crates.add('t27c@0.6.0')
    await walk(
      clock,
      T0 + 96 * MIN,
      () => [s],
      1000,
      async (now) => {
        if ((now - T0) % 30_000 === 0)
          await advanceJobs(p, world, clock.now, true)
      },
    )
    job = await getJob(p, id)
    expect(wakes).toBe(1)
    expect(job?.state).toBe(J_DONE)
    expect(JSON.stringify(job?.log)).toContain('the release pipeline succeeded')
    // one read of the run by id per check: 24 in the 90 minutes, and the one that saw it end
    expect(world.runReads).toBeGreaterThanOrEqual(24)
    expect(world.runReads).toBeLessThanOrEqual(26)
  }, 120_000)

  it('a cancelled job cancels its row, and nobody is woken by it', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const world = new ReleaseWorld()
    const started = await startJob(
      p,
      'release-t27c',
      { version: '0.6.0' },
      false,
      'test',
    )
    if (!started.ok) throw new Error(started.error)
    await advanceJobs(p, world, Date.now, true)
    const parked = await waitOf(p, jobOwner(started.job.id), 3)
    expect(parked?.state).toBe(W_WAITING)
    expect(await cancelJob(p, started.job.id, 'test', true)).toBe('cancelled')
    const after = await getWait(p, parked?.id as number)
    expect(after?.state).toBe(W_CANCELLED)
    expect(after?.epoch).toBe((parked?.epoch as bigint) + 1n)
    const woken: number[] = []
    const s = startScheduler(p, realClock, {}, (row) => woken.push(row.id))
    running.push(s)
    await sleep(200)
    await idle([s])
    expect(woken).toEqual([])
    expect((await getJob(p, started.job.id))?.state).not.toBe(J_RUNNING)
  }, 120_000)

  it('a NOTIFY from another process brings the pass forward', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const other = pool()
    const woken: number[] = []
    const stop = startWaitsActors(p, {
      resolvers: {},
      onWake: (row) => woken.push(row.id),
      sys: createActorSystem(realClock),
    })
    running.push({ stop })
    const row = await createWait(
      other,
      { owner: 'person:owner', key: 'approval:release' },
      Date.now(),
    )
    // let the first pass and the LISTEN settle, then resolve from elsewhere
    await sleep(500)
    const at = Date.now()
    expect(
      await resolveWaitByKey(other, 'approval:release', { yes: true }, at),
    ).toEqual([row.id])
    for (let i = 0; i < 300 && woken.length === 0; i++) await sleep(10)
    expect(woken).toEqual([row.id])
    // well inside the 15 s poll: the notification brought the pass forward
    expect(Date.now() - at).toBeLessThan(3000)
    // a second resolution changes nothing (resolution_lands)
    expect(
      await resolveWaitByKey(
        other,
        'approval:release',
        { yes: false },
        Date.now(),
      ),
    ).toEqual([])
    expect((await getWait(other, row.id))?.resolution).toEqual({ yes: true })
  }, 120_000)
})
