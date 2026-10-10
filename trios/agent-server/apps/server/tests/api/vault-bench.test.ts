/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE VAULT, MEASURED (gHashTag/trios#1759; numbers on gHashTag/t27#7851).
 *   1. Lease latency: a signed lease of a scope of 1 and of 5 names, through
 *      the in-process app, to the values in the caller's memory. One lease is
 *      age-keygen (the one-lease identity), one age --decrypt per name in the
 *      vault, one age --encrypt to the one-lease recipient, one age --decrypt
 *      in the caller: 3 + N process starts.
 *   2. `run --scope S -- true` against starting `true` directly with the same
 *      environment already in hand, which is what a service does today when
 *      its platform injects the variables.
 * Same input both ways: the same five dummy values, made at test time. The
 * asserts only hold that every run completed; which way is faster, and by how
 * much, is the output. Set VAULT_BENCH_OUT to write the results as JSON.
 */

import {
  afterAll,
  beforeAll,
  describe,
  expect,
  it,
  setDefaultTimeout,
} from 'bun:test'
import { randomBytes } from 'node:crypto'
import { writeFileSync } from 'node:fs'
import { mkdtemp, rm } from 'node:fs/promises'
import { loadavg, tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import { vaultRoutes } from '../../src/api/routes/vault'
import { TIER_TRUSTED } from '../../src/api/services/queen-vault-policy-card.gen'
import { keygenToFile } from '../../src/api/services/vault-age'
import { memoryAuditSink } from '../../src/api/services/vault-audit'
import {
  appTransport,
  runScoped,
  sealValue,
  VaultClient,
} from '../../src/api/services/vault-client'
import { initVault, Vault } from '../../src/api/services/vault-service'
import {
  createFileStore,
  IDENTITY_FILE,
} from '../../src/api/services/vault-store'
import { generateKey } from '../../src/api/services/vault-wire'

// the setup starts age a dozen times; on a loaded host that outlasts bun's 5 s
setDefaultTimeout(120_000)

const ROUNDS = Number(process.env.VAULT_BENCH_ROUNDS ?? 15)

const stats = (ms: number[]) => {
  const s = [...ms].sort((a, b) => a - b)
  const at = (q: number) =>
    s[Math.min(s.length - 1, Math.floor(q * s.length))] as number
  return {
    n: s.length,
    median: at(0.5),
    p90: at(0.9),
    min: s[0] as number,
    max: s[s.length - 1] as number,
  }
}
const round1 = (x: number) => Math.round(x * 10) / 10

describe('the vault, measured', () => {
  let root: string
  let worker: VaultClient
  const values: Record<string, string> = {}
  const results: Record<string, ReturnType<typeof stats>> = {}

  beforeAll(async () => {
    root = await mkdtemp(join(tmpdir(), 'vault-bench-'))
    const dir = join(root, 'v')
    const recovery = await keygenToFile(join(root, 'recovery.txt'))
    const owner = generateKey()
    const work = generateKey()
    const audit = memoryAuditSink()
    const store = createFileStore(dir)
    await initVault({
      dir,
      store,
      audit,
      recoveryRecipient: recovery,
      ownerPublicHex: owner.publicHex,
    })
    const vault = new Vault({
      identity: { file: join(dir, IDENTITY_FILE) },
      store,
      audit,
    })
    const app = new Hono().route(
      '/vault',
      vaultRoutes(() => vault),
    )
    const o = new VaultClient(appTransport(app), '/vault', owner.privatePem)
    const r = await o.recipients()
    for (let i = 0; i < 5; i++) {
      const name = `BENCH_KEY_${i}`
      values[name] = `vault-bench-${randomBytes(24).toString('hex')}`
      await o.call('put', {
        id: name,
        class: 'provider-api-key',
        scopes: i === 0 ? ['one', 'five'] : ['five'],
        ct: await sealValue(
          new TextEncoder().encode(values[name] as string),
          r,
        ),
        input: 'stdin',
      })
    }
    await o.call('grant', {
      subject: work.publicHex,
      tier: TIER_TRUSTED,
      scopes: ['one', 'five'],
    })
    worker = new VaultClient(appTransport(app), '/vault', work.privatePem)
  })

  afterAll(async () => {
    const load = loadavg().map(round1)
    const table = Object.entries(results).map(([k, v]) => ({
      measure: k,
      n: v.n,
      median_ms: round1(v.median),
      p90_ms: round1(v.p90),
      min_ms: round1(v.min),
      max_ms: round1(v.max),
    }))
    console.table(table)
    console.log(
      `load average ${load.join(' ')}; ${ROUNDS} rounds; bun ${Bun.version}`,
    )
    if (process.env.VAULT_BENCH_OUT)
      writeFileSync(
        process.env.VAULT_BENCH_OUT,
        JSON.stringify({ load, rounds: ROUNDS, table }, null, 1),
      )
    await rm(root, { recursive: true, force: true })
  })

  for (const scope of ['one', 'five']) {
    it(`leases a scope of ${scope} name(s)`, async () => {
      await worker.open('lease', { scope })
      const ms: number[] = []
      const revoke: number[] = []
      for (let i = 0; i < ROUNDS; i++) {
        let t = performance.now()
        const l = await worker.open('lease', { scope })
        ms.push(performance.now() - t)
        expect(Object.keys(l.env).length).toBe(scope === 'one' ? 1 : 5)
        t = performance.now()
        await worker.call('revoke', { lease: l.lease })
        revoke.push(performance.now() - t)
      }
      results[`lease, ${scope === 'one' ? 1 : 5} name(s)`] = stats(ms)
      results[`revoke after a lease of ${scope === 'one' ? 1 : 5}`] =
        stats(revoke)
    }, 120_000)
  }

  it('runs `true` through the vault and directly, with the same environment', async () => {
    const direct: number[] = []
    const viaVault: number[] = []
    for (let i = 0; i < ROUNDS; i++) {
      let t = performance.now()
      const p = Bun.spawn(['true'], {
        env: { ...process.env, ...values },
        stdio: ['ignore', 'ignore', 'ignore'],
      })
      expect(await p.exited).toBe(0)
      direct.push(performance.now() - t)
      t = performance.now()
      const r = await runScoped(worker, { scope: 'five' }, ['true'], {
        stdio: 'ignore',
      })
      viaVault.push(performance.now() - t)
      expect(r.exitCode).toBe(0)
    }
    results['spawn `true`, env given directly'] = stats(direct)
    results['run --scope five -- true'] = stats(viaVault)
  }, 120_000)
})
