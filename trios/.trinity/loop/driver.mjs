#!/usr/bin/env node
// What, if anything, fires an iteration - measured once, for both generations.
//
// THE DEFECT THIS EXISTS FOR, PART ONE: TWO CONSTANTS FOR ONE QUANTITY.
// This reading lived twice - `sense.mjs driver()` and `loop.mjs
// driverReading()` - and the second carried a comment saying it "agrees with
// sense.mjs by construction". It did not agree by construction. It agreed by
// transcription, which is the repo's signature defect: a hand-copied structure
// that is identical until somebody edits one copy. Both copies are now this
// file, so the agreement is the kind a compiler can check.
//
// THE DEFECT THIS EXISTS FOR, PART TWO: AN ASSERTED ABSENCE.
// Both copies read `.claude/scheduled_tasks.json` and did this:
//
//     out.claudeCron = Array.isArray(j.tasks) ? j.tasks.length : 0
//
// A file that PARSES but whose shape has moved - `scheduledTasks` instead of
// `tasks`, or a bare array at the root - took the `: 0` branch and reported
// ZERO SCHEDULED TASKS. That is not what was measured. What was measured is
// "this file no longer says what I know how to read", and the honest value for
// that is null. The difference is not cosmetic: `claudeCron === 0` is the
// whole meaning of `driver GONE` in the dashboard header and the whole meaning
// of `loop.mjs driver` exiting 1, so a key rename in somebody else's file
// would have printed a red, confident, entirely unmeasured verdict.
//
// Same for launchd. `Number(x) || 0` turns every unparseable answer - a
// missing launchctl, a permissions error, an empty string - into a measured
// zero. NaN is not zero. It is null.
//
// WHAT THE THREE FIELDS MEAN, since they fail independently:
//   claudeCron - a Claude scheduled task. THIS is the one that ran iterations
//                1-96 and the one that silently went away; a session-only job
//                dies with the session that made it and leaves nothing behind.
//   crontab    - the user crontab. On this machine it holds `make doctor`,
//                which is housekeeping and drives no iteration.
//   launchd    - the ai.t27.trios-* timers: feed, heal, cycle, cycle-heal.
//                Housekeeping too, and four of them being alive says nothing
//                about whether an iteration will ever open.
//
// So `claudeCron === 0` is the driver being GONE while the other two are
// healthy - which is exactly the state this loop has been in for six days.
//
// Usage:
//   node driver.mjs          # prints the reading; exits 1 when the driver is gone

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(DIR, '..', '..', '..')

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

/**
 * Count the tasks in a parsed scheduled_tasks.json, or say it was not measured.
 *
 * Exported so a test can reach the branch that mattered without writing a file
 * into the tree the loop is reading. Returns null - never 0 - for any shape
 * this does not recognise.
 */
export function countTasks(j) {
  if (Array.isArray(j)) return j.length
  if (j && typeof j === 'object' && Array.isArray(j.tasks)) return j.tasks.length
  return null
}

/**
 * A count from command output, or null. NaN is not zero - and neither is the
 * empty string, which was the first thing this function got wrong. `Number('')`
 * is 0 and it is finite, so the obvious rewrite of `Number(x) || 0` reproduced
 * the very defect it replaced for the one input that matters most: a command
 * that printed nothing at all. `grep -c` printing "0" is a measurement; a
 * command printing nothing is a missing measurement.
 */
export function countLines(text) {
  const s = String(text).trim()
  if (!s) return null
  const n = Number(s)
  return Number.isFinite(n) ? n : null
}

/**
 * The one reading. `run` and `read` are injectable so the failure branches can
 * be exercised without a real crontab, a real launchd or a real file.
 */
export function driverReading(run = shRead, read = fs.readFileSync) {
  const out = { claudeCron: null, crontab: null, launchd: null, at: new Date().toISOString() }
  try {
    out.claudeCron = countTasks(JSON.parse(read(path.join(REPO, '.claude', 'scheduled_tasks.json'), 'utf8')))
  } catch { out.claudeCron = null }
  try {
    const ct = run('crontab -l 2>/dev/null || true')
    out.crontab = ct.split('\n').filter((l) => l.trim() && !l.trim().startsWith('#')).length
  } catch { out.crontab = null }
  try {
    out.launchd = countLines(run("launchctl list 2>/dev/null | grep -c 'ai.t27.trios-' || true"))
  } catch { out.launchd = null }
  return out
}

/**
 * Is the driver gone? Only a MEASURED zero says yes.
 *
 * An unmeasured claudeCron is not a gone driver and not a live one - it is a
 * question, and the caller renders it as one rather than as a red verdict.
 */
export function driverGone(d) {
  return d.claudeCron === 0
}

const isMain = process.argv[1] && process.argv[1].endsWith('/driver.mjs')
if (isMain) {
  const d = driverReading()
  const n = (v) => (v === null ? '?' : v)
  process.stdout.write(`claude-cron ${n(d.claudeCron)}   crontab ${n(d.crontab)}   launchd ${n(d.launchd)}   read ${d.at}\n`)
  if (driverGone(d)) {
    process.stdout.write('driver GONE - nothing schedules an iteration\n')
    process.exit(1)
  }
  if (d.claudeCron === null) {
    process.stdout.write('driver UNMEASURED - scheduled_tasks.json did not parse into a shape this knows\n')
    process.exit(2)
  }
  process.stdout.write(`driver alive - ${d.claudeCron} claude scheduled task(s)\n`)
}
