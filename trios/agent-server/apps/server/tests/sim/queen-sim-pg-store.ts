/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE STORE AS THE WIRE, SIMULATED (trios#1712 items 5 and 6).
 *
 * The memory net loses mail to a dead node at once and fires its node-down at
 * the TTL whatever happens after; it has no store, so the two PgLink defects
 * read out of the code (the competitor study, item 6) cannot happen on it:
 *   - rows are deleted (DELETE ... RETURNING) before their handlers run, so a
 *     process that dies in between loses them, counted nowhere;
 *   - a restarted node starts its pids from zero, so mail still in the table
 *     for its last incarnation reaches whoever holds those pids now.
 * This drives the REAL createPgLink against an in-memory store that answers
 * exactly the statements it sends, on the virtual clock: each statement takes
 * effect when it is sent and its answer arrives a seeded latency later, as a
 * server's does. A node that dies receives no answer, and its timers stop.
 *
 * createPgLink keeps its heartbeat and its poll on setInterval. For the run,
 * setInterval and clearInterval are the virtual clock's; the node an interval
 * belongs to is the node whose query answered last, because createPgLink
 * starts both right after its first heartbeat's answer.
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
  body: string
}

export function pgStore(world: SimWorld): Transport {
  const clock = world.clock
  const beats = new Map<number, number>()
  let mail: MailRow[] = []
  let nextId = 1
  let answers = 0
  const listeners = new Map<string, Set<() => void>>()
  // which incarnation of a node a pool belongs to; a dead one hears nothing
  const alive = new Map<number, number>()
  let incarnations = 0
  // rows a node took off the table, by wire id, and those whose answer came
  const taken = new Map<number, number>()
  const answered = new Set<number>()
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

  const pool = (node: number): Pool => {
    const me = ++incarnations
    alive.set(node, me)
    // the statement has taken effect; its answer arrives a latency later,
    // or is lost with the connection (answer_lost), or never, to a dead node
    const answer = <T>(
      value: T,
      took: number[] = [],
    ): Promise<{ rows: T; rowCount: number }> =>
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
          for (const w of took) answered.add(w)
          if (lost) {
            world.count('answersLost')
            world.log(
              `store: the answer to node ${node} was lost after its statement took effect${took.length ? ` (it had taken ${took.map((w) => `wire ${w}`).join(', ')})` : ''}`,
              ...took.map((w) => `wire ${w}`),
            )
            reject(new Error('connection reset after commit (simulated)'))
          } else
            resolve({
              rows: value,
              rowCount: Array.isArray(value) ? value.length : 0,
            })
        })
      })
    // the statements createPgLink sends, each answered as Postgres would
    const statements: Array<[RegExp, (params: unknown[]) => Promise<unknown>]> =
      [
        [/CREATE TABLE/i, () => answer([])],
        [
          /INSERT INTO queen_actor_node/,
          (params) => {
            beats.set(Number(params[0]), clock.now())
            return answer([])
          },
        ],
        [
          /FROM queen_actor_node/,
          () =>
            answer(
              [...beats].map(([n, at]) => ({
                node: n,
                age: (clock.now() - at) / 1000,
              })),
            ),
        ],
        [
          /INSERT INTO queen_actor_mail/,
          (params) => {
            const to = Number(params[0])
            mail.push({ id: nextId++, node: to, body: String(params[1]) })
            for (const h of listeners.get(`queen_actor_mail_${to}`) ?? [])
              clock.after(1, () => h())
            return answer([])
          },
        ],
        [/DELETE FROM queen_actor_mail/, (params) => take(Number(params[0]))],
      ]
    // DELETE ... RETURNING: the rows leave the table now, the answer comes later
    const take = (at: number) => {
      const mine = mail.filter((r) => r.node === at).slice(0, 500)
      const ids = new Set(mine.map((r) => r.id))
      mail = mail.filter((r) => !ids.has(r.id))
      const took: number[] = []
      for (const r of mine) {
        const w = wireOf(r.body)
        if (w === undefined) continue
        taken.set(w, at)
        took.push(w)
        world.log(
          `store: node ${at} deleted wire ${w} (row ${r.id})`,
          `wire ${w}`,
        )
      }
      return answer(
        mine.map((r) => ({ id: String(r.id), body: r.body })),
        took,
      )
    }
    const run = (sql: string, params: unknown[] = []) => {
      // a dead process sends nothing and hears nothing
      if (alive.get(node) !== me) return new Promise(() => {})
      const found = statements.find(([re]) => re.test(sql))
      if (!found)
        throw new Error(
          `the simulated store does not know: ${sql.slice(0, 60)}`,
        )
      return found[1](params)
    }
    return {
      query: run,
      connect: async () => ({
        on: (event: string, h: () => void) => {
          if (event !== 'notification') return
          const set = listeners.get(`queen_actor_mail_${node}`) ?? new Set()
          set.add(() => {
            if (alive.get(node) === me) h()
          })
          listeners.set(`queen_actor_mail_${node}`, set)
        },
        query: async () => ({ rows: [] }),
        release: () => {},
      }),
    } as unknown as Pool
  }

  /** Rows a node took and never handed to its runtime. */
  const takenNotHandled = (node?: number) => {
    for (const [w, n] of taken) {
      if (node !== undefined && n !== node) continue
      if (world.onWire(w)) {
        world.fail(
          S.INV_TAKEN_NOT_HANDLED,
          `node ${n} deleted mail ${w} from the store and never handed it to its runtime`,
          `wire ${w}`,
        )
        world.landed(w, false)
      }
      taken.delete(w)
    }
  }

  return {
    link: async (n: number): Promise<NodeLink> => {
      // a boot whose store answer is lost fails, and the host starts the
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
      takenNotHandled(n)
    },
    settle: async () => {
      // a row is handled in the turn its answer arrives; one whose answer came
      // and that is still on the wire at the end of the step never will be
      for (const [w, n] of taken) {
        if (!world.onWire(w)) {
          taken.delete(w)
          answered.delete(w)
        } else if (answered.has(w)) {
          world.fail(
            S.INV_TAKEN_NOT_HANDLED,
            `node ${n} deleted mail ${w} from the store, its answer was lost, and the mail was never handled`,
            `wire ${w}`,
          )
          world.landed(w, false)
          taken.delete(w)
          answered.delete(w)
        }
      }
    },
    stop: async () => {
      takenNotHandled()
      for (const l of links) l.stop().catch(() => undefined)
      for (const [, t] of intervals) t.cancel()
      intervals.clear()
      globalThis.setInterval = realSetInterval
      globalThis.clearInterval = realClearInterval
      logger.setLevel((process.env.LOG_LEVEL as 'info' | undefined) ?? 'info')
    },
  }
}
