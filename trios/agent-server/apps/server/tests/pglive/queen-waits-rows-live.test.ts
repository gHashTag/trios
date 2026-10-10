/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * LONG WAITS AS ROWS, the row operations the scheduler tests did not reach
 * (gHashTag/t27 specs/queen/waits.t27, gHashTag/trios#1731), against a real
 * PostgreSQL: a wait asked twice, the reads of the board, a transaction that
 * fails and must leave its connection clean, an owner whose wake throws, and
 * the route a person or a webhook uses.
 */

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'
import type { Pool } from 'pg'
import { createQueenWaitsRoute } from '../../src/api/routes/queen-waits'
import {
  cancelWaitsOf,
  claimDue,
  createWait,
  ensureWaitTables,
  getWait,
  isTerminal,
  listWaits,
  resolveWaitByKey,
  W_CANCELLED,
  W_EXPIRED,
  W_WAITING,
  type WaitRow,
  waitOf,
} from '../../src/api/services/queen-waits'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool } from '../../src/lib/db/queen-pool'
import { logger } from '../../src/lib/logger'
import { VirtualClock } from '../api/queen-virtual-clock'
import {
  offlineRequested,
  scratchDatabase,
  startScheduler,
  walk,
} from './queen-waits-world'

/** 2026-10-10 06:00Z. */
const T0 = Date.UTC(2026, 9, 10, 6, 0)

describe('wait rows, against PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const previousUrl = process.env.DATABASE_URL

  // one connection: a transaction left open on it fails the next statement
  const pool = (max = 1): Pool => {
    const p = createQueenPool((scratch as { url: string }).url, { max })
    pools.push(p)
    return p
  }

  beforeEach(async () => {
    scratch = await scratchDatabase('queen_waits_rows')
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

  it('a wait its owner asks for again on the same key is the row it already has', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const first = await createWait(
      p,
      { owner: 'job:5', key: 'gh-run:a/b:1', wakeInSeconds: 60 },
      T0,
    )
    // a restart asks again, later and with another timer: the same row
    const again = await createWait(
      p,
      { owner: 'job:5', key: 'gh-run:a/b:1', wakeInSeconds: 600 },
      T0 + 30_000,
    )
    expect(again.id).toBe(first.id)
    expect(again.wakeAt).toBe(first.wakeAt)
    expect((await listWaits(p)).length).toBe(1)
  }, 60_000)

  it("an owner's latest wait, and its latest for one step", async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const a = await createWait(
      p,
      { owner: 'job:9', wakeInSeconds: 60, detail: { step: 1 } },
      T0,
    )
    const b = await createWait(
      p,
      { owner: 'job:9', wakeInSeconds: 60, detail: { step: 2 } },
      T0,
    )
    expect((await waitOf(p, 'job:9'))?.id).toBe(b.id)
    expect((await waitOf(p, 'job:9', 1))?.id).toBe(a.id)
    expect(await waitOf(p, 'job:404')).toBeNull()
  }, 60_000)

  it('the board lists what waits first, newest first, then what ended', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const ended = await createWait(p, { owner: 'job:1', wakeInSeconds: 60 }, T0)
    const older = await createWait(p, { owner: 'job:2', wakeInSeconds: 60 }, T0)
    const newer = await createWait(p, { owner: 'job:3', wakeInSeconds: 60 }, T0)
    expect(await cancelWaitsOf(p, 'job:1', T0 + 1000)).toEqual([ended.id])
    const board = await listWaits(p)
    expect(board.map((w) => [w.id, w.state])).toEqual([
      [newer.id, W_WAITING],
      [older.id, W_WAITING],
      [ended.id, W_CANCELLED],
    ])
    expect((await listWaits(p, 1)).map((w) => w.id)).toEqual([newer.id])
  }, 60_000)

  it('a resolution that cannot be written rolls back: the row still waits, unresolved, on a clean connection', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const row = await createWait(p, { owner: 'job:4', key: 'k-4' }, T0)
    // JSON.stringify refuses a BigInt inside the transaction, after the lock
    await expect(
      resolveWaitByKey(p, 'k-4', { n: 1n }, T0 + 1000),
    ).rejects.toThrow()
    // the row's lock went with the transaction: another connection takes it
    // without waiting (a transaction left open would still hold it)
    const other = await pool().connect()
    try {
      await other.query('BEGIN')
      const locked = await other.query(
        'SELECT state, resolution FROM queen_wait WHERE id = $1 FOR UPDATE NOWAIT',
        [row.id],
      )
      expect(locked.rows).toEqual([{ state: W_WAITING, resolution: null }])
      await other.query('ROLLBACK')
    } finally {
      other.release()
    }
    // and a good resolution lands at once
    expect(await resolveWaitByKey(p, 'k-4', { ok: true }, T0 + 2000)).toEqual([
      row.id,
    ])
    expect(((await getWait(p, row.id)) as WaitRow).resolution).toEqual({
      ok: true,
    })
  }, 60_000)

  it('a claim the database refuses rolls back: the next claim takes the row at the next epoch', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool()
    const row = await createWait(p, { owner: 'job:6', wakeInSeconds: 0 }, T0)
    // a negative LIMIT is an error inside the claim's transaction
    await expect(claimDue(p, T0, -1)).rejects.toThrow()
    const [mine] = await claimDue(p, T0)
    expect(mine.id).toBe(row.id)
    expect(mine.epoch).toBe(row.epoch + 1n)
  }, 60_000)

  it('an owner whose wake throws is logged by name, and its row still ended', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const warn = spyOn(logger, 'warn').mockImplementation(() => {})
    try {
      const p = pool(4)
      const clock = new VirtualClock()
      await clock.runUntil(T0)
      const row = await createWait(
        p,
        { owner: 'job:8', wakeInSeconds: 5 },
        clock.now(),
      )
      const s = startScheduler(p, clock, {}, () => {
        throw new Error('the owner is gone')
      })
      await walk(clock, T0 + 30_000, () => [s])
      s.stop()
      expect(s.w.stats.woken).toBe(1)
      expect(s.w.stats.failedPasses).toBe(0)
      const ended = (await getWait(p, row.id)) as WaitRow
      expect(isTerminal(ended.state)).toBe(true)
      const lines = warn.mock.calls.filter(
        (c) => c[0] === 'Queen wait woke an owner that threw',
      )
      expect(lines.length).toBe(1)
      expect(lines[0][1]).toMatchObject({
        id: row.id,
        owner: 'job:8',
        error: 'the owner is gone',
      })
    } finally {
      warn.mockRestore()
    }
  }, 120_000)

  it('a keyed row no resolver knows is checked, re-armed, and ends at its expiry; its owner hears it expired', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool(4)
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    const row = await createWait(
      p,
      {
        owner: 'job:21',
        key: 'mystery:1',
        wakeInSeconds: 0,
        expirySeconds: 120,
      },
      clock.now(),
    )
    const woken: WaitRow[] = []
    const s = startScheduler(p, clock, {}, (r) => void woken.push(r))
    await walk(clock, T0 + 30_000, () => [s])
    // checked at once, and the check could not complete: re-armed
    const mid = (await getWait(p, row.id)) as WaitRow
    expect(mid.state).toBe(W_WAITING)
    expect(mid.checks).toBeGreaterThanOrEqual(1)
    expect(woken).toEqual([])
    await walk(clock, T0 + 130_000, () => [s])
    s.stop()
    const ended = (await getWait(p, row.id)) as WaitRow
    expect(ended.state).toBe(W_EXPIRED)
    expect(woken.map((r) => [r.id, r.state])).toEqual([[row.id, W_EXPIRED]])
    expect(s.w.stats.checks).toBe(ended.checks)
  }, 120_000)

  it('a resolver that fails is a check that could not read: the row is re-armed, not ended', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool(4)
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    const row = await createWait(
      p,
      { owner: 'job:22', key: 'gh-run:a/b:22', wakeInSeconds: 0 },
      clock.now(),
    )
    let asked = 0
    const s = startScheduler(
      p,
      clock,
      {
        'gh-run': async () => {
          asked++
          throw new Error('GitHub is down')
        },
      },
      () => {},
    )
    await walk(clock, T0 + 30_000, () => [s])
    s.stop()
    expect(asked).toBeGreaterThanOrEqual(1)
    const still = (await getWait(p, row.id)) as WaitRow
    expect(still.state).toBe(W_WAITING)
    expect(still.checks).toBe(asked)
    expect(s.w.stats.failedPasses).toBe(0)
  }, 120_000)

  it('a row a pass finds due with nothing due on it is put back to wait for its own time, and wakes nobody', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = pool(4)
    const clock = new VirtualClock()
    await clock.runUntil(T0)
    const row = await createWait(
      p,
      { owner: 'job:23', wakeInSeconds: 600 },
      clock.now(),
    )
    // another process with its clock ahead left the row due now: what a
    // stale claim or a skewed writer leaves behind
    await p.query('UPDATE queen_wait SET due_at = $2 WHERE id = $1', [
      row.id,
      new Date(T0),
    ])
    let woken = 0
    const s = startScheduler(p, clock, {}, () => {
      woken++
    })
    await walk(clock, T0 + 5000, () => [s])
    s.stop()
    expect(s.w.stats.claimed).toBe(1)
    expect(s.w.stats.refused).toBe(0)
    expect(woken).toBe(0)
    const back = (await getWait(p, row.id)) as WaitRow
    expect(back.state).toBe(W_WAITING)
    expect(back.checks).toBe(0)
    expect(back.dueAt).toBe(row.wakeAt)
  }, 120_000)
})

describe('the waits route, against PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const previousUrl = process.env.DATABASE_URL
  const previousSsot = process.env.RAILWAY_SSOT_URL

  beforeEach(async () => {
    scratch = await scratchDatabase('queen_waits_route')
  })

  afterEach(async () => {
    for (const p of pools.splice(0)) await p.end().catch(() => undefined)
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
    if (previousSsot === undefined) delete process.env.RAILWAY_SSOT_URL
    else process.env.RAILWAY_SSOT_URL = previousSsot
  })

  const post = (body: unknown) => ({
    method: 'POST',
    body: JSON.stringify(body),
  })

  it('makes a wait for a person, lists it, resolves its key once, and refuses what is malformed', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = createQueenPool(scratch.url)
    pools.push(p)
    await ensureWaitTables(p)
    let now = T0
    const app = createQueenWaitsRoute({
      pool: () => p,
      enabled: () => true,
      now: () => now,
    })
    const made = await app.request(
      '/',
      post({ name: 'ops\u0007 desk', key: 'deploy-ok', expirySeconds: 3600 }),
    )
    expect(made.status).toBe(201)
    const wait = ((await made.json()) as { wait: Record<string, unknown> }).wait
    // the control character is stripped from the owner's name
    expect(wait.owner).toBe('person:ops  desk')
    expect(wait.key).toBe('deploy-ok')
    expect(wait.expiresAt).toBe(new Date(T0 + 3600_000).toISOString())

    const listed = (await (await app.request('/')).json()) as {
      waits: Array<{ id: number; epoch: string }>
    }
    expect(listed.waits.map((w) => w.id)).toEqual([wait.id as number])
    expect(typeof listed.waits[0].epoch).toBe('string')

    now += 60_000
    const resolved = await app.request(
      '/resolve',
      post({ key: 'deploy-ok', resolution: { by: 'ops' } }),
    )
    expect(resolved.status).toBe(200)
    expect(await resolved.json()).toEqual({
      key: 'deploy-ok',
      resolved: [wait.id],
    })
    // the first answer stays: a second resolve finds no row to change
    const twice = await app.request('/resolve', post({ key: 'deploy-ok' }))
    expect(twice.status).toBe(404)

    // neither a key nor a timer: the card's validWait refuses it
    const bare = await app.request('/', post({ name: 'ops' }))
    expect(bare.status).toBe(400)
    expect(((await bare.json()) as { error: string }).error).toBe(
      'a wait names a key, a wake time, or both',
    )
    const noKey = await app.request('/resolve', post({}))
    expect(noKey.status).toBe(400)
    // a body that is not JSON is read as an empty one
    const garbled = { method: 'POST', body: 'not json {' }
    const garbledMake = await app.request('/', garbled)
    expect(garbledMake.status).toBe(400)
    expect(((await garbledMake.json()) as { error: string }).error).toBe(
      'a wait names a key, a wake time, or both',
    )
    const garbledResolve = await app.request('/resolve', garbled)
    expect(garbledResolve.status).toBe(400)
    expect(await garbledResolve.json()).toEqual({ error: 'key is required' })

    // a timer: its wake time is shown as a time, its epoch as text
    const timer = await app.request(
      '/',
      post({ name: 'ops', wakeInSeconds: 90.7, detail: { why: 'drain' } }),
    )
    expect(timer.status).toBe(201)
    const t = ((await timer.json()) as { wait: Record<string, unknown> }).wait
    expect(t.wakeAt).toBe(new Date(now + 90_000).toISOString())
    expect(t.key).toBeNull()
    expect(t.detail).toEqual({ why: 'drain' })
  }, 60_000)

  it('is off unless TRIOS_QUEEN_WAITS=rows when nobody says otherwise', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const p = createQueenPool(scratch.url)
    pools.push(p)
    const saved = process.env.TRIOS_QUEEN_WAITS
    try {
      delete process.env.TRIOS_QUEEN_WAITS
      const app = createQueenWaitsRoute({ pool: () => p })
      const off = await app.request('/', post({ key: 'k' }))
      expect(off.status).toBe(503)
      expect(((await off.json()) as { error: string }).error).toContain(
        'TRIOS_QUEEN_WAITS=rows',
      )
      process.env.TRIOS_QUEEN_WAITS = 'rows'
      await ensureWaitTables(p)
      expect((await app.request('/', post({ key: 'k' }))).status).toBe(201)
    } finally {
      if (saved === undefined) delete process.env.TRIOS_QUEEN_WAITS
      else process.env.TRIOS_QUEEN_WAITS = saved
    }
  }, 60_000)

  it('without a database it answers 503 on every path', async () => {
    const app = createQueenWaitsRoute({ pool: () => null, enabled: () => true })
    expect((await app.request('/')).status).toBe(503)
    expect((await app.request('/', post({ key: 'k' }))).status).toBe(503)
    expect((await app.request('/resolve', post({ key: 'k' }))).status).toBe(503)
  })

  it('with no pool given it uses the database DATABASE_URL names, and none when it names none', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    delete process.env.DATABASE_URL
    delete process.env.RAILWAY_SSOT_URL
    const app = createQueenWaitsRoute({ enabled: () => true })
    expect((await app.request('/')).status).toBe(503)
    process.env.DATABASE_URL = scratch.url
    const res = await app.request('/')
    expect(res.status).toBe(200)
    expect(await res.json()).toEqual({ waits: [] })
    // the route's own pool is the process's: its idle connection closes
    // after pg's idle timeout (10 s), before the scratch database is dropped
    await new Promise((r) => setTimeout(r, 11_000))
  }, 60_000)
})
