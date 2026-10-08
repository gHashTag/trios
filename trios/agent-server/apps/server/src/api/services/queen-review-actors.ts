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
 */

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
  ROOT_MAX_RESTARTS,
  ROOT_PERIOD_SECONDS,
  STRAT_ONE_FOR_ONE,
  STRAT_REST_FOR_ONE,
} from './queen-actors-card.gen'
import { addControlEventReader } from './queen-control'
import { EV_TASK_ENDED } from './queen-control.gen'
import {
  type Judged,
  type ReviewFn,
  recordJudged,
  reviewerDeps,
  reviewSlots,
  setReviewerRunning,
  visitFirst,
} from './queen-review-loop'
import {
  REVIEW_ROW_SECONDS,
  REVIEWER_CONCURRENCY,
  REVIEWER_EVERY_SECONDS,
} from './queen-reviewer-card.gen'

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

  const intakeSpec = () => {
    let queue: number[] = []
    const busy = new Map<Pid, number>()
    const lastVisited = new Map<number, number>()
    let idle: Pid[] = []
    let wakeQueued = false

    const dispatch = () => {
      const free = Math.min(idle.length, reviewSlots(busy.size, queue.length))
      for (let k = 0; k < free; k++) {
        const issue = queue.shift()
        if (issue !== undefined && reviewing.has(issue)) continue
        const worker = idle.shift()
        if (issue === undefined || worker === undefined) return
        busy.set(worker, issue)
        lastVisited.set(issue, clock.now())
        sys.send(worker, { issue })
      }
    }
    const beat = (self: Pid) =>
      clock.after(REVIEWER_EVERY_SECONDS * 1000, () => {
        sys.post(self, M_HEARTBEAT_DUE)
      })

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
          idle.push(msg.worker)
        } else if (msg.kind === 'done') {
          if (!busy.has(msg.worker)) return
          busy.delete(msg.worker)
          idle.push(msg.worker)
          queue = queue.filter((n) => n !== msg.issue)
        } else if (msg.kind === 'DOWN') {
          const issue = busy.get(msg.pid)
          busy.delete(msg.pid)
          idle = idle.filter((p) => p !== msg.pid)
          // a crashed review has ended: its row goes back at once. A killed one
          // may still be running; its row waits for it to end (`reviewing`)
          if (issue !== undefined && !reviewing.has(issue)) queue.unshift(issue)
        } else {
          wakeQueued = false
          if (!(await deps.holdsLease())) return
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
        dispatch()
      },
    }
  }

  const intakeChild: Child = {
    name: 'reviewer-intake',
    start: (onExit, slot) => actorChild(sys, intakeSpec()).start(onExit, slot),
  }
  const reservedKeys = () =>
    [...reviewing.values()].filter((k): k is number => typeof k === 'number')
  const worker = (i: number): Child =>
    actorChild(sys, {
      name: `reviewer-worker-${i}`,
      turnMaxSeconds: REVIEW_ROW_SECONDS,
      init: (self: Pid) => {
        if (intake !== undefined) sys.send(intake, { kind: 'up', worker: self })
      },
      receive: async (msg: { issue: number }, self: Pid) => {
        reviewing.set(msg.issue, undefined)
        let round: Judged
        try {
          round = await deps.reviewOne(msg.issue, reservedKeys, (keyIndex) => {
            if (typeof keyIndex === 'number' && reviewing.has(msg.issue))
              reviewing.set(msg.issue, keyIndex)
          })
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
    })

  const tree = supervisor(
    sys,
    {
      name: 'reviewer-domain',
      strategy: STRAT_REST_FOR_ONE,
      maxRestarts: DOMAIN_MAX_RESTARTS,
      periodSeconds: DOMAIN_PERIOD_SECONDS,
    },
    [
      intakeChild,
      ...Array.from({ length: deps.workers ?? REVIEWER_CONCURRENCY }, (_, i) =>
        worker(i),
      ),
    ],
  )
  return {
    tree,
    /** A bus event: something finished and may want a review. */
    wake: () => {
      if (intake !== undefined) sys.send(intake, { kind: 'wake' })
    },
  }
}

/**
 * Start the reviewer as actors in this process: a root supervisor
 * (one_for_one, ROOT_MAX_RESTARTS in ROOT_PERIOD_SECONDS) over the reviewer
 * domain, woken by the bus on every queen/task.ended (EV_TASK_ENDED).
 */
export function startReviewerActors(
  pool: Pool,
  leaseName: string,
  review: ReviewFn,
  waiting: (pool: Pool) => Promise<number[]>,
): () => void {
  const sys = createActorSystem()
  const r = reviewerTree(sys, {
    ...reviewerDeps(pool, leaseName, review, waiting),
    onJudged: recordJudged,
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
    workers: REVIEWER_CONCURRENCY,
    turnMaxSeconds: REVIEW_ROW_SECONDS,
  })
  return () => {
    unread()
    root.stop()
    setReviewerRunning(false)
  }
}
