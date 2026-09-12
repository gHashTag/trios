#!/usr/bin/env node
// Is the cycle still installed correctly, and is its contract still true?
//
// Every claim this loop makes about itself has, at some point, been true when
// written and false when read. The dashboard advertised a cron job for six days
// after it stopped existing. `tri feed --act` ran on a timer for weeks while
// reaching an unrelated arm. A gate declared in the Makefile ran in no
// workflow. The pattern is always the same: a statement nobody re-checks.
//
// So this file re-checks, and it checks the things that are cheap to state and
// expensive to be wrong about:
//
//   1. The instruments exist and PARSE. `node --check` on each, because a file
//      that throws at import turns a scheduled cycle into a silent no-op.
//   2. Every `tri` arm the cycle added is REACHABLE. A shell `case` takes the
//      first match; this resolves each word against the real file the way bash
//      would, and fails if an earlier arm answers.
//   3. The cron contract holds IN CODE, not in a comment. `cycle.mjs` promises
//      it never mutates git and never writes the old loop's files. Both are
//      greppable, so both are checked.
//   4. Something actually schedules it, and says what.
//   5. The paths it writes are writable.
//
// Exit 0 when every check passes, 2 when any fails. Nothing here mutates
// anything - it is a doctor, not a surgeon.
//
// Usage: node cycle-doctor.mjs [--json]

import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { maskLiterals } from './mask.mjs'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const TRI = path.join(process.env.HOME || '', '.local', 'bin', 'tri')
const WANT_JSON = process.argv.slice(2).includes('--json')

// `dash-cc.mjs` joined on 2026-09-12, because `cycle.mjs` imports it and a parse
// error in it takes the cycle down.
//
// Being on THIS list only buys a parse check. The contract scan runs on
// `cycle.mjs` alone - I wrote a comment here claiming otherwise and then read
// the call site, which passes `cycleSrc` and nothing else. The delegation hole
// that exposed is closed separately, by `writesOldLoopFile` below.
const INSTRUMENTS = ['sense.mjs', 'anomaly.mjs', 'dash2.mjs', 'dash-cc.mjs', 'cycle.mjs', 'backlog.mjs', 'cycle-doctor.mjs', 'mask.mjs', 'driver.mjs']

// The words `tri` gained on 2026-09-12, each with the arm that must answer it.
const ARMS = {
  cycle: 'cycle',
  'cycle-repair': 'cycle-repair',
  'cycle-dash': 'cycle-dash',
  // Checked against every arm above before insertion: `cycle-html` collides
  // with nothing, and it is named for the artifact rather than the tool so it
  // stays true if the renderer is ever replaced.
  'cycle-html': 'cycle-html',
  'cycle-anom': 'cycle-anom|cycle-anomalies',
  'cycle-anomalies': 'cycle-anom|cycle-anomalies',
  'cycle-sense': 'cycle-sense',
  'cycle-log': 'cycle-log',
  'cycle-note': 'cycle-note',
  'cycle-doctor': 'cycle-doctor',
  backlog: 'backlog',
  // Added the same day, after `lesson`, `board`, `loop` and `why` were each
  // found to be taken already. A word that reaches somebody else's arm is not a
  // broken command - it is a working command that answers the wrong question,
  // which is how `tri feed --act` ran for its whole life against an arm nobody
  // meant. These three were checked against every arm before insertion; this
  // line is what keeps that true tomorrow.
  learn: 'learn',
  skills: 'skills',
  proof: 'proof',
  // `reach` was checked against every arm above before insertion, for the reason
  // the comment above gives. It is the only short English word in the loop's
  // vocabulary for "did this step get its turn", and nothing else claimed it.
  reach: 'reach',
}

// Files that belong to iterations 1-96. The cycle must not write them.
const NOT_OURS = ['state.json', 'DASHBOARD.txt', 'ledger.jsonl']

const SELFTEST = process.argv.slice(2).includes('--selftest')

const checks = []
const ok = (name, detail) => checks.push({ name, pass: true, detail })
const bad = (name, detail, fix) => checks.push({ name, pass: false, detail, fix })

function sh(cmd, timeout = 15000) {
  return execSync(cmd, { encoding: 'utf8', timeout, stdio: ['ignore', 'pipe', 'pipe'] }).trim()
}

// --- 1. the instruments exist and parse ------------------------------------
for (const f of INSTRUMENTS) {
  const p = path.join(DIR, f)
  if (!fs.existsSync(p)) {
    bad('instrument ' + f, 'missing', 'restore it from git: git checkout -- ' + path.relative(process.cwd(), p))
    continue
  }
  try {
    sh(`node --check ${JSON.stringify(p)}`)
    ok('instrument ' + f, 'exists and parses')
  } catch (e) {
    bad('instrument ' + f, 'does not parse: ' + String(e.stderr || e.message || e).split('\n')[0].slice(0, 140),
      'fix the syntax error; a scheduled cycle would fail silently')
  }
}

// --- 2. every tri arm resolves to the arm we wrote --------------------------
//
// Resolved the way bash resolves it: walk the case arms in file order and take
// the FIRST whose pattern list contains the word. Anything else is a guess.
function firstArmFor(word, triSource) {
  const lines = triSource.split('\n')
  for (const raw of lines) {
    const m = raw.match(/^\s{2}([\w|*?.\-\[\]]+)\)\s*(?:#.*)?$/)
    if (!m) continue
    const pattern = m[1]
    for (const alt of pattern.split('|')) {
      if (alt === word) return pattern
      // A literal `*` or a glob would swallow it before we ever match.
      if (alt.includes('*') || alt.includes('?')) {
        const re = new RegExp('^' + alt.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.') + '$')
        if (re.test(word)) return pattern
      }
    }
  }
  return null
}

if (!fs.existsSync(TRI)) {
  bad('tri', 'not found at ' + TRI, 'the cycle commands have nowhere to live')
} else {
  const triSource = fs.readFileSync(TRI, 'utf8')
  let shadowed = 0
  for (const [word, expected] of Object.entries(ARMS)) {
    const got = firstArmFor(word, triSource)
    if (got === expected) continue
    shadowed++
    bad('tri ' + word, got === null ? 'no arm answers it' : `answered by an earlier arm \`${got})\``,
      'move the arm above the one that shadows it, or rename. This is the defect that made `tri feed` unreachable for weeks.')
  }
  if (!shadowed) ok('tri arms', `${Object.keys(ARMS).length} words each reach their own arm`)
  try {
    sh(`bash -n ${JSON.stringify(TRI)}`)
    ok('tri syntax', 'parses')
  } catch (e) {
    bad('tri syntax', 'does not parse', 'restore from ' + TRI + '.bak-2026-09-12')
  }
}

// --- 3. the cron contract, checked in code ---------------------------------
//
// A contract stated only in a comment is a wish. These two are greppable.
const cycleSrc = fs.existsSync(path.join(DIR, 'cycle.mjs')) ? fs.readFileSync(path.join(DIR, 'cycle.mjs'), 'utf8') : ''

/**
 * The string literals in a source file, and nothing else.
 *
 * THE FIRST RUN OF THIS FILE FAILED ON ITS OWN READER. The check for `--force`
 * was run against the source with whole-line comments stripped, and `cycle.mjs`
 * contains the line
 *
 *     sh('git worktree remove ' + JSON.stringify(wt), 20000) // no --force, ever
 *
 * whose trailing comment survived the strip. The doctor reported a contract
 * violation that was a promise not to violate it. That is the same defect this
 * project has now found in a SQL gate, a claim guard and a boundary rule: an
 * instrument reading prose as code.
 *
 * Stripping trailing `//` is not the fix, because `https://` and any `//`
 * inside a string would go with it. The fix is to look only where a shell
 * command can actually be: inside a string literal. Comments are not string
 * literals, so the prose above is invisible to the checks below.
 *
 * WHAT THIS CANNOT SEE, stated rather than discovered later: a command
 * assembled from variables (`sh('git ' + verb)`) or read from a file. The
 * checks below are a tripwire on the obvious way to break the contract, not a
 * proof that it holds.
 */
// A SECOND false positive followed the first, from the opposite direction. The
// replacement read string literals with a regex - and this file's own prose is
// full of backticked terms like `git worktree add` and `state.json`, which a
// regex happily pairs into template literals. Prose read as code again, one
// layer down.
//
// Regexes cannot do this, because deciding whether a backtick opens a template
// or sits inside a comment requires knowing what came before it. So: a scanner
// that tracks state. Thirty lines, and correct for the thing being asked - it
// knows a `//` inside a string is not a comment, and a backtick inside a
// comment is not a template.
function stringLiterals(src) {
  const out = []
  let i = 0
  const n = src.length
  while (i < n) {
    const c = src[i]
    if (c === '/' && src[i + 1] === '/') {
      while (i < n && src[i] !== '\n') i++
    } else if (c === '/' && src[i + 1] === '*') {
      i += 2
      while (i < n && !(src[i] === '*' && src[i + 1] === '/')) i++
      i += 2
    } else if (c === "'" || c === '"' || c === '`') {
      const quote = c
      const start = ++i
      while (i < n) {
        if (src[i] === '\\') { i += 2; continue }
        if (src[i] === quote) break
        if (quote !== '`' && src[i] === '\n') break // an unterminated quote is not a literal
        i++
      }
      out.push(src.slice(start, i))
      i++
    } else {
      i++
    }
  }
  return out
}

// `maskLiterals` used to live here, as a second copy of the walk in
// `stringLiterals` above, with a note saying that a third caller should make
// the two one. `selftest.mjs` became that third caller within the hour, so the
// walk now lives in `mask.mjs` and both callers import it. `stringLiterals`
// above still carries its own copy of the parsing rules and is still a thing to
// watch; it yields the spans, this yields the holes, and one generator could
// serve both.

const GIT_MUTATORS = /\bgit\s+(?:-C\s+\S+\s+)?(commit|push|checkout|reset|stash|merge|rebase|add|worktree\s+add|branch\s+-[dD])/

// A THIRD false positive, and the same defect a third time: a token read
// without the command it belongs to.
//
// `FORCE` used to be /--force|(?:^|\s)-f(?:\s|$)/ over ALL string literals
// joined together. On 2026-09-12 the cycle gained a peer check -
//
//     sh('pgrep -f ' + JSON.stringify('trinity/loop/' + p) + ' || true')
//
// - and the doctor reported "cycle.mjs builds a command containing -f". It does.
// It is pgrep's -f, which means "match against the full command line" and
// destroys nothing. The clause being checked is about ONE thing: git being told
// to discard work it would otherwise refuse to discard.
//
// So the flag is now only a violation in a literal that ALSO names a destructive
// git verb. That is deliberately narrow, and it is narrow in the safe direction:
// the thing the clause exists to forbid - `git worktree remove --force` - names
// both in one literal, because the flag has to sit next to the verb that reads
// it. A command split across two literals evades this, exactly as the header
// above already says an assembled command does.
const DESTRUCTIVE_GIT = /\bgit\b[^\n]*\b(worktree\s+remove|clean|rm|checkout|reset|branch\s+-[dD]|push)\b/
const FORCE = /--force|(?:^|\s)-f(?:\s|$)/

/**
 * The three contract clauses, as predicates over a source file.
 *
 * Factored out so `--selftest` can prove they still catch a violation. A gate
 * that has only ever been seen passing is a gate nobody has shown can fail -
 * this repository has already shipped one of those, an audit that reported
 * success because it never found its compiler.
 *
 * @returns {Array<{clause: string, hit: string}>} empty when the contract holds
 */
function contractViolations(src) {
  const lits = stringLiterals(src)
  const s = lits.join('\n')
  const found = []
  const g = s.match(GIT_MUTATORS)
  if (g) found.push({ clause: 'no git mutation', hit: g[0] })
  // Per literal, not over the join: a `-f` in one string and a `git clean` in
  // an unrelated one are not a forced git command, and reading them as one was
  // the bug.
  for (const lit of lits) {
    if (!DESTRUCTIVE_GIT.test(lit)) continue
    const f = lit.match(FORCE)
    if (f) { found.push({ clause: 'no --force', hit: f[0].trim() }); break }
  }
  for (const name of NOT_OURS) {
    // Anchored on the left, because `cycle-ledger.jsonl` contains
    // `ledger.jsonl` and the cycle's OWN ledger is not the old loop's. The
    // selftest caught this; a bare substring match had the doctor reporting
    // that the cycle writes a file it was written specifically not to write.
    const re = new RegExp('(?:^|[^\\w.-])' + name.replace('.', '\\.'), 'm')
    if (re.test(s)) found.push({ clause: 'old loop untouched', hit: name })
  }
  return found
}

/**
 * Does this source pass an old-loop filename to a WRITE call?
 *
 * WHY THIS EXISTS SEPARATELY FROM `contractViolations`. That one scans
 * `cycle.mjs` and asks whether an old-loop filename appears anywhere in a string
 * literal. That is the right question for one file and the wrong question for
 * seven: `anomaly.mjs` READS `DASHBOARD.txt` on purpose - it is how the detector
 * catches the dashboard advertising a driver that does not exist - and a
 * presence check would call that a violation. Mentioning a file and writing it
 * are different acts, and a guard that cannot tell them apart can only be
 * applied where nobody needs to mention.
 *
 * So this one reads the FIRST ARGUMENT of each write call and nothing else. It
 * balances parentheses rather than splitting on the first comma, because every
 * real call site here is `writeFileSync(path.join(DIR, 'x'), body)` and the
 * first comma is inside `path.join`.
 *
 * It exists because `cycle.mjs` now writes through an imported renderer. The
 * contract says the cycle must not write those files; before this, the cycle
 * could have imported anything and written them from inside it, and the doctor
 * would have gone on reporting that the contract held.
 */
function writesOldLoopFile(src) {
  const found = []
  // SCAN THE MASK, READ THE ORIGINAL.
  //
  // The first version scanned the raw source and immediately flagged this very
  // file, on the line `{ name: 'the old ledger is caught', src:
  // "fs.appendFileSync('ledger.jsonl', x)" }` - a selftest FIXTURE, quoted,
  // whose entire purpose is to be a violation that never runs. A checker that
  // reads a test fixture as evidence is the same defect as one that reads prose
  // as evidence, and this loop has now found it in three costumes.
  //
  // `maskLiterals` blanks the INSIDE of every comment and string while keeping
  // the file's length, so a call written inside a quote is invisible to the
  // regex and to the paren balancer, and the indices still address the original.
  // The filename we are hunting for lives inside a literal, so the argument text
  // itself is sliced from `src`, not from the mask.
  const mask = maskLiterals(src)
  const CALL = /\b(?:fs\.)?(writeFileSync|appendFileSync|writeFile|appendFile|createWriteStream)\s*\(/g
  let m
  while ((m = CALL.exec(mask))) {
    let depth = 1
    let i = m.index + m[0].length
    const start = i
    for (; i < mask.length && depth > 0; i++) {
      const c = mask[i]
      if (c === '(') depth++
      else if (c === ')') depth--
      else if (c === ',' && depth === 1) break
    }
    const firstArg = src.slice(start, i)
    for (const name of NOT_OURS) {
      const re = new RegExp('(?:^|[^\\w.-])' + name.replace(/\./g, '\\.'))
      if (re.test(firstArg)) found.push({ call: m[1], hit: name, arg: firstArg.trim().slice(0, 60) })
    }
  }
  return found
}

const CLAUSE_FIX = {
  'no git mutation': 'the cycle runs unattended on a timer; a git-mutating cycle will one day commit a half-typed edit. Remove it or move it behind an explicit operator command.',
  'no --force': 'git refusing to remove a dirty worktree is what protects unlanded bee work. Drop the flag.',
  'old loop untouched': 'iterations 1-96 own those files. The cycle writes DASHBOARD2.*, cycle-ledger.jsonl and state/cycle-readings.jsonl only.',
}

if (!cycleSrc) {
  bad('contract', 'cycle.mjs is missing', 'see above')
} else {
  const violations = contractViolations(cycleSrc)
  const byClause = {}
  for (const v of violations) (byClause[v.clause] = byClause[v.clause] || []).push(v.hit)
  for (const clause of Object.keys(CLAUSE_FIX)) {
    if (byClause[clause]) {
      bad('contract: ' + clause, 'cycle.mjs builds a command containing ' + byClause[clause].join(', '), CLAUSE_FIX[clause])
    } else {
      ok('contract: ' + clause, 'holds')
    }
  }
}

// --- 3a. and nothing the cycle writes THROUGH writes them either ------------
//
// Every instrument, not just `cycle.mjs`, because the cycle imports renderers
// and the contract is about what ends up on disk, not about which file contains
// the call. Read-only mentions are allowed here and are not a loophole: the
// argument scan sees `writeFileSync`'s first argument and nothing else.
{
  const offenders = []
  for (const f of INSTRUMENTS) {
    let src
    try { src = fs.readFileSync(path.join(DIR, f), 'utf8') } catch { continue }
    for (const w of writesOldLoopFile(src)) offenders.push(`${f}: ${w.call}(${w.arg})`)
  }
  if (offenders.length) {
    bad('contract: no instrument writes the old loop', offenders.join('; '),
      'iterations 1-96 own state.json, DASHBOARD.txt and ledger.jsonl. Write DASHBOARD2.*, DASHBOARD.html, cycle-ledger.jsonl or state/cycle-readings.jsonl instead.')
  } else {
    ok('contract: no instrument writes the old loop', `${INSTRUMENTS.length} instruments scanned at their write calls`)
  }
}

// --- 3b. can the contract check still fail? --------------------------------
//
// Four cases, and the last two are the ones that have actually gone wrong here.
// A gate is only evidence if it is sensitive to the thing it claims to detect
// AND blind to prose that merely mentions it.
if (SELFTEST) {
  const CASES = [
    { name: 'clean source passes', src: "sh('git worktree remove ' + wt)\nsh('git status --porcelain')", expect: 0 },
    { name: 'a git commit is caught', src: "sh('git commit -m auto')", expect: 1 },
    // One, not two: `worktree remove` is not in GIT_MUTATORS - removing a
    // worktree is what the repair legitimately does, and the flag is the part
    // that is forbidden. The first run of this case expected 2 and was wrong
    // about the gate it was testing.
    { name: 'a --force is caught', src: "sh('git worktree remove --force ' + wt)", expect: 1 },
    { name: 'the old ledger is caught', src: "fs.appendFileSync('ledger.jsonl', x)", expect: 1 },
    { name: 'our own ledger is not', src: "fs.appendFileSync('cycle-ledger.jsonl', x)", expect: 0 },
    { name: 'a trailing comment is not code', src: "sh('git worktree remove ' + wt) // no --force, ever", expect: 0 },
    { name: 'backticked prose is not a template', src: '// it must never run `git commit` or touch `state.json`\nsh("git status")', expect: 0 },
    { name: 'a URL is not a comment', src: 'const u = "https://api.github.com/x"\nsh("git status")', expect: 0 },
    // The three cases that narrowed FORCE. The first is the real line from
    // cycle.mjs that produced the false positive.
    { name: "pgrep's -f is not git's", src: 'sh("pgrep -f " + JSON.stringify(p))', expect: 0 },
    { name: 'a -f in one literal and a git verb in another do not combine', src: 'sh("pgrep -f " + p)\nsh("git worktree remove " + wt)', expect: 0 },
    { name: 'a -f next to a git verb is still caught', src: 'sh("git clean -f " + wt)', expect: 1 },
  ]
  for (const c of CASES) {
    const n = contractViolations(c.src).length
    if (n === c.expect) ok('selftest: ' + c.name, `${n} violation(s), as expected`)
    else bad('selftest: ' + c.name, `expected ${c.expect} violation(s), got ${n}`, 'the contract check is no longer evidence; fix stringLiterals or the patterns')
  }

  // The write scan, which has a different failure mode from the one above: it
  // must distinguish WRITING an old-loop file from MENTIONING one, and must not
  // read a quoted example as a call. Its first live run failed the last case
  // here, against this file, which is why the case is written down.
  const WRITE_CASES = [
    { name: 'writing the old state is caught', src: "fs.writeFileSync(path.join(DIR, 'state.json'), x)", expect: 1 },
    { name: 'writing the old dashboard is caught', src: "fs.writeFileSync(P('DASHBOARD.txt'), x)", expect: 1 },
    { name: 'writing the old ledger is caught', src: "fs.appendFileSync(LEDGER + 'ledger.jsonl', x)", expect: 1 },
    { name: 'writing our own dashboard is not', src: "fs.writeFileSync(path.join(DIR, 'DASHBOARD2.txt'), x)", expect: 0 },
    { name: 'writing the html dashboard is not', src: "fs.writeFileSync(path.join(DIR, 'DASHBOARD.html'), x)", expect: 0 },
    { name: 'writing our own ledger is not', src: "fs.appendFileSync(path.join(DIR, 'cycle-ledger.jsonl'), x)", expect: 0 },
    // The distinction the whole function exists for. `anomaly.mjs` does exactly
    // this, on purpose, and a presence check would have made it a violation.
    { name: 'READING the old dashboard is not writing it', src: "const d = fs.readFileSync(path.join(DIR, 'DASHBOARD.txt'), 'utf8')", expect: 0 },
    { name: 'a filename in a comment is not a write', src: "// never write state.json here\nfs.writeFileSync(OUT, x)", expect: 0 },
    // The case this file failed. A fixture is not a call.
    { name: 'a quoted example is not a call', src: `const CASES = [{ src: "fs.appendFileSync('ledger.jsonl', x)" }]`, expect: 0 },
    // The body is not the path: a body that happens to contain the name is not
    // a write TO it. This is the second-argument half of the same distinction.
    { name: 'the name in the body is not the path', src: "fs.writeFileSync(OUT, 'see state.json for the old one')", expect: 0 },
  ]
  for (const c of WRITE_CASES) {
    const n = writesOldLoopFile(c.src).length
    if (n === c.expect) ok('selftest: ' + c.name, `${n} finding(s), as expected`)
    else bad('selftest: ' + c.name, `expected ${c.expect} finding(s), got ${n}`, 'the write scan is no longer evidence; fix maskLiterals or the argument walk')
  }

  // --- the volume anomaly says whose disk it is ----------------------------
  //
  // `findAnomalies` had no behavioural test of any kind until this block. The
  // sentence it used to emit put a worktree count next to "95% used", which is
  // two true facts arranged to read as a cause: the repair a reader takes from
  // it is to delete the worktrees, and on 2026-09-13 that would have freed 2.5G
  // out of 384G and could have destroyed a bee's uncommitted work.
  //
  // Importing anomaly.mjs here is only safe because it grew a main guard the
  // same night. Before that this import would have run the detector.
  const { findAnomalies } = await import('./anomaly.mjs')
  // A READING WITH EVERY LEAF UNMEASURED, TAKEN FROM THE REAL SHAPE.
  //
  // The first two attempts typed the shape from memory. The first named six of
  // the eleven top-level facts and the detector threw on the seventh; the second
  // named all eleven and still threw, because `r.loop` is a GROUP of measures
  // (`r.loop.staleHours.v`) and not a measure. Hand-copying a structure is the
  // defect this repository is named for. So the shape comes from `sense.mjs`
  // itself - one fast reading, about a second - and every leaf is then nulled.
  // A field added to a reading tomorrow is in this fixture tomorrow.
  const { takeReading } = await import('./sense.mjs')
  const shape = takeReading({ fast: true })
  const nullLeaves = (o) => {
    if (!o || typeof o !== 'object') return o
    if ('v' in o) { o.v = null; return o }
    for (const k of Object.keys(o)) nullLeaves(o[k])
    return o
  }
  const blank = () => nullLeaves(JSON.parse(JSON.stringify(shape)))
  const volume = (out) => out.find((a) => a.id === 'volume-near-full')

  const DISK_CASES = [
    {
      name: 'a measured small share says the space is not ours',
      disk: { percentUsed: 95, raw: 'disk3s1 460Gi 384Gi 22Gi 95%', ownGB: 2.5, ownPercentOfUsed: 0.6 },
      want: (a) => a && /2\.5G/.test(a.truth) && /would not move this number/.test(a.repair),
      why: 'the share is measured and small, so the repair must say deleting worktrees is not it',
    },
    {
      name: 'an unmeasured share is declared unmeasured, not implied',
      disk: { percentUsed: 95, raw: 'x', ownGB: null, ownPercentOfUsed: null },
      want: (a) => a && /not measured this run/.test(a.truth) && !/worktrees are/.test(a.truth),
      why: 'a null share must read as "not measured", never as a number and never as nothing',
    },
    {
      name: 'a large share keeps the reaping repair',
      disk: { percentUsed: 95, raw: 'x', ownGB: 200, ownPercentOfUsed: 52.1 },
      want: (a) => a && /reap-local/.test(a.repair) && /never --force/.test(a.repair),
      why: 'when the loop really does own the disk, reaping IS the repair - and still never --force',
    },
    {
      name: 'a volume below the threshold raises nothing',
      disk: { percentUsed: 40, raw: 'x', ownGB: 2.5, ownPercentOfUsed: 0.6 },
      want: (a) => !a,
      why: 'the detector must not raise a finding about a volume with room on it',
    },
  ]
  for (const c of DISK_CASES) {
    const r = blank()
    r.disk.v = c.disk
    let a = null
    let threw = null
    try { a = volume(findAnomalies(r)) } catch (e) { threw = e.message }
    if (threw) bad('selftest: ' + c.name, 'findAnomalies threw: ' + threw, 'the detector must survive a reading whose other facts are null')
    else if (c.want(a)) ok('selftest: ' + c.name, c.why)
    else bad('selftest: ' + c.name, `got: ${a ? a.truth + ' || ' + a.repair : '(no finding)'}`, c.why)
  }
}

// --- 4. something schedules it ---------------------------------------------
//
// THE FIRST VERSION OF THIS CHECK WAS THE BUG IT WAS WRITTEN TO CATCH.
//
// It ran `launchctl list | grep -i trios`, found four jobs, and reported
// "scheduled by 4 launchd job(s) named trios" - a pass. Those four were
// trios-feed, trios-heal, com.trios.backup and com.trios.health. Not one of
// them runs the cycle. They were all loaded throughout the 147.9-hour outage
// that this check's own failure message cites, so the check would have been
// green for every hour of the thing it exists to detect.
//
// A name is not a behaviour. The question is not "is there a job whose label
// resembles this project" but "is there a live job whose COMMAND runs the
// cycle". So each candidate is resolved to the command it would execute:
// launchd jobs through their plist's ProgramArguments, Claude tasks through
// their prompt, crontab through the line itself. Nothing counts unless the
// text that would actually run mentions `tri cycle` or `cycle.mjs`.
//
// For launchd there is a second half: a plist ON DISK is not a job. It has to
// be loaded, which is what `launchctl list` answers, so both are required and
// a plist that is present but unloaded is reported as the distinct failure it
// is - that one is a single `launchctl load` away, and saying so beats saying
// "nothing schedules the cycle".

/** Does this command line actually run the cycle? */
const RUNS_CYCLE = (s) => /\btri\s+cycle\b/.test(s || '') || (s || '').includes('cycle.mjs')

const scheduled = []
const nearly = []

try {
  const tasks = JSON.parse(fs.readFileSync(path.join(DIR, '..', '..', '.claude', 'scheduled_tasks.json'), 'utf8'))
  const list = Array.isArray(tasks.tasks) ? tasks.tasks : []
  const hits = list.filter((t) => RUNS_CYCLE(JSON.stringify(t)))
  if (hits.length) scheduled.push(`${hits.length} Claude scheduled task(s)`)
} catch { /* absent is a fine answer */ }

try {
  const cron = sh('crontab -l 2>/dev/null || true', 10000)
  const hits = cron.split('\n').filter((l) => !l.trim().startsWith('#') && RUNS_CYCLE(l))
  if (hits.length) scheduled.push(`${hits.length} crontab line(s)`)
} catch { /* no crontab */ }

try {
  const agents = path.join(process.env.HOME || '', 'Library', 'LaunchAgents')
  const loaded = new Set(
    sh('launchctl list 2>/dev/null || true', 10000)
      .split('\n')
      .map((l) => l.trim().split(/\s+/).pop())
      .filter(Boolean),
  )
  for (const f of fs.readdirSync(agents)) {
    if (!f.endsWith('.plist')) continue
    let body
    // A .bak sitting beside a plist is a real thing in this directory, and
    // reading one must not take the check down.
    try { body = fs.readFileSync(path.join(agents, f), 'utf8') } catch { continue }
    if (!RUNS_CYCLE(body)) continue
    const label = f.replace(/\.plist$/, '')
    if (loaded.has(label)) scheduled.push(`launchd ${label}`)
    else nearly.push(label)
  }
} catch { /* no LaunchAgents directory */ }

if (scheduled.length) {
  ok('scheduled by', scheduled.join(', '))
} else if (nearly.length) {
  bad('scheduled by', `plist(s) present but not loaded: ${nearly.join(', ')}`,
    `launchctl load -w ~/Library/LaunchAgents/${nearly[0]}.plist`)
} else {
  bad('scheduled by', 'nothing runs `tri cycle` on a timer',
    'install one: `tri cycle` from launchd or crontab. This is exactly the state the loop was in for 147.9 hours.')
}

// --- 4b. can the schedule check still fail? --------------------------------
//
// The second case is the whole reason this block exists: `tri feed --act` on a
// timer is what the old check counted as "the cycle is scheduled". It must
// now read as what it is - a different job that happens to live in the same
// project.
if (SELFTEST) {
  const CASES = [
    { name: 'a plist that runs the cycle counts', s: '<string>/Users/x/.local/bin/tri cycle</string>', want: true },
    { name: 'a plist that runs tri feed does not', s: '<string>/Users/x/.local/bin/tri feed --act</string>', want: false },
    { name: 'an unrelated trios job does not', s: '<string>/Users/x/trios/scripts/backup.sh</string>', want: false },
    { name: 'cycle-repair counts', s: 'tri cycle-repair', want: true },
    { name: 'the mjs path counts', s: 'node /x/.trinity/loop/cycle.mjs --deep', want: true },
    { name: 'recycle is not cycle', s: 'tri recycle-everything', want: false },
  ]
  for (const c of CASES) {
    if (RUNS_CYCLE(c.s) === c.want) ok('selftest: ' + c.name, c.want ? 'recognised' : 'ignored, as it should be')
    else bad('selftest: ' + c.name, `RUNS_CYCLE returned ${!c.want}`, 'the schedule check is no longer evidence; fix RUNS_CYCLE')
  }
}

// --- 5. writable --------------------------------------------------------
for (const p of [DIR, path.join(DIR, 'state')]) {
  try {
    fs.mkdirSync(p, { recursive: true })
    fs.accessSync(p, fs.constants.W_OK)
    ok('writable ' + path.basename(p), p)
  } catch {
    bad('writable ' + path.basename(p), 'not writable: ' + p, 'the cycle cannot record anything')
  }
}

// ---------------------------------------------------------------------------

const failed = checks.filter((c) => !c.pass)
if (WANT_JSON) {
  process.stdout.write(JSON.stringify({ at: new Date().toISOString(), checks, failed: failed.length }, null, 2) + '\n')
} else {
  for (const c of checks) {
    process.stdout.write(`${c.pass ? 'ok  ' : 'FAIL'}  ${c.name.padEnd(30)} ${c.detail}\n`)
    if (!c.pass && c.fix) process.stdout.write(`      -> ${c.fix}\n`)
  }
  process.stdout.write(`\n${checks.length - failed.length}/${checks.length} checks pass\n`)
}
process.exit(failed.length ? 2 : 0)
