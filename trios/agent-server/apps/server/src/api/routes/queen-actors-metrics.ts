/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * What the actor runtime counted (gHashTag/t27 specs/queen/telemetry.t27,
 * gHashTag/trios#1712). Read-only, and only while the reviewer runs as actors
 * with TRIOS_QUEEN_ACTORS_TELEMETRY=on; otherwise `enabled: false`.
 *
 * `GET /metrics`   counts, histograms and gauges per actor kind, supervisors,
 *                  the deepest mailboxes (depth only, never a message), the
 *                  last threshold events and sampled message records.
 * `GET /decisions?limit=N`  the newest decision-log records (card, fn, args,
 *                  result, kind, UTC ms, n), u64s as strings, for a replay
 *                  against the pinned cards (replayDecisions).
 * No issue title, no message, no key: numbers, kinds and card names only.
 */

import { Hono } from 'hono'
import { liveActorTelemetry } from '../services/queen-actors-telemetry'
import { DLOG_FIELDS, DLOG_RING } from '../services/queen-telemetry-card.gen'

const noStore = { 'Cache-Control': 'no-store' }
const off = { enabled: false, flag: 'TRIOS_QUEEN_ACTORS_TELEMETRY' }

export function createQueenActorsRoute() {
  return new Hono()
    .get('/metrics', (c) => {
      const t = liveActorTelemetry()
      return c.json(t ? t.snapshot() : off, 200, noStore)
    })
    .get('/decisions', (c) => {
      const t = liveActorTelemetry()
      if (!t) return c.json(off, 200, noStore)
      const raw = c.req.query('limit')
      const limit =
        raw !== undefined && /^\d{1,6}$/.test(raw) ? Number(raw) : DLOG_RING
      return c.json(
        { enabled: true, fields: DLOG_FIELDS, records: t.decisions(limit) },
        200,
        noStore,
      )
    })
}
