/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Where an isolated turn's work runs (actors.t27 section 8, t27#7900).
 * An OS thread, for CPU-bound work of this build. An OS process, for code a
 * bee wrote or a compiler on its work. The OS preempts both, and `stop` really
 * ends the work. The card decides which one a turn gets (turn_isolation).
 */

import type { IsolatedWork } from './queen-actors'

const THREAD_ENTRY = new URL('./queen-actor-thread.ts', import.meta.url).href

/**
 * Run `fn` exported from `module` in a fresh worker thread. The thread is
 * terminated when the work ends or is stopped, so one turn never inherits
 * another's state.
 */
export function threadWork(
  module: string,
  fn: string,
  arg: unknown,
): IsolatedWork {
  const worker = new Worker(THREAD_ENTRY)
  let done = false
  const result = new Promise<unknown>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent) => {
      done = true
      worker.terminate()
      const data = event.data as {
        ok: boolean
        value?: unknown
        error?: string
      }
      if (data.ok) resolve(data.value)
      else reject(new Error(data.error ?? 'thread work failed'))
    }
    worker.onerror = (event) => {
      done = true
      worker.terminate()
      reject(new Error(event.message))
    }
  })
  worker.postMessage({ module, fn, arg })
  return {
    result,
    stop: () => {
      if (!done) worker.terminate()
      done = true
    },
  }
}

/**
 * Run `argv` as a child process. Its stdout is the result when it exits 0.
 * `stop` sends SIGKILL.
 */
export function processWork(argv: string[], cwd?: string): IsolatedWork {
  const proc = Bun.spawn(argv, { cwd, stdout: 'pipe', stderr: 'pipe' })
  const result = (async () => {
    const [out, err, code] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ])
    if (code !== 0) throw new Error(`exit ${code}: ${err.slice(0, 200)}`)
    return out
  })()
  return { result, stop: () => proc.kill(9) }
}
