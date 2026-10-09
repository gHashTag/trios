/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BENCHMARK BEFORE THE SWAP (gHashTag/t27#7851, gHashTag/trios#1712 item
 * 7). The bee dispatch loop against the bee dispatcher as keyed actors, on the
 * same seeded input (queen-bee-sim.ts). The asserts check that every run
 * accounts for its input, and the one property the actors must hold: no
 * duplicate pickup. Which runtime wins is the output, not a gate.
 *
 * Set QUEEN_BENCH_OUT to write the results as JSON as well.
 */

import { describe, expect, it } from 'bun:test'
import { writeFileSync } from 'node:fs'
import {
  HOUR,
  type Result,
  type Runtime,
  simulate,
  type Workload,
  workload,
} from './queen-bee-sim'

const FAULTS = { fail: 0.08, crash: 0.04, hang: 0.03 }
const SCENARIOS: Workload[] = [
  workload('measured load: 58/h, 25 min bees, 70 lanes, faults', 1712, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 70,
    ...FAULTS,
  }),
  workload('lane-bound: 58/h, 25 min bees, 16 lanes, faults', 1713, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 16,
    ...FAULTS,
  }),
  workload('lane-bound, no faults: 58/h, 25 min, 16 lanes', 1714, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 16,
    fail: 0,
    crash: 0,
    hang: 0,
  }),
  workload('burst: 150 in 10 min + 58/h, 70 lanes, faults', 1715, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 70,
    burst: 150,
    ...FAULTS,
  }),
  workload('slow rounds (6 min, 2026-10-08): 58/h, 16 lanes, faults', 1716, {
    perHour: 58,
    hours: 8,
    medianMinutes: 25,
    lanes: 16,
    prepareSeconds: 360,
    ...FAULTS,
  }),
]
const RUNTIMES: Runtime[] = [
  'loop',
  'loop-cap60',
  'actors-round',
  'actors-event',
]

const COLS: Array<keyof Result> = [
  'runtime',
  'done',
  'perHour',
  'duplicates',
  'waitP50',
  'waitP95',
  'readyToStartP95',
  'crashRecoveryP50',
  'crashRecoveryMax',
  'hangRecoveryP50',
  'failRetryP50',
  'effective',
  'occupied',
  'nominal',
  'gaveUp',
]

describe('the bee dispatcher: loop against keyed actors, same input', () => {
  it('runs every scenario three ways; the actors pick up no issue twice', async () => {
    const results: Result[] = []
    for (const w of SCENARIOS)
      for (const runtime of RUNTIMES) {
        const r = await simulate(w, runtime)
        expect(r.arrived).toBe(w.arrivals.length)
        expect(r.done).toBeLessThanOrEqual(r.arrived)
        if (runtime.startsWith('actors')) expect(r.duplicates).toBe(0)
        results.push(r)
      }
    const lines: string[] = []
    for (const w of SCENARIOS) {
      lines.push(
        `\n## ${w.name} (${w.arrivals.length} issues, horizon ${w.horizonMs / HOUR} h)`,
      )
      lines.push(`| ${COLS.join(' | ')} |`)
      lines.push(`|${COLS.map(() => '---').join('|')}|`)
      for (const r of results.filter((x) => x.scenario === w.name))
        lines.push(`| ${COLS.map((c) => String(r[c])).join(' | ')} |`)
    }
    console.log(lines.join('\n'))
    const out = process.env.QUEEN_BENCH_OUT
    if (out) writeFileSync(out, JSON.stringify(results, null, 2))
  }, 600_000)
})

describe('the claim holder is the incarnation (keyed.t27 claim_is_mine)', () => {
  const w = workload('two dispatcher nodes over one lease table', 1717, {
    perHour: 58,
    hours: 4,
    medianMinutes: 25,
    lanes: 32,
    ...FAULTS,
  })
  it('two nodes that both start an actor for an issue: one claim lands, no duplicate', async () => {
    const r = await simulate(w, 'actors-event', { nodes: 2 })
    expect(r.duplicates).toBe(0)
    expect(r.standDowns).toBeGreaterThan(0)
    console.log(
      `\n## two nodes, pid holder: ${r.done} done, ${r.duplicates} duplicates, ${r.standDowns} stand-downs`,
    )
  }, 600_000)
  it('negative control: with the process as the holder, the second claim "renews" and a second bee starts', async () => {
    const r = await simulate(w, 'actors-event', {
      nodes: 2,
      processHolder: true,
    })
    expect(r.duplicates).toBeGreaterThan(0)
    console.log(
      `\n## two nodes, process holder (negative control): ${r.done} done, ${r.duplicates} duplicates`,
    )
  }, 600_000)
})
