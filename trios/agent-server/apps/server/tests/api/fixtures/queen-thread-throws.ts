/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Thread work that fails outside its call (actors.t27 section 8): the call
 * never settles, and a timer throws in the thread. The thread entry cannot
 * catch it, so the worker's error event is the only report.
 */

export function throwLater(message: string): Promise<never> {
  setTimeout(() => {
    throw new Error(message)
  }, 0)
  return new Promise<never>(() => {})
}
