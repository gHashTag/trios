/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BEE DISPATCHER AS KEYED ACTORS (MVP, epic gHashTag/trios#1712 item 7,
 * gHashTag/t27#7851). It does the round's dispatch work, arranged as actors:
 *
 *   admission, one actor: the lanes in use and the issues waiting for one.
 *     It answers one call per ready issue: a lane, wait, or refused
 *     (lane_free, and the policy's word through `deps.admit`, which is
 *     queend in production). When a bee's work ends, its lane goes to the
 *     waiting issues in the policy's order, unless its own issue restarts at
 *     once (keeps_lane_after_end): the wait ends on that event, not on the
 *     next round.
 *   one actor per issue (keyedActors, keyed.t27): the issue's state
 *     (bee_key_next, so a repeated ready starts no second bee), the claim
 *     (claim_step, with this incarnation's pid as the holder), the bee it
 *     started, and what happens when the bee ends (dispatch_exit.t27
 *     exit_reason, restart_decision, restart_wait_seconds).
 *   one bee actor per bee: one turn, which waits for the bee's work and is
 *     killed at the turn bound (actors.t27 turn_signal). The issue actor is
 *     linked to it and traps exits, so the bee actor's end is a message to it.
 *     A bee runs on a runner, so the turn only waits (ISO_LOOP) and a kill
 *     abandons the wait, not the bee. The issue actor then cancels the bee and
 *     keeps its issue and its lane until the bee's work has ended
 *     (holds_after_kill): a second bee never starts beside it.
 *
 * Behind TRIOS_QUEEN_DISPATCH=actors, off by default. The round's loop stays
 * the default until the benchmark's numbers are accepted (t27#7851).
 */

import {
  type ActorSystem,
  type Clock,
  createActorSystem,
  type Pid,
  realClock,
} from './queen-actors'
import {
  CALL_REPLY,
  ISO_LOOP,
  NO_PID,
  X_KILLED,
  X_NORMAL,
} from './queen-actors-card.gen'
import { keyedActors } from './queen-actors-keyed'
import { type Exit, linksOf } from './queen-actors-links'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import {
  DX_NORMAL,
  DX_SHUTDOWN,
  EF_KILL,
  EF_NONE,
  HB_NEVER,
  LEASE_TTL_SECONDS,
  RD_GIVE_UP,
  RD_LATER,
  RD_NOW,
} from './queen-dispatch-exit-card.gen'
import {
  BE_ENDED,
  BE_GIVE_UP,
  BE_READY,
  BE_REFUSED,
  BE_RESTART_LATER,
  BE_RESTART_NOW,
  BE_RETRY_DUE,
  BE_SENT_BACK,
  BE_STARTED,
  BK_IDLE,
  CL_START,
} from './queen-keyed-card.gen'

export const DISPATCH_EXIT_CARD = 'queen/dispatch_exit.wasm'
const keyed = () => loadCardWasm('queen/keyed.wasm')
const exitCard = () => loadCardWasm(DISPATCH_EXIT_CARD)
const actors = () => loadCardWasm('queen/actors.wasm')
const kc = (name: string, ...a: number[]) => keyed().call(name, ...a)
const dx = (name: string, ...a: number[]) => exitCard().call(name, ...a)

/** What the host saw when a bee's work ended (dispatch_exit.t27 section 2). */
export interface BeeEnd {
  /** A completion frame arrived. */
  completion: boolean
  /** The last error frame: EF_NONE, EF_CANCEL, EF_KILL or EF_PROVIDER. */
  errorFrame: number
  /** The provider's last HTTP status, 0 for none. */
  http: number
  /** Seconds since the bee's runtime last beat; HB_NEVER when none was seen. */
  heartbeatAge: number
  /** Whether the bee still held its task lease when it ended. */
  leaseHeld: boolean
}

/** A started bee: its end, and a way to ask it to stop. */
export interface BeeWork {
  /** Settles when the work has ended, however it ended. Never rejects. */
  ended: Promise<BeeEnd>
  /** Ask the work to stop; `ended` settles once it has. */
  cancel: () => void
}

export interface BeeStart {
  /** Whether the claim landed (control.t27 claim_lands, keyed.t27 claim_step). */
  claim: boolean
  /** The started bee, or null when the claim landed and nothing could start. */
  work: BeeWork | null
  /** For a lost claim: seconds since the holder's last renewal, if known. */
  holderSinceRenewal?: number
}

export interface BeeDispatchDeps {
  /** Lanes this dispatcher may fill at once. */
  lanes: () => number
  /** The policy's word on one issue, now. */
  admit: (issue: number) => Promise<boolean>
  /** The policy's order among issues waiting for a lane. Default: as they came. */
  order?: (waiting: number[]) => number[]
  /** Claim the issue for `holder` and start its bee. */
  start: (issue: number, holder: string) => Promise<BeeStart>
  /** The bee's turn bound in seconds; 0 is actors.t27 TURN_MAX_SECONDS. */
  turnMaxSeconds?: number
  /** How long an issue waits for the admission's answer. */
  admitAfterMs?: number
  /** Told of every start, end and stand-down. */
  observe?: (e: BeeEvent) => void
  /** Prefix of the claim's holder; the pid follows it. */
  holderPrefix?: string
}

export type BeeEvent =
  | { kind: 'started'; issue: number; holder: string }
  | { kind: 'ended'; issue: number; reason: number; decision: number }
  | { kind: 'stood-down'; issue: number }
  | { kind: 'killed'; issue: number }

type IssueMsg =
  | { kind: 'ready' }
  | { kind: 'sent-back' }
  | { kind: 'retry-due'; at: number }
  | { kind: 'work-ended'; work: BeeWork; end: BeeEnd; lane: number }
  | Exit

type AdmissionMsg =
  | {
      kind: 'call'
      /** `lane`: the token of a lane this issue still holds (a restart). */
      body: { issue: number; lane?: number }
      alias: Pid
      chain: bigint
      depth: number
    }
  | { kind: 'free'; issue: number; lane: number }

/** The admission's answer: a lane and its token, wait for one, or refused. */
type Admitted = { lane: number } | 'wait' | 'refused'

const UNKNOWN_END: BeeEnd = {
  completion: false,
  errorFrame: EF_NONE,
  http: 0,
  heartbeatAge: HB_NEVER,
  leaseHeld: false,
}

/** The bee dispatcher on an actor system. `ready(issue)` is its input. */
export function beeDispatcher(sys: ActorSystem, deps: BeeDispatchDeps) {
  const clock = sys.clock
  const links = linksOf(sys)
  const admitAfterMs = deps.admitAfterMs ?? 30_000
  const stats = { started: 0, standDowns: 0, killed: 0, refusedLanes: 0 }

  // ---- admission: lanes in use, by issue, each with a token; who waits
  const inUse = new Map<number, number>()
  let nextLane = 1
  const waiting: number[] = []
  const laneFree = (taken: number) =>
    kc('lane_free', u32(inUse.size + taken), u32(deps.lanes())) !== 0
  const wake = () => {
    const ordered = (deps.order ?? ((w) => w))([...waiting])
    let free = 0
    while (laneFree(free)) free++
    for (const issue of ordered.slice(0, free)) {
      waiting.splice(waiting.indexOf(issue), 1)
      issues.send(issue, { kind: 'ready' })
    }
  }
  const release = (issue: number, lane: number) => {
    if (inUse.get(issue) !== lane) return
    inUse.delete(issue)
    wake()
  }
  const admission: Pid = sys.spawn<AdmissionMsg>({
    name: 'bee-admission',
    receive: async (msg, self) => {
      if (msg.kind === 'free') return release(msg.issue, msg.lane)
      const { issue, lane } = msg.body
      const ok = () => deps.admit(issue).catch(() => false)
      let answer: Admitted = 'wait'
      if (lane !== undefined && inUse.get(issue) === lane) {
        // a restart at once, on the lane the issue kept (keeps_lane_after_end)
        if (await ok()) answer = { lane }
        else {
          release(issue, lane)
          answer = 'refused'
        }
      } else if (inUse.has(issue)) {
        answer = 'refused'
      } else if (laneFree(0)) {
        answer = (await ok()) ? { lane: nextLane++ } : 'refused'
        // one turn at a time: nobody took a lane meanwhile, but lanes() may move
        if (answer !== 'refused' && !laneFree(0)) answer = 'wait'
        if (typeof answer === 'object') inUse.set(issue, answer.lane)
      }
      if (answer === 'wait') {
        stats.refusedLanes++
        if (!waiting.includes(issue)) waiting.push(issue)
      }
      links.reply(msg, answer, self)
    },
  })
  const freeLane = (issue: number, lane: number) =>
    sys.send(admission, { kind: 'free', issue, lane } as AdmissionMsg)

  // ---- one actor per issue
  const issueActor = (issue: number) => {
    let state = BK_IDLE
    let bee: Pid | undefined
    let work: BeeWork | undefined
    let end: BeeEnd | undefined
    let killed = false
    let startedAt = 0
    let runs = 0
    let streak = 0
    let retry: (() => void) | undefined
    // the lane this issue holds, by its token: from admission to its bee's end
    let lane: number | undefined
    const giveLaneBack = () => {
      if (lane !== undefined) freeLane(issue, lane)
      lane = undefined
    }

    const move = (ev: number): boolean => {
      if (kc('bee_key_acts', state, ev) === 0) return false
      state = kc('bee_key_next', state, ev)
      return true
    }

    const admitAndStart = async (self: Pid) => {
      const got = await links.call<Admitted>(
        { self },
        admission,
        { issue, lane },
        admitAfterMs,
      )
      if (got.outcome !== CALL_REPLY || typeof got.value !== 'object') {
        // a kept lane the policy refused was released by the admission
        lane = undefined
        move(BE_REFUSED)
        return
      }
      lane = got.value.lane
      const holder = `${deps.holderPrefix ?? 'queen-actor'}:${self}`
      const s = await deps
        .start(issue, holder)
        .catch((): BeeStart => ({ claim: false, work: null }))
      const step = kc('claim_step', 1, flag(s.claim))
      if (kc('keeps_lane', step) === 0 || !s.work) {
        // a lost claim, or nothing started: the lane goes back at once
        giveLaneBack()
        move(BE_REFUSED)
        if (step !== CL_START) {
          stats.standDowns++
          deps.observe?.({ kind: 'stood-down', issue })
          const wait = kc(
            'retry_after_lost_seconds',
            u32(s.holderSinceRenewal ?? 0),
            LEASE_TTL_SECONDS,
          )
          retry?.()
          retry = clock.after(wait * 1000, () =>
            issues.send(issue, { kind: 'ready' }),
          )
        }
        return
      }
      const w = s.work
      work = w
      end = undefined
      killed = false
      startedAt = clock.now()
      // When the work ends, this incarnation decides what its lane does
      // (keeps_lane_after_end). If it has died meanwhile, nobody here restarts
      // the issue, and the lane goes back at once.
      const held = got.value.lane
      void w.ended
        .catch(() => UNKNOWN_END)
        .then((e) => {
          if (sys.alive(self))
            issues.send(issue, {
              kind: 'work-ended',
              work: w,
              end: e,
              lane: held,
            })
          else freeLane(issue, held)
        })
      bee = sys.spawn<{ go: true }>({
        name: `bee-${issue}`,
        turnMaxSeconds: deps.turnMaxSeconds ?? 0,
        receive: async (_m, beeSelf) => {
          await w.ended.catch(() => UNKNOWN_END)
          sys.exit(beeSelf, X_NORMAL)
        },
      })
      links.link(self, bee)
      sys.send(bee, { go: true })
      move(BE_STARTED)
      stats.started++
      deps.observe?.({ kind: 'started', issue, holder })
    }

    // Both halves are in: the work ended and its bee actor is gone.
    const decide = async (self: Pid) => {
      if (!end) return
      const e = end
      end = undefined
      work = undefined
      const reason = dx(
        'exit_reason',
        flag(e.completion),
        killed ? EF_KILL : e.errorFrame,
        dx('http_class', u32(e.http)),
        u32((clock.now() - startedAt) / 1000),
        0,
        flag(e.leaseHeld),
        u32(e.heartbeatAge),
      )
      const http = dx('http_class', u32(e.http))
      const decision = dx(
        'restart_decision',
        reason,
        http,
        u32(runs),
        u32(streak),
      )
      if (dx('counts_against_issue', reason) !== 0) runs++
      if (kc('keeps_lane_after_end', decision) === 0) giveLaneBack()
      deps.observe?.({ kind: 'ended', issue, reason, decision })
      if (decision === RD_NOW) {
        streak++
        move(BE_RESTART_NOW)
        await admitAndStart(self)
      } else if (decision === RD_LATER) {
        const wait = dx(
          'restart_wait_seconds',
          reason,
          u32(streak),
          u32(e.heartbeatAge),
        )
        streak++
        move(BE_RESTART_LATER)
        retry?.()
        retry = clock.after(wait * 1000, () =>
          issues.send(issue, { kind: 'retry-due', at: clock.now() }),
        )
      } else if (decision === RD_GIVE_UP) {
        move(BE_GIVE_UP)
      } else if (reason === DX_NORMAL || reason === DX_SHUTDOWN) {
        streak = 0
        move(BE_ENDED)
      }
    }

    const EVENT_OF = {
      ready: BE_READY,
      'sent-back': BE_SENT_BACK,
      'retry-due': BE_RETRY_DUE,
    } as const

    /** The bee actor ended, or a stopper asked this actor to end. */
    const onExit = async (msg: Exit, self: Pid) => {
      if (msg.pid === BigInt(NO_PID)) {
        sys.exit(self, X_NORMAL)
        return
      }
      if (msg.pid !== bee) return
      bee = undefined
      if (msg.reason === X_KILLED) {
        killed = true
        stats.killed++
        deps.observe?.({ kind: 'killed', issue })
        // the turn bound abandoned the wait; the bee runs on until it is
        // stopped, and holds its issue and its lane until then
        const holds = actors().call('holds_after_kill', ISO_LOOP, flag(!!end))
        if (holds !== 0) {
          work?.cancel()
          return
        }
      }
      await decide(self)
    }

    return {
      name: `issue-${issue}`,
      init: (self: Pid) => links.trapExit(self),
      holdsWork: () => kc('key_holds_work', state) !== 0 || work !== undefined,
      receive: async (msg: IssueMsg, self: Pid) => {
        if (msg.kind === 'EXIT') return onExit(msg, self)
        if (msg.kind !== 'work-ended') {
          // a repeated ready while the bee runs or its work waits: nothing
          if (move(EVENT_OF[msg.kind])) await admitAndStart(self)
          return
        }
        // another incarnation's bee: nobody here restarts it; its lane goes back
        if (msg.work !== work) return void freeLane(issue, msg.lane)
        end = msg.end
        if (bee === undefined) await decide(self)
      },
    }
  }

  const issues = keyedActors<number, IssueMsg>(sys, {
    name: 'bee-issues',
    make: issueActor,
  })

  return {
    /** An issue is ready (the round read it, or an event names it). */
    ready: (issue: number) => issues.send(issue, { kind: 'ready' }),
    /** The review sent the work back: a new attempt. */
    sentBack: (issue: number) => issues.send(issue, { kind: 'sent-back' }),
    lanesInUse: () => inUse.size,
    waiting: () => [...waiting],
    issues,
    stats,
    /** Stop every issue actor, the last started first; bees on runners run on. */
    stop: () => issues.stopAll(),
  }
}

export type BeeDispatcher = ReturnType<typeof beeDispatcher>

/**
 * A dispatcher on its own actor system, for production. `clock` is the real
 * one there; a test hands in a virtual one instead of faking the process's
 * timers, which Bun does not fake alike on every platform.
 */
export function startBeeDispatcher(
  deps: BeeDispatchDeps,
  clock: Clock = realClock,
): BeeDispatcher {
  return beeDispatcher(createActorSystem(clock), deps)
}
