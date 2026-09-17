/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The one place a PostgreSQL pool is built, and the one place a schema is named.
 *
 * WHAT WENT WRONG. Nothing in this repository ever said which schema the Queen's
 * tables live in. Twenty call sites wrote `new Pool({connectionString: url})`,
 * every statement was unqualified, and the whole system resolved correctly
 * because someone once typed `ALTER ROLE postgres SET search_path = trios` into
 * a console. That sentence is not in any file here. It is a production setting
 * the code depends on and cannot see.
 *
 * The cost was already on the disk when this was written: `public` holds six
 * tables named `queen_dispatch`, `queen_issues`, `queen_lease`, `queen_registry`,
 * `queen_tick` and `queen_transcript`, every one of them empty and every one of
 * them a column or twelve short of its `trios` twin. They are what the boot
 * migration below built the day the role's search_path was still `public`. They
 * are not clutter - they are decoys. A connection that arrives with a different
 * search_path (another role, a psql session, a restored snapshot, a pooler that
 * does not carry role settings) finds `public.queen_dispatch`, reads zero rows,
 * and reports an idle swarm. The query that should have failed loudly answers
 * politely with nothing, which is this repository's oldest defect wearing a new
 * hat: EMPTY IS NOT ABSENT.
 *
 * So the schema is named here, on every connection, as a plain `SET search_path`
 * rather than a startup `options` parameter - a statement every pooler forwards,
 * where the startup parameter is exactly what a pooler is free to drop.
 *
 * The default is `trios` alone, which is byte-for-byte what the live role
 * resolves to today: this module makes an invisible setting visible without
 * changing a single resolution. `public` is deliberately NOT in the fallback
 * list. A fallback is how a decoy gets read.
 */

import { Pool, type PoolConfig } from 'pg'
import { logger } from '../logger'

/**
 * The schema every Queen table is in.
 *
 * Overridable because a test database is entitled to its own namespace, and an
 * override is a decision somebody made rather than a setting nobody can see.
 * A value with anything but letters, digits and underscores is refused: this
 * string is interpolated into SQL, and an identifier read from the environment
 * is an injection site if it is not.
 */
export function queenSchema(): string {
  const raw = (process.env.QUEEN_DB_SCHEMA ?? 'trios').trim()
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(raw)) {
    throw new Error(
      `QUEEN_DB_SCHEMA is not a plain identifier: ${JSON.stringify(raw)}`,
    )
  }
  return raw
}

/** Neon terminates TLS with a certificate this driver will not verify. */
function sslFor(databaseUrl: string): PoolConfig['ssl'] {
  return databaseUrl.includes('neon.tech')
    ? { rejectUnauthorized: false }
    : undefined
}

/**
 * Build a pool whose every connection has already been told where to look.
 *
 * `pg` runs a `connect` listener's queries ahead of whatever the borrower asked
 * for on that client, so the search_path is set before the first real statement
 * and no caller has to remember anything.
 *
 * A failure here is logged and NOT thrown. Throwing inside this listener would
 * take down a pool that is otherwise working, and the statement that follows
 * will fail on its own terms - with the table name in the error - which is a
 * far better diagnosis than a dead pool.
 */
export function createQueenPool(
  databaseUrl: string,
  extra: Omit<PoolConfig, 'connectionString' | 'ssl'> = {},
): Pool {
  const schema = queenSchema()
  const pool = new Pool({
    ...extra,
    connectionString: databaseUrl,
    ssl: sslFor(databaseUrl),
  })

  pool.on('connect', (client) => {
    client.query(`SET search_path TO ${schema}`).catch((error: unknown) => {
      logger.error('search_path could not be pinned on a new connection', {
        schema,
        error: error instanceof Error ? error.message : String(error),
      })
    })
  })

  return pool
}
