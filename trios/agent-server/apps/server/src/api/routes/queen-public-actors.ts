/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * What the actors did since a cursor, as anyone may read it (gHashTag/t27
 * specs/queen/actor_events.t27 and events.t27 section 6, trios#1744). The
 * kanban view of the actors (t27#8615) reads this; queen-actor-events.ts says
 * what is public and why.
 *
 * `?since=<seq>&limit=200` - the events of the actors' stream past `since`,
 * paged by events.t27's cursor_read and page_size. No `since` reads from the
 * oldest event kept. `resync: true` means the events right after `since` are
 * gone; `cursor` is where reading resumes.
 *
 * `?replay=mvp` - the card's stand-in trace of the network MVP (t27#8610),
 * through the same projection and the same paging, whatever the flag says:
 * a fixed table from a public spec, read from no store.
 *
 * With TRIOS_QUEEN_ACTOR_EVENTS off (the default) nothing is published, and
 * the answer is an empty page that says so (`on: false`), 200, so a view that
 * polls needs no special case. Pids, kinds, counts, depths, times and task
 * references only - never a message or a card's arguments. The operator's
 * /queen/actors (telemetry and the decision log) stays behind the
 * trusted-origin guard.
 */

import { Hono } from 'hono'
import type { Pool } from 'pg'
import { createQueenPool } from '../../lib/db/queen-pool'
import { logger } from '../../lib/logger'
import {
  type ActorRowSource,
  actorEventsOn,
  liveActorEventWriter,
  OFF_PAGE,
  publicActorsPage,
  replayRows,
  replaySource,
} from '../services/queen-actor-events'
import { pidOf } from '../services/queen-actors'
import {
  cursorRead,
  logBounds,
  pageSize,
  readAfter,
  tokenOk,
} from '../services/queen-events'
import { ACTOR_STREAM } from '../services/queen-events.gen'

let pool: Pool | null = null

const count = (v: string | undefined): number | null => {
  if (v === undefined || !/^\d{1,15}$/.test(v.trim())) return null
  return Number(v.trim())
}

const cards = { cursorRead, pageSize, tokenOk }
const noStore = { 'Cache-Control': 'no-store' }

/** The log's actors stream as a source of rows. */
export const busSource = (p: Pool): ActorRowSource => ({
  bounds: () => logBounds(p, ACTOR_STREAM),
  after: (after, limit) => readAfter(p, ACTOR_STREAM, after, limit),
})

export interface PublicActorsDeps {
  env?: NodeJS.ProcessEnv
  /** The bus, for a test; the Queen's pool otherwise. */
  source?: () => ActorRowSource
}

export function createQueenPublicActorsRoute(deps: PublicActorsDeps = {}) {
  return new Hono().get('/', async (c) => {
    const env = deps.env ?? process.env
    const since = count(c.req.query('since'))
    const limit = count(c.req.query('limit')) ?? 0
    const replay = c.req.query('replay')
    try {
      if (replay !== undefined) {
        if (replay !== 'mvp')
          return c.json({ error: 'replay=mvp is the one trace' }, 400)
        const page = await publicActorsPage(
          replaySource(replayRows((slot, gen) => pidOf(slot, gen))),
          since,
          limit,
          cards,
          'replay',
        )
        return c.json(page, 200, noStore)
      }
      if (!actorEventsOn(env)) return c.json(OFF_PAGE, 200, noStore)
      let source = deps.source?.()
      if (!source) {
        const url = env.DATABASE_URL
        if (!url) return c.json({ error: 'No database configured' }, 503)
        pool ??= createQueenPool(url)
        source = busSource(pool)
      }
      const page = await publicActorsPage(source, since, limit, cards, 'bus')
      const lost = liveActorEventWriter()?.lost()
      return c.json(lost === undefined ? page : { ...page, lost }, 200, noStore)
    } catch (error) {
      logger.warn('Queen public actors could not be read', {
        error: error instanceof Error ? error.message : String(error),
      })
      return c.json({ error: 'The actor events are unavailable' }, 503)
    }
  })
}
