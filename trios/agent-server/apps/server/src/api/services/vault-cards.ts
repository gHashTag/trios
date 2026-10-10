/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's cards (gHashTag/trios#1759): every decision the vault and its
 * CLI make, called here and nowhere restated. The specs are gHashTag/t27
 * specs/vault/*.t27, vendored byte-identical under specs/vault with their wasm
 * (specs/PIN); the constants are `t27c gen-ts` of the same specs
 * (vault-*-card.gen.ts).
 *
 *   policy.t27  classes and tiers (host.t27 eligible), names and ids, the
 *               signed request, who may do what, leasing, TTL and renewal,
 *               delivery, put and rotate, rotation age and the sweep, export,
 *               break-glass, bindings
 *   audit.t27   event kinds, the fields each may carry, text and hashes
 *   merge.t27   the import formats, copies and conflicts, merging, the plan,
 *               the vault's own export format
 *
 * NO VALUE EVER ENTERS A CARD: the texts written into a card's memory are
 * names, ids, scopes, source labels and words. A card takes facts (a flag, a
 * count, a time) and answers a code.
 */

import { type CardWasm, flag, loadCardWasm, u32, u64 } from './queen-card-wasm'

export const VAULT_CARDS = {
  policy: 'vault/policy.wasm',
  audit: 'vault/audit.wasm',
  merge: 'vault/merge.wasm',
} as const

const policy = (): CardWasm => loadCardWasm(VAULT_CARDS.policy)
const audit = (): CardWasm => loadCardWasm(VAULT_CARDS.audit)
const merge = (): CardWasm => loadCardWasm(VAULT_CARDS.merge)
const yes = (n: number | bigint): boolean => Number(n) !== 0
const b8 = (n: number): number => n & 0xff
const code = (n: number | bigint): number => Number(n) >>> 0

const textCall = (card: CardWasm, fn: string, text: string): boolean => {
  const [b] = card.put(text)
  const at = b as { at: number; len: number }
  return yes(card.call(fn, at.at, at.len))
}

// --- policy.t27 -------------------------------------------------------------

export const classOrOther = (cls: number): number =>
  code(policy().call('class_or_other', b8(cls)))
export const mayHoldSecret = (tier: number): boolean =>
  yes(policy().call('may_hold_secret', b8(tier)))
export const classReaches = (cls: number, tier: number): boolean =>
  yes(policy().call('class_reaches', b8(cls), b8(tier)))
export const nameOk = (text: string): boolean =>
  textCall(policy(), 'name_ok', text)
export const scopeOk = (text: string): boolean =>
  textCall(policy(), 'scope_ok', text)
export const sourceOk = (text: string): boolean =>
  textCall(policy(), 'source_ok', text)
export const idOk = (text: string): boolean => textCall(policy(), 'id_ok', text)

export interface RequestFacts {
  keyKnown: boolean
  signatureOk: boolean
  skewSeconds: number
  nonceBytes: number
  nonceSeen: boolean
  cacheSize: number
}

export const requestCode = (f: RequestFacts): number =>
  code(
    policy().call(
      'request_code',
      flag(f.keyKnown),
      flag(f.signatureOk),
      u32(f.skewSeconds),
      u32(f.nonceBytes),
      flag(f.nonceSeen),
      u32(f.cacheSize),
    ),
  )

export const denyRecorded = (keyKnown: boolean): boolean =>
  yes(policy().call('deny_recorded', flag(keyKnown)))

export const nonceForgetAt = (utcUnix: number): number =>
  Number(policy().call64('nonce_forget_at', u64(utcUnix)))

export const opCode = (tier: number, op: number): number =>
  code(policy().call('op_code', b8(tier), b8(op)))

export const grantCode = (tier: number): number =>
  code(policy().call('grant_code', b8(tier)))

export const scopeCode = (tier: number, granted: boolean): number =>
  code(policy().call('scope_code', b8(tier), flag(granted)))

export const leaseCode = (
  tier: number,
  granted: boolean,
  bound: boolean,
  cls: number,
): number =>
  code(
    policy().call('lease_code', b8(tier), flag(granted), flag(bound), b8(cls)),
  )

export const leaseAdmitted = (
  bound: number,
  refused: number,
  firstRefusal: number,
): number =>
  code(
    policy().call('lease_admitted', u32(bound), u32(refused), b8(firstRefusal)),
  )

export const leaseTtl = (breakGlass: boolean): number =>
  Number(policy().call64('lease_ttl', flag(breakGlass)))

export const renewalsCap = (breakGlass: boolean): number =>
  code(policy().call('renewals_cap', flag(breakGlass)))

export const expiresAt = (now: number, breakGlass: boolean): number =>
  Number(policy().call64('expires_at', u64(now), flag(breakGlass)))

export const leaseLive = (now: number, expires: number): boolean =>
  yes(policy().call64('lease_live', u64(now), u64(expires)))

export interface RenewFacts {
  holder: boolean
  revoked: boolean
  rotated: boolean
  now: number
  expires: number
  renewals: number
  breakGlass: boolean
}

export const renewCode = (f: RenewFacts): number =>
  code(
    policy().call64(
      'renew_code',
      flag(f.holder),
      flag(f.revoked),
      flag(f.rotated),
      u64(f.now),
      u64(f.expires),
      u32(f.renewals),
      flag(f.breakGlass),
    ),
  )

export const revokeCode = (holder: boolean, owner: boolean): number =>
  code(policy().call('revoke_code', flag(holder), flag(owner)))

export const renewDue = (heldSeconds: number, breakGlass: boolean): boolean =>
  yes(policy().call64('renew_due', u64(heldSeconds), flag(breakGlass)))

export const childMustStop = (refused: boolean, leftSeconds: number): boolean =>
  yes(policy().call64('child_must_stop', flag(refused), u64(leftSeconds)))

export const deliverCode = (auditWritten: boolean): number =>
  code(policy().call('deliver_code', flag(auditWritten)))

export const inputCode = (isTty: boolean, stdinFlag: boolean): number =>
  code(policy().call('input_code', flag(isTty), flag(stdinFlag)))

export const putCode = (
  idValid: boolean,
  exists: boolean,
  rotate: boolean,
): number =>
  code(policy().call('put_code', flag(idValid), flag(exists), flag(rotate)))

export const rowCode = (opens: boolean, stanzas: number): number =>
  code(policy().call('row_code', flag(opens), u32(stanzas)))

export const sweepDue = (secondsSince: number, sweptBefore: boolean): boolean =>
  yes(policy().call('sweep_due', u32(secondsSince), flag(sweptBefore)))

export const rotationDue = (
  cls: number,
  ageDays: number,
  brokenGlass: boolean,
): boolean =>
  yes(policy().call('rotation_due', b8(cls), u32(ageDays), flag(brokenGlass)))

export const alarmDue = (
  due: boolean,
  alarmedBefore: boolean,
  secondsSince: number,
): boolean =>
  yes(
    policy().call(
      'alarm_due',
      flag(due),
      flag(alarmedBefore),
      u32(secondsSince),
    ),
  )

export const exportCode = (
  tier: number,
  toRecovery: boolean,
  recipients: number,
): number =>
  code(
    policy().call('export_code', b8(tier), flag(toRecovery), u32(recipients)),
  )

export const breakGlassCode = (
  tier: number,
  reasonBytes: number,
  names: number,
): number =>
  code(
    policy().call('break_glass_code', b8(tier), u32(reasonBytes), u32(names)),
  )

export const bindCode = (
  mode: number,
  idExists: boolean,
  taken: boolean,
): number =>
  code(policy().call('bind_code', b8(mode), flag(idExists), flag(taken)))

export const bindRotates = (mode: number, sameSecret: boolean): boolean =>
  yes(policy().call('bind_rotates', b8(mode), flag(sameSecret)))

export interface StartFacts {
  flagOn: boolean
  database: boolean
  tool: boolean
  identitySet: boolean
  identityOk: boolean
  recoveryOk: boolean
  ownersOk: boolean
  sameIdentity: boolean
  sameRecovery: boolean
}

export const startCode = (f: StartFacts): number =>
  code(
    policy().call(
      'start_code',
      flag(f.flagOn),
      flag(f.database),
      flag(f.tool),
      flag(f.identitySet),
      flag(f.identityOk),
      flag(f.recoveryOk),
      flag(f.ownersOk),
      flag(f.sameIdentity),
      flag(f.sameRecovery),
    ),
  )

export const serves = (start: number): boolean =>
  yes(policy().call('serves', b8(start)))

// --- audit.t27 --------------------------------------------------------------

export const eventCode = (kind: number, fields: number, c: number): number =>
  code(audit().call('event_code', b8(kind), fields >>> 0, b8(c)))
export const textOk = (text: string): boolean =>
  textCall(audit(), 'text_ok', text)
export const hashOk = (text: string): boolean =>
  textCall(audit(), 'hash_ok', text)
export const auditPage = (asked: number): number =>
  code(audit().call('audit_page', u32(asked)))

// --- merge.t27 --------------------------------------------------------------

export interface LineFacts {
  blank: boolean
  header: boolean
  comment: boolean
  assignment: boolean
  afterEntry: boolean
  openBlock: boolean
}

export const lineKind = (format: number, f: LineFacts): number =>
  code(
    merge().call(
      'line_kind',
      b8(format),
      flag(f.blank),
      flag(f.header),
      flag(f.comment),
      flag(f.assignment),
      flag(f.afterEntry),
      flag(f.openBlock),
    ),
  )

export const streamEndsClean = (openBlock: boolean): boolean =>
  yes(merge().call('stream_ends_clean', flag(openBlock)))
export const platformName = (name: string): boolean =>
  textCall(merge(), 'platform_name', name)
export const skipOnImport = (format: number, platform: boolean): boolean =>
  yes(merge().call('skip_on_import', b8(format), flag(platform)))
export const entryCode = (
  idValid: boolean,
  seen: boolean,
  exists: boolean,
): number =>
  code(merge().call('entry_code', flag(idValid), flag(seen), flag(exists)))
export const importCode = (
  tier: number,
  entries: number,
  refused: number,
  firstRefusal: number,
): number =>
  code(
    merge().call(
      'import_code',
      b8(tier),
      u32(entries),
      u32(refused),
      b8(firstRefusal),
    ),
  )
export const hasService = (format: number): boolean =>
  yes(merge().call('has_service', b8(format)))
export const importClass = (named: number): number =>
  code(merge().call('import_class', b8(named)))
export const importedRotatedAt = (
  format: number,
  recorded: number,
  now: number,
): number =>
  Number(
    merge().call64('imported_rotated_at', b8(format), u64(recorded), u64(now)),
  )
export const valueGroupKind = (members: number, names: number): number =>
  code(merge().call('value_group_kind', u32(members), u32(names)))
export const nameGroupKind = (
  members: number,
  sources: number,
  values: number,
): number =>
  code(merge().call('name_group_kind', u32(members), u32(sources), u32(values)))
export const valueGroupShown = (kind: number): boolean =>
  yes(merge().call('value_group_shown', b8(kind)))
export const nameGroupShown = (kind: number): boolean =>
  yes(merge().call('name_group_shown', b8(kind)))
export const mergedClass = (
  allAgree: boolean,
  agreed: number,
  planned: number,
): number =>
  code(merge().call('merged_class', flag(allAgree), b8(agreed), b8(planned)))
export const canonicalFree = (taken: boolean, member: boolean): boolean =>
  yes(merge().call('canonical_free', flag(taken), flag(member)))

export interface MergeFacts {
  tier: number
  members: number
  valuesEqual: boolean
  keeperNamed: boolean
  keeperMember: boolean
  canonicalOk: boolean
  cls: number
}

export const mergeCode = (f: MergeFacts): number =>
  code(
    merge().call(
      'merge_code',
      b8(f.tier),
      u32(f.members),
      flag(f.valuesEqual),
      flag(f.keeperNamed),
      flag(f.keeperMember),
      flag(f.canonicalOk),
      b8(f.cls),
    ),
  )

export const targetCode = (exists: boolean, isAlias: boolean): number =>
  code(merge().call('target_code', flag(exists), flag(isAlias)))
export const mergedRotatedAt = (
  valuesEqual: boolean,
  keeperAt: number,
  oldestAt: number,
): number =>
  Number(
    merge().call64(
      'merged_rotated_at',
      flag(valuesEqual),
      u64(keeperAt),
      u64(oldestAt),
    ),
  )
export const mergedDue = (
  anyBroken: boolean,
  valuesEqual: boolean,
  keeperBroken: boolean,
): boolean =>
  yes(
    merge().call(
      'merged_due',
      flag(anyBroken),
      flag(valuesEqual),
      flag(keeperBroken),
    ),
  )
export const mergeRevokes = (
  valuesEqual: boolean,
  isKeeper: boolean,
): boolean =>
  yes(merge().call('merge_revokes', flag(valuesEqual), flag(isKeeper)))
export const planLineKind = (
  blank: boolean,
  comment: boolean,
  wordOk: boolean,
  equalsOk: boolean,
  members: number,
): number =>
  code(
    merge().call(
      'plan_line_kind',
      flag(blank),
      flag(comment),
      flag(wordOk),
      flag(equalsOk),
      u32(members),
    ),
  )
export const planCode = (
  tier: number,
  lines: number,
  overlap: boolean,
): number =>
  code(merge().call('plan_code', b8(tier), u32(lines), flag(overlap)))
export const vaultLineKind = (
  first: boolean,
  domainOk: boolean,
  blank: boolean,
  record: number,
  fields: number,
): number =>
  code(
    merge().call(
      'vault_line_kind',
      flag(first),
      flag(domainOk),
      flag(blank),
      b8(record),
      u32(fields),
    ),
  )
