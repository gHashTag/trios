#!/usr/bin/env node
// Is the CLI the timers run the CLI that is in the repository?
//
// THE DEFECT THIS EXISTS FOR. `tri` is the front end of this whole loop - 1686
// lines of bash, 172 lines of help, the only entry point to some sixty
// instruments - and until 2026-09-13 it existed in exactly ONE place on this
// machine: ~/.local/bin/tri, untracked by any repository, with no copy
// anywhere. Four launchd timers invoke it by absolute path every 300 to 900
// seconds. A single `rm`, a bad `cp`, a disk that filled while a write was in
// flight, and the loop's entire front end would have been gone with nothing to
// restore it from. The file's own header records the near miss: on 2026-09-05 a
// timer read it MID-WRITE and bash reported a syntax error on a line that was
// correct, because open(path,'w') truncates before it writes.
//
// A backup that is never compared is a backup nobody knows is stale. So the
// repository copy is not a backup - it is a claim, and this is the gate that
// checks it.
//
// WHAT IT READS, AND WHY NOT `which`. The question is not "what would tri mean
// if I typed it" - it is "what file do the timers actually execute". Those are
// different questions on a machine where PATH differs between an interactive
// zsh and a launchd job, and the second question is the one that matters. So
// the live paths come from the DECLARATIONS: every ai.t27.trios-*.plist is
// converted by the platform's own parser (`plutil -convert json`) and the
// ProgramArguments strings are scanned for the executable. Regexing the raw XML
// would have read the long prose comments inside those plists as evidence,
// which is this repository's most-repeated defect and would have been the
// eleventh instance of it.
//
// `command -v tri` is read too, but as a SEPARATE reading. If the shell's tri
// and the timers' tri are different files, that is itself the finding.
//
// EXIT CODES:
//   0  every live file was measured and matches the tracked copy
//   1  DRIFT - measured difference, or the tracked copy is missing
//   2  UNMEASURED - no live file could be found or hashed. This is not
//      agreement. It used to be tempting to print "ok" here; an absent
//      measurement is not a passing one.
//
// Usage:
//   node tri-drift.mjs            # report
//   node tri-drift.mjs --adopt    # copy live -> tracked, BY HAND, never in a timer

import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const TRACKED = path.resolve(DIR, '..', '..', 'bin', 'tri')
const LEDGER = path.join(DIR, 'ledger.jsonl')
const AGENTS = path.join(process.env.HOME || '', 'Library', 'LaunchAgents')

function sh(cmd, timeout = 8000) {
  return execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout }).trim()
}

/**
 * The sha256 of a file, or null. Never a sentinel string: a hash that cannot be
 * read must not compare equal or unequal to anything.
 */
export function hashOf(file) {
  try {
    return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex')
  } catch {
    return null
  }
}

/**
 * Pull executable paths ending in /tri out of one plist's ProgramArguments.
 *
 * Exported and pure so the extraction can be tested against a real plist body
 * without a launchd, and so the prose-as-evidence branch can be proven absent:
 * a comment mentioning `tri cycle` is not in ProgramArguments and cannot reach
 * this function.
 */
export function triPathsInArgs(args) {
  const out = []
  for (const a of args || []) {
    for (const tok of String(a).split(/\s+/)) {
      const t = tok.replace(/^["']|["';]+$/g, '')
      if (/^\/\S*\/tri$/.test(t)) out.push(t)
    }
  }
  return out
}

/**
 * Does a job invoke `tri` through PATH rather than by absolute path?
 *
 * Then there is no file to hash, and which file runs depends on the PATH launchd
 * happens to hand the job - which is not the PATH an interactive shell has. All
 * four timers use an absolute path today, so this branch would never run in
 * production; it is exercised in the selftest instead, because a branch that has
 * never run is untested however carefully it was written.
 */
export function mentionsBareTri(args) {
  for (const a of args || []) {
    for (const tok of String(a).split(/\s+/)) {
      if (tok.replace(/^["']|["';]+$/g, '') === 'tri') return true
    }
  }
  return false
}

/**
 * Every path a launchd job is DECLARED to execute. Returns null - not [] - when
 * the declarations could not be read at all, because "no timer mentions tri" and
 * "I could not open the folder" are different answers.
 */
export function declaredPaths(dir = AGENTS) {
  let files
  try {
    files = fs.readdirSync(dir).filter((f) => /^ai\.t27\.trios-.*\.plist$/.test(f))
  } catch {
    return null
  }
  if (!files.length) return null
  const out = []
  for (const f of files) {
    const full = path.join(dir, f)
    let j
    try {
      j = JSON.parse(sh(`plutil -convert json -o - ${JSON.stringify(full)}`))
    } catch {
      continue
    }
    const job = f.replace(/\.plist$/, '')
    const paths = triPathsInArgs(j.ProgramArguments)
    for (const p of paths) out.push({ job, file: p })
    if (!paths.length && mentionsBareTri(j.ProgramArguments)) {
      out.push({ job, file: null, note: 'invokes tri through PATH - there is no file here to compare' })
    }
  }
  return out
}

export function shellPath() {
  try {
    const p = sh("zsh -lc 'command -v tri' 2>/dev/null")
    return p && path.isAbsolute(p) ? p : null
  } catch {
    return null
  }
}

export function reading() {
  const declared = declaredPaths()
  const shell = shellPath()
  const sources = []
  if (declared === null) sources.push({ how: 'launchd declarations', file: null, note: 'LaunchAgents unreadable' })
  else for (const d of declared) sources.push({ how: `launchd ${d.job}`, file: d.file, note: d.note || '' })
  sources.push({ how: 'shell command -v', file: shell, note: shell ? '' : 'tri not on the login PATH' })

  const tracked = hashOf(TRACKED)
  const rows = sources.map((s) => ({ ...s, hash: s.file ? hashOf(s.file) : null }))
  const live = rows.filter((r) => r.hash)
  const distinct = [...new Set(live.map((r) => r.hash))]
  return { tracked, trackedFile: TRACKED, rows, distinct, measured: live.length }
}

/**
 * The verdict, kept apart from the printing so a test can reach every branch.
 * `drift` is only ever a MEASURED difference.
 */
export function verdictOf(r) {
  if (!r.measured) return { code: 2, word: 'UNMEASURED', why: 'no live tri could be read - this is not agreement' }
  if (!r.tracked) return { code: 1, word: 'UNTRACKED', why: `no copy at ${r.trackedFile} - run --adopt` }
  if (r.distinct.length > 1) return { code: 1, word: 'SPLIT', why: `${r.distinct.length} different files are called tri` }
  if (r.distinct[0] !== r.tracked) return { code: 1, word: 'DRIFT', why: 'the live CLI differs from the tracked copy' }
  return { code: 0, word: 'same', why: 'the file the timers run is the file in the repository' }
}

const isMain = process.argv[1] && process.argv[1].endsWith('/tri-drift.mjs')
if (isMain) {
  const r = reading()
  const v = verdictOf(r)
  const short = (h) => (h ? h.slice(0, 12) : '-')

  if (process.argv.includes('--adopt')) {
    if (r.distinct.length !== 1) {
      console.log(`refusing to adopt: ${r.measured ? `${r.distinct.length} distinct live files` : 'nothing measured'}`)
      process.exit(2)
    }
    const src = r.rows.find((x) => x.hash === r.distinct[0]).file
    fs.mkdirSync(path.dirname(TRACKED), { recursive: true })
    fs.copyFileSync(src, TRACKED)
    fs.chmodSync(TRACKED, 0o755)
    const bytes = fs.statSync(TRACKED).size
    try {
      fs.appendFileSync(LEDGER, JSON.stringify({ at: new Date().toISOString(), kind: 'tri-adopt', from: src, bytes, sha: r.distinct[0] }) + '\n')
    } catch { /* the ledger is evidence, not a precondition */ }
    console.log(`adopted ${src} -> ${TRACKED}  (${bytes} bytes, ${short(r.distinct[0])})`)
    process.exit(0)
  }

  console.log('tri drift - is the CLI the timers run the CLI in the repository?\n')
  console.log(`  tracked  ${short(r.tracked)}  ${TRACKED}${r.tracked ? '' : '   MISSING'}`)
  for (const row of r.rows) {
    const mark = !row.file ? '??' : row.hash === null ? '!!' : row.hash === r.tracked ? 'ok' : '!!'
    console.log(`  ${mark}  ${short(row.hash)}  ${row.file || '(none)'}  [${row.how}]${row.note ? '  ' + row.note : ''}`)
  }
  console.log(`\n${v.word} - ${v.why}`)
  if (v.code === 1 && v.word === 'DRIFT') console.log('  node tri-drift.mjs --adopt   # after reading the diff, by hand')
  process.exit(v.code)
}
