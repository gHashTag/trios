/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault (gHashTag/trios#1759): our own key store. A workload asks for a
 * scope by NAME with a signed request, and gets its values back encrypted to a
 * one-lease recipient it made, so no response, log, event or report carries a
 * value. The owner puts, rotates, imports, merges and exports.
 *
 * EVERY DECISION IS A CARD (vault-cards.ts, gHashTag/t27 specs/vault/*.t27):
 * whether a request is signed, fresh and new; whether a tier may do an
 * operation; whether a scope, a name, a class and a tier admit a lease; TTL,
 * renewal and its cap; put, rotate, bind and import admission; row checks;
 * rotation age and alarms; export target; break-glass; merges and plans; what
 * an event may carry. This file holds the state, does the I/O (age, the
 * store, the audit sink) and keeps the order the specs give:
 *   - a request is judged before its nonce is remembered (request_code);
 *   - an act's event is written before the act is delivered, and an act whose
 *     event could not be written is refused (deliver_code);
 *   - an import, a lease and a merge plan are all or nothing.
 *
 * WHERE A VALUE IS IN THE CLEAR: only in this process's memory, between
 * `age --decrypt` and `age --encrypt`, for a lease, a break-glass, an export,
 * a dedup or a merge; and in the owner's CLI before it encrypts. Plaintext
 * buffers are zeroed when the act is done. Put and import arrive as
 * ciphertext: the vault checks that a row opens with its identity, into
 * /dev/null, and keeps it unopened (policy.t27 row_code).
 */

import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto'
import { join } from 'node:path'
import {
  EV_AUDIT_READ,
  EV_BIND,
  EV_BREAK_GLASS,
  EV_CLASS,
  EV_DEDUP,
  EV_DENY,
  EV_EXPIRE,
  EV_EXPORT,
  EV_GRANT,
  EV_IMPORT,
  EV_LEASE,
  EV_MERGE,
  EV_PUT,
  EV_RENEW,
  EV_REVOKE,
  EV_ROTATE,
  EV_ROTATION_DUE,
} from './queen-vault-audit-card.gen'
import {
  DEDUP_KEY_BYTES,
  FMT_RAILWAY_KV,
  FMT_VAULT,
  FORMAT_NAMES,
  NAME_GROUP_NAMES,
  OWNER_SOURCE,
  REPORT_FIELDS,
  VALUE_GROUP_NAMES,
} from './queen-vault-merge-card.gen'
import {
  BIND_ADD,
  BIND_MODE_NAMES,
  BIND_REMOVE,
  CLASS_NAMES,
  CLASS_NONE,
  CODE_NAMES,
  CODE_OK,
  DENY_ALIAS,
  DENY_BAD_CIPHERTEXT,
  DENY_BAD_NAME,
  DENY_DUPLICATE_IN_STREAM,
  DENY_EXISTS,
  DENY_LEASE_UNKNOWN,
  DENY_MISSING,
  DENY_NO_AUDIT,
  DENY_NONCE_FULL,
  DENY_REPLAY,
  DENY_SIGNATURE,
  DENY_STALE,
  INPUT_NAMES,
  OP_AUDIT,
  OP_BIND,
  OP_BREAK_GLASS,
  OP_CLASS,
  OP_DEDUP,
  OP_EXPORT,
  OP_GRANT,
  OP_IMPORT,
  OP_LEASE,
  OP_MERGE,
  OP_NAMES,
  OP_NAMES_TEXT,
  OP_PUT,
  OP_RENEW,
  OP_REVOKE,
  OP_ROTATE,
  SECRET_CLASSES,
  TIER_OWNER,
} from './queen-vault-policy-card.gen'
import {
  decryptWith,
  encrypt,
  isRecipient,
  opensWithIdentity,
  stanzaCount,
  type VaultIdentity,
} from './vault-age'
import type { AuditPayload, AuditSink } from './vault-audit'
import {
  alarmDue,
  auditPage,
  bindCode,
  bindRotates,
  breakGlassCode,
  canonicalFree,
  classOrOther,
  deliverCode,
  denyRecorded,
  entryCode,
  expiresAt,
  exportCode,
  grantCode,
  hasService,
  idOk,
  importClass,
  importCode,
  importedRotatedAt,
  leaseAdmitted,
  leaseCode,
  leaseLive,
  mergeCode,
  mergedClass,
  mergedDue,
  mergedRotatedAt,
  mergeRevokes,
  nameGroupKind,
  nameGroupShown,
  nameOk,
  nonceForgetAt,
  opCode,
  planCode,
  putCode,
  renewalsCap,
  renewCode,
  requestCode,
  revokeCode,
  rotationDue,
  rowCode,
  scopeCode,
  scopeOk,
  sourceOk,
  sweepDue,
  targetCode,
  textOk,
  valueGroupKind,
  valueGroupShown,
} from './vault-cards'
import { formatVaultDump } from './vault-formats'
import {
  type Binding,
  emptyState,
  IDENTITY_FILE,
  type Lease,
  type Provenance,
  type SecretRow,
  StoreConflict,
  type VaultState,
  type VaultStore,
} from './vault-store'
import {
  isPublicHex,
  keyIdOf,
  readSignedHeaders,
  shortHash,
  verifyRequest,
} from './vault-wire'

export interface VaultOptions {
  /** The vault's own age identity: a file (local mode), or text held in memory (the server). */
  identity: VaultIdentity
  store: VaultStore
  audit: AuditSink
  /** Unix seconds; a test passes a virtual clock. */
  now?: () => number
  /** Where `age` and `age-keygen` are (vault-age.ts ageBinaries). */
  env?: Record<string, string | undefined>
}

export interface VaultRequest {
  method: string
  /** The path the client signed, e.g. /vault/lease. */
  path: string
  header(name: string): string | undefined | null
  body: string
}

export interface VaultResponse {
  status: number
  body: Record<string, unknown>
}

interface Ctx {
  state: VaultState
  now: number
  op: number
  key: string
  keyId: string
  tier: number
  scopes: string[]
  body: Record<string, unknown>
  req: string
}

const DAY = 86_400
const str = (v: unknown): string => (typeof v === 'string' ? v : '')
const num = (v: unknown): number =>
  typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 ? v : 0
const strs = (v: unknown): string[] =>
  Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []
const lastPart = (id: string): string => id.slice(id.lastIndexOf('/') + 1)
const classIndex = (word: unknown): number => {
  const i = (CLASS_NAMES as readonly string[]).indexOf(str(word))
  return i < 0 ? CLASS_NONE : i
}
const b64 = (bytes: Uint8Array): string => Buffer.from(bytes).toString('base64')
const unb64 = (text: string): Uint8Array =>
  new Uint8Array(Buffer.from(text, 'base64'))
const unique = <T>(xs: T[]): T[] => [...new Set(xs)]
const tokens = (xs: string[]): string[] => xs.filter((x) => textOk(x))

/** The refusals that are about the request itself answer 401; the rest 403. */
const STATUS_401 = new Set([
  DENY_SIGNATURE,
  DENY_STALE,
  DENY_REPLAY,
  DENY_NONCE_FULL,
])

export class VaultError extends Error {}

export class Vault {
  private readonly identity: VaultIdentity
  private readonly now: () => number
  private readonly env: Record<string, string | undefined>
  private state: VaultState | null = null
  private queue: Promise<unknown> = Promise.resolve()
  /** nonce -> unix second it may be forgotten (policy.t27 nonce_forget_at) */
  private readonly nonces = new Map<string, number>()

  constructor(private readonly opts: VaultOptions) {
    this.identity = opts.identity
    this.now = opts.now ?? (() => Math.floor(Date.now() / 1000))
    this.env = opts.env ?? process.env
  }

  /** One request at a time: every act reads and writes one state. */
  handle(req: VaultRequest): Promise<VaultResponse> {
    const run = this.queue.then(() => this.handleNow(req))
    this.queue = run.catch(() => undefined)
    return run
  }

  private async load(): Promise<VaultState> {
    if (this.state) return this.state
    const loaded = await this.opts.store.load()
    if (!loaded) throw new VaultError('the vault has no store; run init')
    this.state = loaded
    return loaded
  }

  private async save(state: VaultState): Promise<void> {
    await this.opts.store.save(state)
  }

  private async handleNow(req: VaultRequest): Promise<VaultResponse> {
    try {
      return await this.dispatch(req)
    } catch (error) {
      // A failure part-way may have changed the state in memory; the store
      // (file or row) is the truth, so read it again next time.
      this.state = null
      if (error instanceof VaultError)
        return { status: 400, body: { ok: false, error: error.message } }
      if (error instanceof StoreConflict)
        return {
          status: 409,
          body: {
            ok: false,
            error: 'the store was written meanwhile; ask again',
          },
        }
      throw error
    }
  }

  private async dispatch(req: VaultRequest): Promise<VaultResponse> {
    const now = this.now()
    const state = await this.load()
    await this.sweep(state, now)
    const op = (OP_NAMES_TEXT as readonly string[]).indexOf(lastPart(req.path))
    if (op < 0)
      return {
        status: 404,
        body: { ok: false, error: 'no such vault operation' },
      }

    // --- the request: signed, fresh, new (policy.t27 request_code) ---------
    const h = readSignedHeaders((n) => req.header(n) ?? undefined)
    const grant = h ? state.grants[h.key] : undefined
    const signatureOk =
      h !== null &&
      grant !== undefined &&
      verifyRequest(h, req.method, req.path, req.body)
    for (const [nonce, at] of this.nonces) {
      if (at > now) break
      this.nonces.delete(nonce)
    }
    const verdict = requestCode({
      keyKnown: grant !== undefined,
      signatureOk,
      skewSeconds: h ? Math.abs(now - h.utcUnix) : 0,
      nonceBytes: h ? h.nonce.length / 2 : 0,
      nonceSeen: h ? this.nonces.has(h.nonce) : false,
      cacheSize: this.nonces.size,
    })
    const reqHash = shortHash(req.body)
    if (verdict !== CODE_OK || !h || !grant) {
      const payload: AuditPayload = {
        op: OP_NAMES_TEXT[op] as string,
        code: verdict,
        req: reqHash,
      }
      if (h) payload.key = keyIdOf(h.key)
      if (grant) payload.tier = grant.tier
      // a stranger's request writes nothing (policy.t27 deny_recorded)
      if (denyRecorded(grant !== undefined))
        await this.tryAppend(EV_DENY, payload)
      return this.refusal(verdict)
    }
    this.nonces.set(h.nonce, nonceForgetAt(h.utcUnix))

    let body: Record<string, unknown>
    try {
      const parsed = req.body === '' ? {} : JSON.parse(req.body)
      body =
        parsed && typeof parsed === 'object'
          ? (parsed as Record<string, unknown>)
          : {}
    } catch {
      return { status: 400, body: { ok: false, error: 'the body is not JSON' } }
    }
    const c: Ctx = {
      state,
      now,
      op,
      key: h.key,
      keyId: keyIdOf(h.key),
      tier: grant.tier,
      scopes: grant.scopes,
      body,
      req: reqHash,
    }
    const allowed = opCode(c.tier, op)
    if (allowed !== CODE_OK) return this.deny(c, allowed)
    switch (op) {
      case OP_LEASE:
        return this.lease(c)
      case OP_RENEW:
        return this.renew(c)
      case OP_REVOKE:
        return this.revoke(c)
      case OP_NAMES:
        return this.names(c)
      case OP_PUT:
        return this.put(c, false)
      case OP_ROTATE:
        return this.put(c, true)
      case OP_IMPORT:
        return this.importStream(c)
      case OP_EXPORT:
        return this.exportAll(c)
      case OP_GRANT:
        return this.grant(c)
      case OP_AUDIT:
        return this.auditRead(c)
      case OP_BREAK_GLASS:
        return this.breakGlass(c)
      case OP_DEDUP:
        return this.dedup(c)
      case OP_MERGE:
        return this.merge(c)
      case OP_CLASS:
        return this.reclass(c)
      case OP_BIND:
        return this.bind(c)
    }
    return {
      status: 404,
      body: { ok: false, error: 'no such vault operation' },
    }
  }

  // --- answers and events --------------------------------------------------

  private refusal(
    code: number,
    extra: Record<string, unknown> = {},
  ): VaultResponse {
    return {
      status: STATUS_401.has(code) ? 401 : 403,
      body: {
        ok: false,
        code,
        reason: CODE_NAMES[code] ?? 'unknown',
        ...extra,
      },
    }
  }

  private async tryAppend(
    kind: number,
    payload: AuditPayload,
  ): Promise<boolean> {
    try {
      await this.opts.audit.append(kind, payload)
      return true
    } catch {
      return false
    }
  }

  /** A refused act: its deny event (best effort: a refusal delivers nothing), then the answer. */
  private async deny(
    c: Ctx,
    code: number,
    facts: { scope?: string; ids?: string[]; lease?: string } = {},
    extra: Record<string, unknown> = {},
  ): Promise<VaultResponse> {
    const payload: AuditPayload = {
      op: OP_NAMES_TEXT[c.op] as string,
      key: c.keyId,
      tier: c.tier,
      code,
      req: c.req,
    }
    if (facts.scope && textOk(facts.scope)) payload.scope = facts.scope
    const ids = tokens(facts.ids ?? [])
    if (ids.length > 0) payload.ids = ids
    if (facts.lease && textOk(facts.lease)) payload.lease = facts.lease
    await this.tryAppend(EV_DENY, payload)
    return this.refusal(code, extra)
  }

  /** The event an act must have before it happens (policy.t27 deliver_code). */
  private async record(
    c: Ctx,
    kind: number,
    payload: AuditPayload,
  ): Promise<VaultResponse | null> {
    const written = await this.tryAppend(kind, payload)
    const code = deliverCode(written)
    return code === CODE_OK ? null : this.deny(c, code)
  }

  // --- reading the state ---------------------------------------------------

  private resolve(state: VaultState, id: string): string {
    return state.aliases[id] ?? id
  }

  private known(state: VaultState, id: string): boolean {
    return id in state.secrets || id in state.aliases
  }

  private async open(row: SecretRow): Promise<Uint8Array> {
    return decryptWith(unb64(row.ct), this.identity, this.env)
  }

  private markRotated(state: VaultState, pick: (l: Lease) => boolean): void {
    for (const l of Object.values(state.leases)) if (pick(l)) l.rotated = true
  }

  /** A bundle of `ENV base64` lines, encrypted to the one-lease recipient. */
  private async bundle(
    state: VaultState,
    pairs: Array<{ env: string; id: string }>,
    recipient: string,
  ): Promise<string> {
    const lines: string[] = []
    for (const p of pairs) {
      const value = await this.open(state.secrets[p.id] as SecretRow)
      lines.push(`${p.env} ${b64(value)}`)
      value.fill(0)
    }
    const plain = new TextEncoder().encode(`${lines.join('\n')}\n`)
    lines.length = 0
    const ct = await encrypt(plain, [recipient], this.env)
    plain.fill(0)
    return b64(ct)
  }

  // --- the sweep (policy.t27 section 9) ------------------------------------

  private async sweep(state: VaultState, now: number): Promise<void> {
    if (!sweepDue(now - state.sweptAt, state.sweptAt > 0)) return
    for (const [id, row] of Object.entries(state.secrets)) {
      const ageDays = Math.floor(Math.max(0, now - row.rotatedAt) / DAY)
      const due = rotationDue(row.class, ageDays, row.brokenGlass)
      const last = state.alarms[id]
      if (!due) {
        delete state.alarms[id]
        continue
      }
      if (!alarmDue(due, last !== undefined, now - (last ?? now))) continue
      if (!textOk(id)) continue
      const written = await this.tryAppend(EV_ROTATION_DUE, {
        ids: [id],
        class: CLASS_NAMES[classOrOther(row.class)] as string,
        age_days: ageDays,
      })
      if (written) state.alarms[id] = now
    }
    for (const [id, l] of Object.entries(state.leases)) {
      if (leaseLive(now, l.expiresAt)) continue
      const payload: AuditPayload = {
        key: keyIdOf(l.key),
        lease: id,
        renewals: l.renewals,
      }
      if (l.scope) payload.scope = l.scope
      if (await this.tryAppend(EV_EXPIRE, payload)) delete state.leases[id]
    }
    state.sweptAt = now
    await this.save(state)
  }

  // --- workload acts -------------------------------------------------------

  private async lease(c: Ctx): Promise<VaultResponse> {
    const scope = str(c.body.scope)
    const recipient = str(c.body.recipient)
    if (!scopeOk(scope)) return this.deny(c, DENY_BAD_NAME)
    if (!isRecipient(recipient))
      throw new VaultError('recipient is not an age X25519 recipient')
    const granted = c.scopes.includes(scope)
    const scoped = scopeCode(c.tier, granted)
    if (scoped !== CODE_OK) return this.deny(c, scoped, { scope })
    const binds = c.state.bindings.filter((b) => b.scope === scope)
    let refused = 0
    let first = CODE_OK
    const pairs: Array<{ env: string; id: string }> = []
    for (const b of binds) {
      const id = this.resolve(c.state, b.id)
      const row = c.state.secrets[id]
      const code = leaseCode(
        c.tier,
        granted,
        row !== undefined,
        row?.class ?? CLASS_NONE,
      )
      if (code !== CODE_OK) {
        refused++
        if (first === CODE_OK) first = code
      } else pairs.push({ env: b.env, id })
    }
    const admitted = leaseAdmitted(binds.length, refused, first)
    if (admitted !== CODE_OK) return this.deny(c, admitted, { scope })

    const leaseId = randomBytes(16).toString('hex')
    const ids = unique(pairs.map((p) => p.id))
    const expires = expiresAt(c.now, false)
    const refusedEvent = await this.record(c, EV_LEASE, {
      key: c.keyId,
      tier: c.tier,
      scope,
      ids,
      count: pairs.length,
      lease: leaseId,
      expires,
      req: c.req,
    })
    if (refusedEvent) return refusedEvent
    const ct = await this.bundle(c.state, pairs, recipient)
    c.state.leases[leaseId] = {
      id: leaseId,
      key: c.key,
      scope,
      ids,
      issuedAt: c.now,
      expiresAt: expires,
      renewals: 0,
      revoked: false,
      rotated: false,
      breakGlass: false,
    }
    await this.save(c.state)
    return {
      status: 200,
      body: {
        ok: true,
        lease: leaseId,
        scope,
        expires,
        renewals_max: renewalsCap(false),
        names: pairs.map((p) => p.env),
        ct,
      },
    }
  }

  private async renew(c: Ctx): Promise<VaultResponse> {
    const id = str(c.body.lease)
    const l = c.state.leases[id]
    if (!l) return this.deny(c, DENY_LEASE_UNKNOWN, { lease: id })
    const code = renewCode({
      holder: l.key === c.key,
      revoked: l.revoked,
      rotated: l.rotated,
      now: c.now,
      expires: l.expiresAt,
      renewals: l.renewals,
      breakGlass: l.breakGlass,
    })
    if (code !== CODE_OK)
      return this.deny(c, code, { scope: l.scope, lease: id })
    const expires = expiresAt(c.now, l.breakGlass)
    const payload: AuditPayload = {
      key: c.keyId,
      lease: id,
      expires,
      renewals: l.renewals + 1,
    }
    if (l.scope) payload.scope = l.scope
    const refused = await this.record(c, EV_RENEW, payload)
    if (refused) return refused
    l.expiresAt = expires
    l.renewals++
    await this.save(c.state)
    return {
      status: 200,
      body: { ok: true, lease: id, expires, renewals: l.renewals },
    }
  }

  private async revoke(c: Ctx): Promise<VaultResponse> {
    const id = str(c.body.lease)
    const l = c.state.leases[id]
    if (!l) return this.deny(c, DENY_LEASE_UNKNOWN, { lease: id })
    const code = revokeCode(l.key === c.key, c.tier === TIER_OWNER)
    if (code !== CODE_OK) return this.deny(c, code, { lease: id })
    const payload: AuditPayload = { key: c.keyId, lease: id }
    if (l.scope) payload.scope = l.scope
    const refused = await this.record(c, EV_REVOKE, payload)
    if (refused) return refused
    l.revoked = true
    await this.save(c.state)
    return { status: 200, body: { ok: true, lease: id } }
  }

  private async names(c: Ctx): Promise<VaultResponse> {
    const asked = str(c.body.scope)
    const scopes = asked === '' ? c.scopes : [asked]
    if (asked !== '') {
      if (!scopeOk(asked)) return this.deny(c, DENY_BAD_NAME)
      const scoped = scopeCode(c.tier, c.scopes.includes(asked))
      if (scoped !== CODE_OK) return this.deny(c, scoped, { scope: asked })
    }
    return {
      status: 200,
      body: {
        ok: true,
        scopes: scopes.map((scope) => ({
          scope,
          names: c.state.bindings
            .filter((b) => b.scope === scope)
            .map((b) => b.env),
        })),
        recipients: {
          vault: c.state.vaultRecipient,
          recovery: c.state.recoveryRecipient,
        },
      },
    }
  }

  // --- owner acts ----------------------------------------------------------

  /** The checks a row must pass before the vault keeps it (policy.t27 row_code). */
  private async rowFacts(ct: Uint8Array): Promise<number> {
    const opens =
      ct.length > 0 && (await opensWithIdentity(ct, this.identity, this.env))
    return rowCode(opens, stanzaCount(ct))
  }

  private async put(c: Ctx, rotate: boolean): Promise<VaultResponse> {
    const id = str(c.body.id)
    const valid = idOk(id)
    const exists = valid && this.known(c.state, id)
    let code = putCode(valid, exists, rotate)
    if (code === CODE_OK && rotate)
      code = targetCode(exists, id in c.state.aliases)
    if (code !== CODE_OK) return this.deny(c, code, { ids: [id] })
    const input = str(c.body.input)
    if (!(INPUT_NAMES as readonly string[]).includes(input))
      throw new VaultError('input must be one of tty, stdin')
    const ct = unb64(str(c.body.ct))
    const row = await this.rowFacts(ct)
    if (row !== CODE_OK) return this.deny(c, row, { ids: [id] })

    const env = lastPart(id)
    const scopes = rotate ? [] : unique(strs(c.body.scopes))
    for (const scope of scopes) {
      if (!scopeOk(scope)) return this.deny(c, DENY_BAD_NAME, { ids: [id] })
      const taken = c.state.bindings.some(
        (b) => b.scope === scope && b.env === env,
      )
      const bound = bindCode(BIND_ADD, true, taken)
      if (bound !== CODE_OK) return this.deny(c, bound, { scope, ids: [id] })
    }
    const prior = c.state.secrets[id]
    const cls = rotate
      ? (prior as SecretRow).class
      : classOrOther(classIndex(c.body.class))
    const payload: AuditPayload = {
      key: c.keyId,
      ids: [id],
      class: CLASS_NAMES[cls] as string,
      input,
      ct: shortHash(ct),
    }
    if (scopes.length > 0) payload.scope = scopes
    const refused = await this.record(c, rotate ? EV_ROTATE : EV_PUT, payload)
    if (refused) return refused

    if (rotate && prior) {
      prior.ct = b64(ct)
      prior.rotatedAt = c.now
      prior.brokenGlass = false
      delete c.state.alarms[id]
      this.markRotated(c.state, (l) => l.ids.includes(id))
    } else {
      c.state.secrets[id] = {
        class: cls,
        ct: b64(ct),
        createdAt: c.now,
        rotatedAt: c.now,
        brokenGlass: false,
        provenance: [{ source: OWNER_SOURCE, service: null, name: env }],
      }
      for (const scope of scopes) c.state.bindings.push({ scope, env, id })
    }
    await this.save(c.state)
    return {
      status: 200,
      body: { ok: true, id, class: CLASS_NAMES[cls], scopes },
    }
  }

  private async importStream(c: Ctx): Promise<VaultResponse> {
    const format = (FORMAT_NAMES as readonly string[]).indexOf(
      str(c.body.format),
    )
    const source = str(c.body.source)
    if (format < 0 || !sourceOk(source)) return this.deny(c, DENY_BAD_NAME)
    const input = str(c.body.input)
    if (!(INPUT_NAMES as readonly string[]).includes(input))
      throw new VaultError('input must be one of tty, stdin')
    const scope = str(c.body.scope)
    if (scope !== '' && !scopeOk(scope)) return this.deny(c, DENY_BAD_NAME)
    const raw = records(c.body.entries)

    const judge = new ImportJudge(
      c.state,
      format,
      source,
      scope,
      classIndex(c.body.class),
      c.now,
      this,
    )
    for (const e of raw) await judge.entry(e)
    if (format === FMT_VAULT)
      judge.dump(
        records(c.body.aliases),
        records(c.body.binds),
        records(c.body.from),
      )
    const { planned, binds, aliases, refusals } = judge

    const verdict = importCode(
      c.tier,
      raw.length,
      refusals.length,
      refusals[0]?.code ?? CODE_OK,
    )
    if (verdict !== CODE_OK)
      return this.deny(
        c,
        verdict,
        { ids: refusals.map((r) => r.id).slice(0, 50) },
        { refused: refusals },
      )

    const skipped = tokens(strs(c.body.skipped)).filter((n) => nameOk(n))
    const classes = unique(planned.map((p) => p.row.class))
    const scopes = unique([
      ...planned.flatMap((p) => (p.bind ? [p.bind.scope] : [])),
      ...binds.map((b) => b.scope),
    ])
    const payload: AuditPayload = {
      key: c.keyId,
      ids: planned.map((p) => p.id),
      count: planned.length,
      input,
      source,
      format: FORMAT_NAMES[format] as string,
    }
    if (scopes.length > 0) payload.scope = scopes
    if (classes.length === 1)
      payload.class = CLASS_NAMES[classes[0] as number] as string
    if (skipped.length > 0) payload.skipped = skipped
    const refused = await this.record(c, EV_IMPORT, payload)
    if (refused) return refused

    for (const p of planned) {
      c.state.secrets[p.id] = p.row
      if (p.bind) c.state.bindings.push(p.bind)
    }
    for (const b of binds) c.state.bindings.push(b)
    for (const a of aliases) c.state.aliases[a.id] = a.canonical
    await this.save(c.state)
    return {
      status: 200,
      body: {
        ok: true,
        count: planned.length,
        ids: planned.map((p) => p.id),
        scopes,
        skipped,
      },
    }
  }

  /** Whether an id is a secret or an alias in the vault (for ImportJudge). */
  knownId(state: VaultState, id: string): boolean {
    return this.known(state, id)
  }

  /** The row checks of policy.t27 row_code (for ImportJudge). */
  rowCheck(ct: Uint8Array): Promise<number> {
    return this.rowFacts(ct)
  }

  private async exportAll(c: Ctx): Promise<VaultResponse> {
    const recipients = [c.state.recoveryRecipient]
    const code = exportCode(
      c.tier,
      recipients.length === 1 && recipients[0] === c.state.recoveryRecipient,
      recipients.length,
    )
    if (code !== CODE_OK) return this.deny(c, code)
    const ids = Object.keys(c.state.secrets).sort()
    const secrets = []
    for (const id of ids) {
      const row = c.state.secrets[id] as SecretRow
      secrets.push({
        id,
        classWord: CLASS_NAMES[classOrOther(row.class)] as string,
        rotatedAt: row.rotatedAt,
        brokenGlass: row.brokenGlass,
        value: await this.open(row),
      })
    }
    const plain = formatVaultDump({
      secrets,
      binds: c.state.bindings,
      aliases: Object.entries(c.state.aliases).map(([id, canonical]) => ({
        id,
        canonical,
      })),
      from: ids.flatMap((id) =>
        (c.state.secrets[id] as SecretRow).provenance.map((p) => ({
          id,
          ...p,
        })),
      ),
    })
    for (const s of secrets) s.value.fill(0)
    const ct = await encrypt(plain, recipients, this.env)
    plain.fill(0)
    const refused = await this.record(c, EV_EXPORT, {
      key: c.keyId,
      ids: tokens(ids),
      count: ids.length,
      ct: shortHash(ct),
    })
    if (refused) return refused
    return { status: 200, body: { ok: true, count: ids.length, ct: b64(ct) } }
  }

  private async grant(c: Ctx): Promise<VaultResponse> {
    const subject = str(c.body.subject)
    if (!isPublicHex(subject))
      throw new VaultError('subject is not an Ed25519 public key in hex')
    const tier = num(c.body.tier)
    const code = grantCode(tier)
    if (code !== CODE_OK) return this.deny(c, code)
    const scopes = unique(strs(c.body.scopes))
    for (const scope of scopes)
      if (!scopeOk(scope)) return this.deny(c, DENY_BAD_NAME)
    const payload: AuditPayload = {
      key: c.keyId,
      tier,
      subject: keyIdOf(subject),
    }
    if (scopes.length > 0) payload.scope = scopes
    const refused = await this.record(c, EV_GRANT, payload)
    if (refused) return refused
    c.state.grants[subject] = { tier, scopes, grantedAt: c.now }
    await this.save(c.state)
    return {
      status: 200,
      body: { ok: true, subject: keyIdOf(subject), tier, scopes },
    }
  }

  private async auditRead(c: Ctx): Promise<VaultResponse> {
    const rows = await this.opts.audit.read(
      num(c.body.after),
      auditPage(num(c.body.limit)),
    )
    const refused = await this.record(c, EV_AUDIT_READ, {
      key: c.keyId,
      count: rows.length,
    })
    if (refused) return refused
    return { status: 200, body: { ok: true, events: rows } }
  }

  private async breakGlass(c: Ctx): Promise<VaultResponse> {
    const ids = unique(strs(c.body.ids))
    const reason = str(c.body.reason)
    const recipient = str(c.body.recipient)
    const reasonBytes = Buffer.byteLength(reason)
    const code = breakGlassCode(c.tier, reasonBytes, ids.length)
    if (code !== CODE_OK) return this.deny(c, code, { ids })
    if (!isRecipient(recipient))
      throw new VaultError('recipient is not an age X25519 recipient')
    const pairs: Array<{ env: string; id: string }> = []
    for (const id of ids) {
      const resolved = this.resolve(c.state, id)
      const missing = targetCode(resolved in c.state.secrets, false)
      if (!idOk(id) || missing !== CODE_OK)
        return this.deny(c, idOk(id) ? missing : DENY_BAD_NAME, { ids: [id] })
      pairs.push({ env: lastPart(id), id: resolved })
    }
    if (unique(pairs.map((p) => p.env)).length !== pairs.length)
      return this.deny(c, DENY_DUPLICATE_IN_STREAM, { ids })
    const leaseId = randomBytes(16).toString('hex')
    const expires = expiresAt(c.now, true)
    const resolved = unique(pairs.map((p) => p.id))
    const refused = await this.record(c, EV_BREAK_GLASS, {
      key: c.keyId,
      ids: resolved,
      count: pairs.length,
      lease: leaseId,
      expires,
      reason_sha: shortHash(reason),
      reason_len: reasonBytes,
    })
    if (refused) return refused
    for (const id of resolved)
      (c.state.secrets[id] as SecretRow).brokenGlass = true
    const ct = await this.bundle(c.state, pairs, recipient)
    c.state.leases[leaseId] = {
      id: leaseId,
      key: c.key,
      scope: '',
      ids: resolved,
      issuedAt: c.now,
      expiresAt: expires,
      renewals: 0,
      revoked: false,
      rotated: false,
      breakGlass: true,
    }
    await this.save(c.state)
    return {
      status: 200,
      body: {
        ok: true,
        lease: leaseId,
        expires,
        renewals_max: renewalsCap(true),
        names: pairs.map((p) => p.env),
        ct,
      },
    }
  }

  private async reclass(c: Ctx): Promise<VaultResponse> {
    const ids = unique(strs(c.body.ids))
    const cls = classIndex(c.body.class)
    if (cls >= SECRET_CLASSES || ids.length === 0)
      return this.deny(c, DENY_BAD_NAME, { ids })
    for (const id of ids) {
      const code = targetCode(this.known(c.state, id), id in c.state.aliases)
      if (code !== CODE_OK) return this.deny(c, code, { ids: [id] })
    }
    const refused = await this.record(c, EV_CLASS, {
      key: c.keyId,
      ids,
      class: CLASS_NAMES[cls] as string,
    })
    if (refused) return refused
    for (const id of ids) (c.state.secrets[id] as SecretRow).class = cls
    this.markRotated(c.state, (l) => l.ids.some((id) => ids.includes(id)))
    await this.save(c.state)
    return { status: 200, body: { ok: true, ids, class: CLASS_NAMES[cls] } }
  }

  private async bind(c: Ctx): Promise<VaultResponse> {
    const mode = (BIND_MODE_NAMES as readonly string[]).indexOf(
      str(c.body.mode),
    )
    const scope = str(c.body.scope)
    const env = str(c.body.env)
    const id = str(c.body.id)
    if (mode < 0 || !scopeOk(scope) || !nameOk(env))
      return this.deny(c, DENY_BAD_NAME)
    if (mode !== BIND_REMOVE && !idOk(id))
      return this.deny(c, DENY_BAD_NAME, { scope })
    const at = c.state.bindings.findIndex(
      (b) => b.scope === scope && b.env === env,
    )
    const taken = at >= 0 ? (c.state.bindings[at] as Binding) : null
    const code = bindCode(
      mode,
      mode !== BIND_REMOVE && this.known(c.state, id),
      taken !== null,
    )
    if (code !== CODE_OK) return this.deny(c, code, { scope, ids: [id] })
    const same =
      taken !== null &&
      mode !== BIND_REMOVE &&
      this.resolve(c.state, taken.id) === this.resolve(c.state, id)
    const payload: AuditPayload = {
      key: c.keyId,
      scope,
      env,
      mode: BIND_MODE_NAMES[mode] as string,
      ids: [mode === BIND_REMOVE ? (taken as Binding).id : id],
    }
    const refused = await this.record(c, EV_BIND, payload)
    if (refused) return refused
    if (mode === BIND_ADD) c.state.bindings.push({ scope, env, id })
    else if (mode === BIND_REMOVE) c.state.bindings.splice(at, 1)
    else (c.state.bindings[at] as Binding).id = id
    if (bindRotates(mode, same))
      this.markRotated(c.state, (l) => l.scope === scope)
    await this.save(c.state)
    return {
      status: 200,
      body: { ok: true, scope, env, mode: BIND_MODE_NAMES[mode] },
    }
  }

  // --- dedup and merge (merge.t27 sections 3 and 4) ------------------------

  /** Each secret's value under a keyed HMAC whose key lives for this call only. */
  private async digests(
    state: VaultState,
    ids: string[],
  ): Promise<Map<string, Buffer>> {
    const key = randomBytes(DEDUP_KEY_BYTES)
    const out = new Map<string, Buffer>()
    try {
      for (const id of ids) {
        const value = await this.open(state.secrets[id] as SecretRow)
        out.set(id, createHmac('sha256', key).update(value).digest())
        value.fill(0)
      }
    } finally {
      key.fill(0)
    }
    return out
  }

  private namesOf(id: string, row: SecretRow): string[] {
    const names = row.provenance.map((p) => p.name)
    return unique(names.length > 0 ? names : [lastPart(id)])
  }

  private async dedup(c: Ctx): Promise<VaultResponse> {
    const ids = Object.keys(c.state.secrets).sort()
    const digest = await this.digests(c.state, ids)
    const rowOf = (id: string) => c.state.secrets[id] as SecretRow
    const sourcesOf = (members: string[], name?: string) =>
      unique(
        members.flatMap((id) =>
          rowOf(id)
            .provenance.filter((p) => name === undefined || p.name === name)
            .map((p) => p.source),
        ),
      )
    const servicesOf = (members: string[], name?: string) =>
      unique(
        members.flatMap((id) =>
          rowOf(id)
            .provenance.filter(
              (p) =>
                (name === undefined || p.name === name) && p.service !== null,
            )
            .map((p) => p.service as string),
        ),
      )

    // one value under several ids: grouped by the keyed digest, which never leaves this call
    const byValue = new Map<string, string[]>()
    for (const id of ids) {
      const d = (digest.get(id) as Buffer).toString('hex')
      byValue.set(d, [...(byValue.get(d) ?? []), id])
    }
    const groups = []
    for (const members of byValue.values()) {
      const names = unique(members.flatMap((id) => this.namesOf(id, rowOf(id))))
      const kind = valueGroupKind(members.length, names.length)
      if (!valueGroupShown(kind)) continue
      groups.push({
        kind: VALUE_GROUP_NAMES[kind] as string,
        names,
        ids: members,
        sources: sourcesOf(members),
        services: servicesOf(members),
        members: members.length,
      })
    }
    byValue.clear()

    // one name with several values
    const byName = new Map<string, string[]>()
    for (const id of ids)
      for (const name of this.namesOf(id, rowOf(id)))
        byName.set(name, [...(byName.get(name) ?? []), id])
    const conflicts = []
    for (const [name, members] of byName) {
      const values = unique(
        members.map((id) => (digest.get(id) as Buffer).toString('hex')),
      ).length
      const sources = sourcesOf(members, name)
      const kind = nameGroupKind(members.length, sources.length, values)
      if (!nameGroupShown(kind)) continue
      conflicts.push({
        kind: NAME_GROUP_NAMES[kind] as string,
        name,
        ids: members,
        sources,
        services: servicesOf(members, name),
        members: members.length,
        count: values,
      })
    }
    for (const d of digest.values()) d.fill(0)
    digest.clear()

    const report = { count: ids.length, groups, conflicts }
    checkReport(report)
    const refused = await this.record(c, EV_DEDUP, {
      key: c.keyId,
      count: ids.length,
      groups: groups.length,
      conflicts: conflicts.length,
    })
    if (refused) return refused
    return { status: 200, body: { ok: true, report } }
  }

  /** One plan line against the vault as it is (merge.t27 merge_code): a code, or what to do. */
  private async judgeLine(c: Ctx, line: PlanLineIn): Promise<number | Judged> {
    let code = idOk(line.canonical) ? CODE_OK : DENY_BAD_NAME
    for (const m of line.members) {
      if (code !== CODE_OK) break
      code = idOk(m)
        ? targetCode(this.known(c.state, m), m in c.state.aliases)
        : DENY_BAD_NAME
    }
    if (code !== CODE_OK) return code
    const digest = await this.digests(c.state, line.members)
    const first = digest.get(line.members[0] as string) as Buffer
    const valuesEqual = line.members.every((m) =>
      timingSafeEqual(first, digest.get(m) as Buffer),
    )
    for (const d of digest.values()) d.fill(0)
    const rows = line.members.map((m) => c.state.secrets[m] as SecretRow)
    const classes = unique(rows.map((r) => r.class))
    const cls = mergedClass(
      classes.length === 1,
      classes[0] ?? CLASS_NONE,
      line.classWord === null ? CLASS_NONE : classIndex(line.classWord),
    )
    code = mergeCode({
      tier: c.tier,
      members: line.members.length,
      valuesEqual,
      keeperNamed: line.keep !== null,
      keeperMember: line.keep !== null && line.members.includes(line.keep),
      canonicalOk: canonicalFree(
        this.known(c.state, line.canonical),
        line.members.includes(line.canonical),
      ),
      cls,
    })
    if (code !== CODE_OK) return code
    const byAge = [...line.members].sort(
      (a, b) =>
        (c.state.secrets[a] as SecretRow).rotatedAt -
        (c.state.secrets[b] as SecretRow).rotatedAt,
    )
    return { line, keeper: line.keep ?? (byAge[0] as string), valuesEqual, cls }
  }

  /** Apply one judged line: one ciphertext under the canonical id, the members its aliases. */
  private applyLine(state: VaultState, j: Judged): void {
    const { canonical, members } = j.line
    const rows = members.map((m) => state.secrets[m] as SecretRow)
    const keeperRow = state.secrets[j.keeper] as SecretRow
    const merged: SecretRow = {
      class: j.cls,
      ct: keeperRow.ct,
      createdAt: Math.min(...rows.map((r) => r.createdAt)),
      rotatedAt: mergedRotatedAt(
        j.valuesEqual,
        keeperRow.rotatedAt,
        Math.min(...rows.map((r) => r.rotatedAt)),
      ),
      brokenGlass: mergedDue(
        rows.some((r) => r.brokenGlass),
        j.valuesEqual,
        keeperRow.brokenGlass,
      ),
      provenance: rows.flatMap((r) => r.provenance),
    }
    const discarded = members.filter((m) =>
      mergeRevokes(j.valuesEqual, m === j.keeper),
    )
    for (const m of members) {
      delete state.secrets[m]
      delete state.alarms[m]
    }
    state.secrets[canonical] = merged
    for (const m of members) if (m !== canonical) state.aliases[m] = canonical
    // ALIAS_DEPTH_MAX is one: an alias of a member now names the canonical id
    for (const [alias, target] of Object.entries(state.aliases))
      if (members.includes(target) && target !== canonical)
        state.aliases[alias] = canonical
    this.markRotated(state, (l) => l.ids.some((id) => discarded.includes(id)))
  }

  private async merge(c: Ctx): Promise<VaultResponse> {
    const lines: PlanLineIn[] = records(c.body.lines).map((l) => ({
      canonical: str(l.canonical),
      members: unique(strs(l.members)),
      keep: typeof l.keep === 'string' ? l.keep : null,
      classWord: typeof l.classWord === 'string' ? l.classWord : null,
    }))
    const named = lines.flatMap((l) => unique([l.canonical, ...l.members]))
    const plan = planCode(
      c.tier,
      lines.length,
      unique(named).length !== named.length,
    )
    if (plan !== CODE_OK) return this.deny(c, plan)

    const judged: Judged[] = []
    const refusals: Array<{ line: number; code: number; reason: string }> = []
    for (const [i, line] of lines.entries()) {
      const j = await this.judgeLine(c, line)
      if (typeof j === 'number')
        refusals.push({
          line: i + 1,
          code: j,
          reason: CODE_NAMES[j] ?? 'unknown',
        })
      else judged.push(j)
    }
    if (refusals.length > 0)
      return this.deny(
        c,
        refusals[0]?.code ?? DENY_BAD_NAME,
        { ids: tokens(named).slice(0, 50) },
        { refused: refusals },
      )

    for (const j of judged) {
      const keeperRow = c.state.secrets[j.keeper] as SecretRow
      const discarded = j.line.members.filter((m) =>
        mergeRevokes(j.valuesEqual, m === j.keeper),
      )
      const payload: AuditPayload = {
        key: c.keyId,
        ids: j.line.members,
        class: CLASS_NAMES[j.cls] as string,
        ct: shortHash(unb64(keeperRow.ct)),
        canonical: j.line.canonical,
        aliases: j.line.members.filter((m) => m !== j.line.canonical),
      }
      if (discarded.length > 0) payload.discarded = discarded
      const refused = await this.record(c, EV_MERGE, payload)
      if (refused) return refused
    }
    for (const j of judged) this.applyLine(c.state, j)
    await this.save(c.state)
    return {
      status: 200,
      body: {
        ok: true,
        merged: judged.map((j) => ({
          canonical: j.line.canonical,
          aliases: j.line.members.filter((m) => m !== j.line.canonical),
        })),
      },
    }
  }
}

interface PlanLineIn {
  canonical: string
  members: string[]
  keep: string | null
  classWord: string | null
}

interface Judged {
  line: PlanLineIn
  keeper: string
  valuesEqual: boolean
  cls: number
}

const records = (v: unknown): Record<string, unknown>[] =>
  Array.isArray(v)
    ? (v as Record<string, unknown>[]).filter((x) => x && typeof x === 'object')
    : []

interface Planned {
  id: string
  row: SecretRow
  bind: Binding | null
}

/**
 * One import, judged entry by entry before anything is kept (merge.t27
 * section 2): the id, a second copy in the stream, a copy in the vault, the
 * row, and the binding each entry asks for. For the vault's own export also
 * its aliases, bindings and provenance. Every refusal is kept, by id.
 */
class ImportJudge {
  readonly planned: Planned[] = []
  readonly binds: Binding[] = []
  readonly aliases: Array<{ id: string; canonical: string }> = []
  readonly refusals: Array<{ id: string; code: number; reason: string }> = []
  private readonly seen = new Set<string>()
  private readonly taken: Set<string>

  constructor(
    private readonly state: VaultState,
    private readonly format: number,
    private readonly source: string,
    private readonly scope: string,
    private readonly named: number,
    private readonly now: number,
    private readonly vault: Vault,
  ) {
    this.taken = new Set(state.bindings.map((b) => `${b.scope}\n${b.env}`))
  }

  private refuse(id: string, code: number): void {
    this.refusals.push({
      id: textOk(id) ? id : '-',
      code,
      reason: CODE_NAMES[code] ?? 'unknown',
    })
  }

  private idOf(name: string, service: string, given: string): string {
    if (this.format === FMT_VAULT) return given
    return hasService(this.format)
      ? `${this.source}/${service}/${name}`
      : `${this.source}/${name}`
  }

  /** A binding asked for under the format's rule; its code. */
  private claim(bind: Binding, exists: boolean): number {
    const key = `${bind.scope}\n${bind.env}`
    const code = bindCode(BIND_ADD, exists, this.taken.has(key))
    this.taken.add(key)
    return code
  }

  async entry(e: Record<string, unknown>): Promise<void> {
    const name = str(e.name)
    const service = str(e.service)
    const id = this.idOf(name, service, str(e.id))
    const valid =
      idOk(id) &&
      (this.format === FMT_VAULT || nameOk(name)) &&
      (!hasService(this.format) || scopeOk(service))
    let code = entryCode(
      valid,
      this.seen.has(id),
      valid && this.vault.knownId(this.state, id),
    )
    this.seen.add(id)
    const ct = unb64(str(e.ct))
    if (code === CODE_OK) code = await this.vault.rowCheck(ct)
    const bindScope =
      this.format === FMT_RAILWAY_KV
        ? service
        : this.format === FMT_VAULT
          ? ''
          : this.scope
    const bind = bindScope === '' ? null : { scope: bindScope, env: name, id }
    if (code === CODE_OK && bind) code = this.claim(bind, true)
    if (code !== CODE_OK) return this.refuse(id, code)
    const vault = this.format === FMT_VAULT
    this.planned.push({
      id,
      bind,
      row: {
        class: importClass(vault ? classIndex(e.class) : this.named),
        ct: b64(ct),
        createdAt: this.now,
        rotatedAt: importedRotatedAt(this.format, num(e.rotatedAt), this.now),
        brokenGlass: vault && e.brokenGlass === true,
        provenance: vault
          ? []
          : [
              {
                source: this.source,
                service: hasService(this.format) ? service : null,
                name,
              },
            ],
      },
    })
  }

  /** The vault's export: aliases first (a binding may name one), then bindings, then provenance. */
  dump(
    aliases: Record<string, unknown>[],
    binds: Record<string, unknown>[],
    from: Record<string, unknown>[],
  ): void {
    const ids = new Set(this.planned.map((p) => p.id))
    for (const a of aliases) {
      const alias = { id: str(a.id), canonical: str(a.canonical) }
      const free =
        idOk(alias.id) &&
        !ids.has(alias.id) &&
        !this.vault.knownId(this.state, alias.id)
      const target =
        ids.has(alias.canonical) || alias.canonical in this.state.secrets
      if (!free)
        this.refuse(alias.id, idOk(alias.id) ? DENY_EXISTS : DENY_BAD_NAME)
      else if (!target) this.refuse(alias.id, DENY_MISSING)
      else {
        this.aliases.push(alias)
        ids.add(alias.id)
      }
    }
    for (const b of binds) {
      const bind = { scope: str(b.scope), env: str(b.env), id: str(b.id) }
      const exists = ids.has(bind.id) || this.vault.knownId(this.state, bind.id)
      let code =
        scopeOk(bind.scope) && nameOk(bind.env) && idOk(bind.id)
          ? CODE_OK
          : DENY_BAD_NAME
      if (code === CODE_OK) code = this.claim(bind, exists)
      if (code === CODE_OK) this.binds.push(bind)
      else this.refuse(bind.id, code)
    }
    const byId = new Map(this.planned.map((p) => [p.id, p.row]))
    for (const f of from) {
      const row = byId.get(str(f.id))
      const service =
        f.service === null || f.service === undefined ? null : str(f.service)
      const p: Provenance = {
        source: str(f.source),
        service,
        name: str(f.name),
      }
      if (
        row &&
        sourceOk(p.source) &&
        nameOk(p.name) &&
        (service === null || scopeOk(service))
      )
        row.provenance.push(p)
    }
  }
}

/** A report carries only REPORT_FIELDS keys, tokens and counts (merge.t27 section 3). */
export function checkReport(value: unknown, key = 'report'): void {
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value) || value < 0)
      throw new VaultError(`report: ${key} is not a count`)
    return
  }
  if (typeof value === 'string') {
    if (!textOk(value))
      throw new VaultError(`report: a ${key} text is not a token`)
    return
  }
  if (Array.isArray(value)) {
    for (const v of value) checkReport(v, key)
    return
  }
  if (value && typeof value === 'object') {
    for (const [k, v] of Object.entries(value)) {
      if (!(REPORT_FIELDS as readonly string[]).includes(k))
        throw new VaultError(`report: ${k} is not a report field`)
      checkReport(v, k)
    }
    return
  }
  throw new VaultError(`report: ${key} has no place in a report`)
}

/**
 * A new vault in `dir`: the vault's identity (age-keygen, mode 0600), and a
 * store whose one grant is the owner's key at TIER_OWNER with no scope.
 */
export async function initVault(opts: {
  dir: string
  store: VaultStore
  audit: AuditSink
  recoveryRecipient: string
  ownerPublicHex: string
  now?: number
  env?: Record<string, string | undefined>
}): Promise<{ vaultRecipient: string }> {
  const { keygenToFile } = await import('./vault-age')
  const { mkdir } = await import('node:fs/promises')
  if (!isRecipient(opts.recoveryRecipient))
    throw new VaultError(
      'the recovery recipient is not an age X25519 recipient',
    )
  if (!isPublicHex(opts.ownerPublicHex))
    throw new VaultError('the owner key is not an Ed25519 public key')
  if (await opts.store.load())
    throw new VaultError('a vault is already initialised here')
  await mkdir(opts.dir, { recursive: true, mode: 0o700 })
  const vaultRecipient = await keygenToFile(
    join(opts.dir, IDENTITY_FILE),
    opts.env ?? process.env,
  )
  const now = opts.now ?? Math.floor(Date.now() / 1000)
  const state = emptyState(vaultRecipient, opts.recoveryRecipient)
  const keyId = keyIdOf(opts.ownerPublicHex)
  await opts.audit.append(EV_GRANT, {
    key: keyId,
    tier: TIER_OWNER,
    subject: keyId,
  })
  state.grants[opts.ownerPublicHex] = {
    tier: TIER_OWNER,
    scopes: [],
    grantedAt: now,
  }
  await opts.store.save(state)
  return { vaultRecipient }
}

export {
  DENY_ALIAS,
  DENY_BAD_CIPHERTEXT,
  DENY_EXISTS,
  DENY_MISSING,
  DENY_NO_AUDIT,
}
