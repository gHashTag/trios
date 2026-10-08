/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Links between nodes (actors.t27 section 9, t27#7900).
 *
 * createMemoryNet joins several actor systems in one process. Tests and the
 * benchmark use it to run several nodes on one clock, to crash a node, and to
 * watch every monitor across it hear X_NOCONNECTION once its lease lapses.
 * A crashed node renews nothing. Others see it up (node_up) until
 * NODE_TTL_SECONDS have passed since its last heartbeat, as a real lease
 * reads. Mail sent to it meanwhile is lost, as a socket to a dead host loses it.
 */

import type { Clock, Mail, NodeLink } from './queen-actors'
import { NODE_TTL_SECONDS } from './queen-actors-card.gen'
import { flag, loadCardWasm, u32 } from './queen-card-wasm'

const nodeUp = (secondsSinceHeartbeat: number): boolean =>
  loadCardWasm('queen/actors.wasm').call(
    'node_up',
    u32(secondsSinceHeartbeat),
  ) !== 0

export function createMemoryNet(
  clock: Clock,
  options: { latencyMs?: number } = {},
) {
  const latency = options.latencyMs ?? 0
  const nodes = new Map<
    number,
    {
      crashedAt: number | null
      mail: Array<(m: Mail) => void>
      down: Array<(n: number) => void>
    }
  >()
  const seen = (n: number) => {
    const at = nodes.get(n)
    if (!at) return false
    if (at.crashedAt === null) return true
    return nodeUp((clock.now() - at.crashedAt) / 1000)
  }
  const link = (node: number): NodeLink => {
    nodes.set(node, { crashedAt: null, mail: [], down: [] })
    return {
      node,
      up: (n) => seen(n),
      carry: (to, mail) => {
        const copy = structuredClone(mail)
        clock.after(latency, () => {
          const target = nodes.get(to)
          if (!target || target.crashedAt !== null) return
          if (nodes.get(node)?.crashedAt !== null) return
          for (const h of target.mail) h(copy)
        })
      },
      onMail: (h) => nodes.get(node)?.mail.push(h),
      onNodeDown: (h) => nodes.get(node)?.down.push(h),
    }
  }
  /** The node stops: no heartbeat, no mail in or out. */
  const crash = (node: number) => {
    const at = nodes.get(node)
    if (!at || at.crashedAt !== null) return
    at.crashedAt = clock.now()
    clock.after(NODE_TTL_SECONDS * 1000, () => {
      for (const [n, other] of nodes)
        if (n !== node && other.crashedAt === null)
          for (const h of other.down) h(node)
    })
  }
  return { link, crash, up: (n: number) => flag(seen(n)) === 1 }
}
