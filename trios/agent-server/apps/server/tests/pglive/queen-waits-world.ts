/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * What the waits tests and the waits benchmark share: a scratch PostgreSQL
 * database, the world a release sees (GitHub and crates.io, counting every
 * read), and a way to run the waits scheduler on a virtual clock against a
 * real database - each step of virtual time waits until the scheduler is idle,
 * so a pass that awaits the database finishes at its own virtual moment.
 */

import { randomBytes } from 'node:crypto'
import { userInfo } from 'node:os'
import { Pool } from 'pg'
import {
  type Clock,
  createActorSystem,
} from '../../src/api/services/queen-actors'
import type { JobIo } from '../../src/api/services/queen-jobs'
import {
  type Resolver,
  type WaitRow,
  waitsTree,
} from '../../src/api/services/queen-waits'
import { queenSchema } from '../../src/lib/db/queen-pool'
import type { VirtualClock } from '../api/queen-virtual-clock'

const OFFLINE_KEY = 'TRIOS_PG_MIGRATE_GATE'
const URL_KEY = 'TRIOS_PG_TEST_URL'
export const offlineRequested = (): boolean =>
  (process.env[OFFLINE_KEY] ?? '').toLowerCase() === 'offline'
const adminUrl = (): string =>
  process.env[URL_KEY] ??
  `postgres://${userInfo().username}@127.0.0.1:5432/postgres`

export async function scratchDatabase(prefix: string): Promise<{
  url: string
  drop: () => Promise<void>
} | null> {
  const name = `${prefix}_${randomBytes(6).toString('hex')}`
  const admin = new Pool({ connectionString: adminUrl(), max: 1 })
  try {
    await admin.query(`CREATE DATABASE ${name}`)
  } catch (error) {
    await admin.end().catch(() => undefined)
    if (offlineRequested()) return null
    throw error
  }
  const url = new URL(adminUrl())
  url.pathname = `/${name}`
  const fresh = new Pool({ connectionString: url.toString(), max: 1 })
  try {
    await fresh.query(`CREATE SCHEMA IF NOT EXISTS ${queenSchema()}`)
  } finally {
    await fresh.end().catch(() => undefined)
  }
  return {
    url: url.toString(),
    drop: async () => {
      await admin
        .query(`DROP DATABASE IF EXISTS ${name} WITH (FORCE)`)
        .catch(() => undefined)
      await admin.end().catch(() => undefined)
    },
  }
}

export const HEAD = 'a'.repeat(40)
/** The release.yml run that published t27c 0.5.2 (2026-10-09). */
export const RUN_ID = 37900501380
const b64 = (s: string): string => Buffer.from(s).toString('base64')

/** GitHub and crates.io, as a release sees them, counting every read. */
export class ReleaseWorld implements JobIo {
  manifests = { cargo: '0.6.0', zenodo: '0.6.0' }
  tags = new Set<string>()
  releases = new Map<string, number>()
  crates = new Set<string>()
  run: { status: string; conclusion: string | null } | null = null
  posts = 0
  /** Reads of the workflow's run list (the old step reads it every round). */
  listReads = 0
  /** Reads of one run by its id (the waits resolver). */
  runReads = 0
  reads = 0

  async get(url: string): Promise<{ status: number; body: unknown }> {
    this.reads += 1
    const p = new URL(url).pathname
    if (p.endsWith('/commits/master'))
      return { status: 200, body: { sha: HEAD } }
    if (p.endsWith('/contents/bootstrap/Cargo.toml'))
      return {
        status: 200,
        body: {
          content: b64(
            `[package]\nname = "t27c"\nversion = "${this.manifests.cargo}"\n`,
          ),
        },
      }
    if (p.endsWith('/contents/.zenodo.json'))
      return {
        status: 200,
        body: {
          content: b64(JSON.stringify({ version: this.manifests.zenodo })),
        },
      }
    const tag = p.match(/\/git\/ref\/tags\/(.+)$/)
    if (tag) return { status: this.tags.has(tag[1]) ? 200 : 404, body: null }
    const rel = p.match(/\/releases\/tags\/(.+)$/)
    if (rel)
      return this.releases.has(rel[1])
        ? { status: 200, body: { id: this.releases.get(rel[1]) } }
        : { status: 404, body: null }
    const crate = p.match(/\/api\/v1\/crates\/([^/]+)\/([^/]+)$/)
    if (crate)
      return {
        status: this.crates.has(`${crate[1]}@${crate[2]}`) ? 200 : 404,
        body: null,
      }
    if (p.includes('/actions/workflows/release.yml/runs')) {
      this.listReads += 1
      return {
        status: 200,
        body: {
          workflow_runs: this.run ? [this.runBody()] : [],
        },
      }
    }
    const one = p.match(/\/actions\/runs\/(\d+)$/)
    if (one) {
      this.runReads += 1
      return Number(one[1]) === RUN_ID && this.run
        ? { status: 200, body: this.runBody() }
        : { status: 404, body: null }
    }
    return { status: 404, body: null }
  }

  private runBody() {
    return {
      id: RUN_ID,
      ...this.run,
      html_url: `https://github.com/gHashTag/t27/actions/runs/${RUN_ID}`,
    }
  }

  async postRelease(
    _url: string,
    payload: unknown,
  ): Promise<{ status: number; body: unknown }> {
    this.posts += 1
    const tag = String((payload as { tag_name: string }).tag_name)
    this.releases.set(tag, 99)
    this.tags.add(tag)
    // release.yml starts on the release event, and waits for a runner
    this.run = { status: 'queued', conclusion: null }
    return { status: 201, body: { id: 99, html_url: 'release' } }
  }

  hasReleaseToken(): boolean {
    return true
  }
}

const settle = () => new Promise<void>((r) => setImmediate(r))
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms))

export interface Scheduler {
  w: ReturnType<typeof waitsTree>
  stop: () => void
}

/** One process's scheduler: its own actor system, on the clock it is given. */
export function startScheduler(
  pool: Pool,
  clock: Clock,
  resolvers: Record<string, Resolver>,
  onWake: (row: WaitRow) => void,
): Scheduler {
  const sys = createActorSystem(clock)
  const w = waitsTree(sys, { pool, resolvers, onWake })
  const h = w.tree.start(() => {})
  return {
    w,
    stop: () => {
      w.halt()
      h.stop()
    },
  }
}

/** Until every scheduler is idle twice in a row, a few settles apart. */
export async function idle(schedulers: Scheduler[]): Promise<void> {
  for (let i = 0; i < 20_000; i++) {
    await settle()
    if (schedulers.every((s) => s.w.idle())) {
      await settle()
      await settle()
      if (schedulers.every((s) => s.w.idle())) return
    }
    if (i > 10) await sleep(1)
  }
  throw new Error('the waits scheduler never went idle')
}

/**
 * Walk virtual time to `end` in steps of `stepMs`, letting every pass finish
 * at its own moment; `each` runs at every step (a round, a world change).
 */
export async function walk(
  clock: VirtualClock,
  end: number,
  schedulers: () => Scheduler[],
  stepMs = 1000,
  each?: (now: number) => Promise<void>,
): Promise<void> {
  while (clock.now() < end) {
    await clock.runUntil(Math.min(end, clock.now() + stepMs))
    await idle(schedulers())
    if (each) {
      await each(clock.now())
      await idle(schedulers())
    }
  }
}
