/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BEAT, ONE RENEWAL AT A TIME (gHashTag/t27 specs/queen/netlink.t27
 * section 3, gHashTag/trios#1731), against a real PostgreSQL.
 *
 * WHY NOT IN ITS THREAD. In production queen-actors-pg-beat.ts is the entry
 * of a worker thread, and Bun's coverage does not see worker threads. Here
 * its message handler runs on the test's thread with the port stubbed: the
 * same code, fed the message createPgLink posts, its interval on fake timers
 * so each beat is one step the test takes. What it shares is the same
 * SharedArrayBuffer of three slots.
 */

import { afterAll, beforeAll, describe, expect, it, jest } from 'bun:test'
import type { Pool } from 'pg'
import {
  BEAT_REFUSED,
  BEAT_RENEWED,
  BEAT_TICK,
  monoNow,
} from '../../src/api/services/queen-actors-pg'
import { NODE_HEARTBEAT_SECONDS } from '../../src/api/services/queen-netlink-card.gen'
import { QUEEN_ACTORS_SQL } from '../../src/lib/db/pg-migrate'
import { createQueenPool } from '../../src/lib/db/queen-pool'
import { offlineRequested, scratchDatabase } from './queen-waits-world'

type Port = {
  onmessage: ((event: MessageEvent) => void) | null
  postMessage: (value: unknown) => void
}
const port = globalThis as unknown as Port

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
const until = async (cond: () => boolean, ms: number) => {
  const end = Date.now() + ms
  while (!cond() && Date.now() < end) await sleep(20)
}

describe('the beat thread, its handler run here', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  let pool: Pool | null = null
  let handler: (event: MessageEvent) => void = () => {}
  let posted: unknown[] = []
  const saved = { onmessage: port.onmessage, postMessage: port.postMessage }

  beforeAll(async () => {
    scratch = await scratchDatabase('queen_pg_beat')
    if (!scratch) return
    pool = createQueenPool(scratch.url)
    await pool.query(QUEEN_ACTORS_SQL)
    await pool.query(
      `INSERT INTO queen_actor_node (node, host, heartbeat_at, incarnation)
       VALUES (1, 'h', clock_timestamp(), 1), (2, 'h', clock_timestamp(), 2)`,
    )
    await import('../../src/api/services/queen-actors-pg-beat')
    handler = port.onmessage as (event: MessageEvent) => void
    port.onmessage = saved.onmessage
    port.postMessage = (value) => void posted.push(value)
  })

  afterAll(async () => {
    port.onmessage = saved.onmessage
    port.postMessage = saved.postMessage
    await pool?.end()
    // each beat's own pool closes its idle connection after pg's 10 s, and
    // only then is the database dropped
    if (scratch) await sleep(11_000)
    await scratch?.drop()
  }, 60_000)

  /** Start one beat handler and fire its interval once. */
  const beatOnce = (
    url: string,
    node: number,
    inc: number,
    tickAgoMs: number,
  ) => {
    const slots = new BigInt64Array(new SharedArrayBuffer(8 * 3))
    posted = []
    jest.useFakeTimers()
    try {
      Atomics.store(slots, BEAT_TICK, BigInt(Math.floor(monoNow() - tickAgoMs)))
      handler({
        data: { url, node, inc, host: 'beat-test', shared: slots.buffer },
      } as MessageEvent)
      jest.advanceTimersByTime(NODE_HEARTBEAT_SECONDS * 1000)
    } finally {
      jest.useRealTimers()
    }
    return slots
  }

  it('renews a lease its incarnation holds: the renewed slot moves to when the renewal was sent', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const before = await (pool as Pool).query(
      'SELECT heartbeat_at FROM queen_actor_node WHERE node = 1',
    )
    const start = Math.floor(monoNow())
    const slots = beatOnce(scratch.url, 1, 1, 0)
    await until(() => Atomics.load(slots, BEAT_RENEWED) !== 0n, 10_000)
    expect(Number(Atomics.load(slots, BEAT_RENEWED))).toBeGreaterThanOrEqual(
      start,
    )
    expect(Atomics.load(slots, BEAT_REFUSED)).toBe(0n)
    expect(posted).toEqual([])
    const after = await (pool as Pool).query(
      'SELECT heartbeat_at FROM queen_actor_node WHERE node = 1',
    )
    expect(new Date(after.rows[0].heartbeat_at).getTime()).toBeGreaterThan(
      new Date(before.rows[0].heartbeat_at).getTime(),
    )
  }, 30_000)

  it('renews nothing while the actors loop is stalled, and says how long it stalled', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const slots = beatOnce(scratch.url, 1, 1, 120_000)
    await until(() => posted.length > 0, 5000)
    const m = posted[0] as { stalled: number }
    expect(m.stalled).toBeGreaterThanOrEqual(120)
    expect(Atomics.load(slots, BEAT_RENEWED)).toBe(0n)
  }, 30_000)

  it('a renewal the store refuses (another incarnation holds the row) sets the refused slot and says so', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    // node 2 is held by incarnation 2; this beat renews for 1
    const slots = beatOnce(scratch.url, 2, 1, 0)
    await until(() => posted.length > 0, 10_000)
    expect(posted).toEqual([{ refused: true }])
    expect(Atomics.load(slots, BEAT_REFUSED)).toBe(1n)
    expect(Atomics.load(slots, BEAT_RENEWED)).toBe(0n)
  }, 30_000)

  it('a renewal that never reaches the store is a failure it reports, not a renewal', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    // nothing listens on port 1
    const slots = beatOnce('postgres://nobody@127.0.0.1:1/none', 1, 1, 0)
    await until(() => posted.length > 0, 10_000)
    const m = posted[0] as { failed: string }
    expect(typeof m.failed).toBe('string')
    expect(m.failed.length).toBeGreaterThan(0)
    expect(Atomics.load(slots, BEAT_RENEWED)).toBe(0n)
    expect(Atomics.load(slots, BEAT_REFUSED)).toBe(0n)
  }, 30_000)

  it('a beat while the last one still waits on the store sends nothing more', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    // hold node 1's row, so the first renewal waits on the lock
    const holder = await (pool as Pool).connect()
    await holder.query('BEGIN')
    await holder.query(
      'SELECT 1 FROM queen_actor_node WHERE node = 1 FOR UPDATE',
    )
    const slots = new BigInt64Array(new SharedArrayBuffer(8 * 3))
    posted = []
    let firstSent = 0
    jest.useFakeTimers()
    try {
      Atomics.store(slots, BEAT_TICK, BigInt(Math.floor(monoNow())))
      handler({
        data: {
          url: (scratch as { url: string }).url,
          node: 1,
          inc: 1,
          host: 'beat-test',
          shared: slots.buffer,
        },
      } as MessageEvent)
      jest.advanceTimersByTime(NODE_HEARTBEAT_SECONDS * 1000)
      firstSent = Math.floor(monoNow())
      // a second beat, five seconds later on the fake clock, finds it busy
      Atomics.store(slots, BEAT_TICK, BigInt(Math.floor(monoNow())))
      jest.advanceTimersByTime(NODE_HEARTBEAT_SECONDS * 1000)
    } finally {
      jest.useRealTimers()
    }
    await sleep(300)
    expect(Atomics.load(slots, BEAT_RENEWED)).toBe(0n)
    await holder.query('COMMIT')
    holder.release()
    await until(() => Atomics.load(slots, BEAT_RENEWED) !== 0n, 10_000)
    // the renewal that landed is the first one: the second never left
    expect(Number(Atomics.load(slots, BEAT_RENEWED))).toBeLessThanOrEqual(
      firstSent,
    )
    expect(Number(Atomics.load(slots, BEAT_RENEWED))).toBeLessThan(
      firstSent + NODE_HEARTBEAT_SECONDS * 1000 - 1000,
    )
    expect(posted).toEqual([])
  }, 30_000)
})
