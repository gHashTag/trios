/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE STORE AS THE WIRE, SIMULATED (trios#1712 items 5 and 6).
 *
 * The memory net stands in for the store; this drives the REAL createPgLink
 * (netlink.t27: incarnations, fenced writes, read - handle - acknowledge)
 * against an in-memory store that answers exactly the statements it sends,
 * on the virtual clock. Each statement takes effect when it is sent and its
 * answer arrives a seeded latency later, as a server's does; the card's
 * answer_lost drops one answer after its statement took effect, as a
 * connection that resets after the commit does. A node that dies receives no
 * answer, and its timers stop. The fences are the ones the SQL carries: a
 * write is admitted only while the writer's row holds its incarnation and its
 * lease is younger than the TTL.
 *
 * Before trios#1714 this model reproduced both defects of item 6 from a
 * seed: a row deleted before it was handled (seed 1188652385, step 463) and a
 * restarted node reissuing its pids while the store held mail for them (seed
 * 3600507402, step 7015). Now it holds the fixed link to the same
 * invariants: mail a node acknowledges must have reached its runtime, or be
 * mail the card expired.
 *
 * createPgLink keeps its heartbeat and its poll on setInterval. For the run,
 * setInterval and clearInterval are the virtual clock's; the node an interval
 * belongs to is the node whose query answered last, because createPgLink
 * starts both right after its first read of the leases. With no database url
 * there is no beat thread: the lease is renewed on the loop, on the clock.
 */

import type { Pool } from 'pg'
import type { NodeLink } from '../../src/api/services/queen-actors'
import {
  createPgLink,
  type PgLink,
} from '../../src/api/services/queen-actors-pg'
import * as S from '../../src/api/services/queen-simulation-card.gen'
import { logger } from '../../src/lib/logger'
import {
  answerLost,
  pickBetween,
  type SimWorld,
  simRoll,
  type Transport,
} from './queen-sim-world'

interface MailRow {
  id: number
  node: number
  toInc: number
  body: string
}

interface NodeRow {
  incarnation: number
  heartbeatAt: number
  host: string
}

type Answer = { rows: unknown[]; rowCount: number }

export function pgStore(world: SimWorld): Transport {
  const clock = world.clock
  const nodes = new Map<number, NodeRow>()
  let mail: MailRow[] = []
  let nextId = 1
  let answers = 0
  const listeners = new Map<number, Set<() => void>>()
  // which process of a node a pool belongs to; a dead one hears nothing
  const alive = new Map<number, number>()
  let processes = 0
  const links: PgLink[] = []

  // the virtual clock's setInterval, owned by the node whose answer came last
  let answering = 0
  const intervals = new Map<number, { node: number; cancel: () => void }>()
  let nextTimer = 1
  const realSetInterval = globalThis.setInterval
  const realClearInterval = globalThis.clearInterval
  const fakeSetInterval = (fn: () => void, ms: number) => {
    const id = nextTimer++
    const node = answering
    const arm = () => {
      const cancel = clock.after(ms, () => {
        if (!intervals.has(id)) return
        arm()
        fn()
      })
      intervals.set(id, { node, cancel })
    }
    arm()
    return id
  }
  const fakeClearInterval = (id: number) => {
    intervals.get(id)?.cancel()
    intervals.delete(id)
  }
  globalThis.setInterval = fakeSetInterval as unknown as typeof setInterval
  globalThis.clearInterval =
    fakeClearInterval as unknown as typeof clearInterval
  // a lost answer makes createPgLink warn, as it should; the world counts
  // them (answersLost) instead of printing hundreds of lines
  logger.setLevel('error')

  const wireOf = (body: string): number | undefined => {
    const m = /"simWire":(\d+)/.exec(body)
    return m ? Number(m[1]) : undefined
  }
  /** write_admitted, as the SQL fence reads it: this incarnation, lease up. */
  const admitted = (node: number, inc: number, ttlSeconds: number) => {
    const row = nodes.get(node)
    return (
      !!row &&
      row.incarnation === inc &&
      row.heartbeatAt > clock.now() - ttlSeconds * 1000
    )
  }
  /** Rows leave the store: each must have reached a runtime, or be expired. */
  const removed = (rows: MailRow[], by: number, why: 'ack' | 'expire') => {
    for (const r of rows) {
      const w = wireOf(r.body)
      if (w === undefined || !world.onWire(w)) continue
      // mail for an incarnation that is over is expired by the card, counted
      const over =
        why === 'expire' || nodes.get(r.node)?.incarnation !== r.toInc
      if (over) {
        world.count('mailExpired')
        world.log(
          `store: wire ${w} expired (for incarnation ${r.toInc})`,
          `wire ${w}`,
        )
        world.landed(w, false)
        continue
      }
      // the answer to the acknowledgement arrives within DB_MAX_MS and its
      // continuation runs at once; a row not handled by then never will be
      clock.after(S.DB_MAX_MS + 1, () => {
        if (!world.onWire(w)) return
        world.fail(
          S.INV_TAKEN_NOT_HANDLED,
          `node ${by} acknowledged mail ${w} (row ${r.id}) and it never reached its runtime`,
          `wire ${w}`,
        )
        world.landed(w, false)
      })
    }
  }

  const pool = (node: number): Pool => {
    const me = ++processes
    alive.set(node, me)
    // the statement has taken effect; its answer arrives a latency later,
    // or is lost with the connection (answer_lost), or never, to a dead node
    const answer = (rows: unknown[], rowCount = rows.length): Promise<Answer> =>
      new Promise((resolve, reject) => {
        const index = answers++
        const ms = pickBetween(
          simRoll(world.seed, S.STREAM_LATENCY, 1_000_000 + index),
          0,
          S.DB_MAX_MS,
        )
        const lost = answerLost(simRoll(world.seed, S.STREAM_STORE, index))
        clock.after(ms, () => {
          if (alive.get(node) !== me) return
          answering = node
          if (lost) {
            world.count('answersLost')
            world.log(
              `store: an answer to node ${node} was lost after its statement took effect`,
            )
            reject(new Error('connection reset after commit (simulated)'))
          } else resolve({ rows, rowCount })
        })
      })
    // the statements createPgLink sends, each answered as Postgres would;
    // the first pattern that matches wins
    const statements: Array<[RegExp, (p: unknown[]) => Promise<Answer>]> = [
      [/CREATE TABLE/i, () => answer([])],
      [
        /INSERT INTO queen_actor_node/,
        (p) => {
          const n = Number(p[0])
          if (!nodes.has(n))
            nodes.set(n, {
              incarnation: 0,
              heartbeatAt: clock.now(),
              host: String(p[1]),
            })
          return answer([])
        },
      ],
      [
        /SELECT incarnation FROM queen_actor_node/,
        (p) =>
          answer([
            { incarnation: String(nodes.get(Number(p[0]))?.incarnation ?? 0) },
          ]),
      ],
      [
        // the renewal: fenced on the incarnation and the lease
        /SET heartbeat_at = clock_timestamp\(\), host = \$3\s+WHERE node = \$1 AND incarnation = \$2/,
        (p) => {
          const n = Number(p[0])
          if (!admitted(n, Number(p[1]), Number(p[3]))) return answer([], 0)
          const row = nodes.get(n) as NodeRow
          row.heartbeatAt = clock.now()
          return answer([], 1)
        },
      ],
      [
        /FROM queen_actor_mail m LEFT JOIN queen_actor_node/,
        () => {
          const groups = new Map<
            string,
            { node: number; toInc: number; rows: number }
          >()
          for (const r of mail) {
            const k = `${r.node}:${r.toInc}`
            const g = groups.get(k) ?? { node: r.node, toInc: r.toInc, rows: 0 }
            g.rows++
            groups.set(k, g)
          }
          return answer(
            [...groups.values()].map((g) => {
              const n = nodes.get(g.node)
              return {
                node: g.node,
                to_inc: String(g.toInc),
                rows: g.rows,
                node_inc: n ? String(n.incarnation) : null,
                age: n ? (clock.now() - n.heartbeatAt) / 1000 : null,
              }
            }),
          )
        },
      ],
      [
        // the leases: every node's incarnation and age (the fenced writes
        // read queen_actor_node too, so this pattern is the exact select)
        /SELECT node, incarnation,\s+EXTRACT/,
        () =>
          answer(
            [...nodes].map(([n, r]) => ({
              node: n,
              incarnation: String(r.incarnation),
              age: (clock.now() - r.heartbeatAt) / 1000,
            })),
          ),
      ],
      [
        // the expiry of mail for an incarnation that is over, fenced
        /DELETE FROM queen_actor_mail\s+WHERE node = \$1 AND to_inc = \$2/,
        (p) => {
          if (!admitted(Number(p[2]), Number(p[3]), Number(p[4])))
            return answer([{ fence: 0, deleted: 0 }])
          const gone = mail.filter(
            (r) => r.node === Number(p[0]) && r.toInc === Number(p[1]),
          )
          mail = mail.filter((r) => !gone.includes(r))
          removed(gone, Number(p[2]), 'expire')
          return answer([{ fence: 1, deleted: gone.length }])
        },
      ],
      [
        // the acknowledgement of rows read and handled, fenced
        /DELETE FROM queen_actor_mail\s+WHERE node = \$1 AND id = ANY/,
        (p) => {
          const n = Number(p[0])
          if (!admitted(n, Number(p[2]), Number(p[3])))
            return answer([{ fence: 0, ids: [] }])
          const ids = new Set((p[1] as unknown[]).map((id) => Number(id)))
          const gone = mail.filter((r) => r.node === n && ids.has(r.id))
          mail = mail.filter((r) => !gone.includes(r))
          removed(gone, n, 'ack')
          return answer([{ fence: 1, ids: gone.map((r) => String(r.id)) }])
        },
      ],
      [
        /SELECT id, to_inc, body FROM queen_actor_mail/,
        (p) =>
          answer(
            mail
              .filter((r) => r.node === Number(p[0]))
              .slice(0, Number(p[1]))
              .map((r) => ({
                id: String(r.id),
                to_inc: String(r.toInc),
                body: r.body,
              })),
          ),
      ],
      [
        // a send: one row and a notification, fenced on the writer
        /INSERT INTO queen_actor_mail/,
        (p) => {
          if (!admitted(Number(p[2]), Number(p[3]), Number(p[5])))
            return answer([{ sent: 0, n: null }])
          const to = Number(p[0])
          mail.push({
            id: nextId++,
            node: to,
            toInc: Number(p[1]),
            body: String(p[4]),
          })
          for (const h of listeners.get(to) ?? []) clock.after(1, () => h())
          return answer([{ sent: 1, n: '' }])
        },
      ],
    ]
    const run = (sql: string, params: unknown[] = []): Promise<Answer> => {
      // a dead process sends nothing and hears nothing
      if (alive.get(node) !== me) return new Promise(() => {})
      const found = statements.find(([re]) => re.test(sql))
      if (!found)
        throw new Error(
          `the simulated store does not know: ${sql.slice(0, 80)}`,
        )
      return found[1](params)
    }
    // a client: the claim's transaction (its incarnation write applies at
    // COMMIT, and is dropped by ROLLBACK), and LISTEN for notifications
    const client = () => {
      let staged: Array<() => void> = []
      return {
        on: (event: string, h: () => void) => {
          if (event !== 'notification') return
          const set = listeners.get(node) ?? new Set()
          set.add(() => {
            if (alive.get(node) === me) h()
          })
          listeners.set(node, set)
        },
        query: (sql: string, params: unknown[] = []) => {
          if (alive.get(node) !== me) return new Promise(() => {})
          if (/^(BEGIN|LISTEN|UNLISTEN)/.test(sql.trim())) return answer([])
          if (sql.trim() === 'ROLLBACK') {
            staged = []
            return answer([])
          }
          if (sql.trim() === 'COMMIT') {
            for (const write of staged) write()
            staged = []
            return answer([])
          }
          if (/SET incarnation = \$2/.test(sql)) {
            staged.push(() => {
              const row = nodes.get(Number(params[0])) as NodeRow
              row.incarnation = Number(params[1])
              row.host = String(params[2])
              row.heartbeatAt = clock.now()
            })
            return answer([], 1)
          }
          return run(sql, params)
        },
        release: () => {},
      }
    }
    return {
      query: run,
      connect: async () => client(),
      options: {},
    } as unknown as Pool
  }

  return {
    link: async (n: number): Promise<NodeLink> => {
      // a start whose store answer is lost fails, and the host starts the
      // process again, as Railway restarts a crashed container
      for (;;) {
        try {
          const link = await createPgLink(pool(n), n, { host: 'sim' })
          links.push(link)
          return link
        } catch {
          world.count('bootRetries')
        }
      }
    },
    crash: (n: number) => {
      // the process dies: no answers, no timers, no listener
      alive.delete(n)
      for (const [id, t] of intervals)
        if (t.node === n) {
          t.cancel()
          intervals.delete(id)
        }
    },
    stop: async () => {
      for (const l of links) l.stop().catch(() => undefined)
      for (const [, t] of intervals) t.cancel()
      intervals.clear()
      globalThis.setInterval = realSetInterval
      globalThis.clearInterval = realClearInterval
      logger.setLevel((process.env.LOG_LEVEL as 'info' | undefined) ?? 'info')
    },
  }
}
