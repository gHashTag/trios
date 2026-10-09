/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE REVIEWER STARTED AS ACTORS IN A PROCESS (startReviewerActors,
 * gHashTag/trios#1731): the switch to the round, the bus that wakes it, its
 * telemetry and adaptive flags, a capacity read that fails, and a root that
 * gives up. The store answers only the lease read; the review is a stand-in.
 * The reviewer runs on the real clock, so the give-up runs on fake timers.
 */

import { afterEach, describe, expect, it, jest, spyOn } from 'bun:test'
import type { Pool } from 'pg'
import { liveActorTelemetry } from '../../src/api/services/queen-actors-telemetry'
import { publishEvent } from '../../src/api/services/queen-control'
import { startReviewerActors } from '../../src/api/services/queen-review-actors'
import {
  type Judged,
  type ReviewFn,
  reviewerRunning,
} from '../../src/api/services/queen-review-loop'
import { logger } from '../../src/lib/logger'

const LEASE = 'FROM queen_lease'

/** A store that holds the lease (or not, or fails), counting its reads. */
function store(answer: 'held' | 'not held' | 'fails') {
  let leaseReads = 0
  let seq = 0
  const pool = {
    query: async (sql: string) => {
      if (sql.includes(LEASE)) {
        leaseReads++
        if (answer === 'fails') throw new Error('the lease table is gone')
        const n = answer === 'held' ? 1 : 0
        return { rows: n ? [{ '?column?': 1 }] : [], rowCount: n }
      }
      if (sql.includes('INSERT INTO queen_event_counter'))
        return { rows: [{ seq: ++seq }], rowCount: 1 }
      return { rows: [], rowCount: 0 }
    },
  }
  return { pool: pool as unknown as Pool, leaseReads: () => leaseReads }
}

const nothing: Judged = { acted: [], strays: [], tally: [] }
const reviewed: number[] = []
const review: ReviewFn = async (_pool, _overrides, scope) => {
  reviewed.push(...scope.issues)
  return nothing
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
async function until(cond: () => boolean, ms = 3000): Promise<void> {
  const end = Date.now() + ms
  while (!cond() && Date.now() < end) await sleep(10)
  expect(cond()).toBe(true)
}

const ENV = [
  'TRIOS_QUEEN_ACTORS_TELEMETRY',
  'TRIOS_QUEEN_REVIEWER_ADAPTIVE',
  'TRIOS_QUEEN_TURN_STOP',
] as const
const saved = Object.fromEntries(ENV.map((k) => [k, process.env[k]]))
const stops: Array<() => void> = []
const spies: Array<{ mockRestore: () => void }> = []
afterEach(() => {
  for (const s of stops.splice(0)) s()
  for (const s of spies.splice(0)) s.mockRestore()
  for (const k of ENV)
    if (saved[k] === undefined) delete process.env[k]
    else process.env[k] = saved[k]
  reviewed.length = 0
})
function logged(level: 'info' | 'warn') {
  const spy = spyOn(logger, level).mockImplementation(() => {})
  spies.push(spy)
  return (msg: string) => spy.mock.calls.filter((c) => c[0] === msg)
}

describe('the reviewer started as actors', () => {
  it('stands the round aside and wakes on task.ended alone; once stopped it hands reviewing back and a task.ended reads no lease', async () => {
    const s = store('not held')
    const stop = startReviewerActors(s.pool, 'queen', review, async () => [])
    expect(reviewerRunning()).toBe(true)
    // the start wakes the intake once: it reads the lease
    await until(() => s.leaseReads() === 1)
    await publishEvent(s.pool, 'queen/task.ended', { issue: 1 })
    await until(() => s.leaseReads() === 2)
    // a bus event that is not task.ended wakes nobody
    await publishEvent(s.pool, 'queen/task.created', { issue: 2 })
    await sleep(50)
    expect(s.leaseReads()).toBe(2)
    stop()
    expect(reviewerRunning()).toBe(false)
    await publishEvent(s.pool, 'queen/task.ended', { issue: 3 })
    await sleep(50)
    expect(s.leaseReads()).toBe(2)
  })

  it('with TRIOS_QUEEN_ACTORS_TELEMETRY=on the live metrics read its system, until it stops', async () => {
    process.env.TRIOS_QUEEN_ACTORS_TELEMETRY = 'on'
    const s = store('not held')
    const stop = startReviewerActors(s.pool, 'queen', review, async () => [])
    await until(() => s.leaseReads() === 1)
    const tel = liveActorTelemetry()
    expect(tel).toBeDefined()
    const snap = tel?.snapshot() as unknown as {
      enabled: boolean
      kinds: Record<string, unknown>
      supervisors: Record<string, unknown>
    }
    expect(snap.enabled).toBe(true)
    expect(Object.keys(snap.kinds)).toContain('reviewer-intake')
    expect(Object.keys(snap.supervisors)).toEqual(
      expect.arrayContaining(['queen-root', 'reviewer-domain']),
    )
    stop()
    expect(liveActorTelemetry()).toBeUndefined()
  })

  it('adaptive: each wake reads the capacity, and a change of the pool is logged', async () => {
    process.env.TRIOS_QUEEN_REVIEWER_ADAPTIVE = '1'
    const info = logged('info')
    const s = store('held')
    const asked: number[][] = []
    const stop = startReviewerActors(
      s.pool,
      'queen',
      review,
      async () => [41, 42],
      async (_pool, reserved) => {
        asked.push(reserved)
        return { freeLanes: 2, freeMb: 8192, mbPerReview: 500 }
      },
    )
    stops.push(stop)
    await until(() => reviewed.length === 2)
    expect(asked[0]).toEqual([])
    expect(reviewed.sort()).toEqual([41, 42])
    const sized = info('Queen reviewer pool')
    expect(sized.length).toBeGreaterThan(0)
    expect((sized[0][1] as { lanes: number }).lanes).toBe(2)
    expect(
      (info('Queen reviewer starting as actors')[0][1] as { workers: string })
        .workers,
    ).toBe('adaptive')
  })

  it('a capacity read that fails is logged, and the intake goes on with its last reading', async () => {
    process.env.TRIOS_QUEEN_REVIEWER_ADAPTIVE = '1'
    const warn = logged('warn')
    const s = store('held')
    let reads = 0
    const stop = startReviewerActors(
      s.pool,
      'queen',
      review,
      async () => [],
      async () => {
        reads++
        throw new Error('cgroup unreadable')
      },
    )
    stops.push(stop)
    await until(() => reads === 1)
    await publishEvent(s.pool, 'queen/task.ended', { issue: 9 })
    await until(() => reads === 2)
    const said = warn('Queen reviewer could not read its capacity')
    expect(said.length).toBe(2)
    expect(said[0][1]).toEqual({ error: 'cgroup unreadable' })
    expect(reviewerRunning()).toBe(true)
  })

  it('whose lease read always fails: the intake crashes until the domain and then the root give up, and the round reviews again', async () => {
    const warn = logged('warn')
    jest.useFakeTimers()
    try {
      const s = store('fails')
      const stop = startReviewerActors(s.pool, 'queen', review, async () => [])
      stops.push(stop)
      expect(reviewerRunning()).toBe(true)
      // walk 30 minutes, a second at a time, letting each turn settle
      for (let t = 0; t < 1800 && reviewerRunning(); t++) {
        jest.advanceTimersByTime(1000)
        for (let i = 0; i < 20; i++)
          await new Promise<void>((r) => setImmediate(r))
      }
      expect(reviewerRunning()).toBe(false)
      const said = warn(
        'Queen reviewer actors gave up; the round reviews again',
      )
      expect(said.length).toBe(1)
      // every wake read the lease and crashed on it
      expect(s.leaseReads()).toBeGreaterThan(8)
    } finally {
      jest.useRealTimers()
    }
  })
})
