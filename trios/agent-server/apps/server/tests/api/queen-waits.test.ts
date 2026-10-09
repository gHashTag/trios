/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * LONG WAITS AS ROWS (gHashTag/t27 specs/queen/waits.t27), the parts that need
 * no database: the vendored card is the one PIN names, it exports every fn,
 * the wrappers answer the spec's own vectors, and the GitHub resolver reads a
 * run. The rows themselves are tested against PostgreSQL in
 * tests/pglive/queen-waits-live.test.ts.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createQueenWaitsRoute } from '../../src/api/routes/queen-waits'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import {
  O_FAIL,
  O_NOT_YET,
  O_PASS,
} from '../../src/api/services/queen-jobs.gen'
import {
  beatAfterSeconds,
  CHECK_CAP_SECONDS,
  CLAIM_BATCH,
  checkAfterSeconds,
  checkEvent,
  deliversWake,
  dueInSeconds,
  EV_CANCEL,
  EV_RECHECK,
  EV_RESOLVE,
  epochAfterClaim,
  expirySeconds,
  ghRunKey,
  githubRunResolver,
  JOB_O_FAIL,
  JOB_O_NOT_YET,
  JOB_O_PASS,
  jobWaitSeconds,
  moreNow,
  NEVER,
  PH_DROP,
  PH_NOW,
  PH_QUEUE,
  passOnHint,
  pickupAfterSeconds,
  pollSeconds,
  recheckInSeconds,
  resolutionLands,
  roundVisitsJob,
  secondsTo,
  stateAfter,
  stepOutcome,
  stepParks,
  transitionAllowed,
  W_CANCELLED,
  W_EXPIRED,
  W_RESOLVED,
  W_WAITING,
  WA_CHECK,
  WA_EXPIRE,
  WA_NONE,
  WA_RESOLVE,
  WAITS_CARD,
  type WaitRow,
  waitsEnabled,
  wakeAction,
  wakesOwner,
  writeLands,
} from '../../src/api/services/queen-waits'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

const CARD_FNS = [
  'is_terminal',
  'state_after',
  'transition_allowed',
  'valid_wait',
  'wakes_owner',
  'delivers_wake',
  'resolution_lands',
  'wake_action',
  'due_in_seconds',
  'expiry_seconds',
  'job_wait_seconds',
  'check_after_seconds',
  'recheck_in_seconds',
  'check_event',
  'epoch_after_claim',
  'write_lands',
  'more_now',
  'poll_seconds',
  'beat_after_seconds',
  'pass_on_hint',
  'pickup_after_seconds',
  'round_visits_job',
  'step_parks',
  'step_outcome',
]

describe('the vendored waits card is the one PIN names', () => {
  it('waits.t27 and waits.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(`queen/waits.t27 sha256 ${sha('queen/waits.t27')}`)
    expect(pin).toContain(`queen/waits.wasm sha256 ${sha('queen/waits.wasm')}`)
  })

  it('exports every fn of the card, and PIN lists them', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    const spec = readFileSync(
      join(DEFAULT_SPECS_ROOT, 'queen/waits.t27'),
      'utf8',
    )
    const declared = [...spec.matchAll(/^pub fn ([a-z_0-9]+)\(/gm)].map(
      (m) => m[1],
    )
    expect(declared).toEqual(CARD_FNS)
    const module = new WebAssembly.Module(
      readFileSync(join(DEFAULT_SPECS_ROOT, WAITS_CARD)),
    )
    const exported = WebAssembly.Module.exports(module).map((e) => e.name)
    for (const fn of CARD_FNS) expect(exported).toContain(fn)
    expect(pin).toContain(`exports every fn of the card: ${CARD_FNS.join(',')}`)
    expect(loadCardWasm(WAITS_CARD)).toBeDefined()
  })
})

describe('the card, as the runtime calls it', () => {
  it('moves only a waiting row', () => {
    expect(stateAfter(W_WAITING, EV_RESOLVE)).toBe(W_RESOLVED)
    expect(stateAfter(W_WAITING, EV_CANCEL)).toBe(W_CANCELLED)
    expect(stateAfter(W_WAITING, EV_RECHECK)).toBe(W_WAITING)
    expect(stateAfter(W_RESOLVED, EV_CANCEL)).toBe(W_RESOLVED)
    expect(transitionAllowed(W_WAITING, W_EXPIRED)).toBe(true)
    expect(transitionAllowed(W_RESOLVED, W_EXPIRED)).toBe(false)
    expect(wakesOwner(W_CANCELLED)).toBe(false)
    expect(deliversWake(false, W_RESOLVED)).toBe(false)
    expect(deliversWake(true, W_EXPIRED)).toBe(true)
    expect(resolutionLands(W_WAITING, false)).toBe(true)
    expect(resolutionLands(W_WAITING, true)).toBe(false)
    expect(resolutionLands(W_EXPIRED, false)).toBe(false)
  })

  it('wakes a row when its time passed or its key resolved', () => {
    expect(wakeAction(W_WAITING, false, false, true, 0, 100)).toBe(WA_RESOLVE)
    expect(wakeAction(W_WAITING, false, true, true, 0, 100)).toBe(WA_CHECK)
    expect(wakeAction(W_WAITING, false, true, true, 0, 0)).toBe(WA_EXPIRE)
    expect(wakeAction(W_WAITING, true, true, true, 0, 0)).toBe(WA_RESOLVE)
    expect(wakeAction(W_WAITING, false, true, true, 9, 100)).toBe(WA_NONE)
    expect(dueInSeconds(W_WAITING, false, true, 30, 100)).toBe(30)
    expect(dueInSeconds(W_RESOLVED, false, true, 0, 0)).toBe(NEVER)
    expect(secondsTo(1000, 1000)).toBe(0)
    expect(secondsTo(1000, 1001)).toBe(1)
    expect(secondsTo(5000, null)).toBe(0)
  })

  it('caps a wait at a day and keeps a job step its first deadline', () => {
    expect(expirySeconds(0)).toBe(86_400)
    expect(expirySeconds(99_999_999)).toBe(86_400)
    expect(jobWaitSeconds(82_800)).toBe(3600)
    expect(jobWaitSeconds(-5)).toBe(86_400)
  })

  it('checks a run less and less: 24 reads in 90 minutes', () => {
    expect(checkAfterSeconds(0)).toBe(30)
    expect(checkAfterSeconds(3)).toBe(CHECK_CAP_SECONDS)
    expect(recheckInSeconds(3, 100)).toBe(100)
    expect(checkEvent(true)).toBe(EV_RESOLVE)
    expect(checkEvent(false)).toBe(EV_RECHECK)
    let t = checkAfterSeconds(0)
    let checks = 0
    while (t <= 5400) {
      checks += 1
      t += checkAfterSeconds(checks)
    }
    expect(checks).toBe(24)
  })

  it('refuses a write at a stale epoch, across the u64 boundary too', () => {
    const a = epochAfterClaim(4n)
    const b = epochAfterClaim(a)
    expect(a).toBe(5n)
    expect(writeLands(b, a)).toBe(false)
    expect(writeLands(b, b)).toBe(true)
    const high = 2n ** 63n + 7n
    expect(epochAfterClaim(high)).toBe(high + 1n)
    expect(writeLands(high + 1n, high)).toBe(false)
    expect(moreNow(CLAIM_BATCH)).toBe(true)
    expect(moreNow(CLAIM_BATCH - 1)).toBe(false)
  })

  it('polls whether it listens or not, and coalesces hints', () => {
    expect(pollSeconds(true)).toBe(pollSeconds(false))
    expect(beatAfterSeconds(true, false, 0)).toBe(15)
    expect(beatAfterSeconds(true, true, 3)).toBe(3)
    expect(beatAfterSeconds(false, true, 0)).toBe(1)
    expect(passOnHint(false, false)).toBe(PH_NOW)
    expect(passOnHint(true, false)).toBe(PH_QUEUE)
    expect(passOnHint(true, true)).toBe(PH_DROP)
    expect(pickupAfterSeconds(true, 45)).toBe(15)
    expect(pickupAfterSeconds(false, 0)).toBe(0)
  })

  it("answers a job's step with jobs.t27's own outcome numbers", () => {
    expect(JOB_O_PASS).toBe(O_PASS)
    expect(JOB_O_NOT_YET).toBe(O_NOT_YET)
    expect(JOB_O_FAIL).toBe(O_FAIL)
    expect(stepOutcome(W_RESOLVED, true)).toBe(O_PASS)
    expect(stepOutcome(W_RESOLVED, false)).toBe(O_FAIL)
    expect(stepOutcome(W_EXPIRED, true)).toBe(O_FAIL)
    expect(stepOutcome(W_WAITING, true)).toBe(O_NOT_YET)
    expect(roundVisitsJob(true)).toBe(false)
    expect(stepParks(true, false)).toBe(true)
    expect(stepParks(true, true)).toBe(false)
    expect(stepParks(false, false)).toBe(false)
  })
})

describe('the switch', () => {
  it('is off unless TRIOS_QUEEN_WAITS=rows', () => {
    expect(waitsEnabled({})).toBe(false)
    expect(waitsEnabled({ TRIOS_QUEEN_WAITS: 'on' })).toBe(false)
    expect(waitsEnabled({ TRIOS_QUEEN_WAITS: 'rows' })).toBe(true)
    expect(waitsEnabled({ TRIOS_QUEEN_WAITS: ' ROWS ' })).toBe(true)
  })

  it('refuses to make or resolve a wait while off', async () => {
    const app = createQueenWaitsRoute({
      enabled: () => false,
      pool: () => null,
    })
    const made = await app.request('/', {
      method: 'POST',
      body: JSON.stringify({ key: 'k' }),
    })
    expect(made.status).toBe(503)
    const resolved = await app.request('/resolve', {
      method: 'POST',
      body: JSON.stringify({ key: 'k' }),
    })
    expect(resolved.status).toBe(503)
  })
})

describe('a GitHub run, checked by its id', () => {
  const row = (key: string): WaitRow => ({
    id: 1,
    owner: 'job:1',
    key,
    state: W_WAITING,
    wakeAt: 0,
    expiresAt: 1,
    dueAt: 0,
    checks: 0,
    epoch: 1n,
    resolution: null,
    detail: {},
  })

  it('reads the run the key names and says whether it completed', async () => {
    const asked: string[] = []
    const check = githubRunResolver(async (url) => {
      asked.push(url)
      return {
        status: 200,
        body: {
          status: 'completed',
          conclusion: 'success',
          html_url: 'https://github.com/gHashTag/t27/actions/runs/37900501380',
        },
      }
    })
    const answer = await check(row(ghRunKey('gHashTag/t27', 37900501380)))
    expect(asked).toEqual([
      'https://api.github.com/repos/gHashTag/t27/actions/runs/37900501380',
    ])
    expect(answer.completed).toBe(true)
    expect((answer.resolution as { conclusion: string }).conclusion).toBe(
      'success',
    )
  })

  it('treats a run in the queue, a failed read and a strange key as not completed', async () => {
    const queued = githubRunResolver(async () => ({
      status: 200,
      body: { status: 'queued', conclusion: null },
    }))
    expect((await queued(row(ghRunKey('a/b', 1)))).completed).toBe(false)
    const down = githubRunResolver(async () => {
      throw new Error('ECONNRESET')
    })
    expect((await down(row(ghRunKey('a/b', 1)))).completed).toBe(false)
    const gone = githubRunResolver(async () => ({ status: 404, body: null }))
    expect((await gone(row(ghRunKey('a/b', 1)))).completed).toBe(false)
    let read = false
    const strange = githubRunResolver(async () => {
      read = true
      return { status: 200, body: { status: 'completed' } }
    })
    expect((await strange(row('gh-run:not-a-repo:x'))).completed).toBe(false)
    expect(read).toBe(false)
  })
})
