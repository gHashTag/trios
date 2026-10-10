/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * KEYED ACTORS (gHashTag/t27 specs/queen/keyed.t27, epic gHashTag/trios#1712
 * item 7). An actor is addressed by a key, an issue number for the bee
 * dispatcher, and not by a pid. Orleans grains, Akka Cluster Sharding
 * entities, AutoGen's AgentId(type, key) and Rivet's get-or-create work the
 * same way. The keyed card decides:
 *   - what a send to a key does: deliver it to the live actor, start one,
 *     hold it while the old one stops, evict the longest idle actor at the
 *     cap, or refuse (key_action, evict_before);
 *   - when an idle actor is passivated (passivate_due, evictable);
 *   - which exit clears the directory: only the incarnation it names
 *     (entry_after_exit);
 *   - whether sends held for a stopping key start it again (hold_admits,
 *     restart_after_exit).
 * This file holds the directory and the timers.
 *
 * THE DIRECTORY IS ONE NODE'S. It is not a global name registry (actors.t27
 * GLOBAL_NAME_REGISTRY = false). Two nodes may both start an actor for one
 * key; the store's claim decides which one works (keyed.t27 section 5).
 */

import {
  type ActorSpec,
  type ActorSystem,
  type Pid,
  slotOf,
} from './queen-actors'
import {
  D_DROPPED_FULL,
  D_QUEUED,
  WORKER_SHUTDOWN_MS,
} from './queen-actors-card.gen'
import { linksOf } from './queen-actors-links'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import {
  KA_DELIVER,
  KA_EVICT,
  KA_HOLD,
  KA_SPAWN,
  KEY_NO_PID,
  KEYED_ACTIVE_CAP,
  PASSIVATE_IDLE_SECONDS,
  PASSIVATE_REASON,
} from './queen-keyed-card.gen'

export const KEYED_CARD = 'queen/keyed.wasm'

const card = () => loadCardWasm(KEYED_CARD)
const kc = (name: string, ...a: number[]) => card().call(name, ...a)
const kc64 = (name: string, ...a: Array<number | bigint>) =>
  BigInt.asUintN(64, BigInt(card().call64(name, ...a)))

/** A keyed actor: an actor spec, and whether it watches work right now. */
export interface KeyedActor<M> extends ActorSpec<M> {
  /** A bee it started, a retry it waits for: it is not passivated meanwhile. */
  holdsWork?: () => boolean
}

export interface KeyedOptions<K, M> {
  name: string
  /** The actor for a key, called once for each incarnation. */
  make: (key: K) => KeyedActor<M>
  /** The node's cap on live keyed actors; KEYED_ACTIVE_CAP by default. */
  cap?: number
  /** For tests; PASSIVATE_IDLE_SECONDS is the card's (passivate_due). */
  idleCheckSeconds?: number
  /** How long a trapping actor gets to finish when passivated. */
  shutdownMs?: number
}

interface Entry<M> {
  pid: Pid
  spec: KeyedActor<M>
  stopping: boolean
  held: M[]
  lastActive: number
  cancelCheck?: () => void
}

export interface KeyedStats {
  /** Actors started for a key that had none. */
  activations: number
  /** Of those, keys that had been passivated before: a send brought them back. */
  reactivations: number
  passivations: number
  evictions: number
  /** Sends held while their key stopped. */
  held: number
  /** Sends refused because the node was full of busy actors, or a hold overflowed. */
  refused: number
}

/**
 * A directory of keyed actors on one actor system. `send(key, msg)` reaches
 * the key's actor, starting it if there is none.
 */
export function keyedActors<K extends string | number, M>(
  sys: ActorSystem,
  opts: KeyedOptions<K, M>,
) {
  const clock = sys.clock
  const links = linksOf(sys)
  const cap = opts.cap ?? KEYED_ACTIVE_CAP
  const shutdownMs = opts.shutdownMs ?? WORKER_SHUTDOWN_MS
  const checkMs = (opts.idleCheckSeconds ?? PASSIVATE_IDLE_SECONDS) * 1000
  const entries = new Map<K, Entry<M>>()
  const keyOf = new Map<Pid, K>()
  const passivated = new Set<K>()
  const stats: KeyedStats = {
    activations: 0,
    reactivations: 0,
    passivations: 0,
    evictions: 0,
    held: 0,
    refused: 0,
  }

  const idleSeconds = (e: Entry<M>) =>
    Math.floor((clock.now() - e.lastActive) / 1000)
  const queued = (e: Entry<M>) => sys.mailbox(e.pid)?.length ?? 0
  const holdsWork = (e: Entry<M>) => e.spec.holdsWork?.() ?? false
  const evictable = (e: Entry<M>) =>
    !e.stopping &&
    kc(
      'evictable',
      flag(sys.busy(e.pid)),
      u32(queued(e)),
      flag(holdsWork(e)),
    ) !== 0

  /** Look again once the actor may have been idle long enough. */
  const arm = (key: K, e: Entry<M>) => {
    e.cancelCheck?.()
    e.cancelCheck = clock.after(checkMs, () => {
      if (entries.get(key) !== e || e.stopping) return
      const due =
        kc(
          'passivate_due',
          u32(idleSeconds(e)),
          flag(sys.busy(e.pid)),
          u32(queued(e)),
          flag(holdsWork(e)),
        ) !== 0
      if (due) passivate(key, e)
      else arm(key, e)
    })
  }

  /**
   * Passivate: a normal end (PASSIVATE_REASON). A trapping actor is asked
   * first and may finish (shutdown, then kill after shutdownMs); meanwhile
   * sends to its key are held. Any other actor is idle with nothing queued,
   * so it ends at once and loses nothing.
   */
  function passivate(key: K, e: Entry<M>, evicted = false): void {
    if (evicted) stats.evictions++
    else stats.passivations++
    passivated.add(key)
    e.cancelCheck?.()
    if (links.trapping(e.pid)) {
      e.stopping = true
      void links.stop(e.pid, shutdownMs)
    } else {
      sys.exit(e.pid, PASSIVATE_REASON, true)
    }
  }

  function start(key: K, first: M[]): void {
    const spec = opts.make(key)
    const e: Entry<M> = {
      pid: 0n,
      spec,
      stopping: false,
      held: [],
      lastActive: clock.now(),
    }
    const touched = () => {
      e.lastActive = clock.now()
    }
    const wrapped: ActorSpec<M> = {
      ...spec,
      init: (self) => {
        e.pid = self
        keyOf.set(self, key)
        entries.set(key, e)
        spec.init?.(self)
      },
      receive: async (msg, self, result, signal) => {
        touched()
        try {
          await spec.receive(msg, self, result, signal)
        } finally {
          touched()
          if (entries.get(key) === e && !e.stopping) arm(key, e)
        }
      },
      control: spec.control
        ? (tag, self) => {
            touched()
            spec.control?.(tag, self)
          }
        : undefined,
    }
    stats.activations++
    if (passivated.delete(key)) stats.reactivations++
    sys.spawn(wrapped)
    arm(key, e)
    for (const m of first) sys.send(e.pid, m)
  }

  // Every exit, a quiet one included: the directory forgets the incarnation
  // that ended, never a newer one, and held sends start the key again.
  sys.onExit((pid, _reason) => {
    const key = keyOf.get(pid)
    if (key === undefined) return
    keyOf.delete(pid)
    const e = entries.get(key)
    if (!e) return
    if (kc64('entry_after_exit', e.pid, pid) !== BigInt(KEY_NO_PID)) return
    entries.delete(key)
    e.cancelCheck?.()
    if (kc('restart_after_exit', u32(e.held.length)) !== 0) start(key, e.held)
  })

  /** Send to a key. Returns the delivery code (actors.t27 D_*). */
  // biome-ignore lint/complexity/noExcessiveCognitiveComplexity: one branch per keyed.t27 key_action answer; each decision is the card's, the branches only carry it out
  function send(key: K, msg: M): number {
    const e = entries.get(key)
    const stopping = !!e?.stopping
    const alive = !!e && !stopping && sys.alive(e.pid)
    const active = entries.size
    let idle = 0
    if (active >= cap)
      for (const x of entries.values()) if (evictable(x)) idle++
    const action = kc(
      'key_action',
      flag(alive),
      flag(stopping),
      u32(active),
      u32(cap),
      u32(idle),
    )
    if (action === KA_DELIVER && e) {
      e.lastActive = clock.now()
      return sys.send(e.pid, msg)
    }
    if (action === KA_HOLD && e) {
      if (kc('hold_admits', u32(e.held.length)) !== 0) {
        e.held.push(msg)
        stats.held++
        return D_QUEUED
      }
    } else if (action === KA_SPAWN) {
      start(key, [msg])
      return D_QUEUED
    } else if (action === KA_EVICT) {
      let victim: [K, Entry<M>] | undefined
      for (const [k, x] of entries)
        if (
          evictable(x) &&
          (!victim ||
            card().call64(
              'evict_before',
              u32(idleSeconds(x)),
              slotOf(x.pid),
              u32(idleSeconds(victim[1])),
              slotOf(victim[1].pid),
            ) !== 0)
        )
          victim = [k, x]
      if (victim) {
        // an evictable actor is idle with nothing queued: it ends at once
        stats.evictions++
        passivated.add(victim[0])
        victim[1].cancelCheck?.()
        sys.exit(victim[1].pid, PASSIVATE_REASON, true)
        start(key, [msg])
        return D_QUEUED
      }
    }
    stats.refused++
    sys.stats.deadLetters++
    return D_DROPPED_FULL
  }

  /** Stop every keyed actor, the last started first, each with shutdownMs. */
  function stopAll(): Promise<number[]> {
    return links.stopInOrder(
      [...entries.values()].map((e) => {
        e.stopping = true
        e.cancelCheck?.()
        return { pid: e.pid, shutdownMs }
      }),
    )
  }

  return {
    send,
    pidOf: (key: K): Pid | undefined => entries.get(key)?.pid,
    /** Live keyed actors, the stopping ones included (key_action's `active`). */
    active: () => entries.size,
    keys: () => [...entries.keys()],
    passivate: (key: K) => {
      const e = entries.get(key)
      if (e && !e.stopping) passivate(key, e)
    },
    stopAll,
    stats,
  }
}

export type Keyed<K extends string | number, M> = ReturnType<
  typeof keyedActors<K, M>
>
