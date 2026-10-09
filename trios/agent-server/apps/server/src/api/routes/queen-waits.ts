/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Long waits as rows (gHashTag/t27 specs/queen/waits.t27): what waits for
 * what, a wait a person makes, and a key resolved from outside (a webhook, a
 * person's answer). Guarded like every route that writes:
 *
 *   GET  /queen/waits            what waits now, then what ended last
 *   POST /queen/waits            { name, key?, wakeInSeconds?, expirySeconds?, detail? }
 *   POST /queen/waits/resolve    { key, resolution? }
 *
 * A wait made here belongs to `person:<name>`; nobody is woken by it, its row
 * says when it ended. A resolve makes every row on that key due at once and
 * NOTIFYs the schedulers; the poll would find it anyway.
 * Off unless TRIOS_QUEEN_WAITS=rows: with no scheduler, a row would never end.
 */

import { Hono } from 'hono'
import type { Pool } from 'pg'
import { createQueenPool } from '../../lib/db/queen-pool'
import { queenLeaseDatabaseUrl } from '../services/queen-lease'
import {
  createWait,
  ensureWaitTables,
  listWaits,
  resolveWaitByKey,
  type WaitRow,
  waitsEnabled,
} from '../services/queen-waits'

export interface QueenWaitsDeps {
  pool?: () => Pool | null
  enabled?: () => boolean
  now?: () => number
}

let sharedPool: Pool | null = null

function defaultPool(): Pool | null {
  const url = queenLeaseDatabaseUrl()
  if (!url) return null
  if (!sharedPool) sharedPool = createQueenPool(url)
  return sharedPool
}

function textOf(value: unknown, max: number): string | null {
  if (typeof value !== 'string') return null
  // biome-ignore lint/suspicious/noControlCharactersInRegex: strips control characters
  const clean = value.replace(/[\u0000-\u001f\u007f]/g, ' ').trim()
  return clean ? clean.slice(0, max) : null
}

const seconds = (value: unknown): number | undefined =>
  typeof value === 'number' && Number.isFinite(value) && value >= 0
    ? Math.floor(value)
    : undefined

const shown = (w: WaitRow) => ({
  ...w,
  epoch: w.epoch.toString(),
  wakeAt: w.wakeAt === null ? null : new Date(w.wakeAt).toISOString(),
  expiresAt: new Date(w.expiresAt).toISOString(),
  dueAt: w.dueAt === null ? null : new Date(w.dueAt).toISOString(),
})

export function createQueenWaitsRoute(deps: QueenWaitsDeps = {}) {
  const poolOf = deps.pool ?? defaultPool
  const enabled = deps.enabled ?? (() => waitsEnabled())
  const now = deps.now ?? Date.now
  const off = { error: 'waits are off (TRIOS_QUEEN_WAITS=rows turns them on)' }

  return new Hono()
    .get('/', async (c) => {
      const pool = poolOf()
      if (!pool) return c.json({ error: 'No database configured' }, 503)
      await ensureWaitTables(pool)
      return c.json({ waits: (await listWaits(pool)).map(shown) })
    })
    .post('/', async (c) => {
      if (!enabled()) return c.json(off, 503)
      const pool = poolOf()
      if (!pool) return c.json({ error: 'No database configured' }, 503)
      const body = await c.req.json().catch(() => ({}))
      const name = textOf(body?.name, 80) ?? 'operator'
      const key = textOf(body?.key, 200) ?? undefined
      const detail =
        body?.detail && typeof body.detail === 'object' ? body.detail : {}
      await ensureWaitTables(pool)
      try {
        const row = await createWait(
          pool,
          {
            owner: `person:${name}`,
            key,
            wakeInSeconds: seconds(body?.wakeInSeconds),
            expirySeconds: seconds(body?.expirySeconds),
            detail,
          },
          now(),
        )
        return c.json({ wait: shown(row) }, 201)
      } catch (error) {
        return c.json(
          { error: error instanceof Error ? error.message : String(error) },
          400,
        )
      }
    })
    .post('/resolve', async (c) => {
      if (!enabled()) return c.json(off, 503)
      const pool = poolOf()
      if (!pool) return c.json({ error: 'No database configured' }, 503)
      const body = await c.req.json().catch(() => ({}))
      const key = textOf(body?.key, 200)
      if (!key) return c.json({ error: 'key is required' }, 400)
      await ensureWaitTables(pool)
      const ids = await resolveWaitByKey(
        pool,
        key,
        body?.resolution ?? {},
        now(),
      )
      if (ids.length === 0)
        return c.json({ error: 'no waiting row on that key' }, 404)
      return c.json({ key, resolved: ids })
    })
}
