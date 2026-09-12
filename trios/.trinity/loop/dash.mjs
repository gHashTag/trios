#!/usr/bin/env node
// The dashboard's numbers, measured rather than typed.
//
// WHY THIS EXISTS. Every instrument in this directory measures something and
// refuses to guess. The dashboard did not: `renderDashboard` takes whatever
// numbers its caller hands it, and its caller was me, at three in the morning,
// typing from memory.
//
// On 2026-09-05 I wrote "dispatches finished 258" into iteration #46. The last
// measurement had said 255. Nothing was wrong with the swarm; the number was
// simply invented, in the one artifact whose whole job is to say what is true.
// It went out in a report.
//
// So the facts are gathered here, by asking the same tools everything else asks.
// A fact that cannot be measured comes back null and renders as `-`. It is never
// filled in from memory, and there is no argument by which it could be.
//
// THE PROSE STAYS HAND-WRITTEN. What was done this round, what went wrong, what
// to do next - those are judgements and belong to whoever writes them. The rule
// is narrower and absolute: **numbers are measured, prose is written.**
//
// Usage:
//   node dash.mjs --facts      # what it can measure right now, as JSON
//   node dash.mjs              # the same, as the lines the dashboard shows

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import * as T27 from './t27-parity.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const isMain = process.argv[1] && process.argv[1].endsWith('/dash.mjs')

/**
 * Run a measurement, and return null if it could not be taken.
 *
 * null is a real answer here and the important one: the renderer prints `-` for
 * it. An unmeasurable fact that silently became 0 would be worse than the typed
 * number this file exists to replace.
 */
export function measure(fn) {
  try {
    const v = fn()
    return v === undefined ? null : v
  } catch { return null }
}

/**
 * A NON-ZERO EXIT IS OFTEN THE ANSWER, NOT A FAILURE.
 *
 * `failures.mjs` exits 2 when a step is failing more than a quarter of the time
 * - that is the tool working - and `reap-local` exits 1 to mean "would act".
 * Reading either as an error made the dashboard print `-` for two facts it had
 * just successfully measured, which is the same defect as inventing a number,
 * only quieter.
 *
 * So the output is taken whatever the exit code, and only a signal or a timeout
 * counts as not having measured.
 */
const sh = (cmd, timeout = 120000) => {
  try {
    return execSync(cmd, { cwd: DIR, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout }).trim()
  } catch (e) {
    if (e.killed || e.signal || e.code === 'ETIMEDOUT') throw e
    const out = String(e.stdout || '').trim()
    if (!out) throw e
    return out
  }
}

/**
 * THE FRESHNESS RULE. A read costs nothing, however old the record is.
 *
 * Four rows below are READ from a record another instrument writes, and reading
 * one is free whether it was written a minute ago or a week ago.
 * `proven-history.jsonl` held two lines written on 2026-09-05, and on 2026-09-12
 * the dashboard printed their numbers with `+0` beside them. The `+0` was not a
 * bug: the anchor held the same frozen numbers, so the delta was honestly zero.
 * That is the trap - a writer that has STOPPED is indistinguishable from a
 * metric that is STABLE, and the two look identical for as long as nobody puts
 * the age on the screen.
 *
 * ONE HOUR, MEASURED RATHER THAN CHOSEN. The same heal chain writes both
 * records. Over the 297 samples in the two-views record its real spacing is a
 * 23.5-minute median with a 42.6-minute ninth decile, because the chain skips a
 * fire whenever the lock is held and its own pass takes seven to ten minutes. A
 * fifteen-minute threshold would be red on a healthy loop, and a threshold that
 * cries on healthy machinery is read as decoration within a week. An hour is the
 * first one that means something. `TRIOS_CADENCE_SECONDS` overrides it.
 */
export const CADENCE_SECONDS = Number(process.env.TRIOS_CADENCE_SECONDS || 3600)

/** Milliseconds since an ISO stamp - null when the record carries no clock. */
export function ageMs(at, now = Date.now()) {
  const t = Date.parse(at || '')
  return Number.isFinite(t) ? Math.max(0, now - t) : null
}

/**
 * An age a reader can hold in their head.
 *
 * `age unknown` for null, and never `0m old`: a record with no timestamp has not
 * been shown to be fresh, and printing it as brand new would be the same defect
 * as printing an unmeasured fact as 0.
 */
export function ageWords(ms) {
  if (ms === null || ms === undefined) return 'age unknown'
  const m = Math.round(ms / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m old`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ${m % 60}m old`
  return `${Math.floor(h / 24)}d ${h % 24}h old`
}

/** Older than one cadence: the writer should have run by now and did not. */
export function isStale(ms, cadenceSeconds = CADENCE_SECONDS) {
  return ms !== null && ms !== undefined && ms > cadenceSeconds * 1000
}

/** Every JSON line of a record, skipping whatever does not parse. */
function jsonl(read, file) {
  return read(file)
    .split('\n').filter(Boolean)
    .map((l) => { try { return JSON.parse(l) } catch { return null } })
    .filter(Boolean)
}

const PROVEN_RECORD = ['state', 'proven-history.jsonl']
const PAIRED_RECORD = ['state', 'two-views.jsonl']
const readFile = (f) => fs.readFileSync(f, 'utf8')

/** running / finished, from the swarm's own board. */
export function swarmCounts(run = sh) {
  const out = run(`${JSON.stringify(path.join(process.env.HOME || '', '.local/bin/tri'))} swarm`, 200000)
  const m = out.match(/running\s+(\d+)\s+finished\s+(\d+)/)
  return m ? { running: Number(m[1]), finished: Number(m[2]) } : null
}

/**
 * Send-backs that are re-attempted while their retry ceiling never moves.
 *
 * An attempt is charged only when a criterion was tested and FAILED, so a bee
 * that goes silent is deliberately not charged (#1420 FR-003) and work nobody
 * assessed never reaches a person. A bee silent EVERY time therefore loops with
 * no ceiling at all, and this is the count of issues in that state.
 *
 * `silent-loop.mjs` exits 2 when it finds any, which is precisely when this row
 * matters - `sh` above returns the output whatever the exit code, so the fact
 * is not lost at the moment it becomes interesting.
 */
export function loopingCounts(run = sh) {
  const out = run(`node ${path.join(DIR, 'silent-loop.mjs')}`, 200000)
  const m = out.match(/(\d+) examined, (\d+) looping with no ceiling/)
  return m ? { examined: Number(m[1]), looping: Number(m[2]) } : null
}

/**
 * Rows where two implementations of one rule give different answers.
 *
 * Standing debt, not a transient: this does NOT fall to zero when a defect is
 * fixed on one side. `unjudgedCriteria` and `missingVerdictSlots` will keep
 * disagreeing on the 156 unnumbered blocks until one of them is deleted or
 * wired properly, and that is exactly what the row is for - a duplicated rule
 * is a liability for as long as it exists, not only on the day it bites.
 */
export function divergentRows(run = sh) {
  const out = run(`node ${path.join(DIR, 'agree.mjs')}`, 300000)
  const m = out.match(/(\d+) pair\(s\) compared, (\d+) diverging row\(s\)/)
  return m ? { pairs: Number(m[1]), rows: Number(m[2]) } : null
}

/**
 * The share of the last twelve hours with NO bee working.
 *
 * The row beside it, `bees running (of 4)`, is an INSTANT - one sample per
 * iteration of a quantity that turned out to be bimodal. It read 4 as often as
 * 0 and could never have shown that half the day had no bee running at all. A
 * rate needs a window; an instant needs none, which is exactly why the instant
 * is the one that got measured for weeks.
 */
export function idlePercent(run = sh) {
  const out = run(`node ${path.join(DIR, 'idle.mjs')} --hours 12`, 300000)
  const m = out.match(/(\d+)% of the window had no bee working/)
  return m ? Number(m[1]) : null
}

/** The worst-failing chain step, and its rate. */
export function worstStep(run = sh) {
  // A WINDOW, NOT A LIFETIME. The whole record contains two resolved outages -
  // a client that could not attach and a three-hour crash - and a rate over all
  // of it describes neither the past nor the present.
  //
  // EIGHT RUNS, chosen by what a run costs rather than by which number flatters.
  // A chain run is seven to ten minutes, so eight is about an hour. Measured at
  // several sizes on 2026-09-05, push-work reads 0/5, 2/8, 6/12, 14/20 - the
  // gradient IS the crash receding. Eight is short enough to describe now and
  // long enough that a fresh incident still appears, which is the point of
  // putting it on a dashboard at all.
  const out = run(`node ${path.join(DIR, 'failures.mjs')} --last 8`, 120000)
  const rows = out.split('\n')
    .map((l) => l.match(/^\s*(?:!!|\.\.|ok)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)%/))
    .filter(Boolean)
    .map((m) => ({ step: m[1], runs: Number(m[2]), failed: Number(m[3]), rate: Number(m[4]) }))
  if (!rows.length) return null
  return rows.sort((a, b) => b.rate - a.rate)[0]
}

/** proven / checkable across every accepted verdict, from the warm cache, WITH ITS AGE. */
export function provenCounts(read = readFile) {
  // READ THE RECORD, DO NOT RECOMPUTE IT.
  //
  // This ran `proven.mjs` with a 400-second cap. That was set when the swarm had
  // about 200 pushed branches; it now has 311, the pass walks every one of them,
  // and the dashboard printed `-` for two facts in two consecutive rounds -
  // honestly, and for no reason except that drawing a dashboard had become a
  // five-minute computation.
  //
  // The tool records its own reading with --record, exactly as the paired probe
  // does. The dashboard reads that. A measurement that grows with the system
  // does not belong on the path that draws the picture.
  //
  // AND THE READING CARRIES ITS OWN CLOCK, which is the half that was missing.
  // Reading a record is free at any age, so this returned three numbers and no
  // way to tell that they were written a week earlier. `at` comes from the
  // record's own `at` field - not from the file's mtime, which a touch, a copy
  // or a backup would move without a new measurement ever being taken.
  const rows = jsonl(read, path.join(DIR, ...PROVEN_RECORD))
  if (!rows.length) return null
  const last = rows[rows.length - 1]
  const r = last.recent || {}
  const b = last.baseline || {}
  const proven = (r.proven || 0) + (b.proven || 0)
  const judged = (r.checkable || 0) + (b.checkable || 0)
  const total = (r.total || 0) + (b.total || 0)
  return judged ? { proven, judged, unjudgeable: total - judged, at: last.at || null } : null
}

/** How many cases the gate actually contains. */
export function selftestCases(read = (f) => fs.readFileSync(f, 'utf8')) {
  const src = read(path.join(DIR, 'selftest.mjs'))
  const n = (src.match(/^check\(/gm) || []).length
  return n || null
}

/**
 * How often the one gateway every critical path shares actually answers.
 *
 * Read from the paired record rather than probed here: probing would add a
 * sample to the thing being measured, and a dashboard that changes its own
 * number by looking at it is not a dashboard.
 *
 * Eighteen samples on 2026-09-05 said 39% - HTTP answered every time and the
 * ssh gateway refused eleven. Every operation that frees the swarm goes through
 * it, so this belongs beside the swarm counts and not in a file nobody opens.
 */
export const GATEWAY_WINDOW = Number(process.env.TRIOS_GATEWAY_WINDOW || 12)

export function gatewayPercent(read = readFile, window = GATEWAY_WINDOW) {
  const rows = jsonl(read, path.join(DIR, ...PAIRED_RECORD))
  if (!rows.length) return null
  // A LIFETIME AVERAGE HIDES A FIX FOR HOURS.
  //
  // Naming the railway client took the gateway from 7 of 22 to 4 of 4, and this
  // number moved from 35% to 39% - because it was averaging over every sample
  // the broken client ever produced. A dashboard whose job is to show the
  // current state must not be dominated by a period that has ended.
  //
  // The window is short on purpose. It is the same discipline as splitting a
  // before/after at the file's real mtime: the question is what is true now,
  // and yesterday's samples answer a different one.
  const recent = rows.slice(-window)
  const up = recent.filter((r) => r.ssh && r.ssh.attached).length
  return Math.round((100 * up) / recent.length)
}

/**
 * When the newest sample in that window was taken.
 *
 * A percentage over the last twelve samples says nothing about WHEN those twelve
 * happened. If the paired probe stops, this row keeps printing the rate of a
 * window that ended hours ago, and it will keep printing it in exactly the same
 * ink as a reading taken a minute ago. A rate over a dead window is a statement
 * about the past wearing the present tense.
 *
 * Kept separate from `gatewayPercent` rather than folded into a richer return:
 * that function's answer is a number, several callers compare it as one, and a
 * shape change to carry an extra field would be a change to every one of them.
 */
export function gatewayAt(read = readFile) {
  const rows = jsonl(read, path.join(DIR, ...PAIRED_RECORD))
  return rows.length ? (rows[rows.length - 1].at || null) : null
}

/** Only the samples that actually carry a pid reading; a refused attach has none. */
const withPids = (read) => jsonl(read, path.join(DIR, ...PAIRED_RECORD)).filter((r) => r.ssh && r.ssh.pids && r.ssh.pids.max)

/**
 * How close the container is to running out of process slots.
 *
 * On 2026-09-05 the service CRASHED: `/health` 502, railway reporting the
 * deployment Crashed, and the log full of `EAGAIN: resource temporarily
 * unavailable` on posix_spawn. The container could not fork. A fresh one sits at
 * 65 of 1000, so this accumulates - and a crash is the END of a process nobody
 * was watching.
 *
 * Read from the paired record, which now asks while it is attached anyway.
 */
export function pidPercent(read = readFile) {
  const rows = withPids(read)
  if (!rows.length) return null
  const p = rows[rows.length - 1].ssh.pids
  return Math.round((100 * p.used) / p.max)
}

/**
 * When that pid reading was taken.
 *
 * NOT the newest sample in the record - the newest sample that carries pids. A
 * refused attach brings no pid count back, so a run of refusals leaves this row
 * showing a number from before them, and the age of the record as a whole would
 * describe a sample this row is not using. The gap between the two is itself the
 * finding: process pressure is only measurable while the gateway answers.
 */
export function pidAt(read = readFile) {
  const rows = withPids(read)
  return rows.length ? (rows[rows.length - 1].at || null) : null
}

/** Percent in use of the disk this loop runs on. */
export function diskPercent(run = sh) {
  const out = run(`node ${path.join(DIR, 'reap-local.mjs')} 2>/dev/null | head -3`, 200000)
  const m = out.match(/disk (\d+)% used|(\d+)% used/)
  return m ? Number(m[1] || m[2]) : null
}

/**
 * How many ring-00 cases the generated artifact and the production twin agree on.
 *
 * On the dashboard because L0's entire argument is that transcribed rules agree
 * until someone edits one, and until the transcriptions are gone this number is
 * the only thing standing between "they agree" and "they agreed when somebody
 * last looked". It is a COUNT of agreements, not a percentage: a percentage of a
 * grid whose size can change reads the same whether the grid shrank or the
 * agreement grew.
 */
export function ringParity(run = sh) {
  const out = run(`node ${path.join(DIR, 't27-parity.mjs')}`, 300000)
  const m = out.match(/(\d+) case\(s\) compared, (\d+) disagreement\(s\), (\d+) unanswered/)
  if (!m) return null
  return { compared: Number(m[1]), agree: Number(m[1]) - Number(m[2]) - Number(m[3]) }
}

/**
 * How far this checkout has drifted from the branch that ships.
 *
 * ON THE DASHBOARD BECAUSE EVERY OTHER NUMBER HERE IS SUSPECT IN PROPORTION TO
 * IT. On 2026-09-06 `forked-files` reported "0 forked in history" against local
 * HEAD and TWO against `origin/feat/queen-supervisor`; both readings were
 * honest, and the one that went into a round report described a tree nobody
 * deploys. The checkout was 367 commits behind. Nothing said so, because nothing
 * was measuring it.
 *
 * A tool that reads the working tree is answering about this laptop. That is
 * fine when it says which tree it read, and misleading when it does not - so
 * the drift is printed beside the readings it qualifies.
 */
export function behindShipRef(run = sh, ref = process.env.TRIOS_SHIP_REF || 'origin/feat/queen-supervisor') {
  const n = Number(String(run(`git -C ${JSON.stringify(path.join(DIR, '../../..'))} rev-list --count HEAD..${ref}`, 60000)).trim())
  return Number.isFinite(n) ? n : null
}

export function facts(deps = {}) {
  const { run = sh, read } = deps
  return {
    swarm: measure(() => swarmCounts(run)),
    looping: measure(() => loopingCounts(run)),
    idle: measure(() => idlePercent(run)),
    divergent: measure(() => divergentRows(run)),
    worstStep: measure(() => worstStep(run)),
    proven: measure(() => (read ? provenCounts(read) : provenCounts())),
    selftest: measure(() => (read ? selftestCases(read) : selftestCases())),
    disk: measure(() => diskPercent(run)),
    gateway: measure(() => (read ? gatewayPercent(read) : gatewayPercent())),
    // The percentages above are read from a record, so each one is paired with
    // the clock of the sample it came from. Without it a row that stopped being
    // written is drawn in the same ink as one taken a minute ago.
    gatewayAt: measure(() => (read ? gatewayAt(read) : gatewayAt())),
    pids: measure(() => (read ? pidPercent(read) : pidPercent())),
    pidsAt: measure(() => (read ? pidAt(read) : pidAt())),
    parity: measure(() => ringParity(run)),
    behind: measure(() => behindShipRef(run)),
    // Both, deliberately. The live value is what the swarm will dispatch; the
    // ring is what the source declares; the row prints the first and shows the
    // second when they disagree, because the disagreement is a defect nobody
    // was going to find by reading either number alone.
    capacity: {
      live: measure(() => liveCapacity(run)),
      ring: measure(() => T27.ringConst('MAX_CONCURRENT_WORKERS')),
    },
    at: new Date().toISOString(),
  }
}

/**
 * The measured facts as dashboard rows.
 *
 * `prev` is read from the last recorded reading rather than remembered, so the
 * delta column is a measurement too. A row whose value could not be taken is
 * `null` and the renderer shows `-`.
 */
const READINGS = path.join(DIR, 'state', 'dash-readings.jsonl')

export function lastReading(file = READINGS) {
  try {
    const lines = fs.readFileSync(file, 'utf8').split('\n').filter(Boolean)
    return JSON.parse(lines[lines.length - 1])
  } catch { return null }
}

export function recordReading(f, file = READINGS) {
  try {
    fs.mkdirSync(path.dirname(file), { recursive: true })
    fs.appendFileSync(file, JSON.stringify(f) + '\n')
  } catch { /* a reading that cannot be stored is still a reading */ }
}

/**
 * THE FRESHNESS RULE, APPLIED TO THE ANCHOR THE WHOLE DELTA COLUMN HANGS ON.
 *
 * The rule is already written at the top of this file: a stopped writer and a
 * stable metric look identical until the age is on the screen. It was applied
 * to the four rows READ from other records, and not to `prev` - the reading
 * every delta in every row is measured against.
 *
 * Measured 2026-09-13: the newest line in `dash-readings.jsonl` was stamped
 * 2026-09-06T09:18:28Z, seven days earlier, and NOTHING in the tree ran
 * `dash.mjs --record`. It is in no STEPS list in feed.mjs or heal.mjs, in no
 * plist, in no Makefile target. So the box printed `selftest cases 298 +27`
 * and `bees running 0/6 -4`, which read as "since last time" and were a week.
 *
 * A delta over an unmeasured gap is not a delta. When the anchor is older than
 * one cadence the column is EMPTY and the reason is printed once, exactly as a
 * stale row shows its age instead of a number.
 */
export function anchorAge(prev, now = Date.now()) {
  if (!prev) return null
  return ageMs(prev.at, now)
}

export function anchorIsStale(prev, now = Date.now(), cadenceSeconds = CADENCE_SECONDS) {
  // No anchor at all is not a stale anchor - there is simply nothing to compare
  // against, and every row already renders an empty delta for a null `prev`.
  if (!prev) return false
  const a = anchorAge(prev, now)
  // AN ANCHOR WITH NO CLOCK IS NOT A FRESH ANCHOR. A reading written before the
  // `at` field existed would otherwise licence a delta of unknown span.
  if (a === null) return true
  return isStale(a, cadenceSeconds)
}

/**
 * The swarm's capacity, ASKED rather than typed.
 *
 * `bees running (of 4)` had the ring's `MAX_CONCURRENT_WORKERS` written into it
 * as prose. That made this file the fourth place in the tree holding that
 * number - after the spec and two Swift copies - and `snapshot.mjs` the fifth.
 * L0 exists to stop exactly this, and a loop that reports on the swarm's
 * discipline while keeping its own copy of the swarm's constant has no standing
 * to. It is read out of the generated artifact now.
 *
 * `(of ?)` when the artifact is absent, never `(of 4)`. An invented constant
 * that happens to be right is the same defect wearing a correct answer.
 *
 * AND THE RING IS STILL NOT THE SWARM. Asking the ring was an improvement on
 * typing the number and it is not the last word: the ring says what the source
 * DECLARES, the running service says what it will actually DISPATCH, and on
 * 2026-09-12 those were 4 and 6.
 *
 * This label is unchanged, because its caller is the tick-path box in
 * `snapshot.mjs`, which must not grow a remote query. The dashboard row below
 * asks the Queen instead (`liveCapacity`) and keeps the ring only as the
 * comparison - see `capacityNote`.
 */
export function capacityLabel(read) {
  const n = read ? read() : null
  return `bees running (of ${n ?? '?'})`
}

/**
 * What the ring says, when that is not what the swarm says.
 *
 * THE DRIFT IS THE FINDING, so it is not resolved in either direction. Printing
 * only the live 6 hides that the source declares 4; printing only the ring's 4
 * is the defect this replaced. Both, with the live one in the number column and
 * the ring beside it in dim, and a reader can see there is something to fix.
 *
 * Empty string when they agree - a note that is always there is not read.
 */
export function capacityNote(ring, live) {
  const r = ring ?? null
  const l = live ?? null
  if (l === null && r === null) return 'capacity unread, live and ring both'
  if (l === null) return `live capacity unread; ring says ${r}`
  if (r === null) return 'ring capacity unread - run `tri t27-gen`'
  return r === l ? '' : `ring says ${r}`
}

export const QUEEN_STATUS = process.env.TRIOS_QUEEN_STATUS
  || 'https://trios-agent-server-production.up.railway.app/queen/status'

/**
 * How many bees the running service will actually dispatch.
 *
 * THE BOX REPORTED A SATURATED SWARM OUT OF A LOCAL CONSTANT. `bees running
 * (of 4) 4` read as every slot full; the live board said `capacity 6`. Four of
 * four is saturation, four of six is not, and the difference was never measured
 * - it was transcribed from the ring, which is a statement about the source
 * tree and not about the deployment the rest of this dashboard describes.
 *
 * Null when the Queen does not answer, never a remembered 6. `measure` turns a
 * throw into null and the label then prints `?`.
 */
export function liveCapacity(run = sh) {
  const q = JSON.parse(run(`curl -sS -m 20 ${JSON.stringify(QUEEN_STATUS)}`, 30000))
  const c = q && q.workers ? q.workers.capacity : null
  return typeof c === 'number' ? c : null
}

/**
 * A row that came out of a record, carrying the age of that record.
 *
 * `at` is the record's own stamp, `age` the distance from now, `stale` whether
 * the writer has missed a cadence. The renderer uses the last one to decide
 * between a delta and an age: a delta across a gap nobody measured is a
 * comparison between two readings of the same frozen file, and it is always
 * `+0` however long the writer has been dead.
 */
const fromRecord = (row, at, now) => {
  const age = ageMs(at, now)
  return { ...row, at: at || null, age, stale: isStale(age) }
}

export function rows(f, prev, now = Date.now()) {
  // HERE, NOT IN THE RENDERER. Every caller of this function - the box, the
  // snapshot, the suite - gets the same answer about the same anchor. A rule
  // enforced at one of three readers is two readers of one shape, which is the
  // defect this loop's instruments keep finding in everything else.
  const p = anchorIsStale(prev, now) ? {} : (prev || {})
  const ring = f.capacity?.ring ?? null
  const live = f.capacity?.live ?? null
  const running = f.swarm?.running ?? null
  const provenAt = f.proven?.at ?? null
  return [
    // THE DENOMINATOR IS ASKED OF THE SWARM AND COMPARED WITH THE RING, and the
    // value column carries both numbers - `0 / 6` is a fact about the swarm,
    // where `4` beside a label saying `(of 4)` was a fact about a source file.
    {
      k: 'bees running / capacity',
      v: running,
      vText: `${running ?? '-'} / ${live ?? '?'}`,
      prev: p.swarm?.running ?? null,
      goodDown: false,
      note: capacityNote(ring, live),
    },
    { k: 'dispatches finished', v: f.swarm?.finished ?? null, prev: p.swarm?.finished ?? null, goodDown: false },
    fromRecord({ k: 'judged verdicts that prove', v: f.proven?.proven ?? null, prev: p.proven?.proven ?? null, goodDown: false }, provenAt, now),
    fromRecord({ k: 'briefs with nothing checkable', v: f.proven?.unjudgeable ?? null, prev: p.proven?.unjudgeable ?? null }, provenAt, now),
    { k: 'hours 12: no bee working, percent', v: f.idle ?? null, prev: p.idle ?? null },
    { k: 'send-backs looping with no ceiling', v: f.looping?.looping ?? null, prev: p.looping?.looping ?? null },
    { k: 'rows where one rule answers two ways', v: f.divergent?.rows ?? null, prev: p.divergent?.rows ?? null },
    { k: `worst step, last 8 runs: ${f.worstStep?.step ?? '-'}, percent`, v: f.worstStep?.rate ?? null, prev: p.worstStep?.rate ?? null },
    { k: 'selftest cases', v: f.selftest ?? null, prev: p.selftest ?? null, goodDown: false },
    { k: 'ring T27-00 cases agreeing with the twin', v: f.parity?.agree ?? null, prev: p.parity?.agree ?? null, goodDown: false },
    { k: 'commits this checkout is behind what ships', v: f.behind ?? null, prev: p.behind ?? null },
    { k: 'disk this loop runs on, percent', v: f.disk ?? null, prev: p.disk ?? null },
    fromRecord({ k: `ssh gateway answers, last ${GATEWAY_WINDOW}, percent`, v: f.gateway ?? null, prev: p.gateway ?? null, goodDown: false }, f.gatewayAt ?? null, now),
    fromRecord({ k: 'container process slots used, percent', v: f.pids ?? null, prev: p.pids ?? null }, f.pidsAt ?? null, now),
  ]
}

/**
 * Dim for a row whose source has stopped, red for the age that says so.
 *
 * The only colour in this file, and it carries one distinction: whether the
 * number in front of you was taken within the cadence that was supposed to take
 * it. `NO_COLOR` and `--no-color` turn it off, and the words remain - the age is
 * printed either way, because a reader piping this to a file must not lose the
 * one fact the colour exists to draw attention to.
 */
// THE ENVIRONMENT IS SHARED; THE COMMAND LINE IS NOT. `snapshot.mjs` and
// `selftest.mjs` both import this file, and a `--no-color` aimed at either of
// them used to reach in here and silently reconfigure the exported renderer.
// `NO_COLOR` stays at module scope because it IS shared by design; the flag is
// read in the main block at the foot, where it belongs to this process alone.
let PLAIN = !!process.env.NO_COLOR
const dim = (s) => (PLAIN ? s : `\x1b[2m${s}\x1b[0m`)
const red = (s) => (PLAIN ? s : `\x1b[31m${s}\x1b[0m`)

export function renderRow(r) {
  // A STALE ROW SHOWS ITS AGE INSTEAD OF A DELTA, not as well as one. Both
  // readings come out of the same unchanged file, so the delta is `+0` by
  // construction - it looks like a settled metric and is a dead writer, and
  // printing the two side by side would leave the reader to pick. The age is
  // the fact; the delta across a gap nobody measured is not one.
  const d = !r.stale && r.v !== null && r.prev !== null
    ? (r.v - r.prev >= 0 ? `+${r.v - r.prev}` : String(r.v - r.prev))
    : ''
  const value = r.vText ?? String(r.v ?? '-')
  const line = `  ${String(r.k).padEnd(38)} ${value.padStart(6)}  ${d.padStart(5)}`
  const notes = []
  if (r.age !== null && r.age !== undefined) notes.push(r.stale ? red(ageWords(r.age)) : dim(ageWords(r.age)))
  else if (r.at === null && 'at' in r) notes.push(red('source carries no clock'))
  if (r.note) notes.push(dim(r.note))
  return (r.stale ? dim(line) : line) + (notes.length ? `  ${notes.join('  ')}` : '')
}

if (isMain) {
  if (process.argv.includes('--no-color')) PLAIN = true
  // `--if-due` EXISTS SO THE WRITER CAN BE WIRED WITHOUT SLOWING THE CHAIN.
  //
  // Taking these facts costs about 110 seconds: it curls the service, sshes the
  // container and runs the suite. The heal chain fires every 600 s with an
  // eight-minute deadline for all of its steps, and the cadence these readings
  // are judged against is 3600 s. Recording on every fire would be six times
  // more often than the freshness rule asks and would spend a fifth of the
  // chain's budget doing it.
  //
  // So the step asks first, and a run that is not due exits in milliseconds
  // having measured nothing. The alternative - a second timer - is a second
  // place the cadence is written down.
  if (process.argv.includes('--if-due') && !anchorIsStale(lastReading())) {
    const a = anchorAge(lastReading())
    console.log(`not due: the last reading is ${ageWords(a)}, inside the ${Math.round(CADENCE_SECONDS / 60)}-minute cadence. Measured nothing.`)
    process.exit(0)
  }
  const f = facts()
  if (process.argv.includes('--facts')) {
    console.log(JSON.stringify(f, null, 2))
    process.exit(0)
  }
  const prev = lastReading()
  console.log('measured now, nothing typed:\n')
  for (const r of rows(f, prev)) console.log(renderRow(r))
  if (anchorIsStale(prev)) {
    console.log(`\n  ${red('NO DELTA COLUMN')}: the reading it would be measured against was taken`)
    console.log(`  ${red(ageWords(anchorAge(prev)))}, past the ${Math.round(CADENCE_SECONDS / 60)}-minute cadence. A delta over a gap nobody`)
    console.log(`  measured is not a delta. Write a fresh anchor with ${'`node dash.mjs --record`'}.`)
  } else if (!prev) {
    console.log('\n  No delta column: nothing has been recorded yet.')
  }
  const stale = rows(f, prev).filter((r) => r.stale)
  if (stale.length) {
    console.log(`\n  ${stale.length} row(s) are older than the ${Math.round(CADENCE_SECONDS / 60)}-minute cadence that writes them,`)
    console.log('  shown dim with their age instead of a delta. A stopped writer and a stable')
    console.log('  metric look identical until the age is on the screen.')
  }
  const missing = rows(f, prev).filter((r) => r.v === null).map((r) => r.k)
  if (missing.length) {
    console.log(`\n  ${missing.length} fact(s) could not be measured and are shown as '-': ${missing.join(', ')}`)
    console.log('  They are not filled in from memory. That is the whole point of this file.')
  }
  if (process.argv.includes('--record')) recordReading(f)
}
