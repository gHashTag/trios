/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * SELF-HOSTING, SLICE 1 (gHashTag/trios#1756), as tests. The cards as the
 * wasm answers them; the shard's normalized output; then the demo, all local
 * and on a VirtualClock: a Queen instance on the memory store, behind its own
 * route, and host agents that talk to it through that route.
 *   - two honest hosts and one that tampers: the honest pair agree and are
 *     credited, the tamperer is not, and is struck;
 *   - a host that goes silent: its lease lapses, the job moves to another
 *     host, and its late receipt blames nobody;
 *   - a job that declares a secret is never placed on a public host.
 * The last block runs the real `t27c test-report` on two hosts, when a t27c
 * and a zig are on this machine.
 */

import { describe, expect, it } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import {
  classifyMounts,
  DEFAULT_ALLOWLIST,
  readServerSource,
} from '../../../../../tools/route-guard-audit.mjs'
import { createHostingRoute, originOf } from '../../src/api/routes/hosting'
import {
  createHostAgent,
  createT27cRunner,
  type LeasedJob,
  type RunShard,
} from '../../src/api/services/hosting-agent'
import * as cards from '../../src/api/services/hosting-cards'
import { createHostingQueen } from '../../src/api/services/hosting-queen'
import { createMemoryHostingStore } from '../../src/api/services/hosting-store'
import {
  loadOrCreateKey,
  useClosure,
} from '../../src/api/services/hosting-toolchain'
import {
  generateHostKey,
  keyIdOf,
  messageOf,
  normalizeShard,
  readTestReport,
  sha256Hex,
  signHex,
} from '../../src/api/services/hosting-wire'
import {
  ISO_JOBDIR,
  NODE_HEARTBEAT_SECONDS,
  NODE_TTL_SECONDS,
  TIER_OWNER,
  TIER_PUBLIC,
  TIER_TRUSTED,
} from '../../src/api/services/queen-hosting-host-card.gen'
import {
  JV_AGREED,
  JV_TIEBREAK,
  RC_LATE,
  REGISTER_DOMAIN,
  REGISTER_SIGNED_FIELDS,
  REPLICAS_K,
  SIDE_AGREED,
  SIDE_DISSENT,
  VW_BLOCKED,
  VW_FAIL,
  VW_PASS,
} from '../../src/api/services/queen-hosting-proof-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const QUEEN = 'http://queen.test'
const CARDS = ['host', 'proof', 'placement', 'credit', 'row'] as const
const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

// --- the cards ----------------------------------------------------------------

describe('the hosting cards', () => {
  const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')

  it('are the pinned bytes', () => {
    for (const card of CARDS)
      for (const f of [`hosting/${card}.t27`, `hosting/${card}.wasm`])
        expect(pin).toContain(`${f} sha256 ${sha(f)}`)
    expect(pin).toContain(
      `hosting/toolchain.t27 sha256 ${sha('hosting/toolchain.t27')}`,
    )
  })

  it('export every fn of each card and the fns it imports, and import nothing', () => {
    for (const card of CARDS) {
      const source = readFileSync(
        join(DEFAULT_SPECS_ROOT, `hosting/${card}.t27`),
        'utf8',
      )
      const own = [...source.matchAll(/^pub fn (\w+)\(/gm)].map((m) => m[1])
      const imported = [
        ...source.matchAll(/^use [\w:-]+::([a-z_0-9]+);$/gm),
      ].map((m) => m[1])
      const mod = new WebAssembly.Module(
        readFileSync(join(DEFAULT_SPECS_ROOT, `hosting/${card}.wasm`)),
      )
      const exported = new Set(
        WebAssembly.Module.exports(mod).map((e) => e.name),
      )
      for (const fn of [...own, ...imported])
        expect(exported.has(fn)).toBe(true)
      expect(WebAssembly.Module.imports(mod)).toEqual([])
      expect(pin).toContain(
        `${card}.wasm = t27c gen-c + zig 0.16.0 cc --target=wasm32-wasi -Oz -DNDEBUG -mexec-model=reactor, exports every fn of the card${imported.length ? ' and the ones it imports' : ''}: ${[...own, ...imported].join(',')} (reproducible: two builds, one hash)`,
      )
    }
  })

  it('answer as the specs say', () => {
    expect(cards.tierOf(TIER_OWNER, 255)).toBe(TIER_PUBLIC)
    expect(cards.tierOf(TIER_OWNER, TIER_OWNER)).toBe(TIER_OWNER)
    expect(cards.tierOf(TIER_OWNER, TIER_TRUSTED)).toBe(TIER_TRUSTED)
    expect(cards.eligible(TIER_PUBLIC, 0, true, false)).toBe(false)
    expect(cards.eligible(TIER_PUBLIC, 0, false, false)).toBe(true)
    expect(cards.hostUp(NODE_TTL_SECONDS)).toBe(false)
    expect(cards.nextIncarnation(4095)).toBe(1)
    expect(cards.jobVerdict(2, 1, 1, REPLICAS_K, false)).toBe(JV_TIEBREAK)
    expect(cards.jobVerdict(3, 2, 1, REPLICAS_K + 1, false)).toBe(JV_AGREED)
    expect(cards.creditMtri(SIDE_AGREED, false, false)).toBe(1)
    expect(cards.creditMtri(SIDE_DISSENT, false, false)).toBe(0)
    expect(cards.settlementDue(10 ** 9, false, true, true)).toBe(false)
    expect(
      cards.receiptCode({
        hostKnown: true,
        signatureOk: true,
        leased: true,
        reportedBefore: false,
        sameAsBefore: false,
        lapsed: true,
        inputMatches: true,
        modelMatches: true,
        outputRehashes: true,
      }),
    ).toBe(RC_LATE)
  })
})

// --- the shard's output ---------------------------------------------------------

const report = (spec: string, opts: { fail?: boolean; tmp?: string } = {}) =>
  [
    `--- test report: ${spec} ---`,
    '  pass  a_first',
    `  ${opts.fail ? 'FAIL' : 'pass'}  a_second`,
    '',
    '  tests       2',
    `  pass        ${opts.fail ? 1 : 2}`,
    `  FAIL        ${opts.fail ? 1 : 0}`,
    '  invariants  1   proved -- comptime, so compiling IS the check',
    `  rate    ${opts.fail ? '50.0' : '100.0'}%`,
    ...(opts.tmp ? [`  note: built in ${opts.tmp}/zig-cache in 812 ms`] : []),
    '',
    '  runtime asserts executed, per test (#6509; a pass with 0 is vacuous, T730):',
    '       3  a_first',
    '       2  a_second',
    '  vacuous passes  0 of 2  (passed with 0 runtime asserts executed)',
    '',
  ].join('\n')

const ran = (
  stdout: string,
  extra: Partial<Parameters<typeof normalizeShard>[1]> = {},
) => ({
  started: true,
  timedOut: false,
  exitCode: 0,
  stdout,
  stderr: '',
  ...extra,
})

describe('the shard output, normalized', () => {
  const spec = 'specs/demo/shard.t27'

  it('is read as the lab reads it', () => {
    const r = readTestReport(report(spec, { fail: true }))
    expect(r.tests).toBe(2)
    expect(r.fail).toBe(1)
    expect(r.results).toEqual([
      { name: 'a_first', passed: true },
      { name: 'a_second', passed: false },
    ])
    expect([...r.asserts.entries()]).toEqual([
      ['a_first', 3],
      ['a_second', 2],
    ])
  })

  it('hashes the facts, not the host: paths, times and the rate line drop out', () => {
    const a = normalizeShard(
      spec,
      ran(report(spec, { tmp: '/Users/alice/.trios-host/job-1' })),
    )
    const b = normalizeShard(spec, ran(report(spec, { tmp: '/home/bob/x' })))
    expect(a.outputHash).toBe(b.outputHash)
    expect(a.verdict).toBe(VW_PASS)
    expect(a.ops).toBe(5)
    expect(a.output).not.toContain('/Users/')
    expect(a.output).not.toContain('rate')
    expect(a.output).toBe(
      [
        't27-hosting-shard-v2',
        `spec=${spec}`,
        'verdict=pass',
        'tests=2',
        'fail=0',
        'invariants=1',
        'asserts=5',
        'vacuous=0',
        'test=pass 3 a_first',
        'test=pass 2 a_second',
        '',
      ].join('\n'),
    )
    const f = normalizeShard(spec, ran(report(spec, { fail: true })))
    expect(f.verdict).toBe(VW_FAIL)
    expect(f.outputHash).not.toBe(a.outputHash)
  })

  it("keeps a blocked report's verdict and the lab's reason, and withholds a reason that names the job's directory", () => {
    // the lab's reason, kept (proof.t27 v2): the row's reference_detail
    const lab = normalizeShard(
      spec,
      ran(
        "  BLOCKED  does not compile: spec.zig:23:16: error: use of undeclared identifier 'Ok'\n",
        { jobDir: '/Users/alice/.trios-host/work/job-1' },
      ),
    )
    expect(lab.verdict).toBe(VW_BLOCKED)
    expect(lab.output).toBe(
      `t27-hosting-shard-v2\nspec=${spec}\nverdict=blocked\nreason=does not compile: spec.zig:23:16: error: use of undeclared identifier 'Ok'\n`,
    )
    // a reason that names the host's own job directory is withheld, so two
    // honest hosts still write one text
    const a = normalizeShard(
      spec,
      ran('BLOCKED zig failed in /tmp/abc/specs/x.zig\n', {
        jobDir: '/tmp/abc',
      }),
    )
    const b = normalizeShard(
      spec,
      ran('BLOCKED zig failed in /var/folders/zz/specs/x.zig\n', {
        jobDir: '/var/folders/zz',
      }),
    )
    expect(a.outputHash).toBe(b.outputHash)
    expect(a.output).toBe(
      `t27-hosting-shard-v2\nspec=${spec}\nverdict=blocked\nreason=withheld: it names the host's job directory\n`,
    )
    // a run that did not exit zero is blocked for its exit, as lab.py says it
    expect(
      normalizeShard(spec, ran('', { exitCode: 101, stderr: 'boom\nmore' }))
        .output,
    ).toContain('reason=t27c test-report exited 101: boom\n')
    // a run cut short says something about the host, not the spec
    const t = normalizeShard(spec, ran('', { timedOut: true }))
    expect(t.word).toBe('timeout')
    const e = normalizeShard(
      spec,
      ran('', { stderr: 'Resource temporarily unavailable' }),
    )
    expect(e.word).toBe('host_error')
  })

  it('resolves a spec closure the way t27c does', () => {
    const files: Record<string, string> = {
      'specs/hosting/placement.t27':
        'use hosting::host::eligible;\nuse queen::netlink::NODE_TTL_SECONDS;\nuse a::b::{X, Y};\n',
      'specs/hosting/host.t27': 'use queen::netlink::lease_up;   // note\n',
      'specs/queen/netlink.t27': 'module QueenNetlink;\n',
      'specs/a/b.t27': '',
    }
    expect(
      useClosure(
        'specs/hosting/placement.t27',
        (p) => files[p] ?? '',
        (p) => p in files,
      ),
    ).toEqual([
      'specs/a/b.t27',
      'specs/hosting/host.t27',
      'specs/hosting/placement.t27',
      'specs/queen/netlink.t27',
    ])
  })
})

// --- the demo -------------------------------------------------------------------

const SPEC = 'specs/demo/shard.t27'
const SPEC_BYTES = 'module DemoShard;\n'
const INPUT = sha256Hex(SPEC_BYTES)
const COMMIT = 'a'.repeat(40)
const MODEL = sha256Hex('t27c 0.5.0 as built for the demo')

const job = (declare: { secret?: boolean; personal?: boolean } = {}) => ({
  commit: COMMIT,
  spec: SPEC,
  input_hash: INPUT,
  files: [{ path: SPEC, sha256: INPUT }],
  holds_secret: declare.secret ?? false,
  holds_personal: declare.personal ?? false,
})

/** An honest host: t27c's report, as every honest host sees it. */
const honest: RunShard = async (j: LeasedJob) => ({
  result: normalizeShard(j.spec, ran(report(j.spec, { fail: true }))),
  modelHash: MODEL,
  zig: '0.16.0',
})

/** The tamperer: reports a failing spec as passing, and signs it. */
const tamperer: RunShard = async (j: LeasedJob) => ({
  result: normalizeShard(j.spec, ran(report(j.spec))),
  modelHash: MODEL,
  zig: '0.16.0',
})

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
  const messages = new Map<string, number>()
  const agent = (
    privatePem: string,
    ip: string,
    runShard: RunShard,
    tierClaim = TIER_PUBLIC,
  ) =>
    createHostAgent({
      queenUrl: QUEEN,
      fetch: async (url, init) => {
        const path = url.slice(QUEEN.length)
        const kind = path.replace(
          /\/hosting\/hosts\/[0-9a-f]+\/beat/,
          '/hosting/hosts/:id/beat',
        )
        messages.set(kind, (messages.get(kind) ?? 0) + 1)
        return app.request(path, {
          ...init,
          headers: {
            ...(init.headers as Record<string, string>),
            'x-forwarded-for': ip,
          },
        })
      },
      privatePem,
      tierClaim,
      slots: 1,
      platform: 'darwin-arm64',
      isolation: ISO_JOBDIR,
      runShard,
      clock,
    })
  const get = async (path: string) =>
    // biome-ignore lint/suspicious/noExplicitAny: the tests read the routes' JSON as it comes
    (await app.request(path)).json() as Promise<any>
  return { clock, queen, app, agent, get, messages }
}

describe('the demo: two honest hosts and one that tampers', () => {
  it('credits the agreeing pair, and only them', async () => {
    const owner = generateHostKey()
    const stranger = generateHostKey()
    const cheat = generateHostKey()
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const other = w.agent(stranger.privatePem, '198.51.100.20', honest)
    const bad = w.agent(cheat.privatePem, '192.0.2.66', tamperer)
    expect((await mac.register()).tier).toBe('owner')
    expect((await other.register()).tier).toBe('public')
    expect((await bad.register()).tier).toBe('public')

    const made = await mac.submitJob(job())
    expect(made.status).toBe(200)
    const id = (made.json.job as { id: string }).id

    // the owner's Mac and the tamperer take the two replicas; nobody else fits
    const first = await mac.leaseOnce()
    const second = await bad.leaseOnce()
    expect(first?.job.id).toBe(id)
    expect(second?.job.id).toBe(id)
    expect(await other.leaseOnce()).toBeNull()

    const a = await mac.work(first as NonNullable<typeof first>)
    expect(a).toMatchObject({
      code: 'ok',
      accepted: true,
      job: { verdict: 'pending' },
    })
    // a signed, well-formed receipt with another output hash: a disagreement
    const b = await bad.work(second as NonNullable<typeof second>)
    expect(b).toMatchObject({
      code: 'ok',
      tampered: false,
      job: { verdict: 'tiebreak' },
    })

    // the tiebreak goes to the third host
    const third = await other.leaseOnce()
    expect(third?.job.id).toBe(id)
    const c = await other.work(third as NonNullable<typeof third>)
    expect(c).toMatchObject({ code: 'ok', job: { verdict: 'agreed' } })

    const ledger = await w.get('/hosting/ledger')
    const row = (key: string) =>
      ledger.hosts.find((h: { host: string }) => h.host === keyIdOf(key))
    expect(row(owner.publicHex)).toMatchObject({
      balance_mtri: 1,
      credited_jobs: 1,
      strikes: 0,
    })
    expect(row(stranger.publicHex)).toMatchObject({
      balance_mtri: 1,
      credited_jobs: 1,
      strikes: 0,
    })
    expect(row(cheat.publicHex)).toMatchObject({
      balance_mtri: 0,
      credited_jobs: 0,
      strikes: 1,
    })
    expect(ledger).toMatchObject({
      unit: 'mTRI',
      off_chain: true,
      transferable: false,
      token_value_claimed: false,
      settlement: { enabled: false },
    })
    for (const h of ledger.hosts) expect(h.settlement_due).toBe(false)

    // the agreed half; a row of the lab needs its t27b half too (hosting-row.test.ts)
    const agreed = (await w.get(`/hosting/jobs/${id}`)).result
    expect(agreed).toMatchObject({ word: 'fail', tests: 2, ops: 5 })
    expect([...agreed.hosts].sort()).toEqual(
      [keyIdOf(owner.publicHex), keyIdOf(stranger.publicHex)].sort(),
    )
    expect(await w.get(`/hosting/verdicts?commit=${COMMIT}`)).toEqual([])

    // nothing personal or secret is public: no address, no origin, no key
    const text =
      JSON.stringify(ledger) +
      JSON.stringify(await w.get(`/hosting/jobs/${id}`))
    for (const leak of [
      '203.0.113',
      '198.51.100',
      '192.0.2',
      owner.publicHex,
      'origin',
      'PRIVATE',
    ])
      expect(text).not.toContain(leak)
  })

  it('calls a receipt whose output is not what its hash says tampered, and strikes its signer', async () => {
    const owner = generateHostKey()
    const cheat = generateHostKey()
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const bad = w.agent(cheat.privatePem, '192.0.2.66', honest)
    await mac.register()
    await bad.register()
    await mac.submitJob(job())
    const lease = await bad.leaseOnce()
    const run = await honest(
      lease?.job as LeasedJob,
      new AbortController().signal,
    )
    const receipt = bad.receiptOf(lease as NonNullable<typeof lease>, run)
    // the claimed hash is an honest one; the output sent along is not
    const forged = bad.receiptOf(lease as NonNullable<typeof lease>, {
      ...run,
      result: {
        ...run.result,
        output: run.result.output.replace('verdict=fail', 'verdict=pass'),
      },
    })
    expect(forged.output_hash).toBe(receipt.output_hash)
    const answer = await bad.report(forged)
    expect(answer).toMatchObject({
      code: 'output_mismatch',
      tampered: true,
      accepted: false,
    })
    // and a second, different answer for the same lease is equivocation
    expect(await bad.report(receipt)).toMatchObject({
      code: 'equivocation',
      tampered: true,
    })
    const ledger = await w.get('/hosting/ledger')
    const row = ledger.hosts.find(
      (h: { host: string }) => h.host === keyIdOf(cheat.publicHex),
    )
    expect(row).toMatchObject({ strikes: 1, balance_mtri: 0 })
  })
})

describe('the demo: spread', () => {
  it('keeps two strangers on one network off one job, but not a stranger beside the owner', async () => {
    const owner = generateHostKey()
    const [s1, s2, s3] = [
      generateHostKey(),
      generateHostKey(),
      generateHostKey(),
    ]
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const guest = w.agent(s1.privatePem, '203.0.113.99', honest)
    const twin = w.agent(s2.privatePem, '203.0.113.42', honest)
    const far = w.agent(s3.privatePem, '198.51.100.20', honest)
    for (const a of [mac, guest, twin, far]) await a.register()
    const id = ((await mac.submitJob(job())).json.job as { id: string }).id
    // the owner's Mac and a guest on its network: allowed
    expect((await mac.leaseOnce())?.job.id).toBe(id)
    const g = await guest.leaseOnce()
    expect(g?.job.id).toBe(id)
    await guest.work(g as NonNullable<typeof g>)
    const second = await w.get(`/hosting/jobs/${id}`)
    expect(second.leases).toHaveLength(2)
    // make room for a third replica: the guest's vote stands, the Mac's lease is
    // still out, so a tiebreak is not due; a fresh job shows the stranger rule
    const other = (
      (await mac.submitJob({ ...job(), commit: 'b'.repeat(40) })).json.job as {
        id: string
      }
    ).id
    const t1 = await twin.leaseOnce()
    expect(t1?.job.id).toBe(other)
    // the guest shares the twin's /24: refused; a stranger elsewhere is not
    expect(await guest.leaseOnce()).toBeNull()
    expect((await far.leaseOnce())?.job.id).toBe(other)
  })
})

describe('the demo: a lease that lapses', () => {
  it('moves the job to another host, and the late receipt blames nobody', async () => {
    const owner = generateHostKey()
    const kb = generateHostKey()
    const kc = generateHostKey()
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const b = w.agent(kb.privatePem, '198.51.100.20', honest)
    const c = w.agent(kc.privatePem, '100.64.0.9', honest)
    await mac.register()
    await b.register()
    await c.register()
    const id = ((await mac.submitJob(job())).json.job as { id: string }).id
    const lost = await mac.leaseOnce()
    const kept = await b.leaseOnce()
    expect(await c.leaseOnce()).toBeNull()
    expect(await b.work(kept as NonNullable<typeof kept>)).toMatchObject({
      job: { verdict: 'pending' },
    })

    // the Mac goes silent; the others keep beating, and ask for work on each beat
    b.start()
    c.start()
    await w.clock.runUntil(
      w.clock.now() + (NODE_TTL_SECONDS + 2 * NODE_HEARTBEAT_SECONDS) * 1000,
    )
    b.stop()
    c.stop()

    const state = await w.get(`/hosting/jobs/${id}`)
    expect(state.verdict).toBe('agreed')
    const byHost = (key: string) =>
      state.leases.find((l: { host: string }) => l.host === keyIdOf(key))
    expect(byHost(owner.publicHex).state).toBe('lapsed')
    expect(byHost(kb.publicHex)).toMatchObject({
      state: 'reported',
      vote: true,
    })
    expect(byHost(kc.publicHex)).toMatchObject({
      state: 'reported',
      vote: true,
    })

    // the silent host's result arrives after all: late, not tampered
    expect(await mac.work(lost as NonNullable<typeof lost>)).toMatchObject({
      code: 'late',
      tampered: false,
      accepted: false,
    })
    const ledger = await w.get('/hosting/ledger')
    const row = (key: string) =>
      ledger.hosts.find((h: { host: string }) => h.host === keyIdOf(key))
    expect(row(owner.publicHex)).toMatchObject({ balance_mtri: 0, strikes: 0 })
    expect(row(kb.publicHex)).toMatchObject({ balance_mtri: 1 })
    expect(row(kc.publicHex)).toMatchObject({ balance_mtri: 1 })

    // its next beat is refused (the lease lapsed), so it registers again
    const before = mac.incarnation
    await mac.beat()
    expect(mac.incarnation).toBe(before + 1)
  })
})

describe('the demo: a job that declares a secret', () => {
  it('is never placed on a public host', async () => {
    const owner = generateHostKey()
    const stranger = generateHostKey()
    const named = generateHostKey()
    const w = world({
      [keyIdOf(owner.publicHex)]: TIER_OWNER,
      [keyIdOf(named.publicHex)]: TIER_TRUSTED,
    })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const pub = w.agent(stranger.privatePem, '198.51.100.20', honest)
    const friend = w.agent(named.privatePem, '100.64.0.9', honest, TIER_TRUSTED)
    await mac.register()
    await pub.register()
    await friend.register()
    const secret = (
      (await mac.submitJob(job({ secret: true }))).json.job as { id: string }
    ).id
    expect(await pub.leaseOnce()).toBeNull()
    expect((await w.get(`/hosting/jobs/${secret}`)).leases).toEqual([])
    // the named person and the owner may run it
    expect((await friend.leaseOnce())?.job.id).toBe(secret)
    expect((await mac.leaseOnce())?.job.id).toBe(secret)
  })

  it('is refused when it does not declare, and only the owner creates jobs', async () => {
    const owner = generateHostKey()
    const stranger = generateHostKey()
    const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
    const mac = w.agent(owner.privatePem, '203.0.113.7', honest, TIER_OWNER)
    const pub = w.agent(stranger.privatePem, '198.51.100.20', honest)
    await mac.register()
    await pub.register()
    const { holds_secret: _s, ...undeclared } = job()
    expect((await mac.submitJob(undeclared)).json.error).toBe(
      'undeclared_workload',
    )
    expect((await pub.submitJob(job())).json.error).toBe('owner_only')
    expect(
      (await mac.submitJob({ ...job(), workload_class: 1 })).json.error,
    ).toBe('shards_only')
    expect(
      (await mac.submitJob({ ...job(), input_hash: 'f'.repeat(64) })).json
        .error,
    ).toBe('bad_job')
  })
})

describe('registration and the lease', () => {
  it('grants the tier from the allowlist and caps the slots', async () => {
    const k = generateHostKey()
    const w = world()
    const greedy = createHostAgent({
      queenUrl: QUEEN,
      fetch: (url, init) => w.app.request(url.slice(QUEEN.length), init),
      privatePem: k.privatePem,
      tierClaim: TIER_OWNER,
      slots: 99,
      platform: 'linux-x64',
      isolation: ISO_JOBDIR,
      runShard: honest,
      clock: w.clock,
    })
    expect(await greedy.register()).toMatchObject({
      tier: 'public',
      slots: 2,
      incarnation: 1,
    })
    expect((await greedy.register()).incarnation).toBe(2)
  })

  it('refuses a forged or stale registration', async () => {
    const k = generateHostKey()
    const other = generateHostKey()
    const w = world()
    const fields = {
      public_key: k.publicHex,
      tier_claim: TIER_PUBLIC,
      slots: 1,
      platform: 'linux-x64',
      isolation: ISO_JOBDIR,
      utc_unix: 0,
    }
    const post = (body: unknown) =>
      w.app.request('/hosting/hosts', {
        method: 'POST',
        body: JSON.stringify(body),
      })
    const forged = signHex(
      other.privatePem,
      messageOf(REGISTER_DOMAIN, REGISTER_SIGNED_FIELDS, fields),
    )
    expect((await post({ ...fields, signature: forged })).status).toBe(401)
    await w.clock.runUntil(31_000)
    const stale = signHex(
      k.privatePem,
      messageOf(REGISTER_DOMAIN, REGISTER_SIGNED_FIELDS, fields),
    )
    expect(await (await post({ ...fields, signature: stale })).json()).toEqual({
      error: 'stale_request',
    })
  })

  it('answers 503 and touches nothing while TRIOS_HOSTING is off', async () => {
    const app = new Hono().route(
      '/hosting',
      createHostingRoute({
        enabled: () => false,
        queen: () => {
          throw new Error('touched')
        },
      }),
    )
    for (const [method, path] of [
      ['GET', '/hosting/ledger'],
      ['POST', '/hosting/hosts'],
      ['POST', '/hosting/lease'],
      ['POST', '/hosting/receipts'],
    ] as const)
      expect(
        (
          await app.request(path, {
            method,
            body: method === 'POST' ? '{}' : undefined,
          })
        ).status,
      ).toBe(503)
  })

  it('is mounted once in server.ts, allowlisted with its reason', () => {
    const mounts = classifyMounts(readServerSource()).filter(
      (m: { path: string }) => m.path.startsWith('/hosting'),
    )
    expect(mounts.map((m: { path: string }) => m.path)).toEqual(['/hosting'])
    const entry = DEFAULT_ALLOWLIST.find(
      (e: { path: string }) => e.path === '/hosting',
    )
    expect(entry?.reason).toContain('Ed25519')
    expect(entry?.reason).toContain('TRIOS_HOSTING=on')
  })

  it('keeps a network prefix hashed, never the address', () => {
    expect(originOf('10.0.0.1, 198.51.100.20')).toBe(originOf('198.51.100.99'))
    expect(originOf('198.51.100.20')).not.toBe(originOf('198.51.101.20'))
    expect(originOf('2001:db8:1:2::5')).toBe(originOf('2001:db8:1:ffff::9'))
    expect(originOf('198.51.100.20')).not.toContain('198')
  })
})

describe('the host key', () => {
  it('lives in KEY_FILE with mode 0600 under a 0700 directory, and a readable one is refused', () => {
    const home = join(mkdtempSync(join(tmpdir(), 'trios-host-')), 'home')
    try {
      const made = loadOrCreateKey(home)
      expect(made.created).toBe(true)
      expect(statSync(join(home, 'key')).mode & 0o777).toBe(0o600)
      expect(statSync(home).mode & 0o777).toBe(0o700)
      expect(loadOrCreateKey(home)).toEqual({
        privatePem: made.privatePem,
        created: false,
      })
      spawnSync('chmod', ['644', join(home, 'key')])
      expect(() => loadOrCreateKey(home)).toThrow(/mode 644/)
    } finally {
      rmSync(home, { recursive: true, force: true })
    }
  })
})

// --- the real thing -------------------------------------------------------------

const T27C =
  process.env.T27C ??
  spawnSync('which', ['t27c'], { encoding: 'utf8' }).stdout.trim()
const ZIG =
  process.env.ZIG ??
  spawnSync('which', ['zig'], { encoding: 'utf8' }).stdout.trim()
const haveTools = !!T27C && existsSync(T27C) && !!ZIG && existsSync(ZIG)

describe('two hosts run the real t27c test-report', () => {
  it.if(haveTools)(
    'and agree byte for byte on a vendored card',
    async () => {
      const owner = generateHostKey()
      const stranger = generateHostKey()
      const w = world({ [keyIdOf(owner.publicHex)]: TIER_OWNER })
      const work = mkdtempSync(join(tmpdir(), 'trios-host-run-'))
      // the vendored specs stand in for the t27 repository at the pinned commit
      const local = (_commit: string, path: string) =>
        Promise.resolve(
          new Uint8Array(
            readFileSync(join(DEFAULT_SPECS_ROOT, path.slice('specs/'.length))),
          ),
        )
      const runner = (n: number) =>
        createT27cRunner({
          t27c: T27C,
          zig: ZIG,
          workRoot: join(work, `w${n}`),
          cacheDir: join(work, `cache${n}`),
          fetchFile: local,
          modelHash: sha256Hex(readFileSync(T27C)),
          zigVersion: spawnSync(ZIG, ['version'], {
            encoding: 'utf8',
          }).stdout.trim(),
        })
      try {
        const mac = w.agent(
          owner.privatePem,
          '203.0.113.7',
          runner(1),
          TIER_OWNER,
        )
        const other = w.agent(stranger.privatePem, '198.51.100.20', runner(2))
        await mac.register()
        await other.register()
        const spec = 'specs/hosting/host.t27'
        const files = useClosure(
          spec,
          (p) => readFileSync(join(DEFAULT_SPECS_ROOT, p.slice(6)), 'utf8'),
          (p) => existsSync(join(DEFAULT_SPECS_ROOT, p.slice(6))),
        ).map((path) => ({ path, sha256: sha(path.slice(6)) }))
        expect(files.map((f) => f.path)).toEqual([
          'specs/hosting/host.t27',
          'specs/queen/netlink.t27',
        ])
        await mac.submitJob({
          commit: COMMIT,
          spec,
          input_hash: sha('hosting/host.t27'),
          files,
          holds_secret: false,
          holds_personal: false,
        })
        const [la, lb] = [await mac.leaseOnce(), await other.leaseOnce()]
        const a = await mac.work(la as NonNullable<typeof la>)
        const b = await other.work(lb as NonNullable<typeof lb>)
        expect(a.code).toBe('ok')
        expect(b).toMatchObject({ code: 'ok', job: { verdict: 'agreed' } })
        // the agreed verdict is the one a single run of the same t27c gives: the
        // lab's way, once, on the same bytes (whatever this t27c says of the spec)
        const once = await runner(3)(
          {
            id: 'once',
            commit: COMMIT,
            spec,
            input_hash: sha('hosting/host.t27'),
            files,
            model_hash: null,
          },
          new AbortController().signal,
        )
        const id = (b.job as { id: string }).id
        const { result } = await w.get(`/hosting/jobs/${id}`)
        expect(result).toMatchObject({
          word: once.result.word,
          tests: once.result.tests,
          ops: once.result.ops,
          outputHash: once.result.outputHash,
        })
      } finally {
        rmSync(work, { recursive: true, force: true })
      }
    },
    180_000,
  )
})
