/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * How a bee's work LEAVES the container, without a push credential entering it.
 *
 * The entrypoint states the rule and the reason: "a checkout the agents can
 * write is a checkout whose `.git/config` and `.git/hooks` they control, so any
 * git command a privileged process later runs inside it executes code of their
 * choosing with that process's environment. A token placed here to enable
 * `git push` is therefore a token they can take. Publishing happens from a
 * machine they cannot write to."
 *
 * That rule was kept, and the consequence was never handled: measured
 * 2026-09-15, the swarm had accepted 29 of 36 t27 tasks and the repository held
 * ZERO `queen-*` branches, with every target file still carrying the stubs the
 * tasks were opened to remove. Six days of finished work, judged and accepted,
 * sitting in a volume nobody could reach.
 *
 * So the container does not publish. It EXPORTS: it hands out a git bundle and
 * holds no credential with which to do anything else. The push happens
 * elsewhere, by whoever holds the token, against a bundle they can inspect
 * first. The asymmetry is the point - reading a bundle out is safe in a way
 * that pushing from in here is not.
 *
 * Git is still run defensively even so. It never executes the checkout's hooks
 * or config, because "read-only" is a property of what git is ASKED to do and
 * not of what a hostile `.git/config` can make it do.
 */

import { spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Hono } from 'hono'
import { logger } from '../../lib/logger'
import { workspaceRoot } from '../services/queen-dispatch'

/** The ref bee branches are cut from, as the tick sees it. */
function baseRef(): string {
  return process.env.TRIOS_REPO_REF
    ? `origin/${process.env.TRIOS_REPO_REF}`
    : 'origin/dev'
}

/**
 * Git, run so that nothing in the checkout can steer it.
 *
 * `core.hooksPath=/dev/null` stops a hook the bee wrote from running; the two
 * GIT_CONFIG_* variables stop a global or system file being read at all; and
 * `protocol.ext.allow=never` refuses the `ext::` transport, which is a shell
 * command wearing a URL. None of this is theoretical tidiness - the checkout is
 * writable by the agents, so its configuration is their input, not ours.
 *
 * Note what is NOT here: any credential. A process with nothing to steal is the
 * strongest form of this guarantee, and it is why exporting is safe where
 * pushing is not.
 */
function git(
  args: string[],
  timeoutMs = 60_000,
): Promise<{ code: number; out: string; err: string }> {
  // DROPPED TO THE BEE, exactly as every other git call in this server is.
  //
  // The server runs as root and the checkout belongs to the unprivileged
  // account, so root running git in it is refused outright:
  //
  //   fatal: detected dubious ownership in repository at '/workspace/t27'
  //
  // git then suggests `safe.directory`, and queen-dispatch.ts already records
  // why that is the wrong fix: it tells git to stop minding that a root process
  // is operating on another user's tree, which is the thing the uid split
  // exists to prevent. Dropping to the same account the agents use keeps the
  // split and makes git happy for the real reason.
  const hardened = [
    '-c',
    'core.hooksPath=/dev/null',
    '-c',
    'protocol.ext.allow=never',
    ...args,
  ]
  const quoted = ['git', ...hardened]
    .map((a) => `'${a.replaceAll("'", `'\\''`)}'`)
    .join(' ')
  const user = process.env.TRIOS_TOOL_SHELL_USER
  const argv =
    user && user.trim()
      ? ['su', '-s', '/bin/sh', user.trim(), '-c', quoted]
      : ['sh', '-c', quoted]

  return new Promise((resolve) => {
    const child = spawn(argv[0], argv.slice(1), {
      detached: true,
      env: {
        PATH: process.env.PATH ?? '/usr/bin:/bin',
        HOME: tmpdir(),
        GIT_CONFIG_GLOBAL: '/dev/null',
        GIT_CONFIG_SYSTEM: '/dev/null',
        GIT_TERMINAL_PROMPT: '0',
        // No token, by construction. Do not add one here.
      },
    })
    let out = ''
    let err = ''
    let settled = false
    const finish = (code: number) => {
      if (settled) return
      settled = true
      clearTimeout(killer)
      resolve({ code, out, err })
    }
    // The whole group: killing `su` alone leaves the git it spawned holding the
    // pipes this promise waits on, and 'close' then never fires.
    const killer = setTimeout(() => {
      try {
        if (child.pid) process.kill(-child.pid, 'SIGKILL')
      } catch {
        child.kill('SIGKILL')
      }
    }, timeoutMs)
    child.stdout.on('data', (d) => {
      out += d
    })
    child.stderr.on('data', (d) => {
      err += d
    })
    child.on('error', (e) => {
      err += String(e)
      finish(-1)
    })
    child.on('close', (code) => finish(code ?? -1))
  })
}

/** `queen-1234` -> 1234, and nothing else. */
function issueOf(branch: string): number | null {
  const m = /^queen-(\d+)$/.exec(branch.trim())
  return m ? Number(m[1]) : null
}

export function createQueenExportRoute() {
  return new Hono()
    /**
     * What is waiting to be published.
     *
     * Only branches that are AHEAD of the base: a branch level with it carries
     * no work, and listing it would send a publisher to fetch an empty bundle.
     */
    .get('/', async (c) => {
      const root = workspaceRoot()
      const base = baseRef()
      const listed = await git(['-C', root, 'branch', '--list', 'queen-*'])
      if (listed.code !== 0) {
        return c.json(
          { error: `could not list branches: ${listed.err.slice(0, 300)}` },
          500,
        )
      }
      const branches = listed.out
        .split('\n')
        .map((l) => l.replace(/^[*+]?\s*/, '').trim())
        .filter((l) => l.length > 0)

      const waiting: Array<{
        issue: number
        branch: string
        commits: number
        files: number
        head: string
      }> = []
      for (const branch of branches) {
        const issue = issueOf(branch)
        if (issue === null) continue
        const count = await git([
          '-C',
          root,
          'rev-list',
          '--count',
          `${base}..${branch}`,
        ])
        const ahead = Number(count.out.trim())
        if (!Number.isInteger(ahead) || ahead <= 0) continue
        const names = await git([
          '-C',
          root,
          'diff',
          '--name-only',
          `${base}...${branch}`,
        ])
        const head = await git(['-C', root, 'rev-parse', branch])
        waiting.push({
          issue,
          branch,
          commits: ahead,
          files: names.out.split('\n').filter((l) => l.trim()).length,
          head: head.out.trim(),
        })
      }
      waiting.sort((a, b) => a.issue - b.issue)
      return c.json({ base, count: waiting.length, branches: waiting })
    })

    /**
     * One branch, as a git bundle.
     *
     * A bundle rather than a patch because it carries the commits themselves -
     * their shas, authors and dates survive, so what lands upstream is what the
     * bee actually wrote rather than a replay of it. It is emitted relative to
     * the base, so it holds only this branch's work and stays small enough to
     * pass through JSON.
     */
    .get('/:issue', async (c) => {
      const issue = Number(c.req.param('issue'))
      if (!Number.isInteger(issue) || issue <= 0) {
        return c.json({ error: 'issue must be a positive integer' }, 400)
      }
      const root = workspaceRoot()
      const base = baseRef()
      const branch = `queen-${issue}`

      const exists = await git(['-C', root, 'rev-parse', '--verify', branch])
      if (exists.code !== 0) {
        return c.json({ error: `no branch ${branch} in this checkout` }, 404)
      }
      const ahead = await git([
        '-C',
        root,
        'rev-list',
        '--count',
        `${base}..${branch}`,
      ])
      if (Number(ahead.out.trim()) <= 0) {
        return c.json(
          { error: `${branch} has no commits beyond ${base}` },
          409,
        )
      }

      // Written to a path OUTSIDE the checkout: a bundle created inside a tree
      // the agents can write is a file they can replace between creation and
      // read.
      // Written by the BEE (git drops to it), then read by this process as root:
      // a path both can reach, and outside the checkout so the agents cannot
      // swap the file between its creation and its read.
      const path = join(tmpdir(), `queen-export-${issue}-${randomUUID()}.bundle`)
      try {
        const made = await git(
          ['-C', root, 'bundle', 'create', path, `${base}..${branch}`],
          120_000,
        )
        if (made.code !== 0) {
          return c.json(
            { error: `bundle failed: ${made.err.slice(0, 400)}` },
            500,
          )
        }
        const log = await git([
          '-C',
          root,
          'log',
          '--format=%H%x1f%an%x1f%aI%x1f%s',
          `${base}..${branch}`,
        ])
        const commits = log.out
          .split('\n')
          .filter((l) => l.trim())
          .map((l) => {
            const [sha, author, date, subject] = l.split('\x1f')
            return { sha, author, date, subject }
          })
        const files = await git([
          '-C',
          root,
          'diff',
          '--name-only',
          `${base}...${branch}`,
        ])
        const bytes = await readFile(path)
        logger.info('Queen export served', {
          issue,
          branch,
          commits: commits.length,
          bundleBytes: bytes.length,
        })
        return c.json({
          issue,
          branch,
          base,
          commits,
          files: files.out.split('\n').filter((l) => l.trim()),
          bundleBase64: bytes.toString('base64'),
        })
      } finally {
        await rm(path, { force: true }).catch(() => {})
      }
    })
}
