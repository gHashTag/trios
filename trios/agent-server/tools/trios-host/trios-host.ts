/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * trios-host: lend this computer to the Queen (gHashTag/trios#1756, slice 1).
 * It runs shards of the t27 corpus (`t27c test-report` on one spec at a
 * pinned commit) and is credited TRI off-chain for every result another host
 * agrees with. It is one self-contained binary:
 *
 *   bun build --compile --target=bun-darwin-arm64 tools/trios-host/trios-host.ts --outfile trios-host
 *   bun build --compile --target=bun-linux-x64    tools/trios-host/trios-host.ts --outfile trios-host
 *
 * The four cards travel inside it (the `with { type: 'file' }` imports below);
 * every decision it makes is one of their calls. Its key and its toolchain
 * live in ~/.trios-host (TRIOS_HOST_HOME), the key with mode 0600.
 */

import { join } from 'node:path'
import {
  createHostAgent,
  createT27cRunner,
} from '../../apps/server/src/api/services/hosting-agent'
import { useHostingCardFiles } from '../../apps/server/src/api/services/hosting-cards'
import {
  agentHome,
  fileSource,
  loadOrCreateKey,
  platformIndex,
  resolveT27c,
  resolveZig,
  shardJobOf,
} from '../../apps/server/src/api/services/hosting-toolchain'
import {
  keyIdOf,
  publicHexOf,
} from '../../apps/server/src/api/services/hosting-wire'
import { PLATFORMS } from '../../apps/server/src/api/services/queen-hosting-toolchain.gen'
import creditWasm from '../../specs/hosting/credit.wasm' with { type: 'file' }
import hostWasm from '../../specs/hosting/host.wasm' with { type: 'file' }
import placementWasm from '../../specs/hosting/placement.wasm' with {
  type: 'file',
}
import proofWasm from '../../specs/hosting/proof.wasm' with { type: 'file' }

useHostingCardFiles({
  host: hostWasm,
  proof: proofWasm,
  placement: placementWasm,
  credit: creditWasm,
})

const USAGE = `trios-host -- lend this computer to the Queen (gHashTag/trios#1756)

  trios-host key
      make or read the host key (~/.trios-host/key, mode 0600); print its id
  trios-host join --queen <url> [--tier public|trusted|owner] [--slots <n>]
                  [--t27c <path>] [--zig <path>] [--t27-repo <dir>] [--once]
      fetch and verify the toolchain, register, beat, run shards, post receipts.
      The tier is a claim; the Queen grants at most what the owner's allowlist
      names for this key, and public otherwise. --once runs one shard and exits.
  trios-host job --queen <url> --t27-repo <dir> --commit <sha> --spec <path>
                 [--secret] [--personal]
      (owner key only) ask the Queen to run one spec at one commit
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
  if (command === 'job') {
    const { privatePem } = loadOrCreateKey(home)
    const job = shardJobOf(
      need(f, 't27-repo'),
      need(f, 'commit'),
      need(f, 'spec'),
      {
        holdsSecret: f.secret === true,
        holdsPersonal: f.personal === true,
      },
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
    const { status, json } = await agent.submitJob(job)
    console.log(status, JSON.stringify(json, null, 2))
    process.exit(status === 200 ? 0 : 1)
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
  console.log(`zig ${zig.version} ${zig.path}`)
  console.log(`t27c ${t27c.sha256} ${t27c.path}`)
  const tier = TIERS.indexOf(typeof f.tier === 'string' ? f.tier : 'public')
  const agent = createHostAgent({
    queenUrl,
    fetch: (url, init) => fetch(url, init),
    privatePem,
    tierClaim: tier < 0 ? 2 : tier,
    slots: typeof f.slots === 'string' ? Number(f.slots) : 1,
    platform: PLATFORMS[index] ?? `${process.platform}-${process.arch}`,
    runShard: createT27cRunner({
      t27c: t27c.path,
      zig: zig.path,
      workRoot: join(home, 'work'),
      cacheDir: join(home, 'cache', 'zig'),
      fetchFile: fileSource(local('t27-repo')),
      modelHash: t27c.sha256,
      zigVersion: zig.version,
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

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err))
  process.exit(1)
})
