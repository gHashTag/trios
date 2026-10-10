/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE VAULT AGAINST POSTGRESQL (gHashTag/trios#1759, specs/vault/*.t27). The
 * unit tests run the vault on the file and memory stores; this asks Postgres
 * what only it can answer:
 *   1. off means off at boot: with TRIOS_VAULT unset the boot migration
 *      creates no vault table; on, it creates it, and a second boot is a
 *      no-op;
 *   2. the server's start on a real database: the store row is written,
 *      a value put, leased back, and leased again by a vault started anew on
 *      the same database (a redeploy); the row holds no value;
 *   3. two writers over one row: the second is refused, not merged;
 *   4. the audit on the events bus: its own gapless stream beside the
 *      Queen's, and an event audit.t27 refuses never reaches the table.
 *
 * SKIPPED without TRIOS_PG_TEST_URL. Unlike the other live tests this one does
 * not fall back to a local server, because the vault lane installs none (the
 * throwaway Postgres is trios#1761 lane B's). Same harness otherwise: a scratch
 * database per test, migrated by the boot migration.
 */

import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  setDefaultTimeout,
} from 'bun:test'
import { randomBytes } from 'node:crypto'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import { Pool } from 'pg'
import { createVaultRoute } from '../../src/api/routes/vault'
import { publishEvent } from '../../src/api/services/queen-control'
import {
  AUDIT_STREAM,
  EV_DENY,
  EV_LEASE,
} from '../../src/api/services/queen-vault-audit-card.gen'
import {
  IDENTITY_VAR,
  OWNER_KEYS_VAR,
  RECOVERY_VAR,
  START_ON,
  TIER_OWNER,
} from '../../src/api/services/queen-vault-policy-card.gen'
import { decryptWithFile, keygenToFile } from '../../src/api/services/vault-age'
import { AuditRefused, busAuditSink } from '../../src/api/services/vault-audit'
import {
  appTransport,
  sealValue,
  VaultClient,
} from '../../src/api/services/vault-client'
import { startVault, stopVault } from '../../src/api/services/vault-start'
import {
  createPgStore,
  emptyState,
  StoreConflict,
} from '../../src/api/services/vault-store'
import { generateKey, keyIdOf } from '../../src/api/services/vault-wire'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool, queenSchema } from '../../src/lib/db/queen-pool'

setDefaultTimeout(60_000)

const adminUrl = process.env.TRIOS_PG_TEST_URL
/** A key id as the vault writes one: 16 hex characters of a public key's SHA-256. */
const KEY_ID = keyIdOf(generateKey().publicHex)

async function scratchDatabase(
  admin: string,
): Promise<{ url: string; drop: () => Promise<void> }> {
  const name = `vault_live_${randomBytes(6).toString('hex')}`
  const pool = new Pool({ connectionString: admin, max: 1 })
  await pool.query(`CREATE DATABASE ${name}`)
  const url = new URL(admin)
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
      await pool
        .query(`DROP DATABASE IF EXISTS ${name} WITH (FORCE)`)
        .catch(() => undefined)
      await pool.end().catch(() => undefined)
    },
  }
}

describe.skipIf(!adminUrl)('the vault on PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  let pool: Pool | null = null
  let root = ''
  const saved = {
    DATABASE_URL: process.env.DATABASE_URL,
    TRIOS_VAULT: process.env.TRIOS_VAULT,
  }

  beforeAll(async () => {
    root = await mkdtemp(join(tmpdir(), 'vault-live-'))
  })
  afterAll(async () => {
    await rm(root, { recursive: true, force: true })
  })

  beforeEach(async () => {
    scratch = await scratchDatabase(adminUrl as string)
    process.env.DATABASE_URL = scratch.url
    delete process.env.TRIOS_VAULT
    await runPgMigrations()
    pool = createQueenPool(scratch.url)
  })

  afterEach(async () => {
    stopVault()
    await pool?.end().catch(() => undefined)
    pool = null
    await scratch?.drop()
    scratch = null
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]
      else process.env[k] = v
    }
  })

  const tableExists = async () =>
    Number(
      (
        await (pool as Pool).query(
          "SELECT COUNT(*)::int AS n FROM information_schema.tables WHERE table_name = 'queen_vault_store'",
        )
      ).rows[0].n,
    ) === 1

  it('creates no vault table while TRIOS_VAULT is unset, and creates it once when on', async () => {
    expect(await tableExists()).toBe(false)
    const off = await startVault({ env: { DATABASE_URL: scratch?.url } })
    expect(off.code).not.toBe(START_ON)
    expect(await tableExists()).toBe(false)
    process.env.TRIOS_VAULT = 'on'
    await runPgMigrations()
    expect(await tableExists()).toBe(true)
    await runPgMigrations()
    expect(await tableExists()).toBe(true)
  })

  it('starts on the database, serves a lease, and serves it again after a restart', async () => {
    const owner = generateKey()
    await keygenToFile(join(root, 'id.txt'))
    const identity = await readFile(join(root, 'id.txt'), 'utf8')
    const recovery = await keygenToFile(join(root, 'recovery.txt'))
    const env = () => ({
      TRIOS_VAULT: 'on',
      DATABASE_URL: scratch?.url,
      [IDENTITY_VAR]: identity,
      [RECOVERY_VAR]: recovery,
      [OWNER_KEYS_VAR]: owner.publicHex,
    })
    const first = await startVault({ env: env() })
    expect(first.code).toBe(START_ON)
    const app = new Hono().route('/vault', createVaultRoute())
    const c = new VaultClient(appTransport(app), '/vault', owner.privatePem)
    const value = `vault-test-live-${randomBytes(16).toString('hex')}`
    await c.call('put', {
      id: 'SMOKE_SECRET',
      class: 'provider-api-key',
      scopes: ['smoke'],
      ct: await sealValue(
        new TextEncoder().encode(value),
        await c.recipients(),
      ),
      input: 'stdin',
    })
    await c.call('grant', {
      subject: owner.publicHex,
      tier: TIER_OWNER,
      scopes: ['smoke'],
    })
    expect((await c.open('lease', { scope: 'smoke' })).env.SMOKE_SECRET).toBe(
      value,
    )

    // a redeploy: a new start over the same database
    stopVault()
    const second = await startVault({ env: env() })
    expect(second.code).toBe(START_ON)
    const c2 = new VaultClient(
      appTransport(new Hono().route('/vault', createVaultRoute())),
      '/vault',
      owner.privatePem,
    )
    expect((await c2.open('lease', { scope: 'smoke' })).env.SMOKE_SECRET).toBe(
      value,
    )

    const row = await (pool as Pool).query(
      'SELECT state::text AS s, version FROM queen_vault_store',
    )
    expect(row.rows.length).toBe(1)
    expect(String(row.rows[0].s)).not.toContain(value)
    expect(Number(row.rows[0].version)).toBeGreaterThan(1)
    const events = await (pool as Pool).query(
      'SELECT payload::text AS p FROM queen_event_log WHERE stream = $1',
      [AUDIT_STREAM],
    )
    expect(events.rows.length).toBeGreaterThan(3)
    for (const e of events.rows) expect(String(e.p)).not.toContain(value)

    // the export opens with the recovery identity
    const ex = await c2.call('export')
    const plain = await decryptWithFile(
      new Uint8Array(Buffer.from(String(ex.ct), 'base64')),
      join(root, 'recovery.txt'),
    )
    expect(new TextDecoder().decode(plain)).toContain(
      Buffer.from(value).toString('base64'),
    )
  })

  it('refuses the second of two writers over one row', async () => {
    process.env.TRIOS_VAULT = 'on'
    await runPgMigrations()
    const a = createPgStore(pool as Pool)
    const b = createPgStore(pool as Pool)
    const state = emptyState('age1x', 'age1y')
    await a.save(state)
    const sa = await a.load()
    const sb = await b.load()
    expect(sa).not.toBeNull()
    await a.save({ ...(sa as typeof state), sweptAt: 1 })
    await expect(
      b.save({ ...(sb as typeof state), sweptAt: 2 }),
    ).rejects.toBeInstanceOf(StoreConflict)
    expect((await b.load())?.sweptAt).toBe(1)
  })

  it('numbers its own audit stream without a gap beside the Queen stream, and reads it back', async () => {
    const p = pool as Pool
    const sink = busAuditSink(p)
    const lease = {
      key: KEY_ID,
      scope: 'svc-a',
      ids: ['SVC_API_KEY'],
      lease: 'a'.repeat(32),
    }
    const got = await Promise.all(
      Array.from({ length: 20 }, () => sink.append(EV_LEASE, lease)),
    )
    await publishEvent(p, 'queen/task.assign', { issue: 1 })
    expect([...got].sort((x, y) => x - y)).toEqual(
      Array.from({ length: 20 }, (_, i) => i + 1),
    )
    const rows = await sink.read(0, 100)
    expect(rows.length).toBe(20)
    expect(rows[0]?.name).toBe('vault.lease')
    expect(rows[0]?.payload).toEqual(lease)
    const streams = await p.query(
      'SELECT stream, COUNT(*)::int AS n FROM queen_event_log GROUP BY stream ORDER BY stream',
    )
    expect(streams.rows).toEqual([
      { stream: 'queen', n: 1 },
      { stream: AUDIT_STREAM, n: 20 },
    ])
  })

  it('never writes an event audit.t27 refuses', async () => {
    const p = pool as Pool
    const sink = busAuditSink(p)
    await expect(
      sink.append(EV_DENY, { op: 'lease', code: 0 }),
    ).rejects.toBeInstanceOf(AuditRefused)
    await expect(
      sink.append(EV_LEASE, {
        key: KEY_ID,
        scope: 'a',
        ids: ['A'],
        lease: 'b',
        value: 'x',
      } as never),
    ).rejects.toBeInstanceOf(AuditRefused)
    const n = await p.query(
      'SELECT COUNT(*)::int AS n FROM queen_event_log WHERE stream = $1',
      [AUDIT_STREAM],
    )
    expect(n.rows[0].n).toBe(0)
  })
})
