#!/usr/bin/env node
// One reading of the loop's own vital signs, measured rather than remembered.
//
// WHY THIS EXISTS, SEPARATELY FROM dash.mjs.
//
// `dash.mjs` measures the SWARM - how many bees, how many dispatches, how many
// send-backs. That is the thing the loop works on. It does not measure the LOOP
// ITSELF, and on 2026-09-12 that gap had a cost: `DASHBOARD.txt` was advertising
// `cron */15 job 23d6fe89` at the top of every screen while that job had not
// existed for six days. Every number under the header was honestly measured and
// the header was a fossil, so the whole artifact read as live.
//
// An instrument that reports on a process cannot also be the only evidence that
// the process is running. So this file measures the machinery: is there a
// driver, when did the ledger last move, who holds the lock, how old is the
// newest iteration, are the timers firing. Those are the facts that decide
// whether anything else on the screen means what it says.
//
// THE RULES ARE dash.mjs's RULES.
//
//   * A fact that cannot be measured comes back null and renders as `-`.
//     It is never filled in from memory and never silently becomes 0.
//   * A non-zero exit from a probe is often the answer, not a failure.
//   * Every fact carries its own provenance: what was asked, and when.
//
// Usage:
//   node sense.mjs              # the reading, as lines
//   node sense.mjs --json       # the reading, as JSON
//   node sense.mjs --record     # append the reading to state/cycle-readings.jsonl
//   node sense.mjs --deep       # also run the slow probes (idle histogram)
//   node sense.mjs --fast       # skip every network probe

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const TRINITY = path.resolve(DIR, '..')
const TRIOS = path.resolve(TRINITY, '..')
const REPO = path.resolve(TRIOS, '..')
const STATE_DIR = path.join(DIR, 'state')
const READINGS = path.join(STATE_DIR, 'cycle-readings.jsonl')

// Mutable because `takeReading()` is called programmatically by `cycle.mjs` as
// well as from the command line, and a caller must be able to ask for a deep
// reading without forging process.argv. Single process, single thread, set once
// at the top of takeReading.
//
// THEY START FALSE AND THE COMMAND LINE IS READ IN THE GUARD AT THE FOOT.
// They used to initialise from `process.argv` here, which meant that for every
// importer - `cycle.mjs`, `anomaly.mjs`, `dash2.mjs` - the DEFAULT depth of a
// reading was decided by flags aimed at a different program. `cycle.mjs` passes
// `{ deep: false }` explicitly and so was never bitten, which is exactly how a
// defect like this survives: it is correct at every call site that exists, and
// wrong for the one somebody adds next.
let DEEP = false
let FAST = false

const QUEEN = 'https://trios-agent-server-production.up.railway.app/queen/status'

// ---------------------------------------------------------------------------
// measurement primitives
// ---------------------------------------------------------------------------

/**
 * A single measured fact.
 *
 * `v === null` means NOT MEASURED. That is a real answer and it is different
 * from zero in every way that matters: zero bees running is a working swarm
 * with nothing to do, and an unreachable Queen is no information at all. The
 * renderer must be able to tell them apart, so the shape carries the
 * difference rather than the caller remembering it.
 */
function fact(v, src, note) {
  return { v: v === undefined ? null : v, src: src || '', note: note || '', at: new Date().toISOString() }
}

function measure(src, fn, note) {
  try {
    return fact(fn(), src, note)
  } catch (e) {
    return fact(null, src, 'unmeasured: ' + String((e && e.message) || e).slice(0, 120))
  }
}

/**
 * Run a shell command and keep the output whatever the exit code.
 *
 * Several instruments in this directory signal through their exit status -
 * `failures.mjs` exits 2 when a step fails too often, `reap-local` exits 1 for
 * "would act". Treating those as errors throws away a measurement that was
 * taken successfully. Only a signal or a timeout means nothing was measured.
 */
function sh(cmd, timeout) {
  try {
    return execSync(cmd, { cwd: DIR, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: timeout || 20000 }).trim()
  } catch (e) {
    if (e.killed || e.signal || e.code === 'ETIMEDOUT') throw e
    const out = String(e.stdout || '').trim()
    if (!out) throw e
    return out
  }
}

const hoursSince = (iso) => (Date.now() - Date.parse(iso)) / 36e5
const round1 = (n) => Math.round(n * 10) / 10

// ---------------------------------------------------------------------------
// the loop's own machinery
// ---------------------------------------------------------------------------

function readState() {
  return JSON.parse(fs.readFileSync(path.join(DIR, 'state.json'), 'utf8'))
}

// Is anything actually driving the loop? The reading moved to `driver.mjs`,
// because this function and `loop.mjs driverReading()` were the same twenty
// lines typed twice - and one of them carried a comment asserting they agreed
// "by construction". They agreed by transcription. The move also fixed two
// zeros that were asserted rather than measured; the argument is in that file.
//
// A pure-function import, not a state one: the new cycle still reads none of
// the old generation's files, which is what `cycle-doctor.mjs` guards.
import { driverReading as driver } from './driver.mjs'

/**
 * The lock, and whether it is stale.
 *
 * `tri iter` prints a warning that the pid being alive is NOT what decides the
 * lock, and it is right: heal and feed are separate processes that hand the
 * lock between them, so a live pid proves only that some process exists. Age is
 * what decides. A lock older than the longest honest run is wedged.
 */
function lockFacts() {
  const p = path.join(DIR, 'loop.lock')
  if (!fs.existsSync(p)) return { held: false, holder: null, ageMin: null, pid: null, pidAlive: null }
  const j = JSON.parse(fs.readFileSync(p, 'utf8'))
  let alive = null
  if (j.pid) {
    try { process.kill(j.pid, 0); alive = true } catch (e) { alive = e.code === 'EPERM' }
  }
  return { held: true, holder: j.holder || null, ageMin: round1(hoursSince(j.at) * 60), pid: j.pid || null, pidAlive: alive }
}

/**
 * When did the ledger last move, and how much has it moved in a day?
 *
 * The ledger is 1.6 MB of JSONL and reading it whole to answer "is the loop
 * alive" would be the instrument costing more than the fact. The tail is
 * enough: entries are append-only and chronological.
 */
function ledgerFacts() {
  const p = path.join(DIR, 'ledger.jsonl')
  const tail = sh(`tail -400 ${JSON.stringify(p)}`, 15000)
  const rows = tail.split('\n').map((l) => { try { return JSON.parse(l) } catch { return null } }).filter(Boolean)
  if (!rows.length) return { lastAt: null, last24h: null, kinds: {} }
  const last = rows[rows.length - 1]
  const dayAgo = Date.now() - 864e5
  const recent = rows.filter((r) => r.at && Date.parse(r.at) >= dayAgo)
  const kinds = {}
  for (const r of recent) kinds[r.kind || 'unknown'] = (kinds[r.kind || 'unknown'] || 0) + 1
  // last24h is a floor, not a count: the tail may not reach back a full day.
  return { lastAt: last.at || null, last24h: recent.length, truncated: recent.length === rows.length, kinds }
}

/** Timer liveness read from the mtime of the log each timer appends to. */
function timerFacts() {
  const out = {}
  for (const name of ['heal', 'feed']) {
    const p = path.join(DIR, name + '.timer.log')
    try {
      const st = fs.statSync(p)
      out[name] = { lastWriteMin: round1((Date.now() - st.mtimeMs) / 6e4), sizeMB: round1(st.size / 1048576) }
    } catch { out[name] = { lastWriteMin: null, sizeMB: null } }
  }
  return out
}

// ---------------------------------------------------------------------------
// the swarm
// ---------------------------------------------------------------------------

/**
 * One bucket of the Queen's skip summary, as a number or null.
 *
 * THE SHAPE CHANGED UNDER TWO READERS AND ONLY ONE WAS TOLD. `skipSummary` once
 * served plain integers; it now serves `{count, issues[], more}`. This function
 * learned the new shape and kept the old one working. `why.mjs` did not: it
 * still wrote `Number(skips.missingBoundary ?? 0)`, and `Number({count:448})` is
 * NaN, so every comparison against it was false and four of that file's six idle
 * diagnoses could not fire. `tri why` was structurally blind to the 448-issue
 * missingBoundary cause - the single largest fact about the swarm - and said
 * nothing rather than saying it could not tell.
 *
 * So it lives here once, exported, and both read it. A hand-copied accessor is
 * how the drift happened.
 *
 * null, never 0: an absent bucket is a bucket nobody measured, and a diagnosis
 * that fires on `>= 3` must not be handed a zero it was never given.
 */
export function skipCount(summary, key) {
  const s = summary || {}
  const v = s[key]
  if (v && typeof v.count === 'number') return v.count
  if (typeof v === 'number') return v
  return null
}

/**
 * The Queen's own board: bees, dispatches, and - the number this loop exists
 * for - the reason she refused to start anything.
 *
 * The skip summary is the diagnosis. `missingBoundary` dominating it means the
 * backlog is full and unusable, which looks identical from the bee count to a
 * backlog that is empty.
 */
function queenFacts() {
  if (FAST) return null
  const raw = sh(`curl -sS -m 20 ${JSON.stringify(QUEEN)}`, 30000)
  const j = JSON.parse(raw)
  const t = j.lastTick || {}
  const d = j.dispatches || {}
  const s = t.skipSummary || {}
  const n = (k) => skipCount(s, k)
  return {
    running: typeof d.running === 'number' ? d.running : null,
    finished: typeof d.finished === 'number' ? d.finished : null,
    refusal: t.allowed ? 'DISPATCHED' : (t.refusal || null),
    skips: typeof t.skippedCount === 'number' ? t.skippedCount : null,
    missingBoundary: n('missingBoundary'),
    claimed: n('claimed'),
    completed: n('completed'),
    fileConflict: n('fileConflict'),
    notFirst: n('notFirst'),
    tickAt: t.at || j.at || null,
  }
}

/**
 * Is there anything a bee could actually take?
 *
 * DEEP ONLY, because it reads 588 issue bodies through `gh api --paginate` and
 * takes about a minute. Skipped, it reads `-` rather than 0 - and the difference
 * is load-bearing here more than anywhere else in this file, because 0 means
 * "the swarm has eaten everything" and `-` means "nobody looked".
 *
 * WHY IT IS WORTH A MINUTE. `skipSummary.missingBoundary` was read as the cause
 * of the idle swarm for weeks, and three different remedies were designed
 * against it. The actual cause is one number that nothing measured: of the 38
 * issues the Queen's own parser accepts, 38 were already claimed, finished or
 * running. The queue was empty. Every remedy aimed at the parser was aimed at a
 * symptom of a full backlog that contained no work.
 */
function backlogFacts() {
  if (!DEEP) return null
  const raw = sh(`node ${JSON.stringify(path.join(DIR, 'backlog.mjs'))} --json --record`, 300000)
  const j = JSON.parse(raw)
  return {
    total: typeof j.total === 'number' ? j.total : null,
    hasBoundary: j.counts ? j.counts.HAS_BOUNDARY : null,
    boundaryNoPaths: j.counts ? j.counts.BOUNDARY_NO_PATHS : null,
    repairable: Array.isArray(j.repairable) ? j.repairable.length : null,
    delegableNow: j.delegableNow === undefined ? null : j.delegableNow,
  }
}

/**
 * The idle histogram, and the reasons rounds failed.
 *
 * Slow - it reads a long log window - so it is behind --deep. The failure
 * reasons matter more than the percentage: "89% idle" is a symptom shared by
 * every cause, and "11 rounds failed: GitHub returned 403" is a cause with a
 * one-line fix.
 */
function idleFacts() {
  if (!DEEP || FAST) return null
  const tri = path.join(process.env.HOME || '', '.local/bin/tri')
  const out = sh(`${JSON.stringify(tri)} idle 2>&1`, 180000)
  const pct = out.match(/(\d+)%\s+of the window had no bee working/)
  const failed = out.match(/(\d+)\s+round\(s\)\s+FAILED/)
  const reasons = []
  const rx = /^\s+(\d+)\s+(.+)$/gm
  let m
  const after = out.slice(out.indexOf('The reasons they gave'))
  while ((m = rx.exec(after))) reasons.push({ n: Number(m[1]), why: m[2].trim() })
  return {
    percentNoBee: pct ? Number(pct[1]) : null,
    roundsFailed: failed ? Number(failed[1]) : null,
    reasons: reasons.slice(0, 5),
  }
}

// ---------------------------------------------------------------------------
// the tree this loop runs on
// ---------------------------------------------------------------------------

/**
 * Git facts, split into GENERATED and SOURCE.
 *
 * This split is the whole point. A cron job that commits everything would sweep
 * 1.6 MB of ledger and two timer logs into the history along with the source it
 * meant to save, and a cron job that commits nothing leaves real work
 * uncommitted until a worktree reaper takes it. Neither is safe without knowing
 * which file is which, so the classification lives here, next to the count.
 */
const GENERATED = [
  /\.trinity\/loop\/(ledger\.jsonl|state\.json|loop\.lock|.*\.timer\.log)$/,
  /\.trinity\/loop\/state\/.*\.jsonl$/,
  /\.trinity\/loop\/state\/.*\.json$/,
  /\.trinity\/loop\/DASHBOARD\.(txt|ansi)$/,
  /\.trinity\/.*\.log$/,
  /^doctor_reports\//,
]

function gitFacts() {
  const porcelain = sh(`git -C ${JSON.stringify(REPO)} status --porcelain`, 30000)
  const lines = porcelain.split('\n').filter(Boolean)
  const isGen = (p) => GENERATED.some((rx) => rx.test(p))
  const modified = lines.filter((l) => !l.startsWith('??')).map((l) => l.slice(3))
  const untracked = lines.filter((l) => l.startsWith('??')).map((l) => l.slice(3))
  let ahead = null, behind = null
  try {
    const sb = sh(`git -C ${JSON.stringify(REPO)} status -sb | head -1`, 20000)
    const a = sb.match(/ahead (\d+)/), b = sb.match(/behind (\d+)/)
    ahead = a ? Number(a[1]) : 0
    behind = b ? Number(b[1]) : 0
  } catch { /* leave null */ }
  let branch = null
  try { branch = sh(`git -C ${JSON.stringify(REPO)} rev-parse --abbrev-ref HEAD`, 10000) } catch { /* null */ }
  let worktrees = null
  try { worktrees = sh(`git -C ${JSON.stringify(REPO)} worktree list | wc -l`, 20000).trim() } catch { /* null */ }
  return {
    branch,
    ahead,
    behind,
    modifiedSource: modified.filter((p) => !isGen(p)).length,
    modifiedGenerated: modified.filter(isGen).length,
    untrackedSource: untracked.filter((p) => !isGen(p)).length,
    untrackedGenerated: untracked.filter(isGen).length,
    sourceFiles: modified.filter((p) => !isGen(p)).slice(0, 40),
    worktrees: worktrees === null ? null : Number(worktrees),
  }
}

/**
 * Free space on the volume the loop and its worktrees live on, and how much of
 * what is used belongs to the loop.
 *
 * WHOSE DISK IS IT. The percentage alone, printed beside a worktree count, reads
 * as a cause - and the repair it invites is to delete the worktrees, which can
 * destroy a bee's uncommitted work. Measured on 2026-09-13: the volume was 95%
 * used, 384G of 460G, and every worktree the loop owns came to 2.5G of it. Six
 * tenths of one percent. Deleting all of them would not have moved the number
 * that raised the alarm.
 *
 * `du` over the worktree root takes seconds, so it is behind --deep. When it was
 * not measured the field stays null and the anomaly says it was not measured,
 * rather than implying a share it does not know.
 */
function diskFacts() {
  const out = sh(`df -h ${JSON.stringify(REPO)} | tail -1`, 10000)
  const m = out.match(/(\d+)%/)
  const fact = { percentUsed: m ? Number(m[1]) : null, raw: out.replace(/\s+/g, ' '), ownGB: null, ownPercentOfUsed: null }
  if (DEEP) {
    try {
      const wt = path.join(TRIOS, '.worktrees')
      const ownK = fs.existsSync(wt) ? Number(sh(`du -sk ${JSON.stringify(wt)} | cut -f1`, 180000)) : 0
      const usedK = Number(sh(`df -k ${JSON.stringify(REPO)} | tail -1 | awk '{print $3}'`, 10000))
      if (Number.isFinite(ownK) && Number.isFinite(usedK) && usedK > 0) {
        fact.ownGB = Math.round((ownK / 1048576) * 10) / 10
        fact.ownPercentOfUsed = Math.round((ownK / usedK) * 1000) / 10
      }
    } catch {
      // Stays null. Not measured is not zero, and a zero here would read as
      // "the loop owns none of it", which is a different and stronger claim.
    }
  }
  return fact
}

/**
 * Declared gates versus wired gates.
 *
 * Counted from the two declarations rather than from a list anybody typed: the
 * `check:` prerequisite list in the Makefile, and the workflow files CI will
 * actually load. A gate named in one and absent from the other is the whole
 * finding, and a hand-copied list would drift away from it within a week.
 */
function gateFacts() {
  const mk = fs.readFileSync(path.join(TRIOS, 'Makefile'), 'utf8')
  const m = mk.match(/^check:\s*(.*)$/m)
  const declared = m ? m[1].trim().split(/\s+/).filter(Boolean) : []
  let workflows = []
  try {
    workflows = fs.readdirSync(path.join(REPO, '.github/workflows')).filter((f) => /\.ya?ml$/.test(f))
  } catch { workflows = [] }
  let wired = null
  try {
    const blob = workflows.map((f) => fs.readFileSync(path.join(REPO, '.github/workflows', f), 'utf8')).join('\n')
    wired = declared.filter((g) => new RegExp('make\\s+(-\\w+\\s+)*' + g.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '\\b').test(blob)).length
  } catch { wired = null }
  return { declared: declared.length, declaredNames: declared, workflows: workflows.length, wired }
}

// ---------------------------------------------------------------------------
// the reading
// ---------------------------------------------------------------------------

/**
 * @param {{deep?: boolean, fast?: boolean}} [opts] overrides the command-line flags
 */
export function takeReading(opts) {
  if (opts && opts.deep !== undefined) DEEP = !!opts.deep
  if (opts && opts.fast !== undefined) FAST = !!opts.fast
  const st = measure('state.json', readState)
  const s = st.v || {}
  const r = {
    at: new Date().toISOString(),
    mode: FAST ? 'fast' : DEEP ? 'deep' : 'normal',
    loop: {
      iteration: fact(typeof s.iteration === 'number' ? s.iteration : null, 'state.json'),
      lastFinishedAt: fact(s.lastFinishedAt || null, 'state.json'),
      staleHours: fact(s.lastFinishedAt ? round1(hoursSince(s.lastFinishedAt)) : null, 'state.json', 'hours since the last iteration closed'),
      title: fact(s.title || null, 'state.json'),
      doneUnits: fact(s.done ? Object.keys(s.done).length : null, 'state.json'),
      lessons: fact(Array.isArray(s.lessons) ? s.lessons.length : null, 'state.json'),
      anchors: fact(s.anchors ? Object.keys(s.anchors).length : null, 'state.json'),
    },
    driver: measure('scheduled_tasks.json + crontab + launchctl', driver, 'what, if anything, fires the loop'),
    lock: measure('loop.lock', lockFacts),
    ledger: measure('tail ledger.jsonl', ledgerFacts),
    timers: measure('mtime of *.timer.log', timerFacts),
    queen: measure('GET /queen/status', queenFacts, FAST ? 'skipped: --fast' : ''),
    backlog: measure('tri backlog', backlogFacts, DEEP ? '' : 'skipped: needs --deep'),
    idle: measure('tri idle', idleFacts, DEEP ? '' : 'skipped: needs --deep'),
    git: measure('git status --porcelain', gitFacts),
    disk: measure('df -h', diskFacts),
    gates: measure('Makefile check: + .github/workflows', gateFacts),
  }
  return r
}

// ---------------------------------------------------------------------------
// rendering
// ---------------------------------------------------------------------------

const show = (f) => (f && f.v !== null && f.v !== undefined ? f.v : '-')

function lines(r) {
  const out = []
  const q = r.queen.v
  const g = r.git.v
  const d = r.driver.v
  const lk = r.lock.v
  const t = r.timers.v
  const gt = r.gates.v
  out.push(`reading ${r.at}  (${r.mode})`)
  out.push('')
  out.push('LOOP')
  out.push(`  iteration            ${show(r.loop.iteration)}`)
  out.push(`  last finished        ${show(r.loop.lastFinishedAt)}`)
  out.push(`  stale (hours)        ${show(r.loop.staleHours)}`)
  out.push(`  driver               claude-cron=${d ? d.claudeCron : '-'}  crontab=${d ? d.crontab : '-'}  launchd=${d ? d.launchd : '-'}`)
  out.push(`  lock                 ${lk && lk.held ? lk.holder + ' ' + lk.ageMin + 'm (pid ' + lk.pid + (lk.pidAlive ? ' alive' : ' DEAD') + ')' : 'free'}`)
  out.push(`  ledger last entry    ${r.ledger.v ? r.ledger.v.lastAt : '-'}`)
  out.push(`  timers last wrote    heal=${t && t.heal ? t.heal.lastWriteMin + 'm' : '-'}  feed=${t && t.feed ? t.feed.lastWriteMin + 'm' : '-'}`)
  out.push('')
  out.push('SWARM')
  out.push(`  bees running         ${q ? q.running : '-'}`)
  out.push(`  dispatches finished  ${q ? q.finished : '-'}`)
  out.push(`  refusal              ${q ? q.refusal : '-'}`)
  out.push(`  skips                ${q ? q.skips : '-'}`)
  out.push(`    missing boundary   ${q ? q.missingBoundary : '-'}`)
  out.push(`    claimed            ${q ? q.claimed : '-'}`)
  out.push(`    completed          ${q ? q.completed : '-'}`)
  if (r.idle.v) {
    out.push(`  window with no bee   ${r.idle.v.percentNoBee}%`)
    out.push(`  rounds failed        ${r.idle.v.roundsFailed}  ${r.idle.v.reasons.map((x) => x.n + ' ' + x.why).join('; ')}`)
  }
  out.push('')
  out.push('TREE')
  out.push(`  branch               ${g ? g.branch + '  ahead ' + g.ahead + ' behind ' + g.behind : '-'}`)
  out.push(`  dirty source         ${g ? g.modifiedSource : '-'}   generated ${g ? g.modifiedGenerated : '-'}`)
  out.push(`  untracked source     ${g ? g.untrackedSource : '-'}   generated ${g ? g.untrackedGenerated : '-'}`)
  out.push(`  worktrees            ${g ? g.worktrees : '-'}`)
  out.push(`  disk used            ${r.disk.v ? r.disk.v.percentUsed + '%' : '-'}`)
  out.push(`  gates declared/wired ${gt ? gt.declared + '/' + (gt.wired === null ? '-' : gt.wired) : '-'}`)
  return out.join('\n')
}

// ---------------------------------------------------------------------------

const isMain = process.argv[1] && process.argv[1].endsWith('/sense.mjs')
if (isMain) {
  const argv = process.argv.slice(2)
  const WANT_JSON = argv.includes('--json')
  const WANT_RECORD = argv.includes('--record')
  // Passed as options rather than assigned to the module's `DEEP`/`FAST`, so
  // that the command line reaches the reading by the SAME route a programmatic
  // caller uses. One path in, not two.
  const r = takeReading({ deep: argv.includes('--deep'), fast: argv.includes('--fast') })
  if (WANT_RECORD) {
    fs.mkdirSync(STATE_DIR, { recursive: true })
    fs.appendFileSync(READINGS, JSON.stringify(r) + '\n')
  }
  process.stdout.write((WANT_JSON ? JSON.stringify(r, null, 2) : lines(r)) + '\n')
}
