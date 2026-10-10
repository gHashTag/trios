/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE KEYED ACTORS' THREE GAPS, MEASURED (gHashTag/t27 specs/queen/
 * keyed_guard.t27, gHashTag/trios#1729 item 7, numbers on gHashTag/t27#7851).
 * Each table runs one seeded input several ways (queen-bee-sim.ts). The same
 * file runs on the commit before this change, so "before" and "after" are one
 * input through two runtimes. Which way wins is the output; the asserts only
 * hold what the change promises.
 *   1. Two dispatchers over one lease table: the claim's holder.
 *   2. The admission crashes three times in a lane-bound run.
 *   3. A call cycle among processes past slot 63, and what a call costs.
 *
 * Set QUEEN_BENCH_OUT to write the results as JSON as well.
 */

import { afterAll, beforeAll, describe, expect, it } from 'bun:test'
import { writeFileSync } from 'node:fs'
import {
  createActorSystem,
  type Pid,
} from '../../src/api/services/queen-actors'
import {
  type CallRequest,
  linksOf,
} from '../../src/api/services/queen-actors-links'
import { logger } from '../../src/lib/logger'
import { HOUR, type Options, simulate, workload } from './queen-bee-sim'
import { VirtualClock } from './queen-virtual-clock'

let level: Parameters<typeof logger.setLevel>[0] = 'info'
beforeAll(() => {
  level = (logger as unknown as { level: typeof level }).level ?? 'info'
  logger.setLevel('warn')
})
afterAll(() => logger.setLevel(level))

const FAULTS = { fail: 0.08, crash: 0.04, hang: 0.03 }
const out: Record<string, unknown> = {}
afterAll(() => {
  const file = process.env.QUEEN_BENCH_OUT
  if (file) writeFileSync(file, JSON.stringify(out, null, 2))
})

describe('1. two dispatchers over one lease table: the claim holder', () => {
  // the input of trios#1723's two-node run: 58/h for 4 h, 32 lanes
  const w = workload('two dispatcher nodes over one lease table', 1717, {
    perHour: 58,
    hours: 4,
    medianMinutes: 25,
    lanes: 32,
    ...FAULTS,
  })
  const ways: Array<[string, Options]> = [
    ['two nodes, pid holder', { nodes: 2 }],
    [
      'two nodes, process holder (negative control)',
      { nodes: 2, processHolder: true },
    ],
    [
      'one process name, pid alone (negative control)',
      { nodes: 2, oneName: true },
    ],
    [
      'one process name, boot + pid (production)',
      { nodes: 2, oneName: true, boots: true },
    ],
  ]
  it('only a holder that names the process boot and the pid keeps two dispatchers from starting one issue twice', async () => {
    const lines = [
      '| holder | done | duplicates | stand-downs | max running |',
      '|---|---|---|---|---|',
    ]
    const rows: Record<string, unknown> = {}
    for (const [name, opt] of ways) {
      const r = await simulate(w, 'actors-event', opt)
      rows[name] = r
      lines.push(
        `| ${name} | ${r.done} | ${r.duplicates} | ${r.standDowns} | ${r.maxRunning} |`,
      )
      if (name.includes('production') || name === 'two nodes, pid holder')
        expect(r.duplicates).toBe(0)
    }
    out.holders = rows
    console.log(
      `\n## ${w.name} (${w.arrivals.length} issues)\n${lines.join('\n')}`,
    )
  }, 900_000)
})

describe('2. the admission crashes three times in a lane-bound run', () => {
  const w = workload('lane-bound: 58/h, 25 min bees, 16 lanes, faults', 1713, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 16,
    ...FAULTS,
  })
  const crashes = [2 * HOUR, 4 * HOUR, 6 * HOUR]
  const ways: Array<[string, Options]> = [
    ['no crash', {}],
    ['3 crashes', { admissionCrashesAt: crashes }],
    [
      '3 crashes, lanes forgotten (negative control)',
      { admissionCrashesAt: crashes, forgetful: true },
    ],
  ]
  it('the lanes and the waiting issues come back with the admission; nothing starts twice or past the lanes', async () => {
    const lines = [
      '| run | crashes | done | /h | duplicates | max running | waitP50 | waitP95 | occupied | effective |',
      '|---|---|---|---|---|---|---|---|---|---|',
    ]
    const rows: Record<string, unknown> = {}
    for (const [name, opt] of ways) {
      const r = await simulate(w, 'actors-round', opt)
      rows[name] = r
      lines.push(
        `| ${name} | ${r.admissionCrashes} | ${r.done} | ${r.perHour} | ${r.duplicates} | ${r.maxRunning} | ${r.waitP50} | ${r.waitP95} | ${r.occupied} | ${r.effective} |`,
      )
      expect(r.duplicates).toBe(0)
      if (!opt.forgetful) expect(r.maxRunning).toBeLessThanOrEqual(w.lanes)
    }
    out.admission = rows
    console.log(
      `\n## ${w.name} (${w.arrivals.length} issues, crashes at 2, 4 and 6 h)\n${lines.join('\n')}`,
    )
  }, 900_000)
})

describe('3. a call cycle past slot 63, and what a call costs', () => {
  /** A calls B, B calls A back; `before` processes are started first. */
  const cycle = async (before: number) => {
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    for (let i = 0; i < before; i++) sys.spawn({ name: 'x', receive: () => {} })
    let back: { outcome: number; at: number } | undefined
    let a: Pid = 0n
    const b = sys.spawn<CallRequest>({
      name: 'b',
      receive: async (req, self) => {
        const t0 = clock.now()
        const r = await links.call({ self, serving: req }, a, 'back', 5000)
        back = { outcome: r.outcome, at: clock.now() - t0 }
        links.reply(req, 'done', self)
      },
    })
    a = sys.spawn<string>({ name: 'a', receive: () => {} })
    void links.call({ self: a }, b, 'go', 60_000)
    await clock.runUntil(60_000)
    return back
  }
  it('is refused at once at any slot', async () => {
    const low = await cycle(0)
    const high = await cycle(100)
    // the cost of one admitted call, round trip, on the host's clock
    const clock = new VirtualClock()
    const sys = createActorSystem(clock, { slices: false })
    const links = linksOf(sys)
    const echo = sys.spawn<CallRequest>({
      name: 'echo',
      receive: (req, self) => void links.reply(req, 1, self),
    })
    const caller = sys.spawn({ name: 'c', receive: () => {} })
    const N = 5000
    for (let i = 0; i < 500; i++) {
      const p = links.call({ self: caller }, echo, i, 1000)
      await clock.runUntil(clock.now())
      await p
    }
    const t0 = performance.now()
    for (let i = 0; i < N; i++) {
      const p = links.call({ self: caller }, echo, i, 1000)
      await clock.runUntil(clock.now())
      await p
    }
    const usPerCall = Math.round(((performance.now() - t0) * 1000) / N)
    out.cycle = { low, high, usPerCall }
    console.log(
      `\n## call cycle A -> B -> A\n| slots | outcome | answered after (virtual ms) |\n|---|---|---|\n| below 64 | ${low?.outcome} | ${low?.at} |\n| past 100 | ${high?.outcome} | ${high?.at} |\n\nan admitted call, round trip: ${usPerCall} us (${N} calls)`,
    )
    expect(low?.at).toBe(0)
    expect(high?.at).toBe(0)
  }, 300_000)
})
