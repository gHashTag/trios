/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BENCHMARK BEFORE THE SWAP, FOR SELF-HOSTED SHARDS (trios#1756, numbers
 * on t27#7851). The owner's rule: a mechanism replaces another only after both
 * run the same input and the numbers are posted.
 *
 * `before` is what the t27b and t27c labs do with one spec: run
 * `t27c test-report <spec> --specs-dir specs --verbose` once, with
 * T27C_TEST_REPORT_EXIT_ZERO=1 (contrib/railway/t27b-lab/lab.py reference_one).
 * `after` is the verified path: the same spec as a shard job, run by REPLICAS_K
 * hosts through the Queen's /hosting route (the memory store, in this
 * process), each run by createT27cRunner exactly as `trios-host join` does,
 * the receipts signed and judged, the job agreed and credited.
 *
 * Part A (always): messages per job, on a VirtualClock, with a run that takes
 * the measured wall time of Part B's spec, so the beats a run spans are counted.
 * Part B (HOSTING_BENCH=on, with a t27c and a zig): wall and CPU per job. The
 * replicas run one after the other here (one heavy job at a time on this host),
 * so `after` wall is the sum of two runs; on two machines they overlap.
 * HOSTING_BENCH_OUT writes the tables as JSON. The asserts check only that
 * every job ends agreed; which side costs what is the output.
 */

import { describe, expect, it } from 'bun:test'
import { spawnSync } from 'node:child_process'
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
import { createHostingRoute } from '../../src/api/routes/hosting'
import {
  type Clock,
  createHostAgent,
  createT27cRunner,
  type LeasedJob,
  type RunShard,
} from '../../src/api/services/hosting-agent'
import { createHostingQueen } from '../../src/api/services/hosting-queen'
import { createMemoryHostingStore } from '../../src/api/services/hosting-store'
import { useClosure } from '../../src/api/services/hosting-toolchain'
import {
  generateHostKey,
  keyIdOf,
  normalizeShard,
  sha256Hex,
} from '../../src/api/services/hosting-wire'
import {
  NODE_HEARTBEAT_SECONDS,
  TIER_OWNER,
} from '../../src/api/services/queen-hosting-host-card.gen'
import { REPLICAS_K } from '../../src/api/services/queen-hosting-proof-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const QUEEN = 'http://queen.test'
const SPEC = 'specs/queen/netlink.t27'
const COMMIT = 'c'.repeat(40)
const T27C =
  process.env.T27C ??
  spawnSync('which', ['t27c'], { encoding: 'utf8' }).stdout.trim()
const ZIG =
  process.env.ZIG ??
  spawnSync('which', ['zig'], { encoding: 'utf8' }).stdout.trim()
const BENCH =
  process.env.HOSTING_BENCH === 'on' &&
  !!T27C &&
  existsSync(T27C) &&
  !!ZIG &&
  existsSync(ZIG)
const ROUNDS = Number(process.env.HOSTING_BENCH_ROUNDS ?? 3)
const local = (p: string) => join(DEFAULT_SPECS_ROOT, p.slice('specs/'.length))
const files = () =>
  useClosure(
    SPEC,
    (p) => readFileSync(local(p), 'utf8'),
    (p) => existsSync(local(p)),
  ).map((path) => ({ path, sha256: sha256Hex(readFileSync(local(path))) }))

interface Tally {
  messages: Map<string, number>
  queenCpuMicros: number
}

/** A Queen behind its route, and agents whose every request is counted and timed. */
function world(clock: Clock, allow: Record<string, number>) {
  const queen = createHostingQueen({
    store: createMemoryHostingStore(),
    now: clock.now,
    allowlist: new Map(Object.entries(allow)),
  })
  const app = new Hono().route(
    '/hosting',
    createHostingRoute({ enabled: () => true, queen: () => queen }),
  )
  const tally: Tally = { messages: new Map(), queenCpuMicros: 0 }
  const agent = (
    privatePem: string,
    ip: string,
    runShard: RunShard,
    tier = 2,
  ) =>
    createHostAgent({
      queenUrl: QUEEN,
      fetch: async (url, init) => {
        const path = url
          .slice(QUEEN.length)
          .replace(/\/hosts\/[0-9a-f]+\/beat$/, '/hosts/:id/beat')
        tally.messages.set(path, (tally.messages.get(path) ?? 0) + 1)
        const before = process.cpuUsage()
        const res = await app.request(url.slice(QUEEN.length), {
          ...init,
          headers: {
            ...(init.headers as Record<string, string>),
            'x-forwarded-for': ip,
          },
        })
        const used = process.cpuUsage(before)
        tally.queenCpuMicros += used.user + used.system
        return res
      },
      privatePem,
      tierClaim: tier,
      slots: 1,
      platform: 'darwin-arm64',
      runShard,
      clock,
    })
  return { app, agent, tally }
}

const job = () => {
  const f = files()
  return {
    commit: COMMIT,
    spec: SPEC,
    input_hash: (f.find((x) => x.path === SPEC) as { sha256: string }).sha256,
    files: f,
    holds_secret: false,
    holds_personal: false,
  }
}

const out: Record<string, unknown> = {}

describe('messages per job (virtual clock)', () => {
  it('counts every request two hosts make for one agreed shard', async () => {
    const runSeconds = Number(process.env.HOSTING_BENCH_RUN_SECONDS ?? 10)
    const clock = new VirtualClock()
    const owner = generateHostKey()
    const other = generateHostKey()
    const w = world(clock, { [keyIdOf(owner.publicHex)]: TIER_OWNER })
    // a run that takes runSeconds of the clock, as t27c's wall does
    const slow: RunShard = (j: LeasedJob) =>
      new Promise((resolve) =>
        clock.after(runSeconds * 1000, () =>
          resolve({
            result: normalizeShard(j.spec, {
              started: true,
              timedOut: false,
              exitCode: 0,
              stdout:
                '--- test report: x ---\n  pass  t\n\n  tests       1\n  FAIL        0\n',
              stderr: '',
            }),
            modelHash: 'm',
            zig: '0.16.0',
          }),
        ),
      )
    const a = w.agent(owner.privatePem, '203.0.113.7', slow, TIER_OWNER)
    const b = w.agent(other.privatePem, '198.51.100.20', slow)
    await a.register()
    await b.register()
    await a.submitJob(job())
    const setup = new Map(w.tally.messages)
    w.tally.messages.clear()
    a.start()
    b.start()
    // count until the Queen has the verdict, one virtual second at a time
    let seconds = 0
    const agreed = async () =>
      ((await (await w.app.request('/hosting/verdicts')).json()) as unknown[])
        .length === 1
    while (!(await agreed()) && seconds < runSeconds * 10) {
      await clock.runUntil(clock.now() + 1000)
      seconds += 1
    }
    const counted = Object.fromEntries(w.tally.messages)
    // then what two idle hosts cost, for the same span
    w.tally.messages.clear()
    await clock.runUntil(clock.now() + seconds * 1000)
    const idle = Object.fromEntries(w.tally.messages)
    a.stop()
    b.stop()
    expect(await agreed()).toBe(true)
    out.messages = {
      runSeconds,
      setupOnce: Object.fromEntries(setup),
      untilVerdict: counted,
      secondsToVerdict: seconds,
      idleSameSpan: idle,
    }
    console.log(
      '\nmessages, setup (once per host / per job):',
      Object.fromEntries(setup),
    )
    console.log(
      `messages from the first lease to the verdict (${seconds} s, a ${runSeconds} s run):`,
      counted,
    )
    console.log(`messages two idle hosts send in the same ${seconds} s:`, idle)
  })
})

describe('cost per job: one run against the verified path', () => {
  it.if(BENCH)(
    'runs the same spec both ways',
    async () => {
      const work = mkdtempSync(join(tmpdir(), 'hosting-bench-'))
      const modelHash = sha256Hex(readFileSync(T27C))
      const zigVersion = spawnSync(ZIG, ['version'], {
        encoding: 'utf8',
      }).stdout.trim()
      const runner = (n: string) =>
        createT27cRunner({
          t27c: T27C,
          zig: ZIG,
          workRoot: join(work, n),
          cacheDir: join(work, `cache-${n}`),
          fetchFile: (_c, p) =>
            Promise.resolve(new Uint8Array(readFileSync(local(p)))),
          modelHash,
          zigVersion,
        })
      const before: Array<{ wallMs: number; cpuMs: number; hash: string }> = []
      const after: Array<{
        wallMs: number
        hostCpuMs: number
        queenCpuMs: number
        messages: number
      }> = []
      try {
        // warm each runner's zig cache once, as a host that has run before
        const warm = {
          id: 'warm',
          commit: COMMIT,
          ...job(),
          model_hash: null,
        } as LeasedJob
        for (const n of ['once', 'h1', 'h2'])
          await runner(n)(warm, new AbortController().signal)
        for (let r = 0; r < ROUNDS; r++) {
          const once = await runner('once')(
            { ...warm, id: `once-${r}` },
            new AbortController().signal,
          )
          before.push({
            wallMs: once.wallMs ?? 0,
            cpuMs: (once.cpuMicros ?? 0) / 1000,
            hash: once.result.outputHash,
          })

          const clock = {
            now: () => Date.now(),
            after: (ms: number, fn: () => void) => {
              const t = setTimeout(fn, ms)
              return () => clearTimeout(t)
            },
          }
          const owner = generateHostKey()
          const other = generateHostKey()
          const w = world(clock, { [keyIdOf(owner.publicHex)]: TIER_OWNER })
          let hostCpu = 0
          const timed =
            (inner: RunShard): RunShard =>
            async (j, s) => {
              const run = await inner(j, s)
              hostCpu += run.cpuMicros ?? 0
              return run
            }
          const a = w.agent(
            owner.privatePem,
            '203.0.113.7',
            timed(runner('h1')),
            TIER_OWNER,
          )
          const b = w.agent(
            other.privatePem,
            '198.51.100.20',
            timed(runner('h2')),
          )
          await a.register()
          await b.register()
          await a.submitJob({ ...job(), commit: `${r}`.padStart(40, 'd') })
          w.tally.messages.clear()
          w.tally.queenCpuMicros = 0
          const t0 = performance.now()
          // both hosts beat while they work, as `trios-host join` does
          const beating = setInterval(() => {
            void a.beat()
            void b.beat()
          }, NODE_HEARTBEAT_SECONDS * 1000)
          const la = await a.leaseOnce()
          const lb = await b.leaseOnce()
          await a.work(la as NonNullable<typeof la>)
          const last = await b.work(lb as NonNullable<typeof lb>)
          const wallMs = performance.now() - t0
          clearInterval(beating)
          expect(last.job?.verdict).toBe('agreed')
          let messages = 0
          for (const n of w.tally.messages.values()) messages += n
          after.push({
            wallMs,
            hostCpuMs: hostCpu / 1000,
            queenCpuMs: w.tally.queenCpuMicros / 1000,
            messages,
          })
        }
      } finally {
        rmSync(work, { recursive: true, force: true })
      }
      const median = (xs: number[]) =>
        [...xs].sort((x, y) => x - y)[Math.floor(xs.length / 2)] as number
      const table = {
        spec: SPEC,
        rounds: ROUNDS,
        replicas: REPLICAS_K,
        before: {
          wallMs: median(before.map((x) => x.wallMs)),
          cpuMs: median(before.map((x) => x.cpuMs)),
        },
        after: {
          wallMs: median(after.map((x) => x.wallMs)),
          hostCpuMs: median(after.map((x) => x.hostCpuMs)),
          queenCpuMs: median(after.map((x) => x.queenCpuMs)),
          messages: median(after.map((x) => x.messages)),
        },
        sameHashEveryRun: new Set(before.map((x) => x.hash)).size === 1,
        raw: { before, after },
      }
      out.cost = table
      console.log(
        '\ncost per job (median of',
        ROUNDS,
        'rounds):',
        JSON.stringify({ ...table, raw: undefined }, null, 2),
      )
      if (process.env.HOSTING_BENCH_OUT)
        writeFileSync(
          process.env.HOSTING_BENCH_OUT,
          JSON.stringify(out, null, 2),
        )
    },
    600_000,
  )
})
