/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * WHERE TELEMETRY DOES ITS WORK (gHashTag/t27 specs/queen/telemetry.t27,
 * gHashTag/trios#1729 item 19). Telemetry's answers are the card's; what this
 * file pins is how often the runtime pays for asking, so a later change
 * cannot move the work back onto every message without a test saying so:
 *   1. a chain of turns of one kind enters that kind's context once, not once
 *      a message, and the decision log still names the kind behind every
 *      call; a chain whose kind changes at every step enters it every step;
 *   2. once a window's records are taken, a message asks the telemetry card
 *      nothing, and the actors card no more than with telemetry off;
 *   3. a summary's window is folded from the running totals when its line is
 *      written: a turn writes one count;
 *   4. the bucket memo read at a turn's end answers as the card does, for a
 *      time with a fraction and one below zero too.
 */

import { afterEach, describe, expect, it, spyOn } from 'bun:test'
import {
  ACTORS_CARD,
  type Clock,
  createActorSystem,
  type Pid,
} from '../../src/api/services/queen-actors'
import {
  type DecisionRecord,
  TELEMETRY_CARD,
} from '../../src/api/services/queen-actors-telemetry'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import {
  DLOG_PER_FN_PER_WINDOW,
  MT_MAILBOX_TIME,
  NO_KIND,
} from '../../src/api/services/queen-telemetry-card.gen'
import { VirtualClock } from './queen-virtual-clock'

const undo: Array<() => void> = []
afterEach(() => {
  for (const u of undo.splice(0)) u()
})

/**
 * A ring of `n` actors passing `hops` messages under a virtual clock, slices
 * off: nothing in it reads real time. `kindOf` names each actor's kind.
 */
async function ring(
  n: number,
  hops: number,
  kindOf: (i: number) => string,
  telemetry: boolean,
  onRecord?: (r: DecisionRecord) => void,
) {
  const clock = new VirtualClock()
  const sys = createActorSystem(clock, {
    slices: false,
    ...(telemetry
      ? { telemetry: { cards: [ACTORS_CARD], onRecord, summary: () => {} } }
      : {}),
  })
  const tel = sys.telemetry
  if (tel) undo.push(() => tel.close())
  const pids: Pid[] = []
  let left = hops
  for (let i = 0; i < n; i++)
    pids.push(
      sys.spawn<number>({
        name: `r${i}`,
        kind: kindOf(i),
        receive: (hop, self) => {
          left--
          if (left > 0) sys.send(pids[(i + 1) % n], hop + 1, self)
        },
      }),
    )
  return {
    sys,
    run: async () => {
      sys.send(pids[0], 0)
      await clock.runUntil(1)
      expect(sys.stats.delivered).toBe(hops)
    },
  }
}

/** Every call into a card from now on, counted. */
function counted(file: string) {
  const card = loadCardWasm(file)
  const one = spyOn(card, 'call')
  const wide = spyOn(card, 'call64')
  undo.push(() => {
    one.mockRestore()
    wide.mockRestore()
  })
  return () => one.mock.calls.length + wide.mock.calls.length
}

describe("a kind's context", () => {
  it('a chain of turns of one kind enters it once, and the log names that kind for every call in it', async () => {
    const records: DecisionRecord[] = []
    const r = await ring(
      100,
      10_000,
      () => 'ring',
      true,
      (x) => records.push(x),
    )
    const tel = r.sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    const runs = spyOn(tel.als, 'run')
    undo.push(() => runs.mockRestore())
    await r.run()
    // the first step runs from outside any turn; every later one is sent
    // from a turn of the same kind
    expect(runs.mock.calls.length).toBe(1)
    const kinds = records.filter((x) => x.fn === 'deliver').map((x) => x.kind)
    expect(kinds).toEqual([
      NO_KIND,
      ...Array(DLOG_PER_FN_PER_WINDOW - 1).fill('ring'),
    ])
  })

  it('a chain whose kind changes at every step enters it at every step, and the log follows each change', async () => {
    const records: DecisionRecord[] = []
    const r = await ring(
      100,
      10_000,
      (i) => (i % 2 === 0 ? 'a' : 'b'),
      true,
      (x) => records.push(x),
    )
    const tel = r.sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    const runs = spyOn(tel.als, 'run')
    undo.push(() => runs.mockRestore())
    await r.run()
    expect(runs.mock.calls.length).toBe(10_000)
    const kinds = records.filter((x) => x.fn === 'deliver').map((x) => x.kind)
    expect(kinds).toEqual([
      NO_KIND,
      ...Array.from({ length: DLOG_PER_FN_PER_WINDOW - 1 }, (_, i) =>
        i % 2 === 0 ? 'a' : 'b',
      ),
    ])
  })
})

describe('the cards a message asks', () => {
  it('once the window is taken, a message asks the telemetry card nothing and the actors card no more than with telemetry off', async () => {
    // warm the bucket memo, which lives as long as the process
    await (await ring(100, 1_000, () => 'ring', true)).run()
    const telemetryCalls = counted(TELEMETRY_CARD)
    const actorsCalls = counted(ACTORS_CARD)
    const asked = async (hops: number, telemetry: boolean) => {
      const r = await ring(100, hops, () => 'ring', telemetry)
      const t0 = telemetryCalls()
      const a0 = actorsCalls()
      await r.run()
      return { telemetry: telemetryCalls() - t0, actors: actorsCalls() - a0 }
    }
    const small = await asked(2_000, true)
    const large = await asked(20_000, true)
    const off = await asked(20_000, false)
    // the sampling gates and the tap's records, once per window: ten times
    // the messages, the same calls, and fewer than one per message
    expect(large.telemetry).toBe(small.telemetry)
    expect(large.telemetry).toBeLessThan(2_000)
    expect(off.telemetry).toBe(0)
    expect(large.actors).toBe(off.actors)
  })
})

describe("a summary's window", () => {
  it('is the turns since the last line, folded from the totals when the line is written', async () => {
    const clock = new VirtualClock()
    const lines: Array<{ kinds: Record<string, Record<string, unknown>> }> = []
    const sys = createActorSystem(clock, {
      slices: false,
      telemetry: {
        summary: (l) => lines.push(l as (typeof lines)[number]),
      },
    })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    undo.push(() => tel.close())
    const pid = sys.spawn<number>({
      name: 'w-0',
      kind: 'w',
      receive: (ms) => new Promise<void>((r) => clock.after(ms, r)),
    })
    const k = tel.kind('w')
    const sum = (h: ArrayLike<number>) => Array.from(h).reduce((a, b) => a + b)
    // five turns of 1 s, then, after the period, three of 20 s
    for (let i = 0; i < 5; i++) sys.send(pid, 1_000)
    await clock.runUntil(301_000)
    for (let i = 0; i < 3; i++) sys.send(pid, 20_000)
    await clock.runUntil(700_000)
    // between two lines a turn writes its one count, and the window waits
    expect(sum(k.turnMs)).toBe(8)
    expect(sum(k.turnAtSummary)).toBe(5)
    sys.send(pid, 0)
    await clock.runUntil(700_001)
    expect(lines.length).toBe(2)
    expect(lines[0].kinds.w.turns).toBe(5)
    expect(lines[0].kinds.w.turnP50Ms).toEqual([512, 1024])
    expect(lines[1].kinds.w.turns).toBe(3)
    expect(lines[1].kinds.w.turnP50Ms).toEqual([16_384, 32_768])
    expect(lines[1].kinds.w.turnP95Ms).toEqual([16_384, 32_768])
    // the line took the totals as they stood; the turn that wrote it is the
    // next window's
    expect(sum(k.turnAtSummary)).toBe(8)
    expect(sum(k.turnMs)).toBe(9)
  })
})

describe('the bucket memo', () => {
  it('buckets a wait with a fraction, and one below zero, as the card does', async () => {
    // a wall clock set by hand: a message stamped at 10 ms is taken at 7 ms
    // (the clock was set back), another stamped at 20 ms is taken at 21.5 ms
    let t = 0
    const clock: Clock = { now: () => t, after: () => () => {} }
    const sys = createActorSystem(clock, { slices: false, telemetry: {} })
    const tel = sys.telemetry
    if (!tel) throw new Error('telemetry is on')
    undo.push(() => tel.close())
    const pid = sys.spawn<string>({ name: 'm-0', kind: 'm', receive: () => {} })
    const flush = () => new Promise<void>((r) => setImmediate(r))
    t = 10
    sys.send(pid, 'x')
    t = 7
    await flush()
    t = 20
    sys.send(pid, 'y')
    t = 21.5
    await flush()
    const card = loadCardWasm(TELEMETRY_CARD)
    const floorOf = (ms: number) =>
      card.call('bucket_floor_ms', card.call('bucket_of', ms))
    const snap = tel.snapshot() as unknown as {
      kinds: Record<string, Record<string, { buckets: number[][] }>>
    }
    expect(snap.kinds.m[MT_MAILBOX_TIME].buckets).toEqual([
      [floorOf(1.5), 1],
      [floorOf(-3), 1],
    ])
    expect(floorOf(1.5)).toBe(1)
  })
})
