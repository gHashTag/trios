/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The texts the vault reads and writes (gHashTag/trios#1759):
 *   - the streams the owner pipes into `trios-vault import --stdin`: an
 *     Infisical dotenv export and the Railway `### <service>` + `KEY=VALUE`
 *     stream (merge.t27 section 1);
 *   - the vault's own export, written by the vault and read back by
 *     `import --format vault` (merge.t27 section 5);
 *   - the owner's merge plan (merge.t27 section 4).
 * Every decision -- what a line is, which names are left out, when a stream
 * may end, what a record or a plan line holds -- is the merge card's
 * (vault-cards.ts). This file splits text into lines and quotes into values.
 *
 * THE KV READERS RUN IN THE OWNER'S CLI, never in the server: the values they
 * read are encrypted there, to the vault's and the recovery recipient, and
 * only the ciphertext travels. Nothing here prints, logs or throws a value: a
 * refusal names a line number, never the line.
 */

import {
  DOTENV_EXPORT_PREFIX,
  EXPORT_DOMAIN,
  FMT_DOTENV,
  FMT_RAILWAY_KV,
  LK_BAD,
  LK_CONTINUATION,
  LK_ENTRY,
  LK_SERVICE,
  NO_SERVICE,
  PEM_BEGIN,
  PEM_END,
  PL_BAD,
  PL_MERGE,
  PLAN_CLASS,
  PLAN_EQUALS,
  PLAN_KEEP,
  PLAN_WORD,
  RECORD_WORDS,
  SERVICE_HEADER,
  VL_ALIAS,
  VL_BAD,
  VL_BIND,
  VL_FROM,
  VL_SECRET,
} from './queen-vault-merge-card.gen'
import {
  lineKind,
  planLineKind,
  platformName,
  skipOnImport,
  streamEndsClean,
  vaultLineKind,
} from './vault-cards'

export interface ParsedEntry {
  /** The `### <service>` it came under; null for dotenv. */
  service: string | null
  name: string
  value: Uint8Array
}

export interface ParsedStream {
  entries: ParsedEntry[]
  /** Platform names a railway-kv import leaves out (merge.t27 PLATFORM_PREFIX). */
  skipped: string[]
  /** Line numbers (1-based) the format refuses; never their text. */
  bad: number[]
}

const ASSIGN_KV = /^([A-Za-z_][A-Za-z0-9_]*)=/
const ASSIGN_DOTENV = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*/

const encoder = new TextEncoder()

/**
 * One dotenv value from the text after `=`. A quoted value ends at the first
 * line whose last non-blank character is its quote, so a quote inside a value
 * (`KEY='it's'`) stays in it and a quoted value runs over lines (a PEM key).
 */
function dotenvValue(
  first: string,
  lines: string[],
  next: number,
): { value: string; used: number } | null {
  const q = first[0]
  if (q !== '"' && q !== "'") return { value: first.trim(), used: 0 }
  const closes = (text: string, from: number) => {
    const t = text.trimEnd()
    return t.length > from && t.endsWith(q)
  }
  let text = first
  let used = 0
  while (!closes(text, used === 0 ? 1 : 0)) {
    if (next + used >= lines.length) return null
    text += `\n${lines[next + used]}`
    used++
  }
  const raw = text.trimEnd().slice(1, -1)
  return { value: q === '"' ? unescapeDouble(raw) : raw, used }
}

function unescapeDouble(raw: string): string {
  return raw.replace(/\\([nrt"\\])/g, (_, c: string) =>
    c === 'n' ? '\n' : c === 'r' ? '\r' : c === 't' ? '\t' : c,
  )
}

/** The reader's state between lines: the service, the open entry, an open PEM block. */
class KvReader {
  readonly out: ParsedStream = { entries: [], skipped: [], bad: [] }
  private service: string | null = null
  private current: {
    service: string | null
    name: string
    value: string
  } | null = null
  private open = false

  constructor(
    private readonly format: number,
    private readonly lines: string[],
  ) {}

  private flush(): void {
    const c = this.current
    if (c === null) return
    if (skipOnImport(this.format, platformName(c.name)))
      this.out.skipped.push(c.name)
    else
      this.out.entries.push({
        service: c.service,
        name: c.name,
        value: encoder.encode(c.value),
      })
    this.current = null
  }

  /** One line at index i; returns how many more lines it used, or -1 to stop. */
  step(i: number): number {
    const line = this.lines[i] as string
    const comment = line.trimStart().startsWith('#')
    const body =
      this.format === FMT_DOTENV &&
      line.trimStart().startsWith(DOTENV_EXPORT_PREFIX)
        ? line.trimStart().slice(DOTENV_EXPORT_PREFIX.length)
        : line
    const m = (this.format === FMT_RAILWAY_KV ? ASSIGN_KV : ASSIGN_DOTENV).exec(
      body,
    )
    const kind = lineKind(this.format, {
      blank: line.trim() === '',
      header: line.startsWith(SERVICE_HEADER),
      comment,
      assignment: m !== null && !comment,
      afterEntry: this.current !== null,
      openBlock: this.open,
    })
    if (kind === LK_SERVICE) {
      this.flush()
      this.service = line.slice(SERVICE_HEADER.length).trim()
    } else if (kind === LK_CONTINUATION && this.current !== null) {
      this.current.value += `\n${line}`
      if (this.open && line.startsWith(PEM_END)) this.open = false
    } else if (kind === LK_ENTRY && m !== null) return this.entry(i, m, body)
    else if (kind === LK_BAD) this.out.bad.push(i + 1)
    return 0
  }

  private entry(i: number, m: RegExpExecArray, body: string): number {
    this.flush()
    const after = body.slice(m[0].length)
    const name = m[1] as string
    if (this.format !== FMT_DOTENV) {
      this.current = { service: this.service, name, value: after }
      this.open = after.startsWith(PEM_BEGIN)
      return 0
    }
    const v = dotenvValue(after, this.lines, i + 1)
    if (v === null) {
      this.out.bad.push(i + 1)
      return -1
    }
    this.current = { service: null, name, value: v.value }
    this.flush()
    return v.used
  }

  end(): ParsedStream {
    if (!streamEndsClean(this.open)) this.out.bad.push(this.lines.length)
    this.flush()
    return this.out
  }
}

/** A dotenv or railway-kv stream. */
export function parseKvStream(text: string, format: number): ParsedStream {
  const lines = text
    .split('\n')
    .map((l) => (l.endsWith('\r') ? l.slice(0, -1) : l))
  if (lines.length > 0 && lines[lines.length - 1] === '') lines.pop()
  const reader = new KvReader(format, lines)
  for (let i = 0; i < lines.length; i++) {
    const used = reader.step(i)
    if (used < 0) return reader.out
    i += used
  }
  return reader.end()
}

// --- the vault's own export --------------------------------------------------

export interface DumpSecret {
  id: string
  classWord: string
  rotatedAt: number
  brokenGlass: boolean
  value: Uint8Array
}

export interface VaultDump {
  secrets: DumpSecret[]
  binds: Array<{ scope: string; env: string; id: string }>
  aliases: Array<{ id: string; canonical: string }>
  from: Array<{
    id: string
    source: string
    service: string | null
    name: string
  }>
  bad: number[]
}

/** The export's plaintext. Only the vault calls this, and encrypts it at once. */
export function formatVaultDump(dump: Omit<VaultDump, 'bad'>): Uint8Array {
  const out: string[] = [EXPORT_DOMAIN]
  const [secret, bind, alias, from] = RECORD_WORDS
  for (const s of dump.secrets)
    out.push(
      [
        secret,
        s.id,
        s.classWord,
        String(s.rotatedAt),
        s.brokenGlass ? '1' : '0',
        Buffer.from(s.value).toString('base64'),
      ].join(' '),
    )
  for (const b of dump.binds) out.push([bind, b.scope, b.env, b.id].join(' '))
  for (const a of dump.aliases) out.push([alias, a.id, a.canonical].join(' '))
  for (const f of dump.from)
    out.push([from, f.id, f.source, f.service ?? NO_SERVICE, f.name].join(' '))
  return encoder.encode(`${out.join('\n')}\n`)
}

/** A stream `trios-vault export` wrote, after the owner's `age -d`. */
export function parseVaultDump(text: string): VaultDump {
  const out: VaultDump = {
    secrets: [],
    binds: [],
    aliases: [],
    from: [],
    bad: [],
  }
  const lines = text.split('\n')
  if (lines.length > 1 && lines[lines.length - 1] === '') lines.pop()
  lines.forEach((line, i) => {
    const parts = line === '' ? [] : line.split(' ')
    const record = (RECORD_WORDS as readonly string[]).indexOf(parts[0] ?? '')
    const kind = vaultLineKind(
      i === 0,
      line === EXPORT_DOMAIN,
      line.trim() === '',
      record < 0 ? 255 : record,
      parts.length,
    )
    const p = parts as string[]
    if (kind === VL_BAD) out.bad.push(i + 1)
    else if (kind === VL_SECRET)
      out.secrets.push({
        id: p[1] as string,
        classWord: p[2] as string,
        rotatedAt: /^\d{1,12}$/.test(p[3] as string) ? Number(p[3]) : 0,
        brokenGlass: p[4] === '1',
        value: new Uint8Array(Buffer.from(p[5] as string, 'base64')),
      })
    else if (kind === VL_BIND)
      out.binds.push({
        scope: p[1] as string,
        env: p[2] as string,
        id: p[3] as string,
      })
    else if (kind === VL_ALIAS)
      out.aliases.push({ id: p[1] as string, canonical: p[2] as string })
    else if (kind === VL_FROM)
      out.from.push({
        id: p[1] as string,
        source: p[2] as string,
        service: p[3] === NO_SERVICE ? null : (p[3] as string),
        name: p[4] as string,
      })
  })
  return out
}

// --- the owner's merge plan -------------------------------------------------

export interface PlanLine {
  /** 1-based, for refusals. */
  line: number
  canonical: string
  members: string[]
  keep: string | null
  classWord: string | null
}

export function parsePlan(text: string): { lines: PlanLine[]; bad: number[] } {
  const out: { lines: PlanLine[]; bad: number[] } = { lines: [], bad: [] }
  text.split('\n').forEach((raw, i) => {
    const line = raw.trim()
    const parts = line === '' ? [] : line.split(/\s+/)
    let keep: string | null = null
    let classWord: string | null = null
    const members: string[] = []
    for (const p of parts.slice(3)) {
      if (p.startsWith(PLAN_KEEP)) keep = p.slice(PLAN_KEEP.length)
      else if (p.startsWith(PLAN_CLASS)) classWord = p.slice(PLAN_CLASS.length)
      else members.push(p)
    }
    const kind = planLineKind(
      line === '',
      line.startsWith('#'),
      parts[0] === PLAN_WORD,
      parts[2] === PLAN_EQUALS,
      members.length,
    )
    if (kind === PL_BAD) out.bad.push(i + 1)
    else if (kind === PL_MERGE)
      out.lines.push({
        line: i + 1,
        canonical: parts[1] as string,
        members,
        keep,
        classWord,
      })
  })
  return out
}

export { FMT_DOTENV, FMT_RAILWAY_KV }
