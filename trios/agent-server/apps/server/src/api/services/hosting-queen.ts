/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The Queen's half of self-hosting (gHashTag/trios#1756, slice 1): hosts
 * register with a key, beat, lease one replica of a shard at a time and post
 * a signed receipt; the Queen judges each receipt, decides each job by k-of-n
 * agreement and keeps the off-chain ledger.
 *
 * THIN GLUE. This file reads and writes rows, checks signatures and hashes,
 * and counts. Every decision is a call into a card (hosting-cards.ts):
 * the tier a key gets, whether a write is admitted, whether a host may take a
 * job, when a lease lapses, what a receipt's code is, whether a job is agreed,
 * who is credited and who is struck. A number the cards own (k, the lease
 * TTL, the run bound, the credit rate) is never written here.
 *
 * WHY A CLOCK IS PASSED IN: the demo runs the whole protocol, lapses
 * included, on a VirtualClock; production passes Date.now.
 */

import { randomBytes } from 'node:crypto'
import {
  attributable,
  balanceAfter,
  creditMtri,
  exhausted,
  hostWriteAdmitted,
  isVote,
  jobClosed,
  jobFirst,
  jobVerdict,
  mayTake,
  nextIncarnation,
  openReplicas,
  originBlocks,
  places,
  receiptCode,
  replicaLapsed,
  requestFresh,
  rowAdmitted,
  settlementDue,
  sideOf,
  slotsAdmitted,
  spreadOk,
  strikeDue,
  suspended,
  tampered,
  tierOf,
  wantedAfter,
} from './hosting-cards'
import type {
  HostingStore,
  HostingTx,
  HostRow,
  JobFile,
  JobRow,
  LeaseRow,
} from './hosting-store'
import {
  keyIdOf,
  messageOf,
  readNormalized,
  sha256Hex,
  verifyHex,
} from './hosting-wire'
import {
  CREDIT_TRANSFERABLE,
  LE_CREDIT,
  LE_STRIKE,
  SETTLE_THRESHOLD_MTRI,
  SETTLEMENT_ENABLED,
  SIDE_NONE,
  TOKEN_VALUE_CLAIMED,
} from './queen-hosting-credit-card.gen'
import {
  ALLOW_NONE,
  NODE_HEARTBEAT_SECONDS,
  NODE_TTL_SECONDS,
  TIER_OWNER,
  WC_SHARD,
} from './queen-hosting-host-card.gen'
import { JOB_RUN_BOUND_SECONDS } from './queen-hosting-placement-card.gen'
import {
  JV_AGREED,
  JV_PENDING,
  LEASE_NONCE_BYTES,
  RECEIPT_DOMAIN,
  RECEIPT_SIGNED_FIELDS,
  REGISTER_DOMAIN,
  REGISTER_SIGNED_FIELDS,
  REPLICAS_K,
  REQUEST_DOMAIN,
  REQUEST_SIGNED_FIELDS,
  VERDICT_WORDS,
} from './queen-hosting-proof-card.gen'

export const TIER_WORDS = ['owner', 'trusted', 'public'] as const
export const JOB_VERDICT_WORDS = [
  'pending',
  'agreed',
  'tiebreak',
  'unresolved',
] as const
export const RECEIPT_CODE_WORDS = [
  'ok',
  'unknown_host',
  'bad_signature',
  'not_leased',
  'duplicate',
  'equivocation',
  'late',
  'input_mismatch',
  'model_mismatch',
  'output_mismatch',
] as const

/** A beat age for an incarnation the row no longer holds: it beats no more. */
const SUPERSEDED_AGE = 0xffff_ffff
const HEX40 = /^[0-9a-f]{40}$/
const HEX64 = /^[0-9a-f]{64}$/
const SPEC_PATH = /^specs\/[A-Za-z0-9_\-/.]+\.t27$/
const MAX_OUTPUT_BYTES = 256 * 1024

export class HostingError extends Error {
  constructor(
    readonly code: string,
    readonly status: 400 | 401 | 403 | 404 | 409 | 503,
  ) {
    super(code)
  }
}

/** One signed host request, as the route hands it over. */
export interface HostRequest {
  method: string
  path: string
  header: (name: string) => string | undefined
  body: string
}

export interface HostingQueenDeps {
  store: HostingStore
  /** Milliseconds. */
  now: () => number
  /** The owner's allowlist: key id to tier. A key it does not name is public. */
  allowlist: ReadonlyMap<string, number>
  random?: (bytes: number) => string
}

const int = (v: unknown, max: number): number | null =>
  typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= max ? v : null

const text = (v: unknown, re: RegExp): string | null =>
  typeof v === 'string' && re.test(v) ? v : null

function specPath(v: unknown): string | null {
  const p = text(v, SPEC_PATH)
  return p && !p.split('/').includes('..') ? p : null
}

export function createHostingQueen(deps: HostingQueenDeps) {
  const random =
    deps.random ?? ((bytes: number) => randomBytes(bytes).toString('hex'))
  const nowMs = deps.now
  const secondsSince = (ms: number) =>
    Math.max(0, Math.floor((nowMs() - ms) / 1000))
  const skew = (utc: number) => Math.abs(Math.floor(nowMs() / 1000) - utc)

  /** The beat age of the incarnation a lease was given to. */
  const leaseBeatAge = (h: HostRow | undefined | null, l: LeaseRow) =>
    !h || h.incarnation !== l.incarnation
      ? SUPERSEDED_AGE
      : secondsSince(h.beatAt)

  // --- writes from a host ---------------------------------------------------

  async function admit(s: HostingTx, req: HostRequest): Promise<HostRow> {
    const id = req.header('x-trios-host') ?? ''
    const inc = Number(req.header('x-trios-inc'))
    const utc = Number(req.header('x-trios-ts'))
    if (!Number.isInteger(inc) || !Number.isInteger(utc))
      throw new HostingError('unsigned_request', 401)
    if (!requestFresh(skew(utc))) throw new HostingError('stale_request', 401)
    const host = await s.host(id)
    if (!host) throw new HostingError('unknown_host', 401)
    const message = messageOf(REQUEST_DOMAIN, REQUEST_SIGNED_FIELDS, {
      method: req.method,
      path: req.path,
      host: id,
      incarnation: inc,
      utc_unix: utc,
      body_sha256: sha256Hex(req.body),
    })
    if (!verifyHex(host.publicKey, message, req.header('x-trios-sig')))
      throw new HostingError('bad_signature', 401)
    // netlink section 2: a write from an incarnation that is over, or from a
    // lease that lapsed, is refused, and the host registers again.
    if (!hostWriteAdmitted(host.incarnation, inc, secondsSince(host.beatAt)))
      throw new HostingError('fenced', 409)
    host.beatAt = nowMs()
    await s.putHost(host)
    return host
  }

  async function register(body: unknown, origin: string) {
    const b = (body ?? {}) as Record<string, unknown>
    const publicKey = text(b.public_key, HEX64)
    const tierClaim = int(b.tier_claim, 255)
    const slots = int(b.slots, 1_000_000)
    const platform = text(b.platform, /^[a-z0-9-]{1,32}$/)
    const utc = int(b.utc_unix, Number.MAX_SAFE_INTEGER)
    if (
      publicKey === null ||
      tierClaim === null ||
      slots === null ||
      platform === null ||
      utc === null
    )
      throw new HostingError('bad_registration', 400)
    if (!requestFresh(skew(utc))) throw new HostingError('stale_request', 401)
    const message = messageOf(REGISTER_DOMAIN, REGISTER_SIGNED_FIELDS, {
      public_key: publicKey,
      tier_claim: tierClaim,
      slots,
      platform,
      utc_unix: utc,
    })
    if (!verifyHex(publicKey, message, b.signature))
      throw new HostingError('bad_signature', 401)
    const id = keyIdOf(publicKey)
    return deps.store.tx(async (s) => {
      const prev = await s.host(id)
      if (prev && prev.publicKey !== publicKey)
        throw new HostingError('key_conflict', 409)
      const tier = tierOf(tierClaim, deps.allowlist.get(id) ?? ALLOW_NONE)
      const row: HostRow = {
        id,
        publicKey,
        tier,
        tierClaim,
        slots: slotsAdmitted(tier, slots),
        platform,
        origin,
        incarnation: nextIncarnation(prev?.incarnation ?? 0),
        beatAt: nowMs(),
        strikes: prev?.strikes ?? 0,
        registeredAt: nowMs(),
      }
      // host.t27 section 4: the last incarnation is down for good. Its leases
      // carry its number, so the next sweep reads them as a host that beats
      // no more, and replica_lapsed places them again.
      await s.putHost(row)
      return {
        host: id,
        incarnation: row.incarnation,
        tier: TIER_WORDS[tier],
        slots: row.slots,
        beat_seconds: NODE_HEARTBEAT_SECONDS,
        ttl_seconds: NODE_TTL_SECONDS,
      }
    })
  }

  async function beat(req: HostRequest, id: string) {
    return deps.store.tx(async (s) => {
      const host = await admit(s, req)
      if (host.id !== id) throw new HostingError('not_this_host', 403)
      return { ok: true, incarnation: host.incarnation }
    })
  }

  // --- placement --------------------------------------------------------------

  /** Lapse what the cards say is over, then decide each job again. */
  async function sweep(s: HostingTx) {
    const hosts = new Map((await s.hosts()).map((h) => [h.id, h]))
    for (const job of await s.openJobs()) {
      for (const l of await s.leasesOf(job.id)) {
        if (l.state !== 'out') continue
        const lapsed = replicaLapsed(
          leaseBeatAge(hosts.get(l.hostId), l),
          secondsSince(l.leasedAt),
        )
        if (!lapsed) continue
        l.state = 'lapsed'
        await s.putLease(l)
      }
      await settle(s, job)
    }
  }

  async function lease(req: HostRequest) {
    return deps.store.tx(async (s) => {
      const host = await admit(s, req)
      await sweep(s)
      const hosts = new Map((await s.hosts()).map((h) => [h.id, h]))
      const running = (await s.outLeasesOfHost(host.id)).length
      const candidates = await Promise.all(
        (await s.openJobs()).map(async (job) => {
          const leases = await s.leasesOf(job.id)
          return { job, leases, votes: leases.filter((l) => l.vote).length }
        }),
      )
      candidates.sort((a, b) => {
        const ageA = secondsSince(a.job.createdAt)
        const ageB = secondsSince(b.job.createdAt)
        const ab = jobFirst(a.votes, ageA, b.votes, ageB)
        const ba = jobFirst(b.votes, ageB, a.votes, ageA)
        if (ab && ba) return a.job.id < b.job.id ? -1 : 1
        return ab ? -1 : 1
      })
      for (const { job, leases, votes } of candidates) {
        const outstanding = leases.filter((l) => l.state === 'out').length
        const open = openReplicas(job.wanted, outstanding, votes)
        const leasedBefore = leases.some((l) => l.hostId === host.id)
        // the job's other live leases and votes from this host's network,
        // each counted when the card says that holder's tier blocks it
        const blocking = leases.filter((l) => {
          const other = hosts.get(l.hostId)
          if (!other || other.id === host.id) return false
          if (l.state !== 'out' && !l.vote) return false
          return (
            other.origin === host.origin &&
            originBlocks(host.tier, other.tier, 1)
          )
        }).length
        const take = mayTake({
          tier: host.tier,
          workloadClass: job.workloadClass,
          holdsSecret: job.holdsSecret,
          holdsPersonal: job.holdsPersonal,
          beatAgeSeconds: 0,
          strikes: host.strikes,
          slots: host.slots,
          running,
        })
        const spread = spreadOk(host.tier, leasedBefore, blocking)
        if (!places(take, spread, open, job.leasesIssued)) continue
        const row: LeaseRow = {
          id: `l_${random(8)}`,
          jobId: job.id,
          hostId: host.id,
          incarnation: host.incarnation,
          nonce: random(LEASE_NONCE_BYTES),
          leasedAt: nowMs(),
          state: 'out',
          code: null,
          vote: false,
          outputHash: null,
          receipt: null,
          receivedAt: null,
        }
        job.leasesIssued += 1
        await s.putJob(job)
        await s.putLease(row)
        return {
          lease: row.id,
          nonce: row.nonce,
          run_bound_seconds: JOB_RUN_BOUND_SECONDS,
          job: jobPayload(job),
        }
      }
      return { lease: null }
    })
  }

  // --- receipts and agreement ---------------------------------------------------

  /** Decide the job from its votes (proof.t27), and credit or strike once it closes. */
  async function settle(s: HostingTx, job: JobRow): Promise<number> {
    if (job.state !== 'open') return job.verdict
    const leases = await s.leasesOf(job.id)
    const votes = leases
      .filter((l) => l.vote)
      .sort((a, b) => (a.receivedAt ?? 0) - (b.receivedAt ?? 0))
    const groups = new Map<string, LeaseRow[]>()
    for (const v of votes) {
      const key = v.outputHash ?? ''
      groups.set(key, [...(groups.get(key) ?? []), v])
    }
    // largest first; a tie keeps first arrival (Map order), which no verdict reads
    const ranked = [...groups.values()].sort((a, b) => b.length - a.length)
    const outstanding = leases.filter((l) => l.state === 'out').length
    const verdict = jobVerdict(
      votes.length,
      ranked[0]?.length ?? 0,
      ranked[1]?.length ?? 0,
      job.wanted,
      exhausted(job.leasesIssued, outstanding),
    )
    job.verdict = verdict
    job.wanted = wantedAfter(verdict, job.wanted)
    if (jobClosed(verdict)) await close(s, job, verdict, leases, ranked)
    await s.putJob(job)
    return verdict
  }

  /** A closed job: its open leases end, each vote is accounted, the result is kept. */
  async function close(
    s: HostingTx,
    job: JobRow,
    verdict: number,
    leases: LeaseRow[],
    ranked: LeaseRow[][],
  ) {
    job.state = verdict === JV_AGREED ? 'agreed' : 'unresolved'
    job.closedAt = nowMs()
    for (const l of leases) {
      if (l.state !== 'out') continue
      l.state = 'closed'
      await s.putLease(l)
    }
    for (const [group, members] of ranked.entries())
      for (const l of members)
        await account(s, job, l.hostId, sideOf(verdict, group), false)
    const top = ranked[0]?.[0]?.receipt
    if (verdict !== JV_AGREED || !top) return
    const stated = readNormalized(String(top.output ?? ''))
    job.result = {
      outputHash: String(top.output_hash),
      word: String(top.verdict),
      tests: stated?.tests ?? 0,
      ops: Number(top.ops),
      hosts: (ranked[0] ?? []).map((l) => l.hostId),
    }
  }

  /** credit.t27: one credit, one strike, per (host, job), never more. */
  async function account(
    s: HostingTx,
    job: JobRow,
    hostId: string,
    side: number,
    tamper: boolean,
  ) {
    const credited = await s.ledgerHas(hostId, job.id, LE_CREDIT)
    const mtri = creditMtri(side, false, credited)
    if (mtri > 0 && rowAdmitted(LE_CREDIT, credited))
      await s.addLedger({
        hostId,
        jobId: job.id,
        kind: LE_CREDIT,
        mtri,
        at: nowMs(),
      })
    const struck = await s.ledgerHas(hostId, job.id, LE_STRIKE)
    if (strikeDue(tamper, side, struck) && rowAdmitted(LE_STRIKE, struck)) {
      await s.addLedger({
        hostId,
        jobId: job.id,
        kind: LE_STRIKE,
        mtri: 0,
        at: nowMs(),
      })
      const host = await s.host(hostId)
      if (host) {
        host.strikes += 1
        await s.putHost(host)
      }
    }
  }

  async function receipt(body: unknown) {
    const r = (body ?? {}) as Record<string, unknown>
    if (typeof r.output !== 'string' || r.output.length > MAX_OUTPUT_BYTES)
      throw new HostingError('bad_receipt', 400)
    let message: string
    try {
      message = messageOf(RECEIPT_DOMAIN, RECEIPT_SIGNED_FIELDS, r)
    } catch {
      throw new HostingError('bad_receipt', 400)
    }
    return deps.store.tx(async (s) => {
      await sweep(s)
      const host = await s.host(String(r.host))
      const lease = await s.lease(String(r.lease))
      const job = lease ? await s.job(lease.jobId) : null
      const facts = receiptFacts(r, message, host, lease, job)
      const code = receiptCode(facts)
      const verdictCode = VERDICT_WORDS.indexOf(r.verdict as never)
      const vote = isVote(code, verdictCode < 0 ? 255 : verdictCode)
      if (
        lease &&
        facts.leased &&
        !facts.reportedBefore &&
        lease.state === 'out'
      ) {
        lease.state = 'reported'
        lease.code = code
        lease.vote = vote
        lease.outputHash = String(r.output_hash)
        lease.receipt = r
        lease.receivedAt = nowMs()
        await s.putLease(lease)
      }
      if (host && job && attributable(code))
        await account(s, job, host.id, SIDE_NONE, true)
      const verdict =
        job && job.state === 'open'
          ? await settle(s, job)
          : (job?.verdict ?? JV_PENDING)
      return {
        code: RECEIPT_CODE_WORDS[code] ?? String(code),
        accepted: vote,
        tampered: tampered(code),
        job: job
          ? {
              id: job.id,
              verdict: JOB_VERDICT_WORDS[verdict] ?? String(verdict),
            }
          : null,
      }
    })
  }

  // --- jobs, owner only -------------------------------------------------------

  async function createJob(req: HostRequest) {
    const b = (() => {
      try {
        return JSON.parse(req.body || '{}') as Record<string, unknown>
      } catch {
        throw new HostingError('bad_job', 400)
      }
    })()
    const commit = text(b.commit, HEX40)
    const spec = specPath(b.spec)
    const inputHash = text(b.input_hash, HEX64)
    const modelHash =
      b.model_hash === undefined || b.model_hash === null
        ? null
        : text(b.model_hash, HEX64)
    const files: JobFile[] = Array.isArray(b.files)
      ? b.files.map((f) => ({
          path: specPath((f as JobFile)?.path) ?? '',
          sha256: text((f as JobFile)?.sha256, HEX64) ?? '',
        }))
      : []
    // host.t27 section 2: a workload must carry both declarations
    if (
      typeof b.holds_secret !== 'boolean' ||
      typeof b.holds_personal !== 'boolean'
    )
      throw new HostingError('undeclared_workload', 400)
    if (
      !commit ||
      !spec ||
      !inputHash ||
      (b.model_hash !== undefined && b.model_hash !== null && !modelHash) ||
      files.length === 0 ||
      files.length > 512 ||
      files.some((f) => !f.path || !f.sha256) ||
      !files.some((f) => f.path === spec && f.sha256 === inputHash)
    )
      throw new HostingError('bad_job', 400)
    // slice 1 runs shards only; a service needs slice 2's tunnel
    if (b.workload_class !== undefined && b.workload_class !== WC_SHARD)
      throw new HostingError('shards_only', 400)
    const sorted = [...files].sort((x, y) => (x.path < y.path ? -1 : 1))
    const id = `j_${sha256Hex(
      JSON.stringify([
        commit,
        spec,
        inputHash,
        sorted,
        modelHash,
        b.holds_secret,
        b.holds_personal,
      ]),
    ).slice(0, 16)}`
    return deps.store.tx(async (s) => {
      const owner = await admit(s, req)
      if (owner.tier !== TIER_OWNER) throw new HostingError('owner_only', 403)
      const known = await s.job(id)
      if (known) return { job: jobPayload(known), created: false }
      const row: JobRow = {
        id,
        kind: 'shard',
        commit,
        spec,
        inputHash,
        files: sorted,
        modelHash,
        workloadClass: WC_SHARD,
        holdsSecret: b.holds_secret as boolean,
        holdsPersonal: b.holds_personal as boolean,
        wanted: REPLICAS_K,
        leasesIssued: 0,
        state: 'open',
        verdict: JV_PENDING,
        createdAt: nowMs(),
        closedAt: null,
        result: null,
      }
      await s.putJob(row)
      return { job: jobPayload(row), created: true }
    })
  }

  // --- reads ------------------------------------------------------------------

  /** The public ledger: key ids, tiers, balances, strikes. No address, origin or key. */
  async function ledger() {
    return deps.store.tx(async (s) => {
      const rows = await s.ledger()
      const hosts = (await s.hosts()).map((h) => {
        const mine = rows.filter((r) => r.hostId === h.id)
        let balance = 0
        for (const r of mine)
          if (r.kind === LE_CREDIT) balance = balanceAfter(balance, r.mtri)
        return {
          host: h.id,
          tier: TIER_WORDS[h.tier] ?? 'public',
          balance_mtri: balance,
          credited_jobs: mine.filter((r) => r.kind === LE_CREDIT).length,
          strikes: h.strikes,
          suspended: suspended(h.strikes),
          settlement_due: settlementDue(
            balance,
            SETTLEMENT_ENABLED,
            false,
            false,
          ),
        }
      })
      return {
        unit: 'mTRI',
        off_chain: true,
        transferable: CREDIT_TRANSFERABLE,
        token_value_claimed: TOKEN_VALUE_CLAIMED,
        settlement: {
          enabled: SETTLEMENT_ENABLED,
          threshold_mtri: SETTLE_THRESHOLD_MTRI,
        },
        hosts,
        rows: rows.map((r) => ({
          seq: r.seq,
          host: r.hostId,
          job: r.jobId,
          kind: r.kind === LE_CREDIT ? 'credit' : 'strike',
          mtri: r.mtri,
          at: new Date(r.at).toISOString(),
        })),
      }
    })
  }

  /** Agreed shards in the lab's row shape (file, reference, tests, asserts). */
  async function verdicts(commit?: string) {
    return deps.store.tx(async (s) =>
      (await s.jobs())
        .filter((j) => j.state === 'agreed' && j.result)
        .filter((j) => !commit || j.commit === commit)
        .map((j) => ({
          commit: j.commit,
          file: j.spec,
          reference: j.result?.word,
          tests: j.result?.tests,
          asserts: j.result?.ops,
          input_sha256: j.inputHash,
          output_sha256: j.result?.outputHash,
          hosts: j.result?.hosts,
        })),
    )
  }

  async function job(id: string) {
    return deps.store.tx(async (s) => {
      const row = await s.job(id)
      if (!row) throw new HostingError('no_such_job', 404)
      const leases = await s.leasesOf(id)
      return {
        ...jobPayload(row),
        state: row.state,
        verdict: JOB_VERDICT_WORDS[row.verdict] ?? String(row.verdict),
        wanted: row.wanted,
        leases_issued: row.leasesIssued,
        leases: leases.map((l) => ({
          lease: l.id,
          host: l.hostId,
          state: l.state,
          code: l.code === null ? null : RECEIPT_CODE_WORDS[l.code],
          vote: l.vote,
          output_sha256: l.outputHash,
        })),
        result: row.result,
      }
    })
  }

  return { register, beat, lease, receipt, createJob, ledger, verdicts, job }
}

export type HostingQueen = ReturnType<typeof createHostingQueen>

/** What proof.t27 receipt_code reads, from the receipt and the rows it names. */
function receiptFacts(
  r: Record<string, unknown>,
  message: string,
  host: HostRow | null,
  lease: LeaseRow | null,
  job: JobRow | null,
) {
  const leased =
    !!lease &&
    !!job &&
    lease.hostId === r.host &&
    lease.jobId === r.job &&
    lease.incarnation === r.incarnation &&
    lease.nonce === r.nonce
  const reportedBefore = leased && lease.receipt !== null
  const stated = readNormalized(String(r.output))
  return {
    hostKnown: !!host,
    signatureOk: !!host && verifyHex(host.publicKey, message, r.signature),
    leased,
    reportedBefore,
    sameAsBefore:
      reportedBefore && JSON.stringify(lease.receipt) === JSON.stringify(r),
    lapsed: leased && lease.state !== 'out',
    inputMatches:
      !!job &&
      r.commit === job.commit &&
      r.spec === job.spec &&
      r.input_hash === job.inputHash,
    modelMatches: !job?.modelHash || r.model_hash === job.modelHash,
    outputRehashes:
      sha256Hex(String(r.output)) === r.output_hash &&
      stated?.word === r.verdict &&
      stated?.ops === r.ops &&
      stated?.spec === r.spec,
  }
}

function jobPayload(j: JobRow) {
  return {
    id: j.id,
    kind: j.kind,
    commit: j.commit,
    spec: j.spec,
    input_hash: j.inputHash,
    files: j.files,
    model_hash: j.modelHash,
    holds_secret: j.holdsSecret,
    holds_personal: j.holdsPersonal,
  }
}

/** The owner's allowlist from `<key id>=<owner|trusted|public>,...`. */
export function parseAllowlist(raw: string | undefined): Map<string, number> {
  const out = new Map<string, number>()
  for (const part of (raw ?? '').split(',')) {
    const [id, word] = part.trim().split('=')
    const tier = TIER_WORDS.indexOf((word ?? '').trim() as never)
    if (id && /^[0-9a-f]{16}$/.test(id.trim()) && tier >= 0)
      out.set(id.trim(), tier)
  }
  return out
}
