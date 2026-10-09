/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE CARD LOADER (queen-card-wasm.ts), the part no actor test reaches:
 * text into a card's memory and back, memory that grows for a long text, and
 * the refusal of a module that imports anything (gHashTag/trios#1731).
 */

import { afterAll, describe, expect, it } from 'bun:test'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { loadCardWasm } from '../../src/api/services/queen-card-wasm'
import { EVENTS_CARD, tokenOk } from '../../src/api/services/queen-events'

const dirs: string[] = []
afterAll(() => {
  for (const d of dirs) rmSync(d, { recursive: true, force: true })
})

describe('text crosses into a card and back', () => {
  it('put writes each text where read finds it, one after another', () => {
    const card = loadCardWasm(EVENTS_CARD)
    const [a, b] = card.put('first', 'second text')
    expect(a.len).toBe(5)
    expect(b.len).toBe(11)
    // the second text starts after the first and its terminating byte
    expect(b.at).toBe(a.at + a.len + 1)
    expect(card.read(a.at, a.len)).toBe('first')
    expect(card.read(b.at, b.len)).toBe('second text')
  })

  it('a later put starts again at the same address: scratch memory is reused', () => {
    const card = loadCardWasm(EVENTS_CARD)
    const [first] = card.put('aaaa')
    const [again] = card.put('bb')
    expect(again.at).toBe(first.at)
    expect(card.read(again.at, again.len)).toBe('bb')
  })

  it('the card reads the bytes put wrote: a token it accepts and one it refuses', () => {
    // events.t27 token_ok judges the text at (at, len) in its own memory. Two
    // texts of one length that differ in one byte get different answers, so
    // the card read the bytes, not only the length
    expect(tokenOk('gHashTag/t27')).toBe(true)
    expect(tokenOk('gHashTag t27')).toBe(false)
  })

  it('a text longer than the memory the module was built with grows it', () => {
    const card = loadCardWasm(EVENTS_CARD)
    const long = 'x'.repeat(300_000)
    const [b] = card.put(long)
    expect(b.len).toBe(300_000)
    expect(card.read(b.at + 299_990, 10)).toBe('x'.repeat(10))
  })

  it('room reserves bytes after the last put, growing the memory for them', () => {
    const card = loadCardWasm(EVENTS_CARD)
    const [t] = card.put('abc')
    const at = card.room(400_000)
    expect(at).toBe(t.at + t.len + 1)
    // without the growth this read past the old end throws a RangeError
    expect(card.read(at + 399_999, 1)).toBe('\0')
    // the next room starts after this one
    expect(card.room(8)).toBe(at + 400_000)
  })
})

describe('a card imports nothing', () => {
  it('a module with an import is refused by name, before it is instantiated', () => {
    const dir = mkdtempSync(join(tmpdir(), 'queen-card-wasm-'))
    dirs.push(dir)
    // magic, version; a type section with () -> (); an import section with
    // one function "m"."f" of that type
    const bytes = new Uint8Array([
      0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60,
      0x00, 0x00, 0x02, 0x07, 0x01, 0x01, 0x6d, 0x01, 0x66, 0x00, 0x00,
    ])
    writeFileSync(join(dir, 'imports.wasm'), bytes)
    expect(() => loadCardWasm('imports.wasm', dir)).toThrow(
      'imports.wasm imports 1 names; a card must import nothing',
    )
  })

  it('a function the card does not export is refused by name', () => {
    const card = loadCardWasm(EVENTS_CARD)
    expect(() => card.call('no_such_fn')).toThrow(
      `${EVENTS_CARD} exports no no_such_fn`,
    )
  })
})
