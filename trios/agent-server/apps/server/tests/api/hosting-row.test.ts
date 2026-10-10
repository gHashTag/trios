/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * SELF-HOSTING, SLICE 1b, LANE A (gHashTag/trios#1762, part of #1761), as
 * tests:
 *   - the lab's row in two halves: the t27b half's output, the row assembled
 *     from both, byte for byte the t27b lab's own rows of master cf42584c9;
 *   - placement: a t27b half never reaches an x64 host, and a host that
 *     cannot confine a job gets no public job; a row is served only once both
 *     halves are agreed;
 *   - the download: a flipped byte is refused by name, and a kept file that
 *     changed on disk is fetched again;
 *   - the sandbox: the planted job (a socket, a file in $HOME) fails inside it
 *     and succeeds outside it, which is the negative control;
 *   - off means off.
 * The last block runs the real t27c and t27b, in the sandbox, on two hosts.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import {
  createHostingRoute,
  hostingEnabled,
} from '../../src/api/routes/hosting'
import {
  createHostAgent,
  createT27cRunner,
  type LeasedJob,
  type RunShard,
} from '../../src/api/services/hosting-agent'
import * as cards from '../../src/api/services/hosting-cards'
import { createHostingQueen } from '../../src/api/services/hosting-queen'
import {
  detectSandbox,
  probeIsolation,
} from '../../src/api/services/hosting-sandbox'
import { createMemoryHostingStore } from '../../src/api/services/hosting-store'
import {
  archOf,
  fetchPinned,
  resolveT27b,
  resolveT27c,
  useClosure,
} from '../../src/api/services/hosting-toolchain'
import {
  assembleRow,
  generateHostKey,
  keyIdOf,
  messageOf,
  normalizeShard,
  normalizeT27b,
  readNormalized,
  sha256Hex,
  signHex,
} from '../../src/api/services/hosting-wire'
import {
  ISO_JOBDIR,
  ISO_NO_NETWORK,
  ISO_NONE,
  SANDBOX_BWRAP,
  SANDBOX_EXEC,
  SANDBOX_NONE,
  SANDBOX_UNSHARE,
  TIER_OWNER,
  TIER_PUBLIC,
  TIER_TRUSTED,
  WC_SHARD,
} from '../../src/api/services/queen-hosting-host-card.gen'
import {
  REGISTER_DOMAIN,
  REGISTER_SIGNED_FIELDS,
} from '../../src/api/services/queen-hosting-proof-card.gen'
import {
  DIS_BOTH,
  DIS_REFERENCE_ONLY,
  DIS_T27B_ONLY,
  HALF_REFERENCE,
  HALF_T27B,
  TW_BLOCKED,
  TW_TIMEOUT,
} from '../../src/api/services/queen-hosting-row-card.gen'
import {
  ARCH_ARM64,
  ARCH_X64,
  PLATFORMS,
  T27B_SHA256,
  T27C_BYTES,
  T27C_SHA256,
  T27C_URLS,
} from '../../src/api/services/queen-hosting-toolchain.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const QUEEN = 'http://queen.test'

/** A value with its object keys sorted: the lab's JSON and ours may order keys differently. */
const canon = (v: unknown): unknown =>
  Array.isArray(v)
    ? v.map(canon)
    : v && typeof v === 'object'
      ? Object.fromEntries(
          Object.keys(v as object)
            .sort()
            .map((k) => [k, canon((v as Record<string, unknown>)[k])]),
        )
      : v

// --- fixtures: the t27b lab's run of master cf42584c9, four rows as published --

const LAB_ROWS: Record<string, Record<string, unknown>> = {
  'specs/account/auth.t27': {
    file: 'specs/account/auth.t27',
    reference: 'blocked',
    reference_detail:
      "does not compile: spec.zig:23:16: error: use of undeclared identifier 'Ok'",
    t27b: 'blocked',
    tests: 0,
    invariants: 0,
    asserts: null,
    blockers: [
      'ExprFieldAccess',
      'ExprCall(Result/Option constructor)',
      'type lexer::Lexer',
      'ExprCall(undeclared fn)',
      'ExprCast',
      'ExprCall(method)',
      'ExprStructLit',
    ],
    detail: 'ExprFieldAccess',
  },
  'specs/base/ternary_encoding.t27': {
    file: 'specs/base/ternary_encoding.t27',
    reference: 'blocked',
    reference_detail: 'does not compile: spec.zig:8:9: error: assertion failed',
    t27b: 'fail',
    tests: 0,
    invariants: 0,
    asserts: null,
    blockers: [],
    detail: 'FAIL bits_to_trits_max_nibble: assert failed at line 266 (assert)',
    test_verdicts: {
      bit_to_trit_pair_zero: true,
      bit_to_trit_pair_one: true,
      bits_to_trits_zero: true,
      bits_to_trits_max_nibble: false,
      trits_to_bits_roundtrip: false,
      byte_to_trits_roundtrip: false,
      balanced_unipolar_conversion: true,
      is_valid_trit_check: true,
      is_valid_unipolar_trit_check: true,
      char_encoding_roundtrip: false,
    },
  },
  'specs/isa/ternary_shift.t27': {
    file: 'specs/isa/ternary_shift.t27',
    reference: 'fail',
    reference_detail: '2 of 9 tests fail',
    t27b: 'blocked',
    tests: 0,
    invariants: 0,
    asserts: null,
    blockers: ['InvariantBlock'],
    detail: 'InvariantBlock',
    reference_tests: {
      extract_trits_basic: true,
      insert_trits_basic: true,
      ternary_arithmetic_shift_right_negative: false,
      ternary_arithmetic_shift_right_positive: true,
      ternary_rotate_full_circle: true,
      ternary_rotate_left_basic: true,
      ternary_rotate_right_basic: true,
      ternary_shift_left_basic: true,
      ternary_shift_right_basic: false,
    },
  },
  'specs/lsp/language.t27': {
    file: 'specs/lsp/language.t27',
    reference: 'pass',
    reference_detail: '',
    t27b: 'pass_vacuous',
    tests: 0,
    invariants: 0,
    asserts: 0,
    blockers: [],
    detail: '',
    test_verdicts: {},
    reference_tests: {},
    reference_disagree: [],
  },
}

/** t27c's report for a spec, as the reference half would read it. */
function referenceReport(row: Record<string, unknown>): string {
  if (row.reference === 'blocked')
    return `--- test report: ${row.file} ---\n  BLOCKED  ${row.reference_detail}\n\n  A blocked spec is not a failing one.\n`
  const tests = (row.reference_tests ?? {}) as Record<string, boolean>
  const names = Object.keys(tests)
  const fail = names.filter((n) => !tests[n]).length
  return [
    `--- test report: ${row.file} ---`,
    ...names.map((n) => `  ${tests[n] ? 'pass' : 'FAIL'}  ${n}`),
    '',
    `  tests       ${names.length}`,
    `  pass        ${names.length - fail}`,
    `  FAIL        ${fail}`,
    '',
  ].join('\n')
}

/** `t27b corpus --json` for a spec, as the t27b half would read it. */
function t27bJson(row: Record<string, unknown>): string {
  const {
    reference: _r,
    reference_detail: _d,
    reference_tests: _t,
    reference_disagree: _x,
    ...t27b
  } = row
  return JSON.stringify({
    dir: 'specs',
    files: 1,
    results: [{ ...t27b, reference: 'skip', reference_detail: '' }],
  })
}

const ran = (stdout: string) => ({
  started: true,
  timedOut: false,
  exitCode: 0,
  stdout,
  stderr: '',
})

// --- the cards ----------------------------------------------------------------

describe('the lane A cards', () => {
  it('answer as row.t27, placement.t27 and host.t27 say', () => {
    expect(cards.rowComplete(1, 1)).toBe(true)
    expect(cards.rowComplete(1, 0)).toBe(false)
    expect(cards.t27bVotes(TW_TIMEOUT)).toBe(false)
    expect(cards.t27bVotes(TW_BLOCKED)).toBe(true)
    expect(cards.halfVote(0, HALF_T27B, 3)).toBe(true)
    expect(cards.halfVote(0, HALF_REFERENCE, 3)).toBe(false)
    expect(cards.halfFits(HALF_T27B, ARCH_X64)).toBe(false)
    expect(cards.halfFits(HALF_T27B, ARCH_ARM64)).toBe(true)
    expect(cards.isolationRequired(WC_SHARD, false, false)).toBe(ISO_JOBDIR)
    expect(cards.isolationMeets(ISO_NO_NETWORK, ISO_JOBDIR)).toBe(false)
    expect(cards.isolationOf(SANDBOX_UNSHARE, true, true)).toBe(ISO_NO_NETWORK)
    expect(cards.isolationOf(SANDBOX_EXEC, true, false)).toBe(ISO_NO_NETWORK)
    expect(cards.isolationOf(SANDBOX_BWRAP, true, true)).toBe(ISO_JOBDIR)
    expect(
      cards.sandboxChoice({
        macos: false,
        hasSandboxExec: false,
        hasBwrap: false,
        hasUnshare: true,
        turnedOff: false,
      }),
    ).toBe(SANDBOX_UNSHARE)
    expect(cards.disagreement(true, true, true, false)).toBe(DIS_BOTH)
    expect(cards.disagreement(true, true, false, false)).toBe(DIS_T27B_ONLY)
    expect(cards.disagreement(false, false, true, true)).toBe(
      DIS_REFERENCE_ONLY,
    )
    expect(archOf('linux-x64')).toBe(ARCH_X64)
    expect(archOf('linux-arm64')).toBe(ARCH_ARM64)
    expect(archOf('plan9-mips')).toBe(255)
  })
})

// --- the row ------------------------------------------------------------------

describe("the lab's row, from two halves", () => {
  for (const [file, lab] of Object.entries(LAB_ROWS))
    it(`equals the lab's row of ${file}`, () => {
      const ref = normalizeShard(file, ran(referenceReport(lab)))
      const t27b = normalizeT27b(file, { started: true, json: t27bJson(lab) })
      expect(ref.half).toBe(HALF_REFERENCE)
      expect(t27b.half).toBe(HALF_T27B)
      expect(t27b.word).toBe(lab.t27b as string)
      const a = readNormalized(ref.output)
      const b = readNormalized(t27b.output)
      expect(a && b).toBeTruthy()
      const row = assembleRow(file, a as never, b as never)
      expect(canon(row)).toEqual(canon(lab))
      // and in the lab's key order
      expect(Object.keys(row)).toEqual(Object.keys(lab))
    })

  it('names each test the two sides judge differently, in name order', () => {
    const file = 'specs/demo/split.t27'
    const ref = normalizeShard(
      file,
      ran(
        referenceReport({
          file,
          reference: 'fail',
          reference_tests: { a: true, b: false, c: true },
        }),
      ),
    )
    const t27b = normalizeT27b(file, {
      started: true,
      json: JSON.stringify({
        results: [
          {
            file,
            t27b: 'fail',
            tests: 3,
            invariants: 0,
            asserts: 4,
            blockers: [],
            detail: 'FAIL a',
            test_verdicts: { a: false, b: false, d: true },
          },
        ],
      }),
    })
    const row = assembleRow(
      file,
      readNormalized(ref.output) as never,
      readNormalized(t27b.output) as never,
    )
    expect(row.reference_disagree).toEqual([
      'a: t27b FAIL, reference pass',
      'c: t27b has no such test, reference pass',
      'd: t27b pass, reference has no such test',
    ])
  })

  it('reads a t27b that wrote no row, or a timeout, as no vote', () => {
    const none = normalizeT27b('specs/x.t27', { started: true, json: null })
    expect(none.word).toBe('host_error')
    expect(none.output).toBe(
      't27-hosting-t27b-v1\nspec=specs/x.t27\nt27b="host_error"\n',
    )
    const late = normalizeT27b('specs/x.t27', {
      started: true,
      json: JSON.stringify({
        results: [{ file: 'specs/x.t27', t27b: 'timeout', tests: 0 }],
      }),
    })
    expect(late.word).toBe('timeout')
    expect(cards.halfVote(0, HALF_T27B, late.verdict)).toBe(false)
  })
})

// --- the demo: two halves on an arm64 and an x64 host ------------------------

const COMMIT = 'c'.repeat(40)

function world(allow: Record<string, number> = {}) {
  const clock = new VirtualClock()
  const queen = createHostingQueen({
    store: createMemoryHostingStore(),
    now: clock.now,
    allowlist: new Map(Object.entries(allow)),
  })
  const app = new Hono().route(
    '/hosting',
    createHostingRoute({ enabled: () => true, queen: () => queen }),
  )
  const agent = (
    privatePem: string,
    ip: string,
    runShard: RunShard,
    o: { tier?: number; platform?: string; isolation?: number } = {},
  ) =>
    createHostAgent({
      queenUrl: QUEEN,
      fetch: (url, init) =>
        app.request(url.slice(QUEEN.length), {
          ...init,
          headers: {
            ...(init.headers as Record<string, string>),
            'x-forwarded-for': ip,
          },
        }),
      privatePem,
      tierClaim: o.tier ?? TIER_PUBLIC,
      slots: 2,
      platform: o.platform ?? 'darwin-arm64',
      isolation: o.isolation ?? ISO_JOBDIR,
      runShard,
      clock,
    })
  // biome-ignore lint/suspicious/noExplicitAny: the tests read the routes' JSON as it comes
  const get = async (path: string): Promise<any> =>
    (await app.request(path)).json()
  return { clock, queen, app, agent, get }
}

/** An honest host of either half: the lab's row of the spec, as t27c and t27b would report it. */
const honest: RunShard = async (j: LeasedJob) => {
  const lab = LAB_ROWS[j.spec] as Record<string, unknown>
  return j.half === HALF_T27B
    ? {
        result: normalizeT27b(j.spec, { started: true, json: t27bJson(lab) }),
        modelHash: sha256Hex('t27b'),
        zig: '',
      }
    : {
        result: normalizeShard(j.spec, ran(referenceReport(lab))),
        modelHash: sha256Hex('t27c'),
        zig: '0.16.0',
      }
}

const halfJob = (spec: string, half: number) => {
  const input = sha256Hex(spec)
  return {
    commit: COMMIT,
    spec,
    input_hash: input,
    files: [{ path: spec, sha256: input }],
    half,
    holds_secret: false,
    holds_personal: false,
  }
}

describe('the demo: a row in two halves', () => {
  it('sends the t27b half to arm64 hosts only, and serves the row once both halves agree', async () => {
    const owner = generateHostKey()
    const [ka, kb, kx] = [
      generateHostKey(),
      generateHostKey(),
      generateHostKey(),
    ]
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, {
      tier: TIER_OWNER,
    })
    const arm = w.agent(ka.privatePem, '198.51.100.20', honest, {
      platform: 'linux-arm64',
    })
    const arm2 = w.agent(kb.privatePem, '192.0.2.5', honest)
    const x64 = w.agent(kx.privatePem, '100.64.0.9', honest, {
      platform: 'linux-x64',
    })
    for (const a of [mac, arm, arm2, x64]) await a.register()
    const spec = 'specs/isa/ternary_shift.t27'
    const t = (await mac.submitJob(halfJob(spec, HALF_T27B))).json.job as {
      id: string
      half: number
    }
    expect(t.half).toBe(HALF_T27B)
    // the x64 host is offered nothing while only a t27b half is open
    expect(await x64.leaseOnce()).toBeNull()
    const r = (await mac.submitJob(halfJob(spec, HALF_REFERENCE))).json.job as {
      id: string
    }
    const xl = await x64.leaseOnce()
    expect(xl?.job.id).toBe(r.id)
    // the t27b half goes to two arm64 hosts
    const al = await arm.leaseOnce()
    const bl = await arm2.leaseOnce()
    expect(al?.job.id).toBe(t.id)
    expect(bl?.job.id).toBe(t.id)
    await arm.work(al as NonNullable<typeof al>)
    expect(await arm2.work(bl as NonNullable<typeof bl>)).toMatchObject({
      code: 'ok',
      accepted: true,
      job: { verdict: 'agreed' },
    })
    // one half agreed is no row
    expect(await w.get(`/hosting/verdicts?commit=${COMMIT}`)).toEqual([])
    await x64.work(xl as NonNullable<typeof xl>)
    const ml = await mac.leaseOnce()
    expect(ml?.job.id).toBe(r.id)
    expect(await mac.work(ml as NonNullable<typeof ml>)).toMatchObject({
      job: { verdict: 'agreed' },
    })
    const [row] = await w.get(`/hosting/verdicts?commit=${COMMIT}`)
    const { commit, input_sha256, halves, ...lab } = row
    expect(commit).toBe(COMMIT)
    expect(canon(lab)).toEqual(canon(LAB_ROWS[spec]))
    expect([...halves.t27b.hosts].sort()).toEqual(
      [keyIdOf(ka.publicHex), keyIdOf(kb.publicHex)].sort(),
    )
    expect([...halves.reference.hosts].sort()).toEqual(
      [keyIdOf(kx.publicHex), keyIdOf(owner.publicHex)].sort(),
    )
    // every host that agreed was credited, the x64 host for its half
    const ledger = await w.get('/hosting/ledger')
    for (const k of [ka, kb, kx, owner])
      expect(
        ledger.hosts.find(
          (h: { host: string }) => h.host === keyIdOf(k.publicHex),
        ).balance_mtri,
      ).toBe(1)
  })

  it('gives a host that cannot confine a job no public job, and still a named job', async () => {
    const owner = generateHostKey()
    const [bare, net, named] = [
      generateHostKey(),
      generateHostKey(),
      generateHostKey(),
    ]
    const w = world({
      [keyIdOf(owner.publicHex)]: TIER_OWNER,
      [keyIdOf(named.publicHex)]: TIER_TRUSTED,
    })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, {
      tier: TIER_OWNER,
      isolation: ISO_NONE,
    })
    const b = w.agent(bare.privatePem, '198.51.100.20', honest, {
      isolation: ISO_NONE,
    })
    const n = w.agent(net.privatePem, '192.0.2.5', honest, {
      isolation: ISO_NO_NETWORK,
    })
    const f = w.agent(named.privatePem, '100.64.0.9', honest, {
      tier: TIER_TRUSTED,
      isolation: ISO_NONE,
    })
    expect((await b.register()).isolation).toBe('none')
    expect((await n.register()).isolation).toBe('no-network')
    await mac.register()
    await f.register()
    await mac.submitJob(halfJob('specs/account/auth.t27', HALF_REFERENCE))
    // a public job: not on an unconfined host, not even the owner's
    expect(await b.leaseOnce()).toBeNull()
    expect(await n.leaseOnce()).toBeNull()
    expect(await mac.leaseOnce()).toBeNull()
    // a job that declares a secret is no public job: the named host takes it
    const secret = (
      await mac.submitJob({
        ...halfJob('specs/account/auth.t27', HALF_REFERENCE),
        holds_secret: true,
      })
    ).json.job as { id: string }
    expect((await f.leaseOnce())?.job.id).toBe(secret.id)
  })

  it('signs the isolation level into the registration', async () => {
    const k = generateHostKey()
    const w = world()
    const fields = {
      public_key: k.publicHex,
      tier_claim: TIER_PUBLIC,
      slots: 1,
      platform: 'darwin-arm64',
      isolation: ISO_NONE,
      utc_unix: Math.floor(w.clock.now() / 1000),
    }
    const signature = signHex(
      k.privatePem,
      messageOf(REGISTER_DOMAIN, REGISTER_SIGNED_FIELDS, fields),
    )
    const post = (body: unknown) =>
      w.app.request('/hosting/hosts', {
        method: 'POST',
        body: JSON.stringify(body),
      })
    // a level raised after signing does not verify
    expect(
      (await post({ ...fields, isolation: ISO_JOBDIR, signature })).status,
    ).toBe(401)
    expect((await post({ ...fields, signature })).status).toBe(200)
    const { isolation: _i, ...unsaid } = fields
    expect((await post({ ...unsaid, signature })).status).toBe(400)
  })
})

// --- the download -------------------------------------------------------------

describe('the pinned download', () => {
  const index = (PLATFORMS as readonly string[]).indexOf('darwin-arm64')
  const pin = T27C_SHA256[index] as string
  const bytes = T27C_BYTES[index] as number
  /** A body that hashes to the pin is the release's own file; here, a stand-in with the pin swapped in. */
  const served = (body: Uint8Array) =>
    (async () => new Response(body)) as unknown as typeof fetch

  it('refuses a corrupted asset by name, and keeps nothing', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'trios-host-dl-'))
    try {
      const body = new Uint8Array(bytes)
      body[1234] = 1
      const dest = join(dir, 't27c')
      const err = await fetchPinned(
        T27C_URLS[index] as string,
        pin,
        bytes,
        dest,
        served(body),
      ).catch((e: Error) => e)
      expect(String(err)).toContain('t27c-darwin-arm64 refused: sha256 ')
      expect(String(err)).toContain(`pinned ${pin}`)
      expect(existsSync(dest)).toBe(false)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it('keeps a file only with the pinned digest and size, and fetches a changed one again', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'trios-host-dl-'))
    try {
      // a real file of known bytes stands in for the asset: the pin is its digest
      const good = new TextEncoder().encode('the release asset\n')
      const goodPin = sha256Hex(good)
      const dest = join(dir, 'tool')
      await fetchPinned(
        'https://x/tool',
        goodPin,
        good.length,
        dest,
        served(good),
      )
      expect(readFileSync(dest, 'utf8')).toBe('the release asset\n')
      // one flipped byte, same size: refused
      const flipped = new Uint8Array(good)
      flipped[0] = (flipped[0] as number) ^ 1
      const err = await fetchPinned(
        'https://x/tool',
        goodPin,
        good.length,
        join(dir, 'tool2'),
        served(flipped),
      ).catch((e: Error) => e)
      expect(String(err)).toContain('tool refused')
      expect(existsSync(join(dir, 'tool2'))).toBe(false)
      // the right digest but another size: refused too (host.t27 download_kept)
      const err2 = await fetchPinned(
        'https://x/tool',
        goodPin,
        good.length + 1,
        join(dir, 'tool3'),
        served(good),
      ).catch((e: Error) => e)
      expect(String(err2)).toContain('refused')
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it('checks a kept t27c again on every start, and fetches it again when it changed', async () => {
    const home = mkdtempSync(join(tmpdir(), 'trios-host-home-'))
    try {
      let fetched = 0
      const asset = new Uint8Array(bytes)
      // a stand-in whose digest is not the pin: the first fetch is refused
      const fetcher = (async () => {
        fetched++
        return new Response(asset)
      }) as unknown as typeof fetch
      const err = await resolveT27c(home, undefined, index, fetcher).catch(
        (e: Error) => e,
      )
      expect(fetched).toBe(1)
      expect(String(err)).toContain('t27c-darwin-arm64 refused')
      expect(existsSync(join(home, 'tools', 't27c'))).toBe(false)
      // a t27c already in the tools directory that is not the pinned file is
      // deleted and fetched again, never run
      writeFileSync(join(home, 'tools', 't27c'), 'changed on disk')
      await resolveT27c(home, undefined, index, fetcher).catch(() => {})
      expect(fetched).toBe(2)
      expect(existsSync(join(home, 'tools', 't27c'))).toBe(false)
      // no t27b is pinned for linux-x64: that half is never placed there
      const x64 = (PLATFORMS as readonly string[]).indexOf('linux-x64')
      expect(T27B_SHA256[x64]).toBe('')
      expect(await resolveT27b(home, undefined, x64, fetcher)).toBeNull()
    } finally {
      rmSync(home, { recursive: true, force: true })
    }
  })
})

// --- off means off ------------------------------------------------------------

describe('with TRIOS_HOSTING unset', () => {
  it('reads as off, answers 503 everywhere, and never builds the Queen or its pool', async () => {
    expect(hostingEnabled({})).toBe(false)
    expect(hostingEnabled({ TRIOS_HOSTING: '' })).toBe(false)
    expect(hostingEnabled({ TRIOS_HOSTING: 'off' })).toBe(false)
    expect(hostingEnabled({ TRIOS_HOSTING: 'yes' })).toBe(false)
    expect(hostingEnabled({ TRIOS_HOSTING: ' ON ' })).toBe(true)
    let built = 0
    const app = new Hono().route(
      '/hosting',
      createHostingRoute({
        enabled: () => hostingEnabled({}),
        queen: () => {
          built++
          return null
        },
      }),
    )
    for (const [method, path] of [
      ['POST', '/hosting/hosts'],
      ['POST', '/hosting/hosts/0123456789abcdef/beat'],
      ['POST', '/hosting/lease'],
      ['POST', '/hosting/receipts'],
      ['POST', '/hosting/jobs'],
      ['GET', '/hosting/ledger'],
      ['GET', '/hosting/verdicts'],
      ['GET', '/hosting/jobs/j_1'],
    ] as const) {
      const res = await app.request(path, {
        method,
        body: method === 'POST' ? '{}' : undefined,
      })
      expect(res.status).toBe(503)
      expect(await res.json()).toEqual({ error: 'hosting_off' })
    }
    expect(built).toBe(0)
    // the default route reads the process environment the same way
    const saved = process.env.TRIOS_HOSTING
    delete process.env.TRIOS_HOSTING
    try {
      const real = new Hono().route('/hosting', createHostingRoute())
      expect((await real.request('/hosting/ledger')).status).toBe(503)
    } finally {
      if (saved !== undefined) process.env.TRIOS_HOSTING = saved
    }
  })
})

// --- the sandbox, measured ----------------------------------------------------

// Named explicitly, never found on PATH: the sandbox lets a job read only the
// tools' own directories, so the zig must be a self-contained one (ziglang.org's
// tarball, as `join` installs it). Homebrew's zig loads LLVM from another
// directory, and inside the sandbox it cannot start -- which the planted job
// would rightly report as isolation none.
const ZIG = process.env.ZIG ?? ''
const T27C = process.env.T27C ?? ''
const T27B = process.env.T27B ?? ''
const haveZig = !!ZIG && existsSync(ZIG)
const sandbox = detectSandbox()

describe('the planted job', () => {
  it.if(haveZig && sandbox !== SANDBOX_NONE)(
    'cannot open a socket or write to $HOME inside the sandbox, and can outside it',
    async () => {
      const work = mkdtempSync(join(tmpdir(), 'trios-host-probe-'))
      try {
        const opts = {
          zig: ZIG,
          workRoot: join(work, 'work'),
          cacheDir: join(work, 'cache'),
        }
        const inside = await probeIsolation({ ...opts, sandbox })
        expect(inside.ran).toBe(true)
        expect(inside.connected).toBe(false)
        expect(inside.wrote).toBe(false)
        expect(inside.isolation).toBe(
          sandbox === SANDBOX_UNSHARE ? ISO_NO_NETWORK : ISO_JOBDIR,
        )
        // the negative control: the same job, bare
        const outside = await probeIsolation({ ...opts, sandbox: SANDBOX_NONE })
        expect(outside.ran).toBe(true)
        expect(outside.connected).toBe(true)
        expect(outside.wrote).toBe(true)
        expect(outside.isolation).toBe(ISO_NONE)
        expect(outside.detail).toBe('write 0, connect 0')
        console.log(
          `planted job: inside ${inside.detail} (${inside.millis} ms), outside ${outside.detail} (${outside.millis} ms)`,
        )
      } finally {
        rmSync(work, { recursive: true, force: true })
      }
    },
    300_000,
  )
})

// --- the real thing: both halves, in the sandbox, on two hosts ----------------

const haveAll =
  haveZig &&
  !!T27C &&
  existsSync(T27C) &&
  !!T27B &&
  existsSync(T27B) &&
  process.arch === 'arm64'

describe('two hosts run both halves for real', () => {
  it.if(haveAll)(
    "agree on each half in the sandbox and serve the lab's row",
    async () => {
      const owner = generateHostKey()
      const stranger = generateHostKey()
      const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
      const work = mkdtempSync(join(tmpdir(), 'trios-host-real-'))
      const sha = (p: string) =>
        createHash('sha256')
          .update(readFileSync(join(DEFAULT_SPECS_ROOT, p.slice(6))))
          .digest('hex')
      const local = (_c: string, path: string) =>
        Promise.resolve(
          new Uint8Array(readFileSync(join(DEFAULT_SPECS_ROOT, path.slice(6)))),
        )
      const runner = (n: number) =>
        createT27cRunner({
          t27c: T27C,
          t27b: T27B,
          t27bHash: sha256Hex(readFileSync(T27B)),
          zig: ZIG,
          workRoot: join(work, `w${n}`),
          cacheDir: join(work, `cache${n}`),
          fetchFile: local,
          modelHash: sha256Hex(readFileSync(T27C)),
          zigVersion: '0.16.0',
          sandbox,
        })
      try {
        const mac = w.agent(owner.privatePem, '203.0.113.7', runner(1), {
          tier: TIER_OWNER,
        })
        const other = w.agent(stranger.privatePem, '198.51.100.20', runner(2))
        await mac.register()
        await other.register()
        const spec = 'specs/hosting/row.t27'
        const files = useClosure(
          spec,
          (p) => readFileSync(join(DEFAULT_SPECS_ROOT, p.slice(6)), 'utf8'),
          (p) => existsSync(join(DEFAULT_SPECS_ROOT, p.slice(6))),
        ).map((path) => ({ path, sha256: sha(path) }))
        for (const half of [HALF_REFERENCE, HALF_T27B])
          await mac.submitJob({
            commit: COMMIT,
            spec,
            input_hash: sha(spec),
            files,
            half,
            holds_secret: false,
            holds_personal: false,
          })
        for (let i = 0; i < 2; i++) {
          const [la, lb] = [await mac.leaseOnce(), await other.leaseOnce()]
          const a = await mac.work(la as NonNullable<typeof la>)
          const b = await other.work(lb as NonNullable<typeof lb>)
          expect(a.code).toBe('ok')
          expect(b).toMatchObject({ code: 'ok', job: { verdict: 'agreed' } })
        }
        const [row] = await w.get(`/hosting/verdicts?commit=${COMMIT}`)
        expect(row).toMatchObject({
          file: spec,
          reference: 'pass',
          t27b: 'pass',
          reference_detail: '',
          reference_disagree: [],
        })
      } finally {
        rmSync(work, { recursive: true, force: true })
      }
    },
    600_000,
  )
})
