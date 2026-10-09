/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * LETTERS STILL IN A MAILBOX (gHashTag/t27 specs/queen/actors.t27 sections
 * 3-4, gHashTag/trios#1731). Two rules read what a mailbox holds before its
 * owner takes it:
 *   - demonitor with flush removes a DOWN already queued for that monitor
 *     (down_left_after_demonitor);
 *   - when a call's `after` fires, a reply or a DOWN already queued for its
 *     alias wins over the timeout (call_outcome).
 * Each is checked with telemetry off and on. With telemetry on, a sampled
 * letter waits in the mailbox wrapped with its arrival time
 * (telemetry.t27 section 4), and the rule must still see the letter.
 *
 * The call tests run on a hand clock: its timers fire when the test says,
 * synchronously, so a timeout can fire while a reply sits in the mailbox.
 */

import { describe, expect, it } from 'bun:test'
import {
  type Clock,
  createActorSystem,
  type Down,
  type Pid,
} from '../../src/api/services/queen-actors'
import {
  CALL_DOWN,
  CALL_REPLY,
  CALL_TIMEOUT,
  X_CRASH,
} from '../../src/api/services/queen-actors-card.gen'
import {
  type CallRequest,
  linksOf,
} from '../../src/api/services/queen-actors-links'
import { VirtualClock } from './queen-virtual-clock'

class HandClock implements Clock {
  private t = 0
  private timers: Array<{ at: number; fn: () => void; dead: boolean }> = []
  now = (): number => this.t
  after = (ms: number, fn: () => void): (() => void) => {
    const timer = { at: this.t + ms, fn, dead: false }
    this.timers.push(timer)
    return () => {
      timer.dead = true
    }
  }
  /** Move to `t` and fire every live timer due by then, synchronously. */
  fire(t: number): void {
    this.t = t
    const due = this.timers.filter((x) => !x.dead && x.at <= t)
    this.timers = this.timers.filter((x) => !x.dead && x.at > t)
    for (const x of due) x.fn()
  }
}

const settle = () => new Promise<void>((r) => setImmediate(r))
const MODES = [
  ['telemetry off', undefined],
  ['telemetry on', {}],
] as const

describe('demonitor with flush', () => {
  for (const [mode, telemetry] of MODES)
    it(`removes a DOWN already queued for that monitor (${mode})`, async () => {
      const clock = new VirtualClock()
      const sys = createActorSystem(clock, { slices: false, telemetry })
      const links = linksOf(sys)
      const got: unknown[] = []
      // the watcher is in a long turn, so the DOWN waits in its mailbox
      const watcher = sys.spawn<unknown>({
        name: 'watcher',
        receive: async (m) => {
          got.push(m)
          if (m === 'hold') await new Promise((r) => clock.after(10_000, r))
        },
      })
      const target = sys.spawn<string>({ name: 'target', receive: () => {} })
      sys.send(watcher, 'hold')
      await clock.runUntil(1)
      const ref = sys.monitor(watcher, target)
      sys.exit(target, X_CRASH)
      expect(
        (sys.mailbox(watcher) ?? []).length,
        'the DOWN is queued behind the long turn',
      ).toBe(1)
      links.demonitor(watcher, ref, true)
      expect((sys.mailbox(watcher) ?? []).length).toBe(0)
      await clock.runUntil(20_000)
      expect(got).toEqual(['hold'])
      expect(got.some((m) => (m as Down)?.kind === 'DOWN')).toBe(false)
      sys.telemetry?.close()
    })
})

describe('a call whose timeout fires', () => {
  for (const [mode, telemetry] of MODES) {
    it(`with the reply already queued for its alias, ends with the reply (${mode})`, async () => {
      const clock = new HandClock()
      const sys = createActorSystem(clock, { slices: false, telemetry })
      const links = linksOf(sys)
      let req: CallRequest | undefined
      const callee = sys.spawn<CallRequest>({
        name: 'callee',
        receive: (m) => {
          req = m
        },
      })
      const caller = sys.spawn<string>({ name: 'caller', receive: () => {} })
      const result = links.call<string>({ self: caller }, callee, 'q', 1000)
      await settle()
      expect(req?.body).toBe('q')
      // the answer lands in the alias's mailbox; before the alias takes it,
      // the call's `after` fires
      links.reply(req as CallRequest, 'answer', callee)
      clock.fire(1000)
      expect(await result).toEqual({
        outcome: CALL_REPLY,
        value: 'answer',
        reason: undefined,
      })
      sys.telemetry?.close()
    })

    it(`with the callee's DOWN already queued, ends as down with its reason (${mode})`, async () => {
      const clock = new HandClock()
      const sys = createActorSystem(clock, { slices: false, telemetry })
      const links = linksOf(sys)
      const callee = sys.spawn<CallRequest>({
        name: 'callee',
        receive: () => {},
      })
      const caller = sys.spawn<string>({ name: 'caller', receive: () => {} })
      const result = links.call<string>({ self: caller }, callee, 'q', 1000)
      await settle()
      sys.exit(callee, X_CRASH)
      clock.fire(1000)
      expect(await result).toEqual({
        outcome: CALL_DOWN,
        value: undefined,
        reason: X_CRASH,
      })
      sys.telemetry?.close()
    })
  }

  it('with nothing queued, ends as a timeout and the alias is gone', async () => {
    const clock = new HandClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    let req: CallRequest | undefined
    const callee = sys.spawn<CallRequest>({
      name: 'callee',
      receive: (m) => {
        req = m
      },
    })
    const caller: Pid = sys.spawn<string>({ name: 'caller', receive: () => {} })
    const result = links.call<string>({ self: caller }, callee, 'q', 1000)
    await settle()
    clock.fire(1000)
    expect((await result).outcome).toBe(CALL_TIMEOUT)
    // a late reply reaches nobody
    expect(links.reply(req as CallRequest, 'late', callee)).not.toBe(0)
    expect(links.stats.lateReplies).toBe(1)
  })
})
