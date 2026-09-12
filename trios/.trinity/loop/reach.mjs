#!/usr/bin/env node
// How often does each step of the heal chain ACTUALLY RUN.
//
// WHY THIS EXISTS. `heal.mjs` has printed `heal complete:` at the end of every
// run it has ever made, followed by a token per step. A skipped step is named in
// that line - `judge-packet=skipped` - and naming is not announcing. One token
// among twenty-seven, in a log nobody reads unless something has already gone
// wrong.
//
// Measured 2026-09-12 over the 230 runs in the ledger since 2026-09-06:
//
//   land               1 of 230 runs      close-done         1 of 230
//   stale-escalations  1 of 230           author             1 of 230
//   verdict-audit      1 of 230           proven             1 of 230
//   judge-packet       0 of 230           brief-gate         2 of 230
//   exposure           2 of 230           forked-files       2 of 230
//   t27-parity         2 of 230
//
// Eleven steps at or below 1%. `verdict-audit` is the instrument that compares
// what the swarm CLAIMS against what it PUSHED. In the week it did not run, the
// swarm reported 439 of 439 dispatches finished and put nothing on the remote
// after 2026-09-05T17:30Z. The check that exists for precisely that failure was
// starved by its position in an array, and every single run said `complete`.
//
// A step is not slow because it is last. It is unreached because it is last, and
// last is a permanent condition under a fixed-order deadline. That is what this
// file measures: not duration, not failure - REACH.
//
// WHAT IT REFUSES TO DO. It does not read `state/step-reach.json`, which is the
// rotation cursor `heal.mjs` writes about itself. An instrument that grades a
// chain by reading the chain's own bookkeeping is the evaluator reading state
// written by the thing it evaluates, which is the mistake this project has now
// made eight times. The ledger is append-only and is written per run, so it is
// the independent record. If the two ever disagree, the ledger is right.
//
// Usage:
//   node reach.mjs                # every heal run in the ledger
//   node reach.mjs --since 2026-09-06
//   node reach.mjs --runs 50      # the newest N runs only
//
// EXIT CODE. Non-zero when any step has been reached in under `--floor` percent
// of runs (default 25). A non-zero exit here is the answer, not a malfunction.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const LEDGER = path.join(DIR, 'ledger.jsonl')

// ARGV IS READ INSIDE THE GUARD, NOT AT MODULE SCOPE. `selftest.mjs` has a case
// for this and it named seven files the moment this one was written; a tool that
// reads the command line merely by being imported cannot be tested by a harness
// that imports it, which is the same argument that put the `isMain` guard in
// `push-work.mjs`. Adding an eighth offender to a list a checker already prints
// would be using a known-failing gate as permission.
const argOf = (argv, flag, dflt) => {
  const i = argv.indexOf(flag)
  return i >= 0 && argv[i + 1] ? argv[i + 1] : dflt
}

// A step that STARTED counts as reached even if it was killed. `land` timing out
// 142 times is a different defect from `judge-packet` never being started, and
// collapsing them would hide both. Only `skipped` means never got its turn.
const NEVER_STARTED = new Set(['skipped'])

export function reachOf(runs) {
  const seen = new Map()
  for (const r of runs) {
    for (const s of r.results || []) {
      if (!seen.has(s.step)) seen.set(s.step, { step: s.step, runs: 0, reached: 0, byStatus: {} })
      const rec = seen.get(s.step)
      rec.runs += 1
      if (!NEVER_STARTED.has(s.status)) rec.reached += 1
      rec.byStatus[s.status] = (rec.byStatus[s.status] || 0) + 1
    }
  }
  return [...seen.values()].map((r) => ({
    ...r,
    // NULL, NOT ZERO. A step that appears in no run at all has no reach to
    // report; printing 0% would be a measurement of something never measured.
    percent: r.runs ? Math.round((100 * r.reached) / r.runs) : null,
  }))
}

export function readRuns({ since = null, runs = 0 } = {}) {
  if (!fs.existsSync(LEDGER)) return []
  const out = []
  for (const line of fs.readFileSync(LEDGER, 'utf8').split('\n')) {
    if (!line.trim()) continue
    let j
    try { j = JSON.parse(line) } catch { continue }
    if (j.kind !== 'heal' || !Array.isArray(j.results)) continue
    if (since && j.at < since) continue
    out.push(j)
  }
  return runs ? out.slice(-runs) : out
}

const isMain = process.argv[1] && process.argv[1].endsWith('/reach.mjs')
if (isMain) {
  const argv = process.argv.slice(2)
  const SINCE = argOf(argv, '--since', null)
  const RUNS = Number(argOf(argv, '--runs', 0)) || 0
  const FLOOR = Number(argOf(argv, '--floor', 25))

  const runs = readRuns({ since: SINCE, runs: RUNS })
  if (!runs.length) {
    console.log('no heal runs in the ledger' + (SINCE ? ` since ${SINCE}` : ''))
    console.log('  reach is unmeasured here, which is not the same as a chain that runs everything.')
    process.exit(0)
  }
  const rows = reachOf(runs)
  const span = `${runs[0].at.slice(0, 16)} .. ${runs[runs.length - 1].at.slice(0, 16)}`
  console.log(`heal step reach over ${runs.length} run(s)   ${span}\n`)
  console.log(`  ${'step'.padEnd(20)} ${'reach'.padStart(6)}   ${'ran'.padStart(5)} ${'timedout'.padStart(9)} ${'skipped'.padStart(8)} ${'FAILED'.padStart(7)}`)
  // Hungriest first: the point of the table is what is NOT running.
  rows.sort((a, b) => (a.percent ?? 0) - (b.percent ?? 0))
  for (const r of rows) {
    const b = r.byStatus
    const pct = r.percent === null ? '-' : `${r.percent}%`
    console.log(`  ${r.step.padEnd(20)} ${pct.padStart(6)}   ${String(r.reached).padStart(5)} ${String(b['timed out'] || 0).padStart(9)} ${String(b.skipped || 0).padStart(8)} ${String(b.FAILED || 0).padStart(7)}`)
  }
  const starved = rows.filter((r) => r.percent !== null && r.percent < FLOOR)
  console.log('')
  if (!starved.length) {
    console.log(`every step reached in at least ${FLOOR}% of runs.`)
    process.exit(0)
  }
  console.log(`${starved.length} step(s) reached in under ${FLOOR}% of runs:`)
  for (const r of starved) console.log(`  ${r.step.padEnd(20)} ${r.percent}%  (ran ${r.reached} of ${r.runs})`)
  console.log('')
  console.log('A step at the end of a phase is not slow. It is last, and under a')
  console.log('fixed-order deadline last is permanent. heal.mjs now orders its')
  console.log('reporting phase by hunger, so these should climb over the next rounds.')
  console.log('If one of them does not, it is being started and killed - read the')
  console.log('timedout column, and give that step its own timer rather than a place')
  console.log('in a queue it cannot finish inside.')
  process.exit(2)
}
