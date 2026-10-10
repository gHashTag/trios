/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE VAULT (gHashTag/t27 specs/vault/*.t27, gHashTag/trios#1759):
 *   1. the vendored cards are the ones PIN names, and the wasm runs the
 *      specs' own vectors;
 *   2. requests: an unknown key, a changed body, a stale time, a replayed
 *      nonce and a short nonce are refused;
 *   3. leases: cross-scope, public-tier and class-tier denies; TTL expiry;
 *      renewal past the cap; rotation, rebinding and merging end leases;
 *      only the owner puts, imports, merges and exports; an act whose event
 *      cannot be written is refused;
 *   4. the owner's migration: an Infisical dotenv stream and a Railway
 *      railway-kv stream imported, leased back value for value, deduplicated
 *      by name only, merged by a plan, exported to the recovery recipient
 *      only, and restored from that export into a new vault;
 *   5. `run` puts the values in the child's environment and nowhere else, and
 *      stops the child when its lease ends;
 *   6. the CLI end to end, as the owner would type it;
 *   7. THE LEAK SCAN: every log line, console line, event, HTTP response,
 *      CLI output, store file and audit file of this whole file is searched
 *      for every dummy secret's bytes, base64, hex and SHA-256.
 * Every secret here is a dummy made at test time. The clock is virtual.
 */

import {
  afterAll,
  beforeAll,
  describe,
  expect,
  it,
  setDefaultTimeout,
} from 'bun:test'
import { createHash, createPrivateKey, randomBytes, sign } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import { vaultRoutes } from '../../src/api/routes/vault'
import {
  EV_BREAK_GLASS,
  EV_DENY,
  EV_EXPIRE,
  EV_IMPORT,
  EV_LEASE,
  EV_MERGE,
  EV_ROTATION_DUE,
  EVENT_NAMES,
} from '../../src/api/services/queen-vault-audit-card.gen'
import {
  LK_CONTINUATION,
  LK_ENTRY,
  REPORT_FIELDS,
  VL_SECRET,
} from '../../src/api/services/queen-vault-merge-card.gen'
import {
  BIND_ADD,
  BIND_REPLACE,
  CLASS_DB_URL,
  CLASS_OTHER,
  CLASS_PROVIDER_API_KEY,
  CODE_OK,
  DENY_ALIAS,
  DENY_BAD_CIPHERTEXT,
  DENY_BAD_NAME,
  DENY_CLASS_TIER,
  DENY_DUPLICATE_IN_STREAM,
  DENY_EMPTY_SCOPE,
  DENY_EXISTS,
  DENY_GRANT_TIER,
  DENY_LEASE_EXPIRED,
  DENY_LEASE_REVOKED,
  DENY_LEASE_ROTATED,
  DENY_MERGE_CONFLICT,
  DENY_MISSING,
  DENY_NO_AUDIT,
  DENY_NO_GRANT,
  DENY_NO_REASON,
  DENY_NOT_HOLDER,
  DENY_NOT_OWNER,
  DENY_PUBLIC_TIER,
  DENY_RENEWALS_SPENT,
  DENY_REPLAY,
  DENY_SIGNATURE,
  DENY_STALE,
  DENY_TOO_MANY,
  LEASE_TTL_SECONDS,
  NONCE_MIN_BYTES,
  RENEWALS_MAX,
  REQUEST_DOMAIN,
  REQUEST_SIGNED_FIELDS,
  REQUEST_SKEW_SECONDS,
  SWEEP_EVERY_SECONDS,
  TIER_OWNER,
  TIER_PUBLIC,
  TIER_TRUSTED,
} from '../../src/api/services/queen-vault-policy-card.gen'
import {
  decryptWithFile,
  encrypt,
  ephemeralIdentity,
  keygenToFile,
  recipientOfFile,
} from '../../src/api/services/vault-age'
import {
  type AuditRow,
  memoryAuditSink,
} from '../../src/api/services/vault-audit'
import {
  bindCode,
  bindRotates,
  classReaches,
  eventCode,
  importCode,
  leaseAdmitted,
  leaseCode,
  lineKind,
  mergeCode,
  renewCode,
  requestCode,
  scopeCode,
  vaultLineKind,
} from '../../src/api/services/vault-cards'
import {
  type Answer,
  appTransport,
  importBody,
  runScoped,
  sealValue,
  type Transport,
  VaultClient,
  VaultRefused,
} from '../../src/api/services/vault-client'
import {
  FMT_DOTENV,
  FMT_RAILWAY_KV,
  parseKvStream,
  parsePlan,
  parseVaultDump,
} from '../../src/api/services/vault-formats'
import { initVault, Vault } from '../../src/api/services/vault-service'
import {
  createFileStore,
  IDENTITY_FILE,
  STORE_FILE,
} from '../../src/api/services/vault-store'
import {
  generateKey,
  keyIdOf,
  sha256Hex,
  signRequest,
} from '../../src/api/services/vault-wire'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { logger } from '../../src/lib/logger'

// Every test here starts age (and the CLI test starts bun) as processes; on a
// loaded host a few of them together outlast bun's 5 s default.
setDefaultTimeout(60_000)

// --- dummies, and everything the vault ever says ------------------------------

const SECRETS: string[] = []
const dummy = (label: string): string => {
  const v = `vault-test-${label}-${randomBytes(18).toString('hex')}`
  SECRETS.push(v)
  return v
}
const pemDummy = (): string => {
  const body = [0, 1, 2].map(() => randomBytes(48).toString('base64'))
  // a base64 line that reads as `NAME=value`: inside a PEM block it is the key
  const tail = `AbC${randomBytes(6).toString('hex')}==`
  const v = [
    '-----BEGIN TEST KEY-----',
    ...body,
    tail,
    '-----END TEST KEY-----',
  ].join('\n')
  SECRETS.push(v, ...body, tail)
  return v
}

/** Every text the vault produced: responses, events, logs, files, CLI output. */
const SAID: string[] = []
const transport = (app: Hono): Transport => {
  const inner = appTransport(app)
  return async (path, init) => {
    const r = await inner(path, init)
    SAID.push(r.text)
    return r
  }
}

const sha = (rel: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, rel)))
    .digest('hex')

const restore: Array<() => void> = []
beforeAll(() => {
  const capture = <T extends (...a: unknown[]) => unknown>(
    obj: Record<string, unknown>,
    name: string,
  ) => {
    const orig = obj[name] as T
    obj[name] = (...a: unknown[]) => {
      SAID.push(
        a.map((x) => (typeof x === 'string' ? x : JSON.stringify(x))).join(' '),
      )
      return orig.apply(obj, a)
    }
    restore.push(() => {
      obj[name] = orig
    })
  }
  for (const n of ['log', 'info', 'warn', 'error', 'debug'])
    capture(console as unknown as Record<string, unknown>, n)
  for (const n of ['debug', 'info', 'warn', 'error'])
    capture(logger as unknown as Record<string, unknown>, n)
})
afterAll(() => {
  for (const r of restore) r()
})

// --- a vault in a temp directory on a virtual clock ---------------------------

const T0 = 1_800_000_000

interface World {
  root: string
  dir: string
  clock: { t: number }
  audit: ReturnType<typeof memoryAuditSink>
  vault: Vault
  app: Hono
  recoveryFile: string
  recovery: string
  owner: { privatePem: string; publicHex: string }
  client(pem: string, skew?: number): VaultClient
}

async function world(root?: string): Promise<World> {
  const base = root ?? (await mkdtemp(join(tmpdir(), 'vault-test-')))
  const dir = join(base, `v-${randomBytes(3).toString('hex')}`)
  const recoveryFile = join(base, 'recovery.txt')
  let recovery: string
  try {
    recovery = await recipientOfFile(recoveryFile)
  } catch {
    recovery = await keygenToFile(recoveryFile)
  }
  const owner = generateKey()
  const audit = memoryAuditSink()
  const clock = { t: T0 }
  const store = createFileStore(dir)
  await initVault({
    dir,
    store,
    audit,
    recoveryRecipient: recovery,
    ownerPublicHex: owner.publicHex,
    now: T0,
  })
  const vault = new Vault({
    identity: { file: join(dir, IDENTITY_FILE) },
    store,
    audit,
    now: () => clock.t,
  })
  const app = new Hono().route(
    '/vault',
    vaultRoutes(() => vault),
  )
  return {
    root: base,
    dir,
    clock,
    audit,
    vault,
    app,
    recoveryFile,
    recovery,
    owner,
    client: (pem, skew = 0) =>
      new VaultClient(transport(app), '/vault', pem, () => clock.t + skew),
  }
}

async function refusal(p: Promise<unknown>): Promise<number> {
  try {
    await p
  } catch (error) {
    if (error instanceof VaultRefused) return error.code
    throw error
  }
  throw new Error('expected a refusal')
}

const WORLDS: World[] = []

// --- 1. the cards ------------------------------------------------------------------

describe('the vendored cards are the ones PIN names', () => {
  for (const card of ['policy', 'audit', 'merge']) {
    it(`vault/${card}.t27 and its wasm match, and the wasm exports every fn`, () => {
      const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN')).toString('utf8')
      expect(pin).toContain(
        `vault/${card}.t27 sha256 ${sha(`vault/${card}.t27`)}`,
      )
      expect(pin).toContain(
        `vault/${card}.wasm sha256 ${sha(`vault/${card}.wasm`)}`,
      )
      const spec = readFileSync(
        join(DEFAULT_SPECS_ROOT, `vault/${card}.t27`),
      ).toString('utf8')
      const fns = [...spec.matchAll(/^pub fn (\w+)\(/gm)].map((m) => m[1])
      const exported = WebAssembly.Module.exports(
        new WebAssembly.Module(
          readFileSync(join(DEFAULT_SPECS_ROOT, `vault/${card}.wasm`)),
        ),
      ).map((e) => e.name)
      expect(fns.length).toBeGreaterThan(5)
      for (const fn of fns) expect(exported).toContain(fn)
    })
  }
})

describe('the cards, as the wasm runs them', () => {
  // policy.t27 test classes_never_reach_a_public_host
  it('lets no class reach a public host, and keeps a db-url and a signing key on the owner', () => {
    for (let c = 0; c < 6; c++) {
      expect(classReaches(c, TIER_PUBLIC)).toBe(false)
      expect(classReaches(c, TIER_OWNER)).toBe(true)
    }
    expect(classReaches(CLASS_PROVIDER_API_KEY, TIER_TRUSTED)).toBe(true)
    expect(classReaches(CLASS_DB_URL, TIER_TRUSTED)).toBe(false)
    expect(classReaches(CLASS_OTHER, TIER_TRUSTED)).toBe(false)
  })

  // policy.t27 tests an_unsigned_stale_or_replayed_request_is_refused, a_lease_*
  it('refuses unsigned, stale and replayed requests, and leases all or nothing', () => {
    const ok = {
      keyKnown: true,
      signatureOk: true,
      skewSeconds: 0,
      nonceBytes: 16,
      nonceSeen: false,
      cacheSize: 0,
    }
    expect(requestCode(ok)).toBe(CODE_OK)
    expect(requestCode({ ...ok, signatureOk: false, nonceSeen: true })).toBe(
      DENY_SIGNATURE,
    )
    expect(requestCode({ ...ok, nonceBytes: NONCE_MIN_BYTES - 1 })).toBe(
      DENY_SIGNATURE,
    )
    expect(requestCode({ ...ok, skewSeconds: REQUEST_SKEW_SECONDS + 1 })).toBe(
      DENY_STALE,
    )
    expect(requestCode({ ...ok, nonceSeen: true })).toBe(DENY_REPLAY)
    expect(scopeCode(TIER_TRUSTED, false)).toBe(DENY_NO_GRANT)
    expect(leaseCode(TIER_PUBLIC, true, true, CLASS_PROVIDER_API_KEY)).toBe(
      DENY_PUBLIC_TIER,
    )
    expect(leaseCode(TIER_TRUSTED, true, true, CLASS_DB_URL)).toBe(
      DENY_CLASS_TIER,
    )
    expect(leaseAdmitted(3, 1, DENY_CLASS_TIER)).toBe(DENY_CLASS_TIER)
    expect(leaseAdmitted(0, 0, CODE_OK)).toBe(DENY_EMPTY_SCOPE)
    const r = {
      holder: true,
      revoked: false,
      rotated: false,
      now: 10,
      expires: 900,
      renewals: 0,
      breakGlass: false,
    }
    expect(renewCode(r)).toBe(CODE_OK)
    expect(renewCode({ ...r, rotated: true })).toBe(DENY_LEASE_ROTATED)
    expect(renewCode({ ...r, renewals: RENEWALS_MAX })).toBe(
      DENY_RENEWALS_SPENT,
    )
    expect(renewCode({ ...r, now: 900 })).toBe(DENY_LEASE_EXPIRED)
    expect(bindCode(BIND_ADD, true, true)).toBe(DENY_EXISTS)
    expect(bindRotates(BIND_REPLACE, false)).toBe(true)
    expect(bindRotates(BIND_REPLACE, true)).toBe(false)
  })

  // merge.t27 tests inside_a_pem_block_every_line_is_the_key, an_import_is_all_or_nothing_*,
  // only_the_owners_plan_merges_*, the_vaults_export_is_read_record_by_record
  it('reads PEM blocks, imports all or nothing, merges only what the owner says', () => {
    const line = {
      blank: false,
      header: false,
      comment: false,
      assignment: true,
      afterEntry: true,
      openBlock: true,
    }
    expect(lineKind(FMT_RAILWAY_KV, line)).toBe(LK_CONTINUATION)
    expect(lineKind(FMT_RAILWAY_KV, { ...line, openBlock: false })).toBe(
      LK_ENTRY,
    )
    expect(importCode(TIER_OWNER, 3, 1, DENY_EXISTS)).toBe(DENY_EXISTS)
    expect(importCode(TIER_TRUSTED, 3, 0, CODE_OK)).toBe(DENY_NOT_OWNER)
    const m = {
      tier: TIER_OWNER,
      members: 2,
      valuesEqual: false,
      keeperNamed: false,
      keeperMember: false,
      canonicalOk: true,
      cls: 0,
    }
    expect(mergeCode(m)).toBe(DENY_MERGE_CONFLICT)
    expect(mergeCode({ ...m, keeperNamed: true, keeperMember: true })).toBe(
      CODE_OK,
    )
    expect(vaultLineKind(false, false, false, 0, 6)).toBe(VL_SECRET)
    // audit.t27: a lease event may not carry a ciphertext hash
    expect(
      eventCode(
        EV_LEASE,
        (1 << 1) | (1 << 3) | (1 << 4) | (1 << 6) | (1 << 12),
        0,
      ),
    ).not.toBe(0)
  })
})

// --- 2-3. requests and leases -----------------------------------------------------------

describe('requests and leases', () => {
  let w: World
  let owner: VaultClient
  const work = generateKey()
  let worker: VaultClient
  const SVC = dummy('svc-api-key')
  const SVC2 = dummy('svc-api-key-rotated')
  const DB = dummy('svc-db-url')

  beforeAll(async () => {
    w = await world()
    WORLDS.push(w)
    owner = w.client(w.owner.privatePem)
    worker = w.client(work.privatePem)
    const r = await owner.recipients()
    await owner.call('put', {
      id: 'SVC_API_KEY',
      class: 'provider-api-key',
      scopes: ['svc-a'],
      ct: await sealValue(new TextEncoder().encode(SVC), r),
      input: 'stdin',
    })
    await owner.call('put', {
      id: 'SVC_DB_URL',
      class: 'db-url',
      scopes: ['svc-db'],
      ct: await sealValue(new TextEncoder().encode(DB), r),
      input: 'stdin',
    })
    await owner.call('grant', {
      subject: work.publicHex,
      tier: TIER_TRUSTED,
      scopes: ['svc-a', 'svc-db'],
    })
  })

  it('leases a granted scope: the answer is ciphertext, the values come out only in memory', async () => {
    const { recipient } = await ephemeralIdentity()
    const raw = await worker.call('lease', { scope: 'svc-a', recipient })
    expect(raw.names).toEqual(['SVC_API_KEY'])
    expect(JSON.stringify(raw)).not.toContain(SVC)
    const leased = await worker.open('lease', { scope: 'svc-a' })
    expect(leased.env.SVC_API_KEY).toBe(SVC)
  })

  it('refuses a scope that is not granted, and does not say whether it exists', async () => {
    expect(await refusal(worker.open('lease', { scope: 'svc-b' }))).toBe(
      DENY_NO_GRANT,
    )
    expect(await refusal(worker.call('names', { scope: 'svc-b' }))).toBe(
      DENY_NO_GRANT,
    )
    expect(
      await refusal(worker.open('lease', { scope: 'no-such-scope' })),
    ).toBe(DENY_NO_GRANT)
  })

  it('refuses a db-url to a trusted host, whatever it was granted', async () => {
    expect(await refusal(worker.open('lease', { scope: 'svc-db' }))).toBe(
      DENY_CLASS_TIER,
    )
  })

  it('never grants a public host, and refuses one even when the store says it was', async () => {
    const pub = generateKey()
    expect(
      await refusal(
        owner.call('grant', {
          subject: pub.publicHex,
          tier: TIER_PUBLIC,
          scopes: ['svc-a'],
        }),
      ),
    ).toBe(DENY_GRANT_TIER)
    // a store edited by hand: the card still refuses the tier on every operation
    const store = createFileStore(w.dir)
    const state = await store.load()
    if (!state) throw new Error('no store')
    state.grants[pub.publicHex] = {
      tier: TIER_PUBLIC,
      scopes: ['svc-a'],
      grantedAt: T0,
    }
    await store.save(state)
    const fresh = new Vault({
      identity: { file: join(w.dir, IDENTITY_FILE) },
      store,
      audit: w.audit,
      now: () => w.clock.t,
    })
    const app = new Hono().route(
      '/vault',
      vaultRoutes(() => fresh),
    )
    const c = new VaultClient(
      transport(app),
      '/vault',
      pub.privatePem,
      () => w.clock.t,
    )
    expect(await refusal(c.open('lease', { scope: 'svc-a' }))).toBe(
      DENY_PUBLIC_TIER,
    )
    expect(await refusal(c.call('names'))).toBe(DENY_PUBLIC_TIER)
  })

  it('refuses an unknown key, a changed body, a stale time, a short nonce and a replay', async () => {
    const stranger = generateKey()
    expect(await refusal(w.client(stranger.privatePem).call('names'))).toBe(
      DENY_SIGNATURE,
    )
    // a key the vault never granted writes nothing (policy.t27 deny_recorded)
    expect(
      w.audit.rows.some(
        (r) =>
          r.kind === EV_DENY && r.payload.key === keyIdOf(stranger.publicHex),
      ),
    ).toBe(false)
    expect(
      await refusal(
        w.client(work.privatePem, -(REQUEST_SKEW_SECONDS + 1)).call('names'),
      ),
    ).toBe(DENY_STALE)

    const send = async (headers: Record<string, string>, body: string) => {
      const r = await transport(w.app)('/vault/names', {
        method: 'POST',
        headers: { 'content-type': 'application/json', ...headers },
        body,
      })
      return { status: r.status, body: JSON.parse(r.text) as Answer }
    }
    const body = JSON.stringify({ scope: 'svc-a' })
    const headers = signRequest(
      work.privatePem,
      'POST',
      '/vault/names',
      body,
      w.clock.t,
    )
    expect(
      (await send(headers, JSON.stringify({ scope: 'svc-db' }))).body.code,
    ).toBe(DENY_SIGNATURE)
    const first = await send(headers, body)
    expect(first.status).toBe(200)
    const again = await send(headers, body)
    expect(again.status).toBe(401)
    expect(again.body.code).toBe(DENY_REPLAY)

    // a nonce shorter than signed_receipt.t27 NONCE_MIN_BYTES, correctly signed
    const nonce = randomBytes(NONCE_MIN_BYTES - 8).toString('hex')
    const facts: Record<string, string | number> = {
      method: 'POST',
      path: '/vault/names',
      key: work.publicHex,
      utc_unix: w.clock.t,
      nonce,
      body_sha256: sha256Hex(body),
    }
    let message = `${REQUEST_DOMAIN}\n`
    for (const f of REQUEST_SIGNED_FIELDS)
      message += `${f}=${JSON.stringify(facts[f])}\n`
    const signature = sign(
      null,
      Buffer.from(message),
      createPrivateKey(work.privatePem),
    ).toString('hex')
    const short = await send(
      {
        'x-vault-key': work.publicHex,
        'x-vault-time': String(w.clock.t),
        'x-vault-nonce': nonce,
        'x-vault-signature': signature,
      },
      body,
    )
    expect(short.body.code).toBe(DENY_SIGNATURE)
    const denies = w.audit.rows
      .filter((r) => r.kind === EV_DENY)
      .map((r) => r.payload.code)
    expect(denies).toContain(DENY_REPLAY)
    expect(denies).toContain(DENY_STALE)
  })

  it('lets a lease run out at its TTL, and writes its expiry', async () => {
    const leased = await worker.open('lease', { scope: 'svc-a' })
    w.clock.t += LEASE_TTL_SECONDS
    expect(await refusal(worker.call('renew', { lease: leased.lease }))).toBe(
      DENY_LEASE_EXPIRED,
    )
    w.clock.t += SWEEP_EVERY_SECONDS
    await worker.call('names')
    expect(
      w.audit.rows.some(
        (r) => r.kind === EV_EXPIRE && r.payload.lease === leased.lease,
      ),
    ).toBe(true)
  })

  it('renews up to RENEWALS_MAX times and refuses the next', async () => {
    const leased = await worker.open('lease', { scope: 'svc-a' })
    for (let i = 0; i < RENEWALS_MAX; i++) {
      w.clock.t += 10
      await worker.call('renew', { lease: leased.lease })
    }
    w.clock.t += 10
    expect(await refusal(worker.call('renew', { lease: leased.lease }))).toBe(
      DENY_RENEWALS_SPENT,
    )
  })

  it('lets only the holder renew, and the holder or the owner revoke', async () => {
    const leased = await worker.open('lease', { scope: 'svc-a' })
    await owner.call('grant', {
      subject: w.owner.publicHex,
      tier: TIER_OWNER,
      scopes: ['svc-a'],
    })
    expect(await refusal(owner.call('renew', { lease: leased.lease }))).toBe(
      DENY_NOT_HOLDER,
    )
    await owner.call('revoke', { lease: leased.lease })
    expect(await refusal(worker.call('renew', { lease: leased.lease }))).toBe(
      DENY_LEASE_REVOKED,
    )
  })

  it('ends a lease at its next renewal when its value is rotated or its binding moves', async () => {
    const before = await worker.open('lease', { scope: 'svc-a' })
    const r = await owner.recipients()
    await owner.call('rotate', {
      id: 'SVC_API_KEY',
      ct: await sealValue(new TextEncoder().encode(SVC2), r),
      input: 'stdin',
    })
    expect(await refusal(worker.call('renew', { lease: before.lease }))).toBe(
      DENY_LEASE_ROTATED,
    )
    const after = await worker.open('lease', { scope: 'svc-a' })
    expect(after.env.SVC_API_KEY).toBe(SVC2)
    // a binding moved to another secret ends the scope's leases too
    await owner.call('put', {
      id: 'OTHER_KEY',
      class: 'provider-api-key',
      ct: await sealValue(new TextEncoder().encode(dummy('other')), r),
      input: 'stdin',
    })
    await owner.call('bind', {
      scope: 'svc-a',
      env: 'SVC_API_KEY',
      id: 'OTHER_KEY',
      mode: 'replace',
    })
    expect(await refusal(worker.call('renew', { lease: after.lease }))).toBe(
      DENY_LEASE_ROTATED,
    )
    await owner.call('bind', {
      scope: 'svc-a',
      env: 'SVC_API_KEY',
      id: 'SVC_API_KEY',
      mode: 'replace',
    })
    expect(
      await refusal(
        owner.call('bind', {
          scope: 'svc-a',
          env: 'SVC_API_KEY',
          id: 'OTHER_KEY',
          mode: 'add',
        }),
      ),
    ).toBe(DENY_EXISTS)
  })

  it('keeps every owner act from a workload key', async () => {
    for (const op of [
      'put',
      'rotate',
      'import',
      'export',
      'grant',
      'audit',
      'dedup',
      'merge',
      'class',
      'bind',
      'break-glass',
    ])
      expect(await refusal(worker.call(op, {}))).toBe(DENY_NOT_OWNER)
  })

  it('keeps only rows the vault can open, sent to exactly two recipients', async () => {
    const r = await owner.recipients()
    const one = Buffer.from(
      await encrypt(new TextEncoder().encode(dummy('one')), [r.vault]),
    ).toString('base64')
    expect(
      await refusal(
        owner.call('put', { id: 'ONE_RECIPIENT', ct: one, input: 'stdin' }),
      ),
    ).toBe(DENY_BAD_CIPHERTEXT)
    const stranger = await keygenToFile(
      join(w.root, `stranger-${randomBytes(3).toString('hex')}.txt`),
    )
    const notVault = Buffer.from(
      await encrypt(new TextEncoder().encode(dummy('not-vault')), [
        stranger,
        r.recovery,
      ]),
    ).toString('base64')
    expect(
      await refusal(
        owner.call('put', {
          id: 'NOT_FOR_VAULT',
          ct: notVault,
          input: 'stdin',
        }),
      ),
    ).toBe(DENY_BAD_CIPHERTEXT)
    const ok = await sealValue(new TextEncoder().encode(dummy('x')), r)
    expect(
      await refusal(
        owner.call('put', { id: 'SVC_API_KEY', ct: ok, input: 'stdin' }),
      ),
    ).toBe(DENY_EXISTS)
    expect(
      await refusal(
        owner.call('rotate', { id: 'NO_SUCH', ct: ok, input: 'stdin' }),
      ),
    ).toBe(DENY_MISSING)
    expect(
      await refusal(owner.call('put', { id: '2FA', ct: ok, input: 'stdin' })),
    ).toBe(DENY_BAD_NAME)
  })

  it('refuses a lease whose event cannot be written, and delivers nothing', async () => {
    w.audit.fail = true
    try {
      const code = await refusal(worker.open('lease', { scope: 'svc-a' }))
      expect(code).toBe(DENY_NO_AUDIT)
    } finally {
      w.audit.fail = false
    }
  })

  it('breaks the glass for the owner only, with a reason, and makes the name due at once', async () => {
    const reason = 'incident: the bot is down and the lease path is broken'
    const opened = await owner.open('break-glass', {
      ids: ['SVC_API_KEY'],
      reason,
    })
    expect(opened.env.SVC_API_KEY).toBe(SVC2)
    const ev = w.audit.rows.find((r) => r.kind === EV_BREAK_GLASS) as AuditRow
    expect(ev.payload.reason_len).toBe(Buffer.byteLength(reason))
    expect(ev.payload.reason_sha).toBe(sha256Hex(reason).slice(0, 16))
    expect(JSON.stringify(ev)).not.toContain('incident')
    expect(
      await refusal(
        owner.open('break-glass', { ids: ['SVC_API_KEY'], reason: 'short' }),
      ),
    ).toBe(DENY_NO_REASON)
    const nine = Array.from({ length: 9 }, (_, i) => `K${i}`)
    expect(
      await refusal(owner.open('break-glass', { ids: nine, reason })),
    ).toBe(DENY_TOO_MANY)
    w.clock.t += SWEEP_EVERY_SECONDS
    await owner.call('names')
    const due = w.audit.rows
      .filter((r) => r.kind === EV_ROTATION_DUE)
      .map((r) => r.payload.ids)
    expect(due).toContainEqual(['SVC_API_KEY'])
  })

  it('raises a rotation alarm when a class is past its age', async () => {
    const before = w.audit.rows.length
    w.clock.t += 181 * 86_400
    await owner.call('names')
    const due = w.audit.rows
      .slice(before)
      .filter((r) => r.kind === EV_ROTATION_DUE)
    expect(due.map((r) => r.payload.ids)).toContainEqual(['SVC_DB_URL'])
    expect(due.every((r) => typeof r.payload.age_days === 'number')).toBe(true)
  })
})

// --- 4. the owner's migration -------------------------------------------------------------

describe("the owner's migration: two sources in, duplicates out by name, export, restore", () => {
  let w: World
  let owner: VaultClient
  const A = dummy('openai')
  const B1 = dummy('bot-token-infisical')
  const B2 = dummy('bot-token-railway')
  const D = dummy('shared-db')
  const E = dummy('bot-b-db')
  const S = dummy('sentry')
  const Q = dummy('quoted')
  const P1 = pemDummy()
  const P2 = pemDummy()

  const INFISICAL = [
    '# infisical export --env=prod --format=dotenv',
    `OPENAI_API_KEY='${A}'`,
    `BOT_TOKEN='${B1}'`,
    `export SHARED_DB_URL="${D}"`,
    `SENTRY_DSN='${S}'`,
    `QUOTE_INSIDE='it's ${Q}'`,
    `TLS_KEY='${P1}'`,
    '',
  ].join('\n')
  const RAILWAY = [
    '### bot-a',
    `OPENAI_API_KEY=${A}`,
    `BOT_TOKEN=${B2}`,
    'RAILWAY_PUBLIC_DOMAIN=bot-a.up.example',
    `DATABASE_URL=${D}`,
    `SENTRY_DSN=${S}`,
    '',
    '### bot-b',
    `DATABASE_URL=${E}`,
    `SIGNING_PEM=${P2}`,
    `LLM_KEY=${A}`,
    'RAILWAY_SERVICE_NAME=bot-b',
    '',
  ].join('\n')

  const expected: Record<string, string> = {
    'infisical:prod/OPENAI_API_KEY': A,
    'infisical:prod/BOT_TOKEN': B1,
    'infisical:prod/SHARED_DB_URL': D,
    'infisical:prod/SENTRY_DSN': S,
    'infisical:prod/QUOTE_INSIDE': `it's ${Q}`,
    'infisical:prod/TLS_KEY': P1,
    'railway:999/bot-a/OPENAI_API_KEY': A,
    'railway:999/bot-a/BOT_TOKEN': B2,
    'railway:999/bot-a/DATABASE_URL': D,
    'railway:999/bot-a/SENTRY_DSN': S,
    'railway:999/bot-b/DATABASE_URL': E,
    'railway:999/bot-b/SIGNING_PEM': P2,
    'railway:999/bot-b/LLM_KEY': A,
  }

  let botA: { lease: string }

  beforeAll(async () => {
    w = await world()
    WORLDS.push(w)
    owner = w.client(w.owner.privatePem)
  })

  it('reads both formats as they are written', () => {
    const inf = parseKvStream(INFISICAL, FMT_DOTENV)
    expect(inf.bad).toEqual([])
    expect(inf.entries.map((e) => e.name)).toEqual([
      'OPENAI_API_KEY',
      'BOT_TOKEN',
      'SHARED_DB_URL',
      'SENTRY_DSN',
      'QUOTE_INSIDE',
      'TLS_KEY',
    ])
    const rw = parseKvStream(RAILWAY, FMT_RAILWAY_KV)
    expect(rw.bad).toEqual([])
    expect(rw.skipped).toEqual([
      'RAILWAY_PUBLIC_DOMAIN',
      'RAILWAY_SERVICE_NAME',
    ])
    const pem = rw.entries.find((e) => e.name === 'SIGNING_PEM')
    expect(new TextDecoder().decode(pem?.value)).toBe(P2)
    // a block left open swallows nothing silently: the stream is refused
    expect(
      parseKvStream('### s\nK=-----BEGIN X-----\nabc\n', FMT_RAILWAY_KV).bad
        .length,
    ).toBe(1)
    expect(parseKvStream('raw line first\n', FMT_RAILWAY_KV).bad).toEqual([1])
    expect(parseKvStream(`K='never closed\n`, FMT_DOTENV).bad).toEqual([1])
  })

  it('imports the Infisical stream and the Railway stream, each entry with its provenance', async () => {
    const r = await owner.recipients()
    const inf = await importBody(INFISICAL, 'dotenv', r, {
      source: 'infisical:prod',
      input: 'stdin',
    })
    const a1 = await owner.call('import', inf.body)
    expect(a1.count).toBe(6)
    const rw = await importBody(RAILWAY, 'railway-kv', r, {
      source: 'railway:999',
      input: 'stdin',
    })
    const a2 = await owner.call('import', rw.body)
    expect(a2.count).toBe(7)
    expect(a2.scopes).toEqual(['bot-a', 'bot-b'])
    expect(a2.skipped).toEqual([
      'RAILWAY_PUBLIC_DOMAIN',
      'RAILWAY_SERVICE_NAME',
    ])
    const ev = w.audit.rows.filter((x) => x.kind === EV_IMPORT)
    expect(ev.map((x) => x.payload.source)).toEqual([
      'infisical:prod',
      'railway:999',
    ])
  })

  it('refuses an import whole: a second copy of a source, or a name twice in one stream', async () => {
    const r = await owner.recipients()
    const again = await importBody(RAILWAY, 'railway-kv', r, {
      source: 'railway:999',
      input: 'stdin',
    })
    expect(await refusal(owner.call('import', again.body))).toBe(DENY_EXISTS)
    const twice = await importBody(
      `A_NEW=${dummy('a')}\nA_NEW=${dummy('b')}\n`,
      'dotenv',
      r,
      {
        source: 'other:one',
        input: 'stdin',
      },
    )
    expect(await refusal(owner.call('import', twice.body))).toBe(
      DENY_DUPLICATE_IN_STREAM,
    )
    const names = await owner.call('names')
    expect(JSON.stringify(names)).not.toContain('A_NEW')
  })

  it('leases each Railway service its old environment, value for value', async () => {
    await owner.call('grant', {
      subject: w.owner.publicHex,
      tier: TIER_OWNER,
      scopes: ['bot-a', 'bot-b'],
    })
    const a = await owner.open('lease', { scope: 'bot-a' })
    botA = a
    expect(a.env).toEqual({
      OPENAI_API_KEY: A,
      BOT_TOKEN: B2,
      DATABASE_URL: D,
      SENTRY_DSN: S,
    })
    const b = await owner.open('lease', { scope: 'bot-b' })
    expect(b.env).toEqual({ DATABASE_URL: E, SIGNING_PEM: P2, LLM_KEY: A })
  })

  it('finds copies and conflicts, and reports them by name only', async () => {
    const r = await owner.call('dedup')
    const report = r.report as {
      count: number
      groups: Array<{ kind: string; names: string[]; ids: string[] }>
      conflicts: Array<{
        kind: string
        name: string
        ids: string[]
        sources: string[]
        count: number
      }>
    }
    expect(report.count).toBe(13)
    const byIds = (ids: string[]) =>
      report.groups.find(
        (g) => [...g.ids].sort().join() === [...ids].sort().join(),
      )
    // one value under two names and three places
    expect(
      byIds([
        'infisical:prod/OPENAI_API_KEY',
        'railway:999/bot-a/OPENAI_API_KEY',
        'railway:999/bot-b/LLM_KEY',
      ])?.kind,
    ).toBe('same-value')
    // one value under one name in both sources
    expect(
      byIds(['infisical:prod/SENTRY_DSN', 'railway:999/bot-a/SENTRY_DSN'])
        ?.kind,
    ).toBe('same-name')
    expect(
      byIds(['infisical:prod/SHARED_DB_URL', 'railway:999/bot-a/DATABASE_URL'])
        ?.kind,
    ).toBe('same-value')
    expect(report.groups.length).toBe(3)
    // one name, two values, two sources: a conflict; DATABASE_URL differs only between services of one source
    expect(report.conflicts.map((c) => c.name)).toEqual(['BOT_TOKEN'])
    expect(report.conflicts[0]?.sources.sort()).toEqual([
      'infisical:prod',
      'railway:999',
    ])
    expect(report.conflicts[0]?.count).toBe(2)
    const keys = new Set<string>()
    const walk = (v: unknown) => {
      if (Array.isArray(v)) v.forEach(walk)
      else if (v && typeof v === 'object')
        for (const [k, x] of Object.entries(v)) {
          keys.add(k)
          walk(x)
        }
    }
    walk(report)
    for (const k of keys)
      expect(REPORT_FIELDS as readonly string[]).toContain(k)
  })

  it('applies a plan whole or not at all: a conflict needs the owner to name the keeper', async () => {
    const bad = parsePlan(
      [
        '# a plan',
        'merge OPENAI_API_KEY = infisical:prod/OPENAI_API_KEY railway:999/bot-a/OPENAI_API_KEY railway:999/bot-b/LLM_KEY class=provider-api-key',
        'merge BOT_TOKEN = infisical:prod/BOT_TOKEN railway:999/bot-a/BOT_TOKEN',
      ].join('\n'),
    )
    expect(bad.bad).toEqual([])
    expect(await refusal(owner.call('merge', { lines: bad.lines }))).toBe(
      DENY_MERGE_CONFLICT,
    )
    expect(
      await refusal(
        owner.call('class', { ids: ['OPENAI_API_KEY'], class: 'oauth' }),
      ),
    ).toBe(DENY_MISSING)

    const good = parsePlan(
      [
        'merge OPENAI_API_KEY = infisical:prod/OPENAI_API_KEY railway:999/bot-a/OPENAI_API_KEY railway:999/bot-b/LLM_KEY class=provider-api-key',
        'merge BOT_TOKEN = infisical:prod/BOT_TOKEN railway:999/bot-a/BOT_TOKEN keep=infisical:prod/BOT_TOKEN class=bot-token',
      ].join('\n'),
    )
    const r = await owner.call('merge', { lines: good.lines })
    expect((r.merged as unknown[]).length).toBe(2)
    const ev = w.audit.rows.filter((x) => x.kind === EV_MERGE)
    expect(ev.map((x) => x.payload.canonical)).toEqual([
      'OPENAI_API_KEY',
      'BOT_TOKEN',
    ])
    expect(ev[1]?.payload.discarded).toEqual(['railway:999/bot-a/BOT_TOKEN'])
    // the lease that held the dropped bot-a value is refused its renewal
    expect(await refusal(owner.call('renew', { lease: botA.lease }))).toBe(
      DENY_LEASE_ROTATED,
    )
    // every service keeps its old names, now through aliases
    const a = await owner.open('lease', { scope: 'bot-a' })
    expect(a.env).toEqual({
      OPENAI_API_KEY: A,
      BOT_TOKEN: B1,
      DATABASE_URL: D,
      SENTRY_DSN: S,
    })
    const b = await owner.open('lease', { scope: 'bot-b' })
    expect(b.env.LLM_KEY).toBe(A)
    // an alias is not rotated or merged again by its old id
    const rr = await owner.recipients()
    expect(
      await refusal(
        owner.call('rotate', {
          id: 'railway:999/bot-b/LLM_KEY',
          ct: await sealValue(new TextEncoder().encode(dummy('z')), rr),
          input: 'stdin',
        }),
      ),
    ).toBe(DENY_ALIAS)
    const after = (await owner.call('dedup')).report as {
      groups: unknown[]
      conflicts: unknown[]
    }
    expect(after.conflicts).toEqual([])
    expect(after.groups.length).toBe(2)
  })

  let exported: Uint8Array

  it('exports to the recovery recipient only: it opens with the recovery identity and with nothing else', async () => {
    const r = await owner.call('export')
    exported = new Uint8Array(Buffer.from(String(r.ct), 'base64'))
    const plain = new TextDecoder().decode(
      await decryptWithFile(exported, w.recoveryFile),
    )
    const dump = parseVaultDump(plain)
    expect(dump.bad).toEqual([])
    const values = Object.fromEntries(
      dump.secrets.map((s) => [s.id, new TextDecoder().decode(s.value)]),
    )
    expect(values.OPENAI_API_KEY).toBe(A)
    expect(values.BOT_TOKEN).toBe(B1)
    expect(values['railway:999/bot-b/SIGNING_PEM']).toBe(P2)
    expect(values['infisical:prod/TLS_KEY']).toBe(P1)
    expect(values['infisical:prod/QUOTE_INSIDE']).toBe(`it's ${Q}`)
    // three ids now point at OPENAI_API_KEY and two at BOT_TOKEN
    expect(dump.aliases.length).toBe(5)
    await expect(
      decryptWithFile(exported, join(w.dir, 'identity')),
    ).rejects.toThrow()
    const stranger = join(
      w.root,
      `stranger-${randomBytes(3).toString('hex')}.txt`,
    )
    await keygenToFile(stranger)
    await expect(decryptWithFile(exported, stranger)).rejects.toThrow()
  })

  it('restores the export into a new vault, with its aliases and bindings', async () => {
    const plain = new TextDecoder().decode(
      await decryptWithFile(exported, w.recoveryFile),
    )
    const w2 = await world(w.root)
    WORLDS.push(w2)
    const o2 = w2.client(w2.owner.privatePem)
    const body = await importBody(plain, 'vault', await o2.recipients(), {
      source: 'restore',
      input: 'stdin',
    })
    expect(body.bad).toEqual([])
    const r = await o2.call('import', body.body)
    // thirteen imported, five of them merged into two
    expect(r.count).toBe(10)
    await o2.call('grant', {
      subject: w2.owner.publicHex,
      tier: TIER_OWNER,
      scopes: ['bot-a', 'bot-b'],
    })
    const a = await o2.open('lease', { scope: 'bot-a' })
    expect(a.env).toEqual({
      OPENAI_API_KEY: A,
      BOT_TOKEN: B1,
      DATABASE_URL: D,
      SENTRY_DSN: S,
    })
    const b = await o2.open('lease', { scope: 'bot-b' })
    expect(b.env).toEqual({ DATABASE_URL: E, SIGNING_PEM: P2, LLM_KEY: A })
    const dd = (await o2.call('dedup')).report as {
      groups: Array<{ ids: string[] }>
    }
    expect(dd.groups.length).toBe(2)
  })

  it('round-trips every imported value, both formats', async () => {
    const plain = new TextDecoder().decode(
      await decryptWithFile(exported, w.recoveryFile),
    )
    const dump = parseVaultDump(plain)
    const values = Object.fromEntries(
      dump.secrets.map((s) => [s.id, new TextDecoder().decode(s.value)]),
    )
    const alias = Object.fromEntries(
      dump.aliases.map((x) => [x.id, x.canonical]),
    )
    for (const [id, v] of Object.entries(expected)) {
      const held = values[id] ?? values[alias[id] as string]
      if (id === 'railway:999/bot-a/BOT_TOKEN') expect(held).toBe(B1)
      else expect(held).toBe(v)
    }
  })
})

// --- 5. run -------------------------------------------------------------------------------------

describe('run: the values go to the child, and the child stops with its lease', () => {
  let w: World
  const work = generateKey()
  const V = dummy('run-key')

  beforeAll(async () => {
    w = await world()
    WORLDS.push(w)
    const owner = w.client(w.owner.privatePem)
    await owner.call('put', {
      id: 'RUN_KEY',
      class: 'provider-api-key',
      scopes: ['runner'],
      ct: await sealValue(
        new TextEncoder().encode(V),
        await owner.recipients(),
      ),
      input: 'stdin',
    })
    await owner.call('grant', {
      subject: work.publicHex,
      tier: TIER_TRUSTED,
      scopes: ['runner'],
    })
  })

  it('starts the child with the leased environment and revokes the lease after', async () => {
    const out = join(w.root, 'child.txt')
    const script = `require('fs').writeFileSync(${JSON.stringify(out)}, require('crypto').createHash('sha256').update(process.env.RUN_KEY ?? '').digest('hex'))`
    const r = await runScoped(
      w.client(work.privatePem),
      { scope: 'runner' },
      [process.execPath, '-e', script],
      {
        stdio: 'ignore',
      },
    )
    expect(r.exitCode).toBe(0)
    expect(r.stopped).toBe(false)
    expect(await readFile(out, 'utf8')).toBe(sha256Hex(V))
    expect(w.audit.rows.some((x) => x.name === 'vault.revoke')).toBe(true)
  })

  it('stops the child when a renewal is refused', async () => {
    const c = w.client(work.privatePem)
    const owner = w.client(w.owner.privatePem)
    const running = runScoped(
      c,
      { scope: 'runner' },
      ['sleep', '30'],
      { stdio: 'ignore', tickMs: 20 },
      () => w.clock.t,
    )
    await Bun.sleep(300)
    await owner.call('rotate', {
      id: 'RUN_KEY',
      ct: await sealValue(
        new TextEncoder().encode(dummy('run-key-2')),
        await owner.recipients(),
      ),
      input: 'stdin',
    })
    w.clock.t += LEASE_TTL_SECONDS / 2
    const r = await running
    expect(r.stopped).toBe(true)
  })
})

// --- 6. the CLI ------------------------------------------------------------------------------------

describe('trios-vault, as the owner types it', () => {
  const CLI = join(import.meta.dir, '../../src/vault-cli.ts')
  let home: string
  let env: Record<string, string>
  const V1 = dummy('cli-infisical')
  const V2 = dummy('cli-railway')

  const cli = async (args: string[], stdin?: string) => {
    const p = Bun.spawn([process.execPath, CLI, ...args], {
      env,
      stdin: stdin === undefined ? 'ignore' : new TextEncoder().encode(stdin),
      stdout: 'pipe',
      stderr: 'pipe',
    })
    const [out, err, code] = await Promise.all([
      new Response(p.stdout).text(),
      new Response(p.stderr).text(),
      p.exited,
    ])
    SAID.push(out, err)
    return { out, err, code }
  }

  beforeAll(async () => {
    home = await mkdtemp(join(tmpdir(), 'vault-cli-'))
    env = {
      PATH: process.env.PATH ?? '',
      HOME: home,
      TRIOS_VAULT_DIR: join(home, 'vault'),
      TRIOS_VAULT_KEY: join(home, 'owner.key'),
    }
  })
  afterAll(async () => {
    for (const f of ['vault/store.json', 'vault/audit.jsonl'])
      SAID.push(await readFile(join(home, f), 'utf8').catch(() => ''))
    await rm(home, { recursive: true, force: true })
  })

  it('init, import both formats, dedup, apply, names, export, run', async () => {
    const rec = await keygenToFile(join(home, 'recovery.txt'))
    expect((await cli(['init', '--recovery', rec])).code).toBe(0)
    const inf = await cli(
      ['import', '--stdin', '--source', 'infisical:prod', '--format', 'dotenv'],
      `SHARED='${V1}'\nONLY_INF='${dummy('only')}'\n`,
    )
    expect(inf.code).toBe(0)
    expect(inf.out).toContain('imported 2 secrets')
    const rw = await cli(
      [
        'import',
        '--stdin',
        '--source',
        'railway:999',
        '--format',
        'railway-kv',
      ],
      `### svc\nSHARED=${V1}\nOWN=${V2}\nRAILWAY_PRIVATE_DOMAIN=x\n`,
    )
    expect(rw.code).toBe(0)
    expect(rw.out).toContain(
      'left out 1 platform names: RAILWAY_PRIVATE_DOMAIN',
    )
    const plan = join(home, 'plan.txt')
    const dd = await cli(['dedup', '--plan-out', plan])
    expect(dd.code).toBe(0)
    expect(dd.out).toContain('same-name: names SHARED')
    const template = await readFile(plan, 'utf8')
    expect(template).toContain(
      '# merge <CANONICAL> = infisical:prod/SHARED railway:999/svc/SHARED',
    )
    await writeFile(
      plan,
      template.replace('# merge <CANONICAL>', 'merge SHARED'),
    )
    const ap = await cli(['dedup', '--apply', plan])
    expect(ap.code).toBe(0)
    expect(ap.out).toContain('merged into SHARED')
    // a value is never an argument, and a pipe needs --stdin
    const put = await cli(['put', 'NEW_KEY'], 'piped')
    expect(put.code).toBe(64)
    expect(put.err).toContain('--stdin')
    expect(
      (
        await cli(
          [
            'put',
            'NEW_KEY',
            '--stdin',
            '--scope',
            'svc',
            '--class',
            'bot-token',
          ],
          dummy('put'),
        )
      ).code,
    ).toBe(0)
    const pub = (await cli(['key'])).out.match(
      /public key: ([0-9a-f]{64})/,
    )?.[1] as string
    expect(
      (await cli(['grant', '--key', pub, '--tier', 'owner', '--scope', 'svc']))
        .code,
    ).toBe(0)
    const names = await cli(['names'])
    expect(names.out).toContain('svc: SHARED, OWN, NEW_KEY')
    const ex = await cli(['export', '--out', join(home, 'export.age')])
    expect(ex.code).toBe(0)
    const dump = parseVaultDump(
      new TextDecoder().decode(
        await decryptWithFile(
          await readFile(join(home, 'export.age')),
          join(home, 'recovery.txt'),
        ),
      ),
    )
    expect(dump.secrets.length).toBe(4)
    // the child compares its environment with a file the test wrote, and says only true or false
    const out = join(home, 'run.txt')
    await writeFile(join(home, 'want.txt'), V1)
    const run2 = await cli([
      'run',
      '--scope',
      'svc',
      '--',
      process.execPath,
      '-e',
      `require('fs').writeFileSync(${JSON.stringify(out)}, String(process.env.SHARED === require('fs').readFileSync(${JSON.stringify(join(home, 'want.txt'))}, 'utf8')))`,
    ])
    expect(run2.code).toBe(0)
    expect(await readFile(out, 'utf8')).toBe('true')
    await rm(join(home, 'want.txt'))
    const audit = await cli(['audit', '--limit', '500'])
    expect(audit.code).toBe(0)
    expect(audit.out).toContain('vault.import')
  })

  it("passes the runbook's smoke check against a vault served over HTTP", async () => {
    const port = 20_000 + Math.floor(Math.random() * 20_000)
    const server = Bun.spawn(
      [process.execPath, CLI, 'serve', '--port', String(port)],
      {
        env,
        stdout: 'pipe',
        stderr: 'pipe',
      },
    )
    try {
      const url = `http://127.0.0.1:${port}/vault`
      for (let i = 0; i < 100; i++) {
        const up = await fetch(`${url}/names`, { method: 'POST' }).catch(
          () => null,
        )
        if (up) break
        await Bun.sleep(100)
      }
      const remote = { ...env, TRIOS_VAULT_URL: url }
      const put = Bun.spawn(
        [
          process.execPath,
          CLI,
          'put',
          'SMOKE_SECRET',
          '--stdin',
          '--scope',
          'smoke',
          '--class',
          'provider-api-key',
        ],
        {
          env: remote,
          stdin: new TextEncoder().encode(dummy('smoke')),
          stdout: 'pipe',
          stderr: 'pipe',
        },
      )
      SAID.push(
        await new Response(put.stdout).text(),
        await new Response(put.stderr).text(),
      )
      expect(await put.exited).toBe(0)
      const pub = (await cli(['key'])).out.match(
        /public key: ([0-9a-f]{64})/,
      )?.[1] as string
      const g = Bun.spawn(
        [
          process.execPath,
          CLI,
          'grant',
          '--key',
          pub,
          '--tier',
          'owner',
          '--scope',
          'svc,smoke',
        ],
        {
          env: remote,
          stdout: 'pipe',
          stderr: 'pipe',
        },
      )
      expect(await g.exited).toBe(0)
      const run = Bun.spawn(
        [
          process.execPath,
          CLI,
          'run',
          '--scope',
          'smoke',
          '--',
          'sh',
          '-c',
          'test -n "$SMOKE_SECRET" && echo present',
        ],
        { env: remote, stdout: 'pipe', stderr: 'pipe' },
      )
      const [out, err] = [
        await new Response(run.stdout).text(),
        await new Response(run.stderr).text(),
      ]
      SAID.push(out, err)
      expect(await run.exited).toBe(0)
      expect(out.trim()).toBe('present')
    } finally {
      server.kill()
      SAID.push(
        await new Response(server.stdout).text(),
        await new Response(server.stderr).text(),
      )
    }
  })
})

// --- 7. the leak scan --------------------------------------------------------------------------------

describe('THE LEAK SCAN', () => {
  it('finds no dummy secret in any log, event, response, report, store, audit file or CLI output', async () => {
    for (const w of WORLDS) {
      SAID.push(JSON.stringify(w.audit.rows))
      SAID.push(await readFile(join(w.dir, STORE_FILE), 'utf8'))
    }
    const haystack = SAID.join('\n')
    expect(haystack.length).toBeGreaterThan(10_000)
    expect(SECRETS.length).toBeGreaterThan(20)
    const leaks: string[] = []
    for (const s of SECRETS) {
      const bytes = Buffer.from(s)
      const needles = [
        s,
        bytes.toString('base64'),
        bytes.toString('base64url'),
        bytes.toString('hex'),
        sha256Hex(s),
        sha256Hex(s).slice(0, 16),
      ]
      for (const n of needles)
        if (haystack.includes(n))
          leaks.push(
            `${s.slice(0, 18)}... as ${n === s ? 'text' : 'an encoding or hash'}`,
          )
    }
    expect(leaks).toEqual([])
    // and every event is one of audit.t27's kinds
    for (const w of WORLDS)
      for (const r of w.audit.rows)
        expect(EVENT_NAMES as readonly string[]).toContain(r.name)
  })
})

afterAll(async () => {
  for (const w of WORLDS)
    await rm(w.root, { recursive: true, force: true }).catch(() => undefined)
})
