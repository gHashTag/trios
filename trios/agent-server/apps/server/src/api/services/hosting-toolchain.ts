/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The host agent's own files (gHashTag/trios#1756): its key, its toolchain,
 * and the inputs of a shard. Plumbing. The pins, the directories and the file
 * modes are data in specs/hosting/toolchain.t27 (queen-hosting-toolchain.gen);
 * whether a tool is downloaded, used as given or refused, and whether a
 * download is kept, are host.t27 card calls.
 *
 * WHY A KEPT FILE IS CHECKED AGAIN ON EVERY START (trios#1761): a tool on disk
 * is a file anyone with the user's rights could have changed since it was
 * fetched. Its digest and size are compared with the pin before each `join`;
 * a file that no longer matches is deleted and fetched again, and a fetch
 * that does not match is refused by name and nothing is kept.
 *
 * THE LAYOUT: the key in AGENT_DIR, the tools in TOOLS_DIR, the jobs in
 * WORK_DIR, zig's cache in CACHE_DIR. A sandboxed job is given the tools, its
 * own job directory and the cache; never the directory that holds the key.
 */

import { spawnSync } from 'node:child_process'
import {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { rm, writeFile } from 'node:fs/promises'
import { homedir } from 'node:os'
import { basename, join } from 'node:path'
import { downloadKept, toolchainAction } from './hosting-cards'
import { generateHostKey, sha256Hex } from './hosting-wire'
import { TA_DOWNLOAD, TA_USE_LOCAL } from './queen-hosting-host-card.gen'
import {
  AGENT_DIR,
  AGENT_DIR_MODE,
  CACHE_DIR,
  KEY_FILE,
  KEY_FILE_MODE,
  PLATFORM_ARCH,
  PLATFORMS,
  SPEC_SOURCE_BASE,
  T27B_BYTES,
  T27B_SHA256,
  T27B_URLS,
  T27C_BYTES,
  T27C_SHA256,
  T27C_URLS,
  TOOLS_DIR,
  WORK_DIR,
  ZIG_BYTES,
  ZIG_DIRS,
  ZIG_SHA256,
  ZIG_URLS,
} from './queen-hosting-toolchain.gen'

export const agentHome = (env = process.env): string =>
  env.TRIOS_HOST_HOME || join(homedir(), AGENT_DIR)

/** The three directories beside the key (toolchain.t27 section 5). */
export const agentDirs = (home: string) => ({
  tools: join(home, TOOLS_DIR),
  work: join(home, WORK_DIR),
  cache: join(home, CACHE_DIR, 'zig'),
})

/** A platform's instruction set (toolchain.t27 PLATFORM_ARCH), or 255 for one it does not list. */
export const archOf = (platform: string): number =>
  (PLATFORM_ARCH as readonly number[])[
    (PLATFORMS as readonly string[]).indexOf(platform)
  ] ?? 255

/** This machine's entry in toolchain.t27 PLATFORMS, or -1. */
export function platformIndex(
  platform: string = process.platform,
  arch: string = process.arch,
): number {
  const os =
    platform === 'darwin' ? 'darwin' : platform === 'linux' ? 'linux' : platform
  const cpu = arch === 'arm64' ? 'arm64' : arch === 'x64' ? 'x64' : arch
  return (PLATFORMS as readonly string[]).indexOf(`${os}-${cpu}`)
}

/**
 * The host key: read from KEY_FILE, or made there. The directory is made
 * owner-only and the key file owner read-write only; a key file anyone else
 * can read is refused rather than used.
 */
export function loadOrCreateKey(home: string): {
  privatePem: string
  created: boolean
} {
  mkdirSync(home, { recursive: true, mode: AGENT_DIR_MODE })
  chmodSync(home, AGENT_DIR_MODE)
  const file = join(home, KEY_FILE)
  if (existsSync(file)) {
    const mode = statSync(file).mode & 0o777
    if (mode !== KEY_FILE_MODE)
      throw new Error(
        `${file} has mode ${mode.toString(8)}; the key must be ${KEY_FILE_MODE.toString(8)}`,
      )
    return { privatePem: readFileSync(file, 'utf8'), created: false }
  }
  const { privatePem } = generateHostKey()
  writeFileSync(file, privatePem, { mode: KEY_FILE_MODE, flag: 'wx' })
  chmodSync(file, KEY_FILE_MODE)
  return { privatePem, created: true }
}

export interface Tool {
  path: string
  sha256: string
}

/** Download `url` to `dest`, keeping it only when the pin and the size match. */
export async function fetchPinned(
  url: string,
  pin: string,
  bytes: number,
  dest: string,
  fetcher: typeof fetch = fetch,
): Promise<void> {
  const res = await fetcher(url)
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`)
  const body = new Uint8Array(await res.arrayBuffer())
  const digest = sha256Hex(body)
  if (!downloadKept(digest === pin, body.length === bytes)) {
    await rm(dest, { force: true })
    throw new Error(
      `${basename(url)} refused: sha256 ${digest} (${body.length} bytes), pinned ${pin} (${bytes} bytes); nothing was kept`,
    )
  }
  await writeFile(dest, body)
}

const digestOf = (path: string) => sha256Hex(readFileSync(path))

/** Whether a kept file still has its pinned digest and size (host.t27 download_kept). */
function stillPinned(path: string, pin: string, bytes: number): boolean {
  const body = readFileSync(path)
  return downloadKept(sha256Hex(body) === pin, body.length === bytes)
}

/** zig: the operator's binary as given, or the pinned archive, unpacked once. */
export async function resolveZig(
  home: string,
  local: string | undefined,
  index: number,
  fetcher: typeof fetch = fetch,
): Promise<Tool & { version: string }> {
  const url: string = ZIG_URLS[index] ?? ''
  const action = toolchainAction(url !== '', local !== undefined)
  let path: string
  if (action === TA_USE_LOCAL) path = local as string
  else if (action === TA_DOWNLOAD) {
    const tools = agentDirs(home).tools
    mkdirSync(tools, { recursive: true })
    const dir = join(tools, ZIG_DIRS[index] as string)
    path = join(dir, 'zig')
    if (!existsSync(path)) {
      const archive = join(tools, `${ZIG_DIRS[index]}.tar.xz`)
      await fetchPinned(
        url,
        ZIG_SHA256[index] as string,
        ZIG_BYTES[index] as number,
        archive,
        fetcher,
      )
      const tar = spawnSync('tar', ['-xJf', archive, '-C', tools], {
        stdio: 'inherit',
      })
      await rm(archive, { force: true })
      if (tar.status !== 0) throw new Error('could not unpack zig')
    }
  } else throw new Error('no zig for this platform: pass --zig <path>')
  const version = spawnSync(path, ['version'], {
    encoding: 'utf8',
  }).stdout.trim()
  return { path, sha256: digestOf(path), version }
}

/**
 * A release binary (t27c or t27b): the operator's as given, or the pinned
 * asset in TOOLS_DIR, checked again on every start. `null` when nothing is
 * pinned for this platform and none is given (t27b on x64): that half is
 * never placed here (placement.t27 half_fits).
 */
async function resolveBinary(
  name: string,
  pins: {
    urls: readonly string[]
    sha: readonly string[]
    bytes: readonly number[]
  },
  home: string,
  local: string | undefined,
  index: number,
  fetcher: typeof fetch,
): Promise<Tool | null> {
  const url: string = pins.urls[index] ?? ''
  const action = toolchainAction(url !== '', local !== undefined)
  if (action === TA_USE_LOCAL)
    return { path: local as string, sha256: digestOf(local as string) }
  if (action !== TA_DOWNLOAD) return null
  const tools = agentDirs(home).tools
  mkdirSync(tools, { recursive: true })
  const path = join(tools, name)
  const pin = pins.sha[index] as string
  const bytes = pins.bytes[index] as number
  if (existsSync(path) && !stillPinned(path, pin, bytes))
    await rm(path, { force: true })
  if (!existsSync(path)) {
    await fetchPinned(url, pin, bytes, path, fetcher)
    chmodSync(path, 0o755)
  }
  return { path, sha256: digestOf(path) }
}

/** t27c: the operator's binary as given, or the pinned release asset. */
export async function resolveT27c(
  home: string,
  local: string | undefined,
  index: number,
  fetcher: typeof fetch = fetch,
): Promise<Tool> {
  const tool = await resolveBinary(
    't27c',
    { urls: T27C_URLS, sha: T27C_SHA256, bytes: T27C_BYTES },
    home,
    local,
    index,
    fetcher,
  )
  if (!tool)
    throw new Error(
      'no t27c is pinned for this platform (specs/hosting/toolchain.t27): pass --t27c <path>',
    )
  return tool
}

/** t27b: as t27c, where the platform is arm64; null elsewhere. */
export const resolveT27b = (
  home: string,
  local: string | undefined,
  index: number,
  fetcher: typeof fetch = fetch,
): Promise<Tool | null> =>
  resolveBinary(
    't27b',
    { urls: T27B_URLS, sha: T27B_SHA256, bytes: T27B_BYTES },
    home,
    local,
    index,
    fetcher,
  )

// --- a shard's inputs ---------------------------------------------------------

/** The bytes of `path` at `commit`: from a local clone when given, else the pinned source. */
export function fileSource(
  repo: string | undefined,
  fetcher: typeof fetch = fetch,
) {
  return async (commit: string, path: string): Promise<Uint8Array> => {
    if (repo) {
      const r = spawnSync('git', ['-C', repo, 'show', `${commit}:${path}`], {
        maxBuffer: 64 * 1024 * 1024,
      })
      if (r.status !== 0) throw new Error(`git show ${commit}:${path} failed`)
      return new Uint8Array(r.stdout)
    }
    const res = await fetcher(`${SPEC_SOURCE_BASE}${commit}/${path}`)
    if (!res.ok) throw new Error(`${path} at ${commit}: HTTP ${res.status}`)
    return new Uint8Array(await res.arrayBuffer())
  }
}

/**
 * The spec and every spec it reaches through `use`, at `commit`: the reading
 * of t27c's bootstrap/src/use_resolve.rs use_path_expr and use_target, over
 * the commit's file list. `use a::b;` names specs/a/b.t27; `use a::b::Item;`
 * names it when specs/a/b/Item.t27 is not a spec; `use a::b::{X, Y};` names
 * specs/a/b.t27. A name that resolves to nothing is left to t27c, which
 * reports it the same way on every host.
 */
export function useClosure(
  spec: string,
  read: (path: string) => string,
  exists: (path: string) => boolean,
): string[] {
  const seen = new Set<string>()
  const todo = [spec]
  const pathOf = (expr: string) =>
    `specs/${expr
      .split('::')
      .flatMap((s) => s.split('.'))
      .join('/')}.t27`
  while (todo.length > 0) {
    const at = todo.pop() as string
    if (seen.has(at)) continue
    seen.add(at)
    for (const line of read(at).split('\n')) {
      const t = line.trim()
      if (!t.startsWith('use ')) continue
      let expr = t.slice(4)
      const comment = expr.indexOf('//')
      if (comment >= 0) expr = expr.slice(0, comment)
      expr = expr.trim().replace(/;+$/, '').trim()
      if (!expr || (!expr.includes('::') && expr.includes(' '))) continue
      const whole = pathOf(expr)
      if (exists(whole)) {
        todo.push(whole)
        continue
      }
      const brace = expr.indexOf('{')
      const moduleExpr =
        brace >= 0
          ? expr
              .slice(0, brace)
              .trim()
              .replace(/(::|\.)$/, '')
          : expr.split('::').slice(0, -1).join('::')
      if (!moduleExpr) continue
      const mod = pathOf(moduleExpr)
      if (exists(mod)) todo.push(mod)
    }
  }
  return [...seen].sort()
}

/** A shard job for `spec` at `commit`, every input pinned, from a local clone. */
export function shardJobOf(
  repo: string,
  commit: string,
  spec: string,
  declarations: { holdsSecret: boolean; holdsPersonal: boolean },
  half = 0,
) {
  const files = new Set(
    spawnSync(
      'git',
      ['-C', repo, 'ls-tree', '-r', '--name-only', commit, 'specs/'],
      {
        encoding: 'utf8',
        maxBuffer: 64 * 1024 * 1024,
      },
    ).stdout.split('\n'),
  )
  const show = (path: string) =>
    spawnSync('git', ['-C', repo, 'show', `${commit}:${path}`], {
      maxBuffer: 64 * 1024 * 1024,
    }).stdout as Buffer
  if (!files.has(spec)) throw new Error(`${spec} is not in ${commit}`)
  const closure = useClosure(
    spec,
    (p) => show(p).toString('utf8'),
    (p) => files.has(p),
  )
  const pinned = closure.map((path) => ({
    path,
    sha256: sha256Hex(show(path)),
  }))
  return {
    commit,
    spec,
    input_hash: (pinned.find((f) => f.path === spec) as { sha256: string })
      .sha256,
    files: pinned,
    half,
    holds_secret: declarations.holdsSecret,
    holds_personal: declarations.holdsPersonal,
  }
}
