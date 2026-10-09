/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BEE DISPATCHER AT ITS EDGES (queen-bee-actors.ts; keyed.t27,
 * dispatch_exit.t27; gHashTag/trios#1731): an issue actor stopped while its
 * bee runs, a policy or a start that fails, lanes that shrink while the
 * policy answers, an end that rejects, a review that sends the work back,
 * and the dispatcher's own stop. Fake bees under a virtual clock, as in
 * queen-bee-actors.test.ts.
 */

import { describe, expect, it } from 'bun:test'
import {
  createActorSystem,
  type Pid,
} from '../../src/api/services/queen-actors'
import { X_NORMAL } from '../../src/api/services/queen-actors-card.gen'
import {
  type BeeDispatchDeps,
  type BeeEnd,
  type BeeEvent,
  beeDispatcher,
} from '../../src/api/services/queen-bee-actors'
import {
  DX_KILLED,
  EF_NONE,
  LEASE_TTL_SECONDS,
  RD_NOW,
} from '../../src/api/services/queen-dispatch-exit-card.gen'
import { VirtualClock } from './queen-virtual-clock'

const OK: BeeEnd = {
  completion: true,
  errorFrame: EF_NONE,
  http: 200,
  heartbeatAge: 0,
  leaseHeld: true,
}

/** Fake bees, each ended by the test; deps may be overridden. */
function harness(over: Partial<BeeDispatchDeps> = {}) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock, { slices: false })
  const ends: Array<{
    issue: number
    end: (e: BeeEnd) => void
    fail: (e: Error) => void
  }> = []
  const events: BeeEvent[] = []
  const d = beeDispatcher(sys, {
    lanes: () => 2,
    admit: async () => true,
    observe: (e) => void events.push(e),
    start: async (issue) => {
      let end: (e: BeeEnd) => void = () => {}
      let fail: (e: Error) => void = () => {}
      const ended = new Promise<BeeEnd>((r, j) => {
        end = r
        fail = j
      })
      ends.push({ issue, end, fail })
      return { claim: true, work: { ended, cancel: () => {} } }
    },
    ...over,
  })
  return { clock, sys, d, ends, events }
}

describe('an issue actor stopped while its bee runs', () => {
  it('ends normally, its lane stays held so a new ready is refused, and the lane comes back when the old bee ends', async () => {
    let admitted = 0
    const { clock, sys, d, ends, events } = harness({
      admit: async () => {
        admitted++
        return true
      },
    })
    d.ready(31)
    await clock.runUntil(10)
    expect(ends.length).toBe(1)
    const first = d.issues.pidOf(31) as Pid
    let firstReason = -1
    sys.onExit((pid, reason) => {
      if (pid === first) firstReason = reason
    })

    // a stopper asks the trapping actor to end: it ends as `normal`
    d.issues.passivate(31)
    await clock.runUntil(20)
    expect(sys.alive(first)).toBe(false)
    expect(firstReason).toBe(X_NORMAL)
    expect(d.lanesInUse()).toBe(1)

    // a new incarnation asks for a lane the old bee still holds: refused
    d.ready(31)
    await clock.runUntil(30)
    expect(ends.length).toBe(1)
    expect(admitted).toBe(1)
    expect(d.lanesInUse()).toBe(1)
    expect(d.waiting()).toEqual([])

    // the old bee ends: nobody restarts it, and its lane goes back
    ends[0].end(OK)
    await clock.runUntil(40)
    expect(d.lanesInUse()).toBe(0)
    expect(events.filter((e) => e.kind === 'ended')).toEqual([])
    d.ready(31)
    await clock.runUntil(50)
    expect(ends.length).toBe(2)
    expect(d.lanesInUse()).toBe(1)
  })
})

describe('the admission', () => {
  it('reads a policy that fails as a refusal: no bee, no lane, and a later ready asks again', async () => {
    let failing = true
    const { clock, d, ends } = harness({
      admit: async () => {
        if (failing) throw new Error('queend is down')
        return true
      },
    })
    d.ready(41)
    await clock.runUntil(10)
    expect(ends.length).toBe(0)
    expect(d.lanesInUse()).toBe(0)
    failing = false
    d.ready(41)
    await clock.runUntil(20)
    expect(ends.map((e) => e.issue)).toEqual([41])
  })

  it('makes an issue wait when the lanes shrank while the policy answered, and starts it once a lane is free', async () => {
    let lanes = 2
    let answer: (yes: boolean) => void = () => {}
    const { clock, d, ends } = harness({
      lanes: () => lanes,
      admit: async (issue) =>
        issue === 42 ? new Promise<boolean>((r) => (answer = r)) : true,
    })
    d.ready(40)
    await clock.runUntil(10)
    expect(ends.map((e) => e.issue)).toEqual([40])
    d.ready(42)
    await clock.runUntil(20)
    // the operator takes a lane away while the policy is asked about 42
    lanes = 1
    answer(true)
    await clock.runUntil(30)
    expect(ends.map((e) => e.issue)).toEqual([40])
    expect(d.waiting()).toEqual([42])
    // 40's bee ends and frees the one lane: 42 is woken and asked again
    ends[0].end(OK)
    await clock.runUntil(40)
    answer(true)
    await clock.runUntil(50)
    expect(ends.map((e) => e.issue)).toEqual([40, 42])
    expect(d.waiting()).toEqual([])
  })
})

describe('a start', () => {
  it('that fails is read as a lost claim: the lane goes back, the issue stands down, and it is tried again after the lease TTL', async () => {
    let starts = 0
    const { clock, d, events } = harness({
      start: async () => {
        starts++
        throw new Error('the runner refused the container')
      },
    })
    d.ready(51)
    await clock.runUntil(10)
    expect(d.lanesInUse()).toBe(0)
    expect(d.stats.started).toBe(0)
    expect(d.stats.standDowns).toBe(1)
    expect(events).toEqual([{ kind: 'stood-down', issue: 51 }])
    // retry_after_lost_seconds with no renewal known: the whole lease TTL
    await clock.runUntil((LEASE_TTL_SECONDS - 1) * 1000)
    expect(starts).toBe(1)
    await clock.runUntil((LEASE_TTL_SECONDS + 1) * 1000)
    expect(starts).toBe(2)
  })
})

describe("a bee's end", () => {
  it('that rejects is read as an unknown end: a lost bee, started again at once', async () => {
    const { clock, d, ends, events } = harness()
    d.ready(61)
    await clock.runUntil(10)
    ends[0].fail(new Error('the runner vanished'))
    await clock.runUntil(20)
    expect(events.filter((e) => e.kind === 'ended')).toEqual([
      { kind: 'ended', issue: 61, reason: DX_KILLED, decision: RD_NOW },
    ])
    expect(ends.map((e) => e.issue)).toEqual([61, 61])
  })
})

describe('a review that sends the work back', () => {
  it('starts a new attempt of an issue whose bee was done', async () => {
    const { clock, d, ends } = harness()
    d.ready(71)
    await clock.runUntil(10)
    ends[0].end(OK)
    await clock.runUntil(20)
    // done: a ready starts nothing, a send-back does
    d.ready(71)
    await clock.runUntil(30)
    expect(ends.length).toBe(1)
    d.sentBack(71)
    await clock.runUntil(40)
    expect(ends.map((e) => e.issue)).toEqual([71, 71])
  })
})

describe("the dispatcher's stop", () => {
  it('stops every issue actor, the last started first, while their bees run on', async () => {
    const { clock, sys, d, ends } = harness()
    d.ready(81)
    d.ready(82)
    await clock.runUntil(10)
    const pids = [d.issues.pidOf(81), d.issues.pidOf(82)] as Pid[]
    const order: Pid[] = []
    sys.onExit((pid) => {
      if (pids.includes(pid)) order.push(pid)
    })
    const stopped = d.stop()
    await clock.runUntil(20)
    await stopped
    expect(order).toEqual([pids[1], pids[0]])
    expect(d.issues.active()).toBe(0)
    // nothing cancelled the bees: their work still waits on its end
    expect(ends.length).toBe(2)
  })
})
