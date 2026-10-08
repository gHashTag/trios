/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * NODES OVER THE STORE (gHashTag/t27 specs/queen/actors.t27 section 9,
 * t27#7900), against a real PostgreSQL: two actor systems, each with its own
 * pool and PgLink, as two machines would hold them.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { randomBytes } from 'node:crypto'
import { userInfo } from 'node:os'
import { Pool } from 'pg'
import {
  createActorSystem,
  type Down,
  nodeOf,
} from '../../src/api/services/queen-actors'
import {
  NODE_TTL_SECONDS,
  X_CRASH,
  X_NOCONNECTION,
} from '../../src/api/services/queen-actors-card.gen'
import {
  createPgLink,
  decodeMail,
  encodeMail,
  type PgLink,
} from '../../src/api/services/queen-actors-pg'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool, queenSchema } from '../../src/lib/db/queen-pool'

const OFFLINE_KEY = 'TRIOS_PG_MIGRATE_GATE'
const URL_KEY = 'TRIOS_PG_TEST_URL'

function offlineRequested(): boolean {
  return (process.env[OFFLINE_KEY] ?? '').toLowerCase() === 'offline'
}

function adminUrl(): string {
  return (
    process.env[URL_KEY] ??
    `postgres://${userInfo().username}@127.0.0.1:5432/postgres`
  )
}

async function scratchDatabase(): Promise<{
  url: string
  drop: () => Promise<void>
} | null> {
  const name = `queen_actors_${randomBytes(6).toString('hex')}`
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

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
const until = async (cond: () => boolean, ms: number) => {
  const end = Date.now() + ms
  while (!cond() && Date.now() < end) await sleep(50)
}

describe('two nodes over PostgreSQL', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const links: PgLink[] = []
  const previousUrl = process.env.DATABASE_URL

  beforeEach(async () => {
    scratch = await scratchDatabase()
    if (!scratch) return
    process.env.DATABASE_URL = scratch.url
    await runPgMigrations()
  })

  afterEach(async () => {
    for (const l of links.splice(0)) await l.stop().catch(() => undefined)
    for (const p of pools.splice(0)) await p.end().catch(() => undefined)
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
  })

  const node = async (n: number) => {
    const pool = createQueenPool((scratch as { url: string }).url)
    pools.push(pool)
    const link = await createPgLink(pool, n, { pollMs: 200 })
    links.push(link)
    return { link, sys: createActorSystem(undefined, { node: n, link }) }
  }

  it('a pid survives the wire exactly', () => {
    const mail = { kind: 'send' as const, to: 2n ** 63n + 5n, msg: { a: 1n } }
    expect(decodeMail(encodeMail(mail))).toEqual(mail)
  })

  it('sends cross in order, and a monitor hears the real reason', async () => {
    if (!scratch) return
    const a = await node(1)
    const b = await node(2)
    await until(() => a.link.up(2) && b.link.up(1), 12_000)
    const got: number[] = []
    const far = b.sys.spawn<number>({
      name: 'far',
      receive: (n) => {
        got.push(n)
        if (n === 99) throw new Error('boom')
      },
    })
    expect(nodeOf(far)).toBe(2)
    const downs: Down[] = []
    const watcher = a.sys.spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    a.sys.monitor(watcher, far)
    for (let i = 0; i < 20; i++) a.sys.send(far, i)
    await until(() => got.length === 20, 10_000)
    expect(got).toEqual(Array.from({ length: 20 }, (_, i) => i))
    a.sys.send(far, 99)
    await until(() => downs.length === 1, 10_000)
    expect(downs).toEqual([{ kind: 'DOWN', pid: far, reason: X_CRASH }])
  }, 60_000)

  it('a node that stops renewing is down, and its monitors hear noconnection', async () => {
    if (!scratch) return
    const a = await node(1)
    const b = await node(2)
    await until(() => a.link.up(2), 12_000)
    const target = b.sys.spawn<number>({ name: 't', receive: () => {} })
    const downs: Down[] = []
    const watcher = a.sys.spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    a.sys.monitor(watcher, target)
    await sleep(1000)
    await b.link.stop()
    await until(() => downs.length === 1, (NODE_TTL_SECONDS + 15) * 1000)
    expect(a.link.up(2)).toBe(false)
    expect(downs).toEqual([
      { kind: 'DOWN', pid: target, reason: X_NOCONNECTION },
    ])
  }, 90_000)
})
