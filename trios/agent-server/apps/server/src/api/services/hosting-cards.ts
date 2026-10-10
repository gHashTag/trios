/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The self-hosting cards (gHashTag/trios#1756): every decision the Queen and
 * the host agent make about hosts, placement, receipts and credit, called
 * here and nowhere restated. The specs are gHashTag/t27 specs/hosting/*.t27,
 * vendored byte-identical under specs/hosting with their wasm (specs/PIN).
 *
 *   host.t27       tiers, eligibility, capacity, the netlink lease, freshness,
 *                  strikes, the toolchain action, isolation
 *   placement.t27  fit, spread, k replicas, the lease that lapses, which
 *                  host runs which half of a lab row
 *   proof.t27      the shard's verdict and reason, the receipt's checks,
 *                  k-of-n agreement
 *   credit.t27     credit, strikes, the ledger, settlement (off)
 *   row.t27        the lab row's two halves, the t27b half's verdict, and
 *                  how a row is assembled from both
 *
 * WHY A LOCATOR: the Queen reads the cards from specs/ on disk; the compiled
 * host agent carries them inside its binary (`bun build --compile`) and points
 * the locator at the embedded files once, before the first call.
 */

import { basename, dirname } from 'node:path'
import { type CardWasm, flag, loadCardWasm, u32, u64 } from './queen-card-wasm'

export const HOSTING_CARDS = {
  host: 'hosting/host.wasm',
  proof: 'hosting/proof.wasm',
  placement: 'hosting/placement.wasm',
  credit: 'hosting/credit.wasm',
  row: 'hosting/row.wasm',
} as const

export type HostingCardName = keyof typeof HOSTING_CARDS

let locate = (name: HostingCardName): CardWasm =>
  loadCardWasm(HOSTING_CARDS[name])

/** The host agent's binary: each card at the path its bundler gave it. */
export function useHostingCardFiles(
  files: Record<HostingCardName, string>,
): void {
  locate = (name) => loadCardWasm(basename(files[name]), dirname(files[name]))
}

const host = () => locate('host')
const proof = () => locate('proof')
const placement = () => locate('placement')
const credit = () => locate('credit')
const row = () => locate('row')
const yes = (n: number | bigint) => Number(n) !== 0

// --- host.t27 ---------------------------------------------------------------

export const tierOf = (claimed: number, allowed: number): number =>
  host().call('tier_of', claimed & 0xff, allowed & 0xff) >>> 0

export const eligible = (
  tier: number,
  workloadClass: number,
  holdsSecret: boolean,
  holdsPersonal: boolean,
): boolean =>
  yes(
    host().call(
      'eligible',
      tier & 0xff,
      workloadClass & 0xff,
      flag(holdsSecret),
      flag(holdsPersonal),
    ),
  )

export const slotsAdmitted = (tier: number, claimed: number): number =>
  host().call('slots_admitted', tier & 0xff, u32(claimed)) >>> 0

export const hasRoom = (slots: number, running: number): boolean =>
  yes(host().call('has_room', u32(slots), u32(running)))

export const hostUp = (beatAgeSeconds: number): boolean =>
  yes(host().call('host_up', u32(beatAgeSeconds)))

export const nextIncarnation = (prev: number): number =>
  Number(host().call64('next_incarnation', u64(prev)))

export const hostWriteAdmitted = (
  rowInc: number,
  writerInc: number,
  beatAgeSeconds: number,
): boolean =>
  yes(
    host().call64(
      'host_write_admitted',
      u64(rowInc),
      u64(writerInc),
      u32(beatAgeSeconds),
    ),
  )

export const mustRegisterAgain = (
  refused: boolean,
  secondsSinceConfirmed: number,
): boolean =>
  yes(
    host().call(
      'must_register_again',
      flag(refused),
      u32(secondsSinceConfirmed),
    ),
  )

export const requestFresh = (skewSeconds: number): boolean =>
  yes(host().call('request_fresh', u32(skewSeconds)))

export const suspended = (strikes: number): boolean =>
  yes(host().call('suspended', u32(strikes)))

export const toolchainAction = (pinned: boolean, localGiven: boolean): number =>
  host().call('toolchain_action', flag(pinned), flag(localGiven)) >>> 0

export const isolationRequired = (
  workloadClass: number,
  holdsSecret: boolean,
  holdsPersonal: boolean,
): number =>
  host().call(
    'isolation_required',
    workloadClass & 0xff,
    flag(holdsSecret),
    flag(holdsPersonal),
  ) >>> 0

export const isolationMeets = (have: number, need: number): boolean =>
  yes(host().call('isolation_meets', have & 0xff, need & 0xff))

export interface SandboxFacts {
  macos: boolean
  hasSandboxExec: boolean
  hasBwrap: boolean
  hasUnshare: boolean
  turnedOff: boolean
}

export const sandboxChoice = (f: SandboxFacts): number =>
  host().call(
    'sandbox_choice',
    flag(f.macos),
    flag(f.hasSandboxExec),
    flag(f.hasBwrap),
    flag(f.hasUnshare),
    flag(f.turnedOff),
  ) >>> 0

export const isolationOf = (
  sandbox: number,
  connectFailed: boolean,
  writeFailed: boolean,
): number =>
  host().call(
    'isolation_of',
    sandbox & 0xff,
    flag(connectFailed),
    flag(writeFailed),
  ) >>> 0

export const downloadKept = (
  digestMatches: boolean,
  sizeMatches: boolean,
): boolean =>
  yes(host().call('download_kept', flag(digestMatches), flag(sizeMatches)))

// --- placement.t27 ----------------------------------------------------------

export interface TakeFacts {
  tier: number
  workloadClass: number
  holdsSecret: boolean
  holdsPersonal: boolean
  beatAgeSeconds: number
  strikes: number
  slots: number
  running: number
}

export const mayTake = (f: TakeFacts): boolean =>
  yes(
    placement().call(
      'may_take',
      f.tier & 0xff,
      f.workloadClass & 0xff,
      flag(f.holdsSecret),
      flag(f.holdsPersonal),
      u32(f.beatAgeSeconds),
      u32(f.strikes),
      u32(f.slots),
      u32(f.running),
    ),
  )

export const originBlocks = (
  tier: number,
  otherTier: number,
  sameOrigin: number,
): boolean =>
  yes(
    placement().call(
      'origin_blocks',
      tier & 0xff,
      otherTier & 0xff,
      u32(sameOrigin),
    ),
  )

export const spreadOk = (
  tier: number,
  leasedBefore: boolean,
  sameOriginPublic: number,
): boolean =>
  yes(
    placement().call(
      'spread_ok',
      tier & 0xff,
      flag(leasedBefore),
      u32(sameOriginPublic),
    ),
  )

export const openReplicas = (
  wanted: number,
  outstanding: number,
  votes: number,
): number =>
  placement().call(
    'open_replicas',
    u32(wanted),
    u32(outstanding),
    u32(votes),
  ) >>> 0

export const exhausted = (leasesIssued: number, outstanding: number): boolean =>
  yes(placement().call('exhausted', u32(leasesIssued), u32(outstanding)))

export const places = (
  mayTakeIt: boolean,
  spreadIsOk: boolean,
  open: number,
  leasesIssued: number,
): boolean =>
  yes(
    placement().call(
      'places',
      flag(mayTakeIt),
      flag(spreadIsOk),
      u32(open),
      u32(leasesIssued),
    ),
  )

export const jobFirst = (
  aVotes: number,
  aAgeSeconds: number,
  bVotes: number,
  bAgeSeconds: number,
): boolean =>
  yes(
    placement().call(
      'job_first',
      u32(aVotes),
      u32(aAgeSeconds),
      u32(bVotes),
      u32(bAgeSeconds),
    ),
  )

export const halfFits = (half: number, arch: number): boolean =>
  yes(placement().call('half_fits', half & 0xff, arch & 0xff))

export interface HalfTakeFacts extends TakeFacts {
  half: number
  arch: number
  isolation: number
}

export const mayTakeHalf = (f: HalfTakeFacts): boolean =>
  yes(
    placement().call(
      'may_take_half',
      f.half & 0xff,
      f.arch & 0xff,
      f.isolation & 0xff,
      f.tier & 0xff,
      f.workloadClass & 0xff,
      flag(f.holdsSecret),
      flag(f.holdsPersonal),
      u32(f.beatAgeSeconds),
      u32(f.strikes),
      u32(f.slots),
      u32(f.running),
    ),
  )

export const replicaLapsed = (
  beatAgeSeconds: number,
  runAgeSeconds: number,
): boolean =>
  yes(
    placement().call('replica_lapsed', u32(beatAgeSeconds), u32(runAgeSeconds)),
  )

// --- proof.t27 --------------------------------------------------------------

export const reportVerdict = (
  blockedLine: boolean,
  hasCounts: boolean,
  fail: number,
): number =>
  proof().call(
    'report_verdict',
    flag(blockedLine),
    flag(hasCounts),
    u32(fail),
  ) >>> 0

export interface RunFacts {
  started: boolean
  outOfResources: boolean
  timedOut: boolean
  exitedZero: boolean
  report: number
}

export const runVerdict = (f: RunFacts): number =>
  proof().call(
    'run_verdict',
    flag(f.started),
    flag(f.outOfResources),
    flag(f.timedOut),
    flag(f.exitedZero),
    f.report & 0xff,
  ) >>> 0

export const listsTests = (verdict: number): boolean =>
  yes(proof().call('lists_tests', verdict & 0xff))

export const reasonOf = (
  verdict: number,
  exitedZero: boolean,
  blockedLine: boolean,
): number =>
  proof().call(
    'reason_of',
    verdict & 0xff,
    flag(exitedZero),
    flag(blockedLine),
  ) >>> 0

export const reasonKept = (namesJobDir: boolean): boolean =>
  yes(proof().call('reason_kept', flag(namesJobDir)))

export interface ReceiptFacts {
  hostKnown: boolean
  signatureOk: boolean
  leased: boolean
  reportedBefore: boolean
  sameAsBefore: boolean
  lapsed: boolean
  inputMatches: boolean
  modelMatches: boolean
  outputRehashes: boolean
}

export const receiptCode = (f: ReceiptFacts): number =>
  proof().call(
    'receipt_code',
    flag(f.hostKnown),
    flag(f.signatureOk),
    flag(f.leased),
    flag(f.reportedBefore),
    flag(f.sameAsBefore),
    flag(f.lapsed),
    flag(f.inputMatches),
    flag(f.modelMatches),
    flag(f.outputRehashes),
  ) >>> 0

export const tampered = (code: number): boolean =>
  yes(proof().call('tampered', code & 0xff))

export const attributable = (code: number): boolean =>
  yes(proof().call('attributable', code & 0xff))

export const isVote = (code: number, verdict: number): boolean =>
  yes(proof().call('is_vote', code & 0xff, verdict & 0xff))

export const jobVerdict = (
  votes: number,
  top: number,
  second: number,
  wanted: number,
  isExhausted: boolean,
): number =>
  proof().call(
    'job_verdict',
    u32(votes),
    u32(top),
    u32(second),
    u32(wanted),
    flag(isExhausted),
  ) >>> 0

export const wantedAfter = (verdict: number, wanted: number): number =>
  proof().call('wanted_after', verdict & 0xff, u32(wanted)) >>> 0

export const jobClosed = (verdict: number): boolean =>
  yes(proof().call('job_closed', verdict & 0xff))

export const sideOf = (verdict: number, group: number): number =>
  proof().call('side_of', verdict & 0xff, u32(group)) >>> 0

// --- credit.t27 -------------------------------------------------------------

export const creditMtri = (
  side: number,
  deviceBound: boolean,
  alreadyCredited: boolean,
): number =>
  Number(
    credit().call64(
      'credit_mtri',
      side & 0xff,
      flag(deviceBound),
      flag(alreadyCredited),
    ),
  )

export const strikeDue = (
  attributableTamper: boolean,
  side: number,
  alreadyStruck: boolean,
): boolean =>
  yes(
    credit().call(
      'strike_due',
      flag(attributableTamper),
      side & 0xff,
      flag(alreadyStruck),
    ),
  )

export const rowAdmitted = (kind: number, exists: boolean): boolean =>
  yes(credit().call('row_admitted', kind & 0xff, flag(exists)))

export const balanceAfter = (balance: number, add: number): number =>
  Number(credit().call64('balance_after', u64(balance), u64(add)))

export const settlementDue = (
  balanceMtri: number,
  enabled: boolean,
  legalCleared: boolean,
  deviceBound: boolean,
): boolean =>
  yes(
    credit().call64(
      'settlement_due',
      u64(balanceMtri),
      flag(enabled),
      flag(legalCleared),
      flag(deviceBound),
    ),
  )

// --- row.t27 ----------------------------------------------------------------

export const t27bRunVerdict = (
  started: boolean,
  wroteRow: boolean,
  word: number,
): number =>
  row().call('t27b_run_verdict', flag(started), flag(wroteRow), word & 0xff) >>>
  0

export const t27bVotes = (verdict: number): boolean =>
  yes(row().call('t27b_votes', verdict & 0xff))

export const halfVote = (
  code: number,
  half: number,
  verdict: number,
): boolean =>
  yes(row().call('half_vote', code & 0xff, half & 0xff, verdict & 0xff))

export const rowComplete = (reference: number, t27b: number): boolean =>
  yes(row().call('row_complete', reference & 0xff, t27b & 0xff))

export const detailKind = (referenceVerdict: number): number =>
  row().call('detail_kind', referenceVerdict & 0xff) >>> 0

export const referenceTestsListed = (
  referenceVerdict: number,
  listed: number,
  tests: number,
): boolean =>
  yes(
    row().call(
      'reference_tests_listed',
      referenceVerdict & 0xff,
      u32(listed),
      u32(tests),
    ),
  )

export const disagreeListed = (
  t27bListed: boolean,
  referenceListed: boolean,
): boolean =>
  yes(row().call('disagree_listed', flag(t27bListed), flag(referenceListed)))

export const disagreement = (
  t27bHas: boolean,
  t27bPassed: boolean,
  referenceHas: boolean,
  referencePassed: boolean,
): number =>
  row().call(
    'disagreement',
    flag(t27bHas),
    flag(t27bPassed),
    flag(referenceHas),
    flag(referencePassed),
  ) >>> 0
