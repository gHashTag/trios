---
name: tri-cli
description: The `tri` bash CLI itself - where the file lives, how to edit it without corrupting a running timer, and how to tell whether the tracked copy still is the one that runs. Load before editing `tri` or when `tri drift` is red.
allowed-tools: Bash, Read, Edit, Write
---

## What this is for

`tri` is the front end of the improvement loop: 1686 lines of bash, 172 lines of
help, the entry point to some sixty `.mjs` instruments and to three other
projects. This file is about the FILE. What the commands do is `tri help`, and
that is the only place to read it - a hand-copied command list is the defect this
repository finds more often than any other, and it has already produced two
constants for one quantity, three copies of one boundary rule, and a skill (the
neighbouring `tri` skill) that describes a tool nobody runs.

## The two facts that are not in `tri help`

**1. It is executed by absolute path, by timers, while you are editing it.**

```
ai.t27.trios-feed        every 300s   tri feed --act
ai.t27.trios-heal        every 600s   tri heal
ai.t27.trios-cycle       every 900s   tri cycle
ai.t27.trios-cycle-heal               tri cycle-repair
```

On 2026-09-05 a timer read the file MID-WRITE and bash reported a syntax error on
a line that was, and is, correct: `open(path, 'w')` truncates before it writes,
and the timer saw the truncated half. The file's own header records this.

**So never edit it in place.** Write a sibling temp file in the same directory
and rename it - a rename is atomic, a write is not:

```bash
cp -p ~/.local/bin/tri ~/.local/bin/.tri.new
# edit ~/.local/bin/.tri.new
bash -n ~/.local/bin/.tri.new && mv ~/.local/bin/.tri.new ~/.local/bin/tri
```

Use a SIBLING, not `/tmp`. `mv` is only atomic within one filesystem; across
filesystems it degrades to copy-then-unlink, which is the truncating write again
under a safer-looking name. On this machine both happen to be `/dev/disk3s1`
(`stat -f %d` gives 16777229 for each), so `/tmp` would have worked here - which
is exactly why it is the wrong habit: it works until the machine changes and then
fails on the one file no timer can afford to read half of.

**2. Until 2026-09-13 it existed in exactly one place and in no repository.**

There is now a tracked copy at `trios/bin/tri`, and a gate that says whether it
is still the same file:

```bash
tri drift              # 0 same, 1 measured difference, 2 nothing could be read
tri drift --adopt      # copy live -> tracked, BY HAND, after reading the diff
```

`tri drift` reads the DECLARATIONS - every plist matching
`ai.t27.trios-*.plist` under `~/Library/LaunchAgents` (outside this tree, which
is why `tri proof` cannot resolve it), converted by `plutil`, ProgramArguments
only - not `which tri`. The shell's PATH and launchd's
PATH are different, and the question is which file the timers execute, not which
file the word would mean if you typed it. The shell's answer is read too, as a
separate row: if the two disagree, that disagreement is the finding.

Exit 2 is not a pass. An absent measurement is not agreement.

## Adding a command

A shell `case` takes the FIRST match. `tri feed --act` ran on a timer for weeks
while reaching an unrelated `feed` arm that fetched a social feed - a working
command answering the wrong question, which is worse than a broken one because
nothing reports it.

1. Resolve the new word against every arm above the insertion point.
2. Add it to `ARMS` in `.trinity/loop/cycle-doctor.mjs`. That map is what keeps a
   working word working: the doctor resolves each listed word against the real
   file the way bash would and fails if an earlier arm answers.
3. Add the help line next to the arm it documents, in the same edit.
4. `node .trinity/loop/cycle-doctor.mjs` - the "tri arms" check must pass.
5. If the arm calls a new `.mjs`, declare it in `.trinity/loop/coverage.mjs`
   with the ledger kind its act path writes, or with an explicit note saying it
   never acts. An undeclared instrument reports UNTRACKED and fails the run.

## After editing

```bash
bash -n ~/.local/bin/tri                      # the timers get no second chance
tri drift                                     # then --adopt, deliberately
node .trinity/loop/cycle-doctor.mjs           # 18 checks
node .trinity/loop/selftest.mjs               # 284 checks, includes the drift gate
```

The selftest gate goes red when the tracked copy stops matching. That is the
point of it: proven by moving `trios/bin/tri` aside and watching it fail, not by
reading the assertion.
