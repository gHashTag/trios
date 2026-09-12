#!/usr/bin/env node
// Where this system's own reports disagree with this system's own data.
//
// WHY THIS EXISTS.
//
// Sixty-six entries in the anomaly register of a sibling project share one
// property: not one of the underlying MEASUREMENTS was wrong. The sentence
// built on the measurement was. A number is copied into a header and the header
// outlives the number; an instrument reports a level where the decision needs a
// rate; two tools read the same field and only one of them was told it gets
// rewritten.
//
// On 2026-09-12 this directory had a live example. `DASHBOARD.txt` opened with
//
//     TRIOS CONTINUOUS LOOP   cron */15   job 23d6fe89
//
// and the job had not existed for six days - it was a session-scoped cron that
// died with the session that created it, leaving `.claude/scheduled_tasks.json`
// holding an empty array. Every number below that header was honestly measured.
// The header made all of them read as current.
//
// So this file does not measure the swarm. It cross-examines the ARTIFACTS
// against the READING: the dashboard against the live counters, the state file
// against the clock, the lock against the process table, the declarations
// against what is wired. Each finding is a pair - what was claimed, what is
// true - plus the one-line repair.
//
// EXIT STATUS IS THE ANSWER.
//   0  no anomaly found
//   2  at least one anomaly found        <- this is the tool working
//   1  the tool could not run
//
// Usage:
//   node anomaly.mjs           # the anomalies, as lines
//   node anomaly.mjs --json    # the anomalies, as JSON
//   node anomaly.mjs --record  # append to state/anomalies.jsonl

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { takeReading } from './sense.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const STATE_DIR = path.join(DIR, 'state')
const LOG = path.join(STATE_DIR, 'anomalies.jsonl')

// The flags are read inside the main guard at the foot of this file, not here.
// `cycle.mjs`, `dash2.mjs` and `dash-cc.mjs` all import this module for
// `findAnomalies`, so anything read at module scope is read out of THEIR command
// line. These two happen to be consumed only by the CLI path, so the leak was
// latent rather than active - but a latent leak is what the next import turns
// into a live one, and this file gained its third importer today.

// How long a loop may go unadvanced before its own dashboard is lying by
// omission. Chosen as four times the longest honest gap between iterations
// observed in the ledger (iterations 1-96 ran at roughly 15-minute intervals,
// with recorded gaps up to ~6 h across sleeps), so a laptop shut overnight does
// not raise it and a dead driver does.
const STALE_HOURS = 24

// The volume threshold that matters is not "full". Bees die at 0 seconds when
// `git worktree add` cannot write, and the measured climb rate on this fleet
// was about fifteen points an hour with four bees. 85% is roughly one hour of
// warning against a timer that looks every twenty minutes.
const DISK_WARN = 85

const A = (id, severity, claim, truth, repair, evidence) => ({ id, severity, claim, truth, repair, evidence })

function readIfExists(p) {
  try { return fs.readFileSync(p, 'utf8') } catch { return null }
}

export function findAnomalies(r) {
  const out = []
  const q = r.queen.v
  const g = r.git.v
  const d = r.driver.v
  const lk = r.lock.v
  const led = r.ledger.v
  const gt = r.gates.v
  const dash = readIfExists(path.join(DIR, 'DASHBOARD.txt'))
  let dashAgeH = null
  try { dashAgeH = Math.round(((Date.now() - fs.statSync(path.join(DIR, 'DASHBOARD.txt')).mtimeMs) / 36e5) * 10) / 10 } catch { /* null */ }

  // --- 1. the dashboard advertises a driver -------------------------------
  //
  // The header names a cron job. If no scheduled task exists, the artifact is
  // asserting that something drives it when nothing does. This is the exact
  // defect that motivated the file.
  if (dash) {
    const m = dash.match(/cron\s+(\S+)\s+job\s+([0-9a-f]{6,})/)
    if (m && d && d.claudeCron === 0) {
      out.push(A(
        'dashboard-advertises-dead-driver', 'blocker',
        `DASHBOARD.txt header says "cron ${m[1]} job ${m[2]}"`,
        `.claude/scheduled_tasks.json holds 0 tasks - that job does not exist`,
        'Render the driver row from scheduled_tasks.json at draw time instead of writing the job id into the template, and show NO DRIVER when the list is empty.',
        `dashboard mtime ${dashAgeH}h ago; claudeCron=${d.claudeCron}`,
      ))
    }
  }

  // --- 2. the loop has not advanced ---------------------------------------
  const stale = r.loop.staleHours.v
  if (stale !== null && stale > STALE_HOURS) {
    out.push(A(
      'loop-not-advancing', 'blocker',
      `state.json presents iteration ${r.loop.iteration.v} as the current one`,
      `it closed ${stale} hours ago (${Math.round(stale / 24 * 10) / 10} days) and no iteration has opened since`,
      'Either start an iteration or make the dashboard say STOPPED. A loop that stops is fine; a loop that stops while reporting as running is not.',
      `lastFinishedAt=${r.loop.lastFinishedAt.v}`,
    ))
  }

  // --- 3. the timers move while the loop does not -------------------------
  //
  // This pair is the interesting one: it is evidence that the machinery is
  // healthy and merely has nothing to do, which is a different repair from a
  // crashed timer. Reporting it as one anomaly keeps the two from being
  // confused in the morning.
  const healM = r.timers.v && r.timers.v.heal ? r.timers.v.heal.lastWriteMin : null
  if (healM !== null && healM < 30 && stale !== null && stale > STALE_HOURS) {
    out.push(A(
      'timers-alive-loop-dead', 'high',
      'the heal and feed timers are firing, which reads as "the loop is running"',
      `they last wrote ${healM} min ago and are doing no-op work (author filed 0, land 0, close-done nothing) while the iteration counter has not moved in ${stale}h`,
      'Separate the two liveness signals on the dashboard: TIMERS (launchd) and DRIVER (iterations). They fail independently and today they disagree.',
      `heal.timer.log mtime ${healM}m; iteration stale ${stale}h`,
    ))
  }

  // --- 4. the dashboard's numbers vs the live board -----------------------
  if (dash && q) {
    const m = dash.match(/bees running \(of \d+\)\s+(\d+)/)
    if (m && q.running !== null && Number(m[1]) !== q.running) {
      out.push(A(
        'dashboard-bee-count-stale', 'high',
        `DASHBOARD.txt shows "bees running ${m[1]}"`,
        `the Queen's board says running ${q.running} right now; the dashboard file is ${dashAgeH}h old`,
        'Stamp every rendered dashboard with the age of its newest measurement and refuse to draw a swarm row older than one hour.',
        `dashboard mtime ${dashAgeH}h; live running=${q.running}`,
      ))
    }
  }

  // --- 5. a backlog that is not a work queue ------------------------------
  //
  // "nothing to choose" is the same string whether the backlog is empty or
  // unusable, and the cure differs, so the refusal must never be read alone.
  //
  // THIS CHECK USED TO GIVE THE WRONG CURE, and the correction is worth keeping
  // in view. It said: "the backlog is full and unreadable - repair the boundary
  // section on existing ones". That reading was drawn entirely from
  // `skipSummary`, which counts what the parser refused and cannot say why.
  // Reading the issues themselves (`backlog.mjs`, all 588 open, 2026-09-12)
  // gave a different answer: 38 have a boundary the Queen would accept, 23 are
  // real briefs one heading short, and 527 - 89% - are reports and free prose.
  // A report has no boundary to repair, because nobody has yet decided what
  // work it implies.
  //
  // So the cure is not formatting. It is that 89% of the backlog was written to
  // record findings, and findings are not instructions. `missingBoundary` is a
  // true count of a real refusal that names the wrong remedy.
  //
  // AND THE CURE ABOVE IS STILL NOT THE WHOLE ONE. Crossed against the tick's
  // own skipSummary the same afternoon, all 38 acceptable issues were already
  // claimed, finished or running: set equality, both directions empty. So
  // "nothing to choose" was not a parser failure OR a composition problem - it
  // was literally true. An advisory that sends the reader to repair formatting
  // when the queue is genuinely empty costs a day. The severity below now
  // depends on which of the two it is, and the difference is measured rather
  // than assumed.
  if (q && q.refusal && /nothing to choose/i.test(q.refusal) && q.missingBoundary && q.missingBoundary > 0) {
    const share = q.skips ? Math.round((q.missingBoundary / q.skips) * 100) : null
    // Measured only on a deep pass; null when nobody looked, which is not zero.
    const bl = r.backlog && r.backlog.v
    const delegable = bl && typeof bl.delegableNow === 'number' ? bl.delegableNow : null
    const empty = delegable === 0
    out.push(A(
      'backlog-full-but-unusable', 'blocker',
      `the Queen reports "${q.refusal}", which reads as an empty backlog`,
      empty
        ? `${q.missingBoundary} of ${q.skips} skipped for missingBoundary (${share}%), AND every well-formed issue is already claimed, finished or running - the queue is genuinely empty, not mis-parsed`
        : `${q.missingBoundary} of ${q.skips} skipped candidates (${share}%) were refused for missingBoundary - the backlog is full, but most of it is reports rather than work orders`,
      empty
        ? 'Do not repair formatting - it will produce nothing. The swarm has eaten everything it can read. New food needs an owner decision on the 23 repairable briefs, or new briefs written as work orders.'
        : 'Run `tri backlog` before acting: it splits the refusals into briefs that are one heading short (repair those) and reports that need a decision (do not paste a boundary onto them). Do not write new findings - that is what filled it.',
      `skipSummary.missingBoundary=${q.missingBoundary} of skippedCount=${q.skips}` +
        (delegable === null ? '; delegableNow unmeasured (needs a deep cycle)' : `; delegableNow=${delegable}`),
    ))
  }

  // --- 6. a lock held by a dead process -----------------------------------
  if (lk && lk.held && lk.pidAlive === false) {
    out.push(A(
      'lock-held-by-dead-pid', 'high',
      `loop.lock is held by ${lk.holder} (pid ${lk.pid})`,
      `that process is gone; the lock has been held ${lk.ageMin} min and nothing will release it`,
      'tri iter-unlock - but only after confirming no timer is mid-run.',
      `pid ${lk.pid} not in the process table`,
    ))
  }
  if (lk && lk.held && lk.ageMin !== null && lk.ageMin > 60) {
    out.push(A(
      'lock-held-too-long', 'medium',
      `loop.lock is held by ${lk.holder}`,
      `for ${lk.ageMin} minutes, which is longer than any honest run in this directory`,
      'Add an age-based takeover to loop.mjs: a lock older than 60 min is stale regardless of pid liveness, because heal and feed hand the lock between processes.',
      `lock age ${lk.ageMin}m`,
    ))
  }

  // --- 7. the volume ------------------------------------------------------
  const disk = r.disk.v ? r.disk.v.percentUsed : null
  if (disk !== null && disk >= DISK_WARN) {
    // THE WORKTREE COUNT USED TO STAND BESIDE THE PERCENTAGE AND READ AS ITS
    // CAUSE. It was measured on 2026-09-13 and it is not: 19 worktrees, 2.5G,
    // against 384G used. The repair that sentence invited - delete them - would
    // have freed six tenths of one percent and could have destroyed a bee's
    // uncommitted work. So the share is stated when it is known, and its
    // absence is stated when it is not.
    const d = r.disk.v
    const share = d.ownPercentOfUsed !== null && d.ownPercentOfUsed !== undefined
      ? `the loop's own worktrees are ${d.ownGB}G of that, ${d.ownPercentOfUsed}% of what is used`
      : 'how much of it the loop owns was not measured this run (needs --deep)'
    const repair = d.ownPercentOfUsed !== null && d.ownPercentOfUsed !== undefined && d.ownPercentOfUsed < 5
      ? `removing every worktree would free ${d.ownGB}G and would not move this number. The space is somebody else's: measure the volume before deleting anything, and treat this as a report.`
      : 'tri reap-local to report, then remove CLEAN worktrees only - never --force, because git refusing a dirty tree is the safety.'
    out.push(A(
      'volume-near-full', 'high',
      'the loop has disk to work with',
      `the volume is ${disk}% used; ${share}; bees die at 0 seconds when git worktree add cannot write`,
      repair,
      d.raw,
    ))
  }

  // --- 8. declared but unwired gates --------------------------------------
  if (gt && gt.wired !== null && gt.declared > gt.wired) {
    out.push(A(
      'gates-declared-not-wired', 'medium',
      `make check declares ${gt.declared} gates, which reads as ${gt.declared} gates guarding the branch`,
      `${gt.wired} of them appear in a workflow CI will load; ${gt.declared - gt.wired} run only on a developer's Mac`,
      'For each unwired gate, either wire it or write the reason next to it. A gate that skips silently when its input is absent must NOT be wired - it manufactures green.',
      `declared=${gt.declared} wired=${gt.wired} workflows=${gt.workflows}`,
    ))
  }

  // --- 9. unlanded work in the tree ---------------------------------------
  if (g && g.untrackedSource > 0) {
    out.push(A(
      'untracked-source-in-tree', 'medium',
      'the working tree is the record of what this project has done',
      `${g.untrackedSource} untracked source file(s) and ${g.modifiedSource} modified one(s) exist only on this disk`,
      'Classify each: output (ignore it) or work (commit it). A worktree reaper removing a tree takes uncommitted work with it.',
      `untrackedSource=${g.untrackedSource} modifiedSource=${g.modifiedSource}`,
    ))
  }

  // --- 10. the branch has drifted from its own remote ---------------------
  if (g && g.behind !== null && g.behind > 50) {
    out.push(A(
      'branch-far-behind-remote', 'high',
      `work is being done on ${g.branch}`,
      `that branch is ${g.behind} commits behind its own remote and ${g.ahead} ahead - measurements taken here describe a tree nobody else has`,
      'Measure against origin, not HEAD, for any claim about what ships. Rebase or state the divergence in every report.',
      `ahead ${g.ahead} behind ${g.behind}`,
    ))
  }

  // --- 11. the ledger moves but records only no-ops ------------------------
  if (led && led.kinds) {
    const noop = ['feed-skipped', 'land', 'reap-local']
    const total = Object.values(led.kinds).reduce((a, b) => a + b, 0)
    const noopCount = noop.reduce((a, k) => a + (led.kinds[k] || 0), 0)
    if (total >= 10 && noopCount / total > 0.8) {
      out.push(A(
        'ledger-records-only-noops', 'medium',
        'the ledger is appending, which reads as progress',
        `${noopCount} of the last ${total} entries in 24h are ${noop.join('/')} - housekeeping that changed nothing`,
        'Count entries that CHANGED something as the progress metric, not entries written. A busy no-op log is the signature of a loop with nothing to do.',
        JSON.stringify(led.kinds),
      ))
    }
  }

  return out
}

function render(list, r) {
  if (!list.length) return 'no anomaly found in this reading (' + r.at + ')'
  const rank = { blocker: 0, high: 1, medium: 2, low: 3 }
  const sorted = list.slice().sort((a, b) => (rank[a.severity] ?? 9) - (rank[b.severity] ?? 9))
  const out = [`${sorted.length} anomal${sorted.length === 1 ? 'y' : 'ies'}  (reading ${r.at})`, '']
  for (const a of sorted) {
    out.push(`[${a.severity.toUpperCase()}] ${a.id}`)
    out.push(`  claimed  ${a.claim}`)
    out.push(`  true     ${a.truth}`)
    out.push(`  repair   ${a.repair}`)
    out.push(`  evidence ${a.evidence}`)
    out.push('')
  }
  return out.join('\n')
}

const isMain = process.argv[1] && process.argv[1].endsWith('/anomaly.mjs')
if (isMain) {
  const argv = process.argv.slice(2)
  const WANT_JSON = argv.includes('--json')
  const WANT_RECORD = argv.includes('--record')
  let r
  try {
    r = takeReading()
  } catch (e) {
    process.stderr.write('anomaly: could not take a reading: ' + ((e && e.message) || e) + '\n')
    process.exit(1)
  }
  const list = findAnomalies(r)
  if (WANT_RECORD) {
    fs.mkdirSync(STATE_DIR, { recursive: true })
    fs.appendFileSync(LOG, JSON.stringify({ at: r.at, count: list.length, anomalies: list }) + '\n')
  }
  process.stdout.write((WANT_JSON ? JSON.stringify({ at: r.at, anomalies: list }, null, 2) : render(list, r)) + '\n')
  process.exit(list.length ? 2 : 0)
}
