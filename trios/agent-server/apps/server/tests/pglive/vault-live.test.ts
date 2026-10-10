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
 * SKIPPED without TRIOS_PG_TEST_URL, and the skip is printed. The URL names
 * ONE database the lane created for itself (on the throwaway cluster of
 * trios#1761 lane B: createdb vault_test). Nothing here creates or drops a
 * database: each test gets a fresh schema in that one (QUEEN_DB_SCHEMA, which
 * every Queen pool pins as its search_path) and drops it after. Lane B's
 * traps are minded: stored JSON is compared as values, never as bytes
 * (jsonb reorders keys), and the audit is read in seq order.
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
import { createVaultRoute, vaultRoutes } from '../../src/api/routes/vault'
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
import { Vault } from '../../src/api/services/vault-service'
import { startVault, stopVault } from '../../src/api/services/vault-start'
import {
  createPgStore,
  emptyState,
  StoreConflict,
} from '../../src/api/services/vault-store'
import { generateKey, keyIdOf } from '../../src/api/services/vault-wire'
import { migrateVaultStore, runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool } from '../../src/lib/db/queen-pool'

setDefaultTimeout(60_000)

const adminUrl = process.env.TRIOS_PG_TEST_URL
/** A key id as the vault writes one: 16 hex characters of a public key's SHA-256. */
const KEY_ID = keyIdOf(generateKey().publicHex)

if (!adminUrl)
  console.log(
    'vault-live: SKIPPED, TRIOS_PG_TEST_URL is unset (no live PostgreSQL named)',
  )

describe.skipIf(!adminUrl)('the vault on PostgreSQL', () => {
  let admin: Pool | null = null
  let pool: Pool | null = null
  let schema = ''
  let root = ''
  const url = adminUrl as string
  const saved = {
    DATABASE_URL: process.env.DATABASE_URL,
    TRIOS_VAULT: process.env.TRIOS_VAULT,
    QUEEN_DB_SCHEMA: process.env.QUEEN_DB_SCHEMA,
  }

  beforeAll(async () => {
    root = await mkdtemp(join(tmpdir(), 'vault-live-'))
    admin = new Pool({ connectionString: url, max: 1 })
  })
  afterAll(async () => {
    await admin?.end().catch(() => undefined)
    await rm(root, { recursive: true, force: true })
  })

  beforeEach(async () => {
    schema = `vault_live_${randomBytes(6).toString('hex')}`
    process.env.QUEEN_DB_SCHEMA = schema
    process.env.DATABASE_URL = url
    delete process.env.TRIOS_VAULT
    await runPgMigrations()
    pool = createQueenPool(url)
  })

  afterEach(async () => {
    stopVault()
    await pool?.end().catch(() => undefined)
    pool = null
    await (admin as Pool).query(`DROP SCHEMA IF EXISTS ${schema} CASCADE`)
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]
      else process.env[k] = v
    }
  })

  /** A Queen of its own: a pool as a second process would have. */
  const secondPool = () => createQueenPool(url)

  const identityAndRecovery = async () => {
    const tag = randomBytes(3).toString('hex')
    await keygenToFile(join(root, `id-${tag}.txt`))
    const identity = await readFile(join(root, `id-${tag}.txt`), 'utf8')
    const recovery = await keygenToFile(join(root, `recovery-${tag}.txt`))
    return {
      identity,
      recovery,
      recoveryFile: join(root, `recovery-${tag}.txt`),
    }
  }

  const tableExists = async () =>
    Number(
      (
        await (pool as Pool).query(
          "SELECT COUNT(*)::int AS n FROM information_schema.tables WHERE table_name = 'queen_vault_store' AND table_schema = $1",
          [schema],
        )
      ).rows[0].n,
    ) === 1

  it('creates no vault table while TRIOS_VAULT is unset, and creates it once when on', async () => {
    expect(await tableExists()).toBe(false)
    const off = await startVault({ env: { DATABASE_URL: url } })
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
      DATABASE_URL: url,
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

  it('migrates under a lock: twice in a row, and by two Queens at once', async () => {
    const other = secondPool()
    try {
      const a = pool as Pool
      await Promise.all([
        migrateVaultStore(a),
        migrateVaultStore(other),
        migrateVaultStore(a),
        migrateVaultStore(other),
      ])
      await migrateVaultStore(a)
      expect(await tableExists()).toBe(true)
    } finally {
      await other.end().catch(() => undefined)
    }
  })

  it('starts two Queens at once on an empty store: one row, both on', async () => {
    const { identity, recovery } = await identityAndRecovery()
    const owner = generateKey()
    const env = () => ({
      TRIOS_VAULT: 'on',
      DATABASE_URL: url,
      [IDENTITY_VAR]: identity,
      [RECOVERY_VAR]: recovery,
      [OWNER_KEYS_VAR]: owner.publicHex,
    })
    const other = secondPool()
    try {
      const [a, b] = await Promise.all([
        startVault({ env: env(), pool: pool as Pool }),
        startVault({ env: env(), pool: other }),
      ])
      expect([a.code, b.code]).toEqual([START_ON, START_ON])
      const rows = await (pool as Pool).query(
        'SELECT state FROM queen_vault_store',
      )
      expect(rows.rows.length).toBe(1)
      expect(rows.rows[0].state.grants[owner.publicHex].tier).toBe(TIER_OWNER)
    } finally {
      await other.end().catch(() => undefined)
    }
  })

  it('never answers from a stale copy: a second Queen leases what the first rotated', async () => {
    const { identity, recovery } = await identityAndRecovery()
    const owner = generateKey()
    const started = await startVault({
      env: {
        TRIOS_VAULT: 'on',
        DATABASE_URL: url,
        [IDENTITY_VAR]: identity,
        [RECOVERY_VAR]: recovery,
        [OWNER_KEYS_VAR]: owner.publicHex,
      },
      pool: pool as Pool,
    })
    expect(started.code).toBe(START_ON)
    const other = secondPool()
    try {
      const second = new Vault({
        identity: { text: identity },
        store: createPgStore(other),
        audit: busAuditSink(other),
      })
      const a = new VaultClient(
        appTransport(new Hono().route('/vault', createVaultRoute())),
        '/vault',
        owner.privatePem,
      )
      const b = new VaultClient(
        appTransport(
          new Hono().route(
            '/vault',
            vaultRoutes(() => second),
          ),
        ),
        '/vault',
        owner.privatePem,
      )
      const one = `vault-test-live-one-${randomBytes(12).toString('hex')}`
      const two = `vault-test-live-two-${randomBytes(12).toString('hex')}`
      const r = await a.recipients()
      await a.call('put', {
        id: 'SHARED_KEY',
        class: 'provider-api-key',
        scopes: ['both'],
        ct: await sealValue(new TextEncoder().encode(one), r),
        input: 'stdin',
      })
      await a.call('grant', {
        subject: owner.publicHex,
        tier: TIER_OWNER,
        scopes: ['both'],
      })
      expect((await b.open('lease', { scope: 'both' })).env.SHARED_KEY).toBe(
        one,
      )
      await a.call('rotate', {
        id: 'SHARED_KEY',
        ct: await sealValue(new TextEncoder().encode(two), r),
        input: 'stdin',
      })
      expect((await b.open('lease', { scope: 'both' })).env.SHARED_KEY).toBe(
        two,
      )
    } finally {
      await other.end().catch(() => undefined)
    }
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
