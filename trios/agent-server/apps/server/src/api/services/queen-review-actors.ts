/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE REVIEWER AS ACTORS (t27#7851, MVP). This is the first domain on the
 * actor runtime. It does the same work as queen-review-loop.ts, arranged as a
 * supervision tree:
 *
 *   reviewer domain supervisor: rest_for_one, at most DOMAIN_MAX_RESTARTS
 *   restarts in DOMAIN_PERIOD_SECONDS
 *     intake: knows which rows wait and which workers are idle, and hands one
 *       row to one idle worker (review_slots, visit_first)
 *     worker x REVIEWER_CONCURRENCY: one review per turn, and a turn is
 *       killed at REVIEW_ROW_SECONDS (turn_signal). The kill frees the worker.
 *       The review itself runs on until it ends, holding its row and its key
 *       lane.
 *
 * Why rest_for_one and not the card's one_for_one for agent domains: workers
 * report to the intake by pid. An intake that comes back has to find them
 * again, and rest_for_one restarts every worker started after it.
 *
 * The intake wakes in two ways: on a bus event (`wake`, a bee finished) and on
 * a heartbeat every REVIEWER_EVERY_SECONDS, posted to its control lane. When a
 * worker dies, its DOWN sends its row back to the front of the queue.
 *
 * Started only with TRIOS_QUEEN_REVIEWER=actors. The default stays the loop
 * until the benchmark's numbers are accepted (t27#7851).
 *
 * THE POOL FOLLOWS THE BACKLOG (TRIOS_QUEEN_REVIEWER_ADAPTIVE=1, trios#1712;
 * gHashTag/t27 specs/queen/reviewer_sizing.t27). The fixed workers become a
 * pool the intake sizes:
 *
 *   reviewer domain supervisor: rest_for_one
 *     intake
 *     reviewer pool: a dynamic supervisor, one_for_one over transient workers
 *
 * On every wake the intake reads the lanes and the memory free (`capacity`).
 * After every message it asks the card for the pool's size (pool_target, which
 * is reviewer.t27 reviewer_concurrency over the pool's lanes, its memory and
 * its busy and queued rows), then:
 *   - grows: the pool starts workers while start_answer allows them under
 *     that size;
 *   - shrinks: an idle worker stops once worker_retires says the pool is too
 *     big and the worker has been idle long enough, as Akka passivates an
 *     idle entity;
 *   - hands out review_slots_within(size, busy, queued) rows.
 * Off by default: the fixed REVIEWER_CONCURRENCY stays until the benchmark and
 * production telemetry are accepted (t27#7851).
 */

import { readFileSync } from 'node:fs'
import { freemem } from 'node:os'
import type { Pool } from 'pg'
import { logger } from '../../lib/logger'
import {
  type ActorSystem,
  actorChild,
  type Child,
  createActorSystem,
  type Down,
  type Pid,
  supervisor,
} from './queen-actors'
import {
  DOMAIN_MAX_RESTARTS,
  DOMAIN_PERIOD_SECONDS,
  M_HEARTBEAT_DUE,
  RESTART_PERMANENT,
  RESTART_TRANSIENT,
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  START_OK,
  STRAT_ONE_FOR_ONE,
  STRAT_REST_FOR_ONE,
} from './queen-actors-card.gen'
import { type DynamicChildren, dynamicSupervisor } from './queen-actors-dynamic'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { addControlEventReader } from './queen-control'
import { EV_TASK_ENDED } from './queen-control.gen'
import {
  type Judged,
  type ReviewFn,
  recordJudged,
  reviewerDeps,
  reviewSlots,
  reviewSlotsWithin,
  setReviewerRunning,
  visitFirst,
} from './queen-review-loop'
import {
  REVIEW_ROW_SECONDS,
  REVIEWER_CEILING,
  REVIEWER_CONCURRENCY,
  REVIEWER_EVERY_SECONDS,
} from './queen-reviewer-card.gen'

export const REVIEWER_SIZING_CARD = 'queen/reviewer_sizing.wasm'

const sizing = () => loadCardWasm(REVIEWER_SIZING_CARD)
/** Lanes one key still has for a review (key_free_lanes). */
export const keyFreeLanes = (
  configured: number,
  carried: number,
  zai: boolean,
): number =>
  sizing().call('key_free_lanes', u32(configured), u32(carried), flag(zai)) >>>
  0
/** Lanes the pool may fill: the free ones and its busy reviews' own. */
export const poolLanes = (freeLanes: number, heldByBusy: number): number =>
  sizing().call('pool_lanes', u32(freeLanes), u32(heldByBusy)) >>> 0
/** How many workers the pool should have. */
export const poolTarget = (
  lanes: number,
  freeMb: number,
  mbPerReview: number,
  busy: number,
  queued: number,
): number =>
  sizing().call(
    'pool_target',
    u32(lanes),
    u32(freeMb),
    u32(mbPerReview),
    u32(busy),
    u32(queued),
  ) >>> 0
/** Whether one idle worker stops now. */
export const workerRetires = (
  live: number,
  target: number,
  idleMs: number,
): boolean =>
  sizing().call(
    'worker_retires',
    u32(live),
    u32(target),
    u32(idleMs / 1000),
  ) !== 0

/** What the pool is sized from, read on every wake. */
export interface ReviewCapacity {
  /** Lanes free for a review now, summed key by key (keyFreeLanes). */
  freeLanes: number
  /** Megabytes this process may still use. */
  freeMb: number
  /** Megabytes one review was measured to take; 0 = not measured. */
  mbPerReview: number
}

/** The pool as the intake last sized it. */
export interface PoolSize {
  target: number
  live: number
  busy: number
  idle: number
  queued: number
  lanes: number
}

/**
 * Whether a lane is a z.ai key, the provider whose per-key limit is measured.
 * The same test environmentContributorKeys makes.
 */
export const isZaiLane = (lane: {
  provider?: string
  baseUrl?: string
}): boolean =>
  lane.provider === 'zai' || lane.baseUrl === 'https://api.z.ai/api/paas/v4'

/**
 * Megabytes this process may still use: the container's cgroup limit less its
 * usage when it has one (Railway runs the server in a container), else what
 * the OS calls free.
 */
export function freeMemoryMb(
  read: (path: string) => string = (path) => readFileSync(path, 'utf8'),
): number {
  const mb = (bytes: number) => Math.floor(bytes / 1_048_576)
  for (const [limit, usage] of [
    ['/sys/fs/cgroup/memory.max', '/sys/fs/cgroup/memory.current'],
    [
      '/sys/fs/cgroup/memory/memory.limit_in_bytes',
      '/sys/fs/cgroup/memory/memory.usage_in_bytes',
    ],
  ]) {
    try {
      // "max", or v1's 2^63 rounded down to a page: no limit, so the OS decides
      const l = Number(read(limit).trim())
      const u = Number(read(usage).trim())
      if (Number.isFinite(l) && Number.isFinite(u) && l > 0 && l < 2 ** 60)
        return mb(Math.max(0, l - u))
    } catch {
      // not this cgroup version
    }
  }
  return mb(freemem())
}

/**
 * Megabytes one review takes, as the operator measured it
 * (TRIOS_QUEEN_REVIEW_MB). 0 when nobody has: the card then charges
 * REVIEW_MB_UNTIL_MEASURED.
 */
export function measuredReviewMb(
  raw = process.env.TRIOS_QUEEN_REVIEW_MB,
): number {
  const parsed = Number(raw)
  return Number.isInteger(parsed) && parsed > 0 ? parsed : 0
}

/** The capacity reading, given the key indices reviews hold now. */
export type ReviewCapacityFn = (
  reservedKeys: number[],
) => Promise<ReviewCapacity>

export interface ReviewerActorDeps {
  holdsLease: () => Promise<boolean>
  waiting: () => Promise<number[]>
  reviewOne: (
    issue: number,
    reservedKeys: () => number[],
    onLane: (keyIndex: number | undefined) => void,
  ) => Promise<Judged>
  onJudged?: (round: Judged) => void
  workers?: number
  /**
   * Only with TRIOS_QUEEN_REVIEWER_ADAPTIVE=1: what the pool is sized from,
   * given the key indices reviews hold now. Without it the pool is the fixed
   * `workers` (REVIEWER_CONCURRENCY).
   */
  capacity?: ReviewCapacityFn
  /** Told when the pool's size or its target changes. */
  onResize?: (size: PoolSize) => void
}

type IntakeMsg =
  | { kind: 'wake' }
  | { kind: 'up'; worker: Pid }
  | { kind: 'done'; worker: Pid; issue: number }
  | Down

/** The reviewer domain as one child, for a root supervisor or a test. */
export function reviewerTree(sys: ActorSystem, deps: ReviewerActorDeps) {
  const clock = sys.clock
  let intake: Pid | undefined
  // The rows under review right now, each with the key lane it holds. This
  // belongs to the review itself, not to the worker. A killed turn's review
  // keeps running until it ends on its own. Until then its row is not handed
  // out again and its lane stays reserved, so no row is ever reviewed twice at
  // once and no lane is double-booked. This is the loop's inFlight guarantee,
  // kept.
  const reviewing = new Map<number, number | undefined>()
  // the adaptive pool's handle, from its dynamic supervisor's latest start
  let pool: DynamicChildren | undefined
  let size: PoolSize = {
    target: 0,
    live: 0,
    busy: 0,
    idle: 0,
    queued: 0,
    lanes: 0,
  }

  const intakeSpec = () => {
    let queue: number[] = []
    const busy = new Map<Pid, number>()
    const lastVisited = new Map<number, number>()
    let idle: Pid[] = []
    const idleSince = new Map<Pid, number>()
    let wakeQueued = false
    // adaptive only: the pool's lanes and memory as the last wake read them
    let lanes = 0
    let freeMb = 0
    let mbPerReview = 0

    const goIdle = (worker: Pid) => {
      idle.push(worker)
      idleSince.set(worker, clock.now())
    }
    // adaptive only: the card sizes the pool; returns the size
    const resize = (): number | undefined => {
      if (!deps.capacity || pool === undefined) return undefined
      const target = poolTarget(
        lanes,
        freeMb,
        mbPerReview,
        busy.size,
        queue.length,
      )
      let started = 0
      while (started < REVIEWER_CEILING && pool.startChild(target) === START_OK)
        started++
      const at = clock.now()
      const since = (w: Pid) => idleSince.get(w) ?? at
      for (const w of [...idle].sort((a, b) => since(a) - since(b))) {
        if (!workerRetires(pool.live(), target, at - since(w))) continue
        idle = idle.filter((p) => p !== w)
        idleSince.delete(w)
        pool.stopChild(w)
      }
      const next: PoolSize = {
        target,
        live: pool.live(),
        busy: busy.size,
        idle: idle.length,
        queued: queue.length,
        lanes,
      }
      if (next.target !== size.target || next.live !== size.live)
        deps.onResize?.(next)
      size = next
      return target
    }
    const dispatch = (target: number | undefined) => {
      const slots =
        target === undefined
          ? reviewSlots(busy.size, queue.length)
          : reviewSlotsWithin(target, busy.size, queue.length)
      const free = Math.min(idle.length, slots)
      for (let k = 0; k < free; k++) {
        const issue = queue.shift()
        if (issue !== undefined && reviewing.has(issue)) continue
        const worker = idle.shift()
        if (issue === undefined || worker === undefined) return
        idleSince.delete(worker)
        busy.set(worker, issue)
        lastVisited.set(issue, clock.now())
        sys.send(worker, { issue })
      }
    }
    const beat = (self: Pid) =>
      clock.after(REVIEWER_EVERY_SECONDS * 1000, () => {
        sys.post(self, M_HEARTBEAT_DUE)
      })
    // adaptive only: read before any await, so the lanes the busy workers
    // hold and the lanes reserved are one snapshot
    const measure = async (capacity: ReviewCapacityFn) => {
      const reserved = reservedKeys()
      const held = [...busy.values()].filter(
        (n) => typeof reviewing.get(n) === 'number',
      ).length
      try {
        const cap = await capacity(reserved)
        lanes = poolLanes(cap.freeLanes, held)
        freeMb = cap.freeMb
        mbPerReview = cap.mbPerReview
      } catch (error) {
        // the last reading stands: a failed read must not take the intake down
        logger.warn('Queen reviewer could not read its capacity', {
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }

    return {
      name: 'reviewer-intake',
      init: (self: Pid) => {
        intake = self
        beat(self)
      },
      control: (tag: number, self: Pid) => {
        if (tag !== M_HEARTBEAT_DUE) return
        if (!wakeQueued) {
          wakeQueued = true
          sys.send(self, { kind: 'wake' })
        }
        beat(self)
      },
      receive: async (msg: IntakeMsg, self: Pid) => {
        if (msg.kind === 'up') {
          sys.monitor(self, msg.worker)
          goIdle(msg.worker)
        } else if (msg.kind === 'done') {
          if (!busy.has(msg.worker)) return
          busy.delete(msg.worker)
          goIdle(msg.worker)
          queue = queue.filter((n) => n !== msg.issue)
        } else if (msg.kind === 'DOWN') {
          const issue = busy.get(msg.pid)
          busy.delete(msg.pid)
          idle = idle.filter((p) => p !== msg.pid)
          idleSince.delete(msg.pid)
          // a crashed review has ended: its row goes back at once. A killed one
          // may still be running; its row waits for it to end (`reviewing`)
          if (issue !== undefined && !reviewing.has(issue)) queue.unshift(issue)
        } else {
          wakeQueued = false
          if (!(await deps.holdsLease())) return
          if (deps.capacity) await measure(deps.capacity)
          const inFlight = new Set(busy.values())
          const at = clock.now()
          queue = (await deps.waiting())
            .filter((n) => !inFlight.has(n) && !reviewing.has(n))
            .sort((a, b) => {
              if (visitFirst(lastVisited.get(a), lastVisited.get(b), at))
                return -1
              if (visitFirst(lastVisited.get(b), lastVisited.get(a), at))
                return 1
              return 0
            })
        }
        dispatch(resize())
      },
    }
  }

  const intakeChild: Child = {
    name: 'reviewer-intake',
    start: (onExit, slot) => actorChild(sys, intakeSpec()).start(onExit, slot),
  }
  const reservedKeys = () =>
    [...reviewing.values()].filter((k): k is number => typeof k === 'number')
  const worker = (i: number, restart: number): Child =>
    actorChild(
      sys,
      {
        name: `reviewer-worker-${i}`,
        turnMaxSeconds: REVIEW_ROW_SECONDS,
        init: (self: Pid) => {
          if (intake !== undefined)
            sys.send(intake, { kind: 'up', worker: self })
        },
        receive: async (msg: { issue: number }, self: Pid) => {
          reviewing.set(msg.issue, undefined)
          let round: Judged
          try {
            round = await deps.reviewOne(
              msg.issue,
              reservedKeys,
              (keyIndex) => {
                if (typeof keyIndex === 'number' && reviewing.has(msg.issue))
                  reviewing.set(msg.issue, keyIndex)
              },
            )
          } finally {
            reviewing.delete(msg.issue)
          }
          // what a review did happened even if its turn was killed: report it
          deps.onJudged?.(round)
          if (intake !== undefined)
            sys.send(
              intake,
              { kind: 'done', worker: self, issue: msg.issue },
              self,
            )
        },
      },
      restart,
    )

  // A worker the pool stops is transient: it stays down. One that crashes or
  // is killed comes back in its slot, after the card's backoff.
  const workers: Child[] = deps.capacity
    ? [
        dynamicSupervisor(
          sys,
          {
            name: 'reviewer-pool',
            maxRestarts: DOMAIN_MAX_RESTARTS,
            periodSeconds: DOMAIN_PERIOD_SECONDS,
          },
          (i) => worker(i, RESTART_TRANSIENT),
          (children) => {
            pool = children
          },
        ),
      ]
    : Array.from({ length: deps.workers ?? REVIEWER_CONCURRENCY }, (_, i) =>
        worker(i, RESTART_PERMANENT),
      )
  const tree = supervisor(
    sys,
    {
      name: 'reviewer-domain',
      strategy: STRAT_REST_FOR_ONE,
      maxRestarts: DOMAIN_MAX_RESTARTS,
      periodSeconds: DOMAIN_PERIOD_SECONDS,
    },
    [intakeChild, ...workers],
  )
  return {
    tree,
    /** A bus event: something finished and may want a review. */
    wake: () => {
      if (intake !== undefined) sys.send(intake, { kind: 'wake' })
    },
    /** The adaptive pool as the intake last sized it. */
    size: (): PoolSize => size,
  }
}

/**
 * Start the reviewer as actors in this process: a root supervisor
 * (one_for_one, ROOT_MAX_RESTARTS in ROOT_PERIOD_SECONDS) over the reviewer
 * domain, woken by the bus on every queen/task.ended (EV_TASK_ENDED). With
 * TRIOS_QUEEN_REVIEWER_ADAPTIVE=1 and a `capacity`, the pool follows the
 * backlog (reviewer_sizing.t27).
 */
export function startReviewerActors(
  pool: Pool,
  leaseName: string,
  review: ReviewFn,
  waiting: (pool: Pool) => Promise<number[]>,
  capacity?: (pool: Pool, reservedKeys: number[]) => Promise<ReviewCapacity>,
): () => void {
  const adaptive =
    capacity !== undefined && process.env.TRIOS_QUEEN_REVIEWER_ADAPTIVE === '1'
  const sys = createActorSystem()
  const r = reviewerTree(sys, {
    ...reviewerDeps(pool, leaseName, review, waiting),
    onJudged: recordJudged,
    ...(adaptive
      ? {
          capacity: (reserved: number[]) => capacity(pool, reserved),
          // one line per change, for the arithmetic before the flag goes on
          onResize: (size: PoolSize) =>
            logger.info('Queen reviewer pool', { ...size }),
        }
      : {}),
  })
  const root = supervisor(
    sys,
    {
      name: 'queen-root',
      strategy: STRAT_ONE_FOR_ONE,
      maxRestarts: ROOT_MAX_RESTARTS,
      periodSeconds: ROOT_PERIOD_SECONDS,
    },
    [r.tree],
  ).start((reason) => {
    // the root gave up: reviewing goes back to the round, and the log says so
    setReviewerRunning(false)
    logger.warn('Queen reviewer actors gave up; the round reviews again', {
      reason,
    })
  })
  const unread = addControlEventReader((event) => {
    if (event.kind === EV_TASK_ENDED) r.wake()
  })
  setReviewerRunning(true)
  r.wake()
  logger.info('Queen reviewer starting as actors', {
    workers: adaptive ? 'adaptive' : REVIEWER_CONCURRENCY,
    turnMaxSeconds: REVIEW_ROW_SECONDS,
  })
  return () => {
    unread()
    root.stop()
    setReviewerRunning(false)
  }
}
