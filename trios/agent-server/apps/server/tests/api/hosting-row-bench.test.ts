/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE SANDBOX'S COST, MEASURED (trios#1762; numbers on t27#7851). The owner's
 * rule: a mechanism replaces another only after both run the same input and
 * the numbers are posted. `before` runs a half bare, as slice 1 did; `after`
 * runs the same half of the same spec in this machine's sandbox
 * (hosting-sandbox.ts), through createT27cRunner exactly as `trios-host join`
 * does. Both use one warm zig cache: each spec is run once first, unmeasured,
 * and the rounds then alternate bare and sandboxed so the machine's load falls
 * on both alike.
 *
 * Off unless HOSTING_BENCH=on with T27C, T27B, ZIG, T27_REPO (a t27 clone) and
 * T27_COMMIT. HOSTING_BENCH_SPECS picks the specs (comma-separated),
 * HOSTING_BENCH_ROUNDS the rounds, HOSTING_BENCH_OUT a JSON file for the
 * numbers. The asserts check only that both ways give one output hash.
 */

import { describe, expect, it } from 'bun:test'
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { createT27cRunner } from '../../src/api/services/hosting-agent'
import { detectSandbox } from '../../src/api/services/hosting-sandbox'
import {
  fileSource,
  shardJobOf,
} from '../../src/api/services/hosting-toolchain'
import { sha256Hex } from '../../src/api/services/hosting-wire'
import { SANDBOX_NONE } from '../../src/api/services/queen-hosting-host-card.gen'
import { HALF_WORDS } from '../../src/api/services/queen-hosting-row-card.gen'

const env = process.env
const ON =
  env.HOSTING_BENCH === 'on' &&
  [env.T27C, env.T27B, env.ZIG, env.T27_REPO].every(
    (p) => !!p && existsSync(p),
  ) &&
  !!env.T27_COMMIT
const SPECS = (env.HOSTING_BENCH_SPECS ?? 'specs/a/b_c.t27')
  .split(',')
  .filter(Boolean)
const ROUNDS = Number(env.HOSTING_BENCH_ROUNDS ?? 3)

const median = (xs: number[]) => {
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.floor(s.length / 2)] ?? 0
}

describe('the sandbox, before and after', () => {
  it.if(ON)(
    'runs each half bare and in the sandbox on the same spec, to one output hash',
    async () => {
      const sandbox = detectSandbox()
      expect(sandbox).not.toBe(SANDBOX_NONE)
      const work = mkdtempSync(join(tmpdir(), 'trios-host-bench-'))
      const runner = (box: number) =>
        createT27cRunner({
          t27c: env.T27C as string,
          t27b: env.T27B as string,
          t27bHash: sha256Hex(readFileSync(env.T27B as string)),
          zig: env.ZIG as string,
          workRoot: join(work, 'work'),
          cacheDir: join(work, 'cache'),
          fetchFile: fileSource(env.T27_REPO),
          modelHash: sha256Hex(readFileSync(env.T27C as string)),
          zigVersion: '0.16.0',
          sandbox: box,
        })
      const rows: Array<Record<string, unknown>> = []
      try {
        for (const spec of SPECS)
          for (const half of [0, 1]) {
            const job = {
              id: 'bench',
              model_hash: null,
              ...shardJobOf(
                env.T27_REPO as string,
                env.T27_COMMIT as string,
                spec,
                { holdsSecret: false, holdsPersonal: false },
                half,
              ),
            }
            const signal = new AbortController().signal
            await runner(sandbox)(job, signal) // warm the cache, unmeasured
            const bare: Array<{ wall: number; cpu: number }> = []
            const boxed: Array<{ wall: number; cpu: number }> = []
            const hashes = new Set<string>()
            for (let r = 0; r < ROUNDS; r++)
              for (const [box, into] of [
                [SANDBOX_NONE, bare],
                [sandbox, boxed],
              ] as const) {
                const run = await runner(box)(job, signal)
                hashes.add(run.result.outputHash)
                into.push({
                  wall: run.wallMs ?? 0,
                  cpu: (run.cpuMicros ?? 0) / 1000,
                })
              }
            expect(hashes.size).toBe(1)
            const row = {
              spec,
              half: HALF_WORDS[half],
              bare_wall_ms: median(bare.map((x) => x.wall)),
              sandbox_wall_ms: median(boxed.map((x) => x.wall)),
              bare_cpu_ms: median(bare.map((x) => x.cpu)),
              sandbox_cpu_ms: median(boxed.map((x) => x.cpu)),
              rounds: { bare, sandbox: boxed },
            }
            rows.push(row)
            console.log(
              `${spec} ${row.half}: wall ${row.bare_wall_ms} -> ${row.sandbox_wall_ms} ms, cpu ${row.bare_cpu_ms.toFixed(0)} -> ${row.sandbox_cpu_ms.toFixed(0)} ms`,
            )
          }
        if (env.HOSTING_BENCH_OUT)
          writeFileSync(env.HOSTING_BENCH_OUT, JSON.stringify(rows, null, 1))
      } finally {
        rmSync(work, { recursive: true, force: true })
      }
    },
    3_600_000,
  )
})
