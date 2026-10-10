/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Where the self-hosting rows live (gHashTag/trios#1756): hosts, jobs, leases
 * and the off-chain ledger. State only; it decides nothing.
 *
 * Two stores, one interface. The Queen runs on Postgres (the `pg` pool every
 * Queen route uses, tables created in code on first use, like the waits and
 * the jobs). Tests run on the memory store. Every Queen operation runs inside
 * one `tx`: on Postgres a transaction holding a transaction-scoped advisory
 * lock, so two Queen processes (a deploy's old and new one) never place the
 * same replica twice; in memory a queue, for the same reason.
 */

import type { Pool, PoolClient } from 'pg'

export interface HostRow {
  /** The key id: KEY_ID_HEX_LEN hex characters of SHA-256(public key). */
  id: string
  publicKey: string
  tier: number
  tierClaim: number
  slots: number
  platform: string
  /** A hash of the network prefix the host registered from, never the address. */
  origin: string
  incarnation: number
  beatAt: number
  strikes: number
  registeredAt: number
}

export interface JobFile {
  path: string
  sha256: string
}

export interface JobResult {
  outputHash: string
  word: string
  tests: number
  ops: number
  hosts: string[]
}

export interface JobRow {
  id: string
  kind: 'shard'
  commit: string
  spec: string
  inputHash: string
  files: JobFile[]
  modelHash: string | null
  workloadClass: number
  holdsSecret: boolean
  holdsPersonal: boolean
  wanted: number
  leasesIssued: number
  state: 'open' | 'agreed' | 'unresolved'
  verdict: number
  createdAt: number
  closedAt: number | null
  result: JobResult | null
}

export interface LeaseRow {
  id: string
  jobId: string
  hostId: string
  incarnation: number
  nonce: string
  leasedAt: number
  /** out: running; reported: a receipt is in; lapsed: placed again; closed: the job ended first. */
  state: 'out' | 'reported' | 'lapsed' | 'closed'
  code: number | null
  vote: boolean
  outputHash: string | null
  /** The first receipt for this lease, exactly as it came. */
  receipt: Record<string, unknown> | null
  receivedAt: number | null
}

export interface LedgerRow {
  seq: number
  hostId: string
  jobId: string
  kind: number
  mtri: number
  at: number
}

export interface HostingTx {
  host(id: string): Promise<HostRow | null>
  hosts(): Promise<HostRow[]>
  putHost(row: HostRow): Promise<void>
  job(id: string): Promise<JobRow | null>
  jobs(): Promise<JobRow[]>
  openJobs(): Promise<JobRow[]>
  putJob(row: JobRow): Promise<void>
  lease(id: string): Promise<LeaseRow | null>
  leasesOf(jobId: string): Promise<LeaseRow[]>
  outLeasesOfHost(hostId: string): Promise<LeaseRow[]>
  putLease(row: LeaseRow): Promise<void>
  ledgerHas(hostId: string, jobId: string, kind: number): Promise<boolean>
  addLedger(row: Omit<LedgerRow, 'seq'>): Promise<void>
  ledger(): Promise<LedgerRow[]>
}

export interface HostingStore {
  tx<T>(fn: (s: HostingTx) => Promise<T>): Promise<T>
}

const clone = <T>(row: T): T => structuredClone(row)

/** The memory store: Maps, and a queue so one operation runs at a time. */
export function createMemoryHostingStore(): HostingStore {
  const hosts = new Map<string, HostRow>()
  const jobs = new Map<string, JobRow>()
  const leases = new Map<string, LeaseRow>()
  const ledger: LedgerRow[] = []
  let tail: Promise<unknown> = Promise.resolve()
  const s: HostingTx = {
    host: async (id) =>
      hosts.has(id) ? clone(hosts.get(id) as HostRow) : null,
    hosts: async () => [...hosts.values()].map(clone),
    putHost: async (row) => {
      hosts.set(row.id, clone(row))
    },
    job: async (id) => (jobs.has(id) ? clone(jobs.get(id) as JobRow) : null),
    jobs: async () => [...jobs.values()].map(clone),
    openJobs: async () =>
      [...jobs.values()].filter((j) => j.state === 'open').map(clone),
    putJob: async (row) => {
      jobs.set(row.id, clone(row))
    },
    lease: async (id) =>
      leases.has(id) ? clone(leases.get(id) as LeaseRow) : null,
    leasesOf: async (jobId) =>
      [...leases.values()].filter((l) => l.jobId === jobId).map(clone),
    outLeasesOfHost: async (hostId) =>
      [...leases.values()]
        .filter((l) => l.hostId === hostId && l.state === 'out')
        .map(clone),
    putLease: async (row) => {
      leases.set(row.id, clone(row))
    },
    ledgerHas: async (hostId, jobId, kind) =>
      ledger.some(
        (r) => r.hostId === hostId && r.jobId === jobId && r.kind === kind,
      ),
    addLedger: async (row) => {
      ledger.push({ ...row, seq: ledger.length + 1 })
    },
    ledger: async () => ledger.map(clone),
  }
  return {
    tx<T>(fn: (s: HostingTx) => Promise<T>): Promise<T> {
      const run = tail.then(() => fn(s))
      tail = run.catch(() => undefined)
      return run
    },
  }
}

// --- Postgres -----------------------------------------------------------------

export const HOSTING_SQL = [
  'CREATE TABLE IF NOT EXISTS hosting_hosts (',
  '  id text PRIMARY KEY,',
  '  public_key text NOT NULL,',
  '  tier smallint NOT NULL,',
  '  tier_claim smallint NOT NULL,',
  '  slots integer NOT NULL,',
  '  platform text NOT NULL,',
  '  origin text NOT NULL,',
  '  incarnation bigint NOT NULL,',
  '  beat_at bigint NOT NULL,',
  '  strikes integer NOT NULL DEFAULT 0,',
  '  registered_at bigint NOT NULL',
  ');',
  'CREATE TABLE IF NOT EXISTS hosting_jobs (',
  '  id text PRIMARY KEY,',
  '  state text NOT NULL,',
  '  created_at bigint NOT NULL,',
  '  body jsonb NOT NULL',
  ');',
  'CREATE INDEX IF NOT EXISTS hosting_jobs_state ON hosting_jobs (state, created_at);',
  'CREATE TABLE IF NOT EXISTS hosting_leases (',
  '  id text PRIMARY KEY,',
  '  job_id text NOT NULL,',
  '  host_id text NOT NULL,',
  '  state text NOT NULL,',
  '  body jsonb NOT NULL',
  ');',
  'CREATE INDEX IF NOT EXISTS hosting_leases_job ON hosting_leases (job_id);',
  'CREATE INDEX IF NOT EXISTS hosting_leases_host ON hosting_leases (host_id, state);',
  'CREATE TABLE IF NOT EXISTS hosting_ledger (',
  '  seq bigserial PRIMARY KEY,',
  '  host_id text NOT NULL,',
  '  job_id text NOT NULL,',
  '  kind smallint NOT NULL,',
  '  mtri bigint NOT NULL,',
  '  at bigint NOT NULL,',
  '  UNIQUE (host_id, job_id, kind)',
  ');',
].join('\n')

/** pg_advisory_xact_lock key: 'host' as four ASCII bytes. */
const HOSTING_LOCK = 0x686f7374

type HostSqlRow = {
  id: string
  public_key: string
  tier: number
  tier_claim: number
  slots: number
  platform: string
  origin: string
  incarnation: string
  beat_at: string
  strikes: number
  registered_at: string
}

const hostOf = (r: HostSqlRow): HostRow => ({
  id: r.id,
  publicKey: r.public_key,
  tier: Number(r.tier),
  tierClaim: Number(r.tier_claim),
  slots: Number(r.slots),
  platform: r.platform,
  origin: r.origin,
  incarnation: Number(r.incarnation),
  beatAt: Number(r.beat_at),
  strikes: Number(r.strikes),
  registeredAt: Number(r.registered_at),
})

function pgTx(c: PoolClient): HostingTx {
  const bodies = async <T>(sql: string, args: unknown[]): Promise<T[]> =>
    (await c.query<{ body: T }>(sql, args)).rows.map((r) => r.body)
  return {
    host: async (id) => {
      const r = await c.query<HostSqlRow>(
        'SELECT * FROM hosting_hosts WHERE id = $1',
        [id],
      )
      return r.rows[0] ? hostOf(r.rows[0]) : null
    },
    hosts: async () =>
      (
        await c.query<HostSqlRow>('SELECT * FROM hosting_hosts ORDER BY id')
      ).rows.map(hostOf),
    putHost: async (h) => {
      await c.query(
        'INSERT INTO hosting_hosts (id, public_key, tier, tier_claim, slots, platform, origin, incarnation, beat_at, strikes, registered_at) ' +
          'VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ' +
          'ON CONFLICT (id) DO UPDATE SET public_key = $2, tier = $3, tier_claim = $4, slots = $5, platform = $6, ' +
          'origin = $7, incarnation = $8, beat_at = $9, strikes = $10, registered_at = $11',
        [
          h.id,
          h.publicKey,
          h.tier,
          h.tierClaim,
          h.slots,
          h.platform,
          h.origin,
          h.incarnation,
          h.beatAt,
          h.strikes,
          h.registeredAt,
        ],
      )
    },
    job: async (id) =>
      (
        await bodies<JobRow>('SELECT body FROM hosting_jobs WHERE id = $1', [
          id,
        ])
      )[0] ?? null,
    jobs: () =>
      bodies<JobRow>(
        'SELECT body FROM hosting_jobs ORDER BY created_at, id',
        [],
      ),
    openJobs: () =>
      bodies<JobRow>(
        "SELECT body FROM hosting_jobs WHERE state = 'open' ORDER BY created_at, id",
        [],
      ),
    putJob: async (j) => {
      await c.query(
        'INSERT INTO hosting_jobs (id, state, created_at, body) VALUES ($1, $2, $3, $4) ' +
          'ON CONFLICT (id) DO UPDATE SET state = $2, body = $4',
        [j.id, j.state, j.createdAt, JSON.stringify(j)],
      )
    },
    lease: async (id) =>
      (
        await bodies<LeaseRow>(
          'SELECT body FROM hosting_leases WHERE id = $1',
          [id],
        )
      )[0] ?? null,
    leasesOf: (jobId) =>
      bodies<LeaseRow>(
        'SELECT body FROM hosting_leases WHERE job_id = $1 ORDER BY id',
        [jobId],
      ),
    outLeasesOfHost: (hostId) =>
      bodies<LeaseRow>(
        "SELECT body FROM hosting_leases WHERE host_id = $1 AND state = 'out' ORDER BY id",
        [hostId],
      ),
    putLease: async (l) => {
      await c.query(
        'INSERT INTO hosting_leases (id, job_id, host_id, state, body) VALUES ($1, $2, $3, $4, $5) ' +
          'ON CONFLICT (id) DO UPDATE SET state = $4, body = $5',
        [l.id, l.jobId, l.hostId, l.state, JSON.stringify(l)],
      )
    },
    ledgerHas: async (hostId, jobId, kind) =>
      (
        await c.query(
          'SELECT 1 FROM hosting_ledger WHERE host_id = $1 AND job_id = $2 AND kind = $3',
          [hostId, jobId, kind],
        )
      ).rows.length > 0,
    addLedger: async (r) => {
      await c.query(
        'INSERT INTO hosting_ledger (host_id, job_id, kind, mtri, at) VALUES ($1, $2, $3, $4, $5)',
        [r.hostId, r.jobId, r.kind, r.mtri, r.at],
      )
    },
    ledger: async () =>
      (
        await c.query<{
          seq: string
          host_id: string
          job_id: string
          kind: number
          mtri: string
          at: string
        }>('SELECT * FROM hosting_ledger ORDER BY seq')
      ).rows.map((r) => ({
        seq: Number(r.seq),
        hostId: r.host_id,
        jobId: r.job_id,
        kind: Number(r.kind),
        mtri: Number(r.mtri),
        at: Number(r.at),
      })),
  }
}

export function createPgHostingStore(pool: Pool): HostingStore {
  let ensured: Promise<unknown> | null = null
  return {
    async tx<T>(fn: (s: HostingTx) => Promise<T>): Promise<T> {
      ensured ??= pool.query(HOSTING_SQL).catch((err) => {
        ensured = null
        throw err
      })
      await ensured
      const c = await pool.connect()
      try {
        await c.query('BEGIN')
        await c.query('SELECT pg_advisory_xact_lock($1)', [HOSTING_LOCK])
        const out = await fn(pgTx(c))
        await c.query('COMMIT')
        return out
      } catch (err) {
        await c.query('ROLLBACK').catch(() => undefined)
        throw err
      } finally {
        c.release()
      }
    },
  }
}
