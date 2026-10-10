/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's client (gHashTag/trios#1759): what `trios-vault` and a workload
 * run. It signs each request with the caller's Ed25519 key (vault-wire.ts),
 * and for a lease makes a one-lease age identity in memory, so the answer is
 * ciphertext only this process can open.
 *
 * `run --scope S -- cmd` (runScoped) is the only place a value leaves the
 * vault in the clear: into the environment of the child it starts. The
 * parent prints nothing about it, renews the lease when policy.t27 renew_due
 * says so, and stops the child when child_must_stop says so -- a refused
 * renewal, or a lease run out because the vault could not be reached.
 */

import { FMT_VAULT, FORMAT_NAMES } from './queen-vault-merge-card.gen'
import { decryptWithIdentity, encrypt, ephemeralIdentity } from './vault-age'
import { childMustStop, renewDue } from './vault-cards'
import {
  FMT_DOTENV,
  FMT_RAILWAY_KV,
  parseKvStream,
  parseVaultDump,
} from './vault-formats'
import { signRequest } from './vault-wire'

export type Transport = (
  path: string,
  init: { method: string; headers: Record<string, string>; body: string },
) => Promise<{
  status: number
  text: string
}>

/** A vault at a URL; requests go to <url>/<operation> and are signed with that path. */
export function httpTransport(): Transport {
  return async (path, init) => {
    const r = await fetch(path, init)
    return { status: r.status, text: await r.text() }
  }
}

/** A vault in this process (the owner's local mode, and tests). */
export function appTransport(app: {
  request: (path: string, init: RequestInit) => Response | Promise<Response>
}): Transport {
  return async (path, init) => {
    const r = await app.request(path, init)
    return { status: r.status, text: await r.text() }
  }
}

export class VaultRefused extends Error {
  constructor(
    readonly code: number,
    readonly reason: string,
    readonly detail: Record<string, unknown> = {},
  ) {
    super(`refused: ${reason} (${code})`)
  }
}

export type Answer = Record<string, unknown> & { ok?: boolean }

export class VaultClient {
  /**
   * @param base where the vault is: a URL (`https://host/vault`) for the
   *   http transport, or a path (`/vault`) for an in-process app.
   */
  constructor(
    private readonly transport: Transport,
    private readonly base: string,
    private readonly keyPem: string,
    private readonly now: () => number = () => Date.now() / 1000,
  ) {}

  /** One signed operation. A refusal is thrown; the answer is returned. */
  async call(op: string, body: Record<string, unknown> = {}): Promise<Answer> {
    const target = `${this.base.replace(/\/$/, '')}/${op}`
    const path = /^https?:\/\//.test(target) ? new URL(target).pathname : target
    const text = JSON.stringify(body)
    const headers = {
      'content-type': 'application/json',
      ...signRequest(this.keyPem, 'POST', path, text, this.now()),
    }
    const r = await this.transport(target, {
      method: 'POST',
      headers,
      body: text,
    })
    let answer: Answer
    try {
      answer = JSON.parse(r.text) as Answer
    } catch {
      throw new Error(`the vault answered ${r.status} with no JSON`)
    }
    if (answer.ok !== true) {
      if (typeof answer.code === 'number')
        throw new VaultRefused(
          answer.code,
          String(answer.reason ?? 'unknown'),
          answer,
        )
      throw new Error(
        `the vault answered ${r.status}: ${String(answer.error ?? 'no reason')}`,
      )
    }
    return answer
  }

  /** The vault's and the recovery recipient: what put and import encrypt to. */
  async recipients(): Promise<{ vault: string; recovery: string }> {
    const a = await this.call('names')
    return a.recipients as { vault: string; recovery: string }
  }

  /** Lease a scope (or break the glass for ids) and open the answer in memory. */
  async open(
    op: 'lease' | 'break-glass',
    body: Record<string, unknown>,
  ): Promise<{ lease: string; expires: number; env: Record<string, string> }> {
    const { identity, recipient } = await ephemeralIdentity()
    try {
      const a = await this.call(op, { ...body, recipient })
      const plain = await decryptWithIdentity(
        new Uint8Array(Buffer.from(String(a.ct), 'base64')),
        identity,
      )
      const env: Record<string, string> = {}
      for (const line of new TextDecoder().decode(plain).split('\n')) {
        const at = line.indexOf(' ')
        if (at > 0)
          env[line.slice(0, at)] = Buffer.from(
            line.slice(at + 1),
            'base64',
          ).toString('utf8')
      }
      plain.fill(0)
      return { lease: String(a.lease), expires: Number(a.expires), env }
    } finally {
      identity.fill(0)
    }
  }
}

/** A value encrypted to the vault's and the recovery recipient; the plaintext is zeroed. */
export async function sealValue(
  value: Uint8Array,
  r: { vault: string; recovery: string },
): Promise<string> {
  const ct = await encrypt(value, [r.vault, r.recovery])
  value.fill(0)
  return Buffer.from(ct).toString('base64')
}

/**
 * The body of an import: the owner's stream read in its format (merge.t27)
 * and every value sealed here, so only ciphertext goes to the vault. `bad`
 * holds the line numbers the format refused; the body is then not to be sent.
 */
export async function importBody(
  text: string,
  format: string,
  recipients: { vault: string; recovery: string },
  opts: { source: string; input: string; scope?: string; class?: string },
): Promise<{ body: Record<string, unknown>; bad: number[] }> {
  const f = (FORMAT_NAMES as readonly string[]).indexOf(format)
  const body: Record<string, unknown> = { ...opts, format }
  if (f === FMT_VAULT) {
    const dump = parseVaultDump(text)
    const entries = []
    for (const s of dump.secrets)
      entries.push({
        id: s.id,
        class: s.classWord,
        rotatedAt: s.rotatedAt,
        brokenGlass: s.brokenGlass,
        ct: dump.bad.length > 0 ? '' : await sealValue(s.value, recipients),
      })
    Object.assign(body, {
      entries,
      binds: dump.binds,
      aliases: dump.aliases,
      from: dump.from,
    })
    return { body, bad: dump.bad }
  }
  const parsed = parseKvStream(
    text,
    f === FMT_RAILWAY_KV ? FMT_RAILWAY_KV : FMT_DOTENV,
  )
  const entries = []
  for (const e of parsed.entries)
    entries.push({
      service: e.service,
      name: e.name,
      ct: parsed.bad.length > 0 ? '' : await sealValue(e.value, recipients),
    })
  for (const e of parsed.entries) e.value.fill(0)
  Object.assign(body, { entries, skipped: parsed.skipped })
  return { body, bad: f < 0 ? [0] : parsed.bad }
}

export interface RunOptions {
  /** Inherit by default; a test passes 'ignore' or 'pipe'. */
  stdio?: 'inherit' | 'ignore' | 'pipe'
  /** How often the parent asks renew_due and child_must_stop, in ms. */
  tickMs?: number
  /** Extra environment for the child, under the leased one. */
  baseEnv?: Record<string, string | undefined>
}

export interface RunResult {
  exitCode: number
  /** The parent stopped the child (policy.t27 child_must_stop). */
  stopped: boolean
  renewals: number
}

/** `run --scope S -- argv`, or `break-glass --ids ... -- argv` when `breakGlass` is given. */
export async function runScoped(
  client: VaultClient,
  target: { scope: string } | { ids: string[]; reason: string },
  argv: string[],
  opts: RunOptions = {},
  now: () => number = () => Date.now() / 1000,
): Promise<RunResult> {
  const breakGlass = 'ids' in target
  const leased = breakGlass
    ? await client.open('break-glass', {
        ids: target.ids,
        reason: target.reason,
      })
    : await client.open('lease', { scope: target.scope })
  const stdio = opts.stdio ?? 'inherit'
  const child = Bun.spawn(argv, {
    env: { ...(opts.baseEnv ?? process.env), ...leased.env },
    stdin: stdio === 'pipe' ? 'ignore' : stdio,
    stdout: stdio,
    stderr: stdio,
  })
  for (const k of Object.keys(leased.env)) leased.env[k] = ''
  let heldFrom = now()
  let expires = leased.expires
  let refused = false
  let stopped = false
  let renewals = 0
  let busy = false
  const timer = setInterval(async () => {
    if (busy || stopped) return
    busy = true
    try {
      const t = now()
      if (!refused && renewDue(Math.floor(t - heldFrom), breakGlass)) {
        try {
          const a = await client.call('renew', { lease: leased.lease })
          heldFrom = now()
          expires = Number(a.expires)
          renewals++
        } catch (error) {
          // a refusal is final; an unreachable vault is retried until the lease runs out
          if (error instanceof VaultRefused) refused = true
        }
      }
      if (childMustStop(refused, Math.max(0, Math.floor(expires - now())))) {
        stopped = true
        child.kill('SIGTERM')
      }
    } finally {
      busy = false
    }
  }, opts.tickMs ?? 1000)
  const exitCode = await child.exited
  clearInterval(timer)
  try {
    await client.call('revoke', { lease: leased.lease })
  } catch {
    // the lease ends at its expiry anyway
  }
  return { exitCode, stopped, renewals }
}
