/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE WAITS SCHEDULER WHEN ITS STORE MISBEHAVES (gHashTag/t27
 * specs/queen/waits.t27, gHashTag/trios#1731). A pass that fails, a hint that
 * lands while a pass runs, a LISTEN that cannot be had, a stop before the
 * LISTEN connection came, and a scheduler whose every pass hangs until its
 * root gives up. The store here is a stand-in that answers only what the
 * scheduler asks; the rows themselves are tested against PostgreSQL in
 * tests/pglive/queen-waits-live.test.ts.
 */

import { afterEach, describe, expect, it, spyOn } from 'bun:test'
import type { Pool } from 'pg'
import {
  type ActorSystem,
  createActorSystem,
} from '../../src/api/services/queen-actors'
import { ROOT_MAX_RESTARTS } from '../../src/api/services/queen-actors-card.gen'
import { publishEvent } from '../../src/api/services/queen-control'
import {
  CLAIM_TTL_SECONDS,
  startWaitsActors,
  WAIT_POLL_SECONDS,
  waitsRunning,
  waitsTree,
} from '../../src/api/services/queen-waits'
import { logger } from '../../src/lib/logger'
import { VirtualClock } from './queen-virtual-clock'

const NEXT_DUE = 'SELECT min(due_at)'

/** A store that answers nextDueSeconds with "nothing waits", or as told. */
function store(
  clock: VirtualClock,
  opts: {
    /** Every query fails with this. */
    fail?: string
    /** Every query takes this long, on the virtual clock. */
    delayMs?: number
    /** Every query hangs for ever. */
    hang?: boolean
    connect?: () => Promise<unknown>
  } = {},
) {
  const asked: string[] = []
  const pool = {
    query: (sql: string) => {
      asked.push(sql)
      if (opts.hang) return new Promise(() => {})
      if (opts.fail) return Promise.reject(new Error(opts.fail))
      const answer = { rows: [{ due: null }], rowCount: 1 }
      if (!opts.delayMs) return Promise.resolve(answer)
      return new Promise((r) =>
        clock.after(opts.delayMs as number, () => r(answer)),
      )
    },
    connect: opts.connect ?? (() => new Promise(() => {})),
  }
  return {
    pool: pool as unknown as Pool,
    passes: () => asked.filter((s) => s.includes(NEXT_DUE)).length,
  }
}

const warnings: Array<ReturnType<typeof spyOn>> = []
afterEach(() => {
  for (const w of warnings.splice(0)) w.mockRestore()
})
function warns() {
  const w = spyOn(logger, 'warn').mockImplementation(() => {})
  warnings.push(w)
  return () => w.mock.calls.map((c) => String(c[0]))
}

function scheduler(sys: ActorSystem, pool: Pool) {
  const w = waitsTree(sys, { pool, resolvers: {}, onWake: () => {} })
  const h = w.tree.start(() => {})
  return {
    w,
    stop: () => {
      w.halt()
      h.stop()
    },
  }
}

describe('the waits scheduler', () => {
  it('a pass whose store fails is counted and logged, and the next poll passes again', async () => {
    const said = warns()
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock, { fail: 'connection refused' })
    const { w, stop } = scheduler(sys, s.pool)
    await clock.runUntil(1)
    expect(w.stats.failedPasses).toBe(1)
    expect(said()).toContain('Queen waits pass failed')
    // nothing is known of the next due row: the beat is the poll
    await clock.runUntil(WAIT_POLL_SECONDS * 1000 + 1)
    expect(w.stats.failedPasses).toBe(2)
    expect(w.stats.passes).toBe(2)
    stop()
  })

  it('a hint taken after a pass was asked for and before it began queues one more pass, right after it', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock, { delayMs: 1000 })
    const w = waitsTree(sys, { pool: s.pool, resolvers: {}, onWake: () => {} })
    const h = w.tree.start(() => {})
    // init asked for the first pass; this hint is a control message, taken
    // before that pass's data message (pass_on_hint: PH_QUEUE)
    w.hint()
    await clock.runUntil(1500)
    expect(w.stats.passes).toBe(2)
    await clock.runUntil(2500)
    expect(w.stats.passes).toBe(2)
    expect(w.idle()).toBe(true)
    w.halt()
    h.stop()
  })

  it('hints that land while a pass runs bring exactly one more pass, right after it', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock, { delayMs: 1000 })
    const { w, stop } = scheduler(sys, s.pool)
    await clock.runUntil(500)
    expect(w.stats.passes).toBe(1)
    expect(w.idle()).toBe(false)
    // three hints in the middle of the pass: one more pass, not three
    w.hint()
    w.hint()
    w.hint()
    await clock.runUntil(2600)
    expect(w.stats.passes).toBe(2)
    expect(w.idle()).toBe(true)
    // and then the poll, not sooner
    await clock.runUntil(2000 + WAIT_POLL_SECONDS * 1000 - 100)
    expect(w.stats.passes).toBe(2)
    await clock.runUntil(2000 + WAIT_POLL_SECONDS * 1000 + 1100)
    expect(w.stats.passes).toBe(3)
    stop()
  })
})

describe('the waits scheduler started in a process', () => {
  it('takes any bus event as a hint: a pass now, not at the next poll', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock)
    const stop = startWaitsActors(s.pool, {
      resolvers: {},
      onWake: () => {},
      sys,
    })
    await clock.runUntil(1000)
    const before = s.passes()
    await publishEvent(s.pool, 'queen/task.created', { issue: 1 })
    await clock.runUntil(1001)
    expect(s.passes()).toBe(before + 1)
    await stop()
  })

  it('gives its LISTEN connection back at stop even when the UNLISTEN fails', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const asked: string[] = []
    let released = 0
    const client = {
      on: () => {},
      query: async (sql: string) => {
        asked.push(sql)
        if (sql.startsWith('UNLISTEN'))
          throw new Error('the connection is gone')
        return { rows: [] }
      },
      release: () => {
        released++
      },
    }
    const s = store(clock, { connect: async () => client })
    const stop = startWaitsActors(s.pool, {
      resolvers: {},
      onWake: () => {},
      sys,
    })
    await clock.runUntil(1)
    expect(asked).toEqual(['LISTEN queen_wait'])
    await stop()
    expect(asked).toEqual(['LISTEN queen_wait', 'UNLISTEN queen_wait'])
    expect(released).toBe(1)
    expect(waitsRunning()).toBe(false)
  })

  it('whose LISTEN connection cannot be had says so, runs, and polls', async () => {
    const said = warns()
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock, {
      connect: () => Promise.reject(new Error('too many clients')),
    })
    const stop = startWaitsActors(s.pool, {
      resolvers: {},
      onWake: () => {},
      sys,
    })
    await clock.runUntil(1)
    expect(said()).toContain('Queen waits listen by polling only')
    expect(waitsRunning()).toBe(true)
    const first = s.passes()
    await clock.runUntil(WAIT_POLL_SECONDS * 1000 * 3 + 1)
    expect(s.passes()).toBe(first + 3)
    await stop()
    expect(waitsRunning()).toBe(false)
  })

  it('stopped before its LISTEN connection came: the connection goes back unused', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    let give: (client: unknown) => void = () => {}
    const asked: string[] = []
    let released = 0
    const client = {
      on: () => {},
      query: (sql: string) => {
        asked.push(sql)
        return Promise.resolve({ rows: [] })
      },
      release: () => {
        released++
      },
    }
    const s = store(clock, {
      connect: () => new Promise((r) => (give = r)),
    })
    const stop = startWaitsActors(s.pool, {
      resolvers: {},
      onWake: () => {},
      sys,
    })
    await clock.runUntil(1)
    await stop()
    give(client)
    await clock.runUntil(2)
    expect(released).toBe(1)
    expect(asked).toEqual([])
  })

  it('whose every pass hangs: each is killed at the claim, and past ROOT_MAX_RESTARTS the root gives up and says so', async () => {
    const said = warns()
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const s = store(clock, { hang: true })
    const stop = startWaitsActors(s.pool, {
      resolvers: {},
      onWake: () => {},
      sys,
    })
    await clock.runUntil(1)
    expect(waitsRunning()).toBe(true)
    // three passes, each killed at CLAIM_TTL_SECONDS, with backoffs between
    await clock.runUntil(CLAIM_TTL_SECONDS * 1000 * 10)
    expect(sys.stats.killed).toBe(ROOT_MAX_RESTARTS + 1)
    expect(waitsRunning()).toBe(false)
    expect(said()).toContain(
      'Queen waits scheduler gave up; jobs are asked by the round',
    )
    await stop()
  })
})
