/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The link between nodes on different machines (actors.t27 section 9,
 * t27#7900): the store is the wire.
 *
 * - LEASE. Every NODE_HEARTBEAT_SECONDS a node renews its row in
 *   queen_actor_node and reads everyone's age. The card decides from the age
 *   who is up (node_up). A node seen up and then down fires onNodeDown, and the
 *   runtime turns that into X_NOCONNECTION for every monitor across it.
 * - MAIL. A send to another node is one row in queen_actor_mail, followed by
 *   pg_notify on that node's channel. The node LISTENs, and also polls each
 *   second in case a notification was lost. It deletes the rows it reads, in
 *   id order. Rows from one process are inserted one at a time, so the order
 *   of each sender-receiver pair holds (section 2's only promise).
 * - Nothing here decides anything. Every decision is the card's.
 */

import { hostname } from 'node:os'
import type { Pool, PoolClient } from 'pg'
import { QUEEN_ACTORS_SQL } from '../../lib/db/pg-migrate'
import { logger } from '../../lib/logger'
import type { Mail, NodeLink } from './queen-actors'
import { NODE_HEARTBEAT_SECONDS } from './queen-actors-card.gen'
import { loadCardWasm, u32 } from './queen-card-wasm'

const nodeUp = (age: number): boolean =>
  loadCardWasm('queen/actors.wasm').call('node_up', u32(age)) !== 0

/** JSON with bigints (pids) kept exact. */
export const encodeMail = (mail: Mail): string =>
  JSON.stringify(mail, (_k, v) =>
    typeof v === 'bigint' ? { $big: v.toString() } : v,
  )
export const decodeMail = (text: string): Mail =>
  JSON.parse(text, (_k, v) =>
    v &&
    typeof v === 'object' &&
    typeof v.$big === 'string' &&
    Object.keys(v).length === 1
      ? BigInt(v.$big)
      : v,
  ) as Mail

export interface PgLink extends NodeLink {
  stop(): Promise<void>
}

export async function createPgLink(
  pool: Pool,
  node: number,
  options: { pollMs?: number; host?: string } = {},
): Promise<PgLink> {
  await pool.query(QUEEN_ACTORS_SQL)
  const channel = `queen_actor_mail_${node}`
  const mailHandlers: Array<(m: Mail) => void> = []
  const downHandlers: Array<(n: number) => void> = []
  const upNow = new Set<number>([node])
  let stopped = false

  const beat = async () => {
    await pool.query(
      `INSERT INTO queen_actor_node (node, host, heartbeat_at)
         VALUES ($1, $2, now())
       ON CONFLICT (node) DO UPDATE SET host = $2, heartbeat_at = now()`,
      [node, options.host ?? hostname()],
    )
    const rows = await pool.query(
      `SELECT node, EXTRACT(EPOCH FROM now() - heartbeat_at)::float8 AS age
         FROM queen_actor_node`,
    )
    for (const r of rows.rows as Array<{ node: number; age: number }>) {
      const n = Number(r.node)
      if (n === node) continue
      const up = nodeUp(Number(r.age))
      if (up) upNow.add(n)
      else if (upNow.delete(n)) for (const h of downHandlers) h(n)
    }
  }

  let draining = false
  const drain = async () => {
    if (draining || stopped) return
    draining = true
    try {
      for (;;) {
        const got = await pool.query(
          `WITH m AS (
             SELECT id FROM queen_actor_mail WHERE node = $1
              ORDER BY id LIMIT 500 FOR UPDATE SKIP LOCKED)
           DELETE FROM queen_actor_mail q USING m WHERE q.id = m.id
           RETURNING q.id, q.body`,
          [node],
        )
        const rows = (got.rows as Array<{ id: string; body: string }>).sort(
          (a, b) => Number(BigInt(a.id) - BigInt(b.id)),
        )
        for (const row of rows) {
          const mail = decodeMail(row.body)
          for (const h of mailHandlers) h(mail)
        }
        if (rows.length < 500) break
      }
    } catch (error) {
      logger.warn('Queen actor node could not read its mail', {
        node,
        error: error instanceof Error ? error.message : String(error),
      })
    } finally {
      draining = false
    }
  }

  // one sender's rows go in one at a time, so their ids keep its order
  let sending: Promise<unknown> = Promise.resolve()
  const carry = (to: number, mail: Mail) => {
    const body = encodeMail(mail)
    sending = sending
      .then(() =>
        pool.query(
          `WITH i AS (INSERT INTO queen_actor_mail (node, body) VALUES ($1, $2))
           SELECT pg_notify('queen_actor_mail_' || $1::text, '')`,
          [to, body],
        ),
      )
      .catch((error) =>
        logger.warn('Queen actor node could not send mail', {
          node,
          to,
          error: error instanceof Error ? error.message : String(error),
        }),
      )
  }

  let listener: PoolClient | null = null
  try {
    listener = await pool.connect()
    listener.on('notification', () => void drain())
    await listener.query(`LISTEN ${channel}`)
  } catch (error) {
    listener?.release()
    listener = null
    logger.warn('Queen actor node listens by polling only', {
      node,
      error: error instanceof Error ? error.message : String(error),
    })
  }

  await beat()
  const heartbeat = setInterval(() => {
    beat().catch((error) =>
      logger.warn('Queen actor node could not renew its lease', {
        node,
        error: error instanceof Error ? error.message : String(error),
      }),
    )
  }, NODE_HEARTBEAT_SECONDS * 1000)
  const poll = setInterval(() => void drain(), options.pollMs ?? 1000)
  void drain()

  return {
    node,
    up: (n) => upNow.has(n),
    carry,
    onMail: (h) => {
      mailHandlers.push(h)
    },
    onNodeDown: (h) => {
      downHandlers.push(h)
    },
    stop: async () => {
      stopped = true
      clearInterval(heartbeat)
      clearInterval(poll)
      await sending.catch(() => undefined)
      if (listener) {
        await listener.query(`UNLISTEN ${channel}`).catch(() => undefined)
        listener.release()
      }
    },
  }
}
