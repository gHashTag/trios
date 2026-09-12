#!/usr/bin/env node
// Run every repair the swarm needs, in the order that makes them true.
//
// WHY A CHAIN AND NOT FOUR COMMANDS. The dashboard settled the argument. Across
// iterations 1 to 5 each repair was run by hand, and the counters it fixed grew
// back at almost exactly the rate they were cleared: `completed` went 6 -> 9 ->
// 10 -> 11 while being closed by hand each time. A repair that only happens
// when someone remembers is not a repair, it is a habit.
//
// THE ORDER IS NOT ARBITRARY.
//   1. reap       - free the volume first. A full disk kills every dispatch at
//                   0 s, and nothing below matters if bees cannot start.
//   2. lease      - release path fences whose claim has gone idle, so the next
//                   tick has candidates at all.
//   3. push-work  - make finished work visible on the remote. Must precede the
//                   close, or a closing comment names a branch that exists only
//                   inside a container.
//   4. close-done - clear accepted issues out of the candidate pool.
//
// WHAT IT WILL NOT DO. Nothing here forces a push, deletes a branch, closes an
// EPIC, releases a claim past the retry ceiling, or touches a worktree holding
// uncommitted work. Each step refuses those on its own; this file only orders
// them. `--dry` runs every step in its report mode.

import { execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const L = await import(path.join(DIR, 'loop.mjs'))
// ONE RULE, NOT A SECOND COPY OF IT. feed.mjs already carries how a chain reads
// a step's output, it is import-safe (everything it does lives behind its
// isMain guard), and the two chains condense the SAME steps - so the rule for
// what survives condensation, and for when a step came back empty-handed, is
// imported rather than transcribed. A rule written twice is two rules that
// agree until someone edits one; that argument is L0 and it applies here.
const F = await import(path.join(DIR, 'feed.mjs'))

// IMPORT-SAFE. This module ran its production query and called process.exit at
// import time, so importing it hit the live database and killed the importer -
// the calibration harness could not test it and died mid-run trying. A module
// that does work merely by being imported cannot be tested, and cannot be
// reused. Everything below the guard runs only when this file IS the program.
const isMain = process.argv[1] && process.argv[1].endsWith('/heal.mjs')


// argv only when this file IS the program, so importing it cannot make it
// think it was asked for a dry run - the class loop.mjs was caught in.
const DRY = isMain && process.argv.includes('--dry')

// EVERY RUN IS DATED, AT ITS FIRST LINE.
//
// Measured 2026-09-12: heal.timer.log held 24794 lines across 305 runs and not
// one clock - `grep -cE '[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{2}:[0-9]{2}:[0-9]{2}'`
// answered 0. A truncation found at log line 5710 could therefore be placed
// only as "somewhere in the earlier 23% of the file", not before or after any
// dated change to the thing that wrote it.
//
// Printed BEFORE the lock is asked for, so a run that stands down is dated too:
// those are the runs whose absence needs explaining. It is the opening bracket
// of the `heal complete:` line that already closes every run.
console.log(`${new Date().toISOString()} heal start${DRY ? ' (dry)' : ''}`)

// TAKE THE LOCK, unless this is a dry run.
//
// This exists because heal is now run by a timer as well as by hand, and the
// two must never overlap: an iteration mid-way through pushing branches while
// the timer starts closing issues would close against a branch that is only
// half-pushed. The loop's lock is the same one iterations take, so the two
// serialise against each other rather than each having a lock of its own.
//
let releaseLock = () => {}
let heldLock = false

// A dry run reads and writes nothing, so it does not queue behind anything.
// RE-ENTRANT FOR THE SAME LOGICAL RUN. An iteration already holds the lock, and
// it should still be able to run the chain as one of its steps. So a caller
// that is part of an existing run announces itself with LOOP_HOLDER; if that
// matches the current holder, heal proceeds and does NOT release on exit -
// releasing someone else's lock is how two writers end up running at once.
if (!DRY) {
  const state = L.lockHolder()
  const mine = process.env.LOOP_HOLDER && state && state.holder === process.env.LOOP_HOLDER
  if (!mine) {
    const got = L.acquire('heal', { singleProcess: true })
    if (!got.ok) {
      console.log(`another run holds the loop lock (${got.held.holder}, ${Math.round(got.ageMs / 60000)} min) - standing down`)
      L.append({ kind: 'heal-skipped', note: `lock held by ${got.held.holder}` })
      process.exit(0)
    }
    const release = () => { try { L.release() } catch { /* already gone */ } }
    releaseLock = release
    heldLock = true
    process.on('exit', release)
    process.on('SIGINT', () => { release(); process.exit(130) })
    process.on('SIGTERM', () => { release(); process.exit(143) })
  } else {
    console.log(`running inside ${state.holder}, which already holds the lock`)
  }
}

// The chain closes the whole cycle: repair, then check the output, then refill.
//
// The first four steps kept the swarm healthy and it still idled, because fuel
// was replenished whenever someone remembered - the same argument that made the
// repairs a chain in the first place. So authoring is the last link, and the
// audit sits between: work is checked before more is asked for.
const STEPS = [
  // The LAPTOP fills too, and for a year nothing reaped it. `git worktree add`
  // failed mid-checkout on 2026-09-04 with "No space left on device" at 100%
  // full, while reap.mjs reported the container volume healthy - which it was.
  // Landing eight PRs in a night is eight 205 MB checkouts, one of them 2.4 GB
  // with node_modules, none removed after merging.
  // FIRST, because it records the state that explains everything after it.
  //
  // Three steps failed at 01:15:53 with "your application is not running", and
  // `/health` answered ok a minute later. A minute is too late: by then the
  // world has moved. Sampling both views together, before the remote steps run,
  // is what turns the next outage into evidence instead of an anecdote.
  { name: 'two-views', file: 'two-views.mjs', act: '--record', dryArgs: '', why: 'both views of the service, at the same moment' },
  { name: 'reap-local', file: 'reap-local.mjs', act: '--reap', dryArgs: '', why: 'free the disk this loop runs on' },
  // BEFORE the reaper, because the reaper cannot take a tree that holds work and
  // the work is the only copy of itself. On 2026-09-05 nineteen trees refused
  // removal while the volume sat at 95% and every bee died at `git worktree
  // add`; they held 52 uncommitted paths, nine of them test files a bee had
  // written and never committed. Rescued first, the same reaper freed 28.6 G.
  { name: 'rescue', file: 'rescue.mjs', act: '--rescue', dryArgs: '', why: 'save work a bee never committed, before anything reclaims it' },
  { name: 'reap', file: 'reap.mjs', act: '--reap', why: 'free the volume before anything else' },
  { name: 'lease', file: 'lease.mjs', act: '--release', why: 'release idle path fences' },
  { name: 'push-work', file: 'push-work.mjs', act: '--push', why: 'make finished work visible' },
  // BETWEEN THE PUSH AND THE CLOSE, because that is where the gap was.
  //
  // close-done used to close on "the branch exists on the remote", and on that
  // basis 169 issues were closed while their code sat outside the base. It now
  // demands LANDED, so without this step nothing would ever close again. The
  // order is push -> land -> close, and each step is the precondition of the
  // next.
  // RIGHT AFTER THE PUSH, because that is when a worktree stops being needed.
  //
  // An hour after the volume went from 95% to 32% it was back at 87%: seventeen
  // worktrees, only TWO belonging to a running bee. Fifteen were left by
  // finished dispatches and eleven had their branch on the remote already -
  // about 27 GB of pure redundancy waiting for a watermark to notice.
  //
  // A worktree has a known lifetime. CI deletes a workspace when the job ends;
  // it does not wait for disk pressure to remember. The watermark reaper stays
  // as the backstop it should always have been.
  { name: 'reap-finished', file: 'reap-finished.mjs', act: '--reap', dryArgs: '', why: 'a worktree stops being needed when its work is published' },
  // AFTER the redundant trees are gone, share what the rest are duplicating.
  //
  // Eight of eleven worktrees carried their own node_modules at ~2.5 GB - about
  // 19 GB of identical packages on a 46 GB volume, which no collector can
  // out-pace. One store, a farm of links per worktree, and the workspace
  // packages linked back home. First run returned 14.3 GB.
  { name: 'share-modules', file: 'share-modules.mjs', act: '--share', dryArgs: '', why: 'one installed dependency tree, linked into every worktree' },
  // `resumable`: it lands a BOUNDED number per run, newest first, and its own
  // header states that merges happen in the object database - "a run that is
  // interrupted leaves the repository exactly as it found it". Both claims were
  // read before this flag was set, because capping a step that can be killed
  // mid-mutation would trade a starved tail for a corrupt index.
  { name: 'land', file: 'land.mjs', act: '--land', dryArgs: '', resumable: true, why: 'put accepted work into the branch it was accepted for' },
  { name: 'close-done', file: 'close-done.mjs', act: '--close', why: 'clear accepted issues from the pool' },
  // An escalation raised by a defect that has SINCE BEEN FIXED is never
  // re-examined by anything else. Three tasks sat 91 hours on the reason "no
  // acceptance criteria" while the parser that produced that reading was
  // corrected four hours after they were escalated (edbc05e11).
  //
  // This is not the wait valve wearing a new hat. `sendBack` and `wait` are
  // released by a CLOCK because their input can never change; this re-measures
  // the stated cause and releases nothing whose cause still holds - and nothing
  // at all whose issue body asks for a person in its own words.
  { name: 'stale-escalations', file: 'stale-escalations.mjs', act: '--release', dryArgs: '', why: 'retire escalations whose stated cause no longer reproduces' },
  // FUEL BEFORE READING MATTER, and this order was measured the hard way.
  //
  // `author` was the LAST link, behind five read-only steps. On 2026-09-04 one
  // chain run took 18 minutes, 6.5 of them in judge-packet assembling 31
  // transcripts - so the refill that keeps four workers busy waited behind work
  // whose only output is something for a person to read later. Worse, the run
  // outlived the 10-minute timer, and every fire after it found the lock held
  // and stood down. The swarm sat at zero while the chain was busy being
  // thorough.
  //
  // Everything above this line frees the swarm; everything below only reports.
  // The refill belongs on the freeing side of that line.
  { name: 'author', file: 'author.mjs', act: '--file', why: 'refill the backlog from a measured deficit' },
  // Reads only. A claim the diff does not support is a finding for a person,
  // never something this chain acts on by itself.
  // Two defect classes that each cost an outage, now checked every round rather
  // than when someone remembers - the argument that made this a chain at all.
  // Both read only; neither can change anything.
  // ---- the line the file has always drawn in prose, now drawn in data. ----
  // Everything above frees the swarm. Everything below only reports, and the
  // two must not compete for the same budget.
  //
  // THE LOOP'S OWN INSTRUMENTS, CHECKED BY A TIMER AND NOT BY A HUMAN.
  //
  // `selftest.mjs` holds 271 cases, every one planting a known defect and
  // asserting the tool notices - the harness built precisely because "a tool of
  // mine reported success without doing its job" runs through most of this
  // directory's lessons. Measured 2026-09-12: it ran on NO timer, in NO CI job
  // and under NO make target. The only way it ever fired was a human typing
  // `tri loop-selftest`. A guard that fires when somebody remembers is not a
  // guard - which is the same argument that made these repairs a chain.
  //
  // FIRST IN THE REPORTING PHASE, and that position is the whole of the
  // scheduling argument. It needs no network and no container, it takes about
  // two seconds, and the lock has been released one line above it - so it
  // cannot starve the steps that reach the remote, and being first means the
  // slow audits behind it cannot push it past the reporting deadline. It takes
  // no argument: the tool has one mode and it is read-only.
  // `pinFirst` exempts it from the hunger rotation below: this position is
  // argued for, not incidental, and the rotation must not take it away.
  { name: 'loop-selftest', file: 'selftest.mjs', reportsOnly: true, pinFirst: true, act: '', dryArgs: '', why: 'the loop\'s own checkers, shown failing before they are believed' },
  { name: 'clocks', file: 'clocks.mjs', act: '', dryArgs: '', reportsOnly: true, why: 'no decision keyed on a field something rewrites' },
  { name: 'fields', file: 'fields.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'no decision reading a field its query never selects' },
  // The checkers checking themselves, against material the WORLD calls good.
  // Six false accusations shipped in one night while the synthetic fixtures
  // agreed with the checkers that wrote them.
  { name: 'fp-check', file: 'fp-check.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'no checker accuses anything known good' },
  // THE RING AGAINST ITS TWIN, EVERY ROUND.
  //
  // L0's whole argument is that a rule transcribed into four languages is four
  // rules that agree until someone edits one. `can_start_another` is in this
  // tree three times today and they agree; nothing anywhere would have noticed
  // when they stopped. Comparing them is cheap - 460 exhaustive cases, a few
  // seconds - and it is the only thing standing between "they agree" and "they
  // agreed when somebody last looked."
  // ONE FILE IN TWO PLACES, EVERY ROUND. Seventeen Swift files exist in both
  // rings/SR-00 and agent-server/queen-core/Sources. Nothing compared them, and
  // on the first run one of the seventeen was already being edited on one side
  // only - identical at HEAD, 45 lines apart on disk. Catching that before it
  // lands is the whole difference between a warning and a fork.
  // WHAT THE DEPLOYMENT SERVES A STRANGER, every round. The source audit was
  // correct and red for days while /queen/needs-you answered 200 to a hostile
  // Origin with the escalations and their worker-written reasons. A guard added
  // in source is not a guard in production until a deploy happens, and a guard
  // removed is a hole immediately - only a live probe knows which is true now.
  // THE BACKLOG'S BRIEFS, EVERY ROUND. All 35 accepted verdicts whose briefs
  // state nothing a checker can reach fail this gate - every one would have
  // been caught before it was filed. 19 of 197 open issues fail it today, and
  // those 19 are the next batch of unauditable verdicts unless somebody looks.
  // A FINISHED DISPATCH WITH NO VERDICT BLOCK WAITS FOR EVER, and `wait` is
  // deliberately the one valve a timer must not touch - so nothing else looks.
  // Sixteen sit there now, every note saying "0 of N criteria judged".
  { name: 'unverdicted', file: 'unverdicted.mjs', reportsOnly: true, act: '--limit 6', dryArgs: '--limit 6', why: 'a finished dispatch whose worker never wrote a verdict block' },
  { name: 'silent-loop', file: 'silent-loop.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'an attempt that charges no retry budget can be repeated for ever' },
  { name: 'rejudge', file: 'rejudge.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'a recorded verdict the current code would no longer give' },
  { name: 'idle', file: 'idle.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'how much of the day the swarm spent doing nothing, and what stopped it' },
  { name: 'unwired', file: 'unwired.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'a gate that exists and runs nowhere' },
  { name: 'agree', file: 'agree.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'two implementations of one rule, asked the same question about real rows' },
  { name: 'brief-gate', file: 'brief-gate.mjs', reportsOnly: true, act: '--open', dryArgs: '--open', why: 'which open briefs will produce a verdict nothing can check' },
  { name: 'exposure', file: 'exposure.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'what the live service serves to an origin it has never heard of' },
  { name: 'forked-files', file: 'forked-files.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'a file that exists twice must not start saying two things' },
  { name: 't27-parity', file: 't27-parity.mjs', reportsOnly: true, act: '', dryArgs: '', why: 'the generated ring and the twin in production still answer the same' },
  { name: 'verdict-audit', file: 'verdict-audit.mjs', reportsOnly: true, act: '--accepted', dryArgs: '--accepted', why: 'check what the swarm claims against what it pushed' },
  // Queue what no mechanical check can reach for a judge to read. Assembles
  // only - the judgement is an explicit act, never something this performs.
  // The audit says what each verdict is worth. This says whether the swarm's
  // RECENT work is worth less than the baseline this same process established -
  // the one question a per-verdict audit cannot answer, because it is about the
  // distribution and not about any one branch.
  { name: 'proven', file: 'proven.mjs', reportsOnly: true, act: '--record', dryArgs: '', why: 'is the recent work still proving anything' },
  { name: 'judge-packet', file: 'judge-packet.mjs', reportsOnly: true, act: '--unauditable', dryArgs: '--unauditable', why: 'queue the unauditable for judgement' },
  // THE BOX'S ANCHOR HAD NO WRITER.
  //
  // `dash.mjs --record` appears in no STEPS list, no plist and no Makefile
  // target, and the newest line in `dash-readings.jsonl` was seven days old on
  // 2026-09-13. Every delta on the box was therefore measured against a week
  // ago and printed as though it meant "since last time" - `selftest cases +27`
  // over seven days, `bees running -4` over seven days.
  //
  // `--if-due` keeps this cheap: the facts cost about 110 s to take and the
  // cadence is an hour, so five runs in six exit in milliseconds having
  // measured nothing. The pacing lives in dash.mjs beside the cadence it is
  // paced by, not in a second timer that would hold a second copy of it.
  { name: 'dash', file: 'dash.mjs', reportsOnly: true, act: '--record --if-due --no-color', dryArgs: '--no-color', why: 'the box compares against a reading somebody has to write' },
]

// One line per step, taken from the step's own output rather than invented, so
// the summary cannot claim more than the step reported.
const SUMMARY = [
  [/(\d+) tree\(s\) rebuilt against one store, about (\d+) MB returned/, (m) => `${m[1]} tree(s) share one store, ${m[2]} MB returned`],
  [/(\d+) worktree\(s\): (\d+) with a private install/, (m) => `${m[2]} of ${m[1]} worktrees still carry a private install`],
  [/removed (\d+) of (\d+) redundant worktree\(s\), (\d+) refused/, (m) => `${m[1]} redundant worktree(s) removed, ${m[3]} still hold work`],
  [/(\d+) worktree\(s\): (\d+) redundant, (\d+) kept/, (m) => `${m[2]} of ${m[1]} worktrees are redundant`],
  [/(\d+) uncommitted path\(s\) across (\d+) tree\(s\), (\d+) tree\(s\) committed/, (m) => `rescued ${m[1]} stranded path(s) from ${m[3]} tree(s)`],
  [/STRANDED total=0/, () => 'nothing stranded in any worktree'],
  [/THEY DISAGREE/, () => 'the two views of the service DISAGREE - a green health check is not evidence the channel will connect'],
  [/http ok   ssh attached   they agree/, () => 'both views agree the service is up'],
  [/ACT NOW: the recent window proves LESS/, () => 'the recent window proves measurably less than the baseline - read the newest verdicts'],
  [/WATCH: the recent rate is lower/, () => 'recent rate lower but inside the noise - watch, do not act'],
  [/overall (\d+)\/(\d+) judged verdicts prove something/, (m) => `${m[1]} of ${m[2]} judged verdicts prove something`],
  [/removed (\d+) of (\d+), freeing about (\d+) MB/, (m) => `${m[1]} merged worktree(s) removed, ${m[3]} MB freed`],
  [/(\d+) worktree\(s\): 0 merged and clean/, (m) => `${m[1]} worktrees, none removable`],
  [/(\d+) worktree\(s\): (\d+) merged and clean/, (m) => `${m[1]} worktrees, ${m[2]} merged and clean`],
  [/reclaimed ([\d.-]+) G/, (m) => `reclaimed ${m[1]} G`],
  [/below the high-water mark/, () => 'volume below the threshold, nothing reaped'],
  [/release (\d+)\s+quarantine (\d+)\s+hold (\d+)/, (m) => `released ${m[1]}, quarantined ${m[2]}, held ${m[3]}`],
  [/pushed (\d+)/, (m) => `pushed ${m[1]}`],
  [/not pushed: 0/, () => 'every branch with work is already on the remote'],
  [/landed (\d+) of (\d+) clean/, (m) => `${m[1]} accepted branch(es) landed`],
  [/REFUSING to continue/, () => 'REFUSED - the landed count did not move; the measure is wrong'],
  [/(\d+) landable, showing/, (m) => `${m[1]} accepted branches still outside the base`],
  [/closed (\d+)\s+failed (\d+)/, (m) => `closed ${m[1]}, failed ${m[2]}`],
  [/closable 0/, () => 'nothing closable'],
  [/released (\d+) of (\d+) back to the pool/, (m) => `${m[1]} escalation(s) retired - the cause no longer reproduces`],
  [/(\d+) escalation\(s\): 0 raised on a cause/, (m) => `${m[1]} escalation(s), every cause still holds`],
  [/(\d+) escalation\(s\): (\d+) raised on a cause/, (m) => `${m[1]} escalation(s), ${m[2]} on a cause that no longer holds`],
  [/(\d+) measurement\(s\): (\d+) on immutable/, (m) => `${m[1]} clocks, ${m[2]} on fields nothing rewrites`],
  [/(\d+) query region\(s\): (\d+) complete/, (m) => `${m[1]} query regions, ${m[2]} selecting every field they read`],
  [/(\d+) known-good input\(s\): (\d+) clean, (\d+) accused/, (m) => `${m[1]} known-good inputs, ${m[3]} falsely accused`],
  [/CLAIM UNSUPPORTED: (\d+)/, (m) => `${m[1]} CLAIM(S) UNSUPPORTED - a person should look`],
  [/SUPPORTED: (\d+)/, (m) => `${m[1]} claims supported by the diff, none unsupported`],
  [/packets written (\d+)\s+skipped (\d+)/, (m) => `${m[1]} packet(s) queued for judgement, ${m[2]} skipped`],
  // The selftest's own tally, read as a number rather than as an exit code: it
  // exits 1 when a case fails, and that is the tool working.
  [/^(\d+) passed, (\d+) failed$/m, (m) => (Number(m[2])
    ? `${m[2]} of ${Number(m[1]) + Number(m[2])} loop selftest case(s) FAILING`
    : `all ${m[1]} loop selftest cases pass`)],
  [/STALLED: /, () => 'REFUSED to file - nobody is draining the backlog'],
  [/filed (\d+)/, (m) => `filed ${m[1]}`],
  [/at the WIP limit|already has an issue/, () => 'at the WIP limit, nothing filed'],
]

// A DEADLINE FOR THE WHOLE CHAIN, not just for each step.
//
// Each step had a 10-minute timeout and there are eleven of them, so the worst
// case was 110 minutes against a timer that fires every 10. One slow run held
// the lock for 18 minutes and starved the swarm for all of it, and nothing in
// here noticed. A chain that can outlive its own cadence is a chain that
// schedules its own outage.
//
// Steps are skipped, never truncated: a half-run step is worse than an unrun
// one, and the summary names every step that did not get its turn.
// TWO BUDGETS, BECAUSE THEY ARE TWO JOBS.
//
// One deadline for the whole chain meant the audits paid for the freeing steps.
// Measured 2026-09-05: reap, push-work, land, close-done and author consumed the
// full eight minutes, and verdict-audit, proven and judge-packet were all
// SKIPPED - not because they are slow (a warm audit is ten seconds) but because
// nothing was left. The chain reported itself complete with a third of its
// steps never run, which is the same shape as every defect in this directory: a
// confident answer about work that did not happen.
//
// So the steps that FREE the swarm have the first budget, and the steps that
// only REPORT have their own, starting when the first phase ends. A slow reap
// can no longer silence the audit that would have found what the reap was for.
const DEADLINE_MS = Number(process.env.HEAL_DEADLINE_MS ?? 8 * 60 * 1000)
const REPORT_DEADLINE_MS = Number(process.env.HEAL_REPORT_DEADLINE_MS ?? 5 * 60 * 1000)
let reportPhaseStartedAt = null
const startedAt = Date.now()

// TWO BUDGETS SPLIT ONE STARVATION INTO TWO. The fix above is real and it did
// not work, and the way it failed is the point: a deadline in FIXED ORDER does
// not ration a phase, it truncates it. Whatever sits at the end of a phase is
// not slow - it is last, and last is a permanent condition.
//
// Measured 2026-09-12 over the 230 heal runs in the ledger since 2026-09-06,
// counting how many times each step actually RAN (ok or FAILED, not skipped and
// not timed out):
//
//   land               1 of 230   (timed out 142, skipped 87)
//   close-done         1 of 230
//   stale-escalations  1 of 230
//   author             1 of 230
//   brief-gate         2 of 230
//   exposure           2 of 230   forked-files 2, t27-parity 2
//   verdict-audit      1 of 230
//   proven             1 of 230
//   judge-packet       0 of 230
//
// Ten steps at or below 1%, and every one of those 230 runs still printed
// `heal complete`. `verdict-audit` is the instrument that checks what the swarm
// CLAIMS against what it PUSHED; it ran once in a week. During that week the
// swarm reported 439 of 439 dispatches finished and put nothing on the remote
// after 2026-09-05T17:30Z. The audit that exists to catch exactly that was
// starved by position, and the summary line said the chain was complete.
//
// SO THE REPORTING PHASE IS ORDERED BY HUNGER, NOT BY THE ARRAY. Least recently
// reached goes first. A step skipped this run is first in line next run, so the
// tail rotates instead of starving and every step gets its turn across a few
// rounds. This is only sound because the reporting steps READ - the file says so
// four times and the lock is released before the first of them - so their order
// carries no meaning to preserve. The freeing phase is NOT rotated: push -> land
// -> close is a precondition chain and reordering it would close issues whose
// code never landed, which is the 169-issue defect `land.mjs` was written for.
// The least time in which a step could plausibly finish. Below this a step is
// not started at all, because starting it would only kill it and spend the
// budget doing so. Also the unit of the reserve a `resumable` step leaves for
// the steps behind it.
const MIN_START_MS = Number(process.env.HEAL_MIN_START_MS ?? 60000)

const REACH_FILE = path.join(DIR, 'state', 'step-reach.json')
let reachedAt = {}
try { reachedAt = JSON.parse(fs.readFileSync(REACH_FILE, 'utf8')) } catch { reachedAt = {} }

function orderedSteps() {
  const freeing = STEPS.filter((s) => !s.reportsOnly)
  const reporting = STEPS.filter((s) => s.reportsOnly)
  // `loop-selftest` documents its own position and the argument is sound: it
  // needs no network, takes about two seconds, and being first means the slow
  // audits cannot push it past the deadline. A pinned step is exempt.
  const pinned = reporting.filter((s) => s.pinFirst)
  const rotating = reporting.filter((s) => !s.pinFirst)
  // Never reached at all sorts before reached-long-ago, which sorts before
  // reached-just-now. Ties keep their declared order, which `sort` preserves.
  rotating.sort((a, b) => (reachedAt[a.name] ?? 0) - (reachedAt[b.name] ?? 0))
  return [...freeing, ...pinned, ...rotating]
}

const ORDER = orderedSteps()

/**
 * How many steps in the same phase still come after this one.
 *
 * Used to reserve their minimum start time so a single long step cannot spend
 * the phase. See the `resumable` cap below for why that is not a theoretical
 * worry.
 */
function stepsBehind(i) {
  const phase = !!ORDER[i].reportsOnly
  return ORDER.slice(i + 1).filter((x) => !!x.reportsOnly === phase).length
}

const results = []
for (let i = 0; i < ORDER.length; i++) {
  const s = ORDER[i]
  if (s.reportsOnly && reportPhaseStartedAt === null) {
    reportPhaseStartedAt = Date.now()
    // THE AUDITS DO NOT NEED THE LOCK, AND HOLDING IT STARVES THE REFILL.
    //
    // Everything above this line changes the swarm's shared state and must not
    // run twice at once. Everything below only reads it. But the lock covered
    // both, so a full run held it for up to thirteen minutes - eight for the
    // freeing phase, five for the audits - and `feed`, which fires every 300
    // seconds and exists precisely to refill the queue, stood down every single
    // time.
    //
    // Measured 2026-09-05: the swarm sat at zero bees for eleven minutes while
    // the chain was in `fp-check`, an entirely read-only step.
    //
    // So the lock is released at the phase boundary. The reporting steps write
    // only their own caches and records - a verdict cache, a paired sample, a
    // dashboard reading - where a second writer costs nothing, and none of them
    // touches a branch, an issue or a worktree.
    if (heldLock) {
      releaseLock()
      heldLock = false
      console.log('\n  lock released: everything from here only reads, and the refill needs it')
    }
  }
  const budget = s.reportsOnly ? REPORT_DEADLINE_MS : DEADLINE_MS
  const since = s.reportsOnly ? reportPhaseStartedAt : startedAt
  const left = budget - (Date.now() - since)
  if (left <= 0) {
    const which = s.reportsOnly ? 'reporting' : 'swarm-freeing'
    process.stdout.write(`\n--- ${s.name}  (${s.why})\n    SKIPPED - past the ${Math.round(budget / 60000)} minute ${which} deadline\n`)
    results.push({ step: s.name, status: 'skipped', summary: `past the ${which} deadline` })
    continue
  }
  // A STEP STARTED WITH THIRTY SECONDS LEFT IS NOT BEING RUN, IT IS BEING KILLED.
  //
  // `Math.max(30000, ...)` raised a nearly-spent budget back up to half a minute
  // and started the step anyway. `land.mjs` takes 45 seconds in report mode -
  // measured 2026-09-12, twice, on this machine - so whenever the chain reached
  // it with under 45 seconds left it was launched, killed at the floor, and
  // recorded `timed out`. It read as a slow step. It is not a slow step. It was
  // never given enough time to finish, 142 times out of 230.
  //
  // And the kill was not free: the thirty seconds came out of the same budget,
  // so `close-done` and `author` behind it inherited a deadline that was already
  // past. One step that cannot finish took the two steps after it down with it -
  // which is how `author`, the step that REFILLS THE BACKLOG, ran once in 230
  // runs while the swarm reported "nothing to choose".
  //
  // So: below the floor the step is not started. It is skipped, and it is
  // skipped under its own name - `no budget left to finish` is a different
  // sentence from `timed out`, and the difference is the whole diagnosis.
  if (left < MIN_START_MS) {
    process.stdout.write(`\n--- ${s.name}  (${s.why})\n    SKIPPED - ${Math.round(left / 1000)}s left, under the ${Math.round(MIN_START_MS / 1000)}s a step needs to finish\n`)
    results.push({ step: s.name, status: 'skipped', summary: 'no budget left to finish' })
    continue
  }
  const args = DRY ? (s.dryArgs || '') : s.act
  process.stdout.write(`\n--- ${s.name}  (${s.why})\n`)
  let out = ''
  let status = 'ok'
  // HOW LONG EACH STEP TOOK, RECORDED. Nothing measured this. The chain has an
  // eight-minute budget it has been exhausting for a week and no instrument in
  // this directory could say where the eight minutes went - so every proposal to
  // reorder it, including the ones I nearly wrote, was a guess. Reordering a
  // chain whose costs are unmeasured is the thing this project keeps calling a
  // defect when other people do it.
  const stepStartedAt = Date.now()

  // NO STEP MAY SPEND THE WHOLE PHASE WHILE THE STEPS BEHIND IT ARE THE POINT.
  //
  // Measured on the live 17:01:33Z run, the first with a real budget: `land`
  // started 3m30s in and was still running 4m30s later when the eight-minute
  // freeing deadline cut it. It reported `timed out`, and `close-done`,
  // `stale-escalations` and `author` behind it were skipped - again. So the
  // hunger rotation fixed the reporting phase and left this untouched: one step
  // at the head of the line consuming everything is not a queueing problem, it
  // is a queueing problem ONLY for the steps behind it, and `author` is the step
  // that refills the backlog.
  //
  // My earlier figure was wrong and worth naming: I measured `land` at 45s and
  // wrote a floor around that number. 45s was REPORT mode. In act mode it merges
  // branches and takes minutes. A measurement taken in the wrong mode is the
  // same defect as a measurement taken against the wrong root.
  //
  // `resumable` marks a step that makes partial progress and continues next run.
  // `land.mjs` says so in its own header - it lands a bounded number per run,
  // newest first - and this chain already prints "what it did before the cut
  // still stands". For such a step, being cut is not a loss, and the reserve
  // below buys the steps behind it their minimum start. For every other step the
  // budget is unchanged: cutting a step that CANNOT resume would just throw its
  // work away, which is worse than starving the tail.
  let cap = Math.min(300000, left)
  let want = 0
  if (s.resumable) {
    want = stepsBehind(i) * MIN_START_MS
    cap = Math.min(cap, Math.max(MIN_START_MS, left - want))
  }
  // REPORT WHAT WAS HELD, NOT WHAT WAS WANTED. The first version of this line
  // printed `want`. On the 200s dry run it announced "holding 180s for the 3
  // steps behind it" and then those three steps were skipped with "17s left" -
  // because `Math.max(MIN_START_MS, ...)` floors the cap, so when the budget is
  // too small to seat everyone the reserve is silently given back. The floor is
  // right: a step that cannot even start is worth less than one that runs 60s
  // and resumes. The MESSAGE was wrong, and wrong in this repository's signature
  // way - a number computed one way and reported another. `held` is subtraction
  // of two numbers on this line, so it cannot disagree with what happens next.
  const held = left - cap
  if (want && cap < left) {
    const behind = stepsBehind(i)
    const short = held < want ? `, ${Math.round((want - held) / 1000)}s short of the ${Math.round(want / 1000)}s they need` : ''
    process.stdout.write(`    capped at ${Math.round(cap / 1000)}s, leaving ${Math.round(held / 1000)}s for the ${behind} step(s) behind it${short} (it resumes next run)\n`)
  }

  try {
    out = execSync(`node ${path.join(DIR, s.file)} ${args}`, {
      encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
      // Never longer than what remains of the chain's own deadline, and never so
      // long that the steps behind it cannot start.
      timeout: cap,
    })
  } catch (e) {
    // A step that exits non-zero is not automatically a failure: reap exits 1
    // to mean "would act", which is its report mode saying yes.
    out = String(e.stdout || '') + String(e.stderr || '')
    // EXIT 2 FROM coverage IS A FINDING, NOT A BREAKAGE.
    //
    // The tool exits 2 when the recent window proves measurably less than the
    // baseline. That is the tool working, and calling it FAILED would bury the
    // one thing it exists to say under a word this chain uses for "the step is
    // broken". It gets its own status so both can be read.
    // A STEP THAT COULD NOT REACH THE CONTAINER HAS NOT FAILED. The container
    // was unreachable, once, for the whole run - and recording that as four
    // separate step failures is what inflated every rate this loop has quoted.
    // A STEP KILLED BY THIS CHAIN'S OWN TIMEOUT WAS BEING RECORDED AS `ok`.
    //
    // `feed.mjs` has had this arm since the day a step was cut off mid-run; this
    // file never got one. `git log -S ETIMEDOUT -- heal.mjs` is empty. So a step
    // that the chain itself SIGTERMed at its `timeout:` produced no output,
    // matched none of the patterns below, and fell through to `ok`.
    //
    // MEASURED on this machine: heal.timer.log holds 62 `(no output)` lines -
    // land 25, fp-check 22, author 5, reap-local 4, lease 2, judge-packet 2,
    // verdict-audit 1, close-done 1 - and every one was recorded ok. That is 25
    // of land's 36 ok records describing a step that produced not one byte.
    // `feed.timer.log` holds zero, because feed classifies the kill.
    //
    // A step cut off part-way is not a step that succeeded and not a step that
    // broke. What it did before the cut still stands, and saying so is the
    // difference between a chain that reports work and one that reports minutes.
    // A FAILING SELFTEST CASE IS A FINDING, NOT A BROKEN STEP - and it must not
    // be `ok` either. The tool exits 1 when one of its cases fails, and that is
    // the tool doing its job; the operator needs the sentence, not the code.
    // Anchored on the tally line the harness prints last, so it cannot be
    // matched by a step that happens to use the words "passed" and "failed".
    status = (e.killed || e.signal === 'SIGTERM' || e.code === 'ETIMEDOUT') ? 'timed out'
      : /THEY DISAGREE/.test(out) ? 'FINDING'
        : /the channel was already found down in this run/.test(out) ? 'channel-down'
          : /ACT NOW:/.test(out) ? 'FINDING'
            : /^\d+ passed, [1-9]\d* failed$/m.test(out) ? 'FINDING'
              : /Error:|Traceback|not a function|ENOENT/.test(out) ? 'FAILED' : 'ok'
  }
  let line = null
  for (const [re, fmt] of SUMMARY) {
    const m = out.match(re)
    if (m) { line = fmt(m); break }
  }
  // A STEP THAT RAN AND DELIVERED NONE OF WHAT IT LINED UP IS NEITHER ok NOR
  // FAILED. Asked only of a step that did not already fail, time out or raise a
  // finding: each of those says something stronger and must not be overwritten.
  const barren = status === 'ok' ? F.emptyHanded(out) : null
  if (barren) status = 'empty-handed'
  const verbatim = F.passThrough(out)
  console.log(`    ${status === 'FAILED' ? 'FAILED'
    : status === 'timed out' ? `timed out - what it did before the cut still stands: ${line || '(nothing recorded)'}`
      : status === 'empty-handed' ? `EMPTY-HANDED - ${barren}; it reported: ${line || '(nothing recorded)'}`
        : line || out.trim().split('\n').pop() || '(no output)'}`)
  if (status === 'FAILED') console.log(out.trim().split('\n').slice(-4).map((l) => '      ' + l).join('\n'))
  // THE REFUSAL AND ITS REASON, IN THE STEP'S OWN WORDS.
  //
  // Condensing these away is what made `grep -c "REFUSED by the gate"` answer 0
  // on both timer logs while the gate refused five briefs on every acting run.
  // One line per step is right for a count; it is wrong for a sentence nothing
  // else will ever say.
  for (const l of verbatim) console.log(`      ${l.trim()}`)
  // THE EVIDENCE IS KEPT, NOT ONLY PRINTED.
  //
  // `line` is set only when a SUMMARY pattern matches, and no pattern matches a
  // failure - so the ledger recorded `line: null` for every failed step and the
  // reason went to a terminal nobody was watching.
  //
  // Measured 2026-09-05 over the whole ledger: push-work, the ONE step that gets
  // a bee's work out of the container, ran 66 times and 47 were not ok. Every
  // one of the 46 FAILED entries carries an empty summary, so what went wrong on
  // any of them cannot now be known. The console had it. The record did not.
  // REACHED, and the word is chosen. A step that TIMED OUT was started and
  // killed; it got its turn and spent it. `land` timing out 142 times is not the
  // same defect as `judge-packet` never being started at all, and the rotation
  // must not treat them alike - a step that eats five minutes every round would
  // otherwise be marked hungry and promoted to the front for ever.
  reachedAt[s.name] = Date.now()
  results.push({
    step: s.name,
    status,
    ms: Date.now() - stepStartedAt,
    line: line || null,
    emptyHanded: barren || undefined,
    // Kept, not only printed, for exactly the reason `evidence` is.
    verbatim: verbatim.length ? verbatim.map((l) => l.trim()) : undefined,
    evidence: (status === 'FAILED' || status === 'FINDING' || status === 'channel-down'
      || status === 'timed out' || status === 'empty-handed')
      ? out.trim().split('\n').slice(-6).join(' | ').slice(0, 400)
      : undefined,
  })
}

try {
  fs.mkdirSync(path.dirname(REACH_FILE), { recursive: true })
  fs.writeFileSync(REACH_FILE, JSON.stringify(reachedAt, null, 1))
} catch (e) {
  // A rotation cursor that cannot be written costs the next run its ordering and
  // nothing else. It must never cost this run its summary.
  console.log(`  (could not record step reach: ${e.message})`)
}

console.log(`\n${DRY ? 'DRY RUN - ' : ''}heal complete: ` +
  results.map((r) => `${r.step}=${r.status}`).join(' '))

// "COMPLETE" IS A CLAIM, AND FOR 230 RUNS IT WAS FALSE.
//
// The line above has always named every skipped step, and naming is not
// announcing: `judge-packet=skipped` is one token among twenty-seven and it was
// read by nobody for a week. The word `complete` sat at the front of all of them
// while a third of the chain had not run. That is this directory's oldest defect
// - a confident summary of work that did not happen - printed by the instrument
// that exists to catch it.
//
// So the count gets a sentence of its own, and the steps that did not get their
// turn are named in it.
const neverRan = results.filter((r) => r.status === 'skipped')
if (neverRan.length) {
  console.log('')
  console.log(`  NOT REACHED: ${neverRan.length} of ${results.length} steps never started - ` +
    neverRan.map((r) => r.step).join(', '))
  console.log('  They are first in line next run; the reporting phase is ordered by hunger.')
  console.log('  tri reach   - how often each step has actually run, over the whole ledger')
}

// A FAILURE OF THE STEP THAT UNBLOCKS EVERYTHING IS NOT A FAILURE LIKE THE
// OTHERS.
//
// Measured 2026-09-05: the summary read
//
//   reap=FAILED lease=FAILED push-work=FAILED land=ok close-done=FAILED ...
//
// and nothing about that shouted. The container volume was 100% full, 71 MB of
// 46 GB, sixty worktrees; every bee was dying at `git worktree add: unable to
// write file`, and the issue just handed to the swarm never ran a line.
//
// The reaper had failed because `railway ssh` refused with "Your application is
// not running or in a unexpected state" - the application being unhealthy
// BECAUSE the volume was full. The tool that repairs the failure reaches through
// the thing the failure breaks, so it needs retrying rather than believing.
//
// An audit that could not run costs a round. A reaper that could not run costs
// the fleet, and the two must not print the same way.
const CRITICAL = new Set(['reap', 'reap-local', 'lease', 'push-work', 'land', 'close-done', 'author'])
// A FINDING is shown, loudly, and does not count as a broken step. The chain's
// exit code is about whether the chain ran; the finding is about what it saw.
const findings = results.filter((r) => r.status === 'FINDING')
if (findings.length) {
  console.log('')
  // `summary` is only set on SKIPPED records; a FINDING carries `line`. This
  // printed `undefined` for every finding it has ever announced - the one word
  // the operator was meant to read.
  for (const f of findings) console.log(`  FINDING  ${f.step}: ${f.line || f.evidence || '(no detail recorded)'}`)
}
// AND SO IS A STEP THAT CAME BACK WITH NOTHING.
//
// Named after the token list, not only inside it. `author=ok` under `filed 0`
// was readable for 305 runs and read by nobody, because a word in a line of
// forty `step=status` tokens is not an announcement. A step that produced none
// of what it lined up gets a sentence, and the refusals it swallowed get to
// speak for themselves underneath it.
const barrenSteps = results.filter((r) => r.status === 'empty-handed')
if (barrenSteps.length) {
  console.log('')
  for (const b of barrenSteps) {
    console.log(`  EMPTY-HANDED  ${b.step}: ${b.emptyHanded}`)
    for (const l of b.verbatim || []) console.log(`      ${l}`)
  }
  const criticalBarren = barrenSteps.filter((r) => CRITICAL.has(r.step))
  if (criticalBarren.length) {
    console.log(`  ${criticalBarren.map((r) => r.step).join(', ')} FREE the swarm, and this round they freed nothing.`)
  }
}
const failed = results.filter((r) => r.status === 'FAILED')
const criticalFailures = failed.filter((r) => CRITICAL.has(r.step))
if (criticalFailures.length) {
  console.log('')
  console.log(`URGENT: ${criticalFailures.length} step(s) that FREE the swarm failed - ${criticalFailures.map((r) => r.step).join(', ')}.`)
  console.log('These are not audits. While they fail the swarm is being starved, and the')
  console.log('failure of `reap` in particular is circular: it reaches the volume through the')
  console.log('container, which stops answering once the volume is full. Retry it.')
  console.log('  tri why    - it checks the CONTAINER volume now, not just this laptop')
}

L.append({ kind: DRY ? 'heal-dry' : 'heal', critical: criticalFailures.map((r) => r.step), results })

process.exit(failed.length ? 1 : 0)
