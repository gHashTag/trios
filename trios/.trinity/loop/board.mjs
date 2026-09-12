#!/usr/bin/env node
// Which GitHub board do the loop instruments actually read, and is it moving?
//
// WHY THIS EXISTS.
//
// Fifteen instruments in this directory open with the same line:
//
//     const REPO = process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios'
//
// and TRIOS_ISSUE_REPO is set nowhere - not in the shell, not in either launchd
// plist, not in `tri`. So the default is not a fallback, it is the value. Every
// one of those instruments measures gHashTag/trios and prints the answer with
// no note of which board it asked.
//
// Meanwhile `backlog.mjs` - the newest of them - opens with
// `process.env.QUEEN_REPO || 'gHashTag/t27'`, and the live Queen dispatches
// there. Measured on 2026-09-12:
//
//     gHashTag/trios   newest issue #1673, created 2026-09-05T17:23:30Z
//     gHashTag/t27     newest issue #3591, created 2026-09-12T12:05:29Z
//
// The audit arm and the working arm are looking at different boards. An
// instrument that says "no such issue" about #3573 is not wrong about the
// issue; it is right about a repository nobody told it to stop reading.
//
// WHAT THIS FILE REFUSES TO DO.
//
// It does not pick a board. Whether gHashTag/trios was retired on purpose or
// abandoned by accident is an owner's decision, and the difference is invisible
// from here: a deliberately frozen board and a forgotten one produce byte-for-
// byte identical measurements. This file makes the split MEASURED and VISIBLE
// and then stops. Setting TRIOS_ISSUE_REPO is one line; knowing what to set it
// to is not this instrument's call.
//
// THE PATTERN, AND WHY IT IS AS NARROW AS IT IS.
//
// House rule 3: the recurring defect here is a regex that reads prose, or a
// token without its command, and calls it evidence. A bare search for the word
// `trios` matches a hundred innocent lines in this tree. Two independent
// narrowings are applied, and BOTH must hold:
//
//   1. THE SHAPE. `process.env.NAME || 'owner/name'` - the whole construct, not
//      the slug. A slug quoted in a comment is prose. A slug in a URL is a URL.
//   2. THE DECLARATION. NAME must carry `REPO` as a whole segment. This is the
//      axis that matters, and it is not decoration:
//
//        heal.mjs:286  process.env.HEAL_REPORT_DEADLINE_MS ?? 5 * 60 * 1000
//          REPORT contains REPO. Rule 1 alone already rejects it (no slug), but
//          rule 2 rejects it on the name, which is the durable reason.
//
//        land.mjs:43     process.env.LAND_BASE  || 'feat/queen-supervisor'
//        salvage.mjs:41  process.env.TRIOS_BASE || 'feat/queen-supervisor'
//        salvage.mjs:220 the same default, inline in a template
//          A BRANCH NAME HAS A SLASH IN IT. Rule 1 alone reports
//          `feat/queen-supervisor` as a third board - three false boards, in
//          this directory, today. Rule 2 is what kills them, and they are
//          pinned as selftest cases so nobody widens the pattern back.
//
//      Rules 1 and 2 are independent, so a widening of either is still caught
//      by the other. The three branch lines are not silently dropped: they are
//      printed under `slug-shaped defaults that are NOT repo slots`, because a
//      filter nobody can see is a filter nobody can check.
//
// KNOWN BLIND SPOTS, stated rather than discovered later. A repo slot named
// without `REPO` in it (`QUEEN_BOARD`) is missed. A host-qualified slug
// (`github.com/gHashTag/trios`) is missed. Both are pinned by selftest cases
// asserting the miss, so the boundary is documented in runnable form.
//
// NUMBERS ARE MEASURED. A board this file could not reach carries null, and
// null renders `-`. It never renders 0: "gh could not answer" and "the board
// has no open issues" are opposite findings, and printing 0 for the first is
// the worst defect class in this repository.
//
// THE EXIT CODE IS THE REPORT. Non-zero here is a measured condition, not a
// crash. Do not make it zero; answer it.
//
//   0  one issue board, and it is moving (or quiet with nothing to compare to)
//   1  the scan found nothing - suspect this file, not the tree
//   2  the instrument set is split, or reads a stale board while another moves
//
// THIS FILE WRITES NOTHING. No issue, no file, no git. It reads and reports.
//
// Usage:
//   node board.mjs              # the human rendering
//   node board.mjs --json       # machine-readable, for other instruments
//   node board.mjs --no-gh      # source scan only, no network
//   node board.mjs --selftest   # prove the pattern still catches and still refuses

import fs from 'node:fs'
import path from 'node:path'
import os from 'node:os'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))

// THIS FILE IS NOT PART OF THE POPULATION IT MEASURES, and the first run proved
// why. Its header quotes the defective line verbatim and its selftest carries a
// dozen fixtures of the same shape, so scanning itself produced 32 slots where
// the tree has 19, listed `board.mjs` as an instrument reading all three
// repositories, and - because one fixture is a bare literal with no role - moved
// gHashTag/trios out of the `issue` class entirely. The headline inverted: it
// reported gHashTag/t27 as the board the instrument set reads, when fifteen
// instruments read gHashTag/trios. A fixture is not tree state.
//
// Only this file is excluded, and only by exact name. `selftest.mjs` stays in
// the population: it holds no repo default today, and that is a measurement,
// not an assumption.
const SELF = 'board.mjs'

const argv = process.argv.slice(2)
const WANT_JSON = argv.includes('--json')
const NO_GH = argv.includes('--no-gh')
const SELFTEST = argv.includes('--selftest')

// A board that has not moved in this long is stale. Two days, because the loop
// authors issues on a timer measured in minutes - a board the swarm is actually
// working cannot go two days silent, and one that does has stopped being the
// board. Override for a slower cadence; it is a judgement, not a measurement.
const STALE_HOURS = Number(process.env.BOARD_STALE_HOURS || 48)

// ---------------------------------------------------------------------------
// The pattern.
// ---------------------------------------------------------------------------

// Rule 1, the shape. The quote is captured and back-referenced so `'x"` is not
// a string. The value must be exactly one `owner/name` between the quotes: the
// leading character class rejects an absolute path (`'/Users/...'`), and
// `[A-Za-z0-9._-]+` cannot cross a second slash, which rejects `'a/b/c'` and
// every URL (`https` is followed by `:`, not `/`).
//
// Written as literals rather than composed from a shared SLUG string. The first
// draft composed them, and the back-reference inside the fragment then pointed
// at group 1 of the ASSEMBLED pattern - the variable name - so the whole scan
// silently matched nothing and reported a clean tree. The selftest caught it on
// its first run, which is the entire argument for having one.
//
// groups: 1 name, 2 operator, 3 quote, 4 value
const ENV_DEFAULT = /process\.env\.([A-Za-z0-9_]+)\s*(\|\||\?\?)\s*(['"])([A-Za-z0-9][A-Za-z0-9._-]*\/[A-Za-z0-9._-]+)\3/g

// The same shape with no env door at all: `const REPO = 'gHashTag/trios'`.
// None exists in this directory today. It is scanned for anyway, because a
// hardcoded board with no environment override is how this defect comes back
// in a form the env-default scan cannot see.
//
// groups: 1 name, 2 quote, 3 value
const LITERAL_SLOT = /(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(['"])([A-Za-z0-9][A-Za-z0-9._-]*\/[A-Za-z0-9._-]+)\2/g

/**
 * Rule 2, the declaration: does this name carry REPO as a whole segment?
 *
 * Underscore and camelCase are both boundaries, so `TRIOS_CODE_REPO` and
 * `codeRepo` both qualify and `HEAL_REPORT_DEADLINE_MS` does not. Matching
 * `REPO` as a substring instead would accept every REPORT in the tree, which
 * is the same mistake as reading a token without its command.
 */
export function hasRepoSegment(name) {
  const segments = String(name || '')
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .toUpperCase()
    .split(/[^A-Z0-9]+/)
  return segments.includes('REPO') || segments.includes('REPOSITORY')
}

/**
 * Every repo slot and every near miss in one pass over one file's text.
 *
 * A `nearMiss` is a default that passes rule 1 and fails rule 2 - slug-shaped,
 * but not declared as a repository. They are returned rather than discarded so
 * the rendering can show what the narrowing removed.
 *
 * @param {string} text
 * @returns {{slots: Array<object>, nearMisses: Array<object>}}
 */
export function scanSource(text, file = '') {
  const slots = []
  const nearMisses = []
  const lines = String(text || '').split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    let m
    ENV_DEFAULT.lastIndex = 0
    while ((m = ENV_DEFAULT.exec(line)) !== null) {
      const [, name, op, , value] = m
      const rec = { file, line: i + 1, var: name, op, value, text: line.trim().slice(0, 160) }
      if (hasRepoSegment(name)) slots.push({ ...rec, kind: 'env-default', repo: value })
      else nearMisses.push(rec)
    }
    LITERAL_SLOT.lastIndex = 0
    while ((m = LITERAL_SLOT.exec(line)) !== null) {
      const [, name, , value] = m
      // Bindings only, and only REPO-declared ones. A bare literal scan would
      // report `'state/backlog-readings.jsonl'` as a repository; near misses
      // are deliberately NOT collected on this axis for that reason.
      if (!hasRepoSegment(name)) continue
      // `const REPO = process.env.X || 'slug'` matches both patterns. The env
      // form is the truer reading, so do not double-count it.
      if (/process\.env\./.test(line)) continue
      slots.push({ file, line: i + 1, var: name, op: '=', value, repo: value, kind: 'literal', text: line.trim().slice(0, 160) })
    }
  }
  return { slots, nearMisses }
}

// ---------------------------------------------------------------------------
// Roles. What a repo slot is FOR, read from the name that declares it.
// ---------------------------------------------------------------------------
//
// An issue board and a code repository are different things, and conflating
// them would report gHashTag/BrowserOS as a third board and turn a real split
// into noise. The table is explicit; anything absent from it is `unknown` and
// is reported as unknown rather than guessed into a class.
const ROLES = {
  TRIOS_ISSUE_REPO: 'issue',
  QUEEN_REPO: 'issue',
  // The name the deployed server reads (queen-tick.ts). Listed so that an
  // instrument adopting it is classified rather than flagged as unknown.
  TRIOS_GITHUB_REPO: 'issue',
  TRIOS_CODE_REPO: 'code',
}

const roleOf = (slot) => (slot.kind === 'env-default' ? ROLES[slot.var] || 'unknown' : 'unknown')

// ---------------------------------------------------------------------------
// Measurement.
// ---------------------------------------------------------------------------

/** @returns {{ok: true, out: string}|{ok: false, err: string}} */
function gh(args, timeout = 60000) {
  try {
    const out = execSync('gh ' + args, {
      encoding: 'utf8',
      timeout,
      stdio: ['ignore', 'pipe', 'pipe'],
      maxBuffer: 16 * 1024 * 1024,
    })
    return { ok: true, out: String(out).trim() }
  } catch (e) {
    return { ok: false, err: String((e && (e.stderr || e.message)) || e).split('\n')[0].slice(0, 200) }
  }
}

const hoursSince = (iso) => {
  if (!iso) return null
  const t = Date.parse(iso)
  if (!Number.isFinite(t)) return null
  return Math.round(((Date.now() - t) / 36e5) * 10) / 10
}

const maxIso = (...isos) => {
  const live = isos.filter((s) => s && Number.isFinite(Date.parse(s)))
  if (!live.length) return null
  return live.reduce((a, b) => (Date.parse(a) >= Date.parse(b) ? a : b))
}

/**
 * What GitHub says about one board. Every field is null when unmeasured.
 *
 * `exists` is three-valued on purpose: true, false (GitHub said 404), or null
 * (nobody got an answer). Collapsing the last two would let a dropped network
 * read as a deleted repository - the empty-versus-absent distinction this tree
 * has already been bitten by.
 */
export function measureBoard(repo) {
  const out = {
    repo,
    exists: null,
    archived: null,
    // Whether the repo has an issue tracker at all. gHashTag/BrowserOS does
    // not, and without this field its honest `newest issue -` reads as a failed
    // measurement instead of a repository that cannot have one. Absent and
    // empty are different findings.
    hasIssues: null,
    openIssues: null,
    newestIssue: null,
    newestIssueAt: null,
    lastIssueTouchedAt: null,
    pushedAt: null,
    lastActivityAt: null,
    staleHours: null,
    error: null,
  }
  if (NO_GH) {
    out.error = 'not measured (--no-gh)'
    return out
  }

  const meta = gh(`api "repos/${repo}" --jq '{pushedAt:.pushed_at,archived:.archived,hasIssues:.has_issues}'`)
  if (!meta.ok) {
    if (/HTTP 404|Not Found/i.test(meta.err)) out.exists = false
    out.error = meta.err
    return out
  }
  out.exists = true
  try {
    const j = JSON.parse(meta.out)
    out.pushedAt = j.pushedAt || null
    out.archived = typeof j.archived === 'boolean' ? j.archived : null
    out.hasIssues = typeof j.hasIssues === 'boolean' ? j.hasIssues : null
  } catch { /* leave null; a parse failure is not a measurement */ }

  // `gh issue list` excludes pull requests, which `/issues` does not. A PR is
  // not a dispatch, and counting one as board movement is how a dead board
  // looks alive.
  const created = gh(`issue list --repo ${repo} --state all --limit 1 --json number,createdAt`)
  if (created.ok) {
    try {
      const rows = JSON.parse(created.out)
      if (rows.length) {
        out.newestIssue = rows[0].number
        out.newestIssueAt = rows[0].createdAt || null
      }
    } catch { /* null */ }
  }

  const touched = gh(`issue list --repo ${repo} --state all --limit 1 --search "sort:updated-desc" --json number,updatedAt`)
  if (touched.ok) {
    try {
      const rows = JSON.parse(touched.out)
      if (rows.length) out.lastIssueTouchedAt = rows[0].updatedAt || null
    } catch { /* null */ }
  }

  // The search API counts issues excluding PRs in one call. `open_issues_count`
  // on the repo object includes PRs and would over-report by the PR backlog.
  const open = gh(`api "search/issues?q=repo:${repo}+type:issue+state:open&per_page=1" --jq .total_count`)
  if (open.ok && /^\d+$/.test(open.out)) out.openIssues = Number(open.out)

  out.lastActivityAt = maxIso(out.newestIssueAt, out.lastIssueTouchedAt, out.pushedAt)
  out.staleHours = hoursSince(out.lastActivityAt)
  return out
}

// ---------------------------------------------------------------------------
// Is the default actually what gets read?
// ---------------------------------------------------------------------------
//
// A default only decides anything when nothing overrides it. This looks in the
// places that would override it for the loop: this process, the launchd jobs
// that drive the loop, the shell rc files, and the `tri` dispatcher. It reports
// which files NAME the variable - naming is checkable, and "set to what, when
// launchd runs it" is not, from here.
function overridesFor(names) {
  const home = os.homedir()
  const candidates = [
    path.join(home, 'Library', 'LaunchAgents', 'ai.t27.trios-feed.plist'),
    path.join(home, 'Library', 'LaunchAgents', 'ai.t27.trios-heal.plist'),
    path.join(home, 'Library', 'LaunchAgents', 'ai.t27.trios-cycle.plist'),
    path.join(home, 'Library', 'LaunchAgents', 'ai.t27.trios-cycle-heal.plist'),
    path.join(home, '.zshrc'),
    path.join(home, '.zshenv'),
    path.join(home, '.zprofile'),
    path.join(home, '.local', 'bin', 'tri'),
  ]
  const present = []
  for (const f of candidates) {
    try { present.push({ file: f, text: fs.readFileSync(f, 'utf8') }) } catch { /* absent is fine */ }
  }
  return names.map((name) => ({
    var: name,
    inProcess: Object.prototype.hasOwnProperty.call(process.env, name) ? process.env[name] : null,
    namedIn: present.filter((p) => p.text.includes(name)).map((p) => path.basename(p.file)),
    searched: present.length,
  }))
}

// ---------------------------------------------------------------------------
// The reading.
// ---------------------------------------------------------------------------

export function readBoards() {
  let files
  try {
    files = fs.readdirSync(DIR).filter((f) => f.endsWith('.mjs') && f !== SELF).sort()
  } catch (e) {
    return { error: 'could not read ' + DIR + ': ' + String(e.message || e) }
  }

  const slots = []
  const nearMisses = []
  for (const f of files) {
    let text
    try { text = fs.readFileSync(path.join(DIR, f), 'utf8') } catch { continue }
    const r = scanSource(text, f)
    slots.push(...r.slots)
    nearMisses.push(...r.nearMisses)
  }
  for (const s of slots) s.role = roleOf(s)

  const byRepo = new Map()
  for (const s of slots) {
    if (!byRepo.has(s.repo)) byRepo.set(s.repo, { repo: s.repo, roles: new Set(), vars: new Set(), instruments: new Set(), slots: [] })
    const b = byRepo.get(s.repo)
    b.roles.add(s.role)
    b.vars.add(s.var)
    b.instruments.add(s.file)
    b.slots.push(s)
  }

  const boards = [...byRepo.values()].map((b) => ({
    repo: b.repo,
    // A repo named through two roles is itself a finding; report the set.
    role: b.roles.size === 1 ? [...b.roles][0] : [...b.roles].sort().join('+'),
    vars: [...b.vars].sort(),
    instruments: [...b.instruments].sort(),
    ...measureBoard(b.repo),
  }))
  boards.sort((a, b) => b.instruments.length - a.instruments.length || a.repo.localeCompare(b.repo))

  const names = [...new Set(slots.filter((s) => s.kind === 'env-default').map((s) => s.var))].sort()
  return {
    at: new Date().toISOString(),
    dir: DIR,
    staleThresholdHours: STALE_HOURS,
    filesScanned: files.length,
    slots,
    nearMisses,
    boards,
    overrides: overridesFor(names),
  }
}

/**
 * The conditions, derived from the reading. Each carries the evidence that
 * produced it, so a caller never has to re-derive the claim from the numbers.
 *
 * `moving` is three-valued like `exists`: true, false, or null for a board
 * whose age nobody measured. An unmeasured board is never called stale - that
 * would manufacture a finding out of a failed `gh` call.
 */
export function findConditions(r) {
  const conditions = []
  const issueBoards = r.boards.filter((b) => b.role === 'issue')
  const aged = (b) => (b.staleHours === null ? null : b.staleHours <= r.staleThresholdHours)

  if (!r.slots.length) {
    conditions.push({
      name: 'the scan found no repo slot at all',
      detail: `${r.filesScanned} file(s) read, 0 repo defaults matched. This directory had 19 on 2026-09-12, so a zero here is this instrument going blind, not the tree going clean.`,
      severity: 'blind',
    })
    return conditions
  }

  if (issueBoards.length > 1) {
    const names = issueBoards.map((b) => `${b.repo} (${b.instruments.length})`).join(', ')
    conditions.push({
      name: 'the instrument set is split across issue boards',
      detail: `${issueBoards.length} distinct issue boards named in one directory: ${names}. Instruments in the same loop are auditing different repositories, and none of them says which.`,
      severity: 'split',
    })
  }

  const moving = issueBoards.filter((b) => aged(b) === true)
  const stale = issueBoards.filter((b) => aged(b) === false)
  for (const s of stale) {
    for (const m of moving) {
      conditions.push({
        // Named exactly as the brief names it.
        name: 'two boards, one instrument set',
        detail:
          `${s.repo} last moved ${s.lastActivityAt || '-'} (${s.staleHours} h ago, over the ${r.staleThresholdHours} h threshold) ` +
          `while ${m.repo} last moved ${m.lastActivityAt || '-'} (${m.staleHours} h ago). ` +
          `${s.instruments.length} instrument(s) read the stale one, ${m.instruments.length} read the moving one.`,
        stale: s.repo,
        moving: m.repo,
        severity: 'stale-split',
      })
    }
  }

  const unknown = r.slots.filter((s) => s.role === 'unknown')
  if (unknown.length) {
    conditions.push({
      name: 'a repo slot this instrument cannot classify',
      detail:
        unknown.map((s) => `${s.file}:${s.line} ${s.var} -> ${s.repo}`).join('; ') +
        '. Its role is not in the ROLES table, so it counts toward no verdict. An unclassified board is unmeasured, and unmeasured is not a pass - add it to ROLES.',
      severity: 'unclassified',
    })
  }

  for (const b of r.boards) {
    if (b.exists === false) {
      conditions.push({ name: 'a named board does not exist', detail: `${b.repo} returned 404; ${b.instruments.length} instrument(s) point at it.`, severity: 'missing' })
    }
    if (b.archived === true) {
      conditions.push({ name: 'a named board is archived', detail: `${b.repo} is archived on GitHub; ${b.instruments.length} instrument(s) point at it.`, severity: 'archived' })
    }
  }

  return conditions
}

export function exitCodeFor(conditions) {
  if (conditions.some((c) => c.severity === 'blind')) return 1
  const answerable = ['split', 'stale-split', 'unclassified', 'missing', 'archived']
  return conditions.some((c) => answerable.includes(c.severity)) ? 2 : 0
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

const dash = (v) => (v === null || v === undefined ? '-' : String(v))

function age(hours) {
  if (hours === null || hours === undefined) return '-'
  if (hours < 1) return Math.round(hours * 60) + 'm ago'
  if (hours < 48) return hours + 'h ago'
  return Math.round((hours / 24) * 10) / 10 + 'd ago'
}

function render(r, conditions) {
  const L = []
  const issueBoards = r.boards.filter((b) => b.role === 'issue')
  const readBoard = issueBoards[0] || null

  L.push(`board: which GitHub board the loop instruments read, measured ${r.at}`)
  L.push(`  ${r.filesScanned} file(s) in ${r.dir}`)
  L.push(`  ${r.slots.length} repo slot(s) in ${new Set(r.slots.map((s) => s.file)).size} instrument(s), ${r.boards.length} distinct repo(s)`)
  L.push(`  stale threshold ${r.staleThresholdHours}h` + (NO_GH ? '   (--no-gh: nothing on this screen was measured against GitHub)' : ''))
  L.push('')

  for (const b of r.boards) {
    const head = `  ${b.repo.padEnd(20)} ${String(b.role + ' repo').padEnd(14)} ${String(b.instruments.length).padStart(2)} instrument(s)  via ${b.vars.join(', ')}`
    L.push(head)
    const exists = b.exists === null ? '-' : b.exists ? 'yes' : 'NO (404)'
    L.push(`      exists ${exists}   archived ${dash(b.archived)}   open issues ${dash(b.openIssues)}   newest issue ${b.newestIssue === null ? '-' : '#' + b.newestIssue}` +
      (b.hasIssues === false ? '   (issues are DISABLED on this repo - the dash is not a failed reading)' : ''))
    L.push(`      last activity ${dash(b.lastActivityAt)}   ${age(b.staleHours)}` +
      (b.staleHours === null ? '   (unmeasured - this is not "quiet")' : b.staleHours > r.staleThresholdHours ? '   STALE' : '   moving'))
    // `last activity` is the newest of three signals, and which one it came
    // from changes what it means: a code push to an issue board does not mean
    // anyone is dispatching there. Printed separately so the max cannot hide
    // an older issue clock behind a fresher commit.
    L.push(`        issue created ${dash(b.newestIssueAt)}   issue touched ${dash(b.lastIssueTouchedAt)}   code pushed ${dash(b.pushedAt)}`)
    if (b.error) L.push(`      gh said: ${b.error}`)
    L.push(`      read by: ${b.instruments.join(' ')}`)
    L.push('')
  }

  L.push('overrides - a default is only the value when nothing overrides it')
  for (const o of r.overrides) {
    const inProc = o.inProcess === null ? 'UNSET' : JSON.stringify(o.inProcess)
    const named = o.namedIn.length ? o.namedIn.join(', ') : `none of the ${o.searched} driver file(s)`
    L.push(`  ${o.var.padEnd(20)} this process: ${inProc.padEnd(10)} named in: ${named}`)
  }
  L.push('  (measured in THIS process. A launchd job carries its own environment,')
  L.push('   which is why the plists are read rather than assumed.)')
  L.push('')

  if (r.nearMisses.length) {
    L.push(`slug-shaped defaults that are NOT repo slots (${r.nearMisses.length}) - shown so the narrowing is checkable`)
    for (const n of r.nearMisses) {
      L.push(`  ${(n.file + ':' + n.line).padEnd(24)} ${n.var} -> ${JSON.stringify(n.value)}   (no REPO segment in the name)`)
    }
    L.push('')
  }

  if (readBoard) {
    const others = issueBoards.length - 1
    L.push(`the board the instrument set reads: ${readBoard.repo}` +
      `  (${readBoard.instruments.length} of ${issueBoards.reduce((n, b) => n + b.instruments.length, 0)} issue-board slots` +
      (others > 0 ? `, ${others} other board(s) named alongside it)` : ')'))
  } else {
    L.push('the board the instrument set reads: -  (no slot classified as an issue board)')
  }
  L.push('')

  L.push('VERDICT')
  if (!conditions.length) {
    L.push('  one issue board, and nothing contradicts it.')
  } else {
    for (const c of conditions) {
      L.push(`  ! ${c.name}`)
      for (const line of wrap(c.detail, 84)) L.push('    ' + line)
    }
    L.push('')
    L.push('  This instrument does not pick a board. A retired board and an abandoned')
    L.push('  one measure identically from here; which is authoritative is the owner\'s')
    L.push('  call. Answer it by setting the variable above in the launchd plists and')
    L.push('  in `tri`, not by editing fifteen defaults.')
  }
  return L.join('\n')
}

function wrap(text, width) {
  const words = String(text).split(/\s+/)
  const out = []
  let line = ''
  for (const w of words) {
    if (line && line.length + 1 + w.length > width) { out.push(line); line = w } else line = line ? line + ' ' + w : w
  }
  if (line) out.push(line)
  return out
}

// ---------------------------------------------------------------------------
// Selftest. A pattern that has only ever been seen matching is a pattern
// nobody has shown can refuse.
// ---------------------------------------------------------------------------

function runSelftest() {
  // Each case: source text, expected repo-slot count, expected near-miss count.
  const CASES = [
    // --- it must catch every form that exists in this directory today ------
    { name: 'the fifteen-instrument line', src: "const REPO = process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios'", slots: 1, near: 0 },
    { name: 'a lowercase binding (brief-gate.mjs:443)', src: "  const repo = process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios'", slots: 1, near: 0 },
    { name: 'inline in a template (proven.mjs:240)', src: "const raw = tryShell(`gh issue list --repo ${process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios'} --state all`)", slots: 1, near: 0 },
    { name: 'the other board (backlog.mjs:74)', src: "const REPO = process.env.QUEEN_REPO || 'gHashTag/t27'", slots: 1, near: 0 },
    { name: 'double quotes', src: 'const REPO = process.env.QUEEN_REPO || "gHashTag/t27"', slots: 1, near: 0 },
    { name: 'the ?? operator', src: "const REPO = process.env.TRIOS_CODE_REPO ?? 'gHashTag/BrowserOS'", slots: 1, near: 0 },
    { name: 'two slots on two lines (land.mjs:41-42)', src: "const REPO = process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios'\nconst CODE_REPO = process.env.TRIOS_CODE_REPO || 'gHashTag/BrowserOS'", slots: 2, near: 0 },
    { name: 'a hardcoded board with no env door', src: "const REPO = 'gHashTag/trios'", slots: 1, near: 0 },
    { name: 'camelCase declares REPO too', src: "const codeRepo = 'gHashTag/BrowserOS'", slots: 1, near: 0 },

    // --- and it must refuse the things that are not boards -----------------
    // THE THREE REAL FALSE POSITIVES. A branch name has a slash in it; rule 1
    // alone reports `feat/queen-supervisor` as a third board.
    { name: 'a branch name is not a board (land.mjs:43)', src: "const BASE = process.env.LAND_BASE || 'feat/queen-supervisor'", slots: 0, near: 1 },
    { name: 'a branch name is not a board (salvage.mjs:41)', src: "const BASE = process.env.TRIOS_BASE || 'feat/queen-supervisor'", slots: 0, near: 1 },
    { name: 'a branch name inline in a template (salvage.mjs:220)', src: "Read `git merge-base origin/${process.env.TRIOS_BASE || 'feat/queen-supervisor'} origin/x`", slots: 0, near: 1 },
    // REPORT contains REPO.
    { name: 'REPORT is not REPO (heal.mjs:286)', src: 'const REPORT_DEADLINE_MS = Number(process.env.HEAL_REPORT_DEADLINE_MS ?? 5 * 60 * 1000)', slots: 0, near: 0 },
    { name: 'a REPORT binding with a slug is still not a repo', src: "const REPORT = 'state/report.jsonl'", slots: 0, near: 0 },
    { name: 'an absolute path is not a slug', src: "const ROOT = process.env.TRIOS_ROOT || '/Users/playra/BrowserOS'", slots: 0, near: 0 },
    // Rejected by rule 1, so it is not even a near miss: a near miss is a
    // default that LOOKS like a repo and is not declared as one.
    { name: 'a two-slash path is not a slug', src: "const LOOP_DIR = process.env.LOOP_DIR || 'trios/.trinity/loop'", slots: 0, near: 0 },
    { name: 'a URL is not a slug', src: "const QUEEN = process.env.TRIOS_QUEEN_STATUS || 'https://host/queen/status'", slots: 0, near: 0 },
    // Prose. The defect class this repository has been bitten by six times.
    { name: 'a slug in a comment is prose', src: '// every loop instrument reads gHashTag/trios while the Queen works gHashTag/t27', slots: 0, near: 0 },
    { name: 'a slug in a sentence about REPO is still prose', src: '// REPO defaults to gHashTag/trios and nobody set TRIOS_ISSUE_REPO', slots: 0, near: 0 },
    { name: 'a computed REPO is not a literal (cycle.mjs:66)', src: "const REPO = path.resolve(DIR, '..', '..', '..')", slots: 0, near: 0 },
    { name: 'mismatched quotes are not a string', src: "const REPO = process.env.TRIOS_ISSUE_REPO || 'gHashTag/trios\"", slots: 0, near: 0 },

    // --- the documented blind spots, pinned so they stay documented --------
    { name: 'BLIND SPOT: a repo slot with no REPO in its name is missed', src: "const BOARD = process.env.QUEEN_BOARD || 'gHashTag/t27'", slots: 0, near: 1 },
    { name: 'BLIND SPOT: a host-qualified slug is missed', src: "const REPO = process.env.TRIOS_ISSUE_REPO || 'github.com/gHashTag/trios'", slots: 0, near: 0 },
  ]

  let failed = 0
  for (const c of CASES) {
    const r = scanSource(c.src, 'case')
    const okSlots = r.slots.length === c.slots
    const okNear = r.nearMisses.length === c.near
    if (okSlots && okNear) {
      process.stdout.write(`  ok    ${c.name}  (${r.slots.length} slot, ${r.nearMisses.length} near)\n`)
    } else {
      failed++
      process.stdout.write(
        `  FAIL  ${c.name}\n        expected ${c.slots} slot(s) / ${c.near} near miss(es), got ${r.slots.length} / ${r.nearMisses.length}\n` +
        '        the pattern has widened or narrowed; fix scanSource, not this case\n')
    }
  }
  // The role table is evidence too: it is what keeps a code repo from being
  // counted as a third board.
  const ROLE_CASES = [
    { v: 'TRIOS_ISSUE_REPO', want: 'issue' },
    { v: 'QUEEN_REPO', want: 'issue' },
    { v: 'TRIOS_CODE_REPO', want: 'code' },
    { v: 'SOMETHING_NEW_REPO', want: 'unknown' },
  ]
  for (const c of ROLE_CASES) {
    const got = roleOf({ kind: 'env-default', var: c.v })
    if (got === c.want) process.stdout.write(`  ok    role of ${c.v} is ${got}\n`)
    else { failed++; process.stdout.write(`  FAIL  role of ${c.v}: expected ${c.want}, got ${got}\n`) }
  }
  process.stdout.write(`\n${CASES.length + ROLE_CASES.length} case(s), ${failed} failed\n`)
  return failed === 0 ? 0 : 2
}

// ---------------------------------------------------------------------------

if (process.argv[1] && process.argv[1].endsWith('/board.mjs')) {
  if (SELFTEST) {
    process.exit(runSelftest())
  }

  const r = readBoards()
  if (r.error) {
    process.stderr.write('board: ' + r.error + '\n')
    process.exit(1)
  }
  const conditions = findConditions(r)
  const code = exitCodeFor(conditions)

  if (WANT_JSON) {
    process.stdout.write(JSON.stringify({
      at: r.at,
      dir: r.dir,
      staleThresholdHours: r.staleThresholdHours,
      filesScanned: r.filesScanned,
      readBoard: (r.boards.find((b) => b.role === 'issue') || {}).repo || null,
      issueBoards: r.boards.filter((b) => b.role === 'issue').map((b) => b.repo),
      boards: r.boards,
      slots: r.slots,
      nearMisses: r.nearMisses,
      overrides: r.overrides,
      conditions,
      exit: code,
    }, null, 2) + '\n')
  } else {
    process.stdout.write(render(r, conditions) + '\n')
  }
  process.exit(code)
}
