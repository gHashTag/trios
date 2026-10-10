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
 * its beat (nodeBeat) runs on the test's thread: the same code, fed the
 * message createPgLink posts, and called by the test where the thread's
 * interval would fire, so each beat is one step the test takes. What it
 * shares is the same SharedArrayBuffer of three slots.
 *
 * WHY THE TEST HOLDS THE CLOCK. The beat reads `now` for the stall and for
 * the time a renewal was sent; the test hands it `at` and moves it one
 * heartbeat per beat. The interval used to fire on jest fake timers, and on
 * Linux CI (Bun 1.3.6) the send time read under them came out 56 s before
 * the start the test read on the real clock (trios#1730). One clock for both
 * sides of a comparison holds on every platform.
 */

import { afterAll, beforeAll, describe, expect, it } from 'bun:test'
import type { Pool } from 'pg'
import {
  BEAT_REFUSED,
  BEAT_RENEWED,
  BEAT_TICK,
  monoNow,
} from '../../src/api/services/queen-actors-pg'
import type { BeatStart } from '../../src/api/services/queen-actors-pg-beat'
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
  let nodeBeat: (
    start: BeatStart,
    post: (message: unknown) => void,
    now: () => number,
  ) => () => Promise<void> = () => async () => {}
  let posted: unknown[] = []
  const saved = { onmessage: port.onmessage, postMessage: port.postMessage }
  // the test's clock: the beat reads it, and each beat moves it one heartbeat
  let at = Math.floor(monoNow())
  const now = () => at
  const HEARTBEAT_MS = NODE_HEARTBEAT_SECONDS * 1000

  beforeAll(async () => {
    scratch = await scratchDatabase('queen_pg_beat')
    if (!scratch) return
    pool = createQueenPool(scratch.url)
    await pool.query(QUEEN_ACTORS_SQL)
    await pool.query(
      `INSERT INTO queen_actor_node (node, host, heartbeat_at, incarnation)
       VALUES (1, 'h', clock_timestamp(), 1), (2, 'h', clock_timestamp(), 2)`,
    )
    // importing it sets this thread's onmessage, as it does in its own thread
    nodeBeat = (await import('../../src/api/services/queen-actors-pg-beat'))
      .nodeBeat
    port.onmessage = saved.onmessage
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

  /** Start one node's beat, the loop's last turn `tickAgoMs` before now. */
  const beatFor = (
    url: string,
    node: number,
    inc: number,
    tickAgoMs: number,
  ) => {
    const slots = new BigInt64Array(new SharedArrayBuffer(8 * 3))
    posted = []
    Atomics.store(slots, BEAT_TICK, BigInt(at - tickAgoMs))
    const beat = nodeBeat(
      { url, node, inc, host: 'beat-test', shared: slots.buffer },
      (message) => void posted.push(message),
      now,
    )
    return { slots, beat }
  }
  /** One heartbeat passes, and the interval fires. */
  const fire = (beat: () => Promise<void>) => {
    at += HEARTBEAT_MS
    void beat()
  }
  /** Start one node's beat and fire its interval once. */
  const beatOnce = (
    url: string,
    node: number,
    inc: number,
    tickAgoMs: number,
  ) => {
    const { slots, beat } = beatFor(url, node, inc, tickAgoMs)
    fire(beat)
    return slots
  }

  it('renews a lease its incarnation holds: the renewed slot moves to when the renewal was sent', async () => {
    if (!scratch) return expect(offlineRequested()).toBe(true)
    const before = await (pool as Pool).query(
      'SELECT heartbeat_at FROM queen_actor_node WHERE node = 1',
    )
    const start = at
    const slots = beatOnce(scratch.url, 1, 1, 0)
    await until(() => Atomics.load(slots, BEAT_RENEWED) !== 0n, 10_000)
    // sent when the interval fired, one heartbeat after the start
    expect(Number(Atomics.load(slots, BEAT_RENEWED))).toBe(start + HEARTBEAT_MS)
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
    const { slots, beat } = beatFor(scratch.url, 1, 1, 0)
    fire(beat)
    const firstSent = at
    // a second beat, one heartbeat later on the test's clock, finds it busy
    Atomics.store(slots, BEAT_TICK, BigInt(at))
    fire(beat)
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
      firstSent + HEARTBEAT_MS - 1000,
    )
    expect(posted).toEqual([])
  }, 30_000)
})
