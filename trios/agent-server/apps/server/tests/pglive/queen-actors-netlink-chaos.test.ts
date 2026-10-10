/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE NODE LINK UNDER CHAOS (gHashTag/t27 specs/queen/netlink.t27, trios#1712
 * lane 6). One script, run against the link from before netlink.t27 and the
 * link after it, on a real PostgreSQL, so the two columns of the benchmark
 * come from the same input. It uses only what both links have.
 *
 * It prints a table and, with QUEEN_BENCH_OUT, writes the numbers as JSON.
 * The asserts check that each scenario ran, not what it measured: the
 * numbers are the result.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { randomBytes } from 'node:crypto'
import { writeFileSync } from 'node:fs'
import { userInfo } from 'node:os'
import { Pool } from 'pg'
import * as actors from '../../src/api/services/queen-actors'
import {
  createActorSystem,
  type Pid,
} from '../../src/api/services/queen-actors'
import { createMemoryNet } from '../../src/api/services/queen-actors-net'
import {
  createPgLink,
  encodeMail,
  type PgLink,
} from '../../src/api/services/queen-actors-pg'
import { runPgMigrations } from '../../src/lib/db/pg-migrate'
import { createQueenPool, queenSchema } from '../../src/lib/db/queen-pool'

const URL_KEY = 'TRIOS_PG_TEST_URL'
const adminUrl = () =>
  process.env[URL_KEY] ??
  `postgres://${userInfo().username}@127.0.0.1:5432/postgres`

async function scratchDatabase() {
  const name = `queen_chaos_${randomBytes(6).toString('hex')}`
  const admin = new Pool({ connectionString: adminUrl(), max: 1 })
  await admin.query(`CREATE DATABASE ${name}`)
  const url = new URL(adminUrl())
  url.pathname = `/${name}`
  const fresh = new Pool({ connectionString: url.toString(), max: 1 })
  await fresh.query(`CREATE SCHEMA IF NOT EXISTS ${queenSchema()}`)
  await fresh.end()
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
  while (!cond() && Date.now() < end) await sleep(25)
}
const pct = (xs: number[], p: number) => {
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((p / 100) * s.length))]
}

function faulty(
  pool: Pool,
  fault: (sql: string, rows: number) => 'after' | 'before' | null,
) {
  const query = pool.query.bind(pool) as (
    sql: unknown,
    params?: unknown,
  ) => Promise<{ rows: unknown[] }>
  ;(pool as unknown as { query: unknown }).query = async (
    sql: unknown,
    params?: unknown,
  ) => {
    const text =
      typeof sql === 'string' ? sql : String((sql as { text?: string }).text)
    if (fault(text, -1) === 'before')
      throw new Error('connection lost before the statement (injected)')
    const result = await query(sql, params)
    if (fault(text, result.rows?.length ?? 0) === 'after')
      throw new Error('connection lost after the server ran it (injected)')
    return result
  }
  return pool
}
const isRead = (sql: string) =>
  sql.includes('FROM queen_actor_mail') &&
  !sql.includes('GROUP BY') &&
  /^\s*(WITH m AS|SELECT)/.test(sql)
const isAck = (sql: string) =>
  sql.includes('DELETE FROM queen_actor_mail') && sql.includes('ANY(')

const results: Record<string, number | string> = {}

describe('PgLink chaos: the same script before and after netlink.t27', () => {
  let scratch: { url: string; drop: () => Promise<void> } | null = null
  const pools: Pool[] = []
  const links: PgLink[] = []
  const children: Array<ReturnType<typeof Bun.spawn>> = []
  const previousUrl = process.env.DATABASE_URL

  beforeEach(async () => {
    scratch = await scratchDatabase()
    process.env.DATABASE_URL = scratch.url
    await runPgMigrations()
  })
  afterEach(async () => {
    for (const c of children.splice(0)) {
      c.kill(9)
      await c.exited
    }
    for (const l of links.splice(0)) await l.stop().catch(() => undefined)
    for (const p of pools.splice(0)) await p.end().catch(() => undefined)
    await scratch?.drop()
    scratch = null
    if (previousUrl === undefined) delete process.env.DATABASE_URL
    else process.env.DATABASE_URL = previousUrl
  })
  afterAll(() => {
    const lines = ['| measure | value |', '|---|---|']
    for (const [k, v] of Object.entries(results)) lines.push(`| ${k} | ${v} |`)
    console.log(`\n## PgLink chaos\n${lines.join('\n')}`)
    const out = process.env.QUEEN_BENCH_OUT
    if (out) writeFileSync(out, JSON.stringify(results, null, 2))
  })

  const url = () => (scratch as { url: string }).url
  const node = async (n: number, wrap?: (pool: Pool) => Pool) => {
    const base = createQueenPool(url())
    pools.push(base)
    const pool = wrap ? wrap(base) : base
    const link = await createPgLink(pool, n, { pollMs: 200 })
    links.push(link)
    return { pool, link, sys: createActorSystem(undefined, { node: n, link }) }
  }
  const hasColumn = async (table: string, column: string) => {
    const p = createQueenPool(url())
    try {
      const r = await p.query(
        `SELECT 1 FROM information_schema.columns
          WHERE table_schema = $1 AND table_name = $2 AND column_name = $3`,
        [queenSchema(), table, column],
      )
      return r.rows.length > 0
    } finally {
      await p.end()
    }
  }
  const child = async (env: Record<string, string>) => {
    const proc = Bun.spawn(
      [
        'bun',
        'run',
        new URL('./fixtures/queen-node-proc.ts', import.meta.url).pathname,
      ],
      {
        env: { ...process.env, QUEEN_NODE_URL: url(), ...env },
        stdout: 'pipe',
        stderr: 'ignore',
      },
    )
    children.push(proc)
    const lines: Array<Record<string, unknown>> = []
    const reader = proc.stdout.getReader()
    let buf = ''
    ;(async () => {
      for (;;) {
        const { value, done } = await reader.read()
        if (done) return
        buf += new TextDecoder().decode(value)
        let i = buf.indexOf('\n')
        while (i >= 0) {
          lines.push(JSON.parse(buf.slice(0, i)) as Record<string, unknown>)
          buf = buf.slice(i + 1)
          i = buf.indexOf('\n')
        }
      }
    })().catch(() => undefined)
    await until(() => lines.some((l) => l.ready), 20_000)
    const ready = lines.find((l) => l.ready) as Record<string, string>
    return { proc, lines, ready }
  }

  it('S1: mail when the receiver loses answers mid-delivery', async () => {
    const N = 200
    let readFaults = 0
    let ackFaults = 0
    let reads = 0
    let acks = 0
    const a = await node(1)
    const b = await node(2, (pool) =>
      faulty(pool, (sql, rows) => {
        if (rows === -1) {
          // every second acknowledgement is lost before it runs, five times
          if (isAck(sql) && acks++ % 2 === 0 && ackFaults < 5) {
            ackFaults++
            return 'before'
          }
          return null
        }
        // every second read that finds rows loses its answer, ten times
        if (isRead(sql) && rows > 0 && reads++ % 2 === 0 && readFaults < 10) {
          readFaults++
          return 'after'
        }
        return null
      }),
    )
    await until(() => a.link.up(2) && b.link.up(1), 12_000)
    const got: number[] = []
    const far = b.sys.spawn<number>({
      name: 'far',
      receive: (n) => void got.push(n),
    })
    for (let i = 0; i < N; i++) a.sys.send(far, i)
    await until(() => new Set(got).size >= N, 20_000)
    await sleep(2000)
    const unique = new Set(got).size
    results['S1 messages sent'] = N
    results['S1 read answers lost / acks lost'] = `${readFaults} / ${ackFaults}`
    results['S1 mail lost'] = N - unique
    results['S1 handled twice'] = got.length - unique
    expect(readFaults).toBeGreaterThan(0)
  }, 60_000)

  it('S1b: a row that does not decode, in the middle of a batch', async () => {
    const withInc = await hasColumn('queen_actor_mail', 'to_inc')
    const a = await node(1)
    const b = await node(2)
    await until(() => a.link.up(2) && b.link.up(1), 12_000)
    const got: number[] = []
    const far = b.sys.spawn<number>({
      name: 'far',
      receive: (n) => void got.push(n),
    })
    const incOf = (actors as unknown as { incOf?: (p: Pid) => number }).incOf
    const toInc = withInc && incOf ? incOf(far) : 0
    // twenty rows and a broken one, committed at once: one batch
    const c = await (a.pool as Pool).connect()
    try {
      await c.query('BEGIN')
      const put = (body: string) =>
        withInc
          ? c.query(
              'INSERT INTO queen_actor_mail (node, to_inc, body) VALUES (2, $1, $2)',
              [toInc, body],
            )
          : c.query(
              'INSERT INTO queen_actor_mail (node, body) VALUES (2, $1)',
              [body],
            )
      for (let i = 0; i < 10; i++)
        await put(encodeMail({ kind: 'send', to: far, msg: i }))
      await put('{"kind":"send","to":')
      for (let i = 10; i < 20; i++)
        await put(encodeMail({ kind: 'send', to: far, msg: i }))
      await c.query('COMMIT')
    } finally {
      c.release()
    }
    await until(() => got.length >= 20, 6000)
    await sleep(1500)
    results['S1b mail lost behind one bad row (of 20)'] = 20 - new Set(got).size
    expect(true).toBe(true)
  }, 60_000)

  it('S3: pids and mail across five restarts of one node', async () => {
    const a = await node(1)
    let prev: { pid: Pid; link: PgLink } | null = null
    let reused = 0
    let stale = 0
    let sentToOld = 0
    for (let k = 0; k < 6; k++) {
      if (prev) {
        // the process ends without a word; its last pid still gets mail
        await prev.link.stop()
        for (let i = 0; i < 5; i++) {
          a.sys.send(prev.pid, i)
          sentToOld++
        }
        await sleep(300)
      }
      const b = await node(2)
      await until(() => a.link.up(2), 12_000)
      const got: unknown[] = []
      const x = b.sys.spawn<unknown>({
        name: 'x',
        receive: (m) => void got.push(m),
      })
      if (prev && x === prev.pid) reused++
      await sleep(1500)
      stale += got.length
      prev = { pid: x, link: b.link }
    }
    results['S3 restarts'] = 5
    results['S3 pid reused'] = reused
    results['S3 old mail delivered to the new incarnation'] =
      `${stale} of ${sentToOld}`
    expect(sentToOld).toBe(25)
  }, 120_000)

  it('S4: false node-down while one turn holds the loop', async () => {
    const a = await node(1)
    let counting = false
    let downs = 0
    a.link.onNodeDown((n: number) => {
      if (counting && n === 2) downs++
    })
    const holds = [10, 15, 15, 20, 20, 25]
    const byHold: string[] = []
    const seenInc = (a.link as { incarnationOf?: (n: number) => number })
      .incarnationOf
    for (const h of holds) {
      const c = await child({ QUEEN_NODE: '2' })
      await until(() => a.link.up(2), 15_000)
      // with incarnations, wait until the peer has read this one: the last
      // child's incarnation going down is not this hold's doing
      const inc = (c.ready as unknown as { inc?: number }).inc
      if (inc !== undefined && seenInc)
        await until(() => seenInc(2) === inc, 15_000)
      await sleep(1000)
      const before = downs
      counting = true
      a.sys.send(BigInt(c.ready.spin) as Pid, h * 1000)
      await sleep(h * 1000 + 8000)
      counting = false
      byHold.push(`${h}s:${downs - before}`)
      c.proc.kill(9)
      await c.proc.exited
    }
    results['S4 holds run (s)'] = holds.join(', ')
    results['S4 false node-down'] = `${downs} (${byHold.join(' ')})`
    expect(byHold.length).toBe(holds.length)
  }, 300_000)

  it('S5: a frozen node, after its peers saw it down', async () => {
    const a = await node(1)
    const ticks: number[] = []
    let downAt = 0
    a.link.onNodeDown((n: number) => {
      if (n === 2 && !downAt) downAt = Date.now()
    })
    const sink = a.sys.spawn<number>({
      name: 'sink',
      receive: () => void ticks.push(Date.now()),
    })
    const c = await child({ QUEEN_NODE: '2', QUEEN_NODE_SINK: String(sink) })
    await until(() => ticks.length >= 4, 15_000)
    const pid = c.proc.pid
    process.kill(pid, 'SIGSTOP')
    const frozeAt = Date.now()
    await until(() => downAt > 0, 40_000)
    await sleep(Math.max(0, frozeAt + 30_000 - Date.now()))
    process.kill(pid, 'SIGCONT')
    const thawAt = Date.now()
    await sleep(10_000)
    const after = ticks.filter((t) => t > downAt)
    const fence = c.lines.find((l) => l.fenced) as { at?: number } | undefined
    results['S5 frozen for (s)'] = Math.round((thawAt - frozeAt) / 1000)
    results['S5 peer saw it down after (s)'] = Math.round(
      (downAt - frozeAt) / 1000,
    )
    results['S5 messages from it after it was seen down'] = after.length
    results['S5 seen up again after the thaw'] = a.link.up(2) ? 'yes' : 'no'
    results['S5 it fenced itself, ms after the thaw'] =
      fence?.at !== undefined ? fence.at - thawAt : 'never'
    expect(downAt).toBeGreaterThan(0)
  }, 120_000)

  it('S6: what a message costs', async () => {
    // over PostgreSQL: sequential round trips between two nodes. Wall time
    // moves with the machine's load; the statements each message costs and
    // the CPU this process spends on it do not, so both are counted too.
    let mailStatements = 0
    const counting = (pool: Pool) =>
      faulty(pool, (sql, rows) => {
        if (rows === -1 && sql.includes('queen_actor_mail'))
          if (!sql.includes('GROUP BY')) mailStatements++
        return null
      })
    const a = await node(1, counting)
    const b = await node(2, counting)
    await until(() => a.link.up(2) && b.link.up(1), 12_000)
    const echo = b.sys.spawn<{ from: Pid; n: number }>({
      name: 'echo',
      receive: (m, self) => void b.sys.send(m.from, m.n, self),
    })
    const N = 200
    const rtt: number[] = []
    let t0 = 0
    let done: () => void = () => {}
    const finished = new Promise<void>((r) => {
      done = r
    })
    const pinger: Pid = a.sys.spawn<number>({
      name: 'ping',
      receive: (n, self) => {
        rtt.push(performance.now() - t0)
        if (n + 1 >= N) return done()
        t0 = performance.now()
        a.sys.send(echo, { from: self, n: n + 1 }, self)
      },
    })
    await sleep(500)
    const statementsBefore = mailStatements
    const cpu0 = process.cpuUsage()
    t0 = performance.now()
    a.sys.send(echo, { from: pinger, n: 0 })
    await Promise.race([finished, sleep(120_000)])
    const cpu = process.cpuUsage(cpu0)
    results['S6 round trips over PostgreSQL'] = rtt.length
    results['S6 round trip p50 / p99 (ms)'] =
      `${pct(rtt, 50).toFixed(1)} / ${pct(rtt, 99).toFixed(1)}`
    results['S6 mail statements per round trip'] = (
      (mailStatements - statementsBefore) /
      rtt.length
    ).toFixed(2)
    results['S6 CPU of both nodes per round trip (ms)'] = (
      (cpu.user + cpu.system) /
      1000 /
      rtt.length
    ).toFixed(2)
    // in process: a ring of 1000 linked actors, 100 000 messages
    const perMessage: number[] = []
    const cpuPerMessage: number[] = []
    for (let run = 0; run < 5; run++) {
      const net = createMemoryNet(actors.realClock)
      const sys = createActorSystem(actors.realClock, {
        node: 0,
        link: net.link(0),
      })
      const R = 1000
      const HOPS = 100_000
      const ring: Pid[] = []
      let left = HOPS
      let fin: () => void = () => {}
      const over = new Promise<void>((r) => {
        fin = r
      })
      for (let i = 0; i < R; i++)
        ring.push(
          sys.spawn<number>({
            name: `r${i}`,
            receive: (hop, self) => {
              left--
              if (left === 0) fin()
              else sys.send(ring[(i + 1) % R], hop + 1, self)
            },
          }),
        )
      const c0 = process.cpuUsage()
      const s = performance.now()
      sys.send(ring[0], 0)
      await over
      perMessage.push(((performance.now() - s) * 1000) / HOPS)
      const c = process.cpuUsage(c0)
      cpuPerMessage.push((c.user + c.system) / HOPS)
    }
    results['S6 linked ring, us per message (median of 5)'] = pct(
      perMessage,
      50,
    ).toFixed(2)
    results['S6 linked ring, CPU us per message (median of 5)'] = pct(
      cpuPerMessage,
      50,
    ).toFixed(2)
    expect(rtt.length).toBeGreaterThan(0)
  }, 300_000)
})
