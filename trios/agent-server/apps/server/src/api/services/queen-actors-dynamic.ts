/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * A DYNAMIC SUPERVISOR (gHashTag/t27 specs/queen/actors.t27 section 5,
 * max_children, "as Elixir's DynamicSupervisor"). Children are started on
 * demand, one at a time, and stopped on demand. `supervisor` in queen-actors.ts
 * starts a fixed list and keeps it; this one grows and shrinks.
 *
 * The card decides, as it does for the fixed supervisor:
 *   - whether a start is allowed (start_answer against the max_children the
 *     caller passes - a lowered bound refuses new starts and stops nobody);
 *   - what a child's exit does (on_child_exit, unstable_streak, with_backoff),
 *     how long a restart waits (backoff_seconds, jittered_seconds), and how
 *     many children remain afterwards (children_after_exit);
 *   - when the supervisor gives up (in_period, intensity) - then it stops every
 *     child and reports GIVE_UP_REASON to its parent.
 * The strategy is one_for_one: a child's crash touches no sibling.
 *
 * A stop asked for here is synchronous (sys.exit), so no child is ever
 * "stopping": start_answer is asked with 0 for it.
 */

import {
  ACTORS_CARD,
  type ActorSystem,
  type Child,
  type Pid,
  slotOf,
} from './queen-actors'
import {
  GIVE_UP_REASON,
  RESTART_PERMANENT,
  START_MAX_CHILDREN,
  STRAT_ONE_FOR_ONE,
  SUP_GIVE_UP,
  SUP_RESTART,
  X_SHUTDOWN,
} from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'

const card = () => loadCardWasm(ACTORS_CARD)
const c = (name: string, ...a: number[]) => card().call(name, ...a)

export interface DynamicSupervisorOptions {
  name: string
  maxRestarts: number
  periodSeconds: number
}

/** The handle a dynamic supervisor gives whoever sizes it. */
export interface DynamicChildren {
  /** Start one child if start_answer allows it under `maxChildren`. */
  startChild: (maxChildren: number) => number
  /** Stop one child by pid. It is not restarted, and its place is freed. */
  stopChild: (pid: Pid) => void
  /** Children started and not stopped, a restarting one included. */
  live: () => number
}

/**
 * A supervisor whose children are made by `make` when a start is asked for.
 * `ready` hears each start of the supervisor itself (a restart included) with
 * the handle that sizes it.
 */
export function dynamicSupervisor(
  sys: ActorSystem,
  opts: DynamicSupervisorOptions,
  make: (n: number) => Child,
  ready: (children: DynamicChildren) => void,
): Child {
  return {
    name: opts.name,
    restart: RESTART_PERMANENT,
    start: (onExit, slot) => {
      const clock = sys.clock
      // with events on, its own pid, the parent its children name
      const ev = sys.events
      const self = ev !== undefined ? sys.reservePid(slot) : undefined
      if (ev !== undefined && self !== undefined)
        ev.spawned(self, opts.name, {
          strategy: STRAT_ONE_FOR_ONE,
          maxRestarts: opts.maxRestarts,
          periodSeconds: opts.periodSeconds,
        })
      // the supervisor's own end, on the feed: a give-up or a stop
      const ended = (reason: number) => {
        if (ev !== undefined && self !== undefined)
          ev.exited(self, reason, undefined, false)
      }
      interface Kid {
        child: Child
        running?: { pid?: Pid; stop: () => void }
        slot?: bigint
        startedAt: number
        streak: number
      }
      const kids = new Map<number, Kid>()
      let count = 0
      let made = 0
      let restarts: number[] = []
      let down = false
      const pending = new Set<() => void>()
      // telemetry (telemetry.t27), as the fixed supervisor reports it
      const tel = sys.telemetry
      const sup = tel?.supervisor(
        opts.name,
        opts.maxRestarts,
        () =>
          restarts.filter(
            (t) =>
              c(
                'in_period',
                u32((clock.now() - t) / 1000),
                u32(opts.periodSeconds),
              ) !== 0,
          ).length,
      )

      // `restartCount` > 0: a start the restart decision asked for
      const run = (id: number, k: Kid, restartCount = 0) => {
        k.startedAt = clock.now()
        const go = () => k.child.start((reason) => exited(id, reason), k.slot)
        k.running =
          ev !== undefined && self !== undefined
            ? ev.under(self, restartCount, restartCount > 0, go)
            : go()
        if (k.running.pid !== undefined) k.slot = slotOf(k.running.pid)
      }
      const stopAll = () => {
        for (const cancel of pending) cancel()
        pending.clear()
        for (const k of [...kids.values()].reverse()) k.running?.stop()
      }
      const exited = (id: number, reason: number) => {
        if (tel === undefined) exitedOn(id, reason)
        else tel.als.run(opts.name, exitedOn, id, reason)
      }
      const exitedOn = (id: number, reason: number) => {
        const k = kids.get(id)
        if (down || !k) return
        k.running = undefined
        const now = clock.now()
        restarts = restarts.filter(
          (t) =>
            c('in_period', u32((now - t) / 1000), u32(opts.periodSeconds)) !==
            0,
        )
        k.streak = c(
          'unstable_streak',
          u32(k.streak),
          u32((now - k.startedAt) / 1000),
        )
        const decision = c(
          'with_backoff',
          c(
            'on_child_exit',
            k.child.restart ?? RESTART_PERMANENT,
            reason,
            u32(restarts.length),
            u32(opts.maxRestarts),
          ),
          u32(k.streak),
        )
        if (decision === SUP_GIVE_UP) {
          if (sup) tel?.gaveUp(sup)
          down = true
          stopAll()
          ended(GIVE_UP_REASON)
          onExit(GIVE_UP_REASON)
          return
        }
        const again = decision === SUP_RESTART
        count = c('children_after_exit', u32(count), flag(again))
        if (!again) {
          kids.delete(id)
          return
        }
        restarts.push(now)
        if (sup) tel?.restarted(sup, k.child.kind ?? k.child.name)
        const wait = Number(
          card().call64(
            'jittered_seconds',
            c('backoff_seconds', u32(k.streak)),
            k.slot ?? BigInt(id + 1),
          ),
        )
        const cancel = clock.after(wait * 1000, () => {
          pending.delete(cancel)
          if (!down && kids.get(id) === k)
            run(id, k, Math.max(1, restarts.length))
        })
        pending.add(cancel)
      }

      ready({
        startChild: (maxChildren) => {
          if (down) return START_MAX_CHILDREN
          const answer = c('start_answer', u32(count), 0, u32(maxChildren))
          if (answer !== START_MAX_CHILDREN) {
            const id = made++
            const k: Kid = { child: make(id), startedAt: 0, streak: 0 }
            kids.set(id, k)
            count += 1
            run(id, k)
          }
          return answer
        },
        stopChild: (pid) => {
          for (const [id, k] of kids)
            if (k.running?.pid === pid) {
              // a stop asked for is quiet: no exit comes back, so the place
              // is freed here, as children_after_exit frees it for a child
              // that is not restarted
              kids.delete(id)
              count = c('children_after_exit', u32(count), flag(false))
              k.running.stop()
              return
            }
        },
        live: () => count,
      })
      return {
        pid: self,
        stop: () => {
          down = true
          stopAll()
          ended(X_SHUTDOWN)
        },
      }
    },
  }
}
