/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Links between nodes (actors.t27 section 9, t27#7900; netlink.t27).
 *
 * createMemoryNet joins several actor systems in one process. Tests and the
 * benchmark use it to run several nodes on one clock, to crash a node, and to
 * watch every monitor across it hear X_NOCONNECTION once its lease lapses.
 * A crashed node renews nothing. Others see it up (node_up) until
 * NODE_TTL_SECONDS have passed since its last heartbeat, as a real lease
 * reads. Mail sent to it meanwhile is lost, as a socket to a dead host loses it.
 *
 * The net stands in for the store, so it plays the store's part of
 * netlink.t27 too: each `link(node)` is a process start and takes the next
 * incarnation (next_incarnation); mail written for any other incarnation of
 * its node is expired (mail_action); a crashed node is cut off from the store
 * and fences itself (must_fence) before its peers see it down; and a start on
 * a node id still held is a restart, which its peers see at once as the old
 * incarnation going down.
 */

import type { Clock, Mail, NodeLink } from './queen-actors'
import { NODE_TTL_SECONDS } from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'
import { MA_DELIVER, SELF_FENCE_SECONDS } from './queen-netlink-card.gen'

const nodeUp = (secondsSinceHeartbeat: number): boolean =>
  loadCardWasm('queen/actors.wasm').call(
    'node_up',
    u32(secondsSinceHeartbeat),
  ) !== 0
const netlink = () => loadCardWasm('queen/netlink.wasm')

interface Incarnation {
  node: number
  inc: number
  crashedAt: number | null
  /** The peers were told this incarnation is down. */
  downTold: boolean
  mail: Array<(m: Mail) => void>
  down: Array<(n: number, inc: number) => void>
  fence: Array<() => void>
}

export function createMemoryNet(
  clock: Clock,
  options: { latencyMs?: number } = {},
) {
  const latency = options.latencyMs ?? 0
  // the current incarnation of each node id
  const nodes = new Map<number, Incarnation>()
  const lastInc = new Map<number, number>()
  const expired = { count: 0 }

  const seen = (n: number) => {
    const at = nodes.get(n)
    if (!at) return false
    if (at.crashedAt === null) return true
    return nodeUp((clock.now() - at.crashedAt) / 1000)
  }
  const tellDown = (gone: Incarnation) => {
    if (gone.downTold) return
    gone.downTold = true
    for (const other of nodes.values())
      if (other !== gone && other.crashedAt === null)
        for (const h of other.down) h(gone.node, gone.inc)
  }
  const fencedNow = (me: Incarnation) =>
    me.crashedAt !== null &&
    netlink().call(
      'must_fence',
      0,
      u32((clock.now() - me.crashedAt) / 1000),
    ) !== 0

  const link = (node: number): NodeLink => {
    const inc = Number(
      netlink().call64('next_incarnation', BigInt(lastInc.get(node) ?? 0)),
    )
    lastInc.set(node, inc)
    const before = nodes.get(node)
    const me: Incarnation = {
      node,
      inc,
      crashedAt: null,
      downTold: false,
      mail: [],
      down: [],
      fence: [],
    }
    nodes.set(node, me)
    // a restart inside the TTL: the last incarnation is down at once
    if (before) tellDown(before)
    return {
      node,
      incarnation: inc,
      up: (n) => seen(n),
      incarnationOf: (n) => nodes.get(n)?.inc ?? 0,
      carry: (to, mail, toInc) => {
        const copy = structuredClone(mail)
        clock.after(latency, () => {
          const target = nodes.get(to)
          if (!target || target.crashedAt !== null) return
          if (me.crashedAt !== null) return
          if (
            netlink().call64(
              'mail_action',
              BigInt(toInc),
              BigInt(target.inc),
              0,
              1,
            ) !== MA_DELIVER
          ) {
            expired.count++
            return
          }
          for (const h of target.mail) h(copy)
        })
      },
      onMail: (h) => me.mail.push(h),
      onNodeDown: (h) => me.down.push(h),
      fenced: () => fencedNow(me),
      onFenced: (h) => me.fence.push(h),
    }
  }
  /** The node is cut off from the store: no heartbeat, no mail in or out. */
  const crash = (node: number) => {
    const at = nodes.get(node)
    if (!at || at.crashedAt !== null) return
    at.crashedAt = clock.now()
    // it fences itself before any peer can see it down
    clock.after(SELF_FENCE_SECONDS * 1000, () => {
      if (fencedNow(at)) for (const h of at.fence) h()
    })
    clock.after(NODE_TTL_SECONDS * 1000, () => tellDown(at))
  }
  return {
    link,
    crash,
    up: (n: number) => flag(seen(n)) === 1,
    /** Mail written for an incarnation that was no longer its node's. */
    expired,
  }
}
