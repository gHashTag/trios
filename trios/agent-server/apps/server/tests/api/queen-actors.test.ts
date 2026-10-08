/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTOR RUNTIME, MVP (gHashTag/t27 specs/queen/actors.t27, t27#7851).
 * This test runs the card as the wasm runs it, under a virtual clock.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import {
  actorChild,
  createActorSystem,
  type Down,
  type Pid,
  slotOf,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  D_DROPPED_DEAD,
  D_DROPPED_FULL,
  D_QUEUED,
  DOMAIN_MAX_RESTARTS,
  DOMAIN_PERIOD_SECONDS,
  GIVE_UP_REASON,
  M_CANCEL,
  MAILBOX_CAP,
  RESTART_TEMPORARY,
  STRAT_ONE_FOR_ONE,
  STRAT_REST_FOR_ONE,
  X_CRASH,
  X_KILLED,
} from '../../src/api/services/queen-actors-card.gen'
import { reviewerTree } from '../../src/api/services/queen-review-actors'
import { REVIEW_ROW_SECONDS } from '../../src/api/services/queen-reviewer-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

describe('the vendored actors card is the one PIN names', () => {
  it('actors.t27 and actors.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(`queen/actors.t27 sha256 ${sha('queen/actors.t27')}`)
    expect(pin).toContain(
      `queen/actors.wasm sha256 ${sha('queen/actors.wasm')}`,
    )
  })
})

describe('a process', () => {
  it('takes its messages in order, one turn at a time', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const seen: number[] = []
    let inTurn = 0
    let most = 0
    const pid = sys.spawn<number>({
      name: 'p',
      receive: async (n) => {
        inTurn++
        most = Math.max(most, inTurn)
        await new Promise<void>((r) => clock.after(1000, r))
        seen.push(n)
        inTurn--
      },
    })
    for (const n of [1, 2, 3]) expect(sys.send(pid, n)).toBe(D_QUEUED)
    await clock.runUntil(10_000)
    expect(seen).toEqual([1, 2, 3])
    expect(most).toBe(1)
  })

  it('refuses a send past MAILBOX_CAP, and a send to a dead pid is a dead letter', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const pid = sys.spawn<number>({
      name: 'slow',
      receive: () => new Promise<void>((r) => clock.after(60_000, r)),
    })
    // the first message starts a turn at once and leaves the box
    sys.send(pid, 0)
    await clock.runUntil(0)
    for (let i = 0; i < MAILBOX_CAP; i++)
      expect(sys.send(pid, i)).toBe(D_QUEUED)
    expect(sys.send(pid, -1)).toBe(D_DROPPED_FULL)
    sys.exit(pid, X_CRASH, true)
    expect(sys.send(pid, 1)).toBe(D_DROPPED_DEAD)
    expect(sys.stats.deadLetters).toBeGreaterThanOrEqual(MAILBOX_CAP + 2)
  })

  it('takes a control message before any queued data message', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const order: string[] = []
    const pid = sys.spawn<string>({
      name: 'p',
      receive: async (m) => {
        order.push(m)
        await new Promise<void>((r) => clock.after(1000, r))
      },
      control: (tag) => order.push(`ctl:${tag}`),
    })
    sys.send(pid, 'a')
    sys.send(pid, 'b')
    sys.send(pid, 'c')
    await clock.runUntil(0)
    sys.post(pid, M_CANCEL)
    await clock.runUntil(10_000)
    expect(order).toEqual(['a', `ctl:${M_CANCEL}`, 'b', 'c'])
  })

  it('a turn past its bound is killed, and what the abandoned turn sends reaches no one', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const got: string[] = []
    const sink = sys.spawn<string>({
      name: 'sink',
      receive: (m) => void got.push(m),
    })
    let reason = -1
    const pid = sys.spawn<string>(
      {
        name: 'hangs',
        turnMaxSeconds: 30,
        receive: async (_m, self) => {
          await new Promise<void>((r) => clock.after(3_600_000, r))
          sys.send(sink, 'late', self)
        },
      },
      (r) => {
        reason = r
      },
    )
    sys.send(pid, 'go')
    await clock.runUntil(29_000)
    expect(sys.alive(pid)).toBe(true)
    await clock.runUntil(30_000)
    expect(sys.alive(pid)).toBe(false)
    expect(reason).toBe(X_KILLED)
    expect(sys.stats.killed).toBe(1)
    await clock.runUntil(4_000_000)
    expect(got).toEqual([])
  })

  it('a monitor hears DOWN when the watched process dies', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const downs: Down[] = []
    const watcher = sys.spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    const target = sys.spawn<number>({
      name: 't',
      receive: () => {
        throw new Error('boom')
      },
    })
    sys.monitor(watcher, target)
    sys.send(target, 1)
    await clock.runUntil(1000)
    expect(downs).toEqual([{ kind: 'DOWN', pid: target, reason: X_CRASH }])
  })
})

describe('a supervisor', () => {
  it('restarts a crashed child in its own slot with a new pid, after the card backoff', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const pids: Pid[] = []
    const child = actorChild<number>(sys, {
      name: 'c',
      init: (self) => void pids.push(self),
      receive: () => {
        throw new Error('boom')
      },
    })
    supervisor(
      sys,
      {
        name: 's',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [child],
    ).start(() => {})
    sys.send(pids[0], 1)
    await clock.runUntil(7_000)
    // a first crash of a short run waits backoff_seconds(1) = 10 s, pulled down by at most 20%
    expect(pids.length).toBe(1)
    await clock.runUntil(10_000)
    expect(pids.length).toBe(2)
    expect(slotOf(pids[1])).toBe(slotOf(pids[0]))
    expect(pids[1]).not.toBe(pids[0])
    expect(sys.send(pids[0], 1)).toBe(D_DROPPED_DEAD)
  })

  it('gives up past its intensity and reports GIVE_UP_REASON to its parent', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const pids: Pid[] = []
    let parentHeard = -1
    supervisor(
      sys,
      {
        name: 's',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: DOMAIN_MAX_RESTARTS,
        periodSeconds: DOMAIN_PERIOD_SECONDS,
      },
      [
        actorChild<number>(sys, {
          name: 'c',
          init: (self) => void pids.push(self),
          receive: () => {
            throw new Error('boom')
          },
        }),
      ],
    ).start((r) => {
      parentHeard = r
    })
    // one crash a minute: the backoff before restart n is 10, 20, 40 s, so each comes back inside
    // its minute, and the fourth crash finds three restarts younger than the 300 s period
    for (let k = 0; k <= DOMAIN_MAX_RESTARTS; k++) {
      sys.send(pids[pids.length - 1], 1)
      await clock.runUntil(clock.now() + 60_000)
    }
    expect(parentHeard).toBe(GIVE_UP_REASON)
    expect(pids.length).toBe(DOMAIN_MAX_RESTARTS + 1)
  })

  it('rest_for_one restarts the failed child and every child after it, not before', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const starts = [0, 0, 0]
    const pids: Pid[] = []
    const kid = (i: number) =>
      actorChild<number>(sys, {
        name: `k${i}`,
        init: (self) => {
          starts[i]++
          pids[i] = self
        },
        receive: () => {
          throw new Error('boom')
        },
      })
    supervisor(
      sys,
      {
        name: 's',
        strategy: STRAT_REST_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [kid(0), kid(1), kid(2)],
    ).start(() => {})
    sys.send(pids[1], 1)
    await clock.runUntil(60_000)
    expect(starts).toEqual([1, 2, 2])
  })

  it('a temporary child is not restarted', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    let starts = 0
    let pid: Pid = 0n
    supervisor(
      sys,
      {
        name: 's',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [
        actorChild<number>(
          sys,
          {
            name: 't',
            init: (self) => {
              starts++
              pid = self
            },
            receive: () => {
              throw new Error('boom')
            },
          },
          RESTART_TEMPORARY,
        ),
      ],
    ).start(() => {})
    sys.send(pid, 1)
    await clock.runUntil(600_000)
    expect(starts).toBe(1)
  })
})

describe('the reviewer as actors', () => {
  const judged = () => ({ acted: [], strays: [], tally: [] })

  it('reviews every waiting row once, never two at once, and at most REVIEWER_CONCURRENCY together', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const waiting = new Set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    const reviewed: number[] = []
    let inFlight = 0
    let most = 0
    const active = new Set<number>()
    const r = reviewerTree(sys, {
      holdsLease: async () => true,
      waiting: async () => [...waiting],
      reviewOne: async (issue) => {
        expect(active.has(issue)).toBe(false)
        active.add(issue)
        inFlight++
        most = Math.max(most, inFlight)
        await new Promise<void>((res) => clock.after(60_000, res))
        inFlight--
        active.delete(issue)
        waiting.delete(issue)
        reviewed.push(issue)
        return judged()
      },
    })
    r.tree.start(() => {})
    r.wake()
    await clock.runUntil(3_600_000)
    expect(reviewed.sort((a, b) => a - b)).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
    ])
    expect(most).toBe(4)
  })

  it('a review past REVIEW_ROW_SECONDS frees its worker, but its row and its lane wait for the review to end', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    const waiting = new Set([7, 8])
    const attempts = new Map<number, number>()
    const running = new Set<number>()
    let overlap = 0
    const reservedSeen: number[][] = []
    const r = reviewerTree(sys, {
      holdsLease: async () => true,
      waiting: async () => [...waiting],
      workers: 1,
      reviewOne: async (issue, reservedKeys, onLane) => {
        if (running.has(issue)) overlap++
        running.add(issue)
        const n = (attempts.get(issue) ?? 0) + 1
        attempts.set(issue, n)
        reservedSeen.push(reservedKeys())
        const hang = issue === 7 && n === 1
        if (hang) onLane(3)
        await new Promise<void>((res) =>
          clock.after(hang ? 2 * 3_600_000 : 30_000, res),
        )
        running.delete(issue)
        if (!hang) waiting.delete(issue)
        return judged()
      },
    })
    r.tree.start(() => {})
    r.wake()
    // the only worker is killed at the bound and comes back; it reviews row 8 while 7 still hangs
    await clock.runUntil((REVIEW_ROW_SECONDS + 120) * 1000)
    expect(sys.stats.killed).toBe(1)
    expect(waiting.has(8)).toBe(false)
    expect(attempts.get(7)).toBe(1)
    // row 8's review saw lane 3 reserved: the hung review still holds it
    expect(reservedSeen[1]).toEqual([3])
    // when the hung review ends, row 7 is reviewed again, never two at once
    await clock.runUntil(3 * 3_600_000)
    expect(attempts.get(7)).toBe(2)
    expect(waiting.size).toBe(0)
    expect(overlap).toBe(0)
  })

  it('does nothing without the lease', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    let calls = 0
    const r = reviewerTree(sys, {
      holdsLease: async () => false,
      waiting: async () => [1, 2],
      reviewOne: async () => {
        calls++
        return judged()
      },
    })
    r.tree.start(() => {})
    r.wake()
    await clock.runUntil(600_000)
    expect(calls).toBe(0)
  })
})
