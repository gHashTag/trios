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
 * With TRIOS_QUEEN_TURN_STOP=on (turn_stop.t27, trios#1713) the kill also
 * stops the review: its abort signal reaches the model call and the git and
 * criterion commands, so it ends at once and gives back its row and lane. A
 * review that ignores its abort still holds them, and is counted against the
 * bound (in_flight) until it ends: no more than the bound's reviews run at
 * once, stalled ones included. Without the flag a killed review is abandoned,
 * as before, and its worker's replacement takes a new row beside it.
 *
 * Why rest_for_one and not the card's one_for_one for agent domains: workers
 * report to the intake by pid. An intake that comes back has to find them
 * again, and rest_for_one restarts every worker started after it. With
 * TRIOS_QUEEN_TURN_STOP=on the fixed workers sit under their own one_for_one
 * supervisor below the intake (see `workers`).
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
 *
 * A REVIEW WITH NO LANE WAITS FOR ONE (TRIOS_QUEEN_REVIEW_LANES=semaphore,
 * trios#1729 item 4; gHashTag/t27 specs/queen/review_lanes.t27). The domain
 * gets one more child, first, so the workers that call it come back when it
 * does:
 *
 *   reviewer domain supervisor: rest_for_one
 *     review-lanes: owns the queue of reviews waiting for a model lane
 *     intake
 *     workers, or the reviewer pool
 *
 * Each review gets a gate (queen-review-lanes.ts). At its model call with no
 * lane free it waits there instead of answering `wait`. With the adaptive
 * pool, the pool is sized by pool_target_queued: past its lanes by the share
 * of a review measured on no lane. Works with the fixed workers too, and with
 * TRIOS_QUEEN_TURN_STOP=on a stopped review leaves the queue or gives back its
 * lane.
 */

import { readFileSync } from 'node:fs'
import { freemem } from 'node:os'
import type { Pool } from 'pg'
import { logger } from '../../lib/logger'
import {
  actorEventsOn,
  actorEventWriter,
  createActorEvents,
  setLiveActorEventWriter,
  type TaskRef,
} from './queen-actor-events'
import {
  ACTORS_CARD,
  type ActorSystem,
  actorChild,
  type Child,
  type Clock,
  createActorSystem,
  type Down,
  type Pid,
  realClock,
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
import { setLiveActorTelemetry } from './queen-actors-telemetry'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { addControlEventReader, appendEvent } from './queen-control'
import { EV_TASK_ENDED } from './queen-control.gen'
import { ACTOR_STREAM } from './queen-events.gen'
import {
  type LaneGate,
  poolTargetQueued,
  REVIEW_LANES_CARD,
  type ReviewLanes,
  reviewLanes,
  reviewLanesOn,
} from './queen-review-lanes'
import {
  type Judged,
  REVIEWER_CARD,
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
import { TK_ISSUE } from './queen-tasks.gen'
import { TURN_STOP_CARD, inFlight as turnsInFlight } from './queen-turn-stop'

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
  /** With the lane queue: the measured share of a review on its lane. */
  holdPercent?: number
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
  /**
   * `signal` is the worker's turn: aborted when the turn is stopped. `lanes`
   * is the review's gate into the lane queue, with laneQueue only.
   */
  reviewOne: (
    issue: number,
    reservedKeys: () => number[],
    onLane: (keyIndex: number | undefined) => void,
    signal?: AbortSignal,
    lanes?: LaneGate,
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
  /**
   * TRIOS_QUEEN_REVIEW_LANES=semaphore: a review with no lane free waits for
   * one (review_lanes.t27) instead of answering `wait`.
   */
  laneQueue?: boolean
}

type IntakeMsg =
  | { kind: 'wake' }
  | { kind: 'up'; worker: Pid }
  | { kind: 'done'; worker: Pid; issue: number }
  | { kind: 'released'; issue: number }
  | Down

/**
 * The task a reviewer message names, for the actors' feed (actor_events.t27):
 * a row offered to a worker (`{ issue }`), its `done` and its `released` name
 * the issue under review. A wake, an "I am up" and a DOWN name none. Glue: it
 * reads the field; the card decides what a message that names a task does.
 */
export const reviewTaskOf =
  (repo: string) =>
  (msg: unknown): TaskRef | undefined => {
    const issue = (msg as { issue?: unknown } | null)?.issue
    return typeof issue === 'number' && Number.isInteger(issue)
      ? { kind: TK_ISSUE, repo, number: issue }
      : undefined
  }

/** The reviewer domain as one child, for a root supervisor or a test. */
export function reviewerTree(sys: ActorSystem, deps: ReviewerActorDeps) {
  const clock = sys.clock
  let intake: Pid | undefined
  // the lane queue's owner and the gates, with laneQueue only
  const laneQueue: ReviewLanes | undefined = deps.laneQueue
    ? reviewLanes(sys)
    : undefined
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
      const hold = laneQueue?.holdPercent()
      const target =
        hold === undefined
          ? poolTarget(lanes, freeMb, mbPerReview, busy.size, queue.length)
          : poolTargetQueued(
              lanes,
              hold,
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
        ...(hold === undefined ? {} : { holdPercent: hold }),
      }
      if (next.target !== size.target || next.live !== size.live)
        deps.onResize?.(next)
      size = next
      return target
    }
    // Reviews still running whose worker was killed. With turnStop they count
    // against the bound like running ones (turn_stop.t27 in_flight); without
    // it they are left out, as before.
    const heldAfterKill = () => {
      if (!sys.turnStop) return 0
      const assigned = new Set(busy.values())
      let held = 0
      for (const issue of reviewing.keys()) if (!assigned.has(issue)) held++
      return held
    }
    const dispatch = (target: number | undefined) => {
      const running = turnsInFlight(busy.size, heldAfterKill())
      const slots =
        target === undefined
          ? reviewSlots(running, queue.length)
          : reviewSlotsWithin(target, running, queue.length)
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
        } else if (msg.kind === 'released') {
          // a stopped review ended and gave back its slot: fill it now, not at
          // the next heartbeat
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
        kind: 'reviewer-worker',
        turnMaxSeconds: REVIEW_ROW_SECONDS,
        init: (self: Pid) => {
          if (intake !== undefined)
            sys.send(intake, { kind: 'up', worker: self })
        },
        receive: async (
          msg: { issue: number },
          self: Pid,
          _result: unknown,
          signal: AbortSignal,
        ) => {
          reviewing.set(msg.issue, undefined)
          // without turnStop the review runs exactly as before: no signal
          const stop = sys.turnStop ? signal : undefined
          const gate = laneQueue?.gate(self, stop)
          let round: Judged
          try {
            round = await deps.reviewOne(
              msg.issue,
              reservedKeys,
              (keyIndex) => {
                if (typeof keyIndex === 'number' && reviewing.has(msg.issue))
                  reviewing.set(msg.issue, keyIndex)
              },
              stop,
              gate,
            )
          } finally {
            // a review that ended with its lane still marked held gives it
            // back: the queue must not count a holder that is gone
            gate?.released()
            reviewing.delete(msg.issue)
            // a stopped review's worker is dead, so its `done` would be
            // dropped: tell the intake its slot is free, from no one
            if (signal.aborted && intake !== undefined)
              sys.send(intake, { kind: 'released', issue: msg.issue })
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

  const fixed = () =>
    Array.from({ length: deps.workers ?? REVIEWER_CONCURRENCY }, (_, i) =>
      worker(i, RESTART_PERMANENT),
    )
  // WHY THE FIXED WORKERS GET THEIR OWN SUPERVISOR WHEN TURNS STOP. Under one
  // rest_for_one, a killed worker also restarts every worker started after
  // it. While a stop only abandoned a turn, those siblings' reviews ran on and
  // finished. Once a stop really stops, the same restart throws their work
  // away. Measured on the turn-stop benchmark's probe: 39 reviews aborted by
  // a sibling's kill against 24 killed at their own bound. So, as in OTP: the
  // intake, then a one_for_one supervisor of the workers, under rest_for_one.
  // An intake that comes back still restarts every worker; a worker's death
  // restarts that worker alone. The adaptive pool is one_for_one already.
  //
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
    : sys.turnStop
      ? [
          supervisor(
            sys,
            {
              name: 'reviewer-workers',
              strategy: STRAT_ONE_FOR_ONE,
              maxRestarts: DOMAIN_MAX_RESTARTS,
              periodSeconds: DOMAIN_PERIOD_SECONDS,
            },
            fixed(),
          ),
        ]
      : fixed()
  const tree = supervisor(
    sys,
    {
      name: 'reviewer-domain',
      strategy: STRAT_REST_FOR_ONE,
      maxRestarts: DOMAIN_MAX_RESTARTS,
      periodSeconds: DOMAIN_PERIOD_SECONDS,
    },
    [...(laneQueue ? [laneQueue.child] : []), intakeChild, ...workers],
  )
  return {
    tree,
    /** A bus event: something finished and may want a review. */
    wake: () => {
      if (intake !== undefined) sys.send(intake, { kind: 'wake' })
    },
    /** The adaptive pool as the intake last sized it. */
    size: (): PoolSize => size,
    /** The lane queue, with laneQueue only. */
    lanes: laneQueue,
  }
}

/**
 * Start the reviewer as actors in this process: a root supervisor
 * (one_for_one, ROOT_MAX_RESTARTS in ROOT_PERIOD_SECONDS) over the reviewer
 * domain, woken by the bus on every queen/task.ended (EV_TASK_ENDED). With
 * TRIOS_QUEEN_REVIEWER_ADAPTIVE=1 and a `capacity`, the pool follows the
 * backlog (reviewer_sizing.t27).
 *
 * With TRIOS_QUEEN_REVIEW_LANES=semaphore a review that finds no lane free at
 * its model call waits for one (review_lanes.t27), with or without the
 * adaptive pool and turn stop.
 *
 * With TRIOS_QUEEN_ACTORS_TELEMETRY=on the system counts (telemetry.t27):
 * GET /queen/actors/metrics and /queen/actors/decisions read it, and a
 * "Queen actors measured" line lands in the log every SUMMARY_EVERY_SECONDS.
 *
 * With TRIOS_QUEEN_ACTOR_EVENTS=on every spawn, exit, DOWN and restart, and
 * the delivers as actor_events.t27 says, go on the event log's actors stream
 * (events.t27 section 6), which GET /queen/public-actors reads.
 *
 * `clock` is the real one in production; a test hands in a virtual one
 * instead of faking the process's timers.
 */
export function startReviewerActors(
  pool: Pool,
  leaseName: string,
  review: ReviewFn,
  waiting: (pool: Pool) => Promise<number[]>,
  capacity?: (pool: Pool, reservedKeys: number[]) => Promise<ReviewCapacity>,
  clock: Clock = realClock,
): () => void {
  const adaptive =
    capacity !== undefined && process.env.TRIOS_QUEEN_REVIEWER_ADAPTIVE === '1'
  // turn_stop.t27 behind a flag until its benchmark is accepted (t27#7851)
  const turnStop =
    (process.env.TRIOS_QUEEN_TURN_STOP ?? 'off').toLowerCase() === 'on'
  // review_lanes.t27 behind a flag until its benchmark is accepted (t27#7851)
  const laneQueue = reviewLanesOn()
  // telemetry.t27 behind a flag as well: it only reads, but it is new
  const telemetry =
    process.env.TRIOS_QUEEN_ACTORS_TELEMETRY === 'on'
      ? {
          cards: [
            ACTORS_CARD,
            REVIEWER_CARD,
            REVIEWER_SIZING_CARD,
            TURN_STOP_CARD,
            ...(laneQueue ? [REVIEW_LANES_CARD] : []),
          ],
        }
      : undefined
  // actor_events.t27 behind a flag: it writes to the store, and it is new
  const writer = actorEventsOn()
    ? actorEventWriter((kind, payload) =>
        appendEvent(pool, ACTOR_STREAM, kind, payload),
      )
    : undefined
  const events = writer
    ? createActorEvents(clock, {
        sink: writer.sink,
        taskOf: reviewTaskOf(process.env.TRIOS_GITHUB_REPO || 'gHashTag/t27'),
      })
    : undefined
  setLiveActorEventWriter(writer)
  const sys = createActorSystem(clock, { turnStop, telemetry, events })
  setLiveActorTelemetry(sys.telemetry)
  const r = reviewerTree(sys, {
    ...reviewerDeps(pool, leaseName, review, waiting),
    onJudged: recordJudged,
    laneQueue,
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
    turnStop,
    laneQueue,
    telemetry: !!sys.telemetry,
    actorEvents: !!events,
  })
  return () => {
    unread()
    root.stop()
    setReviewerRunning(false)
    sys.telemetry?.close()
    events?.stop()
  }
}
