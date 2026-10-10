/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE CLAIM'S HOLDER IN THE STORE (gHashTag/t27 specs/queen/keyed_guard.t27
 * section 1, gHashTag/trios#1729 item 7), against PostgreSQL. The store
 * applies the card's rules inside its statements and t27c has no SQL
 * backend, so the statements mirror claim_lands_for, round_renews and
 * end_releases, and this test holds them to the card at their edges, as the
 * netlink live test does for write_admitted. Each holder below is labelled
 * with what it is (the process itself, one of its actors, someone else's);
 * the card decides, the store must agree.
 *
 * Same harness as queen-control-live.test.ts: a scratch database per test,
 * migrated by the boot migration, and no silent skip unless
 * TRIOS_PG_MIGRATE_GATE=offline says the absence is deliberate.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { randomBytes } from 'node:crypto'
import { userInfo } from 'node:os'
import { Pool } from 'pg'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import {
  claimTaskLease,
  releaseTaskLease,
  renewRunningLeases,
} from '../../src/api/services/queen-control'
import { ensureQueenColumns } from '../../src/api/services/queen-tick'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool, queenSchema } from '../../src/lib/db/queen-pool'

const OFFLINE_KEY = 'TRIOS_PG_MIGRATE_GATE'
const URL_KEY = 'TRIOS_PG_TEST_URL'
const TTL = 180

function offlineRequested(): boolean {
  return (process.env[OFFLINE_KEY] ?? '').toLowerCase() === 'offline'
}

function adminUrl(): string {
  return (
    process.env[URL_KEY] ??
    `postgres://${userInfo().username}@127.0.0.1:5432/postgres`
  )
}

async function scratchDatabase(): Promise<{
  url: string
  drop: () => Promise<void>
} | null> {
  const name = `queen_guard_${randomBytes(6).toString('hex')}`
  const admin = new Pool({ connectionString: adminUrl(), max: 1 })
  try {
    await admin.query(`CREATE DATABASE ${name}`)
  } catch (error) {
    await admin.end().catch(() => undefined)
    if (offlineRequested()) return null
    throw error
  }
  const url = new URL(adminUrl())
  url.pathname = `/${name}`
  const fresh = new Pool({ connectionString: url.toString(), max: 1 })
  try {
    await fresh.query(`CREATE SCHEMA IF NOT EXISTS ${queenSchema()}`)
  } finally {
    await fresh.end().catch(() => undefined)
  }
  return {
    url: url.toString(),
    drop: async () => {
      await admin
        .query(`DROP DATABASE IF EXISTS ${name} WITH (FORCE)`)
        .catch(() => undefined)
      await admin.end().catch(() => undefined)
    },
  }
}

const guard = () => loadCardWasm('queen/keyed_guard.wasm')

// this process, its boot, and someone else; a name that only starts like
// this one's ('...:10' against '...:1') must not pass for it
const P = 'replica-a:1'
const BOOT = `${P}/0a0b0c0d`
interface Holder {
  holder: string
  /** keyed_guard.t27 same_process, from this process's point of view. */
  mine: boolean
  /** The pid in it; 0 for a process's own claim (HOLDER_PROCESS). */
  pid: bigint
  /** Which process it names, for claim_lands_for between two holders. */
  process: string
}
const HOLDERS: Holder[] = [
  { holder: P, mine: true, pid: 0n, process: P },
  {
    holder: `${BOOT}:21474836481`,
    mine: true,
    pid: 21474836481n,
    process: BOOT,
  },
  {
    holder: `${BOOT}:21474836482`,
    mine: true,
    pid: 21474836482n,
    process: BOOT,
  },
  {
    holder: `${BOOT}:25769803777`,
    mine: true,
    pid: 25769803777n,
    process: BOOT,
  },
  {
    holder: `${P}/ffffffff:21474836481`,
    mine: false,
    pid: 21474836481n,
    process: `${P}/ffffffff`,
  },
  { holder: 'replica-a:10', mine: false, pid: 0n, process: 'replica-a:10' },
  {
    holder: 'replica-a:10/0a0b0c0d:21474836481',
    mine: false,
    pid: 21474836481n,
    process: 'replica-a:10/0a0b0c0d',
  },
  { holder: 'replica-b:1', mine: false, pid: 0n, process: 'replica-b:1' },
]

describe('the holder rules of keyed_guard.t27, asked of PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  let pool: Pool | null = null
  const previousUrl = process.env.DATABASE_URL

  beforeEach(async () => {
    scratch = await scratchDatabase()
    if (!scratch) return
    process.env.DATABASE_URL = scratch.url
    await runPgMigrations()
    pool = createQueenPool(scratch.url)
    await ensureQueenColumns(pool)
  })

  afterEach(async () => {
    await pool?.end().catch(() => undefined)
    pool = null
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
  })

  /** A started bee the Queen runs herself, its row open or finished. */
  const row = async (issue: number, open: boolean) => {
    await (pool as Pool).query(
      `INSERT INTO queen_dispatch
         (issue, branch, started, detail, owned_paths, conversation_id,
          key_index, dispatched_at, finished_at)
       VALUES ($1, $2, true, 'running', '[]'::jsonb, $3, 0,
               now() - interval '1 hour', CASE WHEN $4 THEN NULL ELSE now() END)`,
      [issue, `queen-${issue}`, `conv-${issue}`, open],
    )
  }
  const live = async (issue: number) =>
    (
      await (pool as Pool).query(
        'SELECT expires_at >= now() AS live, renewed_at > acquired_at AS renewed FROM queen_task_lease WHERE issue = $1',
        [issue],
      )
    ).rows[0] as { live: boolean; renewed: boolean }

  it('a claim on a live lease lands exactly when claim_lands_for says', async () => {
    if (!pool) return expect(offlineRequested()).toBe(true)
    let issue = 5000
    let pairs = 0
    for (const held of HOLDERS)
      for (const claimer of HOLDERS) {
        issue++
        expect(
          (await claimTaskLease(pool, issue, held.holder, TTL)).landed,
        ).toBe(true)
        const store = (await claimTaskLease(pool, issue, claimer.holder, TTL))
          .landed
        const card =
          guard().call64(
            'claim_lands_for',
            1,
            held.process === claimer.process ? 1 : 0,
            held.pid,
            claimer.pid,
          ) !== 0
        expect([held.holder, claimer.holder, store]).toEqual([
          held.holder,
          claimer.holder,
          card,
        ])
        pairs++
      }
    expect(pairs).toBe(HOLDERS.length ** 2)
    // a lease that is not live lands for anyone
    await claimTaskLease(pool, 6000, HOLDERS[1].holder, TTL)
    await releaseTaskLease(pool, 6000, HOLDERS[1].holder)
    expect((await claimTaskLease(pool, 6000, 'replica-b:1', TTL)).landed).toBe(
      guard().call64('claim_lands_for', 0, 0, HOLDERS[1].pid, 0n) !== 0,
    )
  })

  it('the round renews exactly the leases round_renews names', async () => {
    if (!pool) return expect(offlineRequested()).toBe(true)
    let issue = 7000
    const cases: Array<{ issue: number; h: Holder; open: boolean }> = []
    for (const h of HOLDERS)
      for (const open of [true, false]) {
        issue++
        await row(issue, open)
        await claimTaskLease(pool, issue, h.holder, TTL)
        cases.push({ issue, h, open })
      }
    await new Promise((r) => setTimeout(r, 20))
    await renewRunningLeases(pool, P, TTL, BOOT)
    for (const c of cases) {
      const card =
        guard().call('round_renews', c.h.mine ? 1 : 0, c.open ? 1 : 0) !== 0
      expect([c.h.holder, c.open, (await live(c.issue)).renewed]).toEqual([
        c.h.holder,
        c.open,
        card,
      ])
    }
  })

  it('an end releases exactly the leases end_releases names', async () => {
    if (!pool) return expect(offlineRequested()).toBe(true)
    let issue = 8000
    for (const h of HOLDERS) {
      issue++
      await claimTaskLease(pool, issue, h.holder, TTL)
      await releaseTaskLease(pool, issue, P, BOOT)
      const card = guard().call('end_releases', h.mine ? 1 : 0) !== 0
      expect([h.holder, !(await live(issue)).live]).toEqual([h.holder, card])
    }
  })
})
