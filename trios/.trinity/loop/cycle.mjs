#!/usr/bin/env node
// The driver. One iteration of the continuous loop: measure, cross-examine,
// draw, record - and, only when asked, repair.
//
// WHY THIS FILE EXISTS AT ALL.
//
// Iterations 1 to 96 were driven by a Claude cron job scoped to a chat session.
// When that session ended the job ended with it, and because nothing on the
// dashboard distinguished "the driver is scheduled" from "the timers fired
// recently", the loop looked alive for 147.9 hours while advancing zero
// iterations. This driver is a plain file on disk that any scheduler can run.
//
// THE CRON CONTRACT - what this process may and may not do.
//
// The operator's instruction was: the new cycle must not break the previous
// work. That is not a wish, it is a list, and every line below is enforced in
// code rather than promised in a comment:
//
//   1. NEVER MUTATES GIT. No commit, no push, no checkout, no reset, no stash,
//      no worktree add. `sense.mjs` reads git; nothing here writes it. A loop
//      that commits on a timer will one day commit a half-finished edit made by
//      a human who was still typing.
//   2. NEVER TOUCHES THE OLD LOOP'S FILES. `state.json`, `DASHBOARD.txt` and
//      `ledger.jsonl` belong to iterations 1-96. This driver writes only
//      `DASHBOARD2.*`, `state/cycle-readings.jsonl` and `cycle-ledger.jsonl`.
//      If the old driver ever comes back, it finds its world untouched.
//   3. RESPECTS THE COOPERATIVE LOCK. If `loop.lock` is held by a live process
//      this exits 0 immediately. It is a skip, not a failure - a cron job that
//      treats contention as an error will page someone every fifteen minutes.
//   4. USES THE LOOP'S OWN LOCK RULE, not a second one. Staleness is decided by
//      `isStale` in loop.mjs, imported - the same function heal and feed obey.
//      This file wrote its own copy for one afternoon and the copy was wrong in
//      three ways at once (see THE LOCK, below). A rule transcribed twice is
//      two rules that agree until someone edits one.
//   5. REPAIRS NOTHING UNLESS `--repair` IS PASSED, the repairs are enumerated,
//      and NONE OF THEM IS IMPLEMENTED HERE. Removing a worktree is delegated
//      to `reap-local.mjs`, which already knows the five questions that decide
//      it. See THE REPAIRS, below, for what happened when this file tried to
//      answer them itself.
//   6. IS IDEMPOTENT AND BOUNDED. Running it twice changes nothing the second
//      time beyond one more reading. Every subprocess has a timeout.
//
// Exit codes follow the house convention that a non-zero exit is often the
// answer rather than a failure:
//   0  clean, or skipped because another holder has the lock
//   2  anomalies were found (this is the normal state of an unhealthy loop)
//   1  the driver itself could not run
//
// Usage:
//   node cycle.mjs                 # measure, draw, record. Changes nothing else.
//   node cycle.mjs --repair        # additionally perform the safe repairs below
//   node cycle.mjs --quiet         # no stdout; for cron
//   node cycle.mjs --no-lock       # skip locking (for a manual read while a job runs)

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { takeReading } from './sense.mjs'
import { findAnomalies } from './anomaly.mjs'
import { renderDashboard } from './dash2.mjs'
import { render as renderHTML } from './dash-cc.mjs'
// The lock rule belongs to the loop, not to this file. Imported, never copied.
import { isStale, pidAlive } from './loop.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(DIR, '..', '..', '..')
const LOCK = path.join(DIR, 'loop.lock')
const CYCLE_LEDGER = path.join(DIR, 'cycle-ledger.jsonl')
const READINGS = path.join(DIR, 'state', 'cycle-readings.jsonl')
const NOTES = path.join(DIR, 'cycle-notes.json')
const HOLDER = 'cycle'

const DISK_REPAIR_AT = 88 // percent; below this, do not even ask about worktrees
// The peers that release the lock before they have finished. See acquire().
const PEERS = ['heal.mjs', 'feed.mjs']

const argv = process.argv.slice(2)
const REPAIR = argv.includes('--repair')
const QUIET = argv.includes('--quiet')
const NO_LOCK = argv.includes('--no-lock')

const say = (s) => { if (!QUIET) process.stdout.write(s + '\n') }

function sh(cmd, timeout = 20000, cwd = REPO) {
  return execSync(cmd, { cwd, timeout, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
}

// --- the lock --------------------------------------------------------------
//
// THREE DEFECTS LIVED HERE FOR ONE AFTERNOON. All three came from the same
// decision - writing a second lock implementation instead of using the loop's -
// and all three were found by an adversarial read, not by testing:
//
//   1. The record omitted `singleProcess`. `loop.mjs:isStale` reads that field
//      and, without it, refuses to reclaim a lock for the full 45 minutes NO
//      MATTER WHAT the holder's pid is doing. Every other holder opts in
//      (`heal.mjs:65`, `feed.mjs:116`). A cycle that died holding the lock would
//      have frozen heal for 4 fires and feed for 9.
//   2. `pidAlive` was a private copy that answered the OPPOSITE of loop.mjs's on
//      a lock with no pid field - mine said dead, the loop's says alive. Locks
//      with no pid field are not hypothetical; the one historic reclaim in
//      `ledger.jsonl` has exactly that shape.
//   3. Nothing released the lock on a signal. The only release was a `finally`,
//      and a `finally` does not run on SIGTERM - which is precisely how launchd
//      ends a job that overruns.
//
// So the rule is now imported and this file only decides WHO writes the file
// and WHEN. The one thing kept private is the release: `loop.mjs`'s unlinks
// unconditionally, and this one refuses to delete a lock that someone else has
// taken in the meantime.
//
// THE LOCK IS NOT SUFFICIENT ON ITS OWN, which is why `peerRunning()` exists.
// `heal.mjs:310` deliberately releases the lock before its last five reporting
// steps - up to five minutes of a heal that is still running and still writing
// its caches while `loop.lock` says the loop is free. A reader that trusts the
// lock alone will happily start measuring in the middle of that.

function readLock() {
  try { return JSON.parse(fs.readFileSync(LOCK, 'utf8')) } catch { return null }
}

/** Is a peer instrument running, whatever the lock says? */
function peerRunning() {
  for (const p of PEERS) {
    try {
      const out = sh('pgrep -f ' + JSON.stringify('trinity/loop/' + p) + ' || true', 10000)
      if (out.trim()) return p
    } catch { /* pgrep unavailable: fall back to the lock alone */ }
  }
  return null
}

/**
 * @returns {{ok: true, stole?: string} | {ok: false, why: string}}
 */
function acquire() {
  if (NO_LOCK) return { ok: true }
  const peer = peerRunning()
  if (peer) return { ok: false, why: `${peer} is running (the lock is not proof on its own)` }
  const held = readLock()
  if (held) {
    const ageMin = held.at ? (Date.now() - Date.parse(held.at)) / 60000 : null
    // The loop's rule, not ours: stale after 45 minutes whoever holds it, or -
    // for a holder that declared itself a single process - as soon as that
    // process is gone and a two-minute grace has passed.
    if (!isStale(held)) {
      return { ok: false, why: `held by ${held.holder} for ${ageMin === null ? '?' : Math.round(ageMin)}m, pid ${held.pid} ${pidAlive(held.pid) ? 'alive' : 'dead'}` }
    }
    var stole = `${held.holder} pid ${held.pid}, held ${ageMin === null ? '?' : Math.round(ageMin)}m, stale by loop.mjs's rule`
  }
  fs.writeFileSync(LOCK, JSON.stringify({
    holder: HOLDER,
    pid: process.pid,
    at: new Date().toISOString(),
    // Clause 1 above. Without this the loop cannot reclaim after a crash.
    singleProcess: true,
  }))
  return { ok: true, stole: typeof stole === 'string' ? stole : undefined }
}

function release() {
  if (NO_LOCK) return
  const held = readLock()
  // Only ever remove our own lock. If someone else took it while we ran, that
  // is their lock now and deleting it would be exactly the kind of quiet
  // sabotage this contract exists to prevent.
  if (held && held.holder === HOLDER && held.pid === process.pid) {
    try { fs.unlinkSync(LOCK) } catch { /* already gone */ }
  }
}

// Clause 3. launchd sends SIGTERM to a job it is stopping; `finally` never sees
// it. These three lines are what heal.mjs:74-76 does, for the same reason.
process.on('exit', release)
process.on('SIGINT', () => { release(); process.exit(130) })
process.on('SIGTERM', () => { release(); process.exit(143) })

// --- the repairs -----------------------------------------------------------
//
// Each repair answers one anomaly, is reversible or refusable by git itself,
// and returns a line describing what it actually did - never what it intended.

const REPAIRS = {
  /** A lock whose holder died leaves the loop unable to start for ever. */
  'lock-held-by-dead-pid'(r) {
    const lk = r.lock.v
    if (!lk || !lk.held || lk.pidAlive !== false) return null
    // We already hold it by the time repairs run - acquire() stole it.
    return `released a lock held by dead pid ${lk.pid} (${lk.holder})`
  },

  /**
   * Bees die at 0 seconds when `git worktree add` cannot write, so a full
   * volume has to be answered. It is answered by DELEGATION.
   *
   * THE VERSION THIS REPLACES ASKED ONE QUESTION - "is the worktree clean?" -
   * AND THAT QUESTION IS NOT THE ONE THAT DECIDES.
   *
   * Measured on 2026-09-12, with the disk at 92% and the gate therefore open:
   * `reap-local.mjs` reports 0 of 18 worktrees removable. The clean-but-
   * unmerged ones hold 1, 2, 3, 6, 7 and 10 commits whose subjects appear
   * nowhere in `origin/feat/queen-supervisor` - roughly 36 commits of bee work
   * that exists in no other checkout. `git worktree remove` without `--force`
   * refuses a DIRTY tree; it does not refuse a clean one whose branch is
   * unlanded. The comment "that refusal is the feature" was therefore true
   * about a refusal that would never have happened, and the repair would have
   * deleted all of it - including seven trees under ~/Documents that belong to
   * another agent's sessions entirely, because it never asked who owned them.
   *
   * `reap-local.mjs` was written on 2026-09-06 for this exact job and already
   * asks all five questions: inside the repo, readable, clean, merged (by
   * ancestry OR by squash-subject, because a squashed branch looks unmerged for
   * ever), and not holding deletions git would need --force to discard. So this
   * repair now runs that file and reports what IT did. One rule, one place.
   */
  'volume-near-full'(r) {
    const disk = r.disk.v
    if (!disk || disk.percentUsed < DISK_REPAIR_AT) return null
    const reaper = path.join(DIR, 'reap-local.mjs')
    if (!fs.existsSync(reaper)) return null
    let out
    try {
      // --reap removes only what its own report marks REAPABLE. When that set
      // is empty - as it is today - this does nothing at all, which is the
      // correct behaviour for a full disk holding work nobody has landed.
      out = sh('node ' + JSON.stringify(reaper) + ' --reap', 300000)
    } catch { return null }
    const m = out.match(/^removed (\d+) of (\d+)/m)
    if (!m || m[1] === '0') return null
    return `reap-local removed ${m[1]} of ${m[2]} merged and clean worktree(s)`
  },
}

function runRepairs(anomalies, r) {
  const done = []
  for (const a of anomalies) {
    const fn = REPAIRS[a.id]
    if (!fn) continue
    try {
      const line = fn(r)
      if (line) done.push({ id: a.id, did: line })
    } catch (e) {
      done.push({ id: a.id, did: 'repair failed: ' + String(e.message || e).slice(0, 120) })
    }
  }
  return done
}

// --- the record ------------------------------------------------------------

function appendJSONL(file, obj) {
  fs.mkdirSync(path.dirname(file), { recursive: true })
  fs.appendFileSync(file, JSON.stringify(obj) + '\n')
}

function nextCycleNumber() {
  try {
    const lines = fs.readFileSync(CYCLE_LEDGER, 'utf8').trim().split('\n').filter(Boolean)
    if (!lines.length) return 1
    return (JSON.parse(lines[lines.length - 1]).cycle || 0) + 1
  } catch { return 1 }
}

// --- main ------------------------------------------------------------------

const startedAt = new Date().toISOString()
const t0 = Date.now()

const lock = acquire()
if (!lock.ok) {
  // Rule 3: contention is a skip, not a failure.
  say('cycle: skipped - lock ' + lock.why)
  process.exit(0)
}

let exitCode = 0
try {
  if (lock.stole) say('cycle: took a stale lock (' + lock.stole + ')')

  const cycle = nextCycleNumber()

  // `tri idle` walks the swarm's whole round history and can take two minutes.
  // That is too slow to do every quarter hour and too valuable to never do, so
  // every fourth cycle goes deep. The reading records which mode it was, so a
  // missing idle figure reads as "not measured this cycle" and not as zero.
  const deep = argv.includes('--deep') || cycle % 4 === 0

  const reading = takeReading({ deep })
  const anomalies = findAnomalies(reading)
  const repairs = REPAIR ? runRepairs(anomalies, reading) : []

  // Re-measure after a repair, so the dashboard shows the world we left behind
  // rather than the one we found. A dashboard drawn from a pre-repair reading
  // is the same class of lie this whole cycle was built to end.
  const finalReading = repairs.length ? takeReading({ deep: false }) : reading
  const finalAnomalies = repairs.length ? findAnomalies(finalReading) : anomalies

  let notes = null
  try { notes = JSON.parse(fs.readFileSync(NOTES, 'utf8')) } catch { /* prose is optional */ }

  const coloured = renderDashboard(finalReading, finalAnomalies, notes)
  // The same reading and the SAME anomaly list, rendered wide as HTML. When run
  // standalone `dash-cc.mjs` re-derives the anomalies; here they are handed
  // over, so the two surfaces cannot drift into disagreeing about how many there
  // are. Wrapped, because a renderer that throws must not cost the cycle its
  // ledger entry: the reading is the product, the picture of it is not.
  try {
    fs.writeFileSync(path.join(DIR, 'DASHBOARD.html'), renderHTML(finalReading, { anomalies: finalAnomalies }))
  } catch (e) {
    say(`dashboard.html not written: ${e.message}`)
  }

  fs.writeFileSync(path.join(DIR, 'DASHBOARD2.ansi'), coloured + '\n')
  fs.writeFileSync(path.join(DIR, 'DASHBOARD2.txt'), coloured.replace(/\[[0-9;]*m/g, '') + '\n')

  appendJSONL(READINGS, finalReading)

  const bySev = finalAnomalies.reduce((m, a) => ((m[a.severity] = (m[a.severity] || 0) + 1), m), {})
  appendJSONL(CYCLE_LEDGER, {
    cycle,
    at: startedAt,
    ms: Date.now() - t0,
    mode: REPAIR ? 'repair' : 'observe',
    anomalies: finalAnomalies.length,
    severity: bySev,
    ids: finalAnomalies.map((a) => a.id),
    repairs,
    // The handful of numbers worth being able to graph later without
    // re-reading a whole reading blob.
    vitals: {
      iteration: finalReading.loop.iteration.v,
      staleHours: finalReading.loop.staleHours.v,
      driverCron: finalReading.driver.v ? finalReading.driver.v.claudeCron : null,
      beesRunning: finalReading.queen.v ? finalReading.queen.v.running : null,
      missingBoundary: finalReading.queen.v ? finalReading.queen.v.missingBoundary : null,
      diskPercent: finalReading.disk.v ? finalReading.disk.v.percentUsed : null,
    },
  })

  if (!QUIET) {
    process.stdout.write(coloured + '\n')
    for (const r of repairs) say('repaired: ' + r.did)
  }
  say(`cycle ${cycle}: ${finalAnomalies.length} anomalies in ${Math.round((Date.now() - t0) / 100) / 10}s`)

  exitCode = finalAnomalies.length ? 2 : 0
} catch (e) {
  process.stderr.write('cycle: driver failed: ' + (e && e.stack ? e.stack : String(e)) + '\n')
  exitCode = 1
} finally {
  release()
}

process.exit(exitCode)
