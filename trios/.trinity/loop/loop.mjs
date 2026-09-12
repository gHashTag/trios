#!/usr/bin/env node
// Continuous improvement loop - bookkeeping, idempotency and the dashboard.
//
// The contract this file exists to keep: a new cron fire must never break or
// repeat what a previous fire did. Three mechanisms, in the order they have
// mattered elsewhere in this project:
//
//   1. A LOCK carrying a pid and a start time. A fire that finds a live lock
//      exits and writes nothing. A lock whose pid is gone is a corpse and is
//      taken, with a line in the ledger recording that it was taken.
//   2. An append-only LEDGER. Nothing is ever rewritten, so a crashed fire
//      leaves evidence rather than a hole.
//   3. A DONE SET keyed by a stable hash of the unit of work, so re-running a
//      unit is a no-op and the loop advances instead of spinning.

import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(DIR, '..', '..', '..')
const STATE = path.join(DIR, 'state.json')
const LEDGER = path.join(DIR, 'ledger.jsonl')
const LOCK = path.join(DIR, 'loop.lock')
const DASH = path.join(DIR, 'DASHBOARD.txt')
const DASH_ANSI = path.join(DIR, 'DASHBOARD.ansi')
const DASH_META = path.join(DIR, 'DASHBOARD.meta.json')
const READINGS = path.join(DIR, 'state', 'dash-readings.jsonl')
const LOCK_STALE_MS = 45 * 60 * 1000

// THE CADENCE IS A DECLARATION, NOT A MEASUREMENT. It is what the loop was
// designed around and what the old header asserted as fact for six days after
// the job that honoured it stopped existing. Nothing here believes it: it is
// used only as the yardstick a measured age is compared against, and
// `driverReading()` is what says whether anything is honouring it at all.
export const CADENCE_MS = 15 * 60 * 1000
// Three fires missed is no longer a late fire; it is a stopped loop.
export const STALE_FACTOR = 3

const readJSON = (f, d) => { try { return JSON.parse(fs.readFileSync(f, 'utf8')) } catch { return d } }
const writeJSON = (f, v) => fs.writeFileSync(f, JSON.stringify(v, null, 1) + '\n')

export function loadState() {
  return readJSON(STATE, {
    iteration: 0,
    startedAt: null,
    lastFinishedAt: null,
    title: '',
    done: {},     // unitHash -> {unit, iteration, at}
    doneNone: {}, // iteration -> {reason, at}  an iteration that produced no unit, and said so
    anchors: {},  // name -> {value, at, prev, prevAt}  what a later fire compares against
    lessons: [],
  })
}

export const unitHash = (unit) =>
  crypto.createHash('sha256').update(String(unit)).digest('hex').slice(0, 12)

export const isDone = (unit) => Boolean(loadState().done[unitHash(unit)])

export function markDone(unit, note) {
  const s = loadState()
  s.done[unitHash(unit)] = {
    unit: String(unit).slice(0, 200),
    iteration: s.iteration,
    at: new Date().toISOString(),
    note: note || '',
  }
  writeJSON(STATE, s)
  append({ kind: 'done', unit: String(unit).slice(0, 200), note: note || '' })
}

// Record a measurement and return what it was last time, so a later fire can
// say whether it moved rather than restating it.
//
// AND KEEP THE PREVIOUS READING'S CLOCK. The record already carried `at`, and
// the renderer never read it, so "+3" was printed under a caption reading
// "since the last iteration" across gaps of 2 minutes, 198 minutes and - on
// 2026-09-12 - six days. `at` alone cannot fix that: by the time the dashboard
// is drawn, `at` has been overwritten with the time of THIS reading. So the
// time of the value being compared against is saved beside it as `prevAt`.
//
// The RETURN SHAPE IS UNCHANGED on purpose: a dozen instruments do
// `prev: L.anchor(key, v)` and would all have to change together.
export function anchor(name, value) {
  const s = loadState()
  const prev = s.anchors[name]
  const rec = {
    value,
    at: new Date().toISOString(),
    prev: prev ? prev.value : null,
    prevAt: prev && prev.at ? prev.at : null,
  }
  s.anchors[name] = rec
  writeJSON(STATE, s)
  rememberSpan(name, rec)
  return prev ? prev.value : null
}

export const anchorOf = (name) => (loadState().anchors[name] || {}).value

/** The whole record - value, when it was taken, what it replaced and when. */
export const anchorRecord = (name) => loadState().anchors[name] || null

// What this PROCESS anchored, in the order it anchored it.
//
// The renderer is handed rows that carry a label, a value and a previous value
// - not the anchor key - so it cannot look the span up by name. It can,
// however, remember: `snapshot.mjs` anchors and renders in one process, so
// every row it is about to draw was anchored moments ago by this same module.
// A row matches the first unconsumed entry with the same value AND the same
// previous value. The failure mode is bounded: two rows can only be confused
// when both numbers are identical, and their two writes are milliseconds
// apart, so the span is the same either way. A producer that supplies `prevAt`
// or `key` on the row skips this bridge entirely.
const SPANS = []
function rememberSpan(key, rec) {
  SPANS.push({ key, value: rec.value, prev: rec.prev, prevAt: rec.prevAt, used: false })
  if (SPANS.length > 200) SPANS.splice(0, SPANS.length - 200)
}
const same = (a, b) => (a === b) || (a !== null && b !== null && typeof a === 'object' && typeof b === 'object' && JSON.stringify(a) === JSON.stringify(b))
function takeSpan(value, prev) {
  const hit = SPANS.find((e) => !e.used && same(e.value, value) && same(e.prev, prev))
  if (!hit) return null
  hit.used = true
  return hit
}

export function lesson(text) {
  const s = loadState()
  s.lessons.push({ at: new Date().toISOString(), iteration: s.iteration, text })
  writeJSON(STATE, s)
  append({ kind: 'lesson', note: text })
}

export function append(row) {
  fs.appendFileSync(LEDGER, JSON.stringify({ at: new Date().toISOString(), ...row }) + '\n')
}

/**
 * Quote a script so the LOCAL shell passes it through untouched.
 *
 * This cost two wrong diagnoses before it was understood. `execSync` runs its
 * argument through /bin/sh. If a remote script is wrapped with
 * `JSON.stringify`, it arrives in DOUBLE quotes, and inside double quotes the
 * local shell expands `$base`, `$b`, `$1` and `$(...)` before `railway ssh`
 * ever sees them. The symptoms looked like remote problems and were not:
 *
 *   - `any($1)` reached Postgres as `any()` and failed with "syntax error at
 *     or near )". Diagnosed at the time as the REMOTE shell eating it; it was
 *     the local one.
 *   - A branch survey reported "0 branches with work" against a container that
 *     had 118, because `$base..$b` had already collapsed to `..` locally.
 *
 * Single quotes stop all of it. An embedded single quote is closed, escaped and
 * reopened, which is the only way to get one inside a single-quoted string.
 *
 * A separate trap, unrelated to quoting: a real newline survives this fine, but
 * `JSON.stringify` turns it into a literal backslash-n, so anything still using
 * that path must be one line. With `shq` newlines are safe.
 */
export const shq = (s) => `'` + String(s).replace(/'/g, `'\\''`) + `'`

const alive = (pid) => { try { process.kill(pid, 0); return true } catch { return false } }

// The lock is held for the duration of an ITERATION, and an iteration is a
// Claude turn made of many short-lived processes. So pid liveness is useless
// here: the process that took the lock has always exited by the time the next
// call looks at it, and a pid-liveness check would hand the lock straight to a
// concurrent cron fire. Age is the only sound test. The window is deliberately
// longer than the 15-minute cadence: after a crash it is better to skip two
// fires than to let two of them write at once.
//
// ...AND THE EXCEPTION, WHICH DOES NOT WEAKEN ANY OF THAT.
//
// The paragraph above is right about a lock held by a TURN. It is not the only
// kind of holder. `heal` and `feed` are single processes: they take the lock,
// do their work and exit, all in one pid. For those, a pid that is gone means
// the run is gone, and waiting the full 45 minutes starves the swarm for no
// reason - which cost 8 minutes of an idle swarm on 2026-09-05 before the
// process turned out to be alive after all.
//
// So liveness applies only where it is sound, and only when the holder SAYS it
// is a single process. Two guards on top of that: a grace period, so a run that
// has just started is never stolen from, and the default is the old behaviour,
// so nothing that does not opt in can be affected. A wrongly-held lock costs a
// wait; a wrongly-taken one costs two writers, which is what the lock exists to
// prevent.
const LOCK_LIVENESS_GRACE_MS = 2 * 60 * 1000

export function pidAlive(pid, kill = process.kill) {
  if (!pid || !Number.isInteger(pid)) return true
  try { kill(pid, 0); return true } catch (e) { return e.code !== 'ESRCH' }
}

export function isStale(l, now = Date.now(), alive = pidAlive) {
  if (!l) return true
  const age = now - Date.parse(l.at)
  if (!(age < LOCK_STALE_MS)) return true
  if (!l.singleProcess) return false
  if (age < LOCK_LIVENESS_GRACE_MS) return false
  return !alive(l.pid)
}

export function acquire(holder, opts = {}) {
  if (fs.existsSync(LOCK)) {
    const l = readJSON(LOCK, null)
    const age = l ? Date.now() - Date.parse(l.at) : Infinity
    if (!isStale(l)) return { ok: false, held: l, ageMs: age }
    append({
      kind: 'lock-reclaimed',
      note: l
        ? (l.singleProcess && age < LOCK_STALE_MS
          ? `single-process holder ${l.holder} (pid ${l.pid}) is gone after ${Math.round(age / 1000)}s`
          : `held since ${l.at}, past the ${LOCK_STALE_MS / 60000}m window`)
        : 'unreadable lock',
    })
  }
  writeJSON(LOCK, {
    holder: holder || 'unnamed',
    pid: process.pid,
    at: new Date().toISOString(),
    singleProcess: Boolean(opts.singleProcess),
  })
  return { ok: true }
}

export const release = () => { try { fs.unlinkSync(LOCK) } catch { /* already gone */ } }

/** The lock file as written, whatever its age. For a reader that wants details. */
export function lockRecord() {
  if (!fs.existsSync(LOCK)) return null
  const l = readJSON(LOCK, null)
  if (!l) return null
  const ageMs = Date.now() - Date.parse(l.at)
  return { ...l, ageMs, expired: isStale(l) }
}

/**
 * Who holds the lock right now, or null. Read-only; takes nothing.
 *
 * An EXPIRED lock is not a holder. This returned the record whatever its age,
 * so it and `acquire` answered the same question differently: `lockHolder`
 * said iteration-28 still held the lock while `acquire` was already entitled to
 * reclaim it. The selftest caught the pair disagreeing - it read a holder, took
 * the lock anyway, and reported "took a lock already held by iteration-28".
 *
 * That is worse than cosmetic. `heal.mjs` prints this holder when it stands
 * down, so the chain could name a corpse as the reason it did nothing, and the
 * operator reading that line would go looking for a run that had finished
 * forty-five minutes earlier.
 */
export function lockHolder() {
  const l = lockRecord()
  if (!l || l.expired) return null
  return l
}

// AN ITERATION THAT SAYS NOTHING IS INDISTINGUISHABLE FROM ONE THAT NEVER RAN.
//
// `state.json` reached iteration 96 with a `done` register that stopped at 25:
// 45 entries, every one of them from 2026-09-04, while the counter kept
// climbing for another 71 turns. The register is honest about what it recorded
// - all five units sampled from it still exist in the tree today - it simply
// stopped being written, and nothing noticed, because nothing ever asked.
//
// So the counter may not move past an iteration that recorded neither a unit
// nor a reason for having none. `markNone(reason)` is the second door and it is
// deliberately cheap to walk through: the point is not to force work, it is to
// force a SENTENCE, so that a silent iteration and an iteration that never
// happened stop looking the same in the record.

/** Did iteration `n` record either a unit of work or an explicit "none"? */
export function iterationRecorded(s = loadState(), n = s.iteration) {
  if (!n) return true // iteration 0 is the state before the first fire
  if (Object.values(s.done || {}).some((d) => d && d.iteration === n)) return true
  return Boolean((s.doneNone || {})[String(n)])
}

/** Close an iteration that produced no unit, on the record, with a reason. */
export function markNone(reason, iteration) {
  const why = String(reason || '').trim()
  if (!why) throw new Error('markNone(reason) needs a reason - "none" without one is the silence it exists to replace')
  const s = loadState()
  const n = iteration === undefined ? s.iteration : iteration
  s.doneNone = s.doneNone || {}
  s.doneNone[String(n)] = { reason: why.slice(0, 400), at: new Date().toISOString() }
  writeJSON(STATE, s)
  append({ kind: 'done-none', iteration: n, note: why.slice(0, 400) })
  return n
}

/**
 * Start the next iteration.
 *
 * Refuses while the current one has written nothing. `opts.noUnit` is the
 * explicit way through - it records the none, with its reason, for the
 * iteration being left behind, and then advances.
 */
export function beginIteration(title, opts = {}) {
  let s = loadState()
  if (!iterationRecorded(s, s.iteration)) {
    if (opts.noUnit) {
      markNone(opts.noUnit, s.iteration)
      s = loadState()
    } else {
      append({ kind: 'begin-refused', iteration: s.iteration, note: 'no done entry and no explicit none for this iteration' })
      throw new Error(
        `iteration ${s.iteration} recorded neither a unit nor a reason, so ${s.iteration + 1} may not start.\n` +
        `  markDone(unit, note)                       - it produced something\n` +
        `  markNone('why there was no unit')          - it did not, and here is why\n` +
        `  beginIteration(title, { noUnit: 'why' })   - both, in one call`
      )
    }
  }
  s.iteration += 1
  s.startedAt = new Date().toISOString()
  s.title = title || ''
  writeJSON(STATE, s)
  append({ kind: 'begin', iteration: s.iteration, title: title || '' })
  return s.iteration
}

export function endIteration(summary) {
  const s = loadState()
  s.lastFinishedAt = new Date().toISOString()
  writeJSON(STATE, s)
  const recorded = iterationRecorded(s, s.iteration)
  append({ kind: 'end', iteration: s.iteration, recorded, ...(summary || {}) })
  // Said here, where it can still be fixed, rather than at the next `begin`
  // where it becomes a refusal.
  if (!recorded) {
    process.emitWarning(
      `iteration ${s.iteration} is closing with no done entry and no explicit none - ` +
      `the next beginIteration() will refuse until one is written`
    )
  }
  return s.iteration
}

// -------------------------------------------------------------- the dashboard

const ESC = String.fromCharCode(27)
const sgr = (code) => (t) => `${ESC}[${code}m${t}${ESC}[0m`
const dim = sgr(2), bold = sgr(1), green = sgr(32), red = sgr(31)
const yellow = sgr(33), cyan = sgr(36)

const W = 76
const strip = (t) => t.replace(new RegExp(ESC + '\\[[0-9;]*m', 'g'), '')
const pad = (t, w) => t + ' '.repeat(Math.max(0, w - strip(t).length))
const lpad = (t, w) => ' '.repeat(Math.max(0, w - strip(t).length)) + t

/**
 * Cut to a VISIBLE width, colour codes not counted and never left dangling.
 *
 * `pad` only ever pads. Anything wider than the box therefore ran straight
 * through the right border, and every row of the last dashboard's WORK section
 * did: the frame was broken by the content it was drawn around. The subject row
 * had already learned this and fixed it with a `.slice()` for itself alone,
 * which is why the lesson did not reach the four sections written afterwards.
 *
 * Fixing it in `row` instead means no future section can reintroduce it, and
 * the callers stop carrying magic numbers that have to agree with W by hand.
 */
const clip = (t, w) => {
  if (strip(t).length <= w) return t
  let visible = 0
  let out = ''
  for (let i = 0; i < t.length; i++) {
    const esc = t.slice(i).match(new RegExp('^' + ESC + '\\[[0-9;]*m'))
    if (esc) { out += esc[0]; i += esc[0].length - 1; continue }
    if (visible >= w - 1) break
    out += t[i]
    visible++
  }
  // The ellipsis says the row was cut; the reset makes sure a colour opened
  // before the cut cannot bleed into the border.
  return `${out}…${ESC}[0m`
}

const row = (t = '') => `│ ${pad(clip(t, W - 3), W - 3)}│`
// THE BOX WAS NEVER SQUARE. A `row` is 2 + (W-3) + 1 = W characters; the rules
// were `'─'.repeat(W - 1)` between two corners, so every horizontal line was
// W+1 - one character past the vertical borders it was supposed to meet. It has
// looked like that in every dashboard this loop has drawn, and it was found by
// the calibration case written for a different defect in the same function.
const rule = () => `├${'─'.repeat(W - 2)}┤`
const top = (t) => `╭─ ${t} ${'─'.repeat(Math.max(0, W - 5 - strip(t).length))}╮`
const bottom = () => `╰${'─'.repeat(W - 2)}╯`

// ------------------------------------------- what the box has to measure first

/** A span, spoken the way it is read. `?` when there is nothing to measure. */
export function humanAge(ms) {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return '?'
  if (ms < -60000) return 'future'
  const m = Math.max(0, Math.floor(ms / 60000))
  if (m < 60) return `${m}m`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ${m % 60}m`
  return `${Math.floor(h / 24)}d ${h % 24}h`
}

/**
 * Run a command and keep its output whatever the exit code.
 *
 * `crontab -l` exits 1 when there is no crontab, and that IS the answer.
 * Only a signal or a timeout means nothing was measured.
 */
function shRead(cmd, timeout = 8000) {
  try {
    return execSync(cmd, { cwd: DIR, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout }).trim()
  } catch (e) {
    if (e.killed || e.signal || e.code === 'ETIMEDOUT') throw e
    const out = String(e.stdout || '').trim()
    if (!out) throw e
    return out
  }
}

// WHAT, IF ANYTHING, FIRES AN ITERATION - read, not remembered, and read in
// ONE place. The header of this dashboard named a 15-minute cron and a job id
// for six days after that job stopped existing, because the string was a
// LITERAL in this file; a literal cannot report its own absence. The reading
// that replaced it was then written out twice, here and in `sense.mjs`, with a
// comment in this file claiming the two "agree by construction". They agreed
// by transcription. Both are now `driver.mjs`, and two of its zeros turned out
// to be asserted rather than measured - see the header there.
// Imported, not just re-exported: two callers below use the name locally, and
// `export ... from` would leave it undefined in this scope.
import { driverReading } from './driver.mjs'
export { driverReading }

/** The last `bytes` of a file, without reading the rest of it. */
function tailOf(file, bytes) {
  const fd = fs.openSync(file, 'r')
  try {
    const size = fs.fstatSync(fd).size
    const start = Math.max(0, size - bytes)
    const buf = Buffer.alloc(size - start)
    fs.readSync(fd, buf, 0, buf.length, start)
    return buf.toString('utf8')
  } finally { fs.closeSync(fd) }
}

/**
 * When did an iteration last START - from the ledger, which cannot be edited
 * in place, falling back to the state file, which can.
 *
 * The token `"begin"` is only a cheap pre-filter here; the row is parsed and
 * its `kind` is what decides, so `begin-refused` and the word begin inside a
 * note can never answer this question.
 */
export function lastBeginAt(s = loadState()) {
  try {
    const lines = tailOf(LEDGER, 1 << 20).split('\n')
    for (let i = lines.length - 1; i >= 0; i--) {
      const l = lines[i].trim()
      if (!l.startsWith('{') || !l.includes('"begin"')) continue
      let j
      try { j = JSON.parse(l) } catch { continue }
      if (j && j.kind === 'begin' && j.at) return { at: j.at, src: 'ledger' }
    }
  } catch { /* no ledger, or unreadable - the state file is the fallback */ }
  return s.startedAt ? { at: s.startedAt, src: 'state.json' } : { at: null, src: null }
}

/**
 * How many times was the NEXT iteration actually turned away while `n` was the
 * current one, and when was the first refusal?
 *
 * AN UNRECORDED ITERATION IS NORMAL; A REFUSED ONE IS NOT. Every iteration
 * spends its whole working life with nothing in the register - the unit is
 * recorded at the end, which is the point of recording it. So "has this
 * iteration recorded a unit" is the wrong question to paint red on a dashboard:
 * the answer is no for the first minute of every iteration that ever runs, and
 * a warning that is always on is not a warning. The first draft of the wedge
 * line did exactly that, and the very next `beginIteration` proved it by
 * lighting up two minutes into a healthy iteration 97.
 *
 * The event that separates the two cases is measured and already in the ledger:
 * `beginIteration` appends a `begin-refused` row every time it turns someone
 * away. Nobody tried to advance past a fresh iteration, so there is no row.
 * Iteration 96 has one, from 2026-09-12T16:46:14, six days after it closed.
 *
 * Returns null when the ledger cannot be read - which is not zero refusals, it
 * is no reading. `text` is for the test, so the branch can be exercised without
 * writing rows into the ledger the loop is appending to.
 */
export function beginRefusals(n, text) {
  let lines
  try {
    lines = (text === undefined ? tailOf(LEDGER, 1 << 20) : text).split('\n')
  } catch {
    return null
  }
  if (text === undefined && !fs.existsSync(LEDGER)) return null
  let count = 0
  let first = null
  for (const l of lines) {
    const t = l.trim()
    // A cheap pre-filter only. The row is parsed and its `kind` decides, so the
    // words "begin-refused" inside a note can never be counted as a refusal.
    if (!t.startsWith('{') || !t.includes('begin-refused')) continue
    let j
    try { j = JSON.parse(t) } catch { continue }
    if (!j || j.kind !== 'begin-refused' || j.iteration !== n) continue
    count += 1
    if (!first && j.at) first = j.at
  }
  return { count, first }
}

// A RATE IS NOT A COUNT, AND A COLLAPSING SAMPLE IS NOT AN IMPROVEMENT.
//
// `send-backs looping with no ceiling 79` fell to 5 and would have been drawn
// in green as a 74-point win. The sample behind it fell from 158 issues
// examined to 22 - the RATE went 50% to 23% - and neither denominator appeared
// anywhere on the box. So the denominator is printed under the metric, and a
// fall measured against a sample that shrank is not painted as a win.
//
// The keys are exact labels compared with `===`, never patterns. A producer
// that rewords a label loses its denominator line and prints nothing, which is
// the right direction to fail in; inventing one is not.
const RATIO_SAMPLES = {
  'send-backs looping with no ceiling':
    { field: 'looping', value: 'looping', of: 'examined', unit: 'examined', word: 'of' },
  'rows where one rule answers two ways':
    { field: 'divergent', value: 'rows', of: 'pairs', unit: 'pairs compared', word: 'from' },
}

function readings(file = READINGS) {
  try {
    return fs.readFileSync(file, 'utf8').trim().split('\n').filter(Boolean)
      .map((l) => { try { return JSON.parse(l) } catch { return null } })
      .filter((r) => r && r.at)
  } catch { return [] }
}

/**
 * The denominator behind a ratio-derived row, and the clock on it.
 *
 * THE JOIN IS VERIFIED BEFORE IT IS TRUSTED: a reading only supplies the
 * denominator if its own numerator equals the value being drawn. A reading
 * that does not match is a reading of something else, and returns nothing.
 */
export function sampleFromReadings(label, value, prev, prevAt, all) {
  const spec = RATIO_SAMPLES[label]
  if (!spec || !isMeasured(value)) return null
  const rows = all || readings()
  const at = (r) => Date.parse(r.at)
  const pick = (n, before) => {
    for (let i = rows.length - 1; i >= 0; i--) {
      const r = rows[i]
      const f = r[spec.field]
      if (!f || Number(f[spec.value]) !== Number(n)) continue
      if (before && at(r) > Date.parse(before)) continue
      return r
    }
    return null
  }
  const cur = pick(countOf(value))
  if (!cur) return null
  const was = isMeasured(prev) && prevAt ? pick(countOf(prev), prevAt) : null
  return {
    now: cur[spec.field][spec.of],
    prev: was ? was[spec.field][spec.of] : null,
    unit: spec.unit,
    word: spec.word,
    at: cur.at,
  }
}

// AN UNMEASURED VALUE HAS NO DELTA, and pretending otherwise invented two
// numbers at once.
//
// `dash.mjs` returns null for a fact it could not take, and the renderer had
// never been taught what that means. On 2026-09-05 the local disk hit 100%, the
// verdict audit could not write its cache, and the dashboard printed
// `judged verdicts that prove   null   -203` - the word "null" as a value and a
// fabricated fall as its change, in the artifact built three rounds earlier for
// the sole purpose of not making numbers up.
/**
 * A COUNT INSIDE AN ENVELOPE IS STILL A COUNT.
 *
 * The Queen's skip summary changed shape on 2026-09-06 from `7` to
 * `{count: 7, issues: [...], more: 0}`. `Number({...})` is NaN, so a measured
 * seven became `-` - the same mark this box uses for "nobody looked" - and it
 * has drawn `fenced by parked paths  -` ever since while the real answer sat
 * one field inside the value. Unwrapping it here means a future shape change
 * downgrades to something visible rather than to silence.
 *
 * An object WITHOUT a count is still unmeasured, and that now includes an
 * array: `Number([])` is 0, so an empty list used to render as a real zero.
 */
export function countOf(v) {
  if (v !== null && typeof v === 'object') {
    return Object.prototype.hasOwnProperty.call(v, 'count') ? v.count : NaN
  }
  return v
}

export function isMeasured(v) {
  const n = countOf(v)
  return !(n === null || n === undefined || n === '' || Number.isNaN(Number(n)))
}

/**
 * The change, AND THE SPAN IT COVERS.
 *
 * `+3` was printed under a caption reading "since the last iteration" over
 * gaps of 2 minutes, 198 minutes and six days, because this function took no
 * clock. It takes one now: `prevAt` is when the value being compared against
 * was measured, and the span is printed beside the arrow so the number can be
 * read for what it is.
 *
 * Past three missed fires the arrow is withdrawn entirely - a dim `?` with the
 * span beside it - because a difference measured across six days of a stopped
 * loop is not a movement anyone can act on, and drawing it in green or red
 * claims it is.
 */
function delta(now, prev, prevAt, goodDown, opts = {}) {
  if (!isMeasured(now)) return dim('    ')
  const clock = opts.at || Date.now()
  const spanMs = prevAt ? clock - Date.parse(prevAt) : null
  const span = dim(' / ' + (spanMs === null || Number.isNaN(spanMs) ? '?' : humanAge(spanMs)))
  if (prev === null || prev === undefined || Number.isNaN(Number(countOf(prev)))) return dim(lpad('new', 4)) + span

  // The companion sample, when there is one: a fall of 74 against a sample that
  // fell 86% is a smaller sample, not a smaller problem.
  const sample = opts.sample
  let shrank = null
  if (sample && isMeasured(sample.now) && isMeasured(sample.prev) && Number(sample.prev) > 0) {
    const pct = Math.round(((Number(sample.now) - Number(sample.prev)) / Number(sample.prev)) * 100)
    if (pct < -10) shrank = dim(` sample ${pct}%`)
  }

  const d = Number(countOf(now)) - Number(countOf(prev))
  const arrow = d === 0 ? '=' : (d > 0 ? '+' : '') + d
  if (spanMs === null || Number.isNaN(spanMs) || spanMs > STALE_FACTOR * CADENCE_MS) {
    return dim(lpad('?', 4)) + span + (shrank || '')
  }
  if (d === 0) return dim(lpad('=', 4)) + span
  if (shrank) return dim(lpad(arrow, 4)) + span + shrank
  const good = goodDown ? d < 0 : d > 0
  return (good ? green : red)(lpad(arrow, 4)) + span
}

// THE ONE LABEL WIDTH. It was two numbers: this 34, which the renderer clips
// to, and `LABEL_MAX = 55` in snapshot.mjs, which the producers sliced to. The
// producers therefore built labels 21 columns longer than anything that could be
// drawn, and a calibration case asserted `length <= 55` - so it passed at the
// exact moment the renderer was cutting the line. Two constants for one quantity
// is the defect this repository keeps finding in other people's code (the
// boundary rule in three copies, `can_start_another` in five). Exported so there
// is one.
export const LABEL_W = 34
const VALUE_W = 10

/**
 * THE DASHBOARD SAYS WHEN IT WAS DRAWN AND WHAT IS DRIVING IT.
 *
 * `DASHBOARD.txt` was six days and four hours old, said nothing about its own
 * age, and asserted a scheduler that three registries denied. Nothing on it
 * was falsified; it simply had no way to say "this is old", so every figure
 * borrowed the authority of the freshest one. Two lines fix that, and both are
 * measured at draw time: when this render happened, and how long ago an
 * iteration last began.
 */
export function renderDashboard(facts, to = {}) {
  // `to.state` for the same reason as `to.driver`: the wedge line below has two
  // branches and only one of them is true today, so without injection the other
  // could only be proven by writing a hostile state.json into the tree the loop
  // is reading. An untested branch is not a working branch.
  const s = to.state || loadState()
  const clock = to.at || Date.now()
  const begin = lastBeginAt(s)
  const sinceBegin = begin.at ? clock - Date.parse(begin.at) : null
  const stale = sinceBegin === null || sinceBegin > STALE_FACTOR * CADENCE_MS
  // Injectable so a calibration case can render a hostile fixture without
  // shelling out, and so the probe is the only thing that ever names a driver.
  const drv = to.driver || driverReading()
  const all = readings()
  const out = []

  const n = (v) => (v === null || v === undefined ? '-' : String(v))
  const driverWord = drv.claudeCron === null ? '?' : drv.claudeCron === 0 ? 'GONE' : `${drv.claudeCron} claude-cron`
  const driverPaint = drv.claudeCron ? green : red
  const headline = bold('TRIOS CONTINUOUS LOOP') + dim('   driver ') + driverPaint(driverWord)
  out.push(top(clip(stale ? red(strip(headline)) : headline, W - 5)))
  out.push(row(
    `${dim('rendered')}   ${new Date(clock).toISOString().slice(0, 19)}Z   ` +
    `${dim('last begin')} ${begin.at ? humanAge(sinceBegin) + ' ago' : '-'}` +
    (stale ? `   ${red(`STALE > ${STALE_FACTOR}x ${CADENCE_MS / 60000}m`)}` : '')
  ))
  out.push(row(
    `${dim('driver')}     ${driverPaint(driverWord)}   ` +
    dim(`claude-cron ${n(drv.claudeCron)}  crontab ${n(drv.crontab)}  launchd ${n(drv.launchd)}`) +
    (drv.claudeCron === 0 ? dim('  nothing fires an iteration') : '')
  ))
  // AND WHETHER THE COUNTER CAN STILL MOVE.
  //
  // Iteration 96 closed on 2026-09-06 having written neither a unit nor an
  // explicit none, which means `beginIteration` must refuse 97 - by design, and
  // the design is right. What was wrong is that NOTHING SAID SO. For six days
  // this box drew `#96` in bold beside a start time, while feed, heal and land
  // went on appending rows underneath it, and the loop looked like a loop.
  //
  // There was a warning. `endIteration` calls `process.emitWarning` when it
  // closes an unrecorded iteration - to a stderr nobody reads, from a timer
  // nobody watches. It has also never once run: it was added beside a
  // `recorded` field on the `end` row, and 0 of the 98 `end` rows in the ledger
  // carry that field, because no iteration has closed since it was written. A
  // warning that has never executed is not a warning; it is a comment with a
  // function call in it.
  //
  // So the refusal is stated here, on the face, where the operator looks - but
  // only once it IS a refusal. See `beginRefusals`: an iteration with nothing in
  // the register is every iteration for its whole working life, and the first
  // draft of this line painted that red. What is worth red is a `begin-refused`
  // row: something tried to advance the counter and was turned away.
  const ref = to.refusals === undefined ? beginRefusals(s.iteration) : to.refusals
  const wedged = !iterationRecorded(s, s.iteration) && ref && ref.count > 0
  out.push(row(`${dim('iteration')}  ${bold('#' + s.iteration)}    ${dim('started')} ${(s.startedAt || '-').slice(0, 19)}Z`))
  // Its own row, not a suffix: at W=76 a suffix carrying both the count and the
  // date is clipped, and the first thing clipped is the date - which is the half
  // that says whether this happened once an hour ago or has been happening for
  // six days. The gate below asserts the date, and it caught exactly that.
  if (wedged) {
    out.push(row(`           ${red(`#${s.iteration + 1} REFUSED x${ref.count}`)}${dim(` - first refused ${(ref.first || '-').slice(0, 16)}Z`)}`))
  }
  out.push(row(`${dim('subject')}    ${s.title || '-'}`))
  out.push(rule())
  out.push(row(bold('SWARM') + dim('   value | change / the span it covers | a row says if it is older')))
  for (const raw of facts.swarm || []) {
    // `{count: N}` is a measurement wearing an envelope; the column shows the
    // number, not `[object Object]` and not the `-` that means nobody looked.
    const m = { ...raw, v: countOf(raw.v), prev: countOf(raw.prev) }
    // `-` for a fact that could not be taken, never the word "null" and never a
    // zero standing in for it.
    const shown = isMeasured(m.v) ? String(m.v) : '-'
    // When the value was last compared: from the row, from its anchor key, or
    // from what this process anchored moments ago. Never assumed.
    let prevAt = raw.prevAt || null
    if (!prevAt && raw.key) prevAt = (s.anchors[raw.key] || {}).prevAt || null
    if (!prevAt) { const hit = takeSpan(raw.v, raw.prev); prevAt = hit ? hit.prevAt : null }
    const sample = raw.sample || sampleFromReadings(m.k, m.v, m.prev, prevAt, all)
    out.push(row(`  ${pad(dim(clip(m.k, LABEL_W)), LABEL_W)} ${pad(bold(shown), VALUE_W)} ${delta(m.v, m.prev, prevAt, m.goodDown !== false, { sample, at: clock })}`))

    // The denominator, and the clock on the reading it came from - under the
    // metric rather than squeezed into it, so neither has to be abbreviated.
    const notes = []
    // THE GLOSS IS A NOTE, NOT PART OF THE LABEL. It used to be concatenated on
    // to `k` by the producer and then clipped to 34 here, which rendered
    // `missingBoundary 92.2%  no bound...` - the share survived and the sentence
    // explaining it did not. A row that says a thing is 92.2% of everything and
    // then cuts off what the thing IS has spent its column on the part a reader
    // could already guess. The label column cannot simply be widened: the delta
    // can reach ~28 columns with a `sample` suffix, and 2+34+1+10+1+28 already
    // exceeds the 74 columns inside the box. So the gloss goes where this file
    // has always put what will not fit - under the metric, per the comment below,
    // unabbreviated.
    if (raw.gloss) notes.push(dim(raw.gloss))
    if (sample && isMeasured(sample.now)) {
      const was = isMeasured(sample.prev) && Number(sample.prev) !== Number(sample.now)
        ? ` (was ${sample.prev})`
        : ''
      notes.push(dim(`${sample.word || 'of'} ${sample.now} ${sample.unit || 'sampled'}${was}`))
    }
    const takenAt = raw.at || (sample && sample.at) || null
    if (takenAt) {
      const age = clock - Date.parse(takenAt)
      if (Number.isFinite(age) && age > CADENCE_MS) notes.push(red(`measured ${humanAge(age)} ago - STALE`))
    }
    if (notes.length) out.push(row('     ' + notes.join(dim('   '))))
  }
  out.push(rule())
  out.push(row(bold('WORK THIS ITERATION')))
  for (const t of facts.work || []) {
    const mark = { done: green('●'), blocked: red('●'), running: yellow('●') }[t.state] || dim('○')
    // The note is what the title is worth: a title long enough to crowd it out
    // keeps the note, and `clip` in `row` decides where the line ends.
    out.push(row(`  ${mark} ${pad(clip(t.title, W - 26), W - 26)} ${dim(clip(t.note || '', 20))}`))
  }
  if ((facts.anomalies || []).length) {
    out.push(rule())
    out.push(row(bold(red('ANOMALIES')) + dim('   found by this iteration, against its own work')))
    for (const a of facts.anomalies) out.push(row(`  ${red('!')} ${a}`))
  }
  if ((facts.next || []).length) {
    out.push(rule())
    // The heading counts what is actually there. It said THREE unconditionally,
    // and a list of four printed under it - a caption contradicted by the rows
    // directly beneath it, which is the smallest possible version of the defect
    // this whole loop exists to hunt.
    const n = facts.next.length
    const word = { 1: 'ONE WAY', 2: 'TWO WAYS', 3: 'THREE WAYS', 4: 'FOUR WAYS', 5: 'FIVE WAYS' }[n] || `${n} WAYS`
    out.push(row(bold(`${word} TO CONTINUE`)))
    facts.next.forEach((n, i) => out.push(row(`  ${cyan(i + 1 + '.')} ${n}`)))
  }
  out.push(bottom())
  const text = out.join('\n')
  // The destinations are arguments so a calibration case can render a hostile
  // fixture WITHOUT overwriting the round's real dashboard. A suite in this
  // repository once truncated a shipped file to zero bytes by testing against
  // the live artifact; the rule since then is that a test never writes where
  // production reads.
  fs.writeFileSync(to.ansi || DASH_ANSI, text + '\n')
  fs.writeFileSync(to.text || DASH, strip(text) + '\n')
  // A consumer should not have to parse a drawn box to find out how old it is.
  // Written only beside the artifact it describes: a calibration case that
  // redirects the render must not leave production's freshness record behind.
  const metaPath = to.meta || (to.text || to.ansi ? null : DASH_META)
  if (metaPath) {
    fs.writeFileSync(metaPath, JSON.stringify({
      renderedAt: new Date(clock).toISOString(),
      iteration: s.iteration,
      lastBeginAt: begin.at,
      lastBeginSource: begin.src,
      sinceLastBeginMs: sinceBegin,
      sinceLastBegin: humanAge(sinceBegin),
      cadenceMs: CADENCE_MS,
      staleFactor: STALE_FACTOR,
      stale,
      driver: drv,
    }, null, 1) + '\n')
  }
  return text
}

// ------------------------------------------------------------------------ cli

// GATED ON isMain, and this one was dangerous.
//
// Every other tool imports this file. It read `process.argv[2]` at module
// scope and dispatched on it, so ANY tool invoked with `unlock` as its first
// argument released the loop lock - the exact protection the lock exists to
// provide, removed by an argument meant for something else. Proven, not
// supposed: an importer run as `probe.mjs unlock` printed "lock released" and
// the lock went free.
//
// Found by a guard written minutes earlier for this same class, after
// brief-gate was caught doing a milder version of it.
const isMain = process.argv[1] && process.argv[1].endsWith('/loop.mjs')
const cmd = isMain ? process.argv[2] : undefined
if (cmd === 'status') {
  const s = loadState()
  console.log(`iteration ${s.iteration} | started ${s.startedAt || '-'} | finished ${s.lastFinishedAt || '-'}`)
  console.log(`done units ${Object.keys(s.done).length} | anchors ${Object.keys(s.anchors).length} | lessons ${s.lessons.length}`)
  // THE TWO REGISTERS, SIDE BY SIDE. They were 71 iterations apart and nothing
  // ever printed them together, which is how it stayed unnoticed.
  {
    const its = Object.values(s.done || {}).map((d) => d && d.iteration).filter((n) => typeof n === 'number')
    const none = Object.keys(s.doneNone || {}).map(Number)
    const last = Math.max(0, ...its, ...none)
    console.log(`done register: last recorded iteration ${last || '-'} of ${s.iteration}` +
      (iterationRecorded(s, s.iteration) ? '' : `   <- iteration ${s.iteration} has recorded nothing; the next begin will refuse`))
  }
  if (fs.existsSync(LOCK)) {
    const l = readJSON(LOCK, {})
    const mins = Math.round((Date.now() - Date.parse(l.at)) / 60000)
    const held = mins < LOCK_STALE_MS / 60000
    console.log(`LOCK ${l.holder || '?'} since ${l.at} (${mins}m) - ${held ? 'HELD' : 'stale, reclaimable'}`)
    console.log(`  (the pid ${l.pid} is ${alive(l.pid) ? 'alive' : 'gone'}, which is expected and not what decides the lock)`)
  } else console.log('LOCK free')
} else if (cmd === 'dash') {
  // `tri dash` prints a FILE. The file said nothing about its own age for six
  // days, so the reader is told here too, from the artifact's mtime - which is
  // measured, not remembered.
  if (!fs.existsSync(DASH_ANSI)) process.stdout.write('no dashboard yet\n')
  else {
    process.stdout.write(fs.readFileSync(DASH_ANSI, 'utf8'))
    const age = Date.now() - fs.statSync(DASH_ANSI).mtimeMs
    const note = `this file was drawn ${humanAge(age)} ago - re-render with: tri snapshot`
    process.stdout.write((age > CADENCE_MS ? red(note) : dim(note)) + '\n')
  }
} else if (cmd === 'driver') {
  const d = driverReading()
  const n = (v) => (v === null || v === undefined ? '-' : String(v))
  console.log(`claude-cron ${n(d.claudeCron)}   crontab ${n(d.crontab)}   launchd ${n(d.launchd)}   read ${d.at}`)
  console.log(d.claudeCron === 0
    ? 'driver GONE - nothing schedules an iteration'
    : d.claudeCron === null ? 'driver UNMEASURED - .claude/scheduled_tasks.json could not be read'
    : `driver ${d.claudeCron} claude-cron task(s)`)
  if (d.claudeCron === 0) process.exitCode = 1
} else if (cmd === 'ledger') {
  const n = Number(process.argv[3] || 20)
  const rows = fs.existsSync(LEDGER) ? fs.readFileSync(LEDGER, 'utf8').trim().split('\n').filter(Boolean).slice(-n) : []
  for (const r of rows) {
    const j = JSON.parse(r)
    console.log(`${j.at.slice(5, 16)}  ${pad(j.kind, 16)} ${(j.title || j.unit || j.note || '').slice(0, 70)}`)
  }
} else if (cmd === 'unlock') {
  release()
  console.log('lock released')
} else if (cmd === 'anchors') {
  const a = loadState().anchors
  // The span, not just the two values: `79 prev 79` says nothing about whether
  // that was fifteen minutes of stability or six days of a stopped loop.
  for (const [k, v] of Object.entries(a)) {
    const show = (x) => (isMeasured(x) ? String(countOf(x)) : x === null || x === undefined ? '-' : `${JSON.stringify(x).slice(0, 18)}?`)
    const span = v.prevAt ? humanAge(Date.parse(v.at) - Date.parse(v.prevAt)) : '?'
    console.log(`${pad(k, 34)} ${pad(show(v.value), 10)} prev ${pad(show(v.prev), 10)} over ${pad(span, 8)} @ ${v.at.slice(5, 16)} (${humanAge(Date.now() - Date.parse(v.at))} ago)`)
  }
}
