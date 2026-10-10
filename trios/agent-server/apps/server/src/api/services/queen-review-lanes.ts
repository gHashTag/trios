/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE REVIEW LANE QUEUE (gHashTag/t27 specs/queen/review_lanes.t27; night
 * loop trios#1729 item 4, epic trios#1712).
 *
 * WHY. A review holds its model lane only during its model call. It spends
 * its first part (the witness, the criteria measurement) on no lane, and asks
 * for one at its model call. With none free it used to end there and answer
 * `wait`: the witness and the measurement thrown away, the row pushed back by
 * due_again. Measured on the reviewer benchmark with 3 free lanes: the pool
 * sized to the lanes (reviewer_sizing.t27) left them idle during every
 * review's first part and lost to the fixed 4 (360 against 429 of 660 rows),
 * and the fixed 4 paid with 427 lane-less `wait` visits.
 *
 * WHAT. One actor owns the queue: `review-lanes`. A review that finds no lane
 * free calls it and waits for its answer, as Discord's semaphore makes a
 * request wait on a count of requests in flight. The card decides:
 *   - whether the review may wait at all (queue_admits): one waiter for each
 *     review in a model call; past that it answers `wait` at once;
 *   - who goes first (grant_before): the oldest review;
 *   - when the oldest waiter tries again (head_tries): when a review gives back
 *     its lane, and at the heartbeat, because a bee frees its key silently;
 *   - how long a review may wait (wait_left_seconds): the call's own timeout,
 *     so the wait never outlasts the row's budget for its model call;
 *   - what the wait answers (lane_answer, gives_row_back): go on, give the row
 *     back as `wait` (never a reviewer miss), or stopped.
 * The owner does not own the lanes: the keys are the truth, and bees hold
 * them too. Let through, a review reads the keys again.
 *
 * THE POOL. A review spends part of its life on no lane, so the pool may
 * exceed its lanes by that share (queue_lanes, pool_target_queued). The share
 * is measured here, from what the owner sees: seconds from a review's start
 * to its ask for a lane, and seconds the lane was held (hold_percent). The
 * time a review spends in the queue is in neither sum: counting it would read
 * a long queue as a short hold and grow the pool that made the queue.
 *
 * Behind TRIOS_QUEEN_REVIEW_LANES=semaphore, actors reviewer only. Without it
 * a review with no lane answers `wait` as before.
 */

import {
  type ActorSystem,
  actorChild,
  type Child,
  type Pid,
} from './queen-actors'
import { CALL_REPLY, M_HEARTBEAT_DUE } from './queen-actors-card.gen'
import { type CallRequest, linksOf } from './queen-actors-links'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import {
  LANE_ASKED,
  LANE_BEAT,
  LANE_LEFT,
  LANE_RELEASED,
} from './queen-review-lanes-card.gen'
import { REVIEWER_EVERY_SECONDS } from './queen-reviewer-card.gen'

export const REVIEW_LANES_CARD = 'queen/review_lanes.wasm'

const card = () => loadCardWasm(REVIEW_LANES_CARD)
/** Whether a review with no lane may join the queue (queue_admits). */
export const queueAdmits = (waiting: number, holding: number): boolean =>
  card().call('queue_admits', u32(waiting), u32(holding)) !== 0
/** Whether ticket `a` goes before ticket `b` (grant_before). */
export const grantBefore = (a: number, b: number): boolean =>
  card().call('grant_before', u32(a), u32(b)) !== 0
/** Whether the oldest waiter tries again on this event (head_tries). */
export const headTries = (event: number): boolean =>
  card().call('head_tries', event) !== 0
/** Seconds a review `elapsedMs` into its row may still wait. */
export const waitLeftSeconds = (elapsedMs: number): number =>
  card().call('wait_left_seconds', u32(elapsedMs / 1000)) >>> 0
/** LANE_GO, LANE_WAIT_ROW or LANE_STOPPED, once a wait has ended. */
export const laneAnswer = (granted: boolean, stopped: boolean): number =>
  card().call('lane_answer', flag(granted), flag(stopped))
/** Whether the answer gives the row back as `wait`. */
export const givesRowBack = (answer: number): boolean =>
  card().call('gives_row_back', answer) !== 0
/** The share of a review's life it holds its lane, in percent. */
export const holdPercent = (beforeS: number, holdingS: number): number =>
  card().call('hold_percent', u32(beforeS), u32(holdingS)) >>> 0
/** Whether the two sums are halved now. */
export const samplesHalve = (beforeS: number, holdingS: number): boolean =>
  card().call('samples_halve', u32(beforeS), u32(holdingS)) !== 0
/** The lanes the pool is sized against when reviews may wait. */
export const queueLanes = (lanes: number, holdPct: number): number =>
  card().call('queue_lanes', u32(lanes), u32(holdPct)) >>> 0
/** reviewer_sizing.t27 pool_target over queue_lanes. */
export const poolTargetQueued = (
  lanes: number,
  holdPct: number,
  freeMb: number,
  mbPerReview: number,
  busy: number,
  queued: number,
): number =>
  card().call(
    'pool_target_queued',
    u32(lanes),
    u32(holdPct),
    u32(freeMb),
    u32(mbPerReview),
    u32(busy),
    u32(queued),
  ) >>> 0

/** TRIOS_QUEEN_REVIEW_LANES=semaphore turns the queue on. */
export function reviewLanesOn(
  raw = process.env.TRIOS_QUEEN_REVIEW_LANES,
): boolean {
  return (raw ?? 'off').toLowerCase() === 'semaphore'
}

/** One review's way through the queue. */
export interface LaneGate {
  /**
   * No lane was free at the model call: wait for one. LANE_GO: read the keys
   * again; LANE_WAIT_ROW: give the row back as `wait`; LANE_STOPPED: the
   * review's turn was stopped.
   */
  wait(): Promise<number>
  /** The review took a lane for a model call. */
  held(): void
  /** That model call ended: its lane is free. Safe to call twice. */
  released(): void
}

export interface LaneQueueStats {
  asked: number
  queued: number
  refused: number
  /** Waiters let through to try again. */
  granted: number
  /** Waiters that left: stopped, or their wait ran out. */
  left: number
  mostWaiting: number
  mostHolding: number
}

interface Ask {
  ticket: number
}
type OwnerMsg =
  | CallRequest<Ask>
  | { kind: 'held'; ticket: number; beforeMs: number }
  | { kind: 'released'; ticket: number }
  | { kind: 'left'; ticket: number }

/**
 * The queue's owner, as a child for the reviewer's domain supervisor, and the
 * gates reviews wait through. One per reviewer tree.
 */
export function reviewLanes(sys: ActorSystem) {
  const clock = sys.clock
  const links = linksOf(sys)
  let owner: Pid | undefined
  let nextTicket = 0
  // the hold share's two sums, in seconds (hold_percent). They outlive an
  // owner's restart: what was measured stays measured
  let beforeS = 0
  let holdingS = 0
  const stats: LaneQueueStats = {
    asked: 0,
    queued: 0,
    refused: 0,
    granted: 0,
    left: 0,
    mostWaiting: 0,
    mostHolding: 0,
  }

  const ownerSpec = () => {
    // reviews in a model call, by ticket; a restarted owner starts empty, and
    // a release it never saw held is ignored
    const holding = new Map<number, { at: number; beforeMs: number }>()
    let waiting: Array<{ ticket: number; req: CallRequest<Ask> }> = []

    const letHeadTry = (event: number, self: Pid) => {
      if (!headTries(event)) return
      // a waiter whose call ended (its wait ran out) has a dead alias
      waiting = waiting.filter((w) => sys.alive(w.req.alias))
      if (waiting.length === 0) return
      let head = 0
      for (let i = 1; i < waiting.length; i++)
        if (grantBefore(waiting[i].ticket, waiting[head].ticket)) head = i
      const [w] = waiting.splice(head, 1)
      stats.granted++
      links.reply(w.req, true, self)
    }
    const beat = (self: Pid) =>
      clock.after(REVIEWER_EVERY_SECONDS * 1000, () => {
        sys.post(self, M_HEARTBEAT_DUE)
      })

    return {
      name: 'review-lanes',
      kind: 'review-lanes',
      init: (self: Pid) => {
        owner = self
        beat(self)
      },
      control: (tag: number, self: Pid) => {
        if (tag !== M_HEARTBEAT_DUE) return
        letHeadTry(LANE_BEAT, self)
        beat(self)
      },
      receive: (msg: OwnerMsg, self: Pid) => {
        if (msg.kind === 'call') {
          stats.asked++
          waiting = waiting.filter((w) => sys.alive(w.req.alias))
          // a review let through that found its lane taken again asks with
          // its first ticket; it is not a second waiter
          waiting = waiting.filter((w) => w.ticket !== msg.body.ticket)
          if (!queueAdmits(waiting.length, holding.size)) {
            stats.refused++
            links.reply(msg, false, self)
            return
          }
          stats.queued++
          waiting.push({ ticket: msg.body.ticket, req: msg })
          stats.mostWaiting = Math.max(stats.mostWaiting, waiting.length)
          letHeadTry(LANE_ASKED, self)
        } else if (msg.kind === 'held') {
          holding.set(msg.ticket, { at: clock.now(), beforeMs: msg.beforeMs })
          stats.mostHolding = Math.max(stats.mostHolding, holding.size)
        } else if (msg.kind === 'released') {
          const h = holding.get(msg.ticket)
          holding.delete(msg.ticket)
          if (h) {
            beforeS += h.beforeMs / 1000
            holdingS += (clock.now() - h.at) / 1000
            if (samplesHalve(beforeS, holdingS)) {
              beforeS /= 2
              holdingS /= 2
            }
          }
          letHeadTry(LANE_RELEASED, self)
        } else if (msg.kind === 'left') {
          stats.left++
          waiting = waiting.filter((w) => w.ticket !== msg.ticket)
          letHeadTry(LANE_LEFT, self)
        }
      },
    }
  }

  const child: Child = {
    name: 'review-lanes',
    kind: 'review-lanes',
    start: (onExit, slot) => actorChild(sys, ownerSpec()).start(onExit, slot),
  }

  /**
   * A gate for one review, started now by worker `self`. `signal` is its
   * turn's, with turn stop only: a stopped review leaves the queue.
   */
  function gate(self: Pid, signal?: AbortSignal): LaneGate {
    // oldest first: the ticket is the review's start, not its ask
    const ticket = nextTicket
    nextTicket = (nextTicket + 1) >>> 0
    const startedAt = clock.now()
    let askedAt: number | undefined
    let measured = false
    let holds = false
    // sent from no one: a review whose worker was killed still gives back
    // what it holds, as a stopped review's `released` reaches the intake
    const tell = (msg: OwnerMsg) => {
      if (owner !== undefined) sys.send(owner, msg)
    }
    return {
      async wait() {
        if (signal?.aborted) return laneAnswer(false, true)
        const left = waitLeftSeconds(clock.now() - startedAt)
        // no time left, no owner, or a worker already dead (an abandoned
        // review cannot call: what a dead pid sends is dropped)
        if (left === 0 || owner === undefined || !sys.alive(self))
          return laneAnswer(false, false)
        askedAt ??= clock.now()
        let onStop = () => {}
        const stopped = new Promise<'stopped'>((resolve) => {
          onStop = () => resolve('stopped')
          signal?.addEventListener('abort', onStop, { once: true })
        })
        try {
          const r = await Promise.race([
            links.call<boolean>({ self }, owner, { ticket }, left * 1000),
            stopped,
          ])
          if (r === 'stopped') {
            tell({ kind: 'left', ticket })
            return laneAnswer(false, true)
          }
          const granted = r.outcome === CALL_REPLY && r.value === true
          // a wait that ran out, or an owner that went down, leaves the queue
          if (!granted && r.outcome !== CALL_REPLY)
            tell({ kind: 'left', ticket })
          return laneAnswer(granted, !!signal?.aborted)
        } finally {
          signal?.removeEventListener('abort', onStop)
        }
      },
      held() {
        if (holds) return
        holds = true
        // only the first hold measures the review's lane-less first part;
        // the queue time is in neither sum
        const beforeMs = measured ? 0 : (askedAt ?? clock.now()) - startedAt
        measured = true
        tell({ kind: 'held', ticket, beforeMs })
      },
      released() {
        if (!holds) return
        holds = false
        tell({ kind: 'released', ticket })
      },
    }
  }

  return {
    child,
    gate,
    /** The measured share of a review's life on its lane (hold_percent). */
    holdPercent: (): number => holdPercent(beforeS, holdingS),
    stats,
  }
}

export type ReviewLanes = ReturnType<typeof reviewLanes>
