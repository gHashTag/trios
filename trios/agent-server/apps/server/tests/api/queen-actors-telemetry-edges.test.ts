/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * TELEMETRY AT ITS EDGES (gHashTag/t27 specs/queen/telemetry.t27,
 * gHashTag/trios#1731): the top list's order and a busy entry's turn age, a
 * turn in the last, open bucket, the summary with no sink of its own, a
 * process that dies with its depth alarm on, u64 decisions as the endpoint
 * shows them, a record its card cannot take, and the endpoint with
 * telemetry off.
 */

import { afterEach, describe, expect, it, spyOn } from 'bun:test'
import { createQueenActorsRoute } from '../../src/api/routes/queen-actors-metrics'
import {
  ACTORS_CARD,
  createActorSystem,
  type Pid,
  slotOf,
} from '../../src/api/services/queen-actors'
import {
  MAILBOX_CAP,
  X_CRASH,
} from '../../src/api/services/queen-actors-card.gen'
import {
  createActorTelemetry,
  replayDecisions,
  TELEMETRY_CARD,
} from '../../src/api/services/queen-actors-telemetry'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import {
  DEPTH_ON_PERCENT,
  EV_DEPTH_OK,
  HIST_BUCKETS,
  MT_LIVE,
  MT_MAILBOX_DEPTH,
  MT_TURN_TIME,
  MT_YIELDS,
  NO_KIND,
  RV_MISMATCH,
  RV_PASS,
  SUMMARY_EVERY_SECONDS,
  TEL_EVENT_NAMES,
} from '../../src/api/services/queen-telemetry-card.gen'
import { logger } from '../../src/lib/logger'
import { VirtualClock } from './queen-virtual-clock'

interface Snap {
  kinds: Record<string, Record<string, unknown>>
  top: Array<{
    pid: string
    kind: string
    depth: number
    busy: boolean
    turnAgeMs: number
  }>
}

const closers: Array<() => void> = []
afterEach(() => {
  for (const c of closers.splice(0)) c()
})
function system(clock: VirtualClock, cards?: string[]) {
  const sys = createActorSystem(clock, {
    slices: false,
    telemetry: cards ? { cards } : {},
  })
  const tel = sys.telemetry
  if (!tel) throw new Error('telemetry is on')
  closers.push(() => tel.close())
  return { sys, tel }
}
/** An actor whose first message holds its turn for `ms`. */
function slow(
  sys: ReturnType<typeof system>['sys'],
  clock: VirtualClock,
  name: string,
  ms: number,
  turnMaxSeconds = 0,
) {
  return sys.spawn<string>({
    name,
    turnMaxSeconds,
    receive: async (m) => {
      if (m === 'hold') await new Promise((r) => clock.after(ms, r))
    },
  })
}

describe('the top list', () => {
  it('ranks the deeper mailbox first, breaks a tie by the lower slot, and gives a busy entry the age of its turn', async () => {
    const clock = new VirtualClock()
    const { sys, tel } = system(clock)
    const a = slow(sys, clock, 'a', 60_000)
    const b = slow(sys, clock, 'b', 60_000)
    const c = slow(sys, clock, 'c', 60_000)
    expect(slotOf(a) < slotOf(c)).toBe(true)
    for (const p of [a, b, c]) sys.send(p, 'hold')
    await clock.runUntil(1)
    const queue = (p: Pid, n: number) => {
      for (let i = 0; i < n; i++) sys.send(p, 'more')
    }
    queue(a, 3)
    queue(b, 5)
    queue(c, 3)
    await clock.runUntil(5001)
    const top = (tel.snapshot() as unknown as Snap).top
    expect(top.map((t) => [t.pid, t.depth])).toEqual([
      [String(b), 5],
      [String(a), 3],
      [String(c), 3],
    ])
    expect(top.every((t) => t.busy)).toBe(true)
    // the turns began at 0, when "hold" arrived
    expect(top[0].turnAgeMs).toBe(5001)
  })
})

describe('the turn-time histogram', () => {
  it('puts a turn past the last bucket floor in the last bucket, open above', async () => {
    const card = loadCardWasm(TELEMETRY_CARD)
    const lastFloor = card.call('bucket_floor_ms', HIST_BUCKETS - 1)
    const clock = new VirtualClock()
    const { sys, tel } = system(clock)
    const turnMs = lastFloor + 60_000
    const p = slow(sys, clock, 'long', turnMs, Math.ceil(turnMs / 1000) + 600)
    sys.send(p, 'hold')
    await clock.runUntil(turnMs + 1000)
    const hist = (tel.snapshot() as unknown as Snap).kinds.long[
      MT_TURN_TIME
    ] as { count: number; p50: unknown; p95: unknown }
    expect(hist.count).toBe(1)
    expect(hist.p50).toEqual([lastFloor, null])
    expect(hist.p95).toEqual([lastFloor, null])
  })
})

describe('the summary', () => {
  it('with no sink of its own goes to the server log once SUMMARY_EVERY_SECONDS have passed', async () => {
    const info = spyOn(logger, 'info').mockImplementation(() => {})
    closers.push(() => info.mockRestore())
    const clock = new VirtualClock()
    const { sys } = system(clock)
    const p = sys.spawn<string>({ name: 'beats', receive: () => {} })
    const said = () =>
      info.mock.calls.filter((c) => c[0] === 'Queen actors measured')
    sys.send(p, 'x')
    await clock.runUntil((SUMMARY_EVERY_SECONDS - 10) * 1000)
    sys.send(p, 'x')
    await clock.runUntil((SUMMARY_EVERY_SECONDS - 10) * 1000 + 1)
    expect(said().length).toBe(0)
    await clock.runUntil((SUMMARY_EVERY_SECONDS + 1) * 1000)
    sys.send(p, 'x')
    await clock.runUntil((SUMMARY_EVERY_SECONDS + 1) * 1000 + 1)
    expect(said().length).toBe(1)
    const line = said()[0][1] as { kinds: Record<string, { sent: number }> }
    expect(line.kinds.beats.sent).toBeGreaterThan(0)
  })
})

describe('the depth alarm', () => {
  it('of a process that dies with it on is cleared, and the clearing is an event', async () => {
    const clock = new VirtualClock()
    const { sys, tel } = system(clock)
    const p = slow(sys, clock, 'flooded', 60_000)
    sys.send(p, 'hold')
    await clock.runUntil(1)
    const high = Math.ceil((MAILBOX_CAP * DEPTH_ON_PERCENT) / 100)
    for (let i = 0; i < high; i++) sys.send(p, 'more')
    let k = (tel.snapshot() as unknown as Snap).kinds.flooded
    expect(
      (k[MT_MAILBOX_DEPTH] as { alarmsOn: number }).alarmsOn,
      'the alarm is on',
    ).toBe(1)
    sys.exit(p, X_CRASH)
    k = (tel.snapshot() as unknown as Snap).kinds.flooded
    expect((k[MT_MAILBOX_DEPTH] as { alarmsOn: number }).alarmsOn).toBe(0)
    expect(
      (k.events as Record<string, number>)[TEL_EVENT_NAMES[EV_DEPTH_OK]],
    ).toBe(1)
  })
})

describe('the decision log', () => {
  it('shows a u64 card call with its numbers as strings, and those records replay', async () => {
    const clock = new VirtualClock()
    const { sys, tel } = system(clock, [ACTORS_CARD])
    const p = sys.spawn<string>({ name: 'p', receive: () => {} })
    const slot = slotOf(p)
    const shown = tel
      .decisions()
      .filter((r) => r.card === ACTORS_CARD && r.fn === 'slot_of')
    expect(shown.length).toBeGreaterThan(0)
    const mine = shown.find((r) => (r.args as unknown[])[0] === String(p))
    expect(mine?.result).toBe(String(slot))
    expect(replayDecisions(shown).verdict).toBe(RV_PASS)
  })

  it('a record whose arguments its card cannot take is a mismatch in the replay, not a crash of it', () => {
    // slot_of takes a u64: a plain number cannot cross into it
    const r = replayDecisions([
      {
        card: ACTORS_CARD,
        fn: 'slot_of',
        args: [5],
        result: '5',
        kind: '-',
        at: 0,
        n: 0,
      },
    ])
    expect(r.mismatches).toBe(1)
    expect(r.verdict).toBe(RV_MISMATCH)
    expect(String(r.firstBad?.bad)).toMatch(/^replayed threw /)
  })
})

describe('a telemetry no system watches yet', () => {
  it('reports no processes, no yields and an empty top list', () => {
    const tel = createActorTelemetry(new VirtualClock(), {})
    closers.push(() => tel.close())
    const s = tel.snapshot() as unknown as Snap & {
      kinds: Record<string, unknown>
      [k: string]: unknown
    }
    expect(s.top).toEqual([])
    // only the kind for work outside any actor, and nothing live in it
    expect(Object.keys(s.kinds)).toEqual([NO_KIND])
    expect(s.kinds[NO_KIND][MT_LIVE]).toBe(0)
    expect(s[MT_YIELDS]).toBe(0)
  })
})

describe('the endpoint with telemetry off', () => {
  it('answers the decisions read with the flag that turns it on', async () => {
    const app = createQueenActorsRoute()
    const res = await app.request('/decisions')
    expect(res.status).toBe(200)
    expect(res.headers.get('Cache-Control')).toBe('no-store')
    expect(await res.json()).toEqual({
      enabled: false,
      flag: 'TRIOS_QUEEN_ACTORS_TELEMETRY',
    })
  })
})
