/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * HOW A SUPERVISOR ENDS (gHashTag/t27 specs/queen/actors.t27 section 5),
 * for the fixed supervisor and the dynamic one (queen-actors-dynamic.ts):
 * giving up past the intensity, a child that is not restarted, and a stop
 * asked for by the parent while a restart waits (gHashTag/trios#1731).
 * The card decides each exit; these tests read what the runtime then did.
 */

import { describe, expect, it } from 'bun:test'
import {
  type ActorSystem,
  actorChild,
  createActorSystem,
  type Pid,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  BACKOFF_BASE_SECONDS,
  GIVE_UP_REASON,
  RESTART_TEMPORARY,
  START_MAX_CHILDREN,
  START_OK,
  STRAT_ONE_FOR_ONE,
} from '../../src/api/services/queen-actors-card.gen'
import {
  type DynamicChildren,
  dynamicSupervisor,
} from '../../src/api/services/queen-actors-dynamic'
import {
  MT_GIVE_UPS,
  MT_RESTARTS,
} from '../../src/api/services/queen-telemetry-card.gen'
import { VirtualClock } from './queen-virtual-clock'

/** A child that crashes on 'crash' and records its live pid under `n`. */
function kids(sys: ActorSystem, pids: Map<number, Pid>, restart?: number) {
  return (n: number) =>
    actorChild<string>(
      sys,
      {
        name: `kid-${n}`,
        kind: 'kid',
        init: (self) => {
          pids.set(n, self)
        },
        receive: (m) => {
          if (m === 'crash') throw new Error(`kid-${n} crashed`)
        },
      },
      restart,
    )
}

interface SupSnap {
  supervisors: Record<string, Record<string, number | boolean>>
}

// a restart waits backoff_seconds(1) = BACKOFF_BASE_SECONDS, jittered down
const PAST_BACKOFF_MS = (BACKOFF_BASE_SECONDS + 1) * 1000

describe('a dynamic supervisor', () => {
  it('past its intensity gives up: every child stops, the parent hears GIVE_UP_REASON, telemetry counts the give-up', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    const pids = new Map<number, Pid>()
    let handle: DynamicChildren | undefined
    let parentHeard = -1
    dynamicSupervisor(
      sys,
      { name: 'dyn', maxRestarts: 1, periodSeconds: 300 },
      kids(sys, pids),
      (h) => {
        handle = h
      },
    ).start((reason) => {
      parentHeard = reason
    })
    const h = handle as DynamicChildren
    expect(h.startChild(4)).toBe(START_OK)
    expect(h.startChild(4)).toBe(START_OK)
    expect(h.live()).toBe(2)
    const first = pids.get(0) as Pid

    // the first crash is restarted after the backoff, on a new pid
    sys.send(first, 'crash')
    await clock.runUntil(PAST_BACKOFF_MS)
    const second = pids.get(0) as Pid
    expect(second).not.toBe(first)
    expect(sys.alive(second)).toBe(true)
    let s = tel.snapshot() as unknown as SupSnap
    expect(s.supervisors.dyn[MT_RESTARTS]).toBe(1)
    // the restart is inside the period, as the card counts it
    expect(s.supervisors.dyn.inPeriod).toBe(1)
    expect(parentHeard).toBe(-1)

    // the second crash inside the period is one more than maxRestarts allows
    sys.send(second, 'crash')
    await clock.runUntil(PAST_BACKOFF_MS * 2)
    expect(parentHeard).toBe(GIVE_UP_REASON)
    expect(sys.alive(pids.get(1) as Pid)).toBe(false)
    s = tel.snapshot() as unknown as SupSnap
    expect(s.supervisors.dyn[MT_GIVE_UPS]).toBe(1)
    // a supervisor that gave up starts nobody
    expect(h.startChild(4)).toBe(START_MAX_CHILDREN)
    tel.close()
  })

  it('a child that is not restarted frees its place: a temporary child ends and a start fits again', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const pids = new Map<number, Pid>()
    let handle: DynamicChildren | undefined
    let parentHeard = -1
    dynamicSupervisor(
      sys,
      { name: 'dyn-temp', maxRestarts: 3, periodSeconds: 300 },
      kids(sys, pids, RESTART_TEMPORARY),
      (h) => {
        handle = h
      },
    ).start((reason) => {
      parentHeard = reason
    })
    const h = handle as DynamicChildren
    expect(h.startChild(1)).toBe(START_OK)
    expect(h.startChild(1)).toBe(START_MAX_CHILDREN)
    sys.send(pids.get(0) as Pid, 'crash')
    await clock.runUntil(PAST_BACKOFF_MS)
    // temporary: never restarted, and its place is free (children_after_exit)
    expect(sys.alive(pids.get(0) as Pid)).toBe(false)
    expect(h.live()).toBe(0)
    expect(h.startChild(1)).toBe(START_OK)
    expect(pids.get(1)).toBeDefined()
    expect(parentHeard).toBe(-1)
  })

  it('stopped by its parent while a restart waits: its children stop and the restart never comes', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const pids = new Map<number, Pid>()
    let made = 0
    let handle: DynamicChildren | undefined
    const make = kids(sys, pids)
    const running = dynamicSupervisor(
      sys,
      { name: 'dyn-stop', maxRestarts: 3, periodSeconds: 300 },
      (n) => {
        made++
        return make(n)
      },
      (h) => {
        handle = h
      },
    ).start(() => {})
    const h = handle as DynamicChildren
    h.startChild(4)
    h.startChild(4)
    const crashed = pids.get(0) as Pid
    sys.send(crashed, 'crash')
    await clock.runUntil(1000)
    // the restart of kid-0 now waits for its backoff
    expect(sys.alive(crashed)).toBe(false)
    running.stop()
    expect(sys.alive(pids.get(1) as Pid)).toBe(false)
    await clock.runUntil(PAST_BACKOFF_MS * 3)
    // the pending restart was cancelled: kid-0 has the pid that crashed
    expect(pids.get(0)).toBe(crashed)
    expect(made).toBe(2)
    expect(h.startChild(4)).toBe(START_MAX_CHILDREN)
  })
})

describe('a fixed supervisor', () => {
  it('stopped by its parent: its children stop, the last started first, and a waiting restart never comes', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const pids = new Map<number, Pid>()
    const make = kids(sys, pids)
    const stopped: number[] = []
    sys.onExit((pid) => {
      for (const [n, p] of pids) if (p === pid) stopped.push(n)
    })
    const running = supervisor(
      sys,
      {
        name: 'fixed',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [make(0), make(1), make(2)],
    ).start(() => {})
    const crashed = pids.get(1) as Pid
    sys.send(crashed, 'crash')
    await clock.runUntil(1000)
    stopped.length = 0
    running.stop()
    // kid-1 is already down and waits for its restart: 2, then 0
    expect(stopped).toEqual([2, 0])
    await clock.runUntil(PAST_BACKOFF_MS * 3)
    expect(pids.get(1)).toBe(crashed)
    expect(sys.alive(crashed)).toBe(false)
  })
})
