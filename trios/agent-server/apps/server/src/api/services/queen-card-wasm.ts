/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A t27 card compiled to wasm, called from here: `t27c gen-c` of the card,
 * then `zig cc --target=wasm32-wasi -Oz -DNDEBUG -mexec-model=reactor` with one
 * `--export` per function (the command and the hashes are in specs/PIN).
 *
 * A card imports nothing, so it can reach no file, clock or network. It is the
 * decision and nothing else; this file only carries bytes in and numbers out.
 * Text goes in through the module's own memory, written past the end of the
 * memory the module was built with, so nothing the generated code uses is
 * overwritten.
 */

import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { DEFAULT_SPECS_ROOT } from '../../inngest/spec-catalog'

export interface CardWasm {
  /** Call an exported function with numbers (bools as 0/1). */
  call(name: string, ...args: number[]): number
  /** Call one that takes or returns a u64: those cross as BigInt. */
  call64(name: string, ...args: Array<number | bigint>): number | bigint
  /** Write texts into scratch memory; returns each one's address and length. */
  put(...texts: string[]): Array<{ at: number; len: number }>
  /** Room for an output buffer of `bytes` after the last put; returns its address. */
  room(bytes: number): number
  /** Read bytes the card wrote. */
  read(at: number, len: number): string
}

const cache = new Map<string, CardWasm>()

/**
 * THE DECISION LOG'S TAP (specs/queen/telemetry.t27 section 9). With a tap
 * set, a call into one of its cards may be recorded. Per function and window
 * the tap is asked while the card's `kept` still says yes, and not again
 * until the window turns: `kept` never says yes again once it said no in a
 * window (a test in the spec). Calls the tap makes itself are not recorded.
 * With no tap, a call costs one check more than before.
 */
export interface CardTap {
  /** The card files whose calls are recorded. */
  files: ReadonlySet<string>
  /** Call number `n` of one function in this window: record it or not. */
  keep(n: number): boolean
  record(
    file: string,
    name: string,
    args: Array<number | bigint>,
    result: number | bigint,
    n: number,
  ): void
}

interface Entry {
  file: string
  fn: (...a: Array<number | bigint>) => number | bigint
  /** Calls asked about in this window; the next one's number. */
  n: number
  open: boolean
}

let tap: CardTap | undefined
let tapping = false
const entries: Entry[] = []

/** Set the tap, or clear it with undefined. Every gate opens for its cards. */
export function setCardTap(next: CardTap | undefined): void {
  tap = next
  reopenCardGates()
}

/** A new window: every tapped function is asked about again from n = 0. */
export function reopenCardGates(): void {
  for (const e of entries) {
    e.n = 0
    e.open = tap?.files.has(e.file) === true
  }
}

function noted(
  e: Entry,
  name: string,
  args: Array<number | bigint>,
  r: number | bigint,
) {
  const t = tap as CardTap
  tapping = true
  try {
    if (t.keep(e.n)) t.record(e.file, name, args, r, e.n)
    else e.open = false
    e.n++
  } finally {
    tapping = false
  }
}

export function loadCardWasm(
  file: string,
  root: string = DEFAULT_SPECS_ROOT,
): CardWasm {
  const key = `${root}:${file}`
  const hit = cache.get(key)
  if (hit) return hit
  const module = new WebAssembly.Module(readFileSync(join(root, file)))
  const imports = WebAssembly.Module.imports(module)
  if (imports.length > 0)
    throw new Error(
      `${file} imports ${imports.length} names; a card must import nothing`,
    )
  const x = new WebAssembly.Instance(module, {}).exports as Record<
    string,
    unknown
  >
  ;(x._initialize as (() => void) | undefined)?.()
  const memory = x.memory as WebAssembly.Memory
  const base = memory.buffer.byteLength
  let cursor = base

  const ensure = (end: number) => {
    const have = memory.buffer.byteLength
    if (end > have) memory.grow(Math.ceil((end - have) / 65_536))
  }

  const table: Record<string, Entry> = Object.create(null)
  const entry = (name: string): Entry => {
    const hit = table[name]
    if (hit !== undefined) return hit
    const fn = x[name] as Entry['fn'] | undefined
    if (typeof fn !== 'function') throw new Error(`${file} exports no ${name}`)
    const e: Entry = {
      file,
      fn,
      n: 0,
      open: tap?.files.has(file) === true,
    }
    table[name] = e
    entries.push(e)
    return e
  }

  const card: CardWasm = {
    call(name, ...args) {
      const e = entry(name)
      const r = e.fn(...args) as number
      if (e.open && !tapping && tap !== undefined) noted(e, name, args, r)
      return r
    },
    call64(name, ...args) {
      const e = entry(name)
      const r = e.fn(...args)
      if (e.open && !tapping && tap !== undefined) noted(e, name, args, r)
      return r
    },
    put(...texts) {
      cursor = base
      return texts.map((text) => {
        const bytes = new TextEncoder().encode(text)
        ensure(cursor + bytes.length + 1)
        new Uint8Array(memory.buffer, cursor, bytes.length).set(bytes)
        const at = cursor
        cursor += bytes.length + 1
        return { at, len: bytes.length }
      })
    },
    room(bytes) {
      ensure(cursor + bytes)
      const at = cursor
      cursor += bytes
      return at
    },
    read(at, len) {
      return new TextDecoder().decode(new Uint8Array(memory.buffer, at, len))
    },
  }
  cache.set(key, card)
  return card
}

export const flag = (b: boolean): number => (b ? 1 : 0)
export const u32 = (n: number): number =>
  Math.max(0, Math.min(0xffff_ffff, Math.floor(n))) >>> 0
export const u64 = (n: number): bigint =>
  BigInt(Number.isFinite(n) ? Math.max(0, Math.floor(n)) : 0)
