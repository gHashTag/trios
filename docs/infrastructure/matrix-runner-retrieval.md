# Matrix Runner JSONL Retrieval Runbook

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

This runbook describes how the `trios-mr-priority-runner` Railway service
publishes its training output back to `gHashTag/trios:assertions/matrix_samples.jsonl`.
Origin lane: issue [#598](https://github.com/gHashTag/trios/issues/598). Parent
matrix: [#446](https://github.com/gHashTag/trios/issues/446). Throne spark:
[#264 comment-4408024957](https://github.com/gHashTag/trios/issues/264#issuecomment-4408024957).

## Why this exists

`scripts/run_priority_matrix.ts` (PR [#589](https://github.com/gHashTag/trios/pull/589))
appends one row per training run to `assertions/matrix_samples.jsonl` inside the
runner container. The runner does not push back to GitHub. Without retrieval, all
output is lost when the Railway service shuts down at end-of-job (~12.5 h after start
for the canonical 50 cells × 3 seeds × 3000 steps workload).

## How retrieval works

The retrieval lane has three components, ranked by deployment preference:

1. **Sidecar (preferred):** `scripts/postrun_sidecar.ts` runs as a separate Railway
   service that mounts the same volume as the runner. It wakes every 30 min
   (configurable), invokes `scripts/postrun_commit_back.ts`, and exits gracefully
   on `SIGTERM` after one final flush.

2. **Scheduled workflow (fallback):** `.github/workflows/matrix-runner-postrun.yml`
   runs hourly in GitHub Actions. It expects the runner — or a small auxiliary
   process on the runner side — to have pushed the current JSONL to a sibling
   branch named `data/matrix-runner-staging` (configurable). The workflow
   then invokes `scripts/postrun_commit_back.ts` with that branch's contents.

3. **Manual run (last resort):** an operator can `scp` the JSONL out of the
   runner volume to a local checkout of `gHashTag/trios` and execute
   `GITHUB_TOKEN=<pat> npx tsx scripts/postrun_commit_back.ts`.

In all three modes the orchestrator (`postrun_commit_back.ts`) is identical and
idempotent.

## Row identity and dedup

A row's identity is the SHA-256 of the canonical JSON of
`{format, algo, seed_phi, step, bpb}`. The orchestrator:

1. Reads the local JSONL.
2. Fetches `assertions/matrix_samples.jsonl` from the base branch (default `main`)
   via the GitHub API.
3. Computes hashes for both sets.
4. Subtracts the remote set from the local set.
5. Splits the remainder into batches of `BATCH_SIZE` (default 25).
6. For each batch, creates `data/matrix-runner-<UTC-timestamp>-batch-<N>`,
   writes the batch via the GitHub Contents API, and opens a pull request.

Hash includes only the five numerical/identity fields, so re-runs that produce
identical numerical outputs are not double-committed even if the runner emits a
fresh `timestamp`/`sha`/`source` on every retry.

## Branch and PR conventions

- Branch name: `data/matrix-runner-<timestamp>-batch-<N>` where `<timestamp>` is
  ISO-8601 UTC with `:`/`.` replaced by `-`.
- Commit message: `data(matrix): commit-back batch N/M (K rows) from runner SHA <pinned> · phi^2+phi^-2=3`.
- PR title: `data(matrix): commit-back batch N/M from runner SHA <pinned>`.
- PR body lists each row's hash prefix (16 hex chars) and `(format, algo, seed_phi, step, bpb)`
  for traceability.
- Author: `Dmitrii Vasilev <admin@t27.ai>`.
- The orchestrator never `--admin` merges; queen merges manually.

## Deploying the sidecar on Railway

Required: a Railway project with both the runner and the sidecar in the same
environment, sharing a volume that contains `assertions/matrix_samples.jsonl`.

1. Create a new Railway service in the same project as `trios-mr-priority-runner`
   (project IGLA, runner service id `71f5aac2-d4d5-4640-8895-90ced5d4ea63`).
2. Source: this repo (`gHashTag/trios`), branch `main`.
3. Build command: `npm i -g tsx@4`.
4. Start command: `tsx scripts/postrun_sidecar.ts`.
5. Volume mount: identical to the runner's mount path so the sidecar can read
   `assertions/matrix_samples.jsonl` written by the runner.
6. Environment variables:
   - `GITHUB_TOKEN` — fine-grained PAT with `contents:write` and `pull-requests:write`
     on `gHashTag/trios`.
   - `INTERVAL_MIN` — sleep between iterations, default 30.
   - `RUNNER_SHA` — the image SHA pin the runner uses (default `6cf0b5bd`); appears
     in commit messages.
   - `BATCH_SIZE` — rows per PR, default 25.
7. Deploy. The sidecar will immediately invoke the orchestrator once, then sleep.

## Deploying the scheduled workflow

The workflow ships in `.github/workflows/matrix-runner-postrun.yml`. It runs hourly
at `:23` and on `workflow_dispatch`. To use it:

1. Ensure the runner (or a small Railway auxiliary process) periodically pushes the
   current `assertions/matrix_samples.jsonl` to the staging branch
   `data/matrix-runner-staging`. A minimal pattern is `git push origin <local>:data/matrix-runner-staging --force`
   (force-push is acceptable here because the staging branch is a **work-in-progress
   buffer**, never merged directly into `main`).
2. The workflow then sees the staging branch, copies the JSONL into its checkout,
   and runs the orchestrator. Output PRs are visible in the trios PR list as usual.

## Troubleshooting

### No new rows committed

Check that the local JSONL actually contains rows. The schema header line
(starting `{"_schema":"trios.assertions.matrix_samples.v1"…`) is deliberately
ignored — only objects with `format`, `algo`, `seed_phi`, `step`, `bpb` count.

### Hash mismatch — same row appears in multiple PRs

This should not happen because the orchestrator dedupes against `main` before each
batch. If it does, check that the previous batch's PR was actually merged into
`main` before the next iteration ran. If the previous PR is still open, the next
iteration will re-emit the same rows on a fresh branch. Resolution: merge the
oldest open `data/matrix-runner-*` PR before re-running.

### Partial-batch recovery

If the orchestrator dies mid-batch (network blip, GitHub 500), the partial branch
will exist but the PR may not. Re-running the orchestrator notices the rows are
still absent from `main` and starts a fresh batch with a new timestamp. The dead
branch can be deleted manually; nothing in `main` was modified.

### Forbidden file extension

Per repo law (`CLAUDE.md` L1), `.sh` files are banned. All scripts must be Rust or
TypeScript. The orchestrator and sidecar are TypeScript executed via `npx tsx`.

## End-of-job summary

The orchestrator posts a heartbeat comment on parent issue #446 after each
multi-batch run. The comment includes:

- batches opened
- rows committed
- list of PRs opened (with #-numbers)
- runtime (implicitly, via comment timestamp)
- anchor footer

For non-convergent cells (rows present in JSONL with `bpb` exceeding the calibrated
ceiling for that format/algo), the orchestrator does **not** filter them out — the
JSONL is the honest record of what the runner observed. Filtering happens
downstream in the matrix-bot (`.github/scripts/matrix_bot.py`) and the closure
gate.

## R5-honest notes

- The orchestrator never fabricates rows. If the local JSONL is empty, no PRs are
  opened.
- The orchestrator never modifies existing rows. Append-only is preserved both
  locally and on the remote.
- The orchestrator preserves the schema header line of the remote file (PUTs the
  remote's existing content + appended new rows + trailing newline).
- The orchestrator does not block on PR merge. PR-per-batch is opened and the
  orchestrator moves on; queen merges asynchronously.

## See also

- `scripts/postrun_commit_back.ts` — the orchestrator itself.
- `scripts/postrun_sidecar.ts` — the long-running daemon.
- `.github/workflows/matrix-runner-postrun.yml` — the scheduled fallback.
- `scripts/run_priority_matrix.ts` (PR #589) — the runner that produces the JSONL.
- `assertions/matrix_samples.jsonl` — the canonical destination file (schema
  header at line 1; sample rows beyond).
- Issue [#598](https://github.com/gHashTag/trios/issues/598) — this lane.
- Issue [#446](https://github.com/gHashTag/trios/issues/446) — parent matrix.

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
