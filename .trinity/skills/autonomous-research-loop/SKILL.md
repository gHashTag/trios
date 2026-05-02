---
name: autonomous-research-loop
description: >-
  Execute long-running autonomous research tasks without stopping after each step.
  Use when the user says "work autonomously", "don't stop", "work until done",
  "I'm going to sleep", "finish everything", "run to completion", or any variant
  indicating they want uninterrupted multi-step execution. Implements a timer-based
  loop that forces continuation after each phase until all TODO items are completed.
  Covers: research, coding, testing, git commits, document creation, and any
  multi-phase project work. Never ask for confirmation mid-loop — just execute.
metadata:
  author: gHashTag
  version: '1.0'
---

# Autonomous Research Loop

## When to Use This Skill

Use when the user explicitly requests autonomous, uninterrupted work:
- "work until done"
- "don't stop until finished"
- "I'm going to sleep, finish this"
- "autonomous mode"
- "keep going without asking"
- "run the full loop"
- Any indication they want hands-off execution to completion

## Core Principle

**NEVER STOP AFTER ONE ITERATION.** The biggest failure mode is completing one step and then presenting results and waiting. Instead, immediately proceed to the next step. Only stop when ALL tasks in the TODO list are completed or when a blocking error requires human input.

## Protocol

### 1. Create Exhaustive TODO List

At the start, create a detailed TODO list with ALL tasks needed to reach the goal. Be specific — each task should be independently verifiable.

### 2. Execute Loop

For each task in the TODO list:

```
LOOP:
  1. Mark task as in_progress
  2. Execute the task (research, code, test, commit, etc.)
  3. Mark task as completed
  4. Check: are there remaining tasks?
     YES → immediately start next task (DO NOT present intermediate results)
     NO → present final summary
```

### 3. Anti-Sleep Rules

These rules prevent the agent from stopping prematurely:

- **NEVER use submit_answer until ALL tasks are done**
- **NEVER ask "should I continue?"** — the answer is always YES
- **NEVER present intermediate results** — just keep working
- **If a task fails, log the failure and move to the next task** — don't stop to ask
- **If you need information from the user, skip that task and continue with others** — come back to blocked tasks at the end
- **Each tool call should immediately be followed by the next action** — no idle pauses

### 4. Timer Mechanism

After completing each major phase, check progress:

```python
# Pseudo-logic for continuation
completed = count(tasks where status == "completed")
total = count(all tasks)
if completed < total:
    # DO NOT STOP — proceed to next task immediately
    continue_working()
else:
    # ALL DONE — now present final summary
    present_results()
```

### 5. Git Commit Strategy

- Commit after each logical group of changes (not after each file)
- Use descriptive commit messages
- Push to remote after each commit
- Don't wait for PR approval — keep working on the next task

### 6. Error Handling

- **Recoverable error**: Log it, try alternative approach, continue
- **Non-recoverable error**: Log it, mark task as blocked, continue with next task
- **Rate limit**: Wait briefly, retry, continue
- **Never stop the loop for an error unless literally every remaining task depends on the failed one**

### 7. Final Summary

Only after ALL tasks are completed (or all remaining are blocked), present a single comprehensive summary:
- What was accomplished
- What tests were run and their results
- What was committed and where
- What tasks remain blocked (if any)
- What the user should review

## Example Workflow

```
User: "Implement the full research plan. I'm going to sleep."

Agent:
1. Create TODO with 15 tasks
2. Task 1: Clone repo → done
3. Task 2: Create spec file A → done
4. Task 3: Create spec file B → done
5. Task 4: Run tests → done
6. Task 5: Fix test failures → done
7. Task 6: Create conformance JSON → done
8. Task 7: Git commit → done
9. Task 8: Research paper X → done
10. Task 9: Write analysis script → done
11. Task 10: Run analysis → done
12. Task 11: Create findings doc → done
13. Task 12: Create conjecture spec → done
14. Task 13: Git commit → done
15. Task 14: Create PR → done
16. Task 15: Write summary → done

[Only NOW present results to user]
```

## Common Mistakes to Avoid

1. **Stopping after first commit** — This is the #1 failure. Keep going.
2. **Asking "shall I continue?"** — Never ask. Just do it.
3. **Presenting intermediate findings** — Save it for the end.
4. **Waiting for user response** — They're asleep. Keep working.
5. **Getting stuck on one task** — Skip it, come back later.
6. **Making the TODO list too vague** — Be specific: "create file X with content Y" not "work on module"
