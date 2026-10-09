/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BEE DISPATCHER AS KEYED ACTORS, MVP (queen-bee-actors.ts; keyed.t27,
 * dispatch_exit.t27, actors.t27). Each test drives the dispatcher with fake
 * bees under a virtual clock.
 */

import { describe, expect, it } from 'bun:test'
import { createActorSystem } from '../../src/api/services/queen-actors'
import { TURN_MAX_SECONDS } from '../../src/api/services/queen-actors-card.gen'
import {
  type BeeEnd,
  type BeeEvent,
  type BeeStart,
  beeDispatcher,
} from '../../src/api/services/queen-bee-actors'
import {
  DX_KILLED,
  DX_NORMAL,
  EF_CANCEL,
  EF_NONE,
  RD_NONE,
  RD_NOW,
} from '../../src/api/services/queen-dispatch-exit-card.gen'
import { PASSIVATE_IDLE_SECONDS } from '../../src/api/services/queen-keyed-card.gen'
import { VirtualClock } from './queen-virtual-clock'

const OK: BeeEnd = {
  completion: true,
  errorFrame: EF_NONE,
  http: 200,
  heartbeatAge: 0,
  leaseHeld: true,
}

/** A world of fake bees: each start makes one, ended by the test. */
function harness(
  lanes: number,
  claimLands: (issue: number, holder: string) => boolean = () => true,
) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock, { slices: false })
  const bees: Array<{
    issue: number
    at: number
    end: (e: BeeEnd) => void
    cancelled: number
    ended?: number
  }> = []
  const events: BeeEvent[] = []
  const d = beeDispatcher(sys, {
    lanes: () => lanes,
    admit: async () => true,
    observe: (e) => void events.push(e),
    start: async (issue, holder): Promise<BeeStart> => {
      if (!claimLands(issue, holder))
        return { claim: false, work: null, holderSinceRenewal: 60 }
      let end: (e: BeeEnd) => void = () => {}
      const ended = new Promise<BeeEnd>((r) => {
        end = r
      })
      const bee = {
        issue,
        at: clock.now(),
        cancelled: 0,
        end: (e: BeeEnd) => {
          if (bee.ended === undefined) bee.ended = clock.now()
          end(e)
        },
      } as (typeof bees)[number]
      bees.push(bee)
      return {
        claim: true,
        work: {
          ended,
          cancel: () => {
            bee.cancelled++
            // the runner sees the cancel at its next poll, 15 s later
            clock.after(15_000, () =>
              bee.end({ ...OK, completion: false, errorFrame: EF_CANCEL }),
            )
          },
        },
      }
    },
  })
  return { clock, sys, d, bees, events }
}

describe('the bee dispatcher as keyed actors', () => {
  it('a ready issue starts one bee, and a repeated ready starts none', async () => {
    const { clock, d, bees } = harness(4)
    for (let i = 0; i < 5; i++) d.ready(101)
    await clock.runUntil(1000)
    d.ready(101)
    await clock.runUntil(2000)
    expect(bees.map((b) => b.issue)).toEqual([101])
    expect(d.lanesInUse()).toBe(1)
  })

  it('a full set of lanes makes an issue wait, and a freed lane starts it at once', async () => {
    const { clock, d, bees } = harness(1)
    d.ready(1)
    await clock.runUntil(10)
    d.ready(2)
    await clock.runUntil(60_000)
    expect(bees.map((b) => b.issue)).toEqual([1])
    expect(d.waiting()).toEqual([2])
    bees[0].end(OK)
    await clock.runUntil(60_001)
    expect(bees.map((b) => [b.issue, b.at])).toEqual([
      [1, 0],
      [2, 60_000],
    ])
  })

  it('a bee that ends normally is done: its lane goes back and a later ready starts nothing', async () => {
    const { clock, d, bees, events } = harness(2)
    d.ready(7)
    await clock.runUntil(10)
    bees[0].end(OK)
    await clock.runUntil(20)
    d.ready(7)
    await clock.runUntil(30)
    expect(bees.length).toBe(1)
    expect(d.lanesInUse()).toBe(0)
    expect(events.filter((e) => e.kind === 'ended')).toEqual([
      { kind: 'ended', issue: 7, reason: DX_NORMAL, decision: RD_NONE },
    ])
  })

  it('a hung bee is killed at the turn bound, cancelled, and restarted on its own lane only once it has stopped', async () => {
    const { clock, d, bees, events } = harness(1)
    d.ready(9)
    d.ready(10) // waits for the only lane
    await clock.runUntil(10)
    expect(bees.map((b) => b.issue)).toEqual([9])
    // the bee never ends; the turn bound kills the wait at TURN_MAX_SECONDS
    await clock.runUntil(TURN_MAX_SECONDS * 1000 + 1)
    expect(bees[0].cancelled).toBe(1)
    expect(bees.length).toBe(1) // not before the bee has stopped
    await clock.runUntil(TURN_MAX_SECONDS * 1000 + 16_000)
    // it stopped 15 s later; the restart took its own kept lane, ahead of #10
    expect(bees.map((b) => b.issue)).toEqual([9, 9])
    expect(bees[1].at).toBeGreaterThanOrEqual(bees[0].ended as number)
    expect(d.waiting()).toEqual([10])
    expect(events.filter((e) => e.kind === 'ended')).toEqual([
      { kind: 'ended', issue: 9, reason: DX_KILLED, decision: RD_NOW },
    ])
  })

  it('a lost claim gives the lane back at once and tries again after the holder lease', async () => {
    let lands = false
    const { clock, d, bees, events } = harness(1, () => lands)
    d.ready(5)
    await clock.runUntil(10)
    expect(bees.length).toBe(0)
    expect(d.lanesInUse()).toBe(0)
    expect(events).toEqual([{ kind: 'stood-down', issue: 5 }])
    lands = true
    // the holder renewed 60 s ago: 180 - 60 = 120 s later, without a poll
    await clock.runUntil(119_000)
    expect(bees.length).toBe(0)
    await clock.runUntil(121_000)
    expect(bees.map((b) => b.issue)).toEqual([5])
  })

  it('an idle issue actor passivates, and a ready brings it back', async () => {
    const { clock, d } = harness(1)
    d.ready(3)
    d.ready(4) // refused a lane: idle, holding nothing
    await clock.runUntil(10)
    const first = d.issues.pidOf(4)
    expect(first).toBeDefined()
    await clock.runUntil((2 * PASSIVATE_IDLE_SECONDS + 1) * 1000)
    expect(d.issues.pidOf(4)).toBeUndefined()
    // #3 runs a bee: never passivated
    expect(d.issues.pidOf(3)).toBeDefined()
    d.ready(4)
    await clock.runUntil((2 * PASSIVATE_IDLE_SECONDS + 2) * 1000)
    expect(d.issues.pidOf(4)).not.toBe(first)
    expect(d.issues.stats.reactivations).toBeGreaterThanOrEqual(1)
  })
})
