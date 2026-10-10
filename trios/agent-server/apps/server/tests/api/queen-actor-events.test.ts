/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE ACTORS ON THE BUS (gHashTag/t27 specs/queen/actor_events.t27 and
 * events.t27 section 6, gHashTag/trios#1744):
 *   1. the vendored cards are the ones PIN names, and the wasm runs the
 *      spec's own vectors;
 *   2. the runtime's hooks write spawn, exit, DOWN, restart and deliver as the
 *      card says - one by one for a task, counted per pair per window else,
 *      each pair in send order - on a virtual clock;
 *   3. the projection never carries a payload;
 *   4. the route pages the bus with events.t27's cursor rules, answers an
 *      empty page with the flag off, and replays the card's stand-in trace.
 * Everything runs on the VirtualClock seam; no fake timers.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { createQueenPublicActorsRoute } from '../../src/api/routes/queen-public-actors'
import {
  type ActorEventPayload,
  type ActorRow,
  actorEventsOn,
  actorEventWriter,
  actorKindOk,
  bufferAdmits,
  carriesTask,
  createActorEvents,
  deliverKind,
  exitTask,
  flushReason,
  isDropped,
  OFF_PAGE,
  ordering,
  publicActorEvent,
  publicActorsPage,
  reasonClass,
  replayRows,
  replaySource,
  tableFull,
} from '../../src/api/services/queen-actor-events'
import {
  ACTOR_PUBLIC_KEYS,
  ACTOR_WINDOW_MS,
  AGG_PAIRS_MAX,
  FL_FULL,
  FL_KEEP,
  FL_ORDER,
  FL_STOP,
  FL_WINDOW,
  NO_TASK,
  OR_NONE,
  OR_PAIR,
  OR_PID,
  PUBLISH_BUFFER_MAX,
  RC_CRASH,
  RC_NORMAL,
  RC_SHUTDOWN,
  REPLAY_EVENTS,
} from '../../src/api/services/queen-actor-events-card.gen'
import {
  actorChild,
  createActorSystem,
  type Pid,
  pidOf,
  realClock,
  slotOf,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  D_DROPPED_DEAD,
  D_DROPPED_FULL,
  D_QUEUED,
  MAILBOX_CAP,
  STRAT_ONE_FOR_ONE,
  X_CRASH,
  X_KILLED,
  X_NOCONNECTION,
  X_NOPROC,
  X_NORMAL,
  X_SHUTDOWN,
} from '../../src/api/services/queen-actors-card.gen'
import {
  cursorRead,
  pageSize,
  tokenOk,
} from '../../src/api/services/queen-events'
import {
  ACTOR_KINDS,
  EV_ACTOR_DELIVER,
  EV_ACTOR_DELIVERS,
  EV_ACTOR_DOWN,
  EV_ACTOR_EXIT,
  EV_ACTOR_RESTART,
  EV_ACTOR_SPAWN,
  EVENTS_PAGE_DEFAULT,
} from '../../src/api/services/queen-events.gen'
import { reviewTaskOf } from '../../src/api/services/queen-review-actors'
import {
  TASK_KINDS,
  TK_ISSUE,
  TK_JOB,
} from '../../src/api/services/queen-tasks.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

const cards = { cursorRead, pageSize, tokenOk }
const REPO = 'gHashTag/t27'
const taskOf = reviewTaskOf(REPO)

interface Row {
  kind: number
  payload: ActorEventPayload
}

/** A system whose events land in `rows`, on a virtual clock, slices off. */
const world = () => {
  const clock = new VirtualClock()
  const rows: Row[] = []
  const events = createActorEvents(clock, {
    sink: (kind, payload) => rows.push({ kind, payload }),
    taskOf,
  })
  const sys = createActorSystem(clock, { slices: false, events })
  return { clock, rows, events, sys }
}
const kinds = (rows: Row[]) => rows.map((r) => r.kind)
const text = (pid: Pid) => String(pid)

describe('the vendored cards are the ones PIN names', () => {
  it('actor_events.t27 and its wasm match, and the wasm exports every fn', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/actor_events.t27 sha256 ${sha('queen/actor_events.t27')}`,
    )
    expect(pin).toContain(
      `queen/actor_events.wasm sha256 ${sha('queen/actor_events.wasm')}`,
    )
    const spec = readFileSync(
      join(DEFAULT_SPECS_ROOT, 'queen/actor_events.t27'),
      'utf8',
    )
    const fns = [...spec.matchAll(/^pub fn (\w+)\(/gm)].map((m) => m[1])
    const exported = WebAssembly.Module.exports(
      new WebAssembly.Module(
        readFileSync(join(DEFAULT_SPECS_ROOT, 'queen/actor_events.wasm')),
      ),
    ).map((e) => e.name)
    expect(fns.length).toBe(9)
    for (const fn of fns) expect(exported).toContain(fn)
  })

  it('events.wasm exports actor_kind_ok, the fn section 6 added', () => {
    const exported = WebAssembly.Module.exports(
      new WebAssembly.Module(
        readFileSync(join(DEFAULT_SPECS_ROOT, 'queen/events.wasm')),
      ),
    ).map((e) => e.name)
    expect(exported).toContain('actor_kind_ok')
  })
})

describe('the card, as the wasm runs it', () => {
  // test a_deliver_naming_a_task_is_written_one_by_one
  it('writes a deliver that names a task one by one, and counts the rest', () => {
    expect(carriesTask(TK_ISSUE)).toBe(true)
    expect(carriesTask(TK_JOB)).toBe(true)
    expect(carriesTask(TASK_KINDS)).toBe(false)
    expect(carriesTask(NO_TASK)).toBe(false)
    expect(deliverKind(TK_ISSUE)).toBe(EV_ACTOR_DELIVER)
    expect(deliverKind(NO_TASK)).toBe(EV_ACTOR_DELIVERS)
  })

  // test an_exit_names_the_task_of_the_turn_it_cut / a_refused_deliver_is_counted_as_dropped
  it('names the task of a cut turn, and counts a refused deliver', () => {
    expect(exitTask(TK_ISSUE, true)).toBe(true)
    expect(exitTask(TK_ISSUE, false)).toBe(false)
    expect(exitTask(NO_TASK, true)).toBe(false)
    expect(isDropped(D_QUEUED)).toBe(false)
    expect(isDropped(D_DROPPED_DEAD)).toBe(true)
    expect(isDropped(D_DROPPED_FULL)).toBe(true)
  })

  // test every_exit_reason_has_one_of_three_classes
  it('folds every exit reason into normal, shutdown or crash', () => {
    expect(reasonClass(X_NORMAL)).toBe(RC_NORMAL)
    expect(reasonClass(X_SHUTDOWN)).toBe(RC_SHUTDOWN)
    for (const r of [X_CRASH, X_KILLED, X_NOPROC, X_NOCONNECTION, 99, 255])
      expect(reasonClass(r)).toBe(RC_CRASH)
  })

  // test an_event_writes_the_counts_it_must_follow / a_count_is_written_at_its_window_or_before_what_follows_it / the_buffer_is_bounded
  it('orders, flushes and bounds as the spec says', () => {
    expect(ordering(EV_ACTOR_DELIVER)).toBe(OR_PAIR)
    expect(ordering(EV_ACTOR_DOWN)).toBe(OR_PAIR)
    expect(ordering(EV_ACTOR_EXIT)).toBe(OR_PID)
    expect(ordering(EV_ACTOR_SPAWN)).toBe(OR_NONE)
    expect(ordering(EV_ACTOR_RESTART)).toBe(OR_NONE)
    expect(flushReason(ACTOR_WINDOW_MS - 1, false, false, false)).toBe(FL_KEEP)
    expect(flushReason(ACTOR_WINDOW_MS, false, false, false)).toBe(FL_WINDOW)
    expect(flushReason(0, true, false, false)).toBe(FL_ORDER)
    expect(flushReason(0, false, true, false)).toBe(FL_FULL)
    expect(flushReason(0, true, true, true)).toBe(FL_STOP)
    expect(tableFull(AGG_PAIRS_MAX - 1)).toBe(false)
    expect(tableFull(AGG_PAIRS_MAX)).toBe(true)
    expect(bufferAdmits(PUBLISH_BUFFER_MAX - 1)).toBe(true)
    expect(bufferAdmits(PUBLISH_BUFFER_MAX)).toBe(false)
  })

  // events.t27 test the_actors_have_kinds_of_their_own
  it('knows the six actor kinds and no seventh', () => {
    for (let k = 0; k < ACTOR_KINDS; k++) expect(actorKindOk(k)).toBe(true)
    expect(actorKindOk(ACTOR_KINDS)).toBe(false)
  })
})

describe('the runtime writes what the card says', () => {
  it('a supervisor names itself and its children; a task deliver is one event; heartbeats are one count per window', async () => {
    const { clock, rows, sys } = world()
    let dispatcher: Pid = 0n
    let worker: Pid = 0n
    const handled: unknown[] = []
    const sup = supervisor(
      sys,
      {
        name: 'net',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [
        actorChild(sys, {
          name: 'dispatcher',
          init: (self) => {
            dispatcher = self
          },
          receive: () => {},
        }),
        actorChild(sys, {
          name: 'worker-1',
          kind: 'worker',
          init: (self) => {
            worker = self
          },
          receive: (m) => {
            handled.push(m)
          },
        }),
      ],
    ).start(() => {})
    expect(sup.pid).toBeDefined()
    const supPid = sup.pid as Pid
    expect(kinds(rows)).toEqual([
      EV_ACTOR_SPAWN,
      EV_ACTOR_SPAWN,
      EV_ACTOR_SPAWN,
    ])
    expect(rows[0].payload).toMatchObject({
      to_pid: text(supPid),
      parent_pid: '0',
      actor: 'net',
      supervisor: 1,
      strategy: STRAT_ONE_FOR_ONE,
      max_restarts: 3,
      period_seconds: 300,
      restart_count: 0,
    })
    expect(rows[1].payload).toMatchObject({
      from_pid: text(supPid),
      to_pid: text(dispatcher),
      parent_pid: text(supPid),
      actor: 'dispatcher',
    })
    expect(rows[2].payload).toMatchObject({ actor: 'worker' })

    // five heartbeats in one window are one count, not five events
    for (let i = 0; i < 5; i++) sys.send(worker, { beat: i }, dispatcher)
    expect(rows.length).toBe(3)
    await clock.runUntil(400)
    sys.send(worker, { beat: 5 }, dispatcher)
    expect(rows.length).toBe(3)
    // the window is over at its first deliver + ACTOR_WINDOW_MS: the next
    // deliver of the pair writes the count and opens a new one
    await clock.runUntil(ACTOR_WINDOW_MS)
    // the sweep at ACTOR_WINDOW_MS found the window over and wrote it
    expect(kinds(rows).slice(3)).toEqual([EV_ACTOR_DELIVERS])
    expect(rows[3].payload).toMatchObject({
      from_pid: text(dispatcher),
      to_pid: text(worker),
      count: 6,
      dropped: 0,
      first_seq: 1,
      last_seq: 6,
    })
    expect(rows[3].payload.max_depth).toBeGreaterThanOrEqual(1)
    expect(handled.length).toBe(6)
  })

  it('a count is written before a task deliver of its pair, so the pair keeps its send order', async () => {
    const { clock, rows, sys } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    const b = sys.spawn({ name: 'b', receive: () => {} })
    const c = sys.spawn({ name: 'c', receive: () => {} })
    rows.length = 0
    sys.send(b, { beat: 1 }, a)
    sys.send(c, { beat: 1 }, a)
    sys.send(b, { beat: 2 }, a)
    sys.send(b, { issue: 7 }, a)
    // a->b's count goes first; a->c's stays open: another pair's order is its own
    expect(kinds(rows)).toEqual([EV_ACTOR_DELIVERS, EV_ACTOR_DELIVER])
    expect(rows[0].payload).toMatchObject({
      to_pid: text(b),
      count: 2,
      first_seq: 1,
      last_seq: 3,
    })
    expect(rows[1].payload).toMatchObject({
      from_pid: text(a),
      to_pid: text(b),
      msg_seq: 4,
      dropped: 0,
      task_kind: TK_ISSUE,
      task_repo: REPO,
      task_number: 7,
    })
    await clock.runUntil(10 * ACTOR_WINDOW_MS)
    expect(kinds(rows)).toEqual([
      EV_ACTOR_DELIVERS,
      EV_ACTOR_DELIVER,
      EV_ACTOR_DELIVERS,
    ])
    expect(rows[2].payload).toMatchObject({ to_pid: text(c), count: 1 })
  })

  it('a crash in a task turn ends the task: exit and DOWN name it, then the restart in the same slot', async () => {
    const { clock, rows, sys } = world()
    let dispatcher: Pid = 0n
    let worker: Pid = 0n
    const sup = supervisor(
      sys,
      {
        name: 'net',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 3,
        periodSeconds: 300,
      },
      [
        actorChild(sys, {
          name: 'dispatcher',
          init: (self) => {
            dispatcher = self
          },
          receive: () => {},
        }),
        actorChild(sys, {
          name: 'worker',
          init: (self) => {
            worker = self
          },
          receive: (m: { issue?: number; crash?: boolean }) => {
            if (m.crash) throw new Error('boom')
          },
        }),
      ],
    ).start(() => {})
    const supPid = sup.pid as Pid
    const first = worker
    sys.monitor(dispatcher, worker)
    sys.send(worker, { beat: 1 }, dispatcher)
    rows.length = 0
    sys.send(worker, { issue: 8, crash: true }, dispatcher)
    await clock.runUntil(1)
    const k = kinds(rows)
    expect(k.slice(0, 4)).toEqual([
      EV_ACTOR_DELIVERS,
      EV_ACTOR_DELIVER,
      EV_ACTOR_EXIT,
      EV_ACTOR_DOWN,
    ])
    const task = { task_kind: TK_ISSUE, task_repo: REPO, task_number: 8 }
    expect(rows[2].payload).toMatchObject({
      from_pid: text(first),
      to_pid: text(supPid),
      reason_class: RC_CRASH,
      ...task,
    })
    expect(rows[3].payload).toMatchObject({
      from_pid: text(first),
      to_pid: text(dispatcher),
      reason_class: RC_CRASH,
      ...task,
    })
    // the backoff passes; the supervisor restarts the worker in its slot
    await clock.runUntil(10 * 60_000)
    const restart = rows.find((r) => r.kind === EV_ACTOR_RESTART)
    expect(restart).toBeDefined()
    expect(restart?.payload).toMatchObject({
      from_pid: text(supPid),
      parent_pid: text(supPid),
      restart_count: 1,
      actor: 'worker',
    })
    expect(worker).not.toBe(first)
    expect(slotOf(worker)).toBe(slotOf(first))
    expect(String(restart?.payload.to_pid)).toBe(text(worker))
    // the DOWN was written once, as its own event: its deliver is not counted
    expect(
      rows.filter(
        (r) =>
          r.kind === EV_ACTOR_DELIVERS && r.payload.to_pid === text(dispatcher),
      ),
    ).toEqual([])
  })

  it('an exit between turns names no task, and writes the counts of its pid first', async () => {
    const { clock, rows, sys } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    const b = sys.spawn({ name: 'b', receive: () => {} })
    sys.send(b, { issue: 3 }, a)
    await clock.runUntil(1)
    sys.send(b, { beat: 1 }, a)
    rows.length = 0
    sys.exit(b, X_NORMAL)
    expect(kinds(rows)).toEqual([EV_ACTOR_DELIVERS, EV_ACTOR_EXIT])
    expect(rows[1].payload).toMatchObject({
      from_pid: text(b),
      reason_class: RC_NORMAL,
    })
    expect(rows[1].payload.task_kind).toBeUndefined()
  })

  it('a full mailbox: the count says how many were refused and how deep it got', async () => {
    const { rows, sys, events } = world()
    const stuck = sys.spawn({
      name: 'stuck',
      receive: () => new Promise<void>(() => {}),
    })
    rows.length = 0
    for (let i = 0; i < MAILBOX_CAP + 44; i++) sys.send(stuck, { beat: i })
    expect(rows.length).toBe(0)
    events.stop()
    expect(kinds(rows)).toEqual([EV_ACTOR_DELIVERS])
    expect(rows[0].payload).toMatchObject({
      from_pid: '0',
      to_pid: text(stuck),
      count: MAILBOX_CAP + 44,
      dropped: 44,
      max_depth: MAILBOX_CAP,
    })
    // stopped: nothing more is written
    sys.send(stuck, { beat: 1 })
    expect(rows.length).toBe(1)
  })

  it('a full table writes every count before a new pair opens', () => {
    const { rows, sys, events } = world()
    const to: Pid[] = []
    for (let i = 0; i <= AGG_PAIRS_MAX; i++)
      to.push(sys.spawn({ name: `r${i}`, receive: () => {} }))
    rows.length = 0
    for (let i = 0; i < AGG_PAIRS_MAX; i++) sys.send(to[i], { beat: i })
    expect(events.openPairs()).toBe(AGG_PAIRS_MAX)
    expect(rows.length).toBe(0)
    sys.send(to[AGG_PAIRS_MAX], { beat: 0 })
    expect(rows.length).toBe(AGG_PAIRS_MAX)
    expect(events.openPairs()).toBe(1)
    // the oldest window first
    expect(rows[0].payload.to_pid).toBe(text(to[0]))
  })

  it('a send from a dead pid is counted as dropped', () => {
    const { rows, sys, events } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    const b = sys.spawn({ name: 'b', receive: () => {} })
    sys.exit(a, X_SHUTDOWN)
    rows.length = 0
    sys.send(b, { beat: 1 }, a)
    events.stop()
    expect(rows[0].payload).toMatchObject({ count: 1, dropped: 1 })
  })

  it('the runtime takes the same turns with the events off and on', async () => {
    const run = async (on: boolean) => {
      const clock = new VirtualClock()
      const events = on
        ? createActorEvents(clock, { sink: () => {}, taskOf })
        : undefined
      const sys = createActorSystem(clock, { slices: false, events })
      const seen: string[] = []
      const N = 20
      const ring: Pid[] = []
      for (let i = 0; i < N; i++)
        ring.push(
          sys.spawn<{ hop: number; issue?: number }>({
            name: `r${i}`,
            receive: (m, self) => {
              seen.push(`${i}:${m.hop}@${clock.now()}`)
              if (m.hop < 500)
                sys.send(
                  ring[(i + 1) % N],
                  {
                    hop: m.hop + 1,
                    ...(m.hop % 7 === 0 ? { issue: m.hop } : {}),
                  },
                  self,
                )
            },
          }),
        )
      sys.send(ring[0], { hop: 0 })
      await clock.runUntil(5 * ACTOR_WINDOW_MS)
      return { seen, delivered: sys.stats.delivered }
    }
    const off = await run(false)
    const on = await run(true)
    expect(on.delivered).toBe(off.delivered)
    expect(on.seen).toEqual(off.seen)
    // without `events`, the supervisor takes no pid: pids are as before
    const plain = createActorSystem(new VirtualClock(), { slices: false })
    const h = supervisor(
      plain,
      {
        name: 's',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 1,
        periodSeconds: 1,
      },
      [],
    ).start(() => {})
    expect(h.pid).toBeUndefined()
  })

  it('the reviewer names the issue of an offer, a done and a release, and nothing else', () => {
    expect(taskOf({ issue: 12 })).toEqual({
      kind: TK_ISSUE,
      repo: REPO,
      number: 12,
    })
    expect(taskOf({ kind: 'done', worker: 1n, issue: 4 })).toMatchObject({
      number: 4,
    })
    expect(taskOf({ kind: 'wake' })).toBeUndefined()
    expect(taskOf({ kind: 'up', worker: 3n })).toBeUndefined()
    expect(taskOf({ kind: 'DOWN', pid: 1n, reason: 4 })).toBeUndefined()
    expect(taskOf(null)).toBeUndefined()
    expect(taskOf({ issue: '12' })).toBeUndefined()
  })
})

describe('what anyone may read', () => {
  it('a message that carries a payload: the event holds none of it, and the projection drops what a row adds', async () => {
    const { clock, rows, sys } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    const b = sys.spawn({ name: 'b', receive: () => {} })
    rows.length = 0
    const secret = {
      issue: 9,
      title: 'Queen: read the operator token from the env',
      body: 'a long issue body with a URL https://x.test/?k=v',
      args: ['--token', 'abc'],
      payload: { prompt: 'hidden' },
    }
    sys.send(b, secret, a)
    await clock.runUntil(1)
    expect(rows.length).toBe(1)
    const keys = Object.keys(rows[0].payload)
    for (const k of keys)
      expect(ACTOR_PUBLIC_KEYS as readonly string[]).toContain(k)
    const flat = JSON.stringify(rows[0].payload)
    for (const leak of [
      'title',
      'body',
      'args',
      'payload',
      'operator',
      'hidden',
      'abc',
    ])
      expect(flat).not.toContain(leak)

    // a row that holds more than the hooks write is still projected bare
    const row: ActorRow = {
      seq: 5,
      kind: EV_ACTOR_DELIVER,
      at: '2026-10-10T00:00:00.000Z',
      payload: {
        ...rows[0].payload,
        payload: secret,
        msg: 'hello there',
        title: secret.title,
        actor: 'two words',
        reason_class: 1,
        count: Number.NaN,
      },
    }
    const e = publicActorEvent(row, tokenOk)
    expect(e).not.toBeNull()
    const out = JSON.stringify(e)
    for (const leak of ['payload', 'hello', 'title', 'two words', 'operator'])
      expect(out).not.toContain(leak)
    expect(e?.task_ref).toEqual({ kind: 'issue', repo: REPO, number: 9 })
    expect(e?.reason_class).toBe('shutdown')
    expect(e?.count).toBeUndefined()
    expect(e?.from_pid).toBe(text(a))
    // a row of a kind the actors' stream does not carry is not projected
    expect(publicActorEvent({ ...row, kind: ACTOR_KINDS }, tokenOk)).toBeNull()
  })

  it('a page past a cursor follows events.t27: page, nothing, resync, page size', async () => {
    const { rows, sys } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    for (let i = 0; i < 30; i++) sys.send(a, { issue: i })
    const log: ActorRow[] = rows.map((r, i) => ({
      seq: i + 1,
      kind: r.kind,
      payload: r.payload,
      at: '',
    }))
    expect(log.length).toBe(31)
    const src = replaySource(log)
    const p1 = await publicActorsPage(src, 0, 10, cards, 'bus')
    expect(p1.events.map((e) => e.seq)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    expect(p1.cursor).toBe(10)
    const p2 = await publicActorsPage(src, p1.cursor, 1000, cards, 'bus')
    expect(p2.events.length).toBe(21)
    expect(p2.cursor).toBe(31)
    const p3 = await publicActorsPage(src, 31, 0, cards, 'bus')
    expect(p3.events).toEqual([])
    expect(p3.cursor).toBe(31)
    // no cursor: from the oldest kept; the default page size
    const p4 = await publicActorsPage(src, null, 0, cards, 'bus')
    expect(p4.events.length).toBe(Math.min(31, EVENTS_PAGE_DEFAULT))
    // pruned under the reader: resync, resume right before the oldest kept
    const pruned = replaySource(log.slice(11))
    const p5 = await publicActorsPage(pruned, 3, 10, cards, 'bus')
    expect(p5.resync).toBe(true)
    expect(p5.cursor).toBe(11)
    expect(p5.events).toEqual([])
    // the log was reset under the reader
    const p6 = await publicActorsPage(src, 99, 10, cards, 'bus')
    expect(p6.resync).toBe(true)
  })
})

describe('the writer between the runtime and the bus', () => {
  it('writes in order, keeps an event the store refused, and counts what the buffer cannot hold', async () => {
    const written: number[] = []
    let fail = 1
    const w = actorEventWriter(async (_kind, payload) => {
      if (fail > 0) {
        fail--
        throw new Error('store down')
      }
      written.push(Number(payload.n))
      return written.length
    })
    w.sink(EV_ACTOR_SPAWN, { n: 1 })
    await w.drained()
    expect(written).toEqual([])
    expect(w.waiting()).toBe(1)
    w.sink(EV_ACTOR_SPAWN, { n: 2 })
    w.sink(EV_ACTOR_SPAWN, { n: 3 })
    await w.drained()
    expect(written).toEqual([1, 2, 3])
    expect(w.lost()).toBe(0)

    const hung = actorEventWriter(() => new Promise<number>(() => {}))
    for (let i = 0; i < PUBLISH_BUFFER_MAX + 3; i++)
      hung.sink(EV_ACTOR_DELIVERS, { n: i })
    expect(hung.waiting()).toBe(PUBLISH_BUFFER_MAX)
    expect(hung.lost()).toBe(3)
  })
})

describe('GET /queen/public-actors', () => {
  const ask = async (
    path: string,
    env: NodeJS.ProcessEnv,
    rows: ActorRow[] = [],
  ) => {
    const app = createQueenPublicActorsRoute({
      env,
      source: () => replaySource(rows),
    })
    const res = await app.request(path)
    return {
      status: res.status,
      body: (await res.json()) as Record<string, unknown>,
    }
  }

  it('is off by default: nothing is published and the page is empty', async () => {
    expect(actorEventsOn({})).toBe(false)
    expect(actorEventsOn({ TRIOS_QUEEN_ACTOR_EVENTS: 'off' })).toBe(false)
    expect(actorEventsOn({ TRIOS_QUEEN_ACTOR_EVENTS: 'ON' })).toBe(true)
    const r = await ask('/?since=0', {}, [
      { seq: 1, kind: EV_ACTOR_SPAWN, payload: { to_pid: '1' }, at: '' },
    ])
    expect(r.status).toBe(200)
    expect(r.body).toEqual({ ...OFF_PAGE })
  })

  it('with the flag on, it pages the actors stream', async () => {
    const { rows, sys } = world()
    const a = sys.spawn({ name: 'a', receive: () => {} })
    sys.send(a, { issue: 1, title: 'not public' })
    const log = rows.map((r, i) => ({
      seq: i + 1,
      kind: r.kind,
      payload: r.payload,
      at: '',
    }))
    const r = await ask(
      '/?since=0&limit=1',
      { TRIOS_QUEEN_ACTOR_EVENTS: 'on' },
      log,
    )
    expect(r.status).toBe(200)
    expect(r.body).toMatchObject({
      on: true,
      source: 'bus',
      cursor: 1,
      newest: 2,
    })
    const events = r.body.events as Array<Record<string, unknown>>
    expect(events.map((e) => e.name)).toEqual(['actor.spawn'])
    const r2 = await ask('/?since=1', { TRIOS_QUEEN_ACTOR_EVENTS: 'on' }, log)
    const e2 = (r2.body.events as Array<Record<string, unknown>>)[0]
    expect(e2.name).toBe('actor.deliver')
    expect(JSON.stringify(e2)).not.toContain('not public')
  })

  it("replays the card's stand-in trace through the same projection, flag or not", async () => {
    const r = await ask('/?replay=mvp&since=0', {})
    expect(r.status).toBe(200)
    expect(r.body).toMatchObject({ on: true, source: 'replay', oldest: 1 })
    expect(r.body.newest).toBe(REPLAY_EVENTS)
    const events = r.body.events as Array<Record<string, unknown>>
    expect(events.length).toBe(REPLAY_EVENTS)
    expect(events.map((e) => e.seq)).toEqual(
      Array.from({ length: REPLAY_EVENTS }, (_, i) => i + 1),
    )
    // the root supervisor, then its six children
    expect(events[0]).toMatchObject({
      name: 'actor.spawn',
      actor: 'mvp-root',
      supervisor: 1,
      from_pid: '0',
      to_pid: String(pidOf(1, 1)),
    })
    expect(events[1]).toMatchObject({
      actor: 'dispatcher',
      parent_pid: String(pidOf(1, 1)),
    })
    // the crash on job 2: exit and DOWN name it, then the restart in slot 5
    const exit = events[14]
    expect(exit).toMatchObject({
      name: 'actor.exit',
      reason_class: 'crash',
      task_ref: { kind: 'job', repo: REPO, number: 2 },
    })
    expect(events[15]).toMatchObject({
      name: 'actor.down',
      reason_class: 'crash',
    })
    expect(events[16]).toMatchObject({
      name: 'actor.restart',
      restart_count: 1,
      to_pid: String(pidOf(5, 2)),
    })
    // the burst to the treasurer
    expect(events[22]).toMatchObject({
      name: 'actor.delivers',
      count: 4,
      dropped: 1,
      max_depth: MAILBOX_CAP,
      first_seq: 22,
      last_seq: 25,
      task_ref: null,
    })
    expect(events[23]).toMatchObject({
      name: 'actor.exit',
      reason_class: 'normal',
    })
    expect(events[0].at).toBe('2026-10-10T00:00:00.000Z')
    // every key is one the card lists, or the page's own
    const own = ['seq', 'kind', 'name', 'at', 'task_ref']
    for (const e of events)
      for (const k of Object.keys(e))
        expect([...own, ...(ACTOR_PUBLIC_KEYS as readonly string[])]).toContain(
          k,
        )
  })

  it('pages the replay with the same cursor rules', async () => {
    const p1 = await ask('/?replay=mvp&since=0&limit=10', {})
    expect(p1.body.cursor).toBe(10)
    const p2 = await ask(`/?replay=mvp&since=${p1.body.cursor}&limit=10`, {})
    expect((p2.body.events as unknown[]).length).toBe(10)
    const p3 = await ask('/?replay=mvp&since=24', {})
    expect(p3.body.events).toEqual([])
    const bad = await ask('/?replay=other', {})
    expect(bad.status).toBe(400)
  })

  it("the replay rows are the card's, row for row", () => {
    const rows = replayRows((slot, gen) => pidOf(slot, gen))
    expect(rows.length).toBe(REPLAY_EVENTS)
    const restart = rows[16]
    expect(restart.kind).toBe(EV_ACTOR_RESTART)
    expect(restart.payload.to_pid).toBe(String(pidOf(5, 2)))
  })
})

describe('the cost per message, events off and on', () => {
  // The ring of queen-actors-bench.test.ts: 1000 actors pass 100 000
  // messages, every decision asked of the wasm card. Rounds alternate off and
  // on; the medians are the result. The ring is the hard case for counting:
  // 1000 pairs open at once. With AGG_PAIRS_MAX at 256 the table filled and
  // emptied as the ring went round, about one event per message (t27#8650);
  // at 4096 each pair is one count a window. The numbers for the PR and
  // t27#7851 come from the same ring run against the tree before this change.
  const ringOnce = async (on: boolean) => {
    let written = 0
    const events = on
      ? createActorEvents(realClock, {
          sink: () => {
            written++
          },
          taskOf,
        })
      : undefined
    const sys = createActorSystem(undefined, events ? { events } : {})
    const N = 1000
    const HOPS = 100_000
    const ring: Pid[] = []
    let left = HOPS
    let finish: () => void = () => {}
    const done = new Promise<void>((r) => {
      finish = r
    })
    for (let i = 0; i < N; i++)
      ring.push(
        sys.spawn<number>({
          name: `r${i}`,
          receive: (hop, self) => {
            left--
            if (left === 0) finish()
            else sys.send(ring[(i + 1) % N], hop + 1, self)
          },
        }),
      )
    const cpu0 = process.cpuUsage()
    sys.send(ring[0], 0)
    await done
    const used = process.cpuUsage(cpu0)
    events?.stop()
    expect(sys.stats.delivered).toBe(HOPS)
    return { cpu: (used.user + used.system) / HOPS, written }
  }

  it('in one window the ring writes its 1000 spawns and one count per pair, not one event per message', async () => {
    const clock = new VirtualClock()
    let written = 0
    const kinds = new Map<number, number>()
    const events = createActorEvents(clock, {
      sink: (kind) => {
        written++
        kinds.set(kind, (kinds.get(kind) ?? 0) + 1)
      },
      taskOf,
    })
    const sys = createActorSystem(clock, { slices: false, events })
    const N = 1000
    const HOPS = 100_000
    const ring: Pid[] = []
    let left = HOPS
    let finish: () => void = () => {}
    const done = new Promise<void>((r) => {
      finish = r
    })
    for (let i = 0; i < N; i++)
      ring.push(
        sys.spawn<number>({
          name: `r${i}`,
          receive: (hop, self) => {
            left--
            if (left === 0) finish()
            else sys.send(ring[(i + 1) % N], hop + 1, self)
          },
        }),
      )
    sys.send(ring[0], 0)
    await done
    // the ring's N pairs, and the first send from no one
    expect(events.openPairs()).toBe(N + 1)
    events.stop()
    expect(kinds.get(EV_ACTOR_SPAWN)).toBe(N)
    expect(kinds.get(EV_ACTOR_DELIVERS)).toBe(N + 1)
    expect(written).toBe(2 * N + 1)
  }, 600_000)

  it('a ring of 1000 actors passes 100 000 messages, off and on', async () => {
    const ROUNDS = Number(process.env.QUEEN_ACTOR_EVENTS_BENCH_ROUNDS ?? 3)
    await ringOnce(false)
    await ringOnce(true)
    const off: number[] = []
    const on: number[] = []
    let written = 0
    for (let k = 0; k < ROUNDS; k++) {
      off.push((await ringOnce(false)).cpu)
      const r = await ringOnce(true)
      on.push(r.cpu)
      written = r.written
    }
    const median = (xs: number[]) =>
      [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]
    // the count per window is real time here, so a loaded host writes more
    // windows; the exact volume is asked of a virtual clock below
    expect(written).toBeGreaterThan(1000)
    console.log(
      `\n## actor events cost per message, cpu us (${ROUNDS} rounds): off ${median(off).toFixed(2)}, on ${median(on).toFixed(2)}; events written per run with it on: ${written}`,
    )
  }, 600_000)
})
