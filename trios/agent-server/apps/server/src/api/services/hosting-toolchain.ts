/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The host agent's own files (gHashTag/trios#1756): its key, its toolchain,
 * and the inputs of a shard. Plumbing. The pins, the directory and the file
 * modes are data in specs/hosting/toolchain.t27 (queen-hosting-toolchain.gen);
 * whether a tool is downloaded, used as given or refused, and whether a
 * download is kept, are host.t27 card calls.
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
import { join } from 'node:path'
import { downloadKept, toolchainAction } from './hosting-cards'
import { generateHostKey, sha256Hex } from './hosting-wire'
import { TA_DOWNLOAD, TA_USE_LOCAL } from './queen-hosting-host-card.gen'
import {
  AGENT_DIR,
  AGENT_DIR_MODE,
  KEY_FILE,
  KEY_FILE_MODE,
  PLATFORMS,
  SPEC_SOURCE_BASE,
  T27C_BYTES,
  T27C_SHA256,
  T27C_URLS,
  ZIG_BYTES,
  ZIG_DIRS,
  ZIG_SHA256,
  ZIG_URLS,
} from './queen-hosting-toolchain.gen'

export const agentHome = (env = process.env): string =>
  env.TRIOS_HOST_HOME || join(homedir(), AGENT_DIR)

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
  if (!downloadKept(sha256Hex(body) === pin, body.length === bytes)) {
    await rm(dest, { force: true })
    throw new Error(`${url} is not the pinned file; nothing was kept`)
  }
  await writeFile(dest, body)
}

const digestOf = (path: string) => sha256Hex(readFileSync(path))

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
    const dir = join(home, ZIG_DIRS[index] as string)
    path = join(dir, 'zig')
    if (!existsSync(path)) {
      const archive = join(home, `${ZIG_DIRS[index]}.tar.xz`)
      await fetchPinned(
        url,
        ZIG_SHA256[index] as string,
        ZIG_BYTES[index] as number,
        archive,
        fetcher,
      )
      const tar = spawnSync('tar', ['-xJf', archive, '-C', home], {
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

/** t27c: the operator's binary as given, or the pinned release asset. */
export async function resolveT27c(
  home: string,
  local: string | undefined,
  index: number,
  fetcher: typeof fetch = fetch,
): Promise<Tool> {
  const url: string = T27C_URLS[index] ?? ''
  const action = toolchainAction(url !== '', local !== undefined)
  if (action === TA_USE_LOCAL)
    return { path: local as string, sha256: digestOf(local as string) }
  if (action !== TA_DOWNLOAD)
    throw new Error(
      'no t27c release is pinned for this platform yet (specs/hosting/toolchain.t27): pass --t27c <path>',
    )
  const path = join(home, 't27c')
  if (!existsSync(path)) {
    await fetchPinned(
      url,
      T27C_SHA256[index] as string,
      T27C_BYTES[index] as number,
      path,
      fetcher,
    )
    chmodSync(path, 0o755)
  }
  return { path, sha256: digestOf(path) }
}

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
    holds_secret: declarations.holdsSecret,
    holds_personal: declarations.holdsPersonal,
  }
}
