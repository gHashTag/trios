/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Where an isolated turn's work runs (actors.t27 section 8, t27#7900).
 * An OS thread, for CPU-bound work of this build. An OS process, for code a
 * bee wrote or a compiler on its work. The OS preempts both, so neither holds
 * the host's thread. Only the process is certain to stop: `stop` on a thread
 * is `worker.terminate()`, and a terminated worker thread that spins keeps
 * running (measured on Bun 1.3.11, still counting 200 ms after terminate), so
 * the card counts a thread kill as an abandon (kill_effect). The card decides
 * which one a turn gets (turn_isolation).
 */

import type { IsolatedWork } from './queen-actors'

const THREAD_ENTRY = new URL('./queen-actor-thread.ts', import.meta.url).href

/**
 * Run `fn` exported from `module` in a fresh worker thread. The thread is
 * terminated when the work ends or is stopped, so one turn never inherits
 * another's state. A terminate is not certain to stop it (see above).
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
 *
 * THE CHILD LEADS ITS OWN PROCESS GROUP (`detached`), and every signal goes
 * to the group. A kill of the one pid spawned here used to leave that
 * process's own children running: `sh -c 'sleep 30 & wait'` killed with
 * SIGKILL to the shell left its sleep alive, holding whatever the turn held
 * (turn_stop.t27 escalation, ESC_KILL_GROUP). The turn's abort (`signal`) is
 * SIGTERM to the group, so the work may clean up; `stop`, the escalation, is
 * SIGKILL to the group.
 */
export function processWork(
  argv: string[],
  cwd?: string,
  signal?: AbortSignal,
): IsolatedWork {
  const proc = Bun.spawn(argv, {
    cwd,
    stdout: 'pipe',
    stderr: 'pipe',
    detached: true,
  })
  const group = (sig: 'SIGTERM' | 'SIGKILL') => {
    try {
      process.kill(-proc.pid, sig)
    } catch {
      // the group is gone: every member has exited
    }
  }
  const onAbort = () => group('SIGTERM')
  signal?.addEventListener('abort', onAbort, { once: true })
  if (signal?.aborted) onAbort()
  const result = (async () => {
    try {
      const [out, err, code] = await Promise.all([
        new Response(proc.stdout).text(),
        new Response(proc.stderr).text(),
        proc.exited,
      ])
      if (code !== 0) throw new Error(`exit ${code}: ${err.slice(0, 200)}`)
      return out
    } finally {
      signal?.removeEventListener('abort', onAbort)
    }
  })()
  return { result, stop: () => group('SIGKILL') }
}
