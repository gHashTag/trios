/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The host agent (gHashTag/trios#1756): the program a user runs to lend a
 * computer to the Queen. It registers its key, beats, asks for one replica of
 * a shard when it has a free slot, runs it, and posts the output hash with a
 * signed receipt. tools/trios-host/trios-host.ts is its command line; the
 * demo tests drive this module directly against an in-process Queen.
 *
 * THIN GLUE. What a host may run, when it must register again and what a run's
 * verdict is are card calls (hosting-cards.ts); the beat period, the run bound
 * and every message layout come from the generated constants.
 *
 * WHAT A SHARD SEES: the files the job pins, each checked against its SHA-256
 * before it is written; an environment built from nothing (PATH to zig and
 * the system tools, a scratch HOME and TMPDIR, zig's caches, and
 * T27C_TEST_REPORT_EXIT_ZERO). Not one variable of the agent's own
 * environment reaches it, so no secret the user's shell holds can. And since
 * trios#1761 it runs inside the OS sandbox the host measured
 * (hosting-sandbox.ts): no network, no write outside its job directory and
 * zig's cache, no read of the key. A spec is only ever one from the t27
 * repository at the job's commit.
 *
 * TWO HALVES (trios#1761): a job is the reference half (`t27c test-report`)
 * or the t27b half (`t27b corpus` over the job's files) of one lab row; the
 * runner is chosen by the job's half, and the Queen sends a t27b half only to
 * a host that can run t27b (placement.t27 may_take_half).
 */

import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { hasRoom, mustRegisterAgain } from './hosting-cards'
import { type SandboxPaths, sandboxed } from './hosting-sandbox'
import {
  messageOf,
  normalizeShard,
  normalizeT27b,
  publicHexOf,
  type ShardResult,
  sha256Hex,
  signHex,
} from './hosting-wire'
import { NODE_HEARTBEAT_SECONDS } from './queen-hosting-host-card.gen'
import { JOB_RUN_BOUND_SECONDS } from './queen-hosting-placement-card.gen'
import {
  RECEIPT_DOMAIN,
  RECEIPT_SIGNED_FIELDS,
  REGISTER_DOMAIN,
  REGISTER_SIGNED_FIELDS,
  REQUEST_DOMAIN,
  REQUEST_SIGNED_FIELDS,
} from './queen-hosting-proof-card.gen'
import { HALF_T27B, T27B_TIMEOUT_MS } from './queen-hosting-row-card.gen'

export interface LeasedJob {
  id: string
  /** row.t27 HALF_*; a job from before the halves is the reference half. */
  half?: number
  commit: string
  spec: string
  input_hash: string
  files: Array<{ path: string; sha256: string }>
  model_hash: string | null
}

export interface Lease {
  lease: string
  nonce: string
  run_bound_seconds: number
  job: LeasedJob
  /** The incarnation the lease was given to: its receipt names that one. */
  incarnation: number
}

export interface ShardRun {
  result: ShardResult
  /** SHA-256 of the t27c binary that ran: the receipt's model_hash. */
  modelHash: string
  zig: string
  /** CPU the run's processes used, in microseconds, when the platform says. */
  cpuMicros?: number
  wallMs?: number
}

export type RunShard = (
  job: LeasedJob,
  signal: AbortSignal,
) => Promise<ShardRun>

export interface Clock {
  now(): number
  after(ms: number, fn: () => void): () => void
}

export interface HostAgentDeps {
  queenUrl: string
  fetch: (url: string, init: RequestInit) => Promise<Response>
  privatePem: string
  tierClaim: number
  slots: number
  platform: string
  /** host.t27 ISO_*: the level the planted job measured on this machine. */
  isolation?: number
  runShard: RunShard
  clock: Clock
  log?: (line: string) => void
}

export interface ReceiptAnswer {
  code: string
  accepted: boolean
  tampered: boolean
  job: { id: string; verdict: string } | null
}

export function createHostAgent(deps: HostAgentDeps) {
  const publicKey = publicHexOf(deps.privatePem)
  const log = deps.log ?? (() => {})
  let host = ''
  let incarnation = 0
  let confirmedAt = 0
  let running = 0
  let stopped = true
  let cancelTimer: (() => void) | null = null
  const inFlight = new Set<AbortController>()
  const answers: ReceiptAnswer[] = []
  const utc = () => Math.floor(deps.clock.now() / 1000)

  async function post(
    path: string,
    body: string,
    headers: Record<string, string>,
  ) {
    const res = await deps.fetch(`${deps.queenUrl}${path}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', ...headers },
      body,
    })
    const json = (await res.json().catch(() => ({}))) as Record<string, unknown>
    return { status: res.status, json }
  }

  /** A request signed with the host key (proof.t27 REQUEST_DOMAIN). */
  function signed(path: string, body: string) {
    const ts = utc()
    const message = messageOf(REQUEST_DOMAIN, REQUEST_SIGNED_FIELDS, {
      method: 'POST',
      path,
      host,
      incarnation,
      utc_unix: ts,
      body_sha256: sha256Hex(body),
    })
    return post(path, body, {
      'x-trios-host': host,
      'x-trios-inc': String(incarnation),
      'x-trios-ts': String(ts),
      'x-trios-sig': signHex(deps.privatePem, message),
    })
  }

  async function register() {
    const fields = {
      public_key: publicKey,
      tier_claim: deps.tierClaim,
      slots: deps.slots,
      platform: deps.platform,
      isolation: deps.isolation ?? 0,
      utc_unix: utc(),
    }
    const signature = signHex(
      deps.privatePem,
      messageOf(REGISTER_DOMAIN, REGISTER_SIGNED_FIELDS, fields),
    )
    const { status, json } = await post(
      '/hosting/hosts',
      JSON.stringify({ ...fields, signature }),
      {},
    )
    if (status !== 200)
      throw new Error(`register refused: ${status} ${json.error}`)
    host = String(json.host)
    incarnation = Number(json.incarnation)
    confirmedAt = deps.clock.now()
    // a new incarnation: what the last one runs is late already
    for (const c of inFlight) c.abort()
    log(
      `registered ${host} incarnation ${incarnation} tier ${json.tier} slots ${json.slots} isolation ${json.isolation}`,
    )
    return json
  }

  /** One beat; a refused one, or one unconfirmed too long, registers again (host.t27). */
  async function beat() {
    const path = `/hosting/hosts/${host}/beat`
    const { status } = await signed(path, '').catch(() => ({ status: 0 }))
    if (status === 200) confirmedAt = deps.clock.now()
    const refused = status === 409 || status === 401
    const since = Math.floor((deps.clock.now() - confirmedAt) / 1000)
    if (mustRegisterAgain(refused, since)) await register()
    return status
  }

  async function leaseOnce(): Promise<Lease | null> {
    const { status, json } = await signed('/hosting/lease', '')
    if (status === 409) {
      await register()
      return null
    }
    if (status !== 200 || !json.lease) return null
    return { ...(json as unknown as Lease), incarnation }
  }

  /** An owner-tier host's request for a new shard job. */
  async function submitJob(job: Record<string, unknown>) {
    return signed('/hosting/jobs', JSON.stringify(job))
  }

  function receiptOf(lease: Lease, run: ShardRun) {
    const fields: Record<string, unknown> = {
      job: lease.job.id,
      lease: lease.lease,
      host,
      incarnation: lease.incarnation,
      commit: lease.job.commit,
      spec: lease.job.spec,
      model_hash: run.modelHash,
      zig: run.zig,
      input_hash: lease.job.input_hash,
      output_hash: run.result.outputHash,
      ops: run.result.ops,
      verdict: run.result.word,
      nonce: lease.nonce,
      utc_unix: utc(),
    }
    const signature = signHex(
      deps.privatePem,
      messageOf(RECEIPT_DOMAIN, RECEIPT_SIGNED_FIELDS, fields),
    )
    return { ...fields, output: run.result.output, signature }
  }

  async function report(
    receipt: Record<string, unknown>,
  ): Promise<ReceiptAnswer> {
    const { json } = await post(
      '/hosting/receipts',
      JSON.stringify(receipt),
      {},
    )
    const answer = json as unknown as ReceiptAnswer
    answers.push(answer)
    log(`receipt ${receipt.lease}: ${answer.code}, job ${answer.job?.verdict}`)
    return answer
  }

  /** Run one leased replica to its receipt. */
  async function work(lease: Lease): Promise<ReceiptAnswer> {
    running += 1
    const ctl = new AbortController()
    inFlight.add(ctl)
    try {
      let run: ShardRun
      try {
        run = await deps.runShard(lease.job, ctl.signal)
      } catch (err) {
        // a pinned file that did not verify, or a t27c that would not start:
        // host_error, which is no vote, so the replica is placed again
        log(`run failed: ${err instanceof Error ? err.message : String(err)}`)
        run = {
          result:
            lease.job.half === HALF_T27B
              ? normalizeT27b(lease.job.spec, { started: false, json: null })
              : normalizeShard(lease.job.spec, {
                  started: false,
                  timedOut: false,
                  exitCode: null,
                  stdout: '',
                  stderr: '',
                }),
          modelHash: '',
          zig: '',
        }
      }
      return await report(receiptOf(lease, run))
    } finally {
      inFlight.delete(ctl)
      running -= 1
    }
  }

  /** One beat of the loop: beat, and with a free slot ask for work. */
  async function tick() {
    await beat()
    if (!hasRoom(deps.slots, running)) return
    const lease = await leaseOnce()
    if (lease) void work(lease).catch((err) => log(`work: ${String(err)}`))
  }

  function start() {
    stopped = false
    const loop = () => {
      if (stopped) return
      void tick()
        .catch((err) => log(`tick: ${String(err)}`))
        .finally(() => {
          if (!stopped)
            cancelTimer = deps.clock.after(NODE_HEARTBEAT_SECONDS * 1000, loop)
        })
    }
    loop()
  }

  function stop() {
    stopped = true
    cancelTimer?.()
    for (const c of inFlight) c.abort()
  }

  return {
    register,
    beat,
    leaseOnce,
    submitJob,
    work,
    receiptOf,
    report,
    tick,
    start,
    stop,
    answers,
    get id() {
      return host
    },
    get incarnation() {
      return incarnation
    },
    get running() {
      return running
    },
  }
}

export type HostAgent = ReturnType<typeof createHostAgent>

// --- running a shard: the reference half with t27c, the t27b half with t27b ------

export interface T27cRunnerDeps {
  t27c: string
  zig: string
  /** t27b, on a host that runs the t27b half (arm64 only). */
  t27b?: string
  /** Where each job's scratch directory is made; it is removed after the run. */
  workRoot: string
  /** zig's global cache, kept between jobs. */
  cacheDir: string
  /** The bytes of `path` at `commit` in the t27 repository. */
  fetchFile: (commit: string, path: string) => Promise<Uint8Array>
  modelHash: string
  /** SHA-256 of the t27b binary: the t27b half's model_hash. */
  t27bHash?: string
  zigVersion: string
  /** host.t27 SANDBOX_*: the sandbox every run goes through; none when absent. */
  sandbox?: number
}

/** The job's directory: every pinned file, checked, written under `specs/`. */
async function prepareJob(d: T27cRunnerDeps, job: LeasedJob): Promise<string> {
  await mkdir(d.workRoot, { recursive: true })
  const dir = await mkdtemp(join(d.workRoot, 'job-'))
  for (const f of job.files) {
    const bytes = await d.fetchFile(job.commit, f.path)
    if (sha256Hex(bytes) !== f.sha256)
      throw new Error(`${f.path} at ${job.commit} is not the pinned bytes`)
    if (f.path === job.spec && f.sha256 !== job.input_hash)
      throw new Error('the spec is not the input the job names')
    const at = join(dir, f.path)
    await mkdir(dirname(at), { recursive: true })
    await writeFile(at, bytes)
  }
  await mkdir(join(dir, 'tmp'), { recursive: true })
  await mkdir(d.cacheDir, { recursive: true })
  return dir
}

interface Bounded {
  stdout: string
  stderr: string
  exitCode: number | null
  timedOut: boolean
  cpuMicros?: number
  wallMs: number
}

/** One command in the job's sandbox; its process group is killed past the run bound or on abort. */
async function runBounded(
  d: T27cRunnerDeps,
  dir: string,
  argv: string[],
  signal: AbortSignal,
): Promise<Bounded> {
  const paths: SandboxPaths = {
    T27C: d.t27c,
    T27B: d.t27b ?? d.t27c,
    ZIG: dirname(d.zig),
    JOB: dir,
    CACHE: d.cacheDir,
  }
  const env = {
    PATH: `${dirname(d.zig)}:/usr/bin:/bin`,
    HOME: dir,
    TMPDIR: join(dir, 'tmp'),
    ZIG_GLOBAL_CACHE_DIR: d.cacheDir,
    ZIG_LOCAL_CACHE_DIR: join(dir, 'zig-local'),
    T27C_TEST_REPORT_EXIT_ZERO: '1',
  }
  const started = Date.now()
  const proc = Bun.spawn(sandboxed(d.sandbox ?? 0, paths, argv), {
    cwd: dir,
    env,
    stdout: 'pipe',
    stderr: 'pipe',
    detached: true,
  })
  let timedOut = false
  const killGroup = () => {
    try {
      process.kill(-proc.pid, 'SIGKILL')
    } catch {
      proc.kill('SIGKILL')
    }
  }
  const timer = setTimeout(() => {
    timedOut = true
    killGroup()
  }, JOB_RUN_BOUND_SECONDS * 1000)
  const onAbort = () => killGroup()
  signal.addEventListener('abort', onAbort)
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ])
  clearTimeout(timer)
  signal.removeEventListener('abort', onAbort)
  const usage = proc.resourceUsage()
  return {
    stdout,
    stderr,
    exitCode,
    timedOut,
    cpuMicros: usage ? Number(usage.cpuTime.total) : undefined,
    wallMs: Date.now() - started,
  }
}

/** The runner of both halves: pinned inputs in, a normalized result out. */
export function createT27cRunner(d: T27cRunnerDeps): RunShard {
  return async (job, signal) => {
    const dir = await prepareJob(d, job)
    try {
      if (job.half === HALF_T27B) {
        if (!d.t27b) throw new Error('this host has no t27b')
        const out = join(dir, 't27b.json')
        const r = await runBounded(
          d,
          dir,
          [
            d.t27b,
            'corpus',
            'specs',
            '--json',
            out,
            '--timeout-ms',
            String(T27B_TIMEOUT_MS),
            '--jobs',
            '1',
          ],
          signal,
        )
        const json = await readFile(out, 'utf8').catch(() => null)
        return {
          result: normalizeT27b(job.spec, { started: true, json }),
          modelHash: d.t27bHash ?? '',
          zig: '',
          cpuMicros: r.cpuMicros,
          wallMs: r.wallMs,
        }
      }
      const r = await runBounded(
        d,
        dir,
        [d.t27c, 'test-report', job.spec, '--specs-dir', 'specs', '--verbose'],
        signal,
      )
      return {
        result: normalizeShard(job.spec, {
          started: true,
          timedOut: r.timedOut,
          exitCode: r.exitCode,
          stdout: r.stdout,
          stderr: r.stderr,
          jobDir: dir,
        }),
        modelHash: d.modelHash,
        zig: d.zigVersion,
        cpuMicros: r.cpuMicros,
        wallMs: r.wallMs,
      }
    } finally {
      await rm(dir, { recursive: true, force: true })
    }
  }
}
