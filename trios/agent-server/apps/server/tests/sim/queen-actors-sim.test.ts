/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE SIMULATION GATE (gHashTag/t27 specs/queen/simulation.t27, trios#1712
 * item 5).
 *
 * Each of the card's SEEDS_PER_RUN seeds plays STEPS_PER_RUN steps of the
 * real actor runtime - the reviewer actors and a pool on two more nodes - with
 * the card's fault mix, and runs RUNS_PER_SEED times. The gate fails when:
 *   - any invariant breaks in any run (the seed, the step and the minimal log
 *     tail are printed, with the command that replays it);
 *   - two runs of one seed log differently (the first step they part at is
 *     printed, both sides);
 *   - the seeds together never reach one of the card's rare states.
 *
 * Replay one seed:  SIM_SEED=<seed> bun test tests/sim/queen-actors-sim.test.ts
 * A soak:           SIM_SEEDS=<count> SIM_BASE=<base> bun test tests/sim/...
 * Timings as JSON:  SIM_OUT=<file>
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { createActorSystem } from '../../src/api/services/queen-actors'
import { X_SHUTDOWN } from '../../src/api/services/queen-actors-card.gen'
import * as S from '../../src/api/services/queen-simulation-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from '../api/queen-virtual-clock'
import { pgStore } from './queen-sim-pg-store'
import {
  INVARIANT_NAMES,
  mustReach,
  RARE_NAMES,
  type RunResult,
  report,
  runSeed,
  SimWorld,
} from './queen-sim-world'

const FILE = 'tests/sim/queen-actors-sim.test.ts'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

describe('the vendored simulation card is the one PIN names', () => {
  it('simulation.t27 and simulation.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/simulation.t27 sha256 ${sha('queen/simulation.t27')}`,
    )
    expect(pin).toContain(
      `queen/simulation.wasm sha256 ${sha('queen/simulation.wasm')}`,
    )
  })
})

/** Where two runs of one seed part, both sides of that step. */
async function divergence(seed: number, a: RunResult, b: RunResult) {
  let k = 0
  while (k < a.stepHashes.length && a.stepHashes[k] === b.stepHashes[k]) k++
  // the hashes past STEPS_PER_RUN are the quiet tail's, one a virtual minute
  const at = Math.min(k, S.STEPS_PER_RUN)
  const where =
    at === S.STEPS_PER_RUN
      ? `minute ${k - S.STEPS_PER_RUN} of the quiet tail`
      : `step ${at}`
  // the two runs' own last lines, matched by line number, where they first differ
  const num = (l: string) => Number(/^#(\d+)/.exec(l)?.[1] ?? -1)
  const bAt = new Map(b.lines.map((l) => [num(l), l]))
  let own = a.lines.findIndex((l) => bAt.has(num(l)) && bAt.get(num(l)) !== l)
  if (own < 0) own = a.lines.length
  const bOwn = b.lines.findIndex((l) => num(l) === num(a.lines[own] ?? ''))
  const one = await new SimWorld(seed, { captureStep: at }).run()
  const two = await new SimWorld(seed, { captureStep: at }).run()
  let line = 0
  while (
    line < Math.max(one.captured.length, two.captured.length) &&
    one.captured[line] === two.captured[line]
  )
    line++
  const side = (r: RunResult) =>
    r.captured.slice(Math.max(0, line - 3), line + 3).map((l) => `    ${l}`)
  return [
    `SIMULATION FAILED: ${INVARIANT_NAMES[S.INV_RUNS_DIFFER]} (invariant ${S.INV_RUNS_DIFFER})`,
    `  seed ${seed}: the runs part at ${where} (hashes: ${a.stepHashes.length} and ${b.stepHashes.length}; violations: ${a.violation?.inv ?? 'none'} and ${b.violation?.inv ?? 'none'})`,
    ...(b.violation ? [report(b, FILE)] : []),
    line < Math.max(one.captured.length, two.captured.length)
      ? `  replayed twice, they part again at line ${line} of it:`
      : '  replayed twice, that step logged the same: the difference came from outside the seed',
    `  replay: SIM_SEED=${seed} bun test ${FILE}`,
    '  first replay:',
    ...side(one),
    '  second replay:',
    ...side(two),
    `  the gate's own two runs, at line ${own} of their last ${a.lines.length}:`,
    ...a.lines
      .slice(Math.max(0, own - 2), own + 3)
      .map((l) => `    first:  ${l}`),
    ...b.lines
      .slice(
        Math.max(0, (bOwn < 0 ? own : bOwn) - 2),
        (bOwn < 0 ? own : bOwn) + 3,
      )
      .map((l) => `    second: ${l}`),
    `  counts, first run:  ${JSON.stringify(a.counts)}`,
    `  counts, second run: ${JSON.stringify(b.counts)}`,
    `  after the steps, quiescing: first run ${a.afterTail.length} card calls (${a.counts.quiesceRounds} rounds), second ${b.afterTail.length} (${b.counts.quiesceRounds} rounds)`,
    ...a.afterTail.slice(0, 12).map((l) => `    first:  ${l}`),
    ...b.afterTail.slice(0, 12).map((l) => `    second: ${l}`),
  ].join('\n')
}

function seedsOfThisRun(): number[] {
  if (process.env.SIM_SEED) return [Number(process.env.SIM_SEED) >>> 0]
  const count = Number(process.env.SIM_SEEDS || S.SEEDS_PER_RUN)
  const base = Number(process.env.SIM_BASE || S.GATE_BASE_SEED) >>> 0
  return Array.from({ length: count }, (_, i) => runSeed(base, i))
}

function summary(lines: string[]) {
  const out = process.env.GITHUB_STEP_SUMMARY
  if (!out) return
  appendFileSync(
    out,
    [
      '## Actor simulation gate failed',
      '',
      `commit ${process.env.GITHUB_SHA ?? '(local)'}`,
      '',
      '```',
      ...lines,
      '```',
      '',
    ].join('\n'),
  )
}

interface SeedOutcome {
  failures: string[]
  rare: Set<number>
  runs: number[]
  steps: number
}

/** One seed, RUNS_PER_SEED times: its violation, and whether its runs agree. */
async function checkSeed(seed: number): Promise<SeedOutcome> {
  const runs: RunResult[] = []
  for (let k = 0; k < S.RUNS_PER_SEED; k++)
    runs.push(await new SimWorld(seed).run())
  const [a] = runs
  const failures: string[] = []
  if (a.violation) failures.push(report(a, FILE))
  for (const b of runs.slice(1))
    if (
      b.stepHashes.length !== a.stepHashes.length ||
      b.stepHashes.some((h, i) => h !== a.stepHashes[i])
    )
      failures.push(await divergence(seed, a, b))
  return {
    failures,
    rare: a.rare,
    runs: runs.map((r) => Math.round(r.ms)),
    steps: a.steps,
  }
}

/** The rare states no seed reached, over the whole gate run. */
function missed(reached: Set<number>, seeds: number): string[] {
  const out: string[] = []
  for (let state = 0; state < S.RARE_KINDS; state++)
    if (mustReach(state) && !reached.has(state))
      out.push(
        `SIMULATION FAILED: ${INVARIANT_NAMES[S.INV_RARE_STATE_MISSED]} (invariant ${S.INV_RARE_STATE_MISSED})\n  no seed of ${seeds} reached: ${RARE_NAMES[state]}`,
      )
  return out
}

describe('the simulation gate (simulation.t27)', () => {
  it('every seed runs twice and logs the same, no invariant breaks, and together the seeds reach every rare state', async () => {
    const seeds = seedsOfThisRun()
    const failures: string[] = []
    const reached = new Set<number>()
    const timing: Array<{ seed: number; runs: number[]; steps: number }> = []
    const t0 = performance.now()
    for (const seed of seeds) {
      const o = await checkSeed(seed)
      failures.push(...o.failures)
      for (const r of o.rare) reached.add(r)
      timing.push({ seed, runs: o.runs, steps: o.steps })
    }
    const totalMs = Math.round(performance.now() - t0)
    // a replay of one seed is not asked to reach every rare state
    if (!process.env.SIM_SEED) failures.push(...missed(reached, seeds.length))
    const runMs = timing.flatMap((t) => t.runs).sort((x, y) => x - y)
    console.log(
      [
        `\n## actor simulation: ${seeds.length} seeds x ${S.RUNS_PER_SEED} runs x ${S.STEPS_PER_RUN} steps`,
        `total ${totalMs} ms; per run min ${runMs[0]} ms, p50 ${runMs[Math.floor(runMs.length / 2)]} ms, max ${runMs[runMs.length - 1]} ms`,
        `rare states reached: ${[...reached]
          .sort()
          .map((s) => RARE_NAMES[s])
          .join('; ')}`,
        ...timing.map(
          (t) =>
            `seed ${t.seed}: ${t.steps} steps, runs ${t.runs.join(' / ')} ms`,
        ),
      ].join('\n'),
    )
    if (process.env.SIM_OUT)
      writeFileSync(
        process.env.SIM_OUT,
        JSON.stringify({ totalMs, timing, reached: [...reached] }, null, 2),
      )
    if (failures.length > 0) {
      console.error(failures.join('\n\n'))
      summary(failures)
    }
    expect(failures).toEqual([])
  }, 900_000)
})

describe('the gate has teeth', () => {
  it('re-creates the go-live hot loop (no wait backoff) and fails it from the first gate seed', async () => {
    const seed = runSeed(S.GATE_BASE_SEED, 0)
    const r = await new SimWorld(seed, { noWaitBackoff: true }).run()
    console.log(`\n## the go-live defect, re-created\n${report(r, FILE)}`)
    expect(r.violation?.inv).toBe(S.INV_HOT_LOOP)
  }, 120_000)

  it('a process stopped between taking its message and running the turn runs no turn (the defect the gate found)', async () => {
    // seed 3600507402 found it: a pool supervisor gave up and stopped a
    // watcher whose DOWN turn was already taken; the turn ran for a dead pid
    const clock = new VirtualClock()
    const sys = createActorSystem(clock)
    let ran = 0
    const pid = sys.spawn<number>({ name: 'p', receive: () => void ran++ })
    sys.send(pid, 1)
    // the scheduler takes the message in the next microtask; stop it after
    await Promise.resolve()
    sys.exit(pid, X_SHUTDOWN, true)
    await clock.runUntil(1000)
    expect(sys.alive(pid)).toBe(false)
    expect(ran).toBe(0)
  })
})

/**
 * KNOWN DEFECTS of the store link (trios#1712 item 6, read out of the code by
 * the competitor study and not tested until now). Both reproduce from a seed
 * on the real createPgLink over the simulated store. Each test asserts that
 * its defect STILL reproduces: when item 6 lands, it fails, and that is the
 * signal to turn it into a gate (expect no violation) - the PgLink world then
 * joins the gate's seeds.
 */
describe('known defects of the PgLink, reproduced from a seed (trios#1712 item 6)', () => {
  it('a node that comes back reissues its pids while the store still holds mail for the old ones', async () => {
    // the other defect is looked past, so this run reaches this one
    const seed = runSeed(S.GATE_BASE_SEED, 0)
    const r = await new SimWorld(seed, {
      transport: pgStore,
      known: [S.INV_TAKEN_NOT_HANDLED],
    }).run()
    console.log(
      `\n## PgLink defect: pids reissued after a restart\n${report(r, FILE)}`,
    )
    expect(r.violation?.inv).toBe(S.INV_PID_REUSED)
  }, 300_000)

  it('mail is deleted before it is handled, so a lost answer to the DELETE loses it, counted nowhere', async () => {
    // pid reuse, and the second incarnation of a child it causes, are looked past
    const seed = runSeed(S.GATE_BASE_SEED, 1)
    const r = await new SimWorld(seed, {
      transport: pgStore,
      known: [S.INV_PID_REUSED, S.INV_RESTART_WITHOUT_EXIT],
    }).run()
    console.log(
      `\n## PgLink defect: mail deleted before it is handled\n${report(r, FILE)}`,
    )
    expect(r.violation?.inv).toBe(S.INV_TAKEN_NOT_HANDLED)
  }, 300_000)
})
