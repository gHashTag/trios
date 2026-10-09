/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * LONG WAITS AS ROWS (gHashTag/t27 specs/queen/waits.t27, trios#1712 item 4).
 *
 * WHY. The swarm's first release (job #3, t27c 0.5.2, 2026-10-09) spent about
 * 90 of its 92 minutes at step 3, release-workflow, waiting for a runner. The
 * job was already a row, but its WAIT step was asked again by every round:
 * one GitHub read and one log entry per round, about 180 of each for one
 * wait, and the wait moved only while a process ran rounds. Here a wait is its
 * own row in queen_wait, checked on the card's schedule and woken by one
 * scheduler actor; the job is not visited until the row ends.
 *
 * WHAT DECIDES. The card, queen/waits.wasm: whether a row is due and what to
 * do with it (wake_action, due_in_seconds), its expiry (expiry_seconds,
 * job_wait_seconds), when a key is checked again (check_after_seconds,
 * recheck_in_seconds, check_event), whether a write lands (epoch_after_claim,
 * write_lands, resolution_lands, state_after, transition_allowed), who hears
 * it (delivers_wake), and when the scheduler looks again (beat_after_seconds,
 * pass_on_hint, more_now). This file holds the rows, reads GitHub, and keeps
 * the clock it is given.
 *
 * THE SCHEDULER is one actor per process (queen-waits) under a root
 * supervisor. It takes a pass on its beat (posted to its control lane, as the
 * reviewer's intake does), on every bus event, and on a NOTIFY on queen_wait
 * from any process. A notification is only a hint: the beat runs whether this
 * process listens or not. Any number of processes may run one. A row is
 * claimed by exactly one pass (FOR UPDATE SKIP LOCKED, and the claim moves its
 * epoch), and a pass that slept past its claim writes nothing and wakes nobody.
 * Every fenced write locks its row, asks the card, and writes only if the card
 * says it lands - the SQL carries no rule of its own.
 *
 * A DEPLOY WRITES NOTHING HERE (DEPLOY_WRITES_WAITS). stop() ends the actor and
 * the listener; the rows stay, and the next process takes them up - a row this
 * one had claimed after its claim runs out (pickup_after_seconds).
 *
 * Off unless TRIOS_QUEEN_WAITS=rows. Off, the release job's WAIT step is asked
 * by every round, as before.
 */

import type { Pool, PoolClient } from 'pg'
import { QUEEN_WAITS_SQL } from '../../lib/db/pg-migrate'
import { logger } from '../../lib/logger'
import {
  type ActorSystem,
  actorChild,
  type Child,
  createActorSystem,
  type Pid,
  supervisor,
} from './queen-actors'
import {
  M_HEARTBEAT_DUE,
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
} from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { addControlEventReader } from './queen-control'
import {
  CLAIM_BATCH,
  CLAIM_TTL_SECONDS,
  EV_CANCEL,
  EV_EXPIRE,
  EV_RESOLVE,
  PH_NOW,
  PH_QUEUE,
  W_WAITING,
  WA_CHECK,
  WA_EXPIRE,
  WA_RESOLVE,
} from './queen-waits-card.gen'

export * from './queen-waits-card.gen'

export const WAITS_CARD = 'queen/waits.wasm'

const card = () => loadCardWasm(WAITS_CARD)
const c = (name: string, ...a: number[]) => card().call(name, ...a)
const yes = (name: string, ...a: number[]) => c(name, ...a) !== 0

export const isTerminal = (state: number): boolean => yes('is_terminal', state)
export const stateAfter = (state: number, ev: number): number =>
  c('state_after', state, ev)
export const transitionAllowed = (from: number, to: number): boolean =>
  yes('transition_allowed', from, to)
export const validWait = (hasKey: boolean, hasWake: boolean): boolean =>
  yes('valid_wait', flag(hasKey), flag(hasWake))
export const wakesOwner = (state: number): boolean => yes('wakes_owner', state)
export const deliversWake = (landed: boolean, to: number): boolean =>
  yes('delivers_wake', flag(landed), to)
export const resolutionLands = (
  state: number,
  alreadyResolved: boolean,
): boolean => yes('resolution_lands', state, flag(alreadyResolved))
export const wakeAction = (
  state: number,
  resolved: boolean,
  hasKey: boolean,
  hasWake: boolean,
  secondsToWake: number,
  secondsToExpiry: number,
): number =>
  c(
    'wake_action',
    state,
    flag(resolved),
    flag(hasKey),
    flag(hasWake),
    u32(secondsToWake),
    u32(secondsToExpiry),
  )
export const dueInSeconds = (
  state: number,
  resolved: boolean,
  hasWake: boolean,
  secondsToWake: number,
  secondsToExpiry: number,
): number =>
  c(
    'due_in_seconds',
    state,
    flag(resolved),
    flag(hasWake),
    u32(secondsToWake),
    u32(secondsToExpiry),
  ) >>> 0
export const expirySeconds = (asked: number): number =>
  c('expiry_seconds', u32(asked)) >>> 0
export const jobWaitSeconds = (waitedSeconds: number): number =>
  c('job_wait_seconds', u32(waitedSeconds)) >>> 0
export const checkAfterSeconds = (checks: number): number =>
  c('check_after_seconds', u32(checks)) >>> 0
export const recheckInSeconds = (
  checks: number,
  secondsToExpiry: number,
): number => c('recheck_in_seconds', u32(checks), u32(secondsToExpiry)) >>> 0
export const checkEvent = (completed: boolean): number =>
  c('check_event', flag(completed))
export const epochAfterClaim = (epoch: bigint): bigint =>
  BigInt.asUintN(64, BigInt(card().call64('epoch_after_claim', epoch)))
export const writeLands = (epoch: bigint, mine: bigint): boolean =>
  Number(card().call64('write_lands', epoch, mine)) !== 0
export const moreNow = (claimed: number): boolean =>
  yes('more_now', u32(claimed))
export const pollSeconds = (listening: boolean): number =>
  c('poll_seconds', flag(listening)) >>> 0
export const beatAfterSeconds = (
  listening: boolean,
  hasNext: boolean,
  secondsToNext: number,
): number =>
  c(
    'beat_after_seconds',
    flag(listening),
    flag(hasNext),
    u32(secondsToNext),
  ) >>> 0
export const passOnHint = (running: boolean, queued: boolean): number =>
  c('pass_on_hint', flag(running), flag(queued))
export const pickupAfterSeconds = (
  claimed: boolean,
  secondsSinceClaim: number,
): number =>
  c('pickup_after_seconds', flag(claimed), u32(secondsSinceClaim)) >>> 0
export const roundVisitsJob = (parked: boolean): boolean =>
  yes('round_visits_job', flag(parked))
export const stepOutcome = (state: number, success: boolean): number =>
  c('step_outcome', state, flag(success))
export const stepParks = (runFound: boolean, runCompleted: boolean): boolean =>
  yes('step_parks', flag(runFound), flag(runCompleted))

/** Whole seconds from `now` until `at` (ms), 0 once it has come. */
export const secondsTo = (now: number, at: number | null): number =>
  at === null ? 0 : Math.max(0, Math.ceil((at - now) / 1000))

/** TRIOS_QUEEN_WAITS=rows turns waits on; anything else leaves them off. */
export function waitsEnabled(
  env: Record<string, string | undefined> = process.env,
): boolean {
  return (env.TRIOS_QUEEN_WAITS ?? '').trim().toLowerCase() === 'rows'
}

const ensured = new WeakSet<Pool>()
/** The table, once per pool: boot's migration made it, this covers a deploy that predates it. */
export async function ensureWaitTables(pool: Pool): Promise<void> {
  if (ensured.has(pool)) return
  await pool.query(QUEEN_WAITS_SQL)
  ensured.add(pool)
}

export interface WaitRow {
  id: number
  owner: string
  key: string | null
  state: number
  wakeAt: number | null
  expiresAt: number
  dueAt: number | null
  checks: number
  epoch: bigint
  resolution: unknown
  detail: Record<string, unknown>
}

const ms = (v: unknown): number | null =>
  v === null || v === undefined ? null : new Date(v as string).getTime()

function rowOf(r: Record<string, unknown>): WaitRow {
  return {
    id: Number(r.id),
    owner: String(r.owner),
    key: r.key === null || r.key === undefined ? null : String(r.key),
    state: Number(r.state),
    wakeAt: ms(r.wake_at),
    expiresAt: ms(r.expires_at) ?? 0,
    dueAt: ms(r.due_at),
    checks: Number(r.checks),
    epoch: BigInt(String(r.epoch)),
    resolution: r.resolution ?? null,
    detail: (r.detail as Record<string, unknown>) ?? {},
  }
}

const at = (now: number, seconds: number): Date =>
  new Date(now + seconds * 1000)

/** The owner a job's waits carry; the wake handler for jobs reads it back. */
export const jobOwner = (jobId: number): string => `job:${jobId}`

/** The key of a wait on one GitHub Actions run. */
export const ghRunKey = (repo: string, runId: number | string): string =>
  `gh-run:${repo}:${runId}`

export interface NewWait {
  owner: string
  /** What resolves it (a resolver checks it, or resolveWaitByKey does). */
  key?: string
  /** A timer, or for a keyed row the first check. */
  wakeInSeconds?: number
  /** Asked expiry; expiry_seconds caps it (0 or absent = the cap). */
  expirySeconds?: number
  detail?: Record<string, unknown>
}

/**
 * Make a wait, or find the one this owner already has on this key: a job that
 * comes back after a restart asks again and gets the same row.
 */
export async function createWait(
  pool: Pool,
  w: NewWait,
  now: number,
): Promise<WaitRow> {
  const hasKey = typeof w.key === 'string' && w.key.length > 0
  const hasWake = w.wakeInSeconds !== undefined
  if (!validWait(hasKey, hasWake))
    throw new Error('a wait names a key, a wake time, or both')
  const expiry = expirySeconds(w.expirySeconds ?? 0)
  const wake = hasWake ? u32(w.wakeInSeconds as number) : 0
  const due = dueInSeconds(W_WAITING, false, hasWake, wake, expiry)
  const made = await pool.query(
    `WITH i AS (
       INSERT INTO queen_wait
         (owner, key, state, wake_at, expires_at, due_at, detail, created_at, updated_at)
       VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $8)
       ON CONFLICT (owner, key) DO NOTHING
       RETURNING *)
     SELECT i.*, pg_notify('queen_wait', '') AS hint FROM i`,
    [
      w.owner,
      hasKey ? w.key : null,
      W_WAITING,
      hasWake ? at(now, wake) : null,
      at(now, expiry),
      at(now, due),
      JSON.stringify(w.detail ?? {}),
      new Date(now),
    ],
  )
  if (made.rows[0]) return rowOf(made.rows[0])
  const found = await pool.query(
    'SELECT * FROM queen_wait WHERE owner = $1 AND key = $2',
    [w.owner, w.key],
  )
  return rowOf(found.rows[0])
}

export async function getWait(pool: Pool, id: number): Promise<WaitRow | null> {
  const r = await pool.query('SELECT * FROM queen_wait WHERE id = $1', [id])
  return r.rows[0] ? rowOf(r.rows[0]) : null
}

/** The latest row an owner made, optionally for one step of a job. */
export async function waitOf(
  pool: Pool,
  owner: string,
  step?: number,
): Promise<WaitRow | null> {
  const r =
    step === undefined
      ? await pool.query(
          'SELECT * FROM queen_wait WHERE owner = $1 ORDER BY id DESC LIMIT 1',
          [owner],
        )
      : await pool.query(
          `SELECT * FROM queen_wait WHERE owner = $1 AND detail->>'step' = $2
            ORDER BY id DESC LIMIT 1`,
          [owner, String(step)],
        )
  return r.rows[0] ? rowOf(r.rows[0]) : null
}

/** Whether an owner has a row still waiting (the job is parked on it). */
export async function ownerWaiting(
  pool: Pool,
  owner: string,
): Promise<boolean> {
  const r = await pool.query(
    `SELECT 1 FROM queen_wait WHERE owner = $1 AND state = ${W_WAITING} LIMIT 1`,
    [owner],
  )
  return (r.rowCount ?? 0) > 0
}

/** What is waiting now, and what ended last: the board's "who waits for what". */
export async function listWaits(pool: Pool, limit = 50): Promise<WaitRow[]> {
  const r = await pool.query(
    `SELECT * FROM queen_wait ORDER BY (state = ${W_WAITING}) DESC, id DESC LIMIT $1`,
    [limit],
  )
  return r.rows.map(rowOf)
}

/** One transaction on one locked row; `write` returns false to write nothing. */
async function onLockedRow<T>(
  pool: Pool,
  where: { sql: string; args: unknown[] },
  write: (row: WaitRow, client: PoolClient) => Promise<T | null>,
): Promise<T[]> {
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    const rows = await client.query(
      `SELECT * FROM queen_wait WHERE ${where.sql} ORDER BY id FOR UPDATE`,
      where.args,
    )
    const done: T[] = []
    for (const r of rows.rows) {
      const out = await write(rowOf(r), client)
      if (out !== null) done.push(out)
    }
    await client.query('COMMIT')
    return done
  } catch (error) {
    await client.query('ROLLBACK').catch(() => undefined)
    throw error
  } finally {
    client.release()
  }
}

/**
 * Resolve every row that waits on `key` (resolution_lands: the first answer
 * stays). The row is due at once, and a NOTIFY asks every scheduler for a pass.
 */
export async function resolveWaitByKey(
  pool: Pool,
  key: string,
  resolution: unknown,
  now: number,
): Promise<number[]> {
  return onLockedRow(
    pool,
    { sql: 'key = $1', args: [key] },
    async (row, client) => {
      if (!resolutionLands(row.state, row.resolution !== null)) return null
      const due = dueInSeconds(
        row.state,
        true,
        row.wakeAt !== null,
        secondsTo(now, row.wakeAt),
        secondsTo(now, row.expiresAt),
      )
      await client.query(
        `UPDATE queen_wait SET resolution = $2::jsonb, due_at = $3, updated_at = $4
          WHERE id = $1`,
        [row.id, JSON.stringify(resolution ?? {}), at(now, due), new Date(now)],
      )
      await client.query(`SELECT pg_notify('queen_wait', '')`)
      return row.id
    },
  )
}

/** The owner gives up (a cancelled job). Each cancel moves the epoch. */
export async function cancelWaitsOf(
  pool: Pool,
  owner: string,
  now: number,
): Promise<number[]> {
  return onLockedRow(
    pool,
    { sql: `owner = $1 AND state = ${W_WAITING}`, args: [owner] },
    async (row, client) => {
      const to = stateAfter(row.state, EV_CANCEL)
      if (!transitionAllowed(row.state, to)) return null
      await client.query(
        `UPDATE queen_wait SET state = $2, epoch = $3, due_at = NULL,
                ended_at = $4, updated_at = $4
          WHERE id = $1`,
        [row.id, to, epochAfterClaim(row.epoch).toString(), new Date(now)],
      )
      return row.id
    },
  )
}

/**
 * Claim the due rows, in one short transaction: SKIP LOCKED leaves rows
 * another pass is claiming to it, the epoch moves (epoch_after_claim), and the
 * due time moves CLAIM_TTL_SECONDS ahead, so no other pass sees the row until
 * this claim runs out. The returned rows carry the claimed epoch.
 */
export async function claimDue(
  pool: Pool,
  now: number,
  batch: number = CLAIM_BATCH,
): Promise<WaitRow[]> {
  const client = await pool.connect()
  try {
    await client.query('BEGIN')
    const due = await client.query(
      `SELECT * FROM queen_wait
        WHERE state = ${W_WAITING} AND due_at <= $1
        ORDER BY due_at, id LIMIT $2
        FOR UPDATE SKIP LOCKED`,
      [new Date(now), batch],
    )
    const rows = due.rows.map(rowOf).map((r) => ({
      ...r,
      epoch: epochAfterClaim(r.epoch),
    }))
    if (rows.length > 0)
      await client.query(
        `UPDATE queen_wait q
            SET epoch = v.epoch, due_at = $3, updated_at = $4
           FROM unnest($1::bigint[], $2::bigint[]) AS v(id, epoch)
          WHERE q.id = v.id`,
        [
          rows.map((r) => r.id),
          rows.map((r) => r.epoch.toString()),
          at(now, CLAIM_TTL_SECONDS),
          new Date(now),
        ],
      )
    await client.query('COMMIT')
    return rows
  } catch (error) {
    await client.query('ROLLBACK').catch(() => undefined)
    throw error
  } finally {
    client.release()
  }
}

export interface Ended {
  landed: boolean
  to: number
  resolution: unknown
}

/** End a claimed row (resolved or expired), at the epoch it was claimed with. */
export async function endClaimed(
  pool: Pool,
  claimed: WaitRow,
  ev: number,
  resolution: unknown,
  now: number,
): Promise<Ended> {
  const out = await onLockedRow(
    pool,
    { sql: 'id = $1', args: [claimed.id] },
    async (row, client) => {
      if (!writeLands(row.epoch, claimed.epoch)) return null
      const to = stateAfter(row.state, ev)
      if (!transitionAllowed(row.state, to)) return null
      const kept = resolutionLands(row.state, row.resolution !== null)
        ? (resolution ?? null)
        : row.resolution
      await client.query(
        `UPDATE queen_wait SET state = $2, resolution = $3::jsonb, due_at = NULL,
                ended_at = $4, updated_at = $4
          WHERE id = $1`,
        [
          row.id,
          to,
          kept === null ? null : JSON.stringify(kept),
          new Date(now),
        ],
      )
      return { landed: true, to, resolution: kept } as Ended
    },
  )
  return out[0] ?? { landed: false, to: claimed.state, resolution: null }
}

/**
 * Put a claimed row back to wait: after a check that said "not yet" (`checks`
 * one more, the next check by recheck_in_seconds), or when the card found it
 * not due after all. Lands only at the claimed epoch.
 */
export async function rearmClaimed(
  pool: Pool,
  claimed: WaitRow,
  checked: boolean,
  now: number,
): Promise<boolean> {
  const out = await onLockedRow(
    pool,
    { sql: 'id = $1', args: [claimed.id] },
    async (row, client) => {
      if (!writeLands(row.epoch, claimed.epoch)) return null
      if (isTerminal(row.state)) return null
      const toExpiry = secondsTo(now, row.expiresAt)
      const checks = checked ? row.checks + 1 : row.checks
      const wakeAt = checked
        ? now + recheckInSeconds(checks, toExpiry) * 1000
        : row.wakeAt
      const due = dueInSeconds(
        row.state,
        row.resolution !== null,
        wakeAt !== null,
        secondsTo(now, wakeAt),
        toExpiry,
      )
      await client.query(
        `UPDATE queen_wait SET checks = $2, wake_at = $3, due_at = $4, updated_at = $5
          WHERE id = $1`,
        [
          row.id,
          checks,
          wakeAt === null ? null : new Date(wakeAt),
          at(now, due),
          new Date(now),
        ],
      )
      return true
    },
  )
  return out.length > 0
}

/** Seconds to the nearest due row, or null when nothing waits. */
export async function nextDueSeconds(
  pool: Pool,
  now: number,
): Promise<number | null> {
  const r = await pool.query(
    `SELECT min(due_at) AS due FROM queen_wait WHERE state = ${W_WAITING}`,
  )
  const due = ms(r.rows[0]?.due)
  return due === null ? null : secondsTo(now, due)
}

/** A check of a key: did what it names end, and how. */
export interface CheckAnswer {
  completed: boolean
  resolution?: unknown
}

/** Checks a row's key; chosen by the key's kind, the text before its first ':'. */
export type Resolver = (row: WaitRow) => Promise<CheckAnswer>

/** The GitHub client a resolver reads through (queen-jobs.ts JobIo.get). */
export type GithubGet = (
  url: string,
  auth: 'read' | 'none',
) => Promise<{ status: number; body: unknown }>

/**
 * One GitHub Actions run, read by its id. A read that fails is not completed
 * (check_event re-arms it), so a GitHub outage costs checks, not the wait.
 */
export const githubRunResolver =
  (get: GithubGet): Resolver =>
  async (row) => {
    const m = row.key?.match(/^gh-run:([^:]+\/[^:]+):(\d+)$/)
    if (!m) return { completed: false }
    const r = await get(
      `https://api.github.com/repos/${m[1]}/actions/runs/${m[2]}`,
      'read',
    ).catch(() => null)
    const body = (r?.body ?? null) as Record<string, unknown> | null
    if (!r || r.status !== 200 || !body) return { completed: false }
    return {
      completed: body.status === 'completed',
      resolution: {
        status: body.status ?? null,
        conclusion: body.conclusion ?? null,
        html_url: body.html_url ?? null,
      },
    }
  }

export interface WaitsStats {
  passes: number
  claimed: number
  checks: number
  woken: number
  /** Writes the card refused: a stale epoch, or a row that had ended. */
  refused: number
  failedPasses: number
  /** Wall-clock milliseconds spent in passes: what the waits cost this process. */
  passMs: number
}

export interface WaitsDeps {
  pool: Pool
  resolvers: Record<string, Resolver>
  /** The owner hears that its row ended. Never awaited; the row is the truth. */
  onWake: (row: WaitRow) => void
  listening?: () => boolean
}

type WaitsMsg = { kind: 'pass' }

/** The scheduler as one child, for a root supervisor or a test. */
export function waitsTree(sys: ActorSystem, deps: WaitsDeps) {
  const clock = sys.clock
  const stats: WaitsStats = {
    passes: 0,
    claimed: 0,
    checks: 0,
    woken: 0,
    refused: 0,
    failedPasses: 0,
    passMs: 0,
  }
  let self: Pid | undefined
  // pass_on_hint's two facts: a pass is queued or running, and one more is wanted
  let running = false
  let queued = false
  let cancelBeat: (() => void) | undefined
  let halted = false
  let inPass = false
  // a killed pass runs on, abandoned (actors.t27 KILL_ABANDONS): it must not
  // touch the next incarnation's bookkeeping, only its fenced rows
  let incarnation = 0

  const hint = () => {
    if (self === undefined) return
    const a = passOnHint(running, queued)
    if (a === PH_NOW) {
      running = true
      sys.send(self, { kind: 'pass' } as WaitsMsg)
    } else if (a === PH_QUEUE) queued = true
  }
  const arm = (seconds: number) => {
    cancelBeat?.()
    if (halted) return
    cancelBeat = clock.after(seconds * 1000, () => {
      if (self !== undefined) sys.post(self, M_HEARTBEAT_DUE)
    })
  }

  const end = async (row: WaitRow, ev: number, resolution: unknown) => {
    const r = await endClaimed(deps.pool, row, ev, resolution, clock.now())
    if (deliversWake(r.landed, r.to)) {
      stats.woken++
      try {
        deps.onWake({ ...row, state: r.to, resolution: r.resolution })
      } catch (error) {
        logger.warn('Queen wait woke an owner that threw', {
          id: row.id,
          owner: row.owner,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    } else if (!r.landed) stats.refused++
  }

  const handle = async (row: WaitRow, now: number) => {
    const action = wakeAction(
      row.state,
      row.resolution !== null,
      row.key !== null,
      row.wakeAt !== null,
      secondsTo(now, row.wakeAt),
      secondsTo(now, row.expiresAt),
    )
    if (action === WA_RESOLVE) return end(row, EV_RESOLVE, row.resolution)
    if (action === WA_EXPIRE) return end(row, EV_EXPIRE, null)
    if (action === WA_CHECK) {
      const kind = (row.key ?? '').split(':')[0]
      const resolver = deps.resolvers[kind]
      stats.checks++
      const answer: CheckAnswer = resolver
        ? await resolver(row).catch(() => ({ completed: false }))
        : { completed: false }
      if (checkEvent(answer.completed) === EV_RESOLVE)
        return end(row, EV_RESOLVE, answer.resolution ?? null)
      if (!(await rearmClaimed(deps.pool, row, true, clock.now())))
        stats.refused++
      return
    }
    if (!(await rearmClaimed(deps.pool, row, false, now))) stats.refused++
  }

  /** The rows it claimed, and the next due time when it claimed none. */
  const pass = async (): Promise<{
    claimed: number
    next?: number | null
  }> => {
    stats.passes++
    // one cheap read first: a pass with nothing due opens no transaction
    const next = await nextDueSeconds(deps.pool, clock.now())
    if (next === null || next > 0) return { claimed: 0, next }
    const rows = await claimDue(deps.pool, clock.now())
    stats.claimed += rows.length
    const now = clock.now()
    await Promise.all(rows.map((row) => handle(row, now)))
    return { claimed: rows.length }
  }

  const child: Child = actorChild(sys, {
    name: 'queen-waits',
    // a pass that outlives its claim writes nothing anyway (write_lands)
    turnMaxSeconds: CLAIM_TTL_SECONDS,
    init: (pid: Pid) => {
      self = pid
      incarnation += 1
      running = false
      queued = false
      inPass = false
      hint()
    },
    control: (tag: number) => {
      if (tag === M_HEARTBEAT_DUE) hint()
    },
    receive: async (_msg: WaitsMsg) => {
      const mine = incarnation
      inPass = true
      const began = performance.now()
      let claimed = 0
      let known: number | null | undefined
      try {
        const done = await pass()
        claimed = done.claimed
        known = done.next
      } catch (error) {
        // the rows are untouched by a pass that failed; the next beat retries
        stats.failedPasses++
        logger.warn('Queen waits pass failed', {
          error: error instanceof Error ? error.message : String(error),
        })
      }
      if (mine !== incarnation) return
      running = false
      if (moreNow(claimed)) {
        stats.passMs += performance.now() - began
        inPass = false
        hint()
        return
      }
      const listening = deps.listening?.() ?? false
      const next =
        known !== undefined
          ? known
          : await nextDueSeconds(deps.pool, clock.now()).catch(() => null)
      if (mine !== incarnation) return
      arm(beatAfterSeconds(listening, next !== null, next ?? 0))
      stats.passMs += performance.now() - began
      inPass = false
      if (queued) {
        queued = false
        hint()
      }
    },
  })

  return {
    tree: child,
    /** A bus event, a NOTIFY, or a new row here: a pass, if one is not coming. */
    hint: () => {
      if (self !== undefined) sys.post(self, M_HEARTBEAT_DUE)
    },
    stats,
    /** No pass running, none asked for: what a test waits for. */
    idle: () => !running && !queued && !inPass,
    /** No beat after this: a pass still in flight arms nothing. */
    halt: () => {
      halted = true
      cancelBeat?.()
    },
  }
}

let schedulerUp = false
/** Whether this process runs a waits scheduler (the jobs read it). */
export const waitsRunning = (): boolean => schedulerUp

/**
 * Start the scheduler in this process: a root supervisor over the waits actor,
 * a LISTEN on queen_wait (a hint; polling stays), and every bus event as a
 * hint. Returns stop(), which writes nothing to any row.
 */
export function startWaitsActors(
  pool: Pool,
  opts: {
    resolvers: Record<string, Resolver>
    onWake: (row: WaitRow) => void
    sys?: ActorSystem
  },
): () => Promise<void> {
  const sys = opts.sys ?? createActorSystem()
  let listener: PoolClient | null = null
  let stopped = false
  const w = waitsTree(sys, {
    pool,
    resolvers: opts.resolvers,
    onWake: opts.onWake,
    listening: () => listener !== null,
  })
  const root = supervisor(
    sys,
    {
      name: 'waits-root',
      strategy: STRAT_ONE_FOR_ONE,
      maxRestarts: ROOT_MAX_RESTARTS,
      periodSeconds: ROOT_PERIOD_SECONDS,
    },
    [w.tree],
  ).start((reason) => {
    // the scheduler gave up: jobs go back to being asked by every round
    schedulerUp = false
    logger.warn('Queen waits scheduler gave up; jobs are asked by the round', {
      reason,
    })
  })
  const unread = addControlEventReader(() => w.hint())
  schedulerUp = true
  void ensureWaitTables(pool)
    .then(async () => {
      if (stopped) return
      const client = await pool.connect()
      if (stopped) {
        client.release()
        return
      }
      client.on('notification', () => w.hint())
      await client.query('LISTEN queen_wait')
      listener = client
    })
    .catch((error) =>
      logger.warn('Queen waits listen by polling only', {
        error: error instanceof Error ? error.message : String(error),
      }),
    )
  logger.info('Queen waits starting as rows', {
    pollSeconds: pollSeconds(false),
    claimSeconds: CLAIM_TTL_SECONDS,
  })
  return async () => {
    stopped = true
    schedulerUp = false
    unread()
    root.stop()
    w.halt()
    const l = listener
    listener = null
    if (l) {
      await l.query('UNLISTEN queen_wait').catch(() => undefined)
      l.release()
    }
  }
}
