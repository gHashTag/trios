/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ROUND HANDS ITS ISSUES TO THE BEE ACTORS (queen-bee-actors-round.ts,
 * behind TRIOS_QUEEN_DISPATCH=actors; gHashTag/trios#1731). The round's
 * choose and dispatch are stand-ins that record what they were asked; the
 * store answers the dispatch rows a bee's end is read from, and the cancel
 * control.t27 writes. The dispatcher is the process's one (a module
 * singleton on the real clock), so these tests run in order and each uses
 * its own issues. The tests that wait out a turn bound or a lease run on
 * fake timers.
 */

import { afterAll, describe, expect, it, jest, spyOn } from 'bun:test'
import type { Pool } from 'pg'
import { TURN_MAX_SECONDS } from '../../src/api/services/queen-actors-card.gen'
import {
  beeEndOfRow,
  dispatchByActors,
  handRoundToBeeActors,
  type RoundHandoff,
} from '../../src/api/services/queen-bee-actors-round'
import { publishEvent } from '../../src/api/services/queen-control'
import {
  DISPATCH_OUTCOME_LABELS,
  type DispatchOutcome,
} from '../../src/api/services/queen-dispatch'
import {
  EF_CANCEL,
  EF_NONE,
  EF_PROVIDER,
  HB_NEVER,
  LEASE_TTL_SECONDS,
} from '../../src/api/services/queen-dispatch-exit-card.gen'
import { logger } from '../../src/lib/logger'

interface Row {
  issue: number
  conversation_id: string
  outcome: string | null
  finished_at: Date | null
}

/** The dispatch rows, and the statements the bee actors sent. */
const rows = new Map<number, Row>()
const cancels: Array<{ issue: number; note: string }> = []
let refuseCancel = false
let refuseRead = false
let seq = 0
const store = {
  query: async (sql: string, params: unknown[] = []) => {
    if (sql.includes('FROM queen_dispatch WHERE issue = ANY')) {
      if (refuseRead) throw new Error('the dispatch table is locked')
      const wanted = new Set(params[0] as number[])
      return { rows: [...rows.values()].filter((r) => wanted.has(r.issue)) }
    }
    if (sql.includes('INSERT INTO queen_event_counter'))
      return { rows: [{ seq: ++seq }], rowCount: 1 }
    if (sql.includes('UPDATE queen_dispatch d')) {
      if (refuseCancel) throw new Error('the store is read-only')
      const issue = params[0] as number
      cancels.push({ issue, note: String(params[1]) })
      const r = rows.get(issue)
      if (!r) return { rows: [], rowCount: 0 }
      const running = r.finished_at === null
      r.finished_at ??= new Date()
      r.outcome ??= 'cancelled'
      return {
        rows: [{ running, conversation_id: r.conversation_id, was: '' }],
        rowCount: 1,
      }
    }
    if (sql.includes('UPDATE queen_task_lease'))
      return { rows: [{ fence: 2 }], rowCount: 1 }
    return { rows: [], rowCount: 0 }
  },
}
const pool = store as unknown as Pool

/** A round over `board`, whose choose and dispatch record their calls. */
function round(
  candidates: number[],
  answer: (issue: number) => {
    paths: string[] | null
    out?: Partial<DispatchOutcome>
  },
) {
  const chosen: Array<{ issue: number; tasks: unknown[] }> = []
  const dispatched: Array<{ issue: number; paths: string[] }> = []
  const r: RoundHandoff = {
    candidates,
    board: [{ board: 'read by the round' }],
    choose: async (issue, tasks) => {
      chosen.push({ issue, tasks })
      return answer(issue).paths
    },
    boardTask: (issue, paths) => ({ issue, paths }),
    dispatch: async (issue, paths) => {
      dispatched.push({ issue, paths })
      const n = dispatched.filter((d) => d.issue === issue).length
      const out: DispatchOutcome = {
        started: true,
        issue,
        branch: `bee/${issue}`,
        detail: '',
        conversationId: `c${issue}-${n}`,
        ...answer(issue).out,
      }
      if (out.started)
        rows.set(issue, {
          issue,
          conversation_id: out.conversationId ?? '',
          outcome: null,
          finished_at: null,
        })
      return out
    },
  }
  return { r, chosen, dispatched }
}

const tick = () => new Promise<void>((r) => setImmediate(r))
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
async function until(cond: () => boolean, ms = 3000): Promise<void> {
  const end = Date.now() + ms
  while (!cond() && Date.now() < end) await sleep(10)
  expect(cond()).toBe(true)
}
/** Under fake timers: let every promise and setImmediate settle. */
async function settled(): Promise<void> {
  for (let i = 0; i < 50; i++) await tick()
}
const ended = (issue: number, outcome: string) => {
  const r = rows.get(issue) as Row
  r.outcome = outcome
  r.finished_at = new Date()
}

const savedLanes = process.env.TRIOS_QUEEN_ACTOR_LANES
afterAll(() => {
  if (savedLanes === undefined) delete process.env.TRIOS_QUEEN_ACTOR_LANES
  else process.env.TRIOS_QUEEN_ACTOR_LANES = savedLanes
})

describe('the switch and the reading of a row', () => {
  it('only TRIOS_QUEEN_DISPATCH=actors hands the round to the bee actors', () => {
    expect(dispatchByActors({})).toBe(false)
    expect(dispatchByActors({ TRIOS_QUEEN_DISPATCH: 'loop' })).toBe(false)
    expect(dispatchByActors({ TRIOS_QUEEN_DISPATCH: ' Actors ' })).toBe(true)
  })

  it("a finished row's outcome is the end the card reads", () => {
    expect(beeEndOfRow(DISPATCH_OUTCOME_LABELS.finished)).toEqual({
      completion: true,
      errorFrame: EF_NONE,
      http: 0,
      heartbeatAge: 0,
      leaseHeld: true,
    })
    expect(
      beeEndOfRow(`${DISPATCH_OUTCOME_LABELS.providerQuotaExhausted} (zai)`)
        .errorFrame,
    ).toBe(EF_PROVIDER)
    expect(beeEndOfRow('cancelled').errorFrame).toBe(EF_CANCEL)
    const reaped = beeEndOfRow('reaped: no heartbeat')
    expect([reaped.heartbeatAge, reaped.leaseHeld]).toEqual([HB_NEVER, false])
    // null: the row now belongs to another dispatch of the issue
    const lost = beeEndOfRow(null)
    expect([lost.completion, lost.heartbeatAge, lost.leaseHeld]).toEqual([
      false,
      HB_NEVER,
      false,
    ])
  })
})

describe('a round handed to the bee actors', () => {
  it('on one lane: the second candidate waits for the first bee, whose end comes from its row on task.ended; queend then sees that bee', async () => {
    process.env.TRIOS_QUEEN_ACTOR_LANES = '1'
    const { r, chosen, dispatched } = round([201, 202], (issue) => ({
      paths: issue === 201 ? ['specs/a.t27'] : ['specs/b.t27'],
    }))
    handRoundToBeeActors(pool, r)
    await until(() => dispatched.length === 1)
    expect(dispatched[0]).toEqual({ issue: 201, paths: ['specs/a.t27'] })
    // a task.ended while the row still runs settles nothing
    await publishEvent(pool, 'queen/task.ended', { issue: 201 })
    await sleep(100)
    expect(dispatched.length).toBe(1)
    expect(chosen.map((c) => c.issue)).toEqual([201])

    ended(201, DISPATCH_OUTCOME_LABELS.finished)
    await publishEvent(pool, 'queen/task.ended', { issue: 201 })
    await until(() => dispatched.length === 2)
    expect(dispatched[1]).toEqual({ issue: 202, paths: ['specs/b.t27'] })
    // queend was asked about 202 against the board plus the bee of 201
    const asked = chosen.find((c) => c.issue === 202)
    expect(asked?.tasks).toEqual([
      { board: 'read by the round' },
      { issue: 201, paths: ['specs/a.t27'] },
    ])
    // a finished bee is done: no second dispatch of 201
    await sleep(100)
    expect(dispatched.filter((d) => d.issue === 201).length).toBe(1)
    ended(202, DISPATCH_OUTCOME_LABELS.finished)
    await publishEvent(pool, 'queen/lease.expired', { issue: 202 })
    await sleep(100)
  })

  it('a read of the dispatch rows that fails breaks neither the bus nor the round, and the next read settles the bee', async () => {
    process.env.TRIOS_QUEEN_ACTOR_LANES = '1'
    const { r, dispatched } = round([209, 210], () => ({
      paths: ['specs/g.t27'],
    }))
    handRoundToBeeActors(pool, r)
    await until(() => dispatched.length === 1)
    expect(dispatched[0].issue).toBe(209)
    ended(209, DISPATCH_OUTCOME_LABELS.finished)
    refuseRead = true
    // the bus's read and the next round's read both fail: 209 is not
    // settled, so 210 still waits for the one lane
    await publishEvent(pool, 'queen/task.ended', { issue: 209 })
    const next = round([], () => ({ paths: ['specs/g.t27'] }))
    handRoundToBeeActors(pool, next.r)
    await sleep(100)
    expect(dispatched.length + next.dispatched.length).toBe(1)
    refuseRead = false
    await publishEvent(pool, 'queen/task.ended', { issue: 209 })
    await until(() => next.dispatched.length === 1)
    expect(next.dispatched[0].issue).toBe(210)
    ended(210, DISPATCH_OUTCOME_LABELS.finished)
    await publishEvent(pool, 'queen/task.ended', { issue: 210 })
    await sleep(50)
  })

  it('an issue queend refuses is not dispatched, and a row taken by another dispatch restarts the bee at once', async () => {
    process.env.TRIOS_QUEEN_ACTOR_LANES = '4'
    const { r, dispatched } = round([203, 204], (issue) => ({
      paths: issue === 203 ? ['specs/c.t27'] : null,
    }))
    handRoundToBeeActors(pool, r)
    await until(() => dispatched.length === 1)
    await sleep(100)
    expect(dispatched.map((d) => d.issue)).toEqual([203])
    // the row now names another dispatch: this bee's end is not its own
    ;(rows.get(203) as Row).conversation_id = 'someone else'
    // the next round settles it (dispatch_exit: DX_KILLED, RD_NOW), and the
    // restart is admitted against that round
    const next = round([], () => ({ paths: ['specs/c.t27'] }))
    handRoundToBeeActors(pool, next.r)
    await until(() => next.dispatched.length === 1)
    expect(next.dispatched[0]).toEqual({ issue: 203, paths: ['specs/c.t27'] })
    expect(dispatched.length).toBe(1)
    ended(203, DISPATCH_OUTCOME_LABELS.finished)
    handRoundToBeeActors(pool, round([], () => ({ paths: null })).r)
    await sleep(100)
  })

  it('a refusal because the task lease is held stands down and is tried again after the lease TTL; any other refusal is not', async () => {
    jest.useFakeTimers()
    try {
      const { r, dispatched } = round([205, 206], (issue) => ({
        paths: ['specs/d.t27'],
        out: {
          started: false,
          detail: issue === 205 ? 'task lease held by queen-2' : 'no key free',
        },
      }))
      handRoundToBeeActors(pool, r)
      await settled()
      expect(dispatched.map((d) => d.issue).sort()).toEqual([205, 206])
      jest.advanceTimersByTime((LEASE_TTL_SECONDS - 1) * 1000)
      await settled()
      expect(dispatched.length).toBe(2)
      jest.advanceTimersByTime(2000)
      await settled()
      expect(dispatched.map((d) => d.issue).sort()).toEqual([205, 205, 206])
    } finally {
      jest.useRealTimers()
    }
  })

  it('a bee past its turn bound is cancelled through control.t27, and the cancelled row settles it', async () => {
    jest.useFakeTimers()
    try {
      const { r, dispatched } = round([207], () => ({ paths: ['specs/e.t27'] }))
      handRoundToBeeActors(pool, r)
      await settled()
      expect(dispatched.length).toBe(1)
      expect(cancels.filter((c) => c.issue === 207)).toEqual([])
      jest.advanceTimersByTime(TURN_MAX_SECONDS * 1000 + 1000)
      await settled()
      const asked = cancels.filter((c) => c.issue === 207)
      expect(asked.length).toBe(1)
      expect(asked[0].note).toContain(
        'Cancelled by bee actors: its turn passed the bound',
      )
      // the cancel ended the row; the settle after it ended the bee, and a
      // killed bee restarts at once (dispatch_exit: RD_NOW)
      expect(dispatched.length).toBe(2)
      ended(207, DISPATCH_OUTCOME_LABELS.finished)
      handRoundToBeeActors(pool, round([], () => ({ paths: null })).r)
      await settled()
    } finally {
      jest.useRealTimers()
    }
  })

  it('a cancel the store refuses is logged with its issue, and the bee keeps its row', async () => {
    const warn = spyOn(logger, 'warn').mockImplementation(() => {})
    jest.useFakeTimers()
    try {
      refuseCancel = true
      const { r, dispatched } = round([208], () => ({ paths: ['specs/f.t27'] }))
      handRoundToBeeActors(pool, r)
      await settled()
      expect(dispatched.length).toBe(1)
      jest.advanceTimersByTime(TURN_MAX_SECONDS * 1000 + 1000)
      await settled()
      const said = warn.mock.calls.filter(
        (c) => c[0] === 'Bee actors could not cancel a bee',
      )
      expect(said.length).toBe(1)
      expect(said[0][1]).toEqual({
        issue: 208,
        error: 'the store is read-only',
      })
      // nothing ended the row, so nothing restarted the bee
      expect((rows.get(208) as Row).finished_at).toBeNull()
      expect(dispatched.length).toBe(1)
    } finally {
      refuseCancel = false
      jest.useRealTimers()
      warn.mockRestore()
    }
  })
})
