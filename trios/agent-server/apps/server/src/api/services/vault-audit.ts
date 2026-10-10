/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's audit (gHashTag/trios#1759, design rule 6): one event per act,
 * on the existing events bus (queen_event_log, written by appendEvent), on a
 * stream of its own (audit.t27 AUDIT_STREAM), so the Queen's board never
 * reads it and its kinds never meet the control card's.
 *
 * WHAT AN EVENT MAY HOLD is audit.t27's: every key must be one of
 * AUDIT_FIELDS and allowed for the kind (event_code); every text must be a
 * token (text_ok), a hash short hex (hash_ok); a number must be a whole
 * number. An event that fails is not written, and the act it records does not
 * happen (policy.t27 deliver_code): checkEvent throws before any I/O, and the
 * caller has delivered nothing yet.
 *
 * WHERE: the bus when the process has a database (the server's DATABASE_URL,
 * or TRIOS_VAULT_DATABASE_URL for the owner's CLI). The owner's own machine
 * may have none -- the bus lives on the Railway account the vault is meant to
 * outlive -- so there the same checked events go to TRIOS_VAULT_DIR/audit.jsonl
 * (mode 0600) instead. Tests use the memory sink.
 */

import { appendFile, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import type { Pool } from 'pg'
import { appendEvent } from './queen-control'
import { readAfter } from './queen-events'
import {
  AUDIT_FIELDS,
  AUDIT_STREAM,
  EVC_OK,
  EVENT_NAMES,
} from './queen-vault-audit-card.gen'
import { eventCode, hashOk, textOk } from './vault-cards'

export type AuditValue = string | number | string[]
export type AuditPayload = Record<string, AuditValue>

export interface AuditRow {
  seq: number
  kind: number
  name: string
  at: string
  payload: AuditPayload
}

export interface AuditSink {
  append(kind: number, payload: AuditPayload): Promise<number>
  read(after: number, limit: number): Promise<AuditRow[]>
}

export class AuditRefused extends Error {}

const HASH_FIELDS = new Set(['ct', 'req', 'reason_sha'])
const FIELDS = AUDIT_FIELDS as readonly string[]

/** Refuse an event audit.t27 does not admit; return it unchanged otherwise. */
export function checkEvent(kind: number, payload: AuditPayload): AuditPayload {
  let mask = 0
  for (const [key, value] of Object.entries(payload)) {
    const bit = FIELDS.indexOf(key)
    if (bit < 0) throw new AuditRefused(`audit: ${key} is not an audit field`)
    mask |= 1 << bit
    if (typeof value === 'number') {
      if (!Number.isSafeInteger(value) || value < 0)
        throw new AuditRefused(`audit: ${key} is not a whole number`)
      continue
    }
    const texts = Array.isArray(value) ? value : [value]
    for (const t of texts) {
      const ok =
        typeof t === 'string' && (HASH_FIELDS.has(key) ? hashOk(t) : textOk(t))
      if (!ok) throw new AuditRefused(`audit: a ${key} text is not a token`)
    }
  }
  const c = typeof payload.code === 'number' ? payload.code : 0
  const verdict = eventCode(kind, mask >>> 0, c)
  if (verdict !== EVC_OK)
    throw new AuditRefused(
      `audit: event ${EVENT_NAMES[kind] ?? kind} refused (${verdict})`,
    )
  return payload
}

const rowOf = (
  seq: number,
  kind: number,
  at: string,
  payload: AuditPayload,
): AuditRow => ({
  seq,
  kind,
  name: EVENT_NAMES[kind] ?? 'unknown',
  at,
  payload,
})

/** The bus: queen_event_log, stream AUDIT_STREAM. */
export function busAuditSink(pool: Pool): AuditSink {
  return {
    // async, so a refused event is a rejected promise and never a throw
    async append(kind, payload) {
      return appendEvent(pool, AUDIT_STREAM, kind, checkEvent(kind, payload))
    },
    async read(after, limit) {
      const rows = await readAfter(pool, AUDIT_STREAM, after, limit)
      return rows.map((r) =>
        rowOf(r.seq, r.kind, String(r.at), r.payload as AuditPayload),
      )
    },
  }
}

/** The owner's machine without a database: one JSON line per event. */
export function fileAuditSink(dir: string): AuditSink {
  const path = join(dir, 'audit.jsonl')
  let seq = -1
  const all = async (): Promise<AuditRow[]> => {
    let text = ''
    try {
      text = await readFile(path, 'utf8')
    } catch (error) {
      if ((error as { code?: string }).code !== 'ENOENT') throw error
    }
    return text
      .split('\n')
      .filter((l) => l !== '')
      .map((l) => JSON.parse(l) as AuditRow)
  }
  return {
    async append(kind, payload) {
      checkEvent(kind, payload)
      if (seq < 0) seq = (await all()).length
      seq++
      const row = rowOf(seq, kind, new Date().toISOString(), payload)
      await appendFile(path, `${JSON.stringify(row)}\n`, { mode: 0o600 })
      return seq
    },
    async read(after, limit) {
      return (await all()).filter((r) => r.seq > after).slice(0, limit)
    },
  }
}

/** In memory, for tests and benches: the same checks, the same numbering. */
export function memoryAuditSink(): AuditSink & {
  rows: AuditRow[]
  fail: boolean
} {
  const sink = {
    rows: [] as AuditRow[],
    /** Set to make every append throw, as a database that is down would. */
    fail: false,
    async append(kind: number, payload: AuditPayload) {
      if (sink.fail) throw new Error('the audit sink is down')
      checkEvent(kind, payload)
      const seq = sink.rows.length + 1
      sink.rows.push(
        rowOf(
          seq,
          kind,
          new Date().toISOString(),
          JSON.parse(JSON.stringify(payload)),
        ),
      )
      return seq
    },
    async read(after: number, limit: number) {
      return sink.rows.filter((r) => r.seq > after).slice(0, limit)
    },
  }
  return sink
}
