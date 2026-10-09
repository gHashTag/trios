/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A ROLLBACK THAT FAILS (gHashTag/trios#1731). Every transaction in the actor
 * runtime ends a failure with ROLLBACK, and a ROLLBACK can fail too, when the
 * connection died with the statement. The caller must still see the error
 * that caused it, and the connection must still go back to the pool. The
 * store is a stand-in whose statements fail as told.
 */

import { describe, expect, it } from 'bun:test'
import type { Pool } from 'pg'
import { createPgLink } from '../../src/api/services/queen-actors-pg'
import { claimDue, resolveWaitByKey } from '../../src/api/services/queen-waits'

/** A pool of one client: `fails` names the statement that throws first. */
function store(fails: RegExp) {
  const said: string[] = []
  let released = 0
  const client = {
    query: async (sql: string) => {
      said.push(sql.trim().split(/\s+/).slice(0, 2).join(' '))
      if (/^\s*ROLLBACK/.test(sql)) throw new Error('the connection is gone')
      if (fails.test(sql)) throw new Error('the statement failed')
      return { rows: [], rowCount: 0 }
    },
    release: () => {
      released++
    },
  }
  const pool = {
    query: async () => ({ rows: [], rowCount: 0 }),
    connect: async () => client,
    options: {},
  }
  return {
    pool: pool as unknown as Pool,
    said,
    released: () => released,
  }
}

describe('a rollback that fails', () => {
  it("hides nothing in a node's start: the claim's own error is thrown, and its connection goes back", async () => {
    const s = store(/INSERT INTO queen_actor_node/)
    await expect(createPgLink(s.pool, 1)).rejects.toThrow(
      'the statement failed',
    )
    expect(s.said).toEqual(['BEGIN', 'INSERT INTO', 'ROLLBACK'])
    expect(s.released()).toBe(1)
  })

  it("hides nothing in a wait's claim: the select's own error is thrown, and its connection goes back", async () => {
    const s = store(/FOR UPDATE SKIP LOCKED/)
    await expect(claimDue(s.pool, Date.now())).rejects.toThrow(
      'the statement failed',
    )
    expect(s.said).toEqual(['BEGIN', 'SELECT *', 'ROLLBACK'])
    expect(s.released()).toBe(1)
  })

  it("hides nothing in a write on a locked row: the lock's own error is thrown, and its connection goes back", async () => {
    const s = store(/ORDER BY id FOR UPDATE/)
    await expect(
      resolveWaitByKey(s.pool, 'k', { ok: true }, Date.now()),
    ).rejects.toThrow('the statement failed')
    expect(s.said).toEqual(['BEGIN', 'SELECT *', 'ROLLBACK'])
    expect(s.released()).toBe(1)
  })
})
