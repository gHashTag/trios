/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The bounded drain (gHashTag/t27 specs/queen/drain.t27, trios#1729 item 2):
 * the vendored card is the one PIN names, the wasm answers as the spec's
 * tests say, and the loop in queen-drain.ts waits only for what the card
 * says, on a virtual clock.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { Pool } from 'pg'
import { DISPATCH_OUTCOME_LABELS } from '../../src/api/services/queen-dispatch'
import {
  beeFreeAfterSeconds,
  type DrainBee,
  type DrainDeps,
  drainBounded,
  drainBoundedEnabled,
  drainCapSeconds,
  drainVerdict,
  exitStep,
  platformDrainSeconds,
  takesNewWork,
} from '../../src/api/services/queen-drain'
import {
  DV_DONE,
  DV_HAND_BACK,
  DV_HOLD,
  DV_LEAVE,
  DV_STOP,
  EXIT_NOW,
  EXIT_WAIT,
  HAND_BACK_SECONDS,
  LANE_FREE_MS,
  WK_BEE,
  WK_JOB,
  WK_REVIEW,
  WK_ROUND,
  WK_WAIT,
} from '../../src/api/services/queen-drain-card.gen'
import { type BeeOrder, handBackBee } from '../../src/api/services/queen-runner'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

describe('the vendored drain card is the one PIN names', () => {
  it('drain.t27 and drain.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(`queen/drain.t27 sha256 ${sha('queen/drain.t27')}`)
    expect(pin).toContain(`queen/drain.wasm sha256 ${sha('queen/drain.wasm')}`)
  })
})

describe('the card, as the wasm answers', () => {
  it('drain_verdict: rows are left, a review is stopped, a bee is held to the cap', () => {
    for (const kind of [WK_WAIT, WK_JOB, WK_ROUND]) {
      expect(drainVerdict(kind, false, 0, 1800)).toBe(DV_LEAVE)
      expect(drainVerdict(kind, false, 9999, 1800)).toBe(DV_LEAVE)
    }
    expect(drainVerdict(WK_REVIEW, false, 0, 1800)).toBe(DV_STOP)
    expect(drainVerdict(WK_BEE, false, 0, 1800)).toBe(DV_HOLD)
    expect(drainVerdict(WK_BEE, false, 1799, 1800)).toBe(DV_HOLD)
    expect(drainVerdict(WK_BEE, false, 1800, 1800)).toBe(DV_HAND_BACK)
    expect(drainVerdict(WK_BEE, true, 1800, 1800)).toBe(DV_DONE)
  })

  it('drain_cap_seconds: today 1800/1830 stays 1800; the platform bounds it', () => {
    expect(drainCapSeconds(1800, 1830)).toBe(1800)
    expect(drainCapSeconds(1800, 1800)).toBe(1775)
    expect(drainCapSeconds(1800, 0)).toBe(1800)
    expect(drainCapSeconds(600, 1830)).toBe(600)
    expect(drainCapSeconds(1800, 20)).toBe(0)
  })

  it('takes_new_work, exit_step, bee_free_after_seconds', () => {
    expect(takesNewWork(true, 5)).toBe(false)
    expect(takesNewWork(false, 1)).toBe(true)
    expect(takesNewWork(false, 0)).toBe(false)
    expect(exitStep(1, 0, 0)).toBe(EXIT_WAIT)
    expect(exitStep(0, 0, 0)).toBe(EXIT_NOW)
    expect(exitStep(0, 1, LANE_FREE_MS - 1)).toBe(EXIT_WAIT)
    expect(exitStep(0, 1, LANE_FREE_MS)).toBe(EXIT_NOW)
    expect(beeFreeAfterSeconds(true, 10)).toBe(0)
    expect(beeFreeAfterSeconds(false, 10)).toBe(170)
  })
})

describe('the flag and the platform drain', () => {
  it('only TRIOS_QUEEN_DRAIN=bounded turns it on', () => {
    expect(drainBoundedEnabled({})).toBe(false)
    expect(drainBoundedEnabled({ TRIOS_QUEEN_DRAIN: 'runner' })).toBe(false)
    expect(drainBoundedEnabled({ TRIOS_QUEEN_DRAIN: ' Bounded ' })).toBe(true)
  })
  it('an unreadable platform drain is 0, which the card reads as unknown', () => {
    expect(platformDrainSeconds({})).toBe(0)
    expect(
      platformDrainSeconds({ RAILWAY_DEPLOYMENT_DRAINING_SECONDS: 'x' }),
    ).toBe(0)
    expect(
      platformDrainSeconds({ RAILWAY_DEPLOYMENT_DRAINING_SECONDS: ' 1830 ' }),
    ).toBe(1830)
  })
})

/** A process on a virtual clock: bees that end at given times, reviews that end when stopped. */
function world(o: {
  bees: Array<{ issue: number; endsAtMs: number }>
  reviews?: number
  reviewStopMs?: number
  capSeconds?: number
  roundsStartBeesHere?: boolean
  handBackMs?: number
}) {
  const clock = new VirtualClock()
  const running = new Map(o.bees.map((b) => [b.issue, b]))
  for (const b of o.bees)
    clock.after(b.endsAtMs, () => {
      running.delete(b.issue)
    })
  let reviews = o.reviews ?? 0
  const log: string[] = []
  const deps: DrainDeps = {
    now: clock.now,
    sleep: (ms) => new Promise<void>((resolve) => clock.after(ms, resolve)),
    capSeconds: o.capSeconds ?? 1800,
    roundsStartBeesHere: o.roundsStartBeesHere ?? false,
    stopClaims: () => log.push(`claims stopped at ${clock.now()}`),
    bees: () =>
      [...running.values()].map(
        (b): DrainBee => ({ issue: b.issue, conversationId: `c${b.issue}` }),
      ),
    handBack: (bee) =>
      new Promise<void>((resolve) =>
        clock.after(o.handBackMs ?? 1000, () => {
          log.push(`handed back ${bee.issue} at ${clock.now()}`)
          running.delete(bee.issue)
          resolve()
        }),
      ),
    reviews: () => reviews,
    stopRest: async () => {
      log.push(`rest stopped at ${clock.now()}`)
      if (reviews > 0)
        clock.after(o.reviewStopMs ?? 200, () => {
          reviews = 0
        })
    },
  }
  return { clock, deps, log }
}

async function run(w: ReturnType<typeof world>) {
  const done = drainBounded(w.deps)
  await w.clock.runUntil(4 * 3600_000)
  return done
}

describe('drainBounded on a virtual clock', () => {
  it('a quiet process exits at once, and takes no new bee', async () => {
    const w = world({ bees: [] })
    const r = await run(w)
    expect(r.tookMs).toBe(0)
    expect(r.handedBack).toEqual([])
    expect(w.log[0]).toBe('claims stopped at 0')
  })

  it('waits for the bees that end before the cap, and for no longer', async () => {
    const w = world({
      bees: [
        { issue: 1, endsAtMs: 300_000 },
        { issue: 2, endsAtMs: 1_000_000 },
      ],
    })
    const r = await run(w)
    expect(r.heldAtStart).toBe(2)
    expect(r.handedBack).toEqual([])
    // the last bee ends at 1000 s; the loop looks every 2 s
    expect(r.tookMs).toBeGreaterThanOrEqual(1_000_000)
    expect(r.tookMs).toBeLessThan(1_002_001)
    // rounds and reviews went on while the bees held, and stopped after
    expect(w.log).toContain(`rest stopped at ${r.tookMs}`)
  })

  it('hands back, side by side, every bee still running at the cap', async () => {
    const w = world({
      bees: [
        { issue: 1, endsAtMs: 300_000 },
        { issue: 2, endsAtMs: 2_400_000 },
        { issue: 3, endsAtMs: 3_000_000 },
      ],
      handBackMs: 5_000,
    })
    const r = await run(w)
    expect(r.handedBack.sort()).toEqual([2, 3])
    // both at the cap, together: 1800 s plus one hand-back, not two
    expect(w.log).toContain('handed back 2 at 1805000')
    expect(w.log).toContain('handed back 3 at 1805000')
    expect(r.tookMs).toBe(1_805_000)
  })

  it('a hand-back that hangs is left at its deadline', async () => {
    const w = world({
      bees: [{ issue: 7, endsAtMs: 9_000_000 }],
      handBackMs: 600_000,
    })
    const r = await run(w)
    expect(r.handedBack).toEqual([7])
    expect(r.tookMs).toBe(1_800_000 + HAND_BACK_SECONDS * 1000)
  })

  it('stops the reviews at the exit and waits for them to end, at most LANE_FREE_MS', async () => {
    const quick = world({ bees: [], reviews: 2, reviewStopMs: 300 })
    const a = await run(quick)
    expect(a.reviewsStopped).toBe(2)
    expect(a.tookMs).toBeGreaterThanOrEqual(300)
    expect(a.tookMs).toBeLessThan(500)
    const deaf = world({ bees: [], reviews: 1, reviewStopMs: 60_000 })
    const b = await run(deaf)
    expect(b.tookMs).toBe(LANE_FREE_MS)
  })

  it('stops the rounds at once when a round would start a bee here', async () => {
    const w = world({
      bees: [{ issue: 1, endsAtMs: 100_000 }],
      roundsStartBeesHere: true,
    })
    const r = await run(w)
    expect(w.log).toContain('rest stopped at 0')
    expect(r.tookMs).toBeGreaterThanOrEqual(100_000)
  })

  it('a cap of 0 hands every bee back at once', async () => {
    const w = world({
      bees: [{ issue: 4, endsAtMs: 600_000 }],
      capSeconds: 0,
    })
    const r = await run(w)
    expect(r.handedBack).toEqual([4])
    expect(r.tookMs).toBe(1000)
  })
})

describe('handBackBee', () => {
  const order: BeeOrder = {
    issue: 42,
    branch: 'queen/42',
    brief: '',
    ownedPaths: [],
    conversationId: 'conv-42',
    keyIndex: 1001,
  }
  it('salvages and stores first, ends the row as reaped, aborts last', async () => {
    const steps: string[] = []
    const ended = await handBackBee({} as Pool, 'runner-a', order, {
      salvage: async (_p, issue, reason, deps) => {
        steps.push(`salvage ${issue} ${reason} ${deps?.conversationId}`)
        return { committed: true, files: [], left: [], sha: null, detail: '' }
      },
      store: async (_p, issue, runner) => {
        steps.push(`store ${issue} ${runner}`)
      },
      finish: async (_p, issue, outcome, _t, conversationId) => {
        steps.push(`finish ${issue} ${outcome} ${conversationId}`)
        return 1
      },
      abort: (issue, conversationId) => {
        steps.push(`abort ${issue} ${conversationId}`)
        return true
      },
    })
    expect(ended).toBe(true)
    expect(steps).toEqual([
      'salvage 42 reaped conv-42',
      'store 42 runner-a',
      `finish 42 ${DISPATCH_OUTCOME_LABELS.reapedDrained} conv-42`,
      'abort 42 conv-42',
    ])
    expect(DISPATCH_OUTCOME_LABELS.reapedDrained.startsWith('reaped')).toBe(
      true,
    )
  })

  it('a salvage that fails still frees the issue and stops the bee', async () => {
    const steps: string[] = []
    const ended = await handBackBee({} as Pool, 'runner-a', order, {
      salvage: async () => {
        throw new Error('index.lock')
      },
      store: async () => {
        steps.push('store')
      },
      finish: async () => {
        steps.push('finish')
        return 0
      },
      abort: () => {
        steps.push('abort')
        return false
      },
    })
    expect(ended).toBe(false)
    expect(steps).toEqual(['finish', 'abort'])
  })
})
