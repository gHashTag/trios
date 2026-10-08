/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * CPU work for the actor tests (actors.t27 section 8): a turn that burns a
 * thread for a while, and one that never stops and counts in shared memory,
 * so a test can see whether a kill really stopped it.
 */

/** Burn the calling thread for `ms`; returns how many rounds it ran. */
export function burn(ms: number): number {
  const until = performance.now() + ms
  let rounds = 0
  while (performance.now() < until) rounds++
  return rounds
}

/** Count in `shared[0]` for ever. Only a terminate ends it. */
export function spin(shared: SharedArrayBuffer): number {
  const counter = new Int32Array(shared)
  for (;;) Atomics.add(counter, 0, 1)
}
