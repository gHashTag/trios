#!/usr/bin/env node
// What the backlog actually contains, measured issue by issue.
//
// WHY THIS EXISTS.
//
// `/queen/status` says `refusal: "nothing to choose"` while `skipSummary`
// reports 446 candidates refused for `missingBoundary`. Read together those two
// facts invite one conclusion - "the backlog is full but unreadable, so repair
// the boundary sections" - and `anomaly.mjs` said exactly that before this file
// was written. The conclusion is wrong, and it is wrong in the expensive
// direction: it prices a formatting fix for work that needs a decision.
//
// The measurement on 2026-09-12, over all 588 open issues in one authenticated
// call:
//
//     HAS_BOUNDARY         38    6%   the Queen would accept it
//     BOUNDARY_NO_PATHS     2    0%   a heading whose section names no path
//     BRIEF_NO_BOUNDARY    23    4%   a real brief, one heading short
//     REPORT              373   63%   an analysis, not a work order
//     NO_HEADINGS         152   26%   free prose
//
// So the swarm is not blocked by a parser. 89% of the open backlog is writing
// ABOUT the system rather than instructions to CHANGE it. A report has no
// boundary to repair, because nobody has yet decided what work it implies -
// inventing one is a design act wearing a formatting fix's clothes.
//
// That distinction is the whole point of this file, and it is why the classes
// are named after what the issue IS rather than after what it is missing.
//
// AND THEN THE SAME MISTAKE, ONE LAYER IN. The column above read "delegable
// today" for months of nothing happening. It is a CLASSIFICATION, not an
// AVAILABILITY, and the difference is the entire question the swarm is stuck
// on. Crossed against the live tick's own `skipSummary` the same afternoon:
//
//     HAS_BOUNDARY 38 | claimed + completed + dispatched 38
//     delegable and not already taken : NONE
//     taken but not HAS_BOUNDARY      : NONE
//
// Set equality, both directions empty. Every issue the Queen can read is
// already claimed, already finished, or running right now. `nothing to choose`
// was never a parser bug - it was the literal truth, and three separate
// diagnoses of the plumbing were written against a queue that does not exist.
//
// Which is why `--json` now carries `delegableNow`: any instrument that wants
// to advise on the swarm must be able to ask "is there food?" without repeating
// the mistake of reading a format check as an answer.
//
// THE RATE LIMIT, which is the other half of the story. `queen-tick.ts` fetches
// GitHub with no Authorization header - the comment there names the anonymous
// ceiling of 60 an hour and works around it by batching bodies into the list
// call, but never adds a token. `tri idle` reported 11 rounds FAILED, every one
// of them "GitHub returned 403", from `openIssues`'s own throw. This file uses
// `gh`, which carries the operator's token, and pulls all 590 in a single call.
//
// THIS FILE NEVER WRITES TO GITHUB. It proposes; a human applies. The standing
// instruction from the previous session is that the old backlog is not to be
// touched pending the owner's answers, and a triage tool that starts editing
// issue bodies on a timer is precisely how that instruction gets forgotten.
//
// Usage:
//   node backlog.mjs                 # the table above, plus the repairable list
//   node backlog.mjs --json          # machine-readable
//   node backlog.mjs --record        # append to state/backlog-readings.jsonl
//   node backlog.mjs --propose <dir> # write one proposed boundary block per
//                                    # BRIEF_NO_BOUNDARY issue, for review

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const READINGS = path.join(DIR, 'state', 'backlog-readings.jsonl')
const REPO = process.env.QUEEN_REPO || 'gHashTag/t27'

const argv = process.argv.slice(2)
const WANT_JSON = argv.includes('--json')
const RECORD = argv.includes('--record')
const proposeAt = argv.indexOf('--propose')

// Both headings open a boundary section. Issues written before 2026-08-19 are
// in Russian - see L3 in trios/CLAUDE.md - and the server's parser accepts both,
// so this must too or it would report a gap the Queen does not have.
const BOUNDARY = /^##\s*(Boundary|Границы)/m
// A brief declares what it will deliver. These are the headings the recent
// well-formed briefs actually use; matched case-insensitively because the
// backlog spans two authors and a year.
const SCOPE = /^##\s*(Scope and deliverables|Scope|Deliverables|Acceptance)/im
const HEADING = /^##\s+(.+)$/gm

/**
 * The Queen's own boundary predicate, ported line for line from
 * `agent-server/apps/server/src/api/services/queen-tick.ts:boundaryPathsOf`.
 *
 * THE FIRST VERSION OF THIS FILE ASKED A DIFFERENT QUESTION THAN THE QUEEN AND
 * PRESENTED THE ANSWER AS HERS. It tested for the HEADING and called the result
 * "delegable today". The Queen tests for PATHS: `boundaryPathsOf` walks the
 * section and keeps tokens containing `/` or ending in an extension, and
 * `delegatable` is `boundary.length > 0`. A section with a heading and no path
 * is counted by the server under `missingBoundary` - a label that names the
 * heading and measures the paths.
 *
 * The two issues that exposed it, both classified HAS_BOUNDARY here and skipped
 * as missingBoundary there on 2026-09-12:
 *
 *   #2059  "## Boundary correction" - a correction to a published property
 *          count. Not a scope declaration at all, and `startsWith('## Boundary')`
 *          cannot tell the difference. The server's parser has the same blind
 *          spot, which is why this port keeps it rather than fixing it here:
 *          two parsers that disagree are worse than one that is wrong.
 *   #2036  "## Boundary" exactly - prose saying which workflows are outside the
 *          campaign. A real boundary in English, naming no path.
 *
 * So `HAS_BOUNDARY` now means what the Queen means, and the sections that have
 * a heading without a path get their own class instead of being counted as work.
 */
export function boundaryPathsOf(body) {
  const paths = []
  let inside = false
  for (const raw of String(body || '').split('\n')) {
    const line = raw.trim()
    if (line.startsWith('## ')) {
      if (inside) break
      inside = line.startsWith('## Boundary') || line.startsWith('## Границы')
      continue
    }
    if (!inside || line.length === 0) continue
    for (const token of line.split(/\s+/)) {
      const cleaned = token.replace(/^[`"'(]+/, '').replace(/[`"'.,;:!?)]+$/, '')
      if (cleaned.includes('/') || /\.\w{1,10}$/.test(cleaned)) {
        paths.push(cleaned)
        break
      }
    }
  }
  return paths
}

const CLASSES = ['HAS_BOUNDARY', 'BOUNDARY_NO_PATHS', 'BRIEF_NO_BOUNDARY', 'REPORT', 'NO_HEADINGS']

/** @returns {'HAS_BOUNDARY'|'BOUNDARY_NO_PATHS'|'BRIEF_NO_BOUNDARY'|'REPORT'|'NO_HEADINGS'} */
export function classify(body) {
  const b = body || ''
  if (BOUNDARY.test(b)) return boundaryPathsOf(b).length > 0 ? 'HAS_BOUNDARY' : 'BOUNDARY_NO_PATHS'
  HEADING.lastIndex = 0
  if (!HEADING.test(b)) return 'NO_HEADINGS'
  if (SCOPE.test(b)) return 'BRIEF_NO_BOUNDARY'
  return 'REPORT'
}

function headingsOf(body) {
  const out = []
  let m
  const re = /^##\s+(.+)$/gm
  while ((m = re.exec(body || '')) !== null) out.push(m[1].trim())
  return out
}

/**
 * Paths an issue already names in its prose, which is the raw material for a
 * proposed boundary. Deliberately NOT a boundary: it is a list of things the
 * text mentions, and turning that into ownership is the judgement this file
 * refuses to make on anyone's behalf.
 */
function pathsMentioned(body) {
  const seen = new Set()
  const re = /[`"']?((?:[\w.\-]+\/)+[\w.\-]+\.\w{1,10})[`"']?/g
  let m
  while ((m = re.exec(body || '')) !== null) {
    const p = m[1]
    if (p.startsWith('http') || p.includes('://')) continue
    seen.add(p)
    if (seen.size >= 12) break
  }
  return [...seen]
}

export function readBacklog() {
  // PAGINATED, in pages of 100, and not because of a rate limit.
  //
  // `gh issue list --limit 900` asks for ~600 issue bodies in one response. It
  // succeeded once and then failed three times in a row with "stream error:
  // CANCEL" and "connection reset by peer" - the payload is simply large enough
  // that a mid-stream reset is an ordinary event. Three retries of the same
  // oversized request is not a fix, it is the same bet placed again, so the
  // request itself got smaller.
  //
  // `gh api --paginate` walks pages of 100 and carries the operator's token, so
  // the ceiling here is 5000 an hour rather than the anonymous 60 that makes
  // `queen-tick.ts` throw "GitHub returned 403".
  //
  // `/issues` returns pull requests too; a PR is not backlog.
  let raw
  for (let attempt = 1; ; attempt++) {
    try {
      raw = execSync(
        `gh api "repos/${REPO}/issues?state=open&per_page=100" --paginate` +
          ` --jq '.[] | select(.pull_request == null) | {number, title, body}'`,
        { encoding: 'utf8', timeout: 300000, maxBuffer: 128 * 1024 * 1024 },
      )
      break
    } catch (e) {
      if (attempt >= 3) throw e
    }
  }
  const issues = raw.trim().split('\n').filter(Boolean).map((l) => JSON.parse(l))
  const buckets = Object.fromEntries(CLASSES.map((c) => [c, []]))
  for (const i of issues) {
    const cls = classify(i.body)
    buckets[cls].push({ number: i.number, title: (i.title || '').slice(0, 120) })
  }
  return { at: new Date().toISOString(), repo: REPO, total: issues.length, buckets, issues }
}

const QUEEN = process.env.TRIOS_QUEEN_STATUS || 'https://trios-agent-server-production.up.railway.app/queen/status'

/**
 * How many classified-delegable issues are ACTUALLY available to a bee.
 *
 * A classification says an issue is well-formed. Availability says nobody has
 * it. Conflating the two is what let this file report 40 delegable issues at a
 * swarm that had already taken all 40, and it is the reason the column exists.
 *
 * Returns null - never 0 - when the Queen cannot be reached. Zero is a measured
 * claim that the backlog is empty; null says nobody asked. A dashboard that
 * prints 0 for "could not measure" is how a swarm gets declared starving
 * because Railway was restarting.
 *
 * @returns {{delegable: number, taken: number, numbers: number[]}|null}
 */
function delegableNow(hasBoundary) {
  let q
  try {
    q = JSON.parse(execSync(`curl -sS -m 20 ${JSON.stringify(QUEEN)}`, { encoding: 'utf8', timeout: 30000 }))
  } catch { return null }
  const ss = q && q.lastTick && q.lastTick.skipSummary
  if (!ss) return null
  // The server caps each skip list at `skipIssueListCap` (25 today). `claimed`
  // and `completed` are small enough to arrive whole; if either is truncated the
  // answer would be an over-count of what is free, so refuse rather than guess.
  const truncated = (ss.claimed && ss.claimed.more) || (ss.completed && ss.completed.more)
  if (truncated) return null
  const taken = new Set([
    ...((ss.claimed && ss.claimed.issues) || []),
    ...((ss.completed && ss.completed.issues) || []),
  ])
  const running = q.dispatches && q.dispatches.latest
  if (running && running.issue && !running.finishedAt) taken.add(running.issue)
  const free = hasBoundary.map((i) => i.number).filter((n) => !taken.has(n)).sort((a, b) => b - a)
  return { delegable: free.length, taken: taken.size, numbers: free.slice(0, 25) }
}

function proposalFor(issue) {
  const paths = pathsMentioned(issue.body)
  const lines = []
  lines.push(`# Proposed boundary for ${REPO}#${issue.number}`)
  lines.push(`# ${(issue.title || '').slice(0, 100)}`)
  lines.push('#')
  lines.push('# NOT APPLIED. Paste the block below into the issue body only after')
  lines.push('# checking that these paths are the ones this brief should own -')
  lines.push('# the list is what the text MENTIONS, which is not the same thing.')
  lines.push('#')
  lines.push(`# Declared headings: ${headingsOf(issue.body).join(' | ') || '(none)'}`)
  lines.push('')
  lines.push('## Boundary')
  lines.push('')
  if (paths.length) {
    for (const p of paths) lines.push(`- \`${p}\``)
  } else {
    lines.push('- (no path appears in the issue text - this one needs a human)')
  }
  lines.push('')
  return lines.join('\n')
}

// ---------------------------------------------------------------------------

if (process.argv[1] && process.argv[1].endsWith('/backlog.mjs')) {
  let r
  try {
    r = readBacklog()
  } catch (e) {
    process.stderr.write('backlog: could not read the backlog: ' + String(e.message || e).slice(0, 300) + '\n')
    process.stderr.write('backlog: `gh auth status` is the first thing to check.\n')
    process.exit(1)
  }

  const avail = delegableNow(r.buckets.HAS_BOUNDARY)
  const summary = {
    at: r.at,
    repo: r.repo,
    total: r.total,
    counts: Object.fromEntries(CLASSES.map((c) => [c, r.buckets[c].length])),
    repairable: r.buckets.BRIEF_NO_BOUNDARY.map((i) => i.number).sort((a, b) => b - a),
    delegableNow: avail ? avail.delegable : null,
    delegableNumbers: avail ? avail.numbers : null,
  }

  if (RECORD) {
    fs.mkdirSync(path.dirname(READINGS), { recursive: true })
    fs.appendFileSync(READINGS, JSON.stringify(summary) + '\n')
  }

  if (proposeAt >= 0) {
    const outDir = argv[proposeAt + 1] || path.join(DIR, 'state', 'boundary-proposals')
    fs.mkdirSync(outDir, { recursive: true })
    const wanted = new Set(r.buckets.BRIEF_NO_BOUNDARY.map((i) => i.number))
    let n = 0
    for (const issue of r.issues) {
      if (!wanted.has(issue.number)) continue
      fs.writeFileSync(path.join(outDir, `${issue.number}.md`), proposalFor(issue))
      n++
    }
    process.stdout.write(`wrote ${n} proposals to ${outDir} - none applied\n`)
  }

  if (WANT_JSON) {
    process.stdout.write(JSON.stringify(summary, null, 2) + '\n')
  } else {
    const pct = (n) => String(Math.round((100 * n) / r.total)).padStart(3) + '%'
    process.stdout.write(`backlog of ${r.repo}: ${r.total} open\n\n`)
    const note = {
      HAS_BOUNDARY: 'the Queen would accept it - a boundary naming paths',
      BOUNDARY_NO_PATHS: 'a boundary heading whose section names no path',
      BRIEF_NO_BOUNDARY: 'a real brief, one heading short',
      REPORT: 'an analysis - needs a decision, not a heading',
      NO_HEADINGS: 'free prose',
    }
    for (const c of CLASSES) {
      const v = r.buckets[c]
      process.stdout.write(`  ${c.padEnd(20)}${String(v.length).padStart(4)}  ${pct(v.length)}   ${note[c]}\n`)
    }
    process.stdout.write('\nrepairable now (add one heading): ' + (summary.repairable.join(' ') || 'none') + '\n')
    // The line that matters, and the one the table alone cannot give.
    if (avail === null) {
      process.stdout.write('available to a bee right now: -  (the Queen did not answer; this is not zero)\n')
    } else if (avail.delegable === 0) {
      process.stdout.write(`available to a bee right now: 0  (all ${r.buckets.HAS_BOUNDARY.length} are claimed, finished or running)\n`)
    } else {
      process.stdout.write(`available to a bee right now: ${avail.delegable}  ${avail.numbers.join(' ')}\n`)
    }
  }

  // 2 when the backlog cannot feed the swarm: fewer issues AVAILABLE than the
  // fleet can run. This used to count HAS_BOUNDARY, which is a format check, so
  // it read 40 and exited 0 while the swarm sat idle with nothing to take.
  // Non-zero is the answer, not a failure; `null` means unmeasured, and an
  // unmeasured backlog is not a passing one.
  process.exit(summary.delegableNow === null || summary.delegableNow < 8 ? 2 : 0)
}
