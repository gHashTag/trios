/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * EDGES OF THE ACTOR RUNTIME that the lane tests did not reach
 * (gHashTag/trios#1731): a keyed actor's control lane, the hold of a
 * stopping key when it is full, an unlink, a trap turned off, a wait for a
 * pid already gone, a thread whose work fails inside or outside its call,
 * the turn's signal and its abort as code inside the turn reads them, an
 * exit listener that unsubscribed, the memory net's view of a node, and u64
 * into a card.
 */

import { describe, expect, it } from 'bun:test'
import {
  createActorSystem,
  type Pid,
} from '../../src/api/services/queen-actors'
import {
  D_DROPPED_FULL,
  D_QUEUED,
  M_CANCEL,
  NODE_TTL_SECONDS,
  X_CRASH,
  X_NOPROC,
  X_NORMAL,
} from '../../src/api/services/queen-actors-card.gen'
import { threadWork } from '../../src/api/services/queen-actors-isolate'
import { keyedActors } from '../../src/api/services/queen-actors-keyed'
import { type Exit, linksOf } from '../../src/api/services/queen-actors-links'
import { createMemoryNet } from '../../src/api/services/queen-actors-net'
import { u64 } from '../../src/api/services/queen-card-wasm'
import { KEY_HOLD_CAP } from '../../src/api/services/queen-keyed-card.gen'
import { abortError, turnSignal } from '../../src/api/services/queen-turn-stop'
import { VirtualClock } from './queen-virtual-clock'

const THROWS = new URL('./fixtures/queen-thread-throws.ts', import.meta.url)
  .href

describe('a keyed actor', () => {
  it('takes a control message, and the message counts as activity: it is not passivated while control keeps coming', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const tags: number[] = []
    const k = keyedActors<number, string>(sys, {
      name: 'k',
      idleCheckSeconds: 10,
      make: (key) => ({
        name: `k-${key}`,
        receive: () => {},
        control: (tag) => void tags.push(tag),
      }),
    })
    k.send(1, 'hello')
    await clock.runUntil(1)
    const pid = k.pidOf(1) as Pid
    // PASSIVATE_IDLE_SECONDS is 120: a control message every 60 s keeps the
    // actor's last activity young, so 10 minutes pass with no passivation
    for (let t = 60_000; t <= 600_000; t += 60_000) {
      await clock.runUntil(t)
      sys.post(pid, M_CANCEL)
    }
    await clock.runUntil(600_001)
    expect(tags.length).toBe(10)
    expect(tags.every((t) => t === M_CANCEL)).toBe(true)
    expect(k.pidOf(1)).toBe(pid)
    expect(k.stats.passivations).toBe(0)
    // and once the control stops, the idle actor is passivated
    await clock.runUntil(1_000_000)
    expect(k.stats.passivations).toBe(1)
    expect(k.pidOf(1)).toBeUndefined()
  })

  it('a send to a stopping key past KEY_HOLD_CAP held sends is refused and counted as a dead letter', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    const firsts: string[] = []
    const k = keyedActors<number, string | Exit>(sys, {
      name: 'k',
      shutdownMs: 60_000,
      make: () => ({
        name: 'slow',
        init: (self) => links.trapExit(self),
        receive: (m, self) => {
          if (typeof m === 'string') firsts.push(m)
          // asked to stop: finish in 30 s
          else clock.after(30_000, () => sys.exit(self, X_NORMAL))
        },
      }),
    })
    k.send(7, 'start')
    await clock.runUntil(1)
    k.passivate(7)
    await clock.runUntil(1000)
    for (let i = 0; i < KEY_HOLD_CAP; i++)
      expect(k.send(7, `held-${i}`)).toBe(D_QUEUED)
    const dead = sys.stats.deadLetters
    expect(k.send(7, 'one too many')).toBe(D_DROPPED_FULL)
    expect(k.stats.held).toBe(KEY_HOLD_CAP)
    expect(k.stats.refused).toBe(1)
    expect(sys.stats.deadLetters).toBe(dead + 1)
    // the held sends start the next incarnation, the refused one is gone
    await clock.runUntil(40_000)
    expect(firsts.length).toBe(1 + KEY_HOLD_CAP)
    expect(firsts.at(-1)).toBe(`held-${KEY_HOLD_CAP - 1}`)
    expect(firsts).not.toContain('one too many')
  })
})

describe('links', () => {
  it('after an unlink a death reaches neither side', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    const spawn = (name: string) =>
      sys.spawn<string>({ name, receive: () => {} })
    const a = spawn('a')
    const b = spawn('b')
    const c = spawn('c')
    links.link(a, b)
    links.link(a, c)
    links.unlink(b, a)
    expect(links.linked(a)).toEqual([c])
    expect(links.linked(b)).toEqual([])
    sys.exit(b, X_CRASH)
    await clock.runUntil(1)
    // a was unlinked from b: b's crash leaves it alive
    expect(sys.alive(a)).toBe(true)
    // the link that stayed still carries a death: c's crash takes a with it
    sys.exit(c, X_CRASH)
    await clock.runUntil(2)
    expect(sys.alive(a)).toBe(false)
  })
})

describe('a thread turn', () => {
  it('whose work throws outside its call fails with that error, and a stop afterwards does nothing more', async () => {
    const w = threadWork(THROWS, 'throwLater', 'thrown outside the call')
    await expect(w.result).rejects.toThrow('thrown outside the call')
    expect(() => w.stop()).not.toThrow()
  })
})

describe('the turn signal', () => {
  it('code inside a turn reads the signal the turn was given; outside a turn there is none', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false, turnStop: true })
    const seen: Array<[AbortSignal | undefined, AbortSignal]> = []
    const pid = sys.spawn<string>({
      name: 'reads',
      receive: async (_m, _self, _r, signal) => {
        await Promise.resolve()
        seen.push([turnSignal(), signal])
      },
    })
    sys.send(pid, 'x')
    sys.send(pid, 'y')
    await clock.runUntil(1)
    expect(seen.length).toBe(2)
    for (const [read, given] of seen) expect(read).toBe(given)
    expect(turnSignal()).toBeUndefined()
  })
})

describe('the system', () => {
  it('stops telling an exit listener once it unsubscribed', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const told: Pid[] = []
    const off = sys.onExit((pid) => void told.push(pid))
    const spawn = () => sys.spawn<string>({ name: 'p', receive: () => {} })
    const a = spawn()
    const b = spawn()
    sys.exit(a, X_CRASH)
    expect(off()).toBe(true)
    sys.exit(b, X_CRASH)
    expect(told).toEqual([a])
  })
})

describe('the memory net', () => {
  it('reads a crashed node as up until its lease lapses, then as down', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    net.link(1)
    expect(net.up(1)).toBe(true)
    expect(net.up(2)).toBe(false)
    net.crash(1)
    await clock.runUntil((NODE_TTL_SECONDS - 1) * 1000)
    expect(net.up(1)).toBe(true)
    await clock.runUntil((NODE_TTL_SECONDS + 1) * 1000)
    expect(net.up(1)).toBe(false)
  })
})

describe('links, trapping and waiting for an end', () => {
  it('a process that stops trapping dies of a linked crash again', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    const spawn = (name: string) =>
      sys.spawn<unknown>({ name, receive: () => {} })
    const a = spawn('a')
    const b = spawn('b')
    const c = spawn('c')
    links.link(a, b)
    links.link(a, c)
    links.trapExit(a)
    sys.exit(b, X_CRASH)
    await clock.runUntil(1)
    expect(sys.alive(a)).toBe(true)
    links.trapExit(a, false)
    expect(links.trapping(a)).toBe(false)
    sys.exit(c, X_CRASH)
    await clock.runUntil(2)
    expect(sys.alive(a)).toBe(false)
  })

  it('waiting for the end of a pid that already ended answers noproc at once', async () => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    const p = sys.spawn<unknown>({ name: 'p', receive: () => {} })
    sys.exit(p, X_CRASH)
    expect(await links.exited(p)).toBe(X_NOPROC)
  })
})

describe('a thread turn whose call fails', () => {
  it('fails with the error its thread reported', async () => {
    const w = threadWork(THROWS, 'noSuchFunction', null)
    await expect(w.result).rejects.toThrow(
      `${THROWS} exports no noSuchFunction`,
    )
  })
})

describe('the abort of a stopped turn', () => {
  it('is the reason itself when the reason is an error, and an AbortError naming it otherwise', () => {
    const own = new Error('the review was cancelled')
    const a = new AbortController()
    a.abort(own)
    expect(abortError(a.signal)).toBe(own)
    const b = new AbortController()
    b.abort('deploy drain')
    const e = abortError(b.signal)
    expect(e).toBeInstanceOf(DOMException)
    expect(e.name).toBe('AbortError')
    expect(e.message).toBe('deploy drain')
    // a signal aborted with no reason at all
    const bare = { reason: undefined } as AbortSignal
    expect(abortError(bare).message).toBe('the turn was stopped')
  })
})

describe('u64 into a card', () => {
  it('takes a finite number down to a whole non-negative BigInt, and anything else as 0', () => {
    expect(u64(3.7)).toBe(3n)
    expect(u64(-5)).toBe(0n)
    expect(u64(Number.NaN)).toBe(0n)
    expect(u64(Number.POSITIVE_INFINITY)).toBe(0n)
    expect(u64(2 ** 40)).toBe(2n ** 40n)
  })
})
