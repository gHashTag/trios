/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A REVIEW WITH NO LANE WAITS FOR ONE (trios#1729 item 4; gHashTag/t27
 * specs/queen/review_lanes.t27). The `review-lanes` actor owns the queue: a
 * review that finds no lane free at its model call waits there, oldest
 * first, one waiter for each review in a model call, never past the row's
 * budget for its call. Run under a virtual clock.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import type { Pool } from 'pg'
import {
  createActorSystem,
  type Pid,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
  X_KILL,
} from '../../src/api/services/queen-actors-card.gen'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import type { WorkerProvider } from '../../src/api/services/queen-dispatch'
import {
  REVIEWER_SIZING_CARD,
  reviewerTree,
} from '../../src/api/services/queen-review-actors'
import {
  holdPercent,
  type LaneGate,
  poolTargetQueued,
  queueLanes,
  REVIEW_LANES_CARD,
  reviewLanes,
  reviewLanesOn,
} from '../../src/api/services/queen-review-lanes'
import {
  HOLD_PERCENT_UNTIL_MEASURED,
  LANE_GO,
  LANE_STOPPED,
  LANE_WAIT_ROW,
  MODEL_CALL_SECONDS,
} from '../../src/api/services/queen-review-lanes-card.gen'
import type { Judged } from '../../src/api/services/queen-review-loop'
import {
  forgetReviewerLaneFailures,
  type ReviewDeps,
} from '../../src/api/services/queen-reviewer'
import {
  REVIEW_ROW_SECONDS,
  REVIEWER_EVERY_SECONDS,
} from '../../src/api/services/queen-reviewer-card.gen'
import { reviewFinishedDispatches } from '../../src/api/services/queen-tick'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')
const SECOND = 1000
const judged = (): Judged => ({ acted: [], strays: [], tally: [] })

describe('the vendored lane queue card is the one PIN names', () => {
  it('review_lanes.t27 and review_lanes.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/review_lanes.t27 sha256 ${sha('queen/review_lanes.t27')}`,
    )
    expect(pin).toContain(
      `queen/review_lanes.wasm sha256 ${sha('queen/review_lanes.wasm')}`,
    )
  })

  it('compiles in the same pool_target the sizing card has', () => {
    const lanes = loadCardWasm(REVIEW_LANES_CARD)
    const sizing = loadCardWasm(REVIEWER_SIZING_CARD)
    for (const l of [0, 1, 3, 47])
      for (const [free, per] of [
        [1536, 0],
        [8192, 512],
      ])
        for (const [busy, queued] of [
          [0, 0],
          [0, 5],
          [4, 111],
        ]) {
          const args = [l, free, per, busy, queued]
          expect(lanes.call('pool_target', ...args)).toBe(
            sizing.call('pool_target', ...args),
          )
          // and an unmeasured share sizes exactly as the adaptive pool does
          expect(
            poolTargetQueued(
              l,
              HOLD_PERCENT_UNTIL_MEASURED,
              free,
              per,
              busy,
              queued,
            ),
          ).toBe(sizing.call('pool_target', ...args))
        }
  })

  it('is off unless TRIOS_QUEEN_REVIEW_LANES=semaphore', () => {
    expect(reviewLanesOn(undefined)).toBe(false)
    expect(reviewLanesOn('')).toBe(false)
    expect(reviewLanesOn('on')).toBe(false)
    expect(reviewLanesOn('semaphore')).toBe(true)
    expect(reviewLanesOn('SEMAPHORE')).toBe(true)
  })
})

/** A lane queue with its owner started, and live pids for the reviews. */
function queue(turnStop = false) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock, { turnStop })
  const lanes = reviewLanes(sys)
  const owner = lanes.child.start(() => {}).pid as Pid
  const worker = () => sys.spawn({ name: 'worker', receive: () => {} })
  /** The answer once it comes; undefined while the review waits. */
  const watch = (g: LaneGate) => {
    const seen: { answer?: number } = {}
    void g.wait().then((a) => {
      seen.answer = a
    })
    return seen
  }
  return { clock, sys, lanes, owner, worker, watch }
}

describe('the queue', () => {
  it('nobody in a model call: no wait, the row is given back at once', async () => {
    const q = queue()
    const g = q.watch(q.lanes.gate(q.worker()))
    await q.clock.runUntil(0)
    expect(g.answer).toBe(LANE_WAIT_ROW)
    expect(q.lanes.stats).toMatchObject({ asked: 1, refused: 1, queued: 0 })
  })

  it('a review waits behind one in a model call and goes on when it gives back its lane', async () => {
    const q = queue()
    const holder = q.lanes.gate(q.worker())
    holder.held()
    const g = q.watch(q.lanes.gate(q.worker()))
    await q.clock.runUntil(3 * SECOND)
    expect(g.answer).toBeUndefined()
    expect(q.lanes.stats.queued).toBe(1)
    holder.released()
    await q.clock.runUntil(4 * SECOND)
    expect(g.answer).toBe(LANE_GO)
    expect(q.lanes.stats.granted).toBe(1)
  })

  it('one waiter for each review in a model call: past that, a wait is refused at once', async () => {
    const q = queue()
    q.lanes.gate(q.worker()).held()
    const first = q.watch(q.lanes.gate(q.worker()))
    const second = q.watch(q.lanes.gate(q.worker()))
    await q.clock.runUntil(SECOND)
    expect(first.answer).toBeUndefined()
    expect(second.answer).toBe(LANE_WAIT_ROW)
    expect(q.lanes.stats).toMatchObject({ queued: 1, refused: 1 })
  })

  it('oldest first: the review that started first goes first, whoever asked first', async () => {
    const q = queue()
    const h1 = q.lanes.gate(q.worker())
    const h2 = q.lanes.gate(q.worker())
    h1.held()
    h2.held()
    const older = q.lanes.gate(q.worker())
    const younger = q.lanes.gate(q.worker())
    const y = q.watch(younger)
    await q.clock.runUntil(SECOND)
    const o = q.watch(older)
    await q.clock.runUntil(2 * SECOND)
    expect(q.lanes.stats.queued).toBe(2)
    h1.released()
    await q.clock.runUntil(3 * SECOND)
    expect(o.answer).toBe(LANE_GO)
    expect(y.answer).toBeUndefined()
    h2.released()
    await q.clock.runUntil(4 * SECOND)
    expect(y.answer).toBe(LANE_GO)
  })

  it('the heartbeat lets the oldest try again: a bee frees its key without telling the queue', async () => {
    const q = queue()
    q.lanes.gate(q.worker()).held()
    const g = q.watch(q.lanes.gate(q.worker()))
    await q.clock.runUntil(REVIEWER_EVERY_SECONDS * SECOND - 1)
    expect(g.answer).toBeUndefined()
    await q.clock.runUntil(REVIEWER_EVERY_SECONDS * SECOND)
    expect(g.answer).toBe(LANE_GO)
  })

  it('a wait ends when the model call would no longer fit the row, and keeps its place meanwhile', async () => {
    const q = queue()
    q.lanes.gate(q.worker()).held()
    const g = q.lanes.gate(q.worker())
    let tries = 0
    let answer = -1
    let endedAt = -1
    void (async () => {
      // what the review step does: let through, it finds the lane taken, and asks again
      for (;;) {
        answer = await g.wait()
        if (answer !== LANE_GO) break
        tries++
      }
      endedAt = q.clock.now()
    })()
    await q.clock.runUntil(REVIEW_ROW_SECONDS * SECOND)
    expect(answer).toBe(LANE_WAIT_ROW)
    // started at 0: it may wait until 300 - 120 = 180 s, and not past it
    expect(endedAt).toBe((REVIEW_ROW_SECONDS - MODEL_CALL_SECONDS) * SECOND)
    expect(tries).toBeGreaterThan(10)
    // a review whose call no longer fits does not wait at all: started at
    // 300 s, it asks at 480 s, and wait_left_seconds(180) is 0
    const late = q.lanes.gate(q.worker())
    await q.clock.runUntil(
      (2 * REVIEW_ROW_SECONDS - MODEL_CALL_SECONDS) * SECOND,
    )
    const asked = q.lanes.stats.asked
    expect(await late.wait()).toBe(LANE_WAIT_ROW)
    expect(q.lanes.stats.asked).toBe(asked)
  })

  it('a stopped turn leaves the queue and is let through nowhere', async () => {
    const q = queue(true)
    const holder = q.lanes.gate(q.worker())
    holder.held()
    const stop = new AbortController()
    const g = q.watch(q.lanes.gate(q.worker(), stop.signal))
    await q.clock.runUntil(SECOND)
    expect(q.lanes.stats.queued).toBe(1)
    stop.abort()
    await q.clock.runUntil(2 * SECOND)
    expect(g.answer).toBe(LANE_STOPPED)
    expect(q.lanes.stats.left).toBe(1)
    // the lane it waited for goes to nobody: the queue is empty
    holder.released()
    await q.clock.runUntil(3 * SECOND)
    expect(q.lanes.stats.granted).toBe(0)
    // a gate already stopped does not ask at all
    const asked = q.lanes.stats.asked
    expect(await q.lanes.gate(q.worker(), stop.signal).wait()).toBe(
      LANE_STOPPED,
    )
    expect(q.lanes.stats.asked).toBe(asked)
  })

  it('a review whose worker is dead cannot call, so it gives the row back without asking', async () => {
    const q = queue()
    q.lanes.gate(q.worker()).held()
    const w = q.worker()
    const g = q.lanes.gate(w)
    q.sys.exit(w, X_KILL)
    const asked = q.lanes.stats.asked
    expect(await g.wait()).toBe(LANE_WAIT_ROW)
    expect(q.lanes.stats.asked).toBe(asked)
  })

  it('an owner that goes down gives its waiters back their rows', async () => {
    const q = queue()
    q.lanes.gate(q.worker()).held()
    const g = q.watch(q.lanes.gate(q.worker()))
    await q.clock.runUntil(SECOND)
    q.sys.exit(q.owner, X_KILL)
    await q.clock.runUntil(2 * SECOND)
    expect(g.answer).toBe(LANE_WAIT_ROW)
  })

  it('the hold share is measured from what the owner sees, queue time left out', async () => {
    const q = queue()
    expect(q.lanes.holdPercent()).toBe(HOLD_PERCENT_UNTIL_MEASURED)
    // reviews that ask for their lane 30 s in and hold it 70 s
    let at = 0
    const review = async (waitFirst: boolean) => {
      const g = q.lanes.gate(q.worker())
      at += 30 * SECOND
      await q.clock.runUntil(at)
      if (waitFirst) {
        // waits 50 s for a lane a holder gives back: not in either sum
        const h = q.lanes.gate(q.worker())
        h.held()
        const seen = q.watch(g)
        at += 50 * SECOND
        await q.clock.runUntil(at)
        h.released()
        await q.clock.runUntil(at)
        expect(seen.answer).toBe(LANE_GO)
      }
      g.held()
      at += 70 * SECOND
      await q.clock.runUntil(at)
      g.released()
      await q.clock.runUntil(at)
    }
    // 8 reviews: 800 s, under HOLD_MEASURED_AFTER_SECONDS (900)
    for (let i = 0; i < 8; i++) await review(false)
    expect(q.lanes.holdPercent()).toBe(HOLD_PERCENT_UNTIL_MEASURED)
    await review(true)
    // the waiting review's helper held 50 s from its own start: (300 + 0) before,
    // 700 + 50 holding; the 50 s queue time of the ninth review is in neither
    expect(q.lanes.holdPercent()).toBe(holdPercent(270, 630 + 50))
    expect(q.lanes.holdPercent()).toBe(71)
  })
})

/**
 * A reviewer tree over `lanes` lanes. Each review spends `preS` on no lane,
 * then takes a lane for `callS` - or, with the queue, waits for one.
 */
function laneTree(o: {
  lanes: number
  workers?: number
  laneQueue: boolean
  preS: number
  callS: number
  turnStop?: boolean
  adaptive?: boolean
  /** Rows whose model call never ends unless its turn is stopped. */
  stall?: Set<number>
}) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock, { turnStop: o.turnStop ?? false })
  const rows = new Set<number>()
  const laneFree = Array.from({ length: o.lanes }, (_, i) => i)
  const done: number[] = []
  let laneless = 0
  let calls = 0
  let mostCalls = 0
  const freedAt: number[] = []
  const r = reviewerTree(sys, {
    holdsLease: async () => true,
    waiting: async () => [...rows],
    workers: o.workers,
    laneQueue: o.laneQueue,
    reviewOne: (issue, _reserved, onLane, signal, gate) =>
      new Promise<Judged>((resolve, reject) => {
        clock.after(o.preS * SECOND, async () => {
          let lane = laneFree.shift()
          while (lane === undefined && gate) {
            if ((await gate.wait()) !== LANE_GO) break
            lane = laneFree.shift()
          }
          if (lane === undefined) {
            laneless++
            resolve({ acted: [`#${issue}:wait`], strays: [], tally: [] })
            return
          }
          const held = lane
          gate?.held()
          onLane(held)
          calls++
          mostCalls = Math.max(mostCalls, calls)
          const free = () => {
            laneFree.push(held)
            calls--
            freedAt.push(clock.now())
            gate?.released()
          }
          signal?.addEventListener('abort', () => {
            free()
            reject(new Error('stopped'))
          })
          if (o.stall?.has(issue)) return
          clock.after(o.callS * SECOND, () => {
            if (signal?.aborted) return
            free()
            rows.delete(issue)
            done.push(issue)
            resolve(judged())
          })
        })
      }),
    ...(o.adaptive
      ? {
          capacity: async (reserved: number[]) => ({
            freeLanes: o.lanes - reserved.length,
            freeMb: 8192,
            mbPerReview: 512,
          }),
        }
      : {}),
  })
  supervisor(
    sys,
    {
      name: 'root',
      strategy: STRAT_ONE_FOR_ONE,
      maxRestarts: ROOT_MAX_RESTARTS,
      periodSeconds: ROOT_PERIOD_SECONDS,
    },
    [r.tree],
  ).start(() => {})
  return {
    clock,
    r,
    add: (...issues: number[]) => {
      for (const n of issues) rows.add(n)
      r.wake()
    },
    done,
    freedAt,
    laneless: () => laneless,
    mostCalls: () => mostCalls,
  }
}

describe('the reviewer with the lane queue', () => {
  it('the fixed workers: a review with no lane waits instead of answering wait', async () => {
    for (const laneQueue of [false, true]) {
      const t = laneTree({
        lanes: 1,
        workers: 2,
        laneQueue,
        preS: 30,
        callS: 60,
      })
      t.add(1, 2)
      await t.clock.runUntil(150 * SECOND)
      if (laneQueue) {
        // the second review waited 30 s for the lane and was done at 150 s
        expect(t.done.sort()).toEqual([1, 2])
        expect(t.laneless()).toBe(0)
        // one waiter, never refused; let try at each heartbeat while the lane
        // was still held, and for good when row 1 gave it back
        expect(t.r.lanes?.stats).toMatchObject({ refused: 0, mostWaiting: 1 })
        expect(t.r.lanes?.stats.granted).toBeGreaterThanOrEqual(1)
      } else {
        // the second review found no lane at 30 s and answered `wait`; it is
        // done only on a later visit, its first 30 s thrown away
        expect(t.done[0]).toBe(1)
        expect(t.laneless()).toBeGreaterThanOrEqual(1)
        expect(t.r.lanes).toBeUndefined()
      }
      expect(t.mostCalls()).toBe(1)
    }
  })

  it('with turn stop, a stopped review gives back its lane, and the waiter behind it goes on', async () => {
    const t = laneTree({
      lanes: 1,
      workers: 2,
      laneQueue: true,
      preS: 30,
      callS: 60,
      turnStop: true,
      stall: new Set([1]),
    })
    t.add(1)
    await t.clock.runUntil(150 * SECOND)
    // row 2 comes later, so its wait still fits its row when row 1's turn is stopped
    t.add(2)
    await t.clock.runUntil(REVIEW_ROW_SECONDS * SECOND + 1)
    // row 1's turn was stopped at REVIEW_ROW_SECONDS: its lane is free at once
    expect(t.freedAt[0]).toBe(REVIEW_ROW_SECONDS * SECOND)
    await t.clock.runUntil((REVIEW_ROW_SECONDS + 61) * SECOND)
    expect(t.done).toContain(2)
    expect(t.laneless()).toBe(0)
    expect(t.mostCalls()).toBe(1)
  })

  it('the adaptive pool exceeds its lanes by the measured share on no lane, and calls stay within the lanes', async () => {
    const t = laneTree({
      lanes: 1,
      laneQueue: true,
      adaptive: true,
      preS: 50,
      callS: 50,
    })
    t.add(...Array.from({ length: 40 }, (_, i) => 100 + i))
    // until measured, the pool is the adaptive one: as many workers as lanes
    await t.clock.runUntil(20 * SECOND)
    expect(t.r.size().target).toBe(1)
    await t.clock.runUntil(1500 * SECOND)
    // a review holds its lane half its life: 1 lane keeps 2 reviews going
    expect(t.r.lanes?.holdPercent()).toBe(50)
    expect(queueLanes(1, 50)).toBe(2)
    expect(t.r.size().target).toBe(2)
    expect(t.r.size().holdPercent).toBe(50)
    expect(t.mostCalls()).toBe(1)
    expect(t.laneless()).toBe(0)
  })
})

describe('the sweep waits at its model call (reviewFinishedDispatches)', () => {
  const ISSUE = 7102
  const LANE: WorkerProvider = {
    provider: 'zai',
    model: 'glm-test',
    baseUrl: 'https://z.example.invalid',
    apiKey: 'not-a-real-key',
    keyIndex: 1,
    poolNumber: 1,
    laneIndex: 0,
    laneCount: 1,
  }
  const saved: Record<string, string | undefined> = {}
  beforeEach(() => {
    forgetReviewerLaneFailures()
    saved.WORKSPACE_DIR = process.env.WORKSPACE_DIR
    process.env.WORKSPACE_DIR = join(
      tmpdir(),
      'queen-review-lanes-no-workspace',
    )
  })
  afterEach(() => {
    if (saved.WORKSPACE_DIR === undefined) delete process.env.WORKSPACE_DIR
    else process.env.WORKSPACE_DIR = saved.WORKSPACE_DIR
  })

  /** One finished row, a pool that reads it, and the sweep's fake steps. */
  function sweep(gate: LaneGate, freeAfterReads: number) {
    const row: Record<string, unknown> = {
      issue: ISSUE,
      conversation_id: '00000000-0000-0000-0000-000000001bbe',
      review_state: null,
      criteria: ['The handler scrolls', 'A test covers it'],
      criteria_source: 'stated',
      send_backs: 0,
      owned_paths: [],
      free_attempts: 0,
      key_index: 2,
      provider: 'zai',
      model: 'glm-test',
      said: '',
    }
    const queries: string[] = []
    const pool = {
      query: async (sql: string) => {
        queries.push(String(sql))
        if (String(sql).includes('FROM queen_dispatch d'))
          return { rowCount: 1, rows: [row] }
        return { rowCount: 0, rows: [] }
      },
    } as unknown as Pool
    const ac = new AbortController()
    const seen = { reads: 0, calls: 0, heldAtCall: false }
    let holding = false
    const watched: LaneGate = {
      wait: () => gate.wait(),
      held: () => {
        holding = true
        gate.held()
      },
      released: () => {
        holding = false
        gate.released()
      },
    }
    const deps: Partial<ReviewDeps> = {
      committedFilesResult: async () => ({ ok: true, files: ['src/a.ts'] }),
      branchHeadSha: async () => 'a'.repeat(40),
      mergeBaseSha: async () => 'b'.repeat(40),
      branchPatch: async () => 'diff --git a/src/a.ts b/src/a.ts\n+x',
      worktreeDirtCount: async () => null,
      witness: async () => ({ kind: 'witnessed', t27c: 'fake', specs: [] }),
      // no lane on the first reads, then one
      laneCandidates: () => (++seen.reads > freeAfterReads ? [LANE] : []),
      reviewsPerRound: () => 3,
      // the call is made, then the turn stops, so the sweep writes nothing
      llm: async () => {
        seen.calls++
        seen.heldAtCall = holding
        ac.abort(new DOMException('aborted: test', 'AbortError'))
        return { ok: false, error: 'stopped', transient: true }
      },
    }
    const run = reviewFinishedDispatches(pool, deps, {
      issues: [ISSUE],
      signal: ac.signal,
      lanes: watched,
    })
    return { run, seen, ac, queries, holding: () => holding }
  }

  it('no lane free: it waits, reads the keys again when let through, and holds the lane for the call only', async () => {
    const answers = [LANE_GO]
    const log: string[] = []
    const gate: LaneGate = {
      wait: async () => {
        log.push('wait')
        return answers.shift() ?? LANE_WAIT_ROW
      },
      held: () => log.push('held'),
      released: () => log.push('released'),
    }
    const s = sweep(gate, 1)
    await expect(s.run).rejects.toThrow('aborted')
    expect(log).toEqual(['wait', 'held', 'released'])
    expect(s.seen).toMatchObject({ reads: 2, calls: 1, heldAtCall: true })
    expect(s.holding()).toBe(false)
  })

  it('a stopped wait ends the sweep before any call and any write', async () => {
    const log: string[] = []
    let stop = () => {}
    const gate: LaneGate = {
      wait: async () => {
        log.push('wait')
        stop()
        return LANE_STOPPED
      },
      held: () => log.push('held'),
      released: () => log.push('released'),
    }
    const s = sweep(gate, 99)
    stop = () => s.ac.abort(new DOMException('aborted: test', 'AbortError'))
    await expect(s.run).rejects.toThrow('aborted')
    expect(log).toEqual(['wait'])
    expect(s.seen.calls).toBe(0)
    expect(s.queries.some((q) => q.includes('review_state = $2'))).toBe(false)
  })
})
