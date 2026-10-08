/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A virtual clock for the actor tests and the benchmark (t27#7851). Timers
 * fire in time order. Between two timers every pending promise settles: one
 * macrotask lets the microtask queue drain. So hours of Queen time run in
 * milliseconds, and two runs over the same input see the same order.
 */

import type { Clock } from '../../src/api/services/queen-actors'

interface Timer {
  at: number
  seq: number
  fn: () => void
  dead: boolean
}

const settle = () => new Promise<void>((resolve) => setImmediate(resolve))

export class VirtualClock implements Clock {
  private t = 0
  private seq = 0
  private q: Timer[] = []

  now = (): number => this.t

  after = (ms: number, fn: () => void): (() => void) => {
    const timer: Timer = {
      at: this.t + Math.max(0, ms),
      seq: this.seq++,
      fn,
      dead: false,
    }
    let lo = 0
    let hi = this.q.length
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      const m = this.q[mid]
      if (m.at < timer.at || (m.at === timer.at && m.seq < timer.seq))
        lo = mid + 1
      else hi = mid
    }
    this.q.splice(lo, 0, timer)
    return () => {
      timer.dead = true
    }
  }

  /** Run every timer due up to `end`, settling promises after each. */
  async runUntil(end: number): Promise<void> {
    await settle()
    while (this.q.length > 0 && this.q[0].at <= end) {
      const timer = this.q.shift() as Timer
      if (timer.dead) continue
      this.t = timer.at
      timer.fn()
      await settle()
    }
    this.t = end
    await settle()
  }
}
