/**
 * @license
 * Copyright 2025 BrowserOS
 *
 * The schema is named in exactly one place, and it is named on every connection.
 *
 * `public` holds six empty tables named `queen_dispatch`, `queen_issues`,
 * `queen_lease`, `queen_registry`, `queen_tick` and `queen_transcript`, each a
 * column or twelve short of its `trios` twin. They were built by the boot
 * migration on a day the role's search_path was still `public`, and they are
 * decoys: a connection that arrives without the right search_path finds one,
 * reads zero rows and reports an idle swarm instead of failing. That is the
 * defect these gates exist to keep out - not the decoys themselves, which are a
 * database cleanup, but the twenty call sites that each built a pool and none of
 * which said where to look.
 */

import { afterEach, describe, expect, it } from 'bun:test'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { createQueenPool, queenSchema } from '../../../src/lib/db/queen-pool'

const SRC = join(import.meta.dir, '..', '..', '..', 'src')
const ACCESSOR = join('lib', 'db', 'queen-pool.ts')

function sourceFiles(dir: string, found: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) sourceFiles(full, found)
    else if (entry.endsWith('.ts')) found.push(full)
  }
  return found
}

describe('queenSchema', () => {
  const saved = process.env.QUEEN_DB_SCHEMA

  afterEach(() => {
    if (saved === undefined) delete process.env.QUEEN_DB_SCHEMA
    else process.env.QUEEN_DB_SCHEMA = saved
  })

  it('defaults to what the live role already resolves to', () => {
    delete process.env.QUEEN_DB_SCHEMA
    // NOT `trios, public`. A fallback to public is how a decoy gets read: the
    // query succeeds, the count is zero, and nothing anywhere says the table it
    // answered from is not the table that holds the work.
    expect(queenSchema()).toBe('trios')
  })

  it('takes an override, because a test database owns its own namespace', () => {
    process.env.QUEEN_DB_SCHEMA = 'trios_test'
    expect(queenSchema()).toBe('trios_test')
  })

  it('refuses anything that is not a plain identifier', () => {
    // This string is interpolated into SQL. An environment variable that is not
    // validated here is an injection site, and "it comes from our own Railway
    // config" is the argument every injection site was shipped with.
    for (const bad of [
      'trios; DROP TABLE queen_dispatch',
      'trios public',
      '"trios"',
      '9trios',
      '',
    ]) {
      process.env.QUEEN_DB_SCHEMA = bad
      expect(() => queenSchema()).toThrow(/not a plain identifier/)
    }
  })
})

describe('createQueenPool', () => {
  it('pins the search_path on every connection it hands out', async () => {
    const asked: string[] = []
    const pool = createQueenPool('postgres://example.invalid/db')
    // The listener is what does the pinning; drive it with a client double
    // rather than a live socket so the gate runs anywhere.
    pool.emit('connect', {
      query: (sql: string) => {
        asked.push(sql)
        return Promise.resolve()
      },
    })
    expect(asked).toEqual(['SET search_path TO trios'])
    await pool.end().catch(() => {})
  })

  it('survives a connection that refuses the statement', async () => {
    // Throwing in this listener would kill a pool that otherwise works. The
    // statement that follows will fail on its own terms, naming the table -
    // which is a better diagnosis than a dead pool.
    const pool = createQueenPool('postgres://example.invalid/db')
    expect(() =>
      pool.emit('connect', {
        query: () => Promise.reject(new Error('permission denied')),
      }),
    ).not.toThrow()
    await pool.end().catch(() => {})
  })
})

describe('no file builds its own pool around the accessor', () => {
  it('finds `new Pool(` nowhere but queen-pool.ts', () => {
    const offenders = sourceFiles(SRC)
      .filter((f) => !f.endsWith(ACCESSOR))
      .filter((f) =>
        /\bnew Pool\s*\(/.test(
          stripCommentsAndStrings(readFileSync(f, 'utf8')),
        ),
      )
      .map((f) => f.slice(SRC.length + 1))

    // Named rather than counted: a bare count tells the next reader a number
    // and makes them go find the file themselves.
    expect(offenders).toEqual([])
  })
})

/**
 * Comments and string literals are removed before the search.
 *
 * queen-lease.ts documents the pool-after-end defect with the words
 * `new Pool(...)` in prose, and a gate that reads prose is the exact class this
 * repository exists to catch.
 */
function stripCommentsAndStrings(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/[^\n]*/g, ' ')
    .replace(/`(?:\\.|[^`\\])*`/g, '``')
    .replace(/'(?:\\.|[^'\\\n])*'/g, "''")
    .replace(/"(?:\\.|[^"\\\n])*"/g, '""')
}
