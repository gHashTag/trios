#!/usr/bin/env node
// The loop's dashboard, drawn so that a stopped loop cannot look like a running one.
//
// WHAT THE OLD ONE GOT WRONG, AND WHY IT MATTERS.
//
// `DASHBOARD.txt` was accurate about the swarm and silent about itself. Its
// header read `TRIOS CONTINUOUS LOOP   cron */15   job 23d6fe89` for six days
// after that job stopped existing, and its swarm rows kept the last numbers
// anyone measured - `bees running 4` - while the live board said 0. Nothing was
// falsified. The screen simply had no way to say "this is old", so every figure
// on it borrowed the authority of the freshest one.
//
// Three rules follow, and they are the whole design:
//
//   1. LIVENESS IS TWO SIGNALS, NOT ONE. The launchd timers and the iteration
//      driver fail independently. On 2026-09-12 the timers had fired 1.7
//      minutes earlier and the driver had been dead for 147.8 hours. A single
//      "running" light would have been green.
//
//   2. EVERY NUMBER CARRIES ITS AGE. A measurement older than its own meaning
//      is marked, not drawn plain. The swarm row taken 6 days ago is not the
//      swarm row.
//
//   3. AN UNMEASURED FACT IS `-`. Never 0. `sense.mjs` returns null for a probe
//      that did not answer, and null renders as a dash, because "the Queen did
//      not reply" and "no bees are running" are opposite situations.
//
// The prose - what was done, what went wrong, what to do next - is NOT
// generated. It is read from `cycle-notes.json`, written by whoever ran the
// iteration. Numbers are measured, prose is written; this file will render an
// empty prose block rather than invent one.
//
// Usage:
//   node dash2.mjs                  # draw from a fresh reading, to stdout
//   node dash2.mjs --write          # also write DASHBOARD2.ansi and DASHBOARD2.txt
//   node dash2.mjs --reading <f>    # draw from a recorded reading instead of measuring
//   node dash2.mjs --no-color       # plain text

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { takeReading } from './sense.mjs'
import { findAnomalies } from './anomaly.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const NOTES = path.join(DIR, 'cycle-notes.json')

// THE ENVIRONMENT IS SHARED; THE COMMAND LINE IS NOT.
//
// `NO_COLOR` is a convention every program in a pipeline is meant to honour, so
// reading it at module scope is correct even when this file is imported -
// `cycle.mjs` inheriting it is the intended behaviour.
//
// `--no-color` is this program's own flag and was read here too, which meant
// that `paint` - and therefore the EXPORTED `renderDashboard` - coloured its
// output according to flags passed to whatever imported it. `cycle.mjs` is the
// importer, and a `--no-color` aimed at the cycle silently reached into the
// renderer's private setting by a route nobody wrote down. The guard at the
// foot ORs the flag in for the CLI; nothing else can reach it.
let NO_COLOR = !!process.env.NO_COLOR

const W = 78 // inner width, matching the old dashboard so both fit the same pane

// --- palette ---------------------------------------------------------------
// Claude Code's terminal identity: rounded box, one warm accent, everything
// structural in dim grey so the numbers carry the contrast.
const C = {
  reset: '[0m',
  dim: '[38;5;245m',
  rule: '[38;5;240m',
  bold: '[1m',
  accent: '[38;5;173m',
  ok: '[38;5;71m',
  warn: '[38;5;179m',
  bad: '[38;5;167m',
  faint: '[2m',
}
const paint = (s, c) => (NO_COLOR ? s : c + s + C.reset)

// --- box drawing, ASCII source, unicode output -----------------------------
const B = {
  tl: '╭', tr: '╮', bl: '╰', br: '╯',
  h: '─', v: '│', ls: '├', rs: '┤',
  dot: '●', arrow: '→', warn: '!',
}

/** Visible width: ANSI sequences occupy no columns. */
const vis = (s) => s.replace(/\[[0-9;]*m/g, '').length

function pad(s, n) {
  const d = n - vis(s)
  return d > 0 ? s + ' '.repeat(d) : s
}

function clip(s, n) {
  if (vis(s) <= n) return s
  // Only ever called on plain (uncoloured) text.
  return s.slice(0, Math.max(0, n - 1)) + '…'
}

const row = (s) => paint(B.v, C.rule) + ' ' + pad(s, W - 2) + ' ' + paint(B.v, C.rule)
const top = (title) => paint(B.tl + B.h, C.rule) + ' ' + title + ' ' + paint(B.h.repeat(Math.max(0, W - 3 - vis(title))) + B.tr, C.rule)
const sep = (label) => (label
  ? paint(B.ls + B.h, C.rule) + ' ' + paint(label, C.dim) + ' ' + paint(B.h.repeat(Math.max(0, W - 3 - vis(label))) + B.rs, C.rule)
  : paint(B.ls + B.h.repeat(W) + B.rs, C.rule))
const bottom = () => paint(B.bl + B.h.repeat(W) + B.br, C.rule)

/**
 * label / value / note, in three columns that stay aligned when values are `-`.
 *
 * A value wider than its column pushes the note right rather than colliding
 * with it: `feat/queen-supervisor` is the real branch name and truncating it to
 * fit a column would be the dashboard lying to save space.
 */
const LABEL_W = 26
const VALUE_W = 12
function kv(label, value, note, colour) {
  const raw = value === null || value === undefined ? '-' : String(value)
  const v = value === null || value === undefined ? paint('-', C.dim) : paint(raw, colour || C.bold)
  const used = LABEL_W + Math.max(VALUE_W, raw.length + 2)
  const n = note ? paint(clip(note, Math.max(0, W - 3 - used)), C.dim) : ''
  return row(paint(pad(label, LABEL_W), C.dim) + pad(v, Math.max(VALUE_W, raw.length + 2)) + n)
}

const show = (f) => (f && f.v !== null && f.v !== undefined ? f.v : null)

/** minutes/hours, spoken the way the number is read. */
function age(hours) {
  if (hours === null || hours === undefined) return '-'
  if (hours < 1 / 60) return 'just now'
  if (hours < 1) return Math.round(hours * 60) + 'm ago'
  if (hours < 48) return Math.round(hours * 10) / 10 + 'h ago'
  return Math.round(hours / 24 * 10) / 10 + 'd ago'
}

/**
 * Hours since an ISO stamp, or null when there is no stamp to measure from.
 *
 * This exists because of a lie this very file used to tell. The SWARM header
 * read `measured ' + age(0)` - a literal zero, so the section always said "just
 * now" no matter how old the reading was. Drawn from a live `takeReading()` it
 * happened to be true; drawn with `--reading <file>` from a recording made days
 * earlier it was false, and false in the one direction that matters, because
 * every number under that header borrowed its claimed freshness. `age(0)` was
 * house rule 1 broken by the file that was written to enforce it.
 */
function hoursSinceISO(iso) {
  if (!iso) return null
  const t = Date.parse(iso)
  return Number.isFinite(t) ? (Date.now() - t) / 36e5 : null
}

// ---------------------------------------------------------------------------

/**
 * The verdict, derived from the two liveness signals rather than from either.
 *
 * DRIVER is what advances iterations. TIMERS are what keep the fleet tidy.
 * A loop with live timers and a dead driver is STOPPED - it is not doing the
 * work the loop exists for - and saying so is the whole reason this file was
 * rewritten.
 */
function verdict(r, anomalies) {
  const stale = show(r.loop.staleHours)
  const driverCount = r.driver.v ? r.driver.v.claudeCron : null
  const blockers = anomalies.filter((a) => a.severity === 'blocker').length
  if (stale === null) return { word: 'UNKNOWN', colour: C.dim, why: 'the loop state could not be read' }
  if (driverCount === 0 && stale > 24) return { word: 'STOPPED', colour: C.bad, why: `no driver; last iteration ${age(stale)}` }
  if (blockers > 0) return { word: 'DEGRADED', colour: C.warn, why: `${blockers} blocking anomal${blockers === 1 ? 'y' : 'ies'}` }
  if (stale > 2) return { word: 'IDLE', colour: C.warn, why: `last iteration ${age(stale)}` }
  return { word: 'RUNNING', colour: C.ok, why: `iteration ${show(r.loop.iteration)} advancing` }
}

function readNotes() {
  try { return JSON.parse(fs.readFileSync(NOTES, 'utf8')) } catch { return null }
}

export function renderDashboard(r, anomalies, notes) {
  const L = []
  const v = verdict(r, anomalies)
  const q = r.queen.v
  const g = r.git.v
  const d = r.driver.v
  const t = r.timers.v
  const gt = r.gates.v
  const lk = r.lock.v
  const stale = show(r.loop.staleHours)

  const title = paint('TRIOS', C.accent) + ' ' + paint('CONTINUOUS LOOP', C.bold)
  L.push(top(title + '  ' + paint(B.dot + ' ' + v.word, v.colour)))

  // THE FIRST LINE IS THE READING'S OWN AGE, and it comes before any number it
  // qualifies. A dashboard file sat on disk for six days once; nothing on it was
  // wrong at the moment it was drawn, and everything on it was wrong when it was
  // read. A screen that cannot say how old it is has no way to stop that.
  const readAge = hoursSinceISO(r.at)
  const readOld = readAge !== null && readAge > 1
  L.push(row(
    paint(pad('drawn from a reading', LABEL_W), C.dim) +
    paint(pad(age(readAge), Math.max(VALUE_W, 12)), readOld ? C.bad : C.dim) +
    paint(r.at ? (readOld ? 'STALE - re-measure before trusting any row below' : String(r.at).replace('T', ' ').slice(0, 19) + 'Z') : 'the reading carries no timestamp', readOld ? C.bad : C.dim)
  ))
  L.push(sep())

  // The header states what drives it, measured at draw time. The old header
  // named a job id from memory and outlived it by six days.
  L.push(kv('iteration', show(r.loop.iteration), show(r.loop.title) ? clip(String(show(r.loop.title)), 44) : ''))
  L.push(kv('last advanced', age(stale), v.why, stale !== null && stale > 24 ? C.bad : C.bold))
  L.push(kv('driver (iterations)', d ? (d.claudeCron === 0 ? 'NONE' : d.claudeCron + ' cron') : null,
    d && d.claudeCron === 0 ? 'nothing schedules an iteration' : '', d && d.claudeCron === 0 ? C.bad : C.ok))
  L.push(kv('timers (housekeeping)', d ? d.launchd + ' launchd' : null,
    t && t.heal ? 'heal wrote ' + age(t.heal.lastWriteMin / 60) : '', d && d.launchd > 0 ? C.ok : C.bad))
  L.push(kv('lock', lk && lk.held ? lk.holder + ' ' + lk.ageMin + 'm' : 'free',
    lk && lk.held && lk.pidAlive === false ? 'holder pid is DEAD' : '',
    lk && lk.held && lk.ageMin > 60 ? C.warn : C.bold))

  // --- swarm ---------------------------------------------------------------
  const tickAge = q && q.tickAt ? hoursSinceISO(q.tickAt) : null
  // `r.queen.at` is when the probe ran, not when this function was called. They
  // are the same number only for a live draw; for `--reading <file>` they are
  // days apart, which is exactly the case the old `age(0)` got wrong.
  const swarmAge = hoursSinceISO(r.queen && r.queen.at)
  L.push(sep('SWARM' + (q ? '   measured ' + age(swarmAge) : '   UNREACHABLE')))
  if (!q) {
    L.push(row(paint('the Queen did not answer. No swarm number on this screen is current.', C.dim)))
  } else {
    L.push(kv('bees running', q.running, q.running === 0 ? 'nothing is being worked on' : '', q.running === 0 ? C.bad : C.ok))
    L.push(kv('dispatches finished', q.finished))
    L.push(kv('tick refusal', q.refusal ? clip(q.refusal, 30) : null, tickAge !== null ? 'tick ' + age(tickAge) : ''))
    L.push(kv('candidates skipped', q.skips))
    // The line this whole loop turns on: a refusal that reads as an empty
    // backlog, next to the number proving the backlog is full and unreadable.
    const share = q.skips && q.missingBoundary ? Math.round((q.missingBoundary / q.skips) * 100) : null
    L.push(kv('  missing boundary', q.missingBoundary, share !== null ? share + '% of all skips' : '',
      q.missingBoundary > 0 ? C.bad : C.ok))
    L.push(kv('  claimed by a fence', q.claimed))
    L.push(kv('  done but still open', q.completed))
  }
  // THE NUMBER THE SIX ROWS ABOVE CANNOT GIVE, and the reason this loop spent
  // weeks on the wrong cause. `missing boundary 448` invites "the parser is
  // broken"; `claimed 13` and `done 24` look like noise beside it. Cross them
  // and the answer falls out: every issue the Queen can read is already taken,
  // so the refusal is true and the parser is fine. Deep-pass only, and `-` when
  // unmeasured - printing 0 for "nobody looked" would restage the same error in
  // the opposite direction.
  {
    const bl = r.backlog && r.backlog.v
    const d = bl && typeof bl.delegableNow === 'number' ? bl.delegableNow : null
    L.push(kv('available to a bee', d === null ? null : d,
      d === null ? 'needs a deep cycle' : d === 0 ? 'all ' + bl.hasBoundary + ' well-formed issues are taken' : 'ready to dispatch',
      d === 0 ? C.bad : C.ok))
  }
  if (r.idle.v) {
    L.push(kv('window with no bee', r.idle.v.percentNoBee + '%', '', r.idle.v.percentNoBee > 50 ? C.bad : C.ok))
    const top1 = r.idle.v.reasons && r.idle.v.reasons[0]
    L.push(kv('rounds failed', r.idle.v.roundsFailed, top1 ? top1.n + ' x ' + top1.why : ''))
  }

  // --- tree ----------------------------------------------------------------
  L.push(sep('TREE'))
  L.push(kv('branch', g ? clip(g.branch, 28) : null, g ? 'ahead ' + g.ahead + '  behind ' + g.behind : '',
    g && g.behind > 50 ? C.warn : C.bold))
  L.push(kv('uncommitted source', g ? g.modifiedSource + ' mod / ' + g.untrackedSource + ' new' : null,
    'generated files excluded', g && (g.modifiedSource + g.untrackedSource) > 0 ? C.warn : C.ok))
  L.push(kv('worktrees', g ? g.worktrees : null))
  L.push(kv('volume used', r.disk.v ? r.disk.v.percentUsed + '%' : null,
    r.disk.v && r.disk.v.percentUsed >= 85 ? 'bees die at 0s when this fills' : '',
    r.disk.v && r.disk.v.percentUsed >= 85 ? C.bad : C.ok))
  L.push(kv('gates declared / wired', gt ? gt.declared + ' / ' + (gt.wired === null ? '-' : gt.wired) : null,
    gt && gt.wired !== null && gt.declared > gt.wired ? (gt.declared - gt.wired) + ' run only on a Mac' : '',
    gt && gt.wired !== null && gt.declared > gt.wired ? C.warn : C.ok))

  // --- anomalies -----------------------------------------------------------
  const bySev = { blocker: [], high: [], medium: [], low: [] }
  for (const a of anomalies) (bySev[a.severity] || bySev.low).push(a)
  const counts = `${bySev.blocker.length} blocker  ${bySev.high.length} high  ${bySev.medium.length} medium`
  L.push(sep('ANOMALIES   ' + counts))
  if (!anomalies.length) {
    L.push(row(paint('none found in this reading', C.dim)))
  } else {
    for (const a of [...bySev.blocker, ...bySev.high, ...bySev.medium].slice(0, 7)) {
      const mark = a.severity === 'blocker' ? paint(B.warn, C.bad) : a.severity === 'high' ? paint(B.warn, C.warn) : paint(B.dot, C.dim)
      L.push(row(mark + ' ' + clip(a.truth, W - 6)))
    }
    if (anomalies.length > 7) L.push(row(paint('  +' + (anomalies.length - 7) + ' more - tri cycle-anomalies', C.dim)))
  }

  // --- prose ---------------------------------------------------------------
  // Written, never generated. An empty block is the honest render when nobody
  // has written one for this iteration.
  if (notes && Array.isArray(notes.did) && notes.did.length) {
    L.push(sep('WORK THIS ITERATION'))
    for (const line of notes.did.slice(0, 8)) {
      L.push(row(paint(B.dot, C.accent) + ' ' + clip(String(line), W - 4)))
    }
  }
  if (notes && Array.isArray(notes.next) && notes.next.length) {
    L.push(sep('THREE WAYS TO CONTINUE'))
    notes.next.slice(0, 3).forEach((line, i) => {
      L.push(row(paint(String(i + 1) + '.', C.accent) + ' ' + clip(String(line), W - 5)))
    })
  }

  L.push(bottom())
  return L.join('\n')
}

// ---------------------------------------------------------------------------

const isMain = process.argv[1] && process.argv[1].endsWith('/dash2.mjs')
if (isMain) {
  const argv = process.argv.slice(2)
  const WRITE = argv.includes('--write')
  const readingArg = argv.indexOf('--reading')
  if (argv.includes('--no-color')) NO_COLOR = true
  let r
  if (readingArg >= 0 && argv[readingArg + 1]) {
    const raw = fs.readFileSync(argv[readingArg + 1], 'utf8').trim().split('\n')
    r = JSON.parse(raw[raw.length - 1])
  } else {
    r = takeReading()
  }
  const anomalies = findAnomalies(r)
  const notes = readNotes()
  const coloured = renderDashboard(r, anomalies, notes)
  process.stdout.write(coloured + '\n')
  if (WRITE) {
    fs.writeFileSync(path.join(DIR, 'DASHBOARD2.ansi'), coloured + '\n')
    const plain = coloured.replace(/\[[0-9;]*m/g, '')
    fs.writeFileSync(path.join(DIR, 'DASHBOARD2.txt'), plain + '\n')
  }
}
