#!/usr/bin/env bun
/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * trios-vault (gHashTag/trios#1759): the vault's command line.
 *
 *   owner:     init, key, grant, put, rotate, import, class, bind, dedup,
 *              export, names, audit, break-glass, serve
 *   workload:  run --scope S -- cmd ...
 *
 * WHERE THE VAULT IS: TRIOS_VAULT_URL (a running vault, e.g. the trios server
 * with TRIOS_VAULT=on, at https://host/vault), else the directory --dir,
 * TRIOS_VAULT_DIR or ~/.trios-vault, opened in this process. Local mode
 * writes its audit to the events bus when TRIOS_VAULT_DATABASE_URL is set,
 * else to <dir>/audit.jsonl.
 * WHO IS ASKING: the Ed25519 key in TRIOS_VAULT_KEY, else
 * ~/.config/trios-vault/owner.key (mode 0600; `init` and `key new` make one).
 *
 * WHAT IT PRINTS: names, ids, scopes, counts, codes and public keys. Never a
 * value: put and rotate read it at a prompt with echo off (or from a pipe with
 * --stdin, policy.t27 input_code), import reads the owner's pipe, and each
 * value is encrypted here to the vault's and the recovery recipient before it
 * travels. `run` hands the leased values to its child's environment only.
 */

import { chmod, mkdir, readFile, writeFile } from 'node:fs/promises'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { Hono } from 'hono'
import { vaultRoutes } from './api/routes/vault'
import { FORMAT_NAMES } from './api/services/queen-vault-merge-card.gen'
import {
  CODE_OK,
  INPUT_NAMES,
  INPUT_STDIN,
  INPUT_TTY,
  TIER_OWNER,
  TIER_PUBLIC,
  TIER_TRUSTED,
} from './api/services/queen-vault-policy-card.gen'
import { isRecipient } from './api/services/vault-age'
import {
  type AuditSink,
  busAuditSink,
  fileAuditSink,
} from './api/services/vault-audit'
import { inputCode } from './api/services/vault-cards'
import {
  appTransport,
  httpTransport,
  importBody,
  runScoped,
  sealValue,
  VaultClient,
  VaultRefused,
} from './api/services/vault-client'
import { parsePlan } from './api/services/vault-formats'
import { initVault, Vault } from './api/services/vault-service'
import { createFileStore, IDENTITY_FILE } from './api/services/vault-store'
import { generateKey, keyIdOf, publicHexOf } from './api/services/vault-wire'
import { createQueenPool } from './lib/db/queen-pool'

const TIERS: Record<string, number> = {
  owner: TIER_OWNER,
  trusted: TIER_TRUSTED,
  public: TIER_PUBLIC,
}

class Usage extends Error {}

interface Args {
  words: string[]
  flags: Map<string, string | true>
  rest: string[]
}

function parseArgs(argv: string[]): Args {
  const cut = argv.indexOf('--')
  const head = cut >= 0 ? argv.slice(0, cut) : argv
  const rest = cut >= 0 ? argv.slice(cut + 1) : []
  const words: string[] = []
  const flags = new Map<string, string | true>()
  for (let i = 0; i < head.length; i++) {
    const a = head[i] as string
    if (!a.startsWith('--')) {
      words.push(a)
      continue
    }
    const name = a.slice(2)
    const next = head[i + 1]
    const valued = [
      'dir',
      'recovery',
      'out',
      'key',
      'tier',
      'scope',
      'class',
      'source',
      'format',
      'env',
      'id',
      'apply',
      'plan-out',
      'after',
      'limit',
      'reason',
      'ids',
      'port',
      'host',
    ]
    if (valued.includes(name)) {
      if (next === undefined) throw new Usage(`--${name} needs a value`)
      flags.set(name, next)
      i++
    } else flags.set(name, true)
  }
  return { words, flags, rest }
}

const flag = (a: Args, name: string): string | undefined => {
  const v = a.flags.get(name)
  return typeof v === 'string' ? v : undefined
}
const list = (v: string | undefined): string[] =>
  v === undefined
    ? []
    : v
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s !== '')

const vaultDir = (a: Args) =>
  flag(a, 'dir') ??
  process.env.TRIOS_VAULT_DIR?.trim() ??
  join(homedir(), '.trios-vault')
const keyPath = () =>
  process.env.TRIOS_VAULT_KEY?.trim() ||
  join(homedir(), '.config', 'trios-vault', 'owner.key')

function auditSink(dir: string): AuditSink {
  const url = process.env.TRIOS_VAULT_DATABASE_URL?.trim()
  return url ? busAuditSink(createQueenPool(url)) : fileAuditSink(dir)
}

function localApp(dir: string): Hono {
  const vault = new Vault({
    identity: { file: join(dir, IDENTITY_FILE) },
    store: createFileStore(dir),
    audit: auditSink(dir),
  })
  return new Hono().route(
    '/vault',
    vaultRoutes(() => vault),
  )
}

async function client(a: Args): Promise<VaultClient> {
  const pem = await readFile(keyPath(), 'utf8')
  const url = process.env.TRIOS_VAULT_URL?.trim()
  if (url) return new VaultClient(httpTransport(), url, pem)
  return new VaultClient(appTransport(localApp(vaultDir(a))), '/vault', pem)
}

async function writePrivate(
  path: string,
  data: string | Uint8Array,
): Promise<void> {
  await mkdir(dirname(path), { recursive: true, mode: 0o700 })
  await writeFile(path, data, { mode: 0o600, flag: 'wx' })
  await chmod(path, 0o600)
}

const say = (line: string) => process.stdout.write(`${line}\n`)

// --- reading a value --------------------------------------------------------

async function readAll(): Promise<Uint8Array> {
  const chunks: Uint8Array[] = []
  for await (const chunk of process.stdin) chunks.push(chunk as Uint8Array)
  const out = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0))
  let at = 0
  for (const c of chunks) {
    out.set(c, at)
    at += c.length
    c.fill(0)
  }
  return out
}

/** One line typed at the terminal with echo off; Enter ends it, Ctrl-C cancels. */
function promptHidden(prompt: string): Promise<Uint8Array> {
  process.stderr.write(prompt)
  const stdin = process.stdin
  stdin.setRawMode(true)
  stdin.resume()
  const bytes: number[] = []
  return new Promise((resolve, reject) => {
    const done = () => {
      stdin.off('data', onData)
      stdin.setRawMode(false)
      stdin.pause()
      process.stderr.write('\n')
    }
    const onData = (chunk: Buffer) => {
      for (const b of chunk) {
        if (b === 0x03) {
          done()
          bytes.fill(0)
          reject(new Usage('cancelled'))
          return
        }
        if (b === 0x0d || b === 0x0a) {
          done()
          const out = new Uint8Array(bytes)
          bytes.fill(0)
          resolve(out)
          return
        }
        if (b === 0x7f || b === 0x08) bytes.pop()
        else bytes.push(b)
      }
      chunk.fill(0)
    }
    stdin.on('data', onData)
  })
}

/** A value for put or rotate: the terminal, or a pipe the owner named with --stdin. */
async function readValue(
  a: Args,
  name: string,
): Promise<{ value: Uint8Array; input: string }> {
  const isTty = process.stdin.isTTY === true
  const stdinFlag = a.flags.has('stdin')
  if (inputCode(isTty, stdinFlag) !== CODE_OK)
    throw new Usage('stdin is not a terminal: pipe a value only with --stdin')
  if (!isTty)
    return { value: await readAll(), input: INPUT_NAMES[INPUT_STDIN] as string }
  const value = await promptHidden(`value for ${name} (not shown): `)
  const again = await promptHidden('again: ')
  const same =
    value.length === again.length && value.every((b, i) => b === again[i])
  again.fill(0)
  if (!same) {
    value.fill(0)
    throw new Usage('the two entries differ; nothing was stored')
  }
  return { value, input: INPUT_NAMES[INPUT_TTY] as string }
}

// --- commands ---------------------------------------------------------------

async function cmdInit(a: Args) {
  const recovery = flag(a, 'recovery') ?? ''
  if (!isRecipient(recovery))
    throw new Usage('--recovery must be your recovery age recipient (age1...)')
  const dir = vaultDir(a)
  let pem: string
  try {
    pem = await readFile(keyPath(), 'utf8')
  } catch {
    const k = generateKey()
    await writePrivate(keyPath(), k.privatePem)
    pem = k.privatePem
    say(`owner key written to ${keyPath()} (mode 0600)`)
  }
  const owner = publicHexOf(pem)
  const { vaultRecipient } = await initVault({
    dir,
    store: createFileStore(dir),
    audit: auditSink(dir),
    recoveryRecipient: recovery,
    ownerPublicHex: owner,
  })
  say(`vault initialised in ${dir}`)
  say(`vault recipient:  ${vaultRecipient}`)
  say(`owner key id:     ${keyIdOf(owner)}`)
}

async function cmdKey(a: Args) {
  if (a.words[1] === 'new') {
    const out = flag(a, 'out')
    if (!out) throw new Usage('key new --out PATH')
    const k = generateKey()
    await writePrivate(out, k.privatePem)
    say(`key written to ${out} (mode 0600)`)
    say(`public key: ${k.publicHex}`)
    say(`key id:     ${keyIdOf(k.publicHex)}`)
    return
  }
  const pub = publicHexOf(await readFile(keyPath(), 'utf8'))
  say(`public key: ${pub}`)
  say(`key id:     ${keyIdOf(pub)}`)
}

async function cmdGrant(a: Args) {
  const tier = TIERS[flag(a, 'tier') ?? '']
  if (tier === undefined) throw new Usage('--tier owner|trusted|public')
  const r = await (await client(a)).call('grant', {
    subject: flag(a, 'key') ?? '',
    tier,
    scopes: list(flag(a, 'scope')),
  })
  say(
    `granted key ${r.subject}: tier ${flag(a, 'tier')}, scopes ${(r.scopes as string[]).join(', ') || '(none)'}`,
  )
}

async function cmdPut(a: Args, rotate: boolean) {
  const id = a.words[1]
  if (!id)
    throw new Usage(
      rotate
        ? 'rotate ID [--stdin]'
        : 'put NAME [--scope S,..] [--class C] [--stdin]',
    )
  const c = await client(a)
  const r = await c.recipients()
  const { value, input } = await readValue(a, id)
  const ct = await sealValue(value, r)
  const answer = await c.call(rotate ? 'rotate' : 'put', {
    id,
    class: flag(a, 'class'),
    scopes: list(flag(a, 'scope')),
    ct,
    input,
  })
  say(rotate ? `rotated ${id}` : `stored ${id} as ${answer.class}`)
}

async function cmdImport(a: Args) {
  const format = (FORMAT_NAMES as readonly string[]).indexOf(
    flag(a, 'format') ?? '',
  )
  const source = flag(a, 'source')
  if (format < 0 || !source)
    throw new Usage(
      'import --stdin --source LABEL --format dotenv|railway-kv|vault [--scope S] [--class C]',
    )
  const isTty = process.stdin.isTTY === true
  if (inputCode(isTty, a.flags.has('stdin')) !== CODE_OK)
    throw new Usage('stdin is not a terminal: pipe a stream only with --stdin')
  const c = await client(a)
  const r = await c.recipients()
  const raw = await readAll()
  const text = new TextDecoder().decode(raw)
  raw.fill(0)
  const { body, bad } = await importBody(
    text,
    FORMAT_NAMES[format] as string,
    r,
    {
      source,
      input: INPUT_NAMES[isTty ? INPUT_TTY : INPUT_STDIN] as string,
      scope: flag(a, 'scope'),
      class: flag(a, 'class'),
    },
  )
  if (bad.length > 0)
    throw new Usage(
      `the stream is not ${FORMAT_NAMES[format]}: lines ${bad.slice(0, 20).join(', ')}`,
    )
  const answer = await c.call('import', body)
  say(
    `imported ${answer.count} secrets from ${source} (${FORMAT_NAMES[format]})`,
  )
  const scopes = answer.scopes as string[]
  if (scopes.length > 0) say(`bound in scopes: ${scopes.join(', ')}`)
  const skipped = answer.skipped as string[]
  if (skipped.length > 0)
    say(`left out ${skipped.length} platform names: ${skipped.join(', ')}`)
}

async function cmdDedup(a: Args) {
  const c = await client(a)
  const apply = flag(a, 'apply')
  if (apply) {
    const plan = parsePlan(await readFile(apply, 'utf8'))
    if (plan.bad.length > 0)
      throw new Usage(
        `the plan has lines that are not merges: ${plan.bad.join(', ')}`,
      )
    const r = await c.call('merge', { lines: plan.lines })
    for (const m of r.merged as Array<{ canonical: string; aliases: string[] }>)
      say(`merged into ${m.canonical}; aliases: ${m.aliases.join(', ')}`)
    return
  }
  const r = await c.call('dedup')
  const report = r.report as {
    count: number
    groups: Array<{
      kind: string
      names: string[]
      ids: string[]
      sources: string[]
      services: string[]
    }>
    conflicts: Array<{
      name: string
      ids: string[]
      sources: string[]
      services: string[]
      count: number
    }>
  }
  say(
    `${report.count} secrets; ${report.groups.length} groups of copies; ${report.conflicts.length} conflicts`,
  )
  const plan: string[] = [
    '# trios-vault merge plan. Uncomment a line, put the canonical id in place of <CANONICAL>,',
    '# and for a conflict name the value to keep. Then: trios-vault dedup --apply THIS_FILE',
  ]
  for (const g of report.groups) {
    say(`${g.kind}: names ${g.names.join(', ')}`)
    say(`  ids ${g.ids.join(', ')}`)
    say(
      `  sources ${g.sources.join(', ')}${g.services.length ? `; services ${g.services.join(', ')}` : ''}`,
    )
    plan.push(`# merge <CANONICAL> = ${g.ids.join(' ')}`)
  }
  for (const k of report.conflicts) {
    say(
      `conflict: ${k.name} has ${k.count} values across ${k.sources.join(', ')}`,
    )
    say(`  ids ${k.ids.join(', ')}`)
    plan.push(`# merge <CANONICAL> = ${k.ids.join(' ')} keep=<ID>`)
  }
  const out = flag(a, 'plan-out')
  if (out) {
    await writeFile(out, `${plan.join('\n')}\n`)
    say(`plan template written to ${out}`)
  }
}

async function cmdExport(a: Args) {
  const out = flag(a, 'out')
  if (!out) throw new Usage('export --out FILE')
  const r = await (await client(a)).call('export')
  await writePrivate(out, new Uint8Array(Buffer.from(String(r.ct), 'base64')))
  say(
    `exported ${r.count} secrets to ${out}, encrypted to the recovery recipient only`,
  )
  say(`open it with: age -d -i <your recovery identity file> ${out}`)
}

async function cmdNames(a: Args) {
  const r = await (await client(a)).call('names', { scope: flag(a, 'scope') })
  for (const s of r.scopes as Array<{ scope: string; names: string[] }>)
    say(`${s.scope}: ${s.names.join(', ') || '(empty)'}`)
}

async function cmdAudit(a: Args) {
  const r = await (await client(a)).call('audit', {
    after: Number(flag(a, 'after') ?? 0),
    limit: Number(flag(a, 'limit') ?? 0),
  })
  for (const e of r.events as unknown[]) say(JSON.stringify(e))
}

async function cmdClass(a: Args) {
  const ids = a.words.slice(1)
  const r = await (await client(a)).call('class', {
    ids,
    class: flag(a, 'class'),
  })
  say(`${(r.ids as string[]).length} secrets are now ${r.class}`)
}

async function cmdBind(a: Args) {
  const mode = a.flags.has('remove')
    ? 'remove'
    : a.flags.has('replace')
      ? 'replace'
      : 'add'
  const r = await (await client(a)).call('bind', {
    scope: flag(a, 'scope'),
    env: flag(a, 'env'),
    id: flag(a, 'id'),
    mode,
  })
  say(`${r.mode}: ${r.scope} ${r.env}`)
}

async function cmdRun(a: Args, breakGlass: boolean) {
  if (a.rest.length === 0)
    throw new Usage(
      breakGlass
        ? 'break-glass --reason TEXT --ids A,B -- cmd ...'
        : 'run --scope S -- cmd ...',
    )
  const c = await client(a)
  const target = breakGlass
    ? { ids: list(flag(a, 'ids')), reason: flag(a, 'reason') ?? '' }
    : { scope: flag(a, 'scope') ?? '' }
  const r = await runScoped(c, target, a.rest)
  if (r.stopped)
    process.stderr.write(
      'trios-vault: the lease ended; the command was stopped\n',
    )
  process.exit(r.stopped && r.exitCode === 0 ? 1 : r.exitCode)
}

async function cmdServe(a: Args) {
  const dir = vaultDir(a)
  const port = Number(flag(a, 'port') ?? 8787)
  const hostname = flag(a, 'host') ?? '127.0.0.1'
  const app = localApp(dir)
  Bun.serve({ port, hostname, fetch: app.fetch })
  say(`vault ${dir} listening on http://${hostname}:${port}/vault`)
}

const COMMANDS: Record<string, (a: Args) => Promise<void>> = {
  init: cmdInit,
  key: cmdKey,
  grant: cmdGrant,
  put: (a) => cmdPut(a, false),
  rotate: (a) => cmdPut(a, true),
  import: cmdImport,
  dedup: cmdDedup,
  export: cmdExport,
  names: cmdNames,
  audit: cmdAudit,
  class: cmdClass,
  bind: cmdBind,
  run: (a) => cmdRun(a, false),
  'break-glass': (a) => cmdRun(a, true),
  serve: cmdServe,
}

export async function main(argv: string[]): Promise<number> {
  try {
    const a = parseArgs(argv)
    const cmd = COMMANDS[a.words[0] ?? '']
    if (!cmd) throw new Usage(`commands: ${Object.keys(COMMANDS).join(', ')}`)
    await cmd(a)
    return 0
  } catch (error) {
    if (error instanceof VaultRefused) {
      process.stderr.write(`trios-vault: ${error.message}\n`)
      const refused = error.detail.refused as
        | Array<Record<string, unknown>>
        | undefined
      for (const r of refused ?? [])
        process.stderr.write(
          `  ${String(r.id ?? `line ${r.line}`)}: ${String(r.reason)}\n`,
        )
      return 2
    }
    process.stderr.write(
      `trios-vault: ${error instanceof Error ? error.message : 'failed'}\n`,
    )
    return error instanceof Usage ? 64 : 1
  }
}

if (import.meta.main) {
  const code = await main(process.argv.slice(2))
  if (!['serve'].includes(process.argv[2] ?? '')) process.exit(code)
}
