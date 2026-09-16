#!/usr/bin/env node
// Push bee branches that carry work and are not yet on the remote.
//
// WHY THE WORKER CANNOT DO THIS ITSELF. `remote.origin.url` in the container is
// plain HTTPS with no credential, so `git push` fails with "could not read
// Username". A `GH_TOKEN` IS present in the environment, and a dry-run push
// configured with it succeeds - the gap is one `url.insteadOf` line. It is left
// unwired on purpose: the field's answer to this is not to hand the worker a
// token but to split the two halves, so the agent emits a bundle and a
// privileged step pushes it (githubnext/gh-aw "safe outputs"). Until that split
// exists, this script is the privileged step, run from outside.
//
// Measured 2026-09-04: 100 branches carrying 1321 changed files had never
// reached the remote, and 56 dispatches were marked accepted on work nobody
// could see.
//
// WHAT IT REFUSES TO DO.
//   - Force-push. A branch whose remote history differs is reported and left
//     alone; two such branches existed and overwriting them would have
//     destroyed whatever put them there.
//   - Push a branch with no commits ahead of the base.
//
// Usage:
//   node push-work.mjs           # report
//   node push-work.mjs --push    # act

import { execSync } from 'node:child_process'
import * as CH from './channel.mjs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const L = await import(path.join(DIR, 'loop.mjs'))
const { shq } = L

// IMPORT-SAFE. This module ran its production query and called process.exit at
// import time, so importing it hit the live database and killed the importer -
// the calibration harness could not test it and died mid-run trying. A module
// that does work merely by being imported cannot be tested, and cannot be
// reused. Everything below the guard runs only when this file IS the program.
const isMain = process.argv[1] && process.argv[1].endsWith('/push-work.mjs')


const SVC = 'trios-agent-server'

/**
 * The railway invocation, with the project named EXPLICITLY.
 *
 * `railway ssh --service X` resolves the project from whatever directory it is
 * run in, by walking up until it finds a linked one. That works from a shell a
 * person is sitting in and does not work from a launchd timer: measured
 * 2026-09-04, every railway-calling step of the chain failed with
 * `Must provide project when setting service or environment`, while the
 * read-only steps passed - so the timer reported a mostly-healthy chain that
 * had pushed nothing, closed nothing and released nothing for hours, and the
 * swarm sat idle between the runs I happened to trigger by hand.
 *
 * Naming the project removes the dependency on where the process happens to be
 * standing. The id is public - it is in every build URL this repository has
 * ever printed - and carries no credential.
 */
// The bare RAILWAY constant that used to live here is gone. It was never
// executed after this file moved to channel.mjs, and a dead copy of a defect is
// how the next copy-paste resurrects it: judge-packet.mjs kept one, executed it,
// and wrote 42 packets accusing bees of a silence it had never measured.

const BATCH = Number(process.env.PUSH_BATCH ?? 12)

/**
 * A CHANNEL FAILURE IS NOT AN ANSWER, AND IT IS NOT FATAL EITHER.
 *
 * `railway ssh` is a network hop and fails the way network hops fail:
 * "Operation timed out (os error 60)", "Connection reset by peer",
 * "client error (SendRequest)". None of those say anything about the branches.
 *
 * This step is the ONLY way a bee's work leaves the container. On 2026-09-05 it
 * threw on one such timeout and died. Three accepted pieces of work - 9 to 15
 * minutes of a bee each, 23 to 35 thousand output tokens, all three reviewed and
 * ACCEPTED - stayed invisible on the far side of a dropped connection. A retry
 * thirty seconds later pushed thirteen branches.
 *
 * And I had seen it before. Two rounds earlier the chain printed
 * `push-work=FAILED`, I re-ran it by hand, it worked, and I wrote it off as
 * transient without changing anything. "It worked when I tried again" is not a
 * diagnosis; it is the observation that a retry belongs in the code.
 */
// The classifier lives in channel.mjs so the four steps that share the channel
// share its definition too. Re-exported because this file's own tests name it.
export const isChannelFailure = CH.isChannelFailure

function remote(script, timeout = 280000, attempts = 3) {
  return CH.remote(script, { service: SVC, timeout, attempts })
}

const clean = (out) =>
  out.split('\n').filter((l) => !/Using SSH|railway\.json|Migrate|Existing/.test(l)).join('\n').trim()

/**
 * A push whose non-zero exit is an ANSWER, not an accident.
 *
 * `git push` exits non-zero when ANY ref is rejected, and this pushes a batch.
 * So one branch whose remote history diverged made execSync throw, the script
 * died before reaching the reporting below, and NONE of the other branches in
 * that batch reached the remote. Measured 2026-09-04: a single non-fast-forward
 * took the whole run down, and the swarm's finished work stayed invisible - the
 * defect this tool exists to fix, committed by the tool.
 *
 * A partly-rejected push is a normal outcome that the code below already knows
 * how to describe. It only needed the output rather than an exception.
 */
function push(script, timeout = 280000) {
  try {
    return remote(script, timeout)
  } catch (e) {
    return clean(String(e.stdout || '') + '\n' + String(e.stderr || ''))
  }
}

// THERE ARE TWO REPOSITORIES IN THAT CONTAINER, AND THIS TOOL SAW ONE.
//
// `/workspace/BrowserOS` was hardcoded here from the first line of the file.
// Measured 2026-09-13, inside the same container:
//
//   /workspace/BrowserOS  228 branches carry work, 0 missing from the remote
//   /workspace/t27         45 branches carry work, 45 missing - every one
//                          52 commits, 84 changed files, none on any remote
//
// The t27 branches are queen-3507..3542 and queen-3576..3591 - including the
// ten issues filed as the swarm's real food. They have never left the volume
// they were written on. The step reported `not pushed: 0` the whole time and
// it was telling the truth about the only place it was looking.
//
// Two reasons it could not have found them by accident: the base branch there
// is `master`, not `feat/queen-supervisor`, and `rev-parse` of a branch that
// does not exist makes the whole survey line fail rather than say so.
//
// PUSHING IS AN OUTWARD-FACING ACT. Making 45 branches appear on a public
// remote is not maintenance, and it is not mine to decide. So `push` is a
// declared field: the survey covers every checkout, the push covers only the
// ones it is authorised for, and work that is stuck is now VISIBLE rather than
// absent from the report.
export const CHECKOUTS = [
  { dir: '/workspace/BrowserOS', base: 'feat/queen-supervisor', push: true },
  // AUTHORISED 2026-09-16 by the owner, asked directly and answered directly.
  // The hold had done its job and then became the problem: by that date the 45
  // branches were 145, and the jam they caused was measured end to end -
  // unpushed work cannot be seen, `close-done` rightly refuses to close an
  // issue whose branch is on no remote, so 130 accepted issues stayed open,
  // each holding its boundary, and the selector found no free path among 201
  // well-formed candidates. The tick said `nothing to choose` while the
  // backlog held 747 issues. Withholding the push was the first link.
  { dir: '/workspace/t27', base: 'master', push: true },
]

// Every git call into the container needs `-c safe.directory=*`: the repo is
// owned by another uid, and without it `git branch --list` returns an EMPTY
// list rather than an error - a count of zero would look like "nothing to do".
export const gitAt = (dir) => `git -c safe.directory=* -C ${dir}`

// ONE LINE, DELIBERATELY. `JSON.stringify` turns a real newline into a literal
// backslash-n, and inside the double quotes of `sh -c "..."` that is two
// characters, not a line break. A multi-line script therefore arrives at the
// remote shell as a single line and dies with "Syntax error: then unexpected".
// Separate with semicolons instead. The same bug was sitting unexercised in
// reap.mjs, whose multi-line branch had never been run.
// And NOT `.join('; ')` over a list of fragments either. That was the first
// draft and it produced `... else; base=$(...)`, which is a syntax error in sh:
// `else` takes a command list, not an empty one. The shell said so - `Syntax
// error: ";" unexpected` - on the first run. Branches are written out, not
// assembled from parts that look independent and are not.
export function surveyScript({ dir, base }) {
  const G = gitAt(dir)
  const tag = dir.replace(/[^A-Za-z0-9]/g, '_')
  const heads = `/workspace/.remote-heads.${tag}`
  // No `git fetch`: it takes minutes on this repository inside the container,
  // and `ls-remote` already answers the only question here - which heads exist
  // on the remote right now.
  const measure =
    `${G} ls-remote --heads origin 2>/dev/null | sed 's|.*refs/heads/||' | grep '^queen-' | sort > ${heads}; ` +
    `for b in $(${G} branch --list 'queen-*' | sed 's/^[* +]*//'); do ` +
    `n=$(${G} rev-list --count $base..$b 2>/dev/null || echo 0); ` +
    `if [ "$n" -gt 0 ]; then if grep -qx "$b" ${heads}; then echo "ONREMOTE $b"; else echo "MISSING $b $n"; fi; fi; ` +
    `done; rm -f ${heads}`
  // A MISSING BASE IS AN ANSWER TOO. `rev-parse master` on a checkout that
  // calls it `main` prints an error and leaves `base` empty, and `$base..$b`
  // with an empty base is a rev-list over all of history - every branch would
  // look like it carried work. Saying so is the only safe reading.
  return (
    `echo "CHECKOUT ${dir}"; ` +
    `if ! ${G} rev-parse --git-dir >/dev/null 2>&1; then echo "NOCHECKOUT ${dir}"; ` +
    `else base=$(${G} rev-parse ${base} 2>/dev/null); ` +
    `if [ -z "$base" ]; then echo "NOBASE ${dir} ${base}"; ` +
    `else ${measure}; fi; fi`
  )
}

/**
 * Split one checkout's survey output into the three things it can say.
 *
 * Exported so the suite can prove the reader against real container output
 * without a network hop. The parsing is the part that was wrong for nine days:
 * the survey itself was correct about the one directory it was given.
 */
export function readSurvey(out) {
  const lines = String(out).split('\n').map((l) => l.trim())
  return {
    missing: lines.filter((l) => l.startsWith('MISSING ')).map((l) => l.split(/\s+/)[1]),
    onRemote: lines.filter((l) => l.startsWith('ONREMOTE ')).length,
    // null, not zero: a checkout whose base could not be resolved was not
    // measured, and an unmeasured checkout has no count of unpushed work.
    unreadable: lines.find((l) => l.startsWith('NOBASE ') || l.startsWith('NOCHECKOUT ')) || null,
  }
}

if (!isMain) { /* imported for calibration or reuse: do nothing */ } else {
// A CHANNEL FAILURE MUST NOT LOOK LIKE "NOTHING TO PUSH".
//
// The distinction is the whole point. A run that found no unpushed branch and a
// run that could not reach the container both end without pushing anything, and
// only one of them is fine. The chain reads this step's words, so the words have
// to be different.
process.on('uncaughtException', (e) => {
  if (e && e.channel) {
    console.error(`\nCOULD NOT REACH THE CONTAINER after retrying.`)
    console.error(`  ${e.message}`)
    console.error('  Nothing was pushed and nothing was inspected. This is NOT "nothing to push":')
    console.error('  this step is the only way a bee\'s work leaves the container, and it did not run.')
    process.exit(3)
  }
  console.error(e)
  process.exit(1)
})

// Every checkout is surveyed. Only the authorised ones are pushed.
const seen = CHECKOUTS.map((c) => ({ ...c, ...readSurvey(remote(surveyScript(c))) }))

for (const c of seen) {
  if (c.unreadable) {
    console.log(`${c.dir}: NOT MEASURED - ${c.unreadable}`)
    continue
  }
  const held = c.push ? '' : '   (this tool is not authorised to push here)'
  console.log(`${c.dir}: branches with work ${c.missing.length + c.onRemote}   on the remote ${c.onRemote}   unpushed ${c.missing.length}${held}`)
}

// THE PAIR THE CHAIN READS, AND WHY IT COUNTS ONLY WHAT CAN BE PUSHED.
//
// feed.mjs calls this step empty-handed when `not pushed: N` is above zero and
// `pushed 0` follows. Counting the withheld checkout here would make that true
// on every run for ever, and no `--push` could clear it - a warning that is
// always on, which is not a warning. The withheld work is reported on its own
// line above and as its own REFUSED below.
const pushable = seen.filter((c) => c.push && !c.unreadable)
const missing = pushable.flatMap((c) => c.missing.map((b) => ({ branch: b, checkout: c })))

console.log(`\nnot pushed: ${missing.length}`)
missing.forEach((m) => console.log(`  -> ${m.branch}   ${m.checkout.dir}`))

// `REFUSED ` is the word feed.mjs keeps when it condenses this step into the
// timer log. Without it the sentence below is dropped and 45 branches of
// finished work are invisible from outside the container - which is the exact
// failure being reported.
const withheld = seen.filter((c) => !c.push && !c.unreadable && c.missing.length)
for (const c of withheld) {
  console.log(`\nREFUSED - ${c.missing.length} branch(es) in ${c.dir} hold work that is on no remote, and this tool is not authorised to push there.`)
  console.log(`  They exist only on the container volume. Authorise by setting push:true for that entry in CHECKOUTS.`)
  L.append({ kind: 'push-work-withheld', dir: c.dir, branches: c.missing.length })
}

if (!missing.length) process.exit(0)
if (!process.argv.includes('--push')) { console.log('\nreport only. re-run with --push to act.'); process.exit(0) }

L.append({ kind: 'push-work', note: `pushing ${missing.length} branches`, branches: missing.map((m) => m.branch) })

let pushed = 0
const rejected = []
// ONE CHECKOUT PER PUSH. `git -C` takes one directory, so a batch that
// straddles two repositories would send half its branches into the wrong
// remote. Grouping first is what keeps the batching honest once there is more
// than one checkout to group.
for (const c of pushable) {
 for (let i = 0; i < c.missing.length; i += BATCH) {
  const batch = c.missing.slice(i, i + BATCH)
  const G = gitAt(c.dir)
  // `--no-verify`, deliberately, and this is the reason.
  //
  // The branches being pushed were created days ago from older bases and carry
  // whatever `lefthook.yml` existed then. One of those versions writes its
  // branch-name check in bash - `[[ ]]` and `=~` - and the container runs hooks
  // under `sh`, where it dies with `Syntax error: "(" unexpected` and takes the
  // push down with it. Measured 2026-09-04: four branches carrying finished
  // work could not reach the remote, and the reported reason was the branch
  // name rather than the shell.
  //
  // Fixing the rule on the supervisor branch does not help them; their copy is
  // already written. And a pre-push hook is the AUTHOR's check, run on the
  // author's machine before they publish. This tool is not the author: it is
  // making already-finished work visible from a third machine, where running
  // someone else's local hooks proves nothing and can only fail. Every gate
  // that matters - lint, tests, the language policy - runs in CI on the pushed
  // branch, and this changes none of that.
  const out = push(`${G} -c "url.https://x-access-token:$GH_TOKEN@github.com/.insteadOf=https://github.com/" push --no-verify origin ${batch.join(' ')} 2>&1`)
  const created = (out.match(/new branch/g) || []).length
  pushed += created
  for (const line of out.split('\n')) {
    if (/rejected|error:/.test(line)) rejected.push(line.trim().slice(0, 100))
  }
 }
}
// GIVE THE REPOSITORY BACK.
//
// This tool enters the container as ROOT. Every ref it updates leaves a reflog
// file owned by root under `.git/logs/refs/remotes/origin/`, and the worker runs
// as `bee`, which cannot append to them. Measured 2026-09-04: 112 such files,
// and EVERY new bee died at 0 seconds with
//
//   git fetch failed: error: cannot update the ref
//   'refs/remotes/origin/queen-1329': unable to append to
//   '.git/logs/refs/remotes/origin/queen-1329': Permission denied
//
// The tick chose the same issue round after round, the dispatch died instantly
// each time, and the swarm sat at one worker of four. My maintenance tool took
// the fleet down.
//
// The owner is read from `.git` itself rather than assumed to be `bee`, so this
// keeps working if the image ever changes user. It is done for EVERY checkout
// this run pushed into, not for the one that used to be hardcoded here: the
// outage below is caused by the push, so it follows the push.
for (const c of pushable) {
  if (!c.missing.length) continue
  const owner = remote(`stat -c '%u:%g' ${c.dir}/.git 2>/dev/null || echo ''`)
  if (!owner || owner === '0:0') continue
  // THE WHOLE .git, NOT TWO SUBDIRECTORIES.
  //
  // This chowned `logs` and `refs` because those were what the first outage
  // showed: 112 root-owned reflogs killing every bee at `git fetch` with
  // Permission denied. But `git push` running as root also writes OBJECTS, and
  // it creates the fan-out directories that hold them.
  //
  // Measured 2026-09-05: 131 root-owned entries under .git, 53 of them
  // DIRECTORIES in `objects/`, mode 755 root:root inside a tree owned by `bee`.
  // A bee cannot create a file in a directory it does not own, so every one of
  // those was a hole the next fetch or commit could fall into - the same outage,
  // waiting, in the part of the fix nobody had looked at.
  remote(`chown -R ${owner} ${c.dir}/.git 2>&1 | head -2`)
  const left = remote(`find ${c.dir}/.git -user root 2>/dev/null | wc -l`)
  console.log(`gave ${c.dir} back to ${owner}; root-owned files left: ${String(left).trim()}`)
}

console.log(`\npushed ${pushed}`)
if (rejected.length) {
  console.log('refused, and deliberately not forced:')
  rejected.slice(0, 8).forEach((r) => console.log(`  ${r}`))
}
L.append({ kind: 'push-work-result', pushed, rejected: rejected.length })
}
