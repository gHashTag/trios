/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * STARTING THE VAULT IN THE SERVER (specs/vault/policy.t27 section 13,
 * gHashTag/trios#1759):
 *   1. off means off: with TRIOS_VAULT unset nothing is read -- not a
 *      variable, not the database -- and every vault route answers 503;
 *   2. on, a missing or malformed variable refuses the start before the
 *      database, with a log line naming the variable and no value;
 *   3. on, the identity comes from TRIOS_VAULT_IDENTITY held in memory, is
 *      deleted from the environment, and opens and seals rows as a file
 *      identity does; the owner keys come from TRIOS_VAULT_OWNER_KEYS;
 *   4. a store written under another identity or recovery recipient refuses
 *      the start;
 *   5. the vault's DDL is well-formed, re-runnable, and not in MIGRATION_SQL.
 * The database here is the memory store; tests/pglive/vault-live.test.ts asks
 * Postgres the same questions.
 */

import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  setDefaultTimeout,
} from 'bun:test'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import type { Pool } from 'pg'
import { createVaultRoute } from '../../src/api/routes/vault'
import {
  IDENTITY_VAR,
  OWNER_KEYS_VAR,
  RECOVERY_VAR,
  START_BAD_IDENTITY,
  START_IDENTITY_CHANGED,
  START_NO_DATABASE,
  START_NO_IDENTITY,
  START_NO_OWNER,
  START_NO_RECOVERY,
  START_NO_TOOL,
  START_OFF,
  START_ON,
  START_RECOVERY_CHANGED,
  TIER_OWNER,
} from '../../src/api/services/queen-vault-policy-card.gen'
import { keygenToFile } from '../../src/api/services/vault-age'
import { memoryAuditSink } from '../../src/api/services/vault-audit'
import {
  appTransport,
  sealValue,
  VaultClient,
} from '../../src/api/services/vault-client'
import {
  currentStart,
  startVault,
  stopVault,
} from '../../src/api/services/vault-start'
import { createMemoryStore } from '../../src/api/services/vault-store'
import { generateKey } from '../../src/api/services/vault-wire'
import { MIGRATION_SQL, QUEEN_VAULT_SQL } from '../../src/lib/db/pg-migrate'
import { logger } from '../../src/lib/logger'
import { describeStatement } from './pg-migrate-sql-facts'
import { parseStatements } from './pg-migrate-sql-parser'

/** The boot migration's own checks (tests/api/pg-migrate.test.ts), over the vault's DDL. */
function ddlOffences(sql: string): string[] {
  const parsed = parseStatements(sql)
  const offences = [...parsed.problems]
  for (const statement of parsed.statements)
    offences.push(...describeStatement(statement).problems)
  if (parsed.statements.length === 0) offences.push('no statement')
  return offences
}

setDefaultTimeout(30_000)

const LOGGED: string[] = []
const restore: Array<() => void> = []
let root: string
let identityText: string
let otherIdentityText: string
let recovery: string
let otherRecovery: string
const owner = generateKey()

/** A pool that fails the test if anything touches it. */
const untouchable = {
  query: () => {
    throw new Error('the database was touched')
  },
} as unknown as Pool

beforeAll(async () => {
  for (const n of ['debug', 'info', 'warn', 'error']) {
    const obj = logger as unknown as Record<
      string,
      (...a: unknown[]) => unknown
    >
    const orig = obj[n] as (...a: unknown[]) => unknown
    obj[n] = (...a: unknown[]) => {
      LOGGED.push(
        a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' '),
      )
      return orig.apply(obj, a)
    }
    restore.push(() => {
      obj[n] = orig
    })
  }
  root = await mkdtemp(join(tmpdir(), 'vault-start-'))
  await keygenToFile(join(root, 'id.txt'))
  await keygenToFile(join(root, 'id2.txt'))
  identityText = await readFile(join(root, 'id.txt'), 'utf8')
  otherIdentityText = await readFile(join(root, 'id2.txt'), 'utf8')
  recovery = await keygenToFile(join(root, 'recovery.txt'))
  otherRecovery = await keygenToFile(join(root, 'recovery2.txt'))
})

afterAll(async () => {
  for (const r of restore) r()
  stopVault()
  await rm(root, { recursive: true, force: true })
})

afterEach(() => stopVault())

const envOn = (over: Record<string, string | undefined> = {}) => ({
  TRIOS_VAULT: 'on',
  [IDENTITY_VAR]: identityText,
  [RECOVERY_VAR]: recovery,
  [OWNER_KEYS_VAR]: owner.publicHex,
  ...over,
})

const route = () => new Hono().route('/vault', createVaultRoute())

describe('off means off', () => {
  it('reads no variable and touches no database while TRIOS_VAULT is unset', async () => {
    const env: Record<string, string | undefined> = {
      [IDENTITY_VAR]: identityText,
    }
    const r = await startVault({ env, pool: untouchable })
    expect(r.code).toBe(START_OFF)
    // nothing was read: the identity is still where it was
    expect(env[IDENTITY_VAR]).toBe(identityText)
    for (const v of ['off', 'On', 'ON', '1', 'true', '']) {
      const r2 = await startVault({
        env: { TRIOS_VAULT: v },
        pool: untouchable,
      })
      expect(r2.code).toBe(START_OFF)
    }
  })

  it('answers 503 on every vault path and method', async () => {
    await startVault({ env: {}, pool: untouchable })
    const app = route()
    for (const [method, path] of [
      ['POST', '/vault/lease'],
      ['POST', '/vault/names'],
      ['GET', '/vault/names'],
      ['GET', '/vault/'],
      ['PUT', '/vault/export'],
      ['POST', '/vault/no-such-op'],
    ] as const) {
      const r = await app.request(path, { method })
      expect(r.status).toBe(503)
    }
  })

  it('keeps the vault DDL out of the boot migration, and makes it re-runnable', () => {
    expect(MIGRATION_SQL).not.toContain('queen_vault_store')
    expect(QUEEN_VAULT_SQL).toContain(
      'CREATE TABLE IF NOT EXISTS queen_vault_store',
    )
    expect(ddlOffences(QUEEN_VAULT_SQL)).toEqual([])
  })
})

describe('a bad variable refuses the start before the database, and its log names no value', () => {
  const cases: Array<
    [string, Record<string, string | undefined>, number, string]
  > = [
    [
      'no identity',
      { [IDENTITY_VAR]: undefined },
      START_NO_IDENTITY,
      IDENTITY_VAR,
    ],
    [
      'an identity that is not one',
      { [IDENTITY_VAR]: 'AGE-SECRET-KEY-1NOTAKEY' },
      START_BAD_IDENTITY,
      IDENTITY_VAR,
    ],
    [
      'no recovery recipient',
      { [RECOVERY_VAR]: '' },
      START_NO_RECOVERY,
      RECOVERY_VAR,
    ],
    [
      'a recovery that is not a recipient',
      { [RECOVERY_VAR]: 'age1nope' },
      START_NO_RECOVERY,
      RECOVERY_VAR,
    ],
    ['no owner key', { [OWNER_KEYS_VAR]: '' }, START_NO_OWNER, OWNER_KEYS_VAR],
    [
      'an owner key that is not one',
      { [OWNER_KEYS_VAR]: `${owner.publicHex},zz` },
      START_NO_OWNER,
      OWNER_KEYS_VAR,
    ],
  ]
  for (const [what, over, code, variable] of cases) {
    it(`refuses ${what}`, async () => {
      LOGGED.length = 0
      const r = await startVault({ env: envOn(over), pool: untouchable })
      expect(r.code).toBe(code)
      expect(r.vault).toBeNull()
      expect(currentStart()).toBe(code)
      const log = LOGGED.join('\n')
      expect(log).toContain('vault refused to start')
      expect(log).toContain(variable)
      expect(log).not.toContain('AGE-SECRET-KEY')
      expect(
        (await route().request('/vault/names', { method: 'POST' })).status,
      ).toBe(503)
    })
  }

  it('refuses without the age tool, and names the tool, not a value', async () => {
    LOGGED.length = 0
    const r = await startVault({
      env: envOn(),
      pool: untouchable,
      ageEnv: {
        TRIOS_VAULT_AGE: '/nonexistent/age',
        TRIOS_VAULT_AGE_KEYGEN: '/nonexistent/age-keygen',
      },
    })
    expect(r.code).toBe(START_NO_TOOL)
    expect(LOGGED.join('\n')).toContain('no-age')
    expect(LOGGED.join('\n')).not.toContain('AGE-SECRET-KEY')
  })

  it('refuses without a database', async () => {
    const r = await startVault({ env: envOn() })
    expect(r.code).toBe(START_NO_DATABASE)
  })
})

describe('on: the identity from the variable, the owner from the keys', () => {
  it('starts, deletes the identity from the environment, and serves the owner', async () => {
    LOGGED.length = 0
    const env = envOn()
    const store = createMemoryStore()
    const audit = memoryAuditSink()
    const r = await startVault({ env, store, audit })
    expect(r.code).toBe(START_ON)
    expect(env[IDENTITY_VAR]).toBeUndefined()
    expect(LOGGED.join('\n')).toContain('vault on: 0 secrets, 1 owner keys')
    expect(LOGGED.join('\n')).not.toContain('AGE-SECRET-KEY')
    expect(audit.rows.map((x) => x.name)).toEqual(['vault.grant'])

    const app = route()
    const c = new VaultClient(appTransport(app), '/vault', owner.privatePem)
    const rec = await c.recipients()
    expect(rec.recovery).toBe(recovery)
    const value = `vault-test-start-${crypto.randomUUID()}`
    await c.call('put', {
      id: 'SMOKE_SECRET',
      class: 'provider-api-key',
      scopes: ['smoke'],
      ct: await sealValue(new TextEncoder().encode(value), rec),
      input: 'stdin',
    })
    await c.call('grant', {
      subject: owner.publicHex,
      tier: TIER_OWNER,
      scopes: ['smoke'],
    })
    const leased = await c.open('lease', { scope: 'smoke' })
    expect(leased.env.SMOKE_SECRET).toBe(value)
    expect(store.text()).not.toContain(value)
    expect(LOGGED.join('\n')).not.toContain(value)

    // the same store, started again: same identity and recovery start; another refuses
    const again = await startVault({ env: envOn(), store, audit })
    expect(again.code).toBe(START_ON)
    const other = await startVault({
      env: envOn({ [IDENTITY_VAR]: otherIdentityText }),
      store,
      audit,
    })
    expect(other.code).toBe(START_IDENTITY_CHANGED)
    const otherRec = await startVault({
      env: envOn({ [RECOVERY_VAR]: otherRecovery }),
      store,
      audit,
    })
    expect(otherRec.code).toBe(START_RECOVERY_CHANGED)
    expect(
      (await route().request('/vault/names', { method: 'POST' })).status,
    ).toBe(503)
  })
})
