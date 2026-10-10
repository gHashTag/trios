/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * trios-host: lend this computer to the Queen (gHashTag/trios#1756). It runs
 * shards of the t27 corpus -- either half of the t27b lab's row for one spec
 * at a pinned commit: the reference (`t27c test-report`) or t27b (arm64 only)
 * -- inside an OS sandbox, and is credited TRI off-chain for every result
 * another host agrees with. It is one self-contained binary:
 *
 *   bun build --compile --target=bun-darwin-arm64 tools/trios-host/trios-host.ts --outfile trios-host
 *   bun build --compile --target=bun-linux-x64    tools/trios-host/trios-host.ts --outfile trios-host
 *   bun build --compile --target=bun-linux-arm64  tools/trios-host/trios-host.ts --outfile trios-host
 *
 * The five cards travel inside it (the `with { type: 'file' }` imports below);
 * every decision it makes is one of their calls. Its key, its toolchain, its
 * jobs and zig's cache live in ~/.trios-host (TRIOS_HOST_HOME), the key with
 * mode 0600. It fetches t27c, t27b and zig itself and keeps each only when it
 * is the file specs/hosting/toolchain.t27 pins (trios#1761).
 */

import { join } from 'node:path'
import {
  createHostAgent,
  createT27cRunner,
} from '../../apps/server/src/api/services/hosting-agent'
import { useHostingCardFiles } from '../../apps/server/src/api/services/hosting-cards'
import {
  detectSandbox,
  ISOLATION_WORDS,
  probeIsolation,
  SANDBOX_WORDS,
} from '../../apps/server/src/api/services/hosting-sandbox'
import {
  agentDirs,
  agentHome,
  fileSource,
  loadOrCreateKey,
  platformIndex,
  resolveT27b,
  resolveT27c,
  resolveZig,
  shardJobOf,
} from '../../apps/server/src/api/services/hosting-toolchain'
import {
  keyIdOf,
  publicHexOf,
} from '../../apps/server/src/api/services/hosting-wire'
import { SANDBOX_NONE } from '../../apps/server/src/api/services/queen-hosting-host-card.gen'
import {
  HALF_WORDS,
  ROW_KEYS,
} from '../../apps/server/src/api/services/queen-hosting-row-card.gen'
import { PLATFORMS } from '../../apps/server/src/api/services/queen-hosting-toolchain.gen'
import creditWasm from '../../specs/hosting/credit.wasm' with { type: 'file' }
import hostWasm from '../../specs/hosting/host.wasm' with { type: 'file' }
import placementWasm from '../../specs/hosting/placement.wasm' with {
  type: 'file',
}
import proofWasm from '../../specs/hosting/proof.wasm' with { type: 'file' }
import rowWasm from '../../specs/hosting/row.wasm' with { type: 'file' }

useHostingCardFiles({
  host: hostWasm,
  proof: proofWasm,
  placement: placementWasm,
  credit: creditWasm,
  row: rowWasm,
})

const USAGE = `trios-host -- lend this computer to the Queen (gHashTag/trios#1756)

  trios-host key
      make or read the host key (~/.trios-host/key, mode 0600); print its id
  trios-host join --queen <url> [--tier public|trusted|owner] [--slots <n>]
                  [--t27c <path>] [--t27b <path>] [--zig <path>]
                  [--t27-repo <dir>] [--no-sandbox] [--once]
      fetch and check t27c, t27b (arm64) and zig against their pins, measure
      the sandbox with a planted job, register, beat, run shards, post
      receipts. The tier is a claim; the Queen grants at most what the owner's
      allowlist names for this key, and public otherwise. --no-sandbox reports
      isolation none, and such a host gets no public job. --once runs one
      shard and exits.
  trios-host probe [--zig <path>] [--no-sandbox]
      run the planted job (a socket to a local listener, a file in your home)
      inside this machine's sandbox and print what it reached
  trios-host job --queen <url> --t27-repo <dir> --commit <sha>
                 --spec <path>[,<path>...] [--half reference|t27b|both]
                 [--secret] [--personal]
      (owner key only) ask the Queen to run each spec at one commit: both
      halves of the lab's row unless --half names one
  trios-host rows --queen <url> --commit <sha> [--lab <url>]
      the Queen's complete rows for the commit beside the t27b lab's rows of
      the same commit: every key that differs, and the rows that are equal
  trios-host status --queen <url>
      this host's row of the public ledger
`

const TIERS = ['owner', 'trusted', 'public']

function flags(argv: string[]) {
  const out: Record<string, string | true> = {}
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i] as string
    if (!a.startsWith('--')) continue
    const next = argv[i + 1]
    if (next === undefined || next.startsWith('--')) out[a.slice(2)] = true
    else {
      out[a.slice(2)] = next
      i++
    }
  }
  return out
}

const realClock = {
  now: () => Date.now(),
  after: (ms: number, fn: () => void) => {
    const t = setTimeout(fn, ms)
    return () => clearTimeout(t)
  },
}

function need(f: Record<string, string | true>, name: string): string {
  const v = f[name]
  if (typeof v !== 'string') {
    process.stderr.write(`missing --${name}\n\n${USAGE}`)
    process.exit(2)
  }
  return v
}

async function main() {
  const [command, ...rest] = process.argv.slice(2)
  const f = flags(rest)
  const home = agentHome()
  if (command === 'key') {
    const { privatePem, created } = loadOrCreateKey(home)
    const pub = publicHexOf(privatePem)
    console.log(`${created ? 'made' : 'read'} ${join(home, 'key')}`)
    console.log(`host id    ${keyIdOf(pub)}`)
    console.log(`public key ${pub}`)
    return
  }
  if (command === 'status') {
    const { privatePem } = loadOrCreateKey(home)
    const id = keyIdOf(publicHexOf(privatePem))
    const res = await fetch(`${need(f, 'queen')}/hosting/ledger`)
    const ledger = (await res.json()) as { hosts?: Array<{ host: string }> }
    console.log(
      JSON.stringify(ledger.hosts?.find((h) => h.host === id) ?? null, null, 2),
    )
    return
  }
  if (command === 'probe') {
    const index = platformIndex()
    const zig = await resolveZig(
      home,
      typeof f.zig === 'string' ? f.zig : undefined,
      index,
    )
    const dirs = agentDirs(home)
    const sandbox = detectSandbox(f['no-sandbox'] === true)
    const result = await probeIsolation({
      sandbox,
      zig: zig.path,
      workRoot: dirs.work,
      cacheDir: dirs.cache,
    })
    console.log(
      JSON.stringify(
        {
          ...result,
          sandbox: SANDBOX_WORDS[result.sandbox],
          isolation: ISOLATION_WORDS[result.isolation],
        },
        null,
        2,
      ),
    )
    return
  }
  if (command === 'rows') {
    const commit = need(f, 'commit')
    const lab =
      typeof f.lab === 'string'
        ? f.lab
        : 'https://t27b-lab-production.up.railway.app'
    const ours = (await (
      await fetch(`${need(f, 'queen')}/hosting/verdicts?commit=${commit}`)
    ).json()) as Array<Record<string, unknown>>
    const run = (await (await fetch(`${lab}/runs/${commit}.json`)).json()) as {
      results?: Array<Record<string, unknown>>
    }
    const theirs = new Map((run.results ?? []).map((r) => [r.file, r]))
    let equal = 0
    for (const row of ours) {
      const want = theirs.get(row.file)
      const keys = new Set(
        [...Object.keys(want ?? {}), ...Object.keys(row)].filter((k) =>
          (ROW_KEYS as readonly string[]).includes(k),
        ),
      )
      const differ = [...keys].filter(
        (k) =>
          JSON.stringify(canon(row[k])) !== JSON.stringify(canon(want?.[k])),
      )
      if (!want) console.log(`${row.file}: the lab has no row`)
      else if (differ.length === 0) equal++
      else
        for (const k of differ)
          console.log(
            `${row.file}: ${k} hosts ${JSON.stringify(row[k])} lab ${JSON.stringify(want[k])}`,
          )
    }
    console.log(`${equal} of ${ours.length} rows equal the lab's (${commit})`)
    process.exit(equal === ours.length && ours.length > 0 ? 0 : 1)
  }
  if (command === 'job') {
    const { privatePem } = loadOrCreateKey(home)
    const halfWord = typeof f.half === 'string' ? f.half : 'both'
    const halves =
      halfWord === 'both'
        ? [0, 1]
        : [(HALF_WORDS as readonly string[]).indexOf(halfWord)]
    if (halves.some((h) => h < 0)) {
      process.stderr.write(`--half is reference, t27b or both\n`)
      process.exit(2)
    }
    const jobs = need(f, 'spec')
      .split(',')
      .filter(Boolean)
      .flatMap((spec) =>
        halves.map((half) =>
          shardJobOf(
            need(f, 't27-repo'),
            need(f, 'commit'),
            spec,
            {
              holdsSecret: f.secret === true,
              holdsPersonal: f.personal === true,
            },
            half,
          ),
        ),
      )
    const agent = createHostAgent({
      queenUrl: need(f, 'queen'),
      fetch: (url, init) => fetch(url, init),
      privatePem,
      tierClaim: 0,
      slots: 0,
      platform: PLATFORMS[Math.max(0, platformIndex())] as string,
      runShard: () => Promise.reject(new Error('this command runs nothing')),
      clock: realClock,
    })
    await agent.register()
    let failed = 0
    for (const job of jobs) {
      const { status, json } = await agent.submitJob(job)
      const made = json.job as { id?: string } | undefined
      console.log(
        `${status} ${job.spec} ${HALF_WORDS[job.half]} ${made?.id ?? json.error}`,
      )
      if (status !== 200) failed++
    }
    process.exit(failed === 0 ? 0 : 1)
  }
  if (command !== 'join') {
    process.stdout.write(USAGE)
    process.exit(command ? 2 : 0)
  }

  const queenUrl = need(f, 'queen')
  const index = platformIndex()
  const { privatePem, created } = loadOrCreateKey(home)
  console.log(
    `${created ? 'made' : 'read'} key ${keyIdOf(publicHexOf(privatePem))}`,
  )
  const local = (name: string) =>
    typeof f[name] === 'string' ? (f[name] as string) : undefined
  const zig = await resolveZig(home, local('zig'), index)
  const t27c = await resolveT27c(home, local('t27c'), index)
  const t27b = await resolveT27b(home, local('t27b'), index)
  console.log(`zig ${zig.version} ${zig.path}`)
  console.log(`t27c ${t27c.sha256} ${t27c.path}`)
  console.log(
    t27b ? `t27b ${t27b.sha256} ${t27b.path}` : 't27b none for this platform',
  )
  const dirs = agentDirs(home)
  const sandbox = detectSandbox(f['no-sandbox'] === true)
  const probe = await probeIsolation({
    sandbox,
    zig: zig.path,
    workRoot: dirs.work,
    cacheDir: dirs.cache,
    t27c: t27c.path,
    t27b: t27b?.path,
  })
  console.log(
    `sandbox ${SANDBOX_WORDS[sandbox]}: planted job ${probe.ran ? 'ran' : 'did not run'} (${probe.detail}); connected ${probe.connected}, wrote outside its directory ${probe.wrote}; isolation ${ISOLATION_WORDS[probe.isolation]}`,
  )
  if (sandbox !== SANDBOX_NONE && !probe.ran)
    console.log(
      'the sandbox could not run the planted job; this host reports isolation none',
    )
  const tier = TIERS.indexOf(typeof f.tier === 'string' ? f.tier : 'public')
  const agent = createHostAgent({
    queenUrl,
    fetch: (url, init) => fetch(url, init),
    privatePem,
    tierClaim: tier < 0 ? 2 : tier,
    slots: typeof f.slots === 'string' ? Number(f.slots) : 1,
    platform: PLATFORMS[index] ?? `${process.platform}-${process.arch}`,
    isolation: probe.isolation,
    runShard: createT27cRunner({
      t27c: t27c.path,
      t27b: t27b?.path,
      t27bHash: t27b?.sha256,
      zig: zig.path,
      workRoot: dirs.work,
      cacheDir: dirs.cache,
      fetchFile: fileSource(local('t27-repo')),
      modelHash: t27c.sha256,
      zigVersion: zig.version,
      sandbox,
    }),
    clock: realClock,
    log: (line) => console.log(`${new Date().toISOString()} ${line}`),
  })
  await agent.register()
  if (f.once === true) {
    const lease = await agent.leaseOnce()
    if (!lease) {
      console.log('no work for this host now')
      return
    }
    const answer = await agent.work(lease)
    console.log(JSON.stringify(answer))
    return
  }
  agent.start()
  const quit = () => {
    agent.stop()
    process.exit(0)
  }
  process.on('SIGINT', quit)
  process.on('SIGTERM', quit)
}

/** A value with its object keys sorted, so key order never reads as a difference. */
function canon(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(canon)
  if (v && typeof v === 'object')
    return Object.fromEntries(
      Object.keys(v as Record<string, unknown>)
        .sort()
        .map((k) => [k, canon((v as Record<string, unknown>)[k])]),
    )
  return v
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err))
  process.exit(1)
})
