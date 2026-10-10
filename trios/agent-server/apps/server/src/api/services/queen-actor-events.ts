/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTORS ON THE BUS (gHashTag/t27 specs/queen/actor_events.t27 and
 * events.t27 section 6, gHashTag/trios#1744). The kanban view of the actors
 * (t27#8615, actor_view.t27) needs every spawn, exit, DOWN and restart, and
 * the messages between actors, as a feed it can read past a cursor. Before
 * this file no actor transition left the process.
 *
 * The cards decide:
 *   - which deliver is written one by one and which is counted (deliver_kind,
 *     carries_task), and what a refused one is (is_dropped);
 *   - which exit or DOWN names the task it ended (exit_task);
 *   - what anyone may read of an exit reason (reason_class);
 *   - when a pair's count is written: at its window, before an event that must
 *     follow it, when the table is full, when the runtime stops (ordering,
 *     table_full, flush_reason);
 *   - how many events may wait for the bus (buffer_admits);
 *   - which payload keys are public (ACTOR_PUBLIC_KEYS, with events.t27
 *     token_ok), and whether a row's kind is an actor kind (actor_kind_ok);
 *   - the cursor and the page (events.t27 cursor_read, page_size).
 * This file counts, keeps the pairs, numbers the messages, queues the events
 * for the one writer and answers the route. The runtime calls it only through
 * `options.events`; with that unset (TRIOS_QUEEN_ACTOR_EVENTS off, the default)
 * nothing here runs and nothing is written.
 *
 * NEVER A PAYLOAD. An event holds pids, kinds, counts, depths, times and a
 * task reference read by the domain's own `taskOf`. A message, its arguments,
 * a card's arguments and a reason's text are never put in a row, and the
 * projection drops any key the card does not list, whatever a row holds.
 */

import { logger } from '../../lib/logger'
import {
  ACTOR_PUBLIC_KEYS,
  ACTOR_WINDOW_MS,
  FL_KEEP,
  NO_TASK,
  OR_NONE,
  OR_PAIR,
  OR_PID,
  RC_NAMES,
  REPLAY_ACTOR_NAMES,
  REPLAY_AT_MS,
  REPLAY_COUNT,
  REPLAY_DEPTH,
  REPLAY_DROPPED,
  REPLAY_EVENTS,
  REPLAY_FIRST,
  REPLAY_FROM_GEN,
  REPLAY_FROM_SLOT,
  REPLAY_KIND,
  REPLAY_LAST,
  REPLAY_MAX_RESTARTS,
  REPLAY_PERIOD_SECONDS,
  REPLAY_REASON,
  REPLAY_REPO,
  REPLAY_RESTARTS,
  REPLAY_STRATEGY,
  REPLAY_SUPERVISOR_SLOT,
  REPLAY_T0_MS,
  REPLAY_TASK,
  REPLAY_TASK_KIND,
  REPLAY_TO_GEN,
  REPLAY_TO_SLOT,
} from './queen-actor-events-card.gen'
import type { Clock, Pid } from './queen-actors'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import {
  ACTOR_EVENT_NAMES,
  CR_PAGE,
  CR_RESYNC,
  EV_ACTOR_DELIVER,
  EV_ACTOR_DELIVERS,
  EV_ACTOR_DOWN,
  EV_ACTOR_EXIT,
  EV_ACTOR_RESTART,
  EV_ACTOR_SPAWN,
} from './queen-events.gen'
import { KIND_NAMES } from './queen-tasks.gen'

export const ACTOR_EVENTS_CARD = 'queen/actor_events.wasm'
export const EVENTS_CARD_FILE = 'queen/events.wasm'

let loaded: ReturnType<typeof loadCardWasm> | undefined
const card = () => {
  loaded ??= loadCardWasm(ACTOR_EVENTS_CARD)
  return loaded
}
const bus = () => loadCardWasm(EVENTS_CARD_FILE)

export const carriesTask = (taskKind: number): boolean =>
  card().call('carries_task', taskKind) !== 0
export const deliverKind = (taskKind: number): number =>
  card().call('deliver_kind', taskKind)
export const exitTask = (taskKind: number, inTurn: boolean): boolean =>
  card().call('exit_task', taskKind, flag(inTurn)) !== 0
export const isDropped = (verdict: number): boolean =>
  card().call('is_dropped', verdict) !== 0
export const reasonClass = (reason: number): number =>
  card().call('reason_class', reason)
export const ordering = (kind: number): number => card().call('ordering', kind)
export const tableFull = (openPairs: number): boolean =>
  card().call('table_full', u32(openPairs)) !== 0
export const flushReason = (
  elapsedMs: number,
  ordered: boolean,
  full: boolean,
  stopping: boolean,
): number =>
  card().call(
    'flush_reason',
    u32(elapsedMs),
    flag(ordered),
    flag(full),
    flag(stopping),
  )
export const bufferAdmits = (buffered: number): boolean =>
  card().call('buffer_admits', u32(buffered)) !== 0
export const actorKindOk = (kind: number): boolean =>
  bus().call('actor_kind_ok', kind) !== 0

// Each of these is pure in a small domain (a task kind, a verdict, an event
// kind is a u8), so the card's answer for one value is its answer for good,
// as telemetry's bucket memo keeps bucket_of. A deliver asks two of them.
const memo = (fn: (v: number) => number) => {
  const seen: number[] = []
  return (v: number): number => {
    let r = seen[v]
    if (r === undefined) {
      r = fn(v)
      seen[v] = r
    }
    return r
  }
}
const deliverKindOf = memo(deliverKind)
const droppedOf = memo((v) => (isDropped(v) ? 1 : 0))
const orderingOf = memo(ordering)

/** A task as tasks.t27 names one: its kind, its repository, a number or an id. */
export interface TaskRef {
  kind: number
  repo: string
  number?: number
  id?: string
}

/** What a supervisor's own spawn adds: how it restarts its children. */
export interface SupervisorInfo {
  strategy: number
  maxRestarts: number
  periodSeconds: number
}

/** One event as the runtime hands it to the bus: a kind and its fields. */
export type ActorEventPayload = Record<string, string | number>
export type ActorEventSink = (kind: number, payload: ActorEventPayload) => void

export interface ActorEventsOptions {
  /** Where every event goes, in order: the bus writer, or a test's array. */
  sink: ActorEventSink
  /**
   * The task a message names, read by the domain that sends it; undefined
   * when it names none. Glue: it reads a field, the card decides what follows.
   */
  taskOf?: (msg: unknown) => TaskRef | undefined
}

/** The hooks the runtime calls with `options.events`, and their state. */
export interface ActorEvents {
  taskOf: (msg: unknown) => TaskRef | undefined
  /** A pid started: a spawn, or a restart when its supervisor says so. */
  spawned: (pid: Pid, actor: string, sup?: SupervisorInfo) => void
  /** Run `start` with `parent` as the parent of every pid it starts. */
  under: <T>(
    parent: Pid,
    restartCount: number,
    restart: boolean,
    start: () => T,
  ) => T
  /**
   * A pid ended. `task` is the task of the last data turn it started and
   * `inTurn` whether that turn had not ended; returns the task the exit names
   * (exit_task), for the DOWNs that follow it.
   */
  exited: (
    pid: Pid,
    reason: number,
    task: TaskRef | undefined,
    inTurn: boolean,
  ) => TaskRef | undefined
  /** `to` hears that `from` ended. */
  down: (from: Pid, to: Pid, reason: number, task: TaskRef | undefined) => void
  /** One send reached deliver(): its verdict and the receiver's depth after. */
  delivered: (
    from: Pid | undefined,
    to: Pid,
    verdict: number,
    depth: number,
    msg: unknown,
  ) => void
  /** Ask every open count whether its window is over. */
  sweep: () => void
  /** Write every open count; nothing is counted after this. */
  stop: () => void
  /** Pairs counted now, for a test and the status line. */
  openPairs: () => number
}

interface Pair {
  from: Pid
  to: Pid
  openedAt: number
  /** The host's arithmetic for when to ask flush_reason. */
  askAt: number
  count: number
  dropped: number
  maxDepth: number
  first: number
  last: number
  at: number
}

const NO_PID: Pid = 0n
const pidText = (pid: Pid | undefined): string => String(pid ?? NO_PID)
const isDown = (msg: unknown): boolean =>
  typeof msg === 'object' &&
  msg !== null &&
  (msg as { kind?: unknown }).kind === 'DOWN'

const taskFields = (task: TaskRef | undefined): ActorEventPayload => {
  if (task === undefined) return {}
  const out: ActorEventPayload = { task_kind: task.kind, task_repo: task.repo }
  if (task.number !== undefined) out.task_number = task.number
  if (task.id !== undefined) out.task_id = task.id
  return out
}

export function createActorEvents(
  clock: Clock,
  options: ActorEventsOptions,
): ActorEvents {
  const sink = options.sink
  const taskOf = options.taskOf ?? (() => undefined)
  // pairs by sender, then receiver; a send with no sender is NO_PID's
  const open = new Map<Pid, Map<Pid, Pair>>()
  let pairs = 0
  // every deliver takes the next number of this runtime (actor_events.t27
  // section 3): first_seq, last_seq and msg_seq
  let msgNo = 0
  const parentOf = new Map<Pid, Pid>()
  const frames: Array<{ parent: Pid; restarts: number; restart: boolean }> = []
  let sweeping: (() => void) | null = null
  let stopped = false

  const write = (p: Pair) => {
    sink(EV_ACTOR_DELIVERS, {
      at_ms: p.at,
      from_pid: pidText(p.from),
      to_pid: pidText(p.to),
      count: p.count,
      dropped: p.dropped,
      max_depth: p.maxDepth,
      first_seq: p.first,
      last_seq: p.last,
    })
  }
  const close = (p: Pair) => {
    const row = open.get(p.from)
    if (row?.delete(p.to)) {
      pairs--
      if (row.size === 0) open.delete(p.from)
    }
    write(p)
  }
  /** Close each pair the card says goes now; `ordered`, `full`, `stopping` as asked. */
  const flushWhere = (
    pick: (p: Pair) => boolean,
    ordered: boolean,
    full: boolean,
    stopping: boolean,
  ) => {
    const now = clock.now()
    const going: Pair[] = []
    for (const row of open.values())
      for (const p of row.values())
        if (
          pick(p) &&
          flushReason(now - p.openedAt, ordered, full, stopping) !== FL_KEEP
        )
          going.push(p)
    // the oldest window first, so a reader sees counts in the order they began
    going.sort((a, b) => a.first - b.first)
    for (const p of going) close(p)
  }
  /** Before an event of `kind` about (a, b) is written, the counts it must follow. */
  const order = (kind: number, a: Pid, b: Pid) => {
    const o = orderingOf(kind)
    if (o === OR_NONE || pairs === 0) return
    if (o === OR_PAIR) {
      const p = open.get(a)?.get(b)
      if (p) flushWhere((q) => q === p, true, false, false)
    } else if (o === OR_PID) {
      flushWhere((q) => q.from === a || q.to === a, true, false, false)
    }
  }
  const sweep = () => {
    sweeping = null
    if (pairs === 0) return
    const now = clock.now()
    flushWhere((p) => now >= p.askAt, false, false, false)
    armSweep()
  }
  const armSweep = () => {
    if (sweeping === null && pairs > 0 && !stopped)
      sweeping = clock.after(ACTOR_WINDOW_MS, sweep)
  }

  return {
    taskOf,
    spawned: (pid, actor, sup) => {
      if (stopped) return
      const frame = frames[frames.length - 1]
      const parent = frame?.parent ?? NO_PID
      parentOf.set(pid, parent)
      const kind = frame?.restart ? EV_ACTOR_RESTART : EV_ACTOR_SPAWN
      order(kind, parent, pid)
      sink(kind, {
        at_ms: clock.now(),
        from_pid: pidText(parent),
        to_pid: pidText(pid),
        parent_pid: pidText(parent),
        restart_count: frame?.restarts ?? 0,
        actor,
        ...(sup
          ? {
              supervisor: 1,
              strategy: sup.strategy,
              max_restarts: sup.maxRestarts,
              period_seconds: sup.periodSeconds,
            }
          : {}),
      })
    },
    under: (parent, restartCount, restart, start) => {
      frames.push({ parent, restarts: restartCount, restart })
      try {
        return start()
      } finally {
        frames.pop()
      }
    },
    exited: (pid, reason, turnTask, inTurn) => {
      const task =
        turnTask !== undefined && exitTask(turnTask.kind, inTurn)
          ? turnTask
          : undefined
      if (stopped) return task
      const parent = parentOf.get(pid) ?? NO_PID
      parentOf.delete(pid)
      order(EV_ACTOR_EXIT, pid, parent)
      sink(EV_ACTOR_EXIT, {
        at_ms: clock.now(),
        from_pid: pidText(pid),
        to_pid: pidText(parent),
        reason_class: reasonClass(reason),
        ...taskFields(task),
      })
      return task
    },
    down: (from, to, reason, task) => {
      if (stopped) return
      order(EV_ACTOR_DOWN, from, to)
      sink(EV_ACTOR_DOWN, {
        at_ms: clock.now(),
        from_pid: pidText(from),
        to_pid: pidText(to),
        reason_class: reasonClass(reason),
        ...taskFields(task),
      })
    },
    delivered: (from, to, verdict, depth, msg) => {
      // a DOWN is written as its own event where it is made (`down`)
      if (stopped || isDown(msg)) return
      const n = ++msgNo
      const sender = from ?? NO_PID
      const task = taskOf(msg)
      if (deliverKindOf(task?.kind ?? NO_TASK) === EV_ACTOR_DELIVER) {
        order(EV_ACTOR_DELIVER, sender, to)
        sink(EV_ACTOR_DELIVER, {
          at_ms: clock.now(),
          from_pid: pidText(sender),
          to_pid: pidText(to),
          msg_seq: n,
          depth,
          dropped: droppedOf(verdict),
          ...taskFields(task),
        })
        return
      }
      const now = clock.now()
      let row = open.get(sender)
      let p = row?.get(to)
      if (
        p !== undefined &&
        now >= p.askAt &&
        flushReason(now - p.openedAt, false, false, false) !== FL_KEEP
      ) {
        close(p)
        p = undefined
        row = open.get(sender)
      }
      if (p === undefined) {
        if (tableFull(pairs)) {
          flushWhere(() => true, false, true, false)
          row = open.get(sender)
        }
        p = {
          from: sender,
          to,
          openedAt: now,
          askAt: now + ACTOR_WINDOW_MS,
          count: 0,
          dropped: 0,
          maxDepth: 0,
          first: n,
          last: n,
          at: now,
        }
        if (row === undefined) {
          row = new Map()
          open.set(sender, row)
        }
        row.set(to, p)
        pairs++
        armSweep()
      }
      p.count++
      p.dropped += droppedOf(verdict)
      if (depth > p.maxDepth) p.maxDepth = depth
      p.last = n
      p.at = now
    },
    sweep,
    stop: () => {
      if (stopped) return
      sweeping?.()
      sweeping = null
      flushWhere(() => true, false, false, true)
      stopped = true
    },
    openPairs: () => pairs,
  }
}

/**
 * THE ONE WRITER. Events go to the bus in the order they were made, one
 * statement at a time (`append` is appendEvent on ACTOR_STREAM), so the
 * stream's numbers follow the runtime's order. While the store cannot write,
 * at most PUBLISH_BUFFER_MAX wait (buffer_admits); a newer event is counted as
 * lost and the feed shows the count.
 */
export interface ActorEventWriter {
  sink: ActorEventSink
  /** Events waiting for the bus now. */
  waiting: () => number
  /** Events the buffer refused since start. */
  lost: () => number
  /** Resolves once every event queued so far was written or failed. */
  drained: () => Promise<void>
}

export function actorEventWriter(
  append: (kind: number, payload: ActorEventPayload) => Promise<number>,
): ActorEventWriter {
  const queue: Array<{ kind: number; payload: ActorEventPayload }> = []
  let lost = 0
  let running: Promise<void> | null = null
  const pump = async () => {
    while (queue.length > 0) {
      const e = queue[0]
      try {
        await append(e.kind, e.payload)
        queue.shift()
      } catch (error) {
        // the store refused: the event waits and the writer tries again on
        // the next event, so the order is kept and nothing is skipped
        logger.warn('Queen actor events could not be written', {
          waiting: queue.length,
          error: error instanceof Error ? error.message : String(error),
        })
        return
      }
    }
  }
  const kick = () => {
    if (running) return
    running = pump().finally(() => {
      running = null
    })
  }
  return {
    sink: (kind, payload) => {
      if (!bufferAdmits(queue.length)) {
        lost++
        return
      }
      queue.push({ kind, payload })
      kick()
    },
    waiting: () => queue.length,
    lost: () => lost,
    drained: async () => {
      while (running) await running
    },
  }
}

let liveWriter: ActorEventWriter | undefined
/** The writer of this process's actor runtime, for the feed's `lost` count. */
export const setLiveActorEventWriter = (w: ActorEventWriter | undefined) => {
  liveWriter = w
}
export const liveActorEventWriter = (): ActorEventWriter | undefined =>
  liveWriter

/** TRIOS_QUEEN_ACTOR_EVENTS=on publishes; anything else, the default, does not. */
export const actorEventsOn = (env: NodeJS.ProcessEnv = process.env): boolean =>
  (env.TRIOS_QUEEN_ACTOR_EVENTS ?? 'off').toLowerCase() === 'on'

// --- what anyone may read ---------------------------------------------------

/** One row of the actors' stream, as the log or the replay holds it. */
export interface ActorRow {
  seq: number
  kind: number
  payload: Record<string, unknown>
  /** The log's own time of the row, when the payload has no at_ms. */
  at: string
}

export interface PublicTaskRef {
  kind: string
  repo: string
  number?: number
  id?: string
}

export interface PublicActorEvent {
  seq: number
  kind: number
  name: string
  at: string
  task_ref: PublicTaskRef | null
  [field: string]: string | number | PublicTaskRef | null
}

/**
 * The public projection of one row: its number, kind and time, the card's
 * ACTOR_PUBLIC_KEYS with a finite number or a token_ok string, and the task it
 * names. Every other key - a payload, a message, a title - is dropped. A row
 * whose kind is not an actor kind is not projected at all.
 */
export function publicActorEvent(
  row: ActorRow,
  tokenOk: (text: string) => boolean,
): PublicActorEvent | null {
  if (!actorKindOk(row.kind)) return null
  const fields: Record<string, string | number> = {}
  for (const key of ACTOR_PUBLIC_KEYS as readonly string[]) {
    const v = row.payload[key]
    if (typeof v === 'number' && Number.isFinite(v)) fields[key] = v
    else if (typeof v === 'string' && tokenOk(v)) fields[key] = v
  }
  const {
    task_kind,
    task_repo,
    task_number,
    task_id,
    reason_class,
    at_ms,
    ...rest
  } = fields
  let task: PublicTaskRef | null = null
  if (
    typeof task_kind === 'number' &&
    carriesTask(task_kind) &&
    typeof task_repo === 'string'
  ) {
    task = { kind: KIND_NAMES[task_kind] ?? String(task_kind), repo: task_repo }
    if (typeof task_number === 'number') task.number = task_number
    if (typeof task_id === 'string') task.id = task_id
  }
  const out: PublicActorEvent = {
    ...rest,
    seq: row.seq,
    kind: row.kind,
    name: ACTOR_EVENT_NAMES[row.kind] ?? 'unknown',
    at: typeof at_ms === 'number' ? new Date(at_ms).toISOString() : row.at,
    task_ref: task,
  }
  if (typeof reason_class === 'number')
    out.reason_class = RC_NAMES[reason_class] ?? 'crash'
  return out
}

/** Where a page is read from: the log's actors stream, or the replay. */
export interface ActorRowSource {
  bounds: () => Promise<{ oldest: number; newest: number }>
  after: (after: number, limit: number) => Promise<ActorRow[]>
}

export interface ActorEventsPage {
  /** Whether this answer reads anything: false with the flag off. */
  on: boolean
  source: 'bus' | 'replay' | 'off'
  cursor: number
  oldest: number
  newest: number
  /** The events right after `since` are gone; `cursor` is where reading resumes. */
  resync: boolean
  events: PublicActorEvent[]
  /** Events this process's writer could not keep (buffer_admits). */
  lost?: number
}

export const OFF_PAGE: ActorEventsPage = {
  on: false,
  source: 'off',
  cursor: 0,
  oldest: 0,
  newest: 0,
  resync: false,
  events: [],
}

/**
 * One read past `since` with the bus's own rules (events.t27 cursor_read and
 * page_size). With no `since`, the read starts before the oldest event kept.
 * There is no snapshot of the actors to take, so a resync answers the cursor
 * where reading resumes: right before the oldest event kept.
 */
export async function publicActorsPage(
  source: ActorRowSource,
  since: number | null,
  asked: number,
  cards: {
    cursorRead: (c: number, o: number, n: number) => number
    pageSize: (asked: number) => number
    tokenOk: (text: string) => boolean
  },
  kind: 'bus' | 'replay',
): Promise<ActorEventsPage> {
  const { oldest, newest } = await source.bounds()
  const start = oldest > 0 ? oldest - 1 : 0
  const from = since ?? start
  const read = cards.cursorRead(from, oldest, newest)
  const base = { on: true, source: kind, oldest, newest }
  if (read === CR_RESYNC)
    return { ...base, cursor: start, resync: true, events: [] }
  if (read !== CR_PAGE)
    return { ...base, cursor: from, resync: false, events: [] }
  const rows = await source.after(from, cards.pageSize(asked))
  const events: PublicActorEvent[] = []
  for (const r of rows) {
    const e = publicActorEvent(r, cards.tokenOk)
    if (e) events.push(e)
  }
  return {
    ...base,
    cursor: rows.length > 0 ? rows[rows.length - 1].seq : from,
    resync: false,
    events,
  }
}

// --- the replay: actor_events.t27 section 6 ----------------------------------

/**
 * The card's stand-in trace of the network MVP (t27#8610), as rows of the
 * actors' stream: row i is seq i + 1. Pids come from actors.t27 pid_of; every
 * other value is the card's. Fed through publicActorsPage, it is read exactly
 * as the bus is.
 */
export function replayRows(
  pidOf: (slot: number, gen: number) => Pid,
): ActorRow[] {
  const rows: ActorRow[] = []
  const pid = (slot: number, gen: number) =>
    slot === 0 ? NO_PID : pidOf(slot, gen)
  for (let i = 0; i < REPLAY_EVENTS; i++) {
    const kind = REPLAY_KIND[i]
    const from = pid(REPLAY_FROM_SLOT[i], REPLAY_FROM_GEN[i])
    const to = pid(REPLAY_TO_SLOT[i], REPLAY_TO_GEN[i])
    const at = REPLAY_T0_MS + REPLAY_AT_MS[i]
    const payload: ActorEventPayload = {
      at_ms: at,
      from_pid: pidText(from),
      to_pid: pidText(to),
    }
    if (REPLAY_TASK[i] > 0)
      Object.assign(
        payload,
        taskFields({
          kind: REPLAY_TASK_KIND,
          repo: REPLAY_REPO,
          number: REPLAY_TASK[i],
        }),
      )
    if (kind === EV_ACTOR_SPAWN || kind === EV_ACTOR_RESTART) {
      payload.parent_pid = pidText(from)
      payload.restart_count = REPLAY_RESTARTS[i]
      payload.actor = REPLAY_ACTOR_NAMES[REPLAY_TO_SLOT[i] - 1]
      if (REPLAY_TO_SLOT[i] === REPLAY_SUPERVISOR_SLOT)
        Object.assign(payload, {
          supervisor: 1,
          strategy: REPLAY_STRATEGY,
          max_restarts: REPLAY_MAX_RESTARTS,
          period_seconds: REPLAY_PERIOD_SECONDS,
        })
    } else if (kind === EV_ACTOR_EXIT || kind === EV_ACTOR_DOWN) {
      payload.reason_class = reasonClass(REPLAY_REASON[i])
    } else if (kind === EV_ACTOR_DELIVER) {
      payload.msg_seq = REPLAY_FIRST[i]
      payload.depth = REPLAY_DEPTH[i]
      payload.dropped = REPLAY_DROPPED[i]
    } else if (kind === EV_ACTOR_DELIVERS) {
      Object.assign(payload, {
        count: REPLAY_COUNT[i],
        dropped: REPLAY_DROPPED[i],
        max_depth: REPLAY_DEPTH[i],
        first_seq: REPLAY_FIRST[i],
        last_seq: REPLAY_LAST[i],
      })
    }
    rows.push({
      seq: i + 1,
      kind,
      payload,
      at: new Date(at).toISOString(),
    })
  }
  return rows
}

/** The replay as a source of rows, as the log is one. */
export function replaySource(rows: ActorRow[]): ActorRowSource {
  return {
    bounds: async () => ({
      oldest: rows.length > 0 ? rows[0].seq : 0,
      newest: rows.length > 0 ? rows[rows.length - 1].seq : 0,
    }),
    after: async (after, limit) =>
      rows.filter((r) => r.seq > after).slice(0, limit),
  }
}
