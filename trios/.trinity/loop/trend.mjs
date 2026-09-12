#!/usr/bin/env node
// A level is not a rate - for the review queue as much as for the disk.
//
// The reaper learned this the expensive way: a threshold of 80% looked like
// margin until the volume was measured climbing fifteen points an hour, which
// made it fifty-five minutes of warning. The same blindness applies to every
// counter on the dashboard. `claimed` at 15 and falling is a fence coming down;
// `claimed` at 15 and climbing is a fence being rebuilt, and the number alone
// cannot tell them apart.
//
// Everything here comes from `snapshot` lines the loop already appends on every
// render. Nothing new is measured; what was already recorded is finally read.
//
// Usage:
//   node trend.mjs            # the last 6 hours
//   node trend.mjs 2          # the last 2 hours

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { skipCount } from './sense.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const LEDGER = path.join(DIR, 'ledger.jsonl')
const isMain = process.argv[1] && process.argv[1].endsWith('/trend.mjs')

// THE THIRD READER OF A SHAPE THAT CHANGED, AND THE ONE NOBODY TOLD.
//
// `skipSummary` served plain integers until 2026-09-12T16:39Z and now serves
// `{count, issues[], more}`. sense.mjs learned the new shape and why.mjs was
// fixed to read through it; snapshot.mjs keeps its own value-level copy. This
// file kept `(r.skips || {}).claimed` and then dropped every sample on
// `typeof v === 'number'`, so 96 measured snapshots became zero usable points
// and all three skip series reported "too few points" - a phrase that reads as
// "not enough data yet" and so was never chased. The accessor is imported, not
// copied: a hand-copied accessor is how the drift happened the first time.
const SERIES = [
  { key: 'running', label: 'bees running', get: (r) => r.running, goodUp: true },
  { key: 'finished', label: 'dispatches finished', get: (r) => r.finished, goodUp: true },
  { key: 'claimed', label: 'claimed by parked', get: (r) => skipCount(r.skips, 'claimed'), goodUp: false },
  { key: 'completed', label: 'done but not closed', get: (r) => skipCount(r.skips, 'completed'), goodUp: false },
  { key: 'fileConflict', label: 'fenced by paths', get: (r) => skipCount(r.skips, 'fileConflict'), goodUp: false },
  // The largest number on the board - 448 briefs refused for want of a
  // Boundary - and it had no slope here. Whether that backlog is being repaired
  // or is still growing is the one question the level cannot answer, and it is
  // the question this whole file was written to answer for smaller counters.
  { key: 'missingBoundary', label: 'refused: no Boundary', get: (r) => skipCount(r.skips, 'missingBoundary'), goodUp: false },
]

// `ledger` is a parameter so the suite can point this at a fixture. A test that
// had to write into the real ledger to exercise a reader would be a test
// writing where production reads, which this loop forbids for good reason.
export function trend(hours, ledger = LEDGER) {
  const since = Date.now() - hours * 3600000
  const rows = (fs.existsSync(ledger)
    ? fs.readFileSync(ledger, 'utf8').trim().split('\n').filter(Boolean)
    : [])
    .map((l) => { try { return JSON.parse(l) } catch { return null } })
    .filter((r) => r && r.kind === 'snapshot' && Date.parse(r.at) >= since)

  const out = []
  for (const s of SERIES) {
    // A counter that was not measured on a tick is absent, not zero. The
    // dashboard already learned that lesson: a capacity refusal short-circuits
    // before the skip loop, so `skips` is `{}` and every skip counter is
    // MISSING, not 0. Including those as zeroes would invent a crash and then
    // a recovery, twice per hour.
    const points = rows
      .map((r) => ({ at: Date.parse(r.at), v: s.get(r) }))
      .filter((p) => typeof p.v === 'number')
    // ZERO IS NOT "A FEW". No sample in the window carried this key at all,
    // which is a writer that stopped or a reader looking at the wrong shape -
    // not a series that needs more time. "too few points" is a patient phrase
    // and it hid a broken accessor for a day across 96 snapshots.
    if (points.length === 0) { out.push({ ...s, state: 'NEVER MEASURED', n: 0 }); continue }
    if (points.length < 2) { out.push({ ...s, state: 'too few points', n: points.length }); continue }
    const first = points[0]
    const last = points[points.length - 1]
    const span = (last.at - first.at) / 3600000
    if (span < 0.2) { out.push({ ...s, state: 'span too short', n: points.length }); continue }
    const rate = (last.v - first.v) / span
    out.push({ ...s, state: 'measured', n: points.length, first: first.v, last: last.v, span, rate })
  }
  return out
}

if (isMain) {
  const hours = Number(process.argv[2] || 6)
  const rows = trend(hours)
  console.log(`trend over the last ${hours} h, from the snapshot lines already in the ledger\n`)

  for (const r of rows) {
    if (r.state !== 'measured') {
      console.log(`  ${r.label.padEnd(22)} ${r.state} (${r.n} usable point${r.n === 1 ? '' : 's'})`)
      continue
    }
    const dir = r.rate > 0.05 ? 'rising' : r.rate < -0.05 ? 'falling' : 'flat'
    const good = dir === 'flat' ? '' : ((r.rate > 0) === r.goodUp ? '  good' : '  BAD')
    console.log(
      `  ${r.label.padEnd(22)} ${String(r.last).padStart(4)}  ` +
      `${(r.rate >= 0 ? '+' : '') + r.rate.toFixed(1)}/h  ${dir}${good}` +
      `   (${r.first} -> ${r.last} over ${r.span.toFixed(1)} h, ${r.n} points)`,
    )
  }

  const bad = rows.filter((r) => r.state === 'measured' && Math.abs(r.rate) > 0.05 && (r.rate > 0) !== r.goodUp)
  console.log(
    bad.length
      ? `\n${bad.length} counter(s) moving the wrong way. A level says where you are; ` +
        `a rate says whether the chain is winning.`
      : '\nnothing is moving the wrong way.',
  )
}
