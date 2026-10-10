/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE DECISION LOG AS A t27 CONFORMANCE SPEC (gHashTag/t27
 * specs/queen/replay/, epic trios#1712). Test glue, not a rule: it decides
 * nothing the runtime does. It turns what the wasm cards answered on a seeded
 * run into one t27 module per card, one assert per distinct (fn, args), so the
 * same answers are checked on the reference path (`t27c test-report`: gen +
 * zig) and on t27b's native JIT. Three backends, one set of answers.
 *
 * A value crosses into wasm as a JS number or BigInt and comes back signed
 * (an i32 or an i64). It is written into the spec as the t27 type the card's
 * signature names: a u32 as its unsigned value, a u64 the same, a bool as
 * true or false. A value no t27 literal of that type can hold is counted, not
 * written: that is the host passing what the card never declared.
 */

import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import type { DecisionRecord } from '../../src/api/services/queen-actors-telemetry'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'

/** Asserts per test block: a chunk the reference path compiles and runs as one test. */
export const ASSERTS_PER_TEST = 200

type Sig = { params: string[]; ret: string }

const signatures = (spec: string): Map<string, Sig> => {
  const out = new Map<string, Sig>()
  for (const m of spec.matchAll(/^pub fn (\w+)\(([^)]*)\)\s*->\s*(\w+)/gm))
    out.set(m[1], {
      params: m[2]
        .split(',')
        .map((p) => p.trim())
        .filter(Boolean)
        .map((p) => p.split(':')[1].trim()),
      ret: m[3],
    })
  return out
}

const BITS: Record<string, number> = { u8: 8, u16: 16, u32: 32, u64: 64 }

/** The t27 literal for a value the wasm saw as `type`, or undefined if none holds it. */
const literal = (v: unknown, type: string, result: boolean) => {
  const n = typeof v === 'string' ? BigInt(v) : (v as number | bigint)
  if (type === 'bool') {
    if (Number(n) === 0) return 'false'
    if (Number(n) === 1) return 'true'
    return undefined
  }
  const bits = BITS[type]
  if (bits === undefined) return undefined
  if (typeof n === 'number' && !Number.isInteger(n)) return undefined
  const big = BigInt(n)
  // a result is read back signed from an i32 or i64; an argument must already
  // be in range, or the host passed a value the type cannot hold
  const wire = bits <= 32 ? 32 : 64
  const u = result ? BigInt.asUintN(wire, big) : big
  if (u < 0n || u >= 1n << BigInt(bits)) return undefined
  return u.toString()
}

/** Prose as `; ` comment lines of at most 100 columns. */
const prose = (text: string): string[] => {
  const out: string[] = []
  let line = ';'
  for (const word of text.split(' ')) {
    if (line.length + 1 + word.length > 100 && line !== ';') {
      out.push(line)
      line = ';'
    }
    line += ` ${word}`
  }
  out.push(line)
  return out
}

const cmpArgs = (a: string[], b: string[]) => {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    if (a[i] === b[i]) continue
    const x = /^\d+$/.test(a[i]) ? BigInt(a[i]) : a[i] === 'true' ? 1n : 0n
    const y = /^\d+$/.test(b[i]) ? BigInt(b[i]) : b[i] === 'true' ? 1n : 0n
    return x < y ? -1 : 1
  }
  return a.length - b.length
}

export interface DecisionSpecReport {
  logged: number
  unique: number
  /** Same (card, fn, args), two answers: the wasm card was not a function of its arguments. */
  conflicts: Array<Record<string, unknown>>
  /** Records no t27 literal of the declared type can hold. */
  unwritable: Array<Record<string, unknown>>
  perFn: Array<{ card: string; fn: string; logged: number; unique: number }>
  files: string[]
}

interface Distinct {
  card: string
  fn: string
  args: unknown[]
  result: unknown
  /** How often it was logged. */
  n: number
}

const wide = (v: number | bigint) => (typeof v === 'bigint' ? String(v) : v)

/** One entry per (card, fn, args); a second answer to the same call is a conflict. */
const distinct = (records: DecisionRecord[]) => {
  const seen = new Map<string, Distinct>()
  const conflicts: DecisionSpecReport['conflicts'] = []
  for (const r of records) {
    const args = r.args.map(wide)
    const key = JSON.stringify([r.card, r.fn, args])
    const had = seen.get(key)
    if (had === undefined) {
      seen.set(key, {
        card: r.card,
        fn: r.fn,
        args,
        result: wide(r.result),
        n: 1,
      })
      continue
    }
    had.n++
    if (String(had.result) !== String(wide(r.result)))
      conflicts.push({
        card: r.card,
        fn: r.fn,
        args,
        first: had.result,
        second: wide(r.result),
      })
  }
  return { seen, conflicts }
}

const sha256 = (bytes: string | Buffer) =>
  createHash('sha256').update(bytes).digest('hex')

/** The module of one card: its text, its per-fn counts and what it could not write. */
function cardModule(
  card: string,
  calls: Distinct[],
  from: string,
  run: string,
) {
  const name = card.replace(/^queen\//, '').replace(/\.wasm$/, '')
  const specText = readFileSync(
    join(DEFAULT_SPECS_ROOT, `queen/${name}.t27`),
    'utf8',
  )
  const sigs = signatures(specText)
  const fns = new Map<string, Array<{ args: string[]; result: string }>>()
  const loggedOf = new Map<string, number>()
  const unwritable: DecisionSpecReport['unwritable'] = []
  for (const u of calls) {
    loggedOf.set(u.fn, (loggedOf.get(u.fn) ?? 0) + u.n)
    const sig = sigs.get(u.fn)
    const args = (sig ? u.args : []).map((a, i) =>
      literal(a, sig?.params[i] ?? '', false),
    )
    const result = sig ? literal(u.result, sig.ret, true) : undefined
    if (
      !sig ||
      sig.params.length !== u.args.length ||
      result === undefined ||
      args.some((a) => a === undefined)
    ) {
      unwritable.push({ card, fn: u.fn, args: u.args, result: u.result, sig })
      continue
    }
    const list = fns.get(u.fn) ?? []
    list.push({ args: args as string[], result })
    fns.set(u.fn, list)
  }
  // the card's own order, then each fn's calls by their arguments
  const used = [...sigs.keys()].filter((f) => fns.has(f))
  const unique = used.reduce((a, f) => a + (fns.get(f)?.length ?? 0), 0)
  const logged = calls.reduce((a, u) => a + u.n, 0)
  const module = `QueenReplay${name
    .split('_')
    .map((s) => s[0].toUpperCase() + s.slice(1))
    .join('')}Decisions`
  const lines = [
    '// SPDX-License-Identifier: Apache-2.0',
    `; specs/queen/replay/${name}_decisions.t27 -- the ${name} card's logged answers, as asserts`,
    ...prose(
      `GENERATED from a decision log. Do not edit by hand: regenerate it (below). Source: the decision log of ${run} in gHashTag/trios trios/agent-server/apps/server/tests/api/queen-actors-telemetry.test.ts, logged at trios commit ${from}. It holds ${logged} calls into queen/${name}.wasm (sha256 ${sha256(readFileSync(join(DEFAULT_SPECS_ROOT, card)))}), compiled from queen/${name}.t27 (sha256 ${sha256(specText)}).`,
    ),
    ...prose(
      `Each assert is one distinct (fn, args) of those calls with the answer the wasm card gave: ${unique} asserts over ${used.length} functions. t27c test-report runs them on the reference path (gen, then zig) and t27b runs them on its native AArch64 JIT, so production's wasm, the reference and t27b give the same answer to every question production asked, or this module fails. Epic gHashTag/trios#1712.`,
    ),
    '; Regenerate: in trios/agent-server/apps/server,',
    ';   QUEEN_DECISION_SPEC_OUT=<t27 checkout> QUEEN_DECISION_SPEC_FROM=<trios commit>',
    ";   bun test tests/api/queen-actors-telemetry.test.ts -t 'replays to the same'",
    '; ASCII only (L3).',
    '; phi^2 + 1/phi^2 = 3 | TRINITY',
    '',
    `module ${module};`,
    '',
    ...used.map((f) => `use queen::${name}::${f};`),
  ]
  const perFn: DecisionSpecReport['perFn'] = []
  for (const f of used) {
    const list = (fns.get(f) ?? []).sort((a, b) => cmpArgs(a.args, b.args))
    perFn.push({
      card,
      fn: f,
      logged: loggedOf.get(f) ?? 0,
      unique: list.length,
    })
    lines.push(
      '',
      `; ${f}: ${loggedOf.get(f)} calls logged, ${list.length} distinct`,
    )
    const chunks = Math.ceil(list.length / ASSERTS_PER_TEST)
    for (let c = 0; c < chunks; c++) {
      if (c > 0) lines.push('')
      lines.push(
        `test ${f}_answers_as_logged${chunks > 1 ? `_${c + 1}_of_${chunks}` : ''} {`,
      )
      for (const r of list.slice(
        c * ASSERTS_PER_TEST,
        (c + 1) * ASSERTS_PER_TEST,
      ))
        lines.push(`    assert ${f}(${r.args.join(', ')}) == ${r.result};`)
      lines.push('}')
    }
  }
  lines.push('')
  return { name, text: lines.join('\n'), perFn, unwritable }
}

/**
 * Write the log of a run as `specs/queen/replay/<card>_decisions.t27` under
 * `outRoot` (a t27 checkout), and the distinct records, each with how often it
 * was logged, as JSON to `logOut` when given. `from` names the trios commit
 * the log was taken at; `run` says which run it is.
 */
export function writeDecisionSpecs(
  records: DecisionRecord[],
  outRoot: string,
  from: string,
  run: string,
  logOut?: string,
): DecisionSpecReport {
  const { seen, conflicts } = distinct(records)
  const byCard = new Map<string, Distinct[]>()
  for (const u of seen.values())
    byCard.set(u.card, [...(byCard.get(u.card) ?? []), u])
  const dir = join(outRoot, 'specs/queen/replay')
  mkdirSync(dir, { recursive: true })
  const report: DecisionSpecReport = {
    logged: records.length,
    unique: seen.size,
    conflicts,
    unwritable: [],
    perFn: [],
    files: [],
  }
  for (const card of [...byCard.keys()].sort()) {
    const m = cardModule(card, byCard.get(card) ?? [], from, run)
    const file = join(dir, `${m.name}_decisions.t27`)
    writeFileSync(file, m.text)
    report.files.push(file)
    report.perFn.push(...m.perFn)
    report.unwritable.push(...m.unwritable)
  }
  if (logOut) writeFileSync(logOut, JSON.stringify([...seen.values()]))
  return report
}
