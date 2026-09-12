# The cron contract

What the scheduled cycle is allowed to do, why each limit exists, and how to
check that the limits still hold.

The short version: **iterations 1-96 of this loop produced files that are still
read. The new timer must be able to run a thousand times without any of them
changing.** Everything below is that sentence made checkable.

---

## The two jobs

| launchd label | Command | Interval | Mutates |
|---|---|---|---|
| `ai.t27.trios-cycle` | `tri cycle` | 900 s | nothing but its own files |
| `ai.t27.trios-cycle-heal` | `tri cycle-repair` | 3600 s | the above, plus two enumerated repairs |

They are two jobs and not one on purpose. If the only timer were a repairing
one, every fifteen minutes would be a chance to mutate the tree and nobody
reading the dashboard would know which of the two kinds of run they were
looking at. One job observes; one job heals; the ledger records the mode of
every run.

`RunAtLoad` is true for the observer and false for the healer. Login is the
worst moment to decide that a process is dead.

---

## Why launchd and not a Claude scheduled task

Because the last driver was one.

`23d6fe89` was a session-scoped scheduled task. It died when its session ended
on 2026-09-06. `scheduled_tasks.json` then held zero tasks, the loop sat still
for **147.9 hours**, and for all of those hours:

- two other launchd timers kept firing and doing no-op work,
- `DASHBOARD.txt` kept naming the dead job as the thing driving the loop,
- nothing said otherwise.

A beat whose liveness depends on a chat window being open is not a beat. The
schedule now lives in `~/Library/LaunchAgents`, survives logout and reboot, and
is verified by `tri cycle-doctor` in the only way that means anything - by
reading what the loaded jobs actually *run*, not what they are *called*.

> That distinction is not academic. The first version of that check ran
> `launchctl list | grep -i trios`, matched `trios-feed`, `trios-heal`,
> `com.trios.backup` and `com.trios.health`, and reported "scheduled by 4
> launchd jobs" - a pass. All four were loaded throughout the outage. The check
> would have been green for every hour of the thing it exists to detect.

---

## The six clauses

Stated in `cycle.mjs`'s header, enforced by `cycle-doctor.mjs`.

### 1. It never mutates git

No `commit`, `push`, `checkout`, `reset`, `merge`, `rebase`, `stash`, `clean`,
`branch -D`. The cycle reads the tree's state and reports it. A timer that can
commit is a timer that can commit something half-written at 04:00.

*Checked:* `contract: no git mutation` scans the string literals of `cycle.mjs`
for a git mutator. Comments and prose are excluded by a character scanner, not
a regex - three false positives were caused by the regex version reading
backticked prose in a comment as a template literal.

### 2. It never writes the previous loop's files

`state.json`, `DASHBOARD.txt` and `ledger.jsonl` belong to iterations 1-96. The
cycle reads them and writes its own: `DASHBOARD2.ansi`, `DASHBOARD2.txt`,
`cycle-ledger.jsonl`, `state/cycle-readings.jsonl`, `cycle-notes.json`.

This is the clause that answers "will the new cron break the old work". It
cannot: the old work's files are not in its write set, and the gate fails if a
future edit puts one there.

*Checked:* `contract: old loop untouched`, left-anchored so that
`cycle-ledger.jsonl` is not mistaken for `ledger.jsonl`.

### 3. It respects the cooperative lock

`loop.lock` holds `{holder, pid, at}`. If another instrument holds it, the
cycle logs one line and exits 0. It does not queue, retry or wait.

The very first launchd-driven run did exactly this - `heal` held the lock, the
cycle stepped aside - which is the only proof of the clause worth having.

### 4. It uses the loop's own lock rule, not a second one

Staleness is decided by `isStale` in `loop.mjs`, **imported** - the same
function `heal` and `feed` obey.

This clause was rewritten after the first version broke it. The cycle had its
own `STALE_LOCK_MIN = 45` and its own `pidAlive`, and two things followed:

- It wrote a lock record without `singleProcess: true`. `loop.mjs:isStale` only
  applies the pid test when the holder declares that field, so to `heal` and
  `feed` the cycle's lock looked merely *old* - not dead, not stealable. A
  90-second cycle would have frozen `heal` for four fires and `feed` for nine.
- Its private `pidAlive` answered the **opposite** of `loop.mjs`'s for a lock
  with no pid field: mine said dead, the loop's said alive. Two instruments, two
  verdicts, one lock file.

Two locking rules in one directory is not belt and braces; it is two rules that
agree until someone edits one - the same argument L0 makes about four copies of
a spec. There is now one, and the cycle also releases on `SIGTERM` and `SIGINT`
rather than only in a `finally` block, because `launchctl bootout` sends
`SIGTERM` and a `finally` never runs.

Stealing happens **only under `--repair`**.

### 5. Repairs are enumerated, there are two, and neither is implemented here

| Repair | Fires when | Delegates to |
|---|---|---|
| `lock-held-by-dead-pid` | `isStale(lock)` in `loop.mjs` says so | - |
| `volume-near-full` | disk >= `DISK_REPAIR_AT` (88%) | `reap-local.mjs --reap` |

**The first version of the second repair was the most dangerous code in this
work, and it read as the most careful.** It removed any worktree that was clean,
ran `git worktree remove` without `--force`, and carried a comment explaining
that git's refusal to remove a *dirty* tree was the entire safety property. That
is true and it is beside the point. git does not refuse a **clean** worktree
whose branch was never landed.

Measured on 2026-09-12, with the disk at 92% and the gate therefore open:

```
18 worktree(s): 0 merged and clean, holding 0 MB
```

`reap-local.mjs` found **zero** removable. The old repair would have taken about
thirteen, holding 1, 2, 3, 6, 7 and 10 commits whose subjects appear nowhere in
`origin/feat/queen-supervisor` - roughly 36 commits of bee work existing in no
other checkout, seven of them in `~/Documents/Codex/...` and belonging to another
agent's sessions, which the old code never asked about.

So the repair now shells out to `reap-local.mjs` (written 2026-09-06 for exactly
this job), which asks all five questions: inside the repo, readable, clean,
merged **by ancestry or by squash-subject** - a squashed branch looks unmerged
for ever - and not holding deletions git would need `--force` to discard. One
rule, one place.

*Proven, not asserted:* `tri cycle-repair` was run by hand with the gate open.
19 worktrees before, 19 after, and the ledger records
`"mode":"repair","repairs":[]`.

*Checked:* `contract: no --force`, narrowed on the same day to fire only on a
`-f` sitting in the same string literal as a destructive git verb. The first
version failed on `sh('pgrep -f ' + ...)` - pgrep's `-f` means "match the full
command line" and destroys nothing. Three self-test cases pin the narrowing:
pgrep's flag passes, a `-f` and a git verb in two *separate* literals pass, and
`git clean -f` still fails.

### 6. It is idempotent and bounded

Running it twice in a row changes nothing the second time. Every subprocess
carries a timeout. The deep pass (`tri idle`, which walks the swarm's whole
round history and can take two minutes) runs on every fourth cycle rather than
every cycle, so the hourly cost is one deep pass, not four.

After any repair, the cycle **re-measures** before drawing. The dashboard shows
the world the cycle left, not the one it found.

---

## What the cycle does NOT do, deliberately

- **It does not write to GitHub.** `backlog.mjs` classifies 590 open issues and
  writes proposals to `state/boundary-proposals/`; a human applies them. A
  triage tool that edits issue bodies on a timer is precisely how a standing
  instruction not to touch the old backlog gets forgotten.
- **It does not restart the swarm, redeploy, or touch Railway.**
- **It does not edit source.** It reports; the repairs are to the loop's own
  runtime state, not to code.

---

## How to check it in ten seconds

```bash
tri cycle-doctor --selftest
```

31 checks. Seventeen are self-tests, and they exist because a gate that has
never been shown to fail is not evidence. They feed the gate deliberate
violations (`git commit`, a `--force`, `git clean -f`, a write to
`ledger.jsonl`) and deliberate near-misses (a trailing comment, backticked
prose, a URL, `cycle-ledger.jsonl`, `pgrep -f`, `tri feed --act`, the word
`recycle`) and assert the gate's verdict on each.

Every near-miss in that list is a false positive this gate actually produced.
Four of them were the same defect - **an instrument reading prose, or a token
without its command, as evidence** - which is now the most frequent defect class
in this repository, found previously in a SQL gate, a claim guard, a boundary
rule, and a launchd check that matched jobs by *name*.

Exit 0 when all pass, 2 when any fails.

```bash
tri cycle-log 12     # the last dozen runs: mode, anomalies, vitals, repairs
tri cycle-dash       # the dashboard, with its age printed first
```

The age comes first and comes from the filesystem, not from the file's own
header. Reading a dashboard without its age is how six days went unnoticed.

---

## If you need to stop it

```bash
launchctl bootout gui/$UID/ai.t27.trios-cycle
launchctl bootout gui/$UID/ai.t27.trios-cycle-heal
```

Nothing else depends on these two jobs. The pre-existing `trios-feed` and
`trios-heal` timers are untouched by this work and keep their own schedules.
