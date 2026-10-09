/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE REVIEWER'S POOL FOLLOWS ITS BACKLOG (trios#1712; gHashTag/t27
 * specs/queen/reviewer_sizing.t27). With a `capacity`, the reviewer's workers
 * are a pool the intake sizes by the card: grown under start_answer, shrunk by
 * worker_retires, never past the lanes it may fill. Run under a virtual clock,
 * with reviews that end only when the test lets them.
 */

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createActorSystem } from '../../src/api/services/queen-actors'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import {
  freeMemoryMb,
  isZaiLane,
  keyFreeLanes,
  type PoolSize,
  poolTarget,
  REVIEWER_SIZING_CARD,
  reviewerTree,
} from '../../src/api/services/queen-review-actors'
import {
  type Judged,
  REVIEWER_CARD,
} from '../../src/api/services/queen-review-loop'
import {
  REVIEWER_CEILING,
  REVIEWER_CONCURRENCY,
  REVIEWER_EVERY_SECONDS,
} from '../../src/api/services/queen-reviewer-card.gen'
import {
  POOL_IDLE_STOP_SECONDS,
  REVIEW_MB_UNTIL_MEASURED,
  ZAI_REQUESTS_PER_KEY,
} from '../../src/api/services/queen-reviewer-sizing-card.gen'
import { reviewCapacity } from '../../src/api/services/queen-tick'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

const judged = (): Judged => ({ acted: [], strays: [], tally: [] })
const SECOND = 1000

describe('the vendored sizing card is the one PIN names', () => {
  it('reviewer_sizing.t27 and reviewer_sizing.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/reviewer_sizing.t27 sha256 ${sha('queen/reviewer_sizing.t27')}`,
    )
    expect(pin).toContain(
      `queen/reviewer_sizing.wasm sha256 ${sha('queen/reviewer_sizing.wasm')}`,
    )
  })

  it('compiles in the same reviewer_concurrency the reviewer card has', () => {
    // the sizing card imports it from reviewer.t27; a card built against an
    // older reviewer.t27 would answer differently here
    const sizing = loadCardWasm(REVIEWER_SIZING_CARD)
    const reviewer = loadCardWasm(REVIEWER_CARD)
    for (const keys of [0, 1, 3, 7, 47, 100])
      for (const lanes of [0, 1, 2, 10])
        for (const [free, per] of [
          [0, 0],
          [1000, 300],
          [8192, 512],
          [100_000, 1],
        ])
          for (const waiting of [0, 1, 5, 111]) {
            const args = [keys, lanes, free, per, waiting]
            expect(sizing.call('reviewer_concurrency', ...args)).toBe(
              reviewer.call('reviewer_concurrency', ...args),
            )
          }
  })
})

/**
 * A reviewer tree with a pool, over `lanes` lanes. Each review takes a lane
 * at once and ends when `finish` is called for its row.
 */
function adaptiveTree(o: {
  lanes: number
  freeMb?: number
  mbPerReview?: number
  failFirst?: Set<number>
}) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock)
  const rows = new Set<number>()
  const ends = new Map<number, () => void>()
  const laneFree = Array.from({ length: o.lanes }, (_, i) => i)
  let calls = 0
  let mostCalls = 0
  let mostLive = 0
  const sizes: PoolSize[] = []
  const r = reviewerTree(sys, {
    holdsLease: async () => true,
    waiting: async () => [...rows],
    reviewOne: (issue, _reserved, onLane) =>
      new Promise<Judged>((resolve, reject) => {
        if (o.failFirst?.delete(issue)) {
          clock.after(SECOND, () => reject(new Error('review crashed')))
          return
        }
        const lane = laneFree.shift()
        if (lane === undefined) {
          resolve({ acted: [`#${issue}:wait`], strays: [], tally: [] })
          return
        }
        onLane(lane)
        calls++
        mostCalls = Math.max(mostCalls, calls)
        ends.set(issue, () => {
          ends.delete(issue)
          laneFree.push(lane)
          calls--
          rows.delete(issue)
          resolve(judged())
        })
      }),
    capacity: async (reserved) => ({
      freeLanes: o.lanes - reserved.length,
      freeMb: o.freeMb ?? 8192,
      mbPerReview: o.mbPerReview ?? 512,
    }),
    onResize: (size) => {
      sizes.push(size)
      mostLive = Math.max(mostLive, size.live)
    },
  })
  r.tree.start(() => {})
  return {
    clock,
    r,
    sizes,
    add: (...issues: number[]) => {
      for (const n of issues) rows.add(n)
      r.wake()
    },
    finishAll: () => {
      for (const end of [...ends.values()]) end()
    },
    calls: () => calls,
    mostCalls: () => mostCalls,
    mostLive: () => mostLive,
  }
}

const range = (from: number, n: number) =>
  Array.from({ length: n }, (_, i) => from + i)

describe('the adaptive pool follows the card', () => {
  it('backlog 0: no worker', async () => {
    const t = adaptiveTree({ lanes: 47 })
    t.r.wake()
    await t.clock.runUntil(60 * SECOND)
    expect(t.r.size().target).toBe(0)
    expect(t.r.size().live).toBe(0)
    expect(t.calls()).toBe(0)
  })

  it('a small backlog: one worker a row', async () => {
    const t = adaptiveTree({ lanes: 47 })
    t.add(1, 2, 3)
    await t.clock.runUntil(30 * SECOND)
    // every row under review: busy 3, queued 0, lanes 44 free + 3 held
    expect(t.r.size()).toMatchObject({ live: 3, busy: 3, queued: 0 })
    expect(t.r.size().target).toBe(poolTarget(44 + 3, 8192, 512, 3, 0))
    expect(t.r.size().target).toBe(3)
    expect(t.calls()).toBe(3)
  })

  it('a huge backlog: the ceiling, and the rest wait their turn', async () => {
    const t = adaptiveTree({ lanes: 47 })
    t.add(...range(100, 200))
    await t.clock.runUntil(30 * SECOND)
    expect(t.r.size().live).toBe(REVIEWER_CEILING)
    expect(t.r.size().target).toBe(
      poolTarget(47, 8192, 512, REVIEWER_CEILING, 200 - REVIEWER_CEILING),
    )
    expect(t.calls()).toBe(REVIEWER_CEILING)
    expect(t.mostLive()).toBe(REVIEWER_CEILING)
  })

  it('never more workers or calls than the lanes, whatever the backlog', async () => {
    const t = adaptiveTree({ lanes: 5 })
    t.add(...range(100, 200))
    await t.clock.runUntil(30 * SECOND)
    // three heartbeats later every lane is held by a busy worker: those lanes
    // are still the pool's own (pool_lanes), so the target stays 5
    expect(t.r.size()).toMatchObject({ live: 5, busy: 5, target: 5, lanes: 5 })
    // reviews end and more rows come: still never past the lanes
    for (let k = 0; k < 6; k++) {
      t.finishAll()
      t.add(...range(1000 + k * 50, 50))
      await t.clock.runUntil((60 + k * 30) * SECOND)
      expect(t.r.size().live).toBeLessThanOrEqual(5)
    }
    expect(t.mostLive()).toBe(5)
    expect(t.mostCalls()).toBe(5)
  })

  it('memory nobody measured is charged REVIEW_MB_UNTIL_MEASURED a review', async () => {
    const t = adaptiveTree({ lanes: 47, freeMb: 1536, mbPerReview: 0 })
    t.add(...range(100, 50))
    await t.clock.runUntil(30 * SECOND)
    expect(t.r.size().live).toBe(Math.floor(1536 / REVIEW_MB_UNTIL_MEASURED))
  })

  it('an idle worker stops only after POOL_IDLE_STOP_SECONDS, and the pool grows again for the next burst', async () => {
    const t = adaptiveTree({ lanes: 47 })
    t.add(...range(100, 10))
    await t.clock.runUntil(30 * SECOND)
    expect(t.r.size().live).toBe(10)
    // the backlog drains: the target falls to 0 at once, the workers stay a while
    t.finishAll()
    t.r.wake()
    await t.clock.runUntil(40 * SECOND)
    expect(t.r.size().target).toBe(0)
    expect(t.r.size().live).toBe(10)
    await t.clock.runUntil(
      (30 + POOL_IDLE_STOP_SECONDS - REVIEWER_EVERY_SECONDS) * SECOND,
    )
    expect(t.r.size().live).toBe(10)
    // a heartbeat after the idle bound: every idle worker has stopped
    await t.clock.runUntil(
      (30 + POOL_IDLE_STOP_SECONDS + 2 * REVIEWER_EVERY_SECONDS) * SECOND,
    )
    expect(t.r.size().live).toBe(0)
    // the next burst starts them again
    t.add(...range(500, 4))
    await t.clock.runUntil(
      (30 + POOL_IDLE_STOP_SECONDS + 3 * REVIEWER_EVERY_SECONDS) * SECOND,
    )
    expect(t.r.size()).toMatchObject({ live: 4, busy: 4 })
  })

  it('a worker that crashes comes back in its place: the pool does not grow for it', async () => {
    const t = adaptiveTree({ lanes: 47, failFirst: new Set([2]) })
    t.add(1, 2, 3)
    await t.clock.runUntil(5 * SECOND)
    // row 2's review crashed; its worker is waiting out the card's backoff
    expect(t.r.size().live).toBe(3)
    // it came back after the backoff (well inside REVIEW_ROW_SECONDS), and
    // row 2 went back to the queue and is reviewed again
    await t.clock.runUntil(60 * SECOND)
    expect(t.r.size()).toMatchObject({ live: 3, busy: 3 })
    expect(t.calls()).toBe(3)
  })
})

describe('the fixed pool is unchanged without a capacity', () => {
  it('REVIEWER_CONCURRENCY reviews at once, however long the backlog', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    let calls = 0
    let most = 0
    const r = reviewerTree(sys, {
      holdsLease: async () => true,
      waiting: async () => range(1, 100),
      reviewOne: async () => {
        calls++
        most = Math.max(most, calls)
        await new Promise<void>((res) => clock.after(60 * SECOND, res))
        calls--
        return judged()
      },
    })
    r.tree.start(() => {})
    r.wake()
    await clock.runUntil(600 * SECOND)
    expect(most).toBe(REVIEWER_CONCURRENCY)
  })
})

describe('what the pool is sized from', () => {
  it('a z.ai key counts no more than the two requests it carries', () => {
    const zai = { provider: 'zai' }
    const pool = {
      provider: 'openai-compatible',
      baseUrl: 'https://api.z.ai/api/paas/v4',
    }
    const other = { provider: 'openai-compatible', baseUrl: 'https://x.test' }
    expect(isZaiLane(zai)).toBe(true)
    expect(isZaiLane(pool)).toBe(true)
    expect(isZaiLane(other)).toBe(false)
    // four lanes configured, one carried: z.ai has one left, another provider three
    expect(keyFreeLanes(4, 1, isZaiLane(zai))).toBe(ZAI_REQUESTS_PER_KEY - 1)
    expect(keyFreeLanes(4, 1, isZaiLane(other))).toBe(3)
    expect(keyFreeLanes(4, 2, true)).toBe(0)
  })

  it('free memory is the cgroup limit less its usage, else what the OS says', () => {
    const files = (m: Record<string, string>) => (path: string) => {
      const v = m[path]
      if (v === undefined) throw new Error('ENOENT')
      return v
    }
    expect(
      freeMemoryMb(
        files({
          '/sys/fs/cgroup/memory.max': `${8 * 1024 * 1_048_576}\n`,
          '/sys/fs/cgroup/memory.current': `${3 * 1024 * 1_048_576}\n`,
        }),
      ),
    ).toBe(5 * 1024)
    expect(
      freeMemoryMb(
        files({
          '/sys/fs/cgroup/memory/memory.limit_in_bytes': `${2 * 1_048_576 * 1024}`,
          '/sys/fs/cgroup/memory/memory.usage_in_bytes': `${512 * 1_048_576}`,
        }),
      ),
    ).toBe(1536)
    // "max": no limit, so the OS's figure - some positive number of MB
    const unlimited = freeMemoryMb(
      files({
        '/sys/fs/cgroup/memory.max': 'max\n',
        '/sys/fs/cgroup/memory.current': '1\n',
      }),
    )
    expect(unlimited).toBeGreaterThan(0)
  })

  describe('the lanes production counts', () => {
    // the lane arithmetic reads these; each test sets its own and puts back
    // whatever the runner had
    const ENV = [
      'TRIOS_QUEEN_WORKER_BASE_URL',
      'TRIOS_QUEEN_WORKER_PROVIDER',
      'TRIOS_QUEEN_WORKER_MODEL',
      'TRIOS_QUEEN_WORKER_API_KEY',
      'TRIOS_QUEEN_WORKER_API_KEY_2',
      'TRIOS_QUEEN_WORKER_LANES_PER_KEY',
      'TRIOS_QUEEN_REVIEW_EXTRA_LANES',
      'TRIOS_QUEEN_REVIEW_MB',
      'QUEEN_CONTRIBUTOR_PROXY_TOKEN',
      'ZAI_API_KEY',
      'ANTHROPIC_API_KEY',
      'OPENROUTER_API_KEY',
      'MOONSHOT_API_KEY',
      'OPENAI_API_KEY',
    ]
    const saved: Record<string, string | undefined> = {}
    beforeEach(() => {
      for (const key of ENV) {
        saved[key] = process.env[key]
        delete process.env[key]
      }
    })
    afterEach(() => {
      for (const [key, value] of Object.entries(saved)) {
        if (value === undefined) delete process.env[key]
        else process.env[key] = value
      }
    })
    // no bee running, no contributor registry
    const emptyDb = { query: async () => ({ rows: [], rowCount: 0 }) }
    const twoKeys = (baseUrl: string, provider: string) => {
      process.env.TRIOS_QUEEN_WORKER_BASE_URL = baseUrl
      process.env.TRIOS_QUEEN_WORKER_PROVIDER = provider
      process.env.TRIOS_QUEEN_WORKER_MODEL = 'some-model'
      process.env.TRIOS_QUEEN_WORKER_API_KEY = 'not-a-real-key-a'
      process.env.TRIOS_QUEEN_WORKER_API_KEY_2 = 'not-a-real-key-b'
      process.env.TRIOS_QUEEN_WORKER_LANES_PER_KEY = '4'
    }

    it('two z.ai keys configured at four lanes carry two each, less what reviews hold', async () => {
      twoKeys('https://api.z.ai/api/paas/v4', 'zai')
      const db = emptyDb as unknown as Parameters<typeof reviewCapacity>[0]
      expect((await reviewCapacity(db, [])).freeLanes).toBe(
        2 * ZAI_REQUESTS_PER_KEY,
      )
      // a review holds a lane of key 0: one left there, two on key 1
      expect((await reviewCapacity(db, [0])).freeLanes).toBe(3)
      expect((await reviewCapacity(db, [0, 0, 1, 1])).freeLanes).toBe(0)
    })

    it('another provider keeps its configured lanes, and its review lane', async () => {
      twoKeys('https://llm.example.invalid/v1', 'openai-compatible')
      const db = emptyDb as unknown as Parameters<typeof reviewCapacity>[0]
      // four lanes a key and one review lane of its own (reviewExtraLanesPerCredential)
      expect((await reviewCapacity(db, [])).freeLanes).toBe(2 * 5)
      process.env.TRIOS_QUEEN_REVIEW_MB = '700'
      expect((await reviewCapacity(db, [])).mbPerReview).toBe(700)
    })
  })
})
