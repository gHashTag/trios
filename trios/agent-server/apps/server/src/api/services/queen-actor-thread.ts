/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The entry of a worker thread that runs one isolated turn's work
 * (queen-actors-isolate.ts threadWork). It runs one call, posts the result,
 * and is terminated by its parent.
 */

const port = globalThis as unknown as {
  onmessage: ((event: MessageEvent) => void) | null
  postMessage: (value: unknown) => void
}

port.onmessage = async (event: MessageEvent) => {
  const { module, fn, arg } = event.data as {
    module: string
    fn: string
    arg: unknown
  }
  try {
    const loaded = (await import(module)) as Record<string, unknown>
    const call = loaded[fn]
    if (typeof call !== 'function')
      throw new Error(`${module} exports no ${fn}`)
    const value = await (call as (a: unknown) => unknown)(arg)
    port.postMessage({ ok: true, value })
  } catch (error) {
    port.postMessage({
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}
