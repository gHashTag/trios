#!/usr/bin/env node
// The three steps that decide whether a worker has anything to start.
//
// WHY THIS IS SEPARATE FROM THE CHAIN. `heal.mjs` runs eleven steps and takes
// about 480 seconds of its 600-second cycle. Four workers drain the queue in
// roughly 13 minutes. So the refill arrives once per cycle at best, and the
// swarm spends the gap idle - measured repeatedly on 2026-09-04: queue 0,
// running 0, with the chain mid-way through auditing.
//
// The audits are worth running. They are not worth making the workers wait
// for. So the three steps that FEED the swarm are also runnable alone, on a
// cadence of their own:
//
//   push-work   a branch nobody can see cannot be closed against
//   close-done  an accepted issue left open holds its boundary, so the
//               disjoint selector finds no free path and the author files
//               nothing - the failure looks exactly like an empty backlog
//   author      refill to the queue depth
//
// The order is not arbitrary and is the same one heal.mjs uses: skipping the
// first step disables the last.
//
// IT TAKES THE SAME LOCK. Two writers pushing and closing at once is the thing
// the lock exists to prevent, and a feed running beside a chain would be
// exactly that. A feed that finds the lock held stands down and says so - the
// chain it stood down for does the same work a minute later.
//
// Usage:
//   node feed.mjs           # report what it would do
//   node feed.mjs --act     # push, close, file

import path from 'node:path'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const isMain = process.argv[1] && process.argv[1].endsWith('/feed.mjs')
const L = await import(path.join(DIR, 'loop.mjs'))

export const STEPS = [
  { name: 'push-work', file: 'push-work.mjs', act: '--push', why: 'a branch nobody can see cannot be closed against' },
  // Between the push and the close: close-done now demands LANDED rather than
  // pushed, so without this nothing would ever close again.
  { name: 'land', file: 'land.mjs', act: '--land', why: 'put accepted work into the branch' },
  { name: 'close-done', file: 'close-done.mjs', act: '--close', why: 'an open accepted issue holds its boundary' },
  // SHARE-MODULES WAS HERE AND IS NOT ANY MORE.
  //
  // It was put on this 300-second timer when the channel refused three times in
  // five and the step therefore did nothing quickly. With the client named, the
  // channel works - and the step now does REAL work: it walks every container
  // worktree with `du`, promotes a store and rebuilds farms. Its first two
  // working runs both recorded `share-modules: timed out`, because a per-step
  // cap of 300 seconds inside a 300-second cycle cannot hold it.
  //
  // The previous round predicted exactly this: "expect the first working runs to
  // hit a deadline for an entirely new reason". It lives in `heal` now, whose
  // swarm-freeing phase has eight minutes.
  //
  // THE ONLY THING LEFT STANDING AFTER THREE REFUTATIONS.
  //
  // Every dispatch runs `bun install` and writes ~2.5 GB of node_modules into
  // its worktree, and three attempts to stop that at the source were each
  // measured and each failed: moving the package cache onto the volume (bun
  // copies whatever device it is on), pre-building a link farm before the
  // install (bun wipes it - 159M becomes 2562M), and `--backend=symlink`, which
  // gives 41M on a single-package project and 2561M on this workspace.
  //
  // What works is sharing AFTERWARDS, and the only variable left is how long the
  // duplicates sit there. On the chain alone that is 10 to 25 minutes and the
  // volume climbed at 133 points per hour; on this timer it is five, which
  // bounds the accumulation to roughly what two bees produce.
  //
  // It is a mop, not a tap, and it is named as one.
  { name: 'author', file: 'author.mjs', act: '--file', why: 'refill to the queue depth' },
]

/** One line per step, read from the step's own output rather than invented. */
export const SUMMARY = [
  [/(\d+) tree\(s\) rebuilt against one store, about (\d+) MB returned/, (m) => `${m[1]} tree(s) share one store, ${m[2]} MB returned`],
  [/could not read which bees are running/, () => 'stood down: the board could not be read, and rebuilding under a live install kills a dispatch'],
  [/pushed (\d+)/, (m) => `pushed ${m[1]}`],
  [/not pushed: 0/, () => 'every branch with work is already on the remote'],
  [/landed (\d+) of (\d+) clean/, (m) => `${m[1]} accepted branch(es) landed`],
  [/(\d+) landable, showing/, (m) => `${m[1]} accepted branches still outside the base`],
  [/closed (\d+)\s+failed (\d+)/, (m) => `closed ${m[1]}, failed ${m[2]}`],
  [/closable 0/, () => 'nothing closable'],
  [/filed (\d+)/, (m) => `filed ${m[1]}`],
  [/STALLED: /, () => 'REFUSED to file - nobody is draining the backlog'],
  [/at the WIP limit|already has an issue|would file 0/, () => 'queue already at depth, nothing filed'],
]

export function summarise(out) {
  for (const [re, f] of SUMMARY) {
    const m = out.match(re)
    if (m) return f(m)
  }
  const line = out.split('\n').filter((l) => l.trim()).pop()
  return (line || '').slice(0, 90)
}

// ---------------------------------------------------------------------------
// THE LINES THAT MUST SURVIVE CONDENSATION.
//
// `summarise` maps a step's whole stdout to ONE line, so anything the step said
// that no pattern matches is gone. Measured 2026-09-12: `grep -c "REFUSED by
// the gate"` over feed.timer.log and heal.timer.log returned 0 and 0, while the
// gate was refusing five briefs on every acting run - the five drafts are still
// in /tmp and brief-gate still refuses them. author.mjs printed the refusal and
// the reason; both chains threw them away, matched `filed 0`, and reported
// `author=ok`.
//
// NARROW ON PURPOSE, because the recurring defect in this directory is a regex
// that matches a word which is not the thing. Only three openings pass:
//
//   `REFUSED `        author.mjs:802, at column 0
//   `FAILED to file ` author.mjs:807, at column 0
//   `!!`              brief-gate's problem lines, which author re-prints
//                     indented inside the refusal block - hence the leading
//                     \s*, without which the REASON for a refusal is still lost
//
// What deliberately does NOT match: `REFUSING to continue` from land.mjs (it
// has a SUMMARY line of its own), and two-views' `HTTP ok, ssh REFUSED`, which
// is mid-line. Both are pinned in the test below.
export const PASS_THROUGH = /^\s*(REFUSED |FAILED to file |!!)/

// A BOUND, because an unbounded pass-through is how a log becomes unreadable.
// brief-gate --open prints one `!!` line per failing open brief and there were
// 19 of them on the day this was written; author's five refusals cost about ten
// lines. Twenty covers both, and the overflow is counted rather than hidden.
export const PASS_THROUGH_MAX = 20

export function passThrough(out, max = PASS_THROUGH_MAX) {
  const hits = String(out).split('\n').filter((l) => PASS_THROUGH.test(l))
  if (hits.length <= max) return hits
  return [...hits.slice(0, max), `... and ${hits.length - max} more line(s) like these`]
}

// ---------------------------------------------------------------------------
// A THIRD STATUS: the step ran, and produced none of what it set out to produce.
//
// `ok` and `FAILED` were the only two words these chains had, so a step that
// did everything except the one thing it exists for got the first of them.
// author lined up five candidates, the gate refused all five, and the chain
// printed `filed 0` and then `author=ok`.
//
// Each rule reads the step's OWN two numbers - what it lined up, and what it
// delivered - and fires only when the first is above zero and the second is
// exactly zero. A step that lined nothing up is not empty-handed; it is idle,
// and idle is already `ok`. Both numbers must be present: a step that never
// printed its second number was cut off, which is `timed out`, not this.
const pair = (out, wantRe, gotRe, say) => {
  const w = out.match(wantRe)
  const g = out.match(gotRe)
  if (!w || !g) return null
  const want = Number(w[1])
  const got = Number(g[1])
  return want > 0 && got === 0 ? say(want) : null
}

export const EMPTY_HANDED = [
  // author.mjs: `not yet filed: N   would file M` ... `filed K`
  (out) => pair(out, /would file (\d+)/, /^filed (\d+)$/m,
    (want) => `lined up ${want} candidate(s) and filed none of them`),
  // push-work.mjs: `... not pushed: N` ... `pushed M`
  (out) => pair(out, /not pushed: (\d+)/, /^pushed (\d+)$/m,
    (want) => `${want} branch(es) hold work that is still not on the remote`),
  // close-done.mjs: `closable N   skipped X` ... `closed M   failed F`
  (out) => pair(out, /^closable (\d+)/m, /^closed (\d+)\s/m,
    (want) => `${want} issue(s) were closable and none were closed`),
  // land.mjs: `landed M of C clean in this batch`
  (out) => {
    const m = out.match(/landed (\d+) of (\d+) clean/)
    if (!m) return null
    return Number(m[2]) > 0 && Number(m[1]) === 0
      ? `${m[2]} clean branch(es) in the batch and none landed` : null
  },
]

/** The reason this step came back empty-handed, or null if it did not. */
export function emptyHanded(out) {
  for (const rule of EMPTY_HANDED) {
    const why = rule(String(out))
    if (why) return why
  }
  return null
}

if (isMain) {
  const ACT = process.argv.includes('--act')
    // EIGHT MINUTES, not four, and the per-step cap is 300 s rather than 180.
  //
  // The first version capped a step at 180 s. `close-done` and `author` both
  // take longer than that when there is real work - so they were KILLED
  // mid-way, after closing some issues and filing one, and reported FAILED.
  // That breaks the rule the chain already carries: a half-run step is worse
  // than an unrun one. Being slower is not the failure; being cut off is.
  const DEADLINE_MS = Number(process.env.FEED_DEADLINE_MS ?? 8 * 60 * 1000)

  // EVERY RUN IS DATED, AT ITS FIRST LINE.
  //
  // Measured 2026-09-12: heal.timer.log held 24794 lines across 305 runs and
  // not one clock; feed.timer.log the same. A truncation found at log line 5710
  // could therefore be placed only as "somewhere in the earlier 23% of the
  // file" - not before or after any dated change to the thing that wrote it.
  //
  // Printed BEFORE the lock is asked for, so a run that stands down is dated
  // too: those are the runs whose absence needs explaining. It is the opening
  // bracket of the `feed complete:` line that already closes every run.
  console.log(`${new Date().toISOString()} feed start${ACT ? '' : ' (report only)'}`)

  if (ACT) {
    const state = L.lockHolder()
    const mine = process.env.LOOP_HOLDER && state && state.holder === process.env.LOOP_HOLDER
    if (!mine) {
      const got = L.acquire('feed', { singleProcess: true })
      if (!got.ok) {
        // Not an error. The chain holds it and does this same work.
        console.log(`the loop lock is held by ${got.held.holder} (${Math.round(got.ageMs / 60000)} min) - standing down; it feeds too`)
        L.append({ kind: 'feed-skipped', note: `lock held by ${got.held.holder}` })
        process.exit(0)
      }
      const release = () => { try { L.release() } catch { /* already gone */ } }
      process.on('exit', release)
      process.on('SIGINT', () => { release(); process.exit(130) })
      process.on('SIGTERM', () => { release(); process.exit(143) })
    }
  }

  const startedAt = Date.now()
  const results = []
  for (const s of STEPS) {
    const left = DEADLINE_MS - (Date.now() - startedAt)
    if (left <= 0) {
      console.log(`\n--- ${s.name}  (${s.why})\n    SKIPPED - past the ${Math.round(DEADLINE_MS / 60000)} minute deadline`)
      results.push({ step: s.name, status: 'skipped' })
      continue
    }
    process.stdout.write(`\n--- ${s.name}  (${s.why})\n`)
    let out = ''
    let status = 'ok'
    try {
      // THE STEP'S BUDGET IS TOLD TO THE CHANNEL, not just enforced on it.
      //
      // A step killed at its timeout reports "timed out part-way" and loses
      // whatever it was about to say. The channel's app-down backoff sleeps 180
      // seconds across three attempts; on a 300-second cycle that is most of the
      // budget spent waiting. Given the number, the channel stops before the
      // wait that would overrun and the step finishes with an answer.
      const budget = Math.max(30000, Math.min(300000, left))
      out = execSync(`node ${path.join(DIR, s.file)} ${ACT ? s.act : ''}`, {
        encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
        timeout: budget,
        env: { ...process.env, CHANNEL_DEADLINE_MS: String(Math.round(budget * 0.8)) },
      })
    } catch (e) {
      out = String(e.stdout || '') + String(e.stderr || '')
      // A step KILLED by the timeout is not a step that failed, and calling it
      // one sent me hunting a bug in close-done that did not exist. Say what
      // actually happened: it ran out of time, part-way, and what it did before
      // that still counts.
      if (e.killed || e.signal === 'SIGTERM' || e.code === 'ETIMEDOUT') {
        status = 'timed out'
      } else {
        // A non-zero exit is not automatically a failure - author exits 3 when
        // it refuses to file into a backlog nobody is draining, which is it
        // working.
        status = /STALLED|would file 0|at the WIP limit/.test(out) ? 'ok' : 'FAILED'
      }
    }
    // A step that RAN and delivered none of what it lined up is neither `ok`
    // nor `FAILED`. Asked only of a step that did not already fail or time out:
    // those two say something stronger, and must not be overwritten.
    const barren = status === 'ok' ? emptyHanded(out) : null
    if (barren) status = 'empty-handed'
    const verbatim = passThrough(out)
    const line = status === 'ok'
      ? summarise(out)
      : status === 'empty-handed'
        ? `EMPTY-HANDED - ${barren}; it reported: ${summarise(out)}`
        : status === 'timed out'
          ? `timed out part-way - what it did before that stands: ${summarise(out)}`
          : `FAILED\n${out.split('\n').slice(-4).join('\n')}`
    console.log(`    ${line}`)
    // The refusal and its reason, in the step's own words. Condensing these
    // away is what made `grep -c "REFUSED by the gate" feed.timer.log` answer 0
    // through every run that ever refused a brief.
    for (const l of verbatim) console.log(`      ${l.trim()}`)
    results.push({
      step: s.name,
      status,
      summary: line.slice(0, 120),
      emptyHanded: barren || undefined,
      // Kept, not only printed, for the same reason `evidence` is.
      verbatim: verbatim.length ? verbatim.map((l) => l.trim()) : undefined,
      // Same reason as in heal.mjs: a failure whose evidence is only printed is
      // a failure nobody can diagnose an hour later.
      evidence: (status === 'FAILED' || status === 'empty-handed')
        ? out.trim().split('\n').slice(-6).join(' | ').slice(0, 400)
        : undefined,
    })
  }

  const bad = results.filter((r) => r.status === 'FAILED')
  console.log(`\nfeed ${ACT ? 'complete' : '(report only)'}: ${results.map((r) => `${r.step}=${r.status}`).join(' ')}`)
  // NAMED AFTER THE TOKEN LIST, not only inside it. A step that fed the swarm
  // nothing is the one thing this chain exists to prevent, and reading it out
  // of a line of `step=status` tokens is how it went unread for 305 runs.
  const barrenSteps = results.filter((r) => r.status === 'empty-handed')
  for (const b of barrenSteps) {
    console.log(`  EMPTY-HANDED  ${b.step}: ${b.emptyHanded}`)
    for (const l of b.verbatim || []) console.log(`      ${l}`)
  }
  if (ACT) L.append({ kind: 'feed', results, elapsedMs: Date.now() - startedAt })
  process.exit(bad.length ? 1 : 0)
}
