#!/usr/bin/env node
// One command that measures the swarm, updates the anchors in the right order,
// and renders the dashboard.
//
// The ordering matters and got it wrong once. `anchor(name, value)` writes the
// new value and RETURNS the old one. If an iteration calls anchor() for every
// metric first and renders afterwards, every delta reads "=" because the stored
// value is already the new one. So the delta must come from anchor()'s return
// value, in the same pass. That is what this file guarantees, so no future
// iteration has to remember it.
//
// Usage:
//   node snapshot.mjs                       # measure, anchor, render
//   node snapshot.mjs --work work.json      # ...with this iteration's work list
//   node snapshot.mjs --no-live-idle        # skip the one remote measurement
//   node snapshot.mjs --selftest            # the calibration cases, no network

import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const L = await import(path.join(DIR, 'loop.mjs'))
const D = await import(path.join(DIR, 'dash.mjs'))
const T27 = await import(path.join(DIR, 't27-parity.mjs'))
const QUEEN = 'https://trios-agent-server-production.up.railway.app/queen/status'

// argv only when this file IS the program. See loop.mjs: an importer's own
// arguments reaching an imported module's dispatch released the loop lock.
const isMain = process.argv[1] && process.argv[1].endsWith('/snapshot.mjs')
const flag = (f) => Boolean(isMain && process.argv.includes(f))
const argValue = (f) => {
  const at = isMain ? process.argv.indexOf(f) : -1
  return at > 0 ? process.argv[at + 1] : null
}

const ESC = String.fromCharCode(27)
const red = (t) => `${ESC}[31m${t}${ESC}[0m`
const strip = (t) => String(t).replace(new RegExp(ESC + '\\[[0-9;]*m', 'g'), '')

// IMPORTED, NOT RESTATED. This used to read `= 55`, under a comment claiming 55
// was "the widest label that cannot push the value or the delta off the end of
// the box". That was never true: the renderer clips the LABEL itself to 34, not
// the row, so every label between 35 and 55 columns was built here and cut
// there. The number now comes from the renderer that enforces it, and a label
// that will not fit no longer gets truncated mid-word - it hands its gloss to
// the note line under the row, where nothing has to be abbreviated.
export const LABEL_MAX = L.LABEL_W

async function live() {
  const ctl = new AbortController()
  const timer = setTimeout(() => ctl.abort(), 20000)
  try {
    const r = await fetch(QUEEN, { signal: ctl.signal })
    if (!r.ok) return { error: `queen answered ${r.status}` }
    return await r.json()
  } catch (e) {
    // An unreachable Queen is a finding, not a reason to render stale numbers as
    // if they were fresh. Say so on the dashboard.
    return { error: String(e.message || e) }
  } finally { clearTimeout(timer) }
}

// The anchor KEY is a stable identifier and the label is what the dashboard
// prints. They were the same string once, which meant that rewording a label
// silently reset that metric's history to "new" - the delta column lied by
// omission rather than by a wrong number, which is harder to notice.
//
// AN ABSENT VALUE IS NEVER ANCHORED. `anchor(key, undefined)` writes a history
// entry for a number nobody read, and next round's delta is computed against
// it - so one unreadable answer poisons the comparison for every round after,
// which is worse than the missing row it came from. The skip rows carried this
// guard by hand while the swarm rows did not; it belongs here, where every
// caller gets it, rather than in the callers that happened to remember.
export const makeMetric = (anchor) => (key, label, v, goodDown = true) => {
  if (v === null || v === undefined) return { k: `${label}  (key absent)`, v: 'absent', prev: null, goodDown }
  return { k: label, v, prev: anchor(key, v), goodDown }
}

// -------------------------------------------------------------- the skip rows

// THE PRODUCER DECIDES WHICH REASONS EXIST, NOT THIS FILE.
//
// Three reasons were written here by hand - fileConflict, claimed, completed -
// and the list drifted from the service that emits it. On 2026-09-12 the live
// `skipSummary` held missingBoundary 448 of 486 skipped candidates (92%), and
// the box drew no row for it at all: the largest single reason the swarm does
// nothing was invisible on the artifact built to show why the swarm does
// nothing. Counting the ledger, missingBoundary was present in 54 of the last
// 62 snapshots, so this was not a new key catching an old file out.
//
// So the rows are ENUMERATED from `skipSummary`, sorted by count, uncapped. A
// reason the producer invents tomorrow gets a row tomorrow.
//
// The gloss below ANNOTATES a key; it never selects one. An unglossed key
// still prints, under the producer's own name. That is the difference between
// a hand-written index and a hand-written footnote.
const SKIP_GLOSS = {
  missingBoundary: 'no boundary in the brief',
  fileConflict: 'fenced by parked paths',
  claimed: 'claimed by parked dispatch',
  completed: 'done but never closed',
}

// THE VALUE SHAPE CHANGED UNDER THE ROWS AND BOTH FAILURE MODES WERE SILENT.
//
// The service now emits `{count, issues, more}` where it used to emit a number.
// `s.claimed ?? 0` therefore passed an OBJECT to the renderer, whose
// `isMeasured()` turns it into NaN and prints `-` - a measured 15 drawn as "not
// measured" - while a key that was genuinely absent hit the `?? 0` and printed
// a confident 0. Exactly backwards, in the two directions that matter most.
export const skipCount = (v) => (v && typeof v === 'object' ? v.count : v)

/**
 * The skip block: the denominator, then every reason the producer sent.
 *
 * `measured` is false when the tick short-circuited (see the caller). Then
 * nothing here is a measurement and - just as important - nothing is anchored,
 * or the next iteration would compare against a fake zero and print an equally
 * fake recovery.
 */
export function buildSkipRows(t, s, measured, opts = {}) {
  const anchor = opts.anchor || L.anchor
  const metric = makeMetric(anchor)

  const skipMetric = (key, label, raw) => {
    if (!measured) return { k: label, v: 'not measured', prev: null, goodDown: true }
    const v = skipCount(raw)
    // ABSENT IS NOT ZERO. The value column can only draw `-` for anything
    // non-numeric, so the distinction is carried in the label, and an absent
    // key is not anchored: there is nothing to compare next time.
    if (v === null || v === undefined) return { k: `${label}  (key absent)`, v: 'absent', prev: null, goodDown: true }
    return metric(key, label, v)
  }

  const rows = [skipMetric('skip.total', 'candidates skipped', t ? t.skippedCount : undefined)]
  const total = Number(skipCount(t ? t.skippedCount : undefined))
  const entries = Object.entries(s || {})
    .map(([k, v]) => [k, skipCount(v)])
    .sort((a, b) => (Number(b[1]) || 0) - (Number(a[1]) || 0) || String(a[0]).localeCompare(String(b[0])))

  entries.forEach(([k, n], i) => {
    const share = Number.isFinite(total) && total > 0 && Number.isFinite(Number(n))
      ? (100 * Number(n)) / total
      : null
    // The gloss travels beside the row, not inside its label. `missingBoundary`
    // with its share is 23 columns and fits; `missingBoundary 92.2%  no boundary
    // section` is 46 and used to be cut at 34, keeping the share and losing the
    // sentence that says what was skipped. The renderer prints `gloss` on the
    // note line under the metric.
    const gloss = SKIP_GLOSS[k] || null
    const label = `  ${k}${share === null ? '' : ` ${share.toFixed(1)}%`}`.slice(0, LABEL_MAX)
    // Red only for the leader, and only when it is more than half of everything
    // skipped: at that point it is not one reason among several, it is THE
    // reason, and the rest is detail.
    const dominant = i === 0 && share !== null && share > 50
    rows.push({ ...skipMetric(`skip.${k}`, dominant ? red(label) : label, n), gloss })
  })
  return rows
}

// ----------------------------------------------------------- the round failures

/**
 * Why rounds died in the same window the idle percentage is taken over.
 *
 * One failed round dispatches nothing at all - no review, no choice, no bee -
 * so this belongs ABOVE the utilisation it causes. `failures` is whatever
 * idle.mjs read out of the service log: `null` when the log could not be read,
 * which is reported as not measured and never as zero failures.
 */
/**
 * The idle row, from a live measurement or from the last recorded one.
 *
 * EXTRACTED SO THE ANCHOR KEY CAN BE HELD BY A TEST. This was eleven lines
 * inline in `main`, reachable only by running the real thing against the real
 * service, which is why the defect below lived in it unnoticed: the fallback
 * path anchored the RECORDED value on `idle.percent`, the same key a live
 * measurement writes. A live run measured 84; a `--no-live-idle` run then wrote
 * a 151h-old 45 to that key; the next render compared them and printed a
 * 39-point fall over "/ 2m". Nobody measured a fall. Alternating the two paths
 * prints -39 and +39 for ever, in colour, with an arrow, as movement - the
 * exact defect this loop exists to hunt, inside the instrument that hunts it.
 *
 * Two keys, not one key with a careful label. The label is for the reader; the
 * thing that needs protecting is `delta()`, and `delta()` reads the key.
 */
export function buildIdleRow(idle, recordedIdle, recordedAgeH, opts = {}) {
  const metric = makeMetric(opts.anchor || L.anchor)
  if (idle) return metric('idle.percent', `hours ${idle.hours}: no bee working, percent`, idle.idlePercent)
  // Not measured is not zero, and an unmeasured value is not anchored: there
  // would be nothing for the next run to compare against.
  if (recordedIdle === null || recordedIdle === undefined) {
    return { k: `hours ${IDLE_HOURS}: no bee working, percent`, v: 'not measured', prev: null, goodDown: true }
  }
  const age = recordedAgeH === null || recordedAgeH === undefined ? '' : `, ${recordedAgeH}h old`
  return metric('idle.percent.recorded', `hours ${IDLE_HOURS}: no bee working, pct (recorded${age})`, recordedIdle)
}

export function buildFailureRows(failures, hours, opts = {}) {
  const anchor = opts.anchor || L.anchor
  const metric = makeMetric(anchor)
  // The renderer pads labels to 34 before the value column, so a long heading
  // shoves its own number out of line with every row around it. Short heading,
  // aligned number; what the number counts is the rows underneath it.
  const head = `ROUND FAILURES, last ${hours}h`.slice(0, LABEL_MAX)
  if (!failures || !Number.isFinite(Number(failures.total))) {
    return [{ k: head, v: 'not measured', prev: null, goodDown: true }]
  }
  const rows = [metric('rounds.failed', head, Number(failures.total))]
  for (const [reason, n] of failures.byReason || []) {
    // NARROW ON PURPOSE. `/403/` also matches 4030 and 1403; the word boundary
    // keeps the annotation on an HTTP status and off a file count. It only ever
    // annotates - a reason that does not match still gets its row, under the
    // producer's own words.
    const tag = /\b403\b/.test(String(reason)) ? 'aborts the round' : null
    const label = `  ${String(reason).slice(0, LABEL_MAX - 2)}`.slice(0, LABEL_MAX)
    rows.push({ ...metric(`rounds.fail.${slug(reason)}`, tag ? red(label) : label, Number(n)), gloss: tag })
  }
  return rows
}

export const slug = (s) =>
  String(s).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 40) || 'unnamed'

// ------------------------------------------------------------------- the idle

// A RATE NEEDS A WINDOW, AND A WINDOW MEASURED SIX DAYS AGO IS NOT THIS ONE.
//
// This row used to read `D.lastReading()`, a file written whenever someone last
// ran `dash.mjs --record`. On 2026-09-12 that file still said 45% from
// 2026-09-06 while the swarm was actually idle 84% of the last twelve hours -
// the dashboard was half a fact old enough to vote. So the row is taken live.
//
// The comment this replaces was right about the cost and is still obeyed: the
// remote query is made ONCE, bounded by a deadline, and a run that cannot
// afford it passes --no-live-idle (or TRIOS_SNAPSHOT_LIVE_IDLE=0) and falls
// back to the recorded reading - labelled as recorded, with its age, never
// passed off as fresh.
const IDLE_HOURS = Number(process.env.TRIOS_IDLE_HOURS || 12)
const IDLE_TIMEOUT_MS = Number(process.env.TRIOS_IDLE_TIMEOUT_MS || 150000)

/** The `--json` block idle.mjs prints after its prose. */
export function parseIdleJSON(text) {
  const s = String(text || '')
  let at = s.indexOf('{\n "u":')
  if (at < 0) {
    const nl = s.lastIndexOf('\n{\n')
    if (nl < 0) return null
    at = nl + 1
  }
  let depth = 0
  let inStr = false
  let esc = false
  for (let i = at; i < s.length; i++) {
    const c = s[i]
    if (inStr) {
      if (esc) esc = false
      else if (c === '\\') esc = true
      else if (c === '"') inStr = false
      continue
    }
    if (c === '"') { inStr = true; continue }
    if (c === '{') depth++
    else if (c === '}') {
      depth--
      if (depth === 0) { try { return JSON.parse(s.slice(at, i + 1)) } catch { return null } }
    }
  }
  return null
}

/**
 * Run idle.mjs and read its measurement.
 *
 * A NON-ZERO EXIT HERE IS THE ANSWER: idle.mjs exits 2 precisely when the swarm
 * is idle 25% of the window or more, which is the finding, not a failure. So
 * the status is ignored and the stdout is parsed. When nothing parses - an
 * unreadable board, a dead channel, a timeout - this returns null and every row
 * that depends on it says so.
 */
export function runIdle(spawn = spawnSync, hours = IDLE_HOURS, timeoutMs = IDLE_TIMEOUT_MS) {
  let r = null
  try {
    r = spawn(process.execPath, [path.join(DIR, 'idle.mjs'), '--hours', String(hours), '--json'], {
      encoding: 'utf8',
      timeout: timeoutMs,
      maxBuffer: 16 * 1024 * 1024,
      env: { ...process.env, CHANNEL_DEADLINE_MS: String(Math.max(30000, timeoutMs - 20000)) },
    })
  } catch { return null }
  const parsed = parseIdleJSON(r && r.stdout)
  if (!parsed || !parsed.u || !Number.isFinite(Number(parsed.u.idlePercent))) return null
  return {
    hours,
    idlePercent: Number(parsed.u.idlePercent),
    failures: parsed.failures || null,
    why: parsed.v ? parsed.v.why : null,
  }
}

// ------------------------------------------------------------------- the pass

async function main() {
  const j = await live()
  const t = j.lastTick || {}
  const d = j.dispatches || {}
  const s = t.skipSummary || {}
  const metric = makeMetric(L.anchor)

  // THE SKIP COUNTERS ARE NOT ALWAYS MEASURED, AND ZERO IS NOT ALWAYS PROGRESS.
  //
  // When every worker slot is full the round refuses on capacity and returns
  // BEFORE the per-issue skip loop runs. `skippedCount` is then 0 and
  // `skipSummary` is `{}` - not because the fences cleared, but because nobody
  // looked. Rendering that as a fall from 31 to 0 in green is the dashboard
  // telling a comforting lie, which is the defect this whole loop exists to
  // hunt.
  const measuredSkips = !(j.error || (!t.allowed && /workers already running/.test(String(t.refusal || ''))))

  // READ THE RECORD, DO NOT RECOMPUTE IT - the rule `provenCounts` already
  // follows. `dash.mjs --record` measures these; the box shows what was last
  // measured, or `-` if nothing has been. An absent reading is absent, never
  // zero. (The idle row is the deliberate exception: see runIdle above.)
  const recorded = D.lastReading()
  const looping = recorded && recorded.looping ? recorded.looping.looping : null
  const divergent = recorded && recorded.divergent ? recorded.divergent.rows : null
  const recordedIdle = recorded && typeof recorded.idle === 'number' ? recorded.idle : null
  const recordedAgeH = recorded && recorded.at
    ? Math.round((Date.now() - Date.parse(recorded.at)) / 3600000)
    : null

  const wantLiveIdle = !flag('--no-live-idle') && process.env.TRIOS_SNAPSHOT_LIVE_IDLE !== '0'
  const idle = wantLiveIdle ? runIdle() : null

  const idleRow = buildIdleRow(idle, recordedIdle, recordedAgeH, { anchor: L.anchor })
  const failureRows = buildFailureRows(idle ? idle.failures : null, idle ? idle.hours : IDLE_HOURS)

  const swarm = j.error
    ? [
        { k: 'QUEEN UNREACHABLE', v: j.error.slice(0, 20), prev: null, goodDown: true },
        // The failure and idle rows do not come from the Queen's status - they
        // come from the dispatch table and the service log - so an unreachable
        // Queen is not a reason to drop them. It is usually the moment they
        // matter most.
        ...failureRows,
        idleRow,
      ]
    : [
        // ABSENT IS NOT ZERO, here too. `d.running ?? 0` printed a confident
        // "bees running (of 4)  0" whenever the dispatch block was missing from
        // the Queen's answer - the single most alarming number on the box,
        // asserted from a key nobody had read. The skip rows above were taught
        // this distinction and these two were left behind; the value column
        // draws `-` for anything non-numeric, so passing the absent value
        // through unchanged is all that is needed.
        metric('swarm.running', D.capacityLabel(() => T27.ringConst('MAX_CONCURRENT_WORKERS')), d.running, false),
        metric('swarm.finished', 'dispatches finished', d.finished, false),
        ...buildSkipRows(t, s, measuredSkips),
        ...failureRows,
        // An attempt is charged against the retry ceiling only when a criterion
        // was tested and FAILED, so a bee that goes silent is deliberately not
        // charged. A bee silent EVERY time therefore loops with no ceiling at
        // all. THE GOLDEN RULE, as a number: the bees work, and start again the
        // moment they finish. `bees running` above is an instant of a bimodal
        // quantity and hid a swarm that was idle half the day.
        idleRow,
        looping === null
          ? { k: 'send-backs looping with no ceiling', v: 'not measured', prev: null, goodDown: true }
          : metric('looping.noCeiling', 'send-backs looping with no ceiling', looping),
        // Two implementations of one rule, disagreeing. Standing debt: it does
        // not fall when one side is fixed, only when one of them stops existing.
        divergent === null
          ? { k: 'rows where one rule answers two ways', v: 'not measured', prev: null, goodDown: true }
          : metric('agree.divergent', 'rows where one rule answers two ways', divergent),
      ]

  // Explicitly guarded, not implicitly. These lines were safe only because
  // `argWork` is -1 off the main path, and safety nobody can see is safety the
  // next edit removes. The guard that flagged them was right to.
  const workFile = argValue('--work')
  const extra = workFile ? JSON.parse(fs.readFileSync(workFile, 'utf8')) : {}

  const facts = {
    swarm,
    work: extra.work || [],
    anomalies: extra.anomalies || [],
    next: extra.next || [],
  }

  L.append({
    kind: 'snapshot',
    running: d.running ?? null,
    finished: d.finished ?? null,
    refusal: j.error ? 'unreachable' : (t.allowed ? 'dispatched' : t.refusal),
    skips: s,
    idlePercent: idle ? idle.idlePercent : null,
    idleHours: idle ? idle.hours : null,
    roundFailures: idle && idle.failures ? idle.failures.total : null,
    roundFailureReasons: idle && idle.failures ? idle.failures.byReason : null,
  })

  console.log(L.renderDashboard(facts))
  if (!j.error) {
    console.log(`\n${t.allowed ? 'DISPATCHED' : 'refused: ' + t.refusal}   tick ${(t.decidedAt || '').slice(11, 19)}Z`)
  }
  if (idle && idle.why) console.log(`idle ${idle.idlePercent}% over ${idle.hours}h: ${idle.why}`)
  if (!idle && wantLiveIdle) console.log('idle: NOT measured this run - the recorded reading is shown, with its age')
}

// -------------------------------------------------------- the calibration cases

// These pin the two defects this file was carrying, so a later edit cannot
// quietly restore either: a hand-written list of skip reasons, and a value shape
// that printed `-` for a measured number and `0` for an absent one. They touch
// no network and no state - `anchor` is injected.
function selftest() {
  const fails = []
  let ran = 0
  // The count is COUNTED. A hand-written total is the same defect as a
  // hand-written list of skip reasons, one scale down.
  const ok = (name, cond, got) => { ran++; if (!cond) fails.push(`${name}  got: ${JSON.stringify(got)}`) }
  const noAnchor = { anchor: () => null }
  const seen = []
  const spyAnchor = { anchor: (k, v) => { seen.push([k, v]); return null } }

  // The live shape, 2026-09-12: objects, and a key no hand-written list had.
  const s = {
    claimed: { count: 14, issues: [1], more: 0 },
    completed: { count: 24, issues: [2], more: 0 },
    missingBoundary: { count: 448, issues: [3], more: 423 },
  }
  const t = { skippedCount: 486, allowed: false }
  const rows = buildSkipRows(t, s, true, spyAnchor)
  const label = (r) => strip(r.k)

  ok('a row per producer key, plus the denominator', rows.length === 4, rows.map(label))
  ok('the largest reason leads', /missingBoundary/.test(label(rows[1])), label(rows[1]))
  ok('its share is printed', /92\.2%/.test(label(rows[1])), label(rows[1]))
  ok('a dominant reason is painted red', rows[1].k.includes(`${ESC}[31m`), rows[1].k)
  ok('a minority reason is not', !rows[2].k.includes(`${ESC}[31m`), rows[2].k)
  ok('the object shape is unwrapped to its count', rows[1].v === 448, rows[1].v)
  ok('descending by count', rows.slice(1).map((r) => r.v).join() === '448,24,14', rows.slice(1).map((r) => r.v))
  // THIS CASE USED TO BE UNFAILABLE. It asserted `<= 55` while the renderer
  // clipped at 34, so it went green on every label between the two - which is
  // to say it was green for exactly the rows it was written to protect. The
  // bound is now the renderer's own exported constant, so the case fails when
  // the row is cut. A gate whose threshold is looser than the thing it guards
  // is not a weak gate, it is an absent one that reports.
  ok('no row is wider than the renderer will draw', rows.every((r) => strip(r.k).length <= L.LABEL_W), rows.map((r) => strip(r.k).length))
  ok('the bound is the renderer\'s, not a second copy', LABEL_MAX === L.LABEL_W, { LABEL_MAX, LABEL_W: L.LABEL_W })
  // The point of moving the gloss out of the label: it survives whole.
  ok('the gloss rides beside the row, not inside the label', rows[1].gloss === SKIP_GLOSS.missingBoundary, rows[1].gloss)
  ok('and is not truncated to fit', strip(rows[1].gloss).length > L.LABEL_W - strip(rows[1].k).length, strip(rows[1].gloss).length)
  ok('the label keeps the share it can afford', /missingBoundary 92\.2%/.test(strip(rows[1].k)), strip(rows[1].k))
  ok('anchor keys survive the rename', seen.some(([k]) => k === 'skip.claimed'), seen.map(([k]) => k))

  // A key the producer invents tomorrow must get a row tomorrow, without an
  // edit here. This is the case that fails if anyone re-hardcodes the list.
  const withNew = buildSkipRows({ skippedCount: 100 }, { brandNewReason: { count: 99 }, claimed: 1 }, true, noAnchor)
  ok('an unknown key still gets a row', /brandNewReason/.test(strip(withNew[1].k)), withNew.map((r) => strip(r.k)))
  ok('an unglossed key prints its own name', strip(withNew[1].k).trim().startsWith('brandNewReason'), strip(withNew[1].k))
  ok('a plain number still works', withNew[2].v === 1, withNew[2].v)

  // Absent is not zero, and an absent key is not anchored.
  const absentSeen = []
  const absent = buildSkipRows({}, {}, true, { anchor: (k, v) => { absentSeen.push([k, v]); return null } })
  ok('an absent denominator is not a zero', absent[0].v === 'absent', absent[0].v)
  ok('absence is said in the label', /key absent/.test(absent[0].k), absent[0].k)
  ok('an absent value is never anchored', absentSeen.length === 0, absentSeen)

  // A short-circuited tick measures nothing and anchors nothing.
  const unmeasuredSeen = []
  const unmeasured = buildSkipRows(t, s, false, { anchor: (k, v) => { unmeasuredSeen.push([k, v]); return null } })
  ok('a tick that did not look reports so', unmeasured.every((r) => r.v === 'not measured'), unmeasured.map((r) => r.v))
  ok('and writes no anchor', unmeasuredSeen.length === 0, unmeasuredSeen)

  // Round failures.
  const f = buildFailureRows({ total: 14, byReason: [['GitHub returned 403', 14]] }, 12, noAnchor)
  ok('the total is measured', f[0].v === 14, f[0].v)
  ok('the window is named', /12h/.test(strip(f[0].k)), strip(f[0].k))
  ok('403 is named and marked', /403/.test(strip(f[1].k)) && f[1].k.includes(`${ESC}[31m`), f[1].k)
  ok('what a 403 does is said beside the row', f[1].gloss === 'aborts the round', f[1].gloss)
  ok('failure rows fit what the renderer draws', f.every((r) => strip(r.k).length <= L.LABEL_W), f.map((r) => strip(r.k).length))

  // THE SPLICED SERIES. A live reading and a recorded one must never land on the
  // same anchor key, or delta() subtracts two different moments and reports the
  // result as movement.
  const idleKeys = []
  const spyIdle = { anchor: (k, v) => { idleKeys.push([k, v]); return null } }
  buildIdleRow({ hours: 12, idlePercent: 84 }, 45, 151, spyIdle)
  buildIdleRow(null, 45, 151, spyIdle)
  ok('a live idle reading and a recorded one use different anchors',
    idleKeys.length === 2 && idleKeys[0][0] !== idleKeys[1][0], idleKeys.map(([k]) => k))
  ok('the live reading keeps the live key', idleKeys[0][0] === 'idle.percent', idleKeys[0][0])
  ok('the recorded reading says so in its label',
    /recorded, 151h old/.test(strip(buildIdleRow(null, 45, 151, noAnchor).k)), strip(buildIdleRow(null, 45, 151, noAnchor).k))
  const idleNone = buildIdleRow(null, null, null, spyIdle)
  ok('an unmeasured idle row is not a zero', idleNone.v === 'not measured', idleNone.v)
  ok('and is never anchored', idleKeys.length === 2, idleKeys.map(([k]) => k))

  const fNone = buildFailureRows(null, 12, noAnchor)
  ok('an unread log is not zero failures', fNone[0].v === 'not measured', fNone[0].v)
  const fZero = buildFailureRows({ total: 0, byReason: [] }, 12, noAnchor)
  ok('a measured zero is a zero', fZero[0].v === 0 && fZero.length === 1, fZero.map((r) => r.v))

  // THE RECURRING DEFECT CLASS: a token that is not the thing. `403` inside a
  // larger number is not an HTTP status.
  const fWide = buildFailureRows({ total: 3, byReason: [['4030 files changed', 2], ['error 1403', 1]] }, 12, noAnchor)
  ok('4030 is not a 403', !fWide[1].k.includes(`${ESC}[31m`), fWide[1].k)
  ok('1403 is not a 403', !fWide[2].k.includes(`${ESC}[31m`), fWide[2].k)
  ok('slugs are stable anchor keys', slug('GitHub returned 403') === 'github-returned-403', slug('GitHub returned 403'))

  // The idle reader takes its answer from stdout, not from the exit status.
  const sample = 'prose\nand more prose\n{\n "u": {\n  "idlePercent": 84\n },\n "v": {"why": "x"},\n "failures": {"total": 14, "byReason": [["GitHub returned 403", 14]]}\n}\n\n84% of the window had no bee working\n'
  const fake = () => ({ status: 2, stdout: sample, stderr: '' })
  const got = runIdle(fake, 12, 1000)
  ok('exit 2 is an answer, not a failure', got && got.idlePercent === 84, got)
  ok('the failures come with it', got && got.failures.total === 14, got && got.failures)
  ok('an unparseable run measures nothing', runIdle(() => ({ status: 3, stdout: 'unreadable' }), 12, 1000) === null, 'null expected')
  ok('a brace inside a string does not end the json', parseIdleJSON('{\n "u": {"why": "a } brace", "idlePercent": 5}}').u.idlePercent === 5, 'parsed')

  for (const f2 of fails) console.log(`FAIL  ${f2}`)
  console.log(`${fails.length ? fails.length + ' FAILED' : 'all pass'}  (${ran - fails.length}/${ran} calibration cases)`)
  return fails.length
}

if (flag('--selftest')) {
  process.exit(selftest() ? 1 : 0)
} else {
  await main()
}
