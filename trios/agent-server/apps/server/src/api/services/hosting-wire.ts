/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * What the Queen and a host agent both speak (gHashTag/trios#1756): the
 * signed message layout, the Ed25519 keys, and the shard's normalized output.
 * Plumbing only. Every decision is a card call (hosting-cards.ts); every name,
 * domain line and field list comes from the generated constants of
 * specs/hosting/proof.t27.
 *
 * THE MESSAGE: gHashTag/t27 specs/verified/signed_receipt.t27's layout, kept
 * exactly: the domain line, then one `name=value` line per signed field in the
 * spec's order, value being the field's JSON text, every line ending in a
 * newline. A missing field is refused here rather than signed as `undefined`.
 *
 * THE KEY: Ed25519 (RFC 8032) through node:crypto. A public key travels as 64
 * lowercase hex characters (its 32 raw bytes); a host id is the key id,
 * KEY_ID_HEX_LEN hex characters of the SHA-256 of those bytes.
 */

import {
  createHash,
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  type KeyObject,
  sign,
  verify,
} from 'node:crypto'
import { listsTests, reportVerdict, runVerdict } from './hosting-cards'
import {
  KEY_ID_HEX_LEN,
  SHARD_COUNT_KEYS,
  SHARD_DOMAIN,
  SHARD_SPEC_KEY,
  SHARD_TEST_KEY,
  SHARD_VERDICT_KEY,
  VERDICT_WORDS,
} from './queen-hosting-proof-card.gen'

// --- messages ---------------------------------------------------------------

export function messageOf(
  domain: string,
  fields: readonly string[],
  record: Record<string, unknown>,
): string {
  let text = `${domain}\n`
  for (const name of fields) {
    const value = record[name]
    if (value === undefined) throw new Error(`no ${name} to sign`)
    text += `${name}=${JSON.stringify(value)}\n`
  }
  return text
}

export const sha256Hex = (data: string | Uint8Array): string =>
  createHash('sha256').update(data).digest('hex')

// --- keys -------------------------------------------------------------------

/** DER prefix of an Ed25519 SubjectPublicKeyInfo; the 32 raw bytes follow it. */
const SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex')
const HEX64 = /^[0-9a-f]{64}$/

export function generateHostKey(): { privatePem: string; publicHex: string } {
  const { privateKey } = generateKeyPairSync('ed25519')
  const privatePem = privateKey
    .export({ format: 'pem', type: 'pkcs8' })
    .toString()
  return { privatePem, publicHex: publicHexOf(privatePem) }
}

/** DER prefix of an Ed25519 PKCS#8 private key; the 32-byte seed follows it. */
const PKCS8_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex')

/** A PKCS#8 PEM from a 32-byte Ed25519 seed given as hex (statement.t27 section 6). */
export function privatePemOfSeed(seedHex: string): string {
  return createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, Buffer.from(seedHex, 'hex')]),
    format: 'der',
    type: 'pkcs8',
  })
    .export({ format: 'pem', type: 'pkcs8' })
    .toString()
}

export function publicHexOf(privatePem: string): string {
  const der = createPublicKey(createPrivateKey(privatePem)).export({
    format: 'der',
    type: 'spki',
  })
  return Buffer.from(der).subarray(SPKI_PREFIX.length).toString('hex')
}

export function publicKeyOf(publicHex: string): KeyObject | null {
  if (!HEX64.test(publicHex)) return null
  try {
    return createPublicKey({
      key: Buffer.concat([SPKI_PREFIX, Buffer.from(publicHex, 'hex')]),
      format: 'der',
      type: 'spki',
    })
  } catch {
    return null
  }
}

export const keyIdOf = (publicHex: string): string =>
  sha256Hex(Buffer.from(publicHex, 'hex')).slice(0, KEY_ID_HEX_LEN)

export const signHex = (privatePem: string, message: string): string =>
  sign(null, Buffer.from(message), createPrivateKey(privatePem)).toString('hex')

export function verifyHex(
  publicHex: string,
  message: string,
  signatureHex: unknown,
): boolean {
  const key = publicKeyOf(publicHex)
  if (!key || typeof signatureHex !== 'string') return false
  if (!/^[0-9a-f]{128}$/.test(signatureHex)) return false
  try {
    return verify(
      null,
      Buffer.from(message),
      key,
      Buffer.from(signatureHex, 'hex'),
    )
  } catch {
    return false
  }
}

// --- the shard's output -----------------------------------------------------

/** What `t27c test-report <spec> --verbose` reports, read as the t27b lab reads it. */
export interface TestReport {
  blockedLine: boolean
  tests: number | null
  fail: number | null
  invariants: number
  vacuous: number
  /** In report order. */
  results: Array<{ name: string; passed: boolean }>
  /** Runtime asserts executed, by test name. */
  asserts: Map<string, number>
}

/**
 * The reading of contrib/railway/t27b-lab/lab.py parse_test_report and
 * parse_test_verdicts, line for line: the `tests` and `FAIL` counts, a
 * BLOCKED line, the per-test `pass`/`FAIL` lines after `--- test report:` up
 * to the first blank line; and the per-test runtime asserts after their
 * heading.
 */
export function readTestReport(stdout: string): TestReport {
  const lines = stdout.split(/\r?\n/)
  const report: TestReport = {
    blockedLine: lines.some((l) => l.trim().startsWith('BLOCKED')),
    tests: null,
    fail: null,
    invariants: 0,
    vacuous: 0,
    results: [],
    asserts: new Map(),
  }
  for (const line of lines) {
    const count = /^\s*(tests|FAIL)\s+(\d+)\s*$/.exec(line)
    if (count?.[1] === 'tests') report.tests = Number(count[2])
    if (count?.[1] === 'FAIL') report.fail = Number(count[2])
    const inv = /^\s*invariants\s+(\d+)\b/.exec(line)
    if (inv) report.invariants = Number(inv[1])
    const vac = /^\s*vacuous passes\s+(\d+)\s+of\b/.exec(line)
    if (vac) report.vacuous = Number(vac[1])
  }
  const start = lines.findIndex((l) => l.startsWith('--- test report:'))
  if (start >= 0) {
    for (const line of lines.slice(start + 1)) {
      const t = line.trimStart()
      if (!t) break
      if (t.startsWith('pass  '))
        report.results.push({ name: t.slice(6), passed: true })
      else if (t.startsWith('FAIL  '))
        report.results.push({ name: t.slice(6), passed: false })
      else break
    }
  }
  const heading = lines.findIndex((l) =>
    l.trim().startsWith('runtime asserts executed'),
  )
  if (heading >= 0) {
    for (const line of lines.slice(heading + 1)) {
      const m = /^\s+(\d+)\s+(\S+)\s*$/.exec(line)
      if (!m) break
      report.asserts.set(m[2] as string, Number(m[1]))
    }
  }
  return report
}

export interface RunOutcome {
  started: boolean
  timedOut: boolean
  exitCode: number | null
  stdout: string
  stderr: string
}

export interface ShardResult {
  verdict: number
  word: string
  /** The normalized output (proof.t27 section 1); output_hash is its SHA-256. */
  output: string
  outputHash: string
  /** Runtime asserts executed: the receipt's ops. */
  ops: number
  tests: number
  fail: number
}

/** lab.py reference_one: a fork the host could not afford is not a verdict. */
const outOfResources = (stderr: string) =>
  stderr.includes('Resource temporarily unavailable') ||
  stderr.includes('SystemResources')

/** The shard's verdict (the card) and its normalized output (proof.t27 section 1). */
export function normalizeShard(spec: string, run: RunOutcome): ShardResult {
  const report = readTestReport(run.stdout)
  const verdict = runVerdict({
    started: run.started,
    outOfResources: outOfResources(run.stderr),
    timedOut: run.timedOut,
    exitedZero: run.exitCode === 0,
    report: reportVerdict(
      report.blockedLine,
      report.tests !== null && report.fail !== null,
      report.fail ?? 0,
    ),
  })
  const word = VERDICT_WORDS[verdict] ?? 'blocked'
  let output = `${SHARD_DOMAIN}\n${SHARD_SPEC_KEY}=${spec}\n${SHARD_VERDICT_KEY}=${word}\n`
  let ops = 0
  for (const n of report.asserts.values()) ops += n
  if (!listsTests(verdict)) return finish(output, verdict, word, 0, 0, 0)
  const [tests, fail, invariants, asserts, vacuous] = SHARD_COUNT_KEYS
  output += `${tests}=${report.tests ?? 0}\n${fail}=${report.fail ?? 0}\n`
  output += `${invariants}=${report.invariants}\n${asserts}=${ops}\n${vacuous}=${report.vacuous}\n`
  for (const r of report.results) {
    const n = report.asserts.get(r.name)
    output += `${SHARD_TEST_KEY}=${r.passed ? 'pass' : 'FAIL'} ${n ?? '-'} ${r.name}\n`
  }
  return finish(output, verdict, word, ops, report.tests ?? 0, report.fail ?? 0)
}

function finish(
  output: string,
  verdict: number,
  word: string,
  ops: number,
  tests: number,
  fail: number,
): ShardResult {
  return {
    verdict,
    word,
    output,
    outputHash: sha256Hex(output),
    ops,
    tests,
    fail,
  }
}

/** The verdict word and asserts a normalized output states, for the Queen's check. */
export function readNormalized(
  output: string,
): { spec: string; word: string; ops: number; tests: number } | null {
  const lines = output.split('\n')
  if (lines[0] !== SHARD_DOMAIN) return null
  const field = (key: string) => {
    const line = lines.find((l) => l.startsWith(`${key}=`))
    return line === undefined ? undefined : line.slice(key.length + 1)
  }
  const spec = field(SHARD_SPEC_KEY)
  const word = field(SHARD_VERDICT_KEY)
  if (spec === undefined || word === undefined) return null
  return {
    spec,
    word,
    ops: Number(field(SHARD_COUNT_KEYS[3]) ?? 0),
    tests: Number(field(SHARD_COUNT_KEYS[0]) ?? 0),
  }
}
