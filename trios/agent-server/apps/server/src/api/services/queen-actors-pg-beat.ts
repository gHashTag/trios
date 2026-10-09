/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE BEAT (specs/queen/netlink.t27 section 3, trios#1712): the entry of the
 * worker thread that renews a node's lease, with a connection of its own.
 *
 * WHY A THREAD. The lease was renewed by a timer on the event loop the actors
 * run on, so a turn that held the loop for a whole lease made a live node look
 * dead, and its peers started its children elsewhere (reproduced: a 25 s turn,
 * one false node-down). Erlang keeps its tick apart from the data for the same
 * reason. Here no turn and no slice can hold the beat up.
 *
 * WHAT IT SHARES. Three slots of one SharedArrayBuffer, read and written with
 * Atomics: the time the actors' loop last turned (the loop writes it), the
 * time the last confirmed renewal was SENT (this thread writes it), and a
 * refused flag (this thread sets it when the store refused a renewal). The
 * card decides from them: beat_renews here, must_fence on the loop.
 */

import { createQueenPool } from '../../lib/db/queen-pool'
import {
  BEAT_REFUSED,
  BEAT_RENEWED,
  BEAT_TICK,
  monoNow,
  RENEW_SQL,
} from './queen-actors-pg'
import { loadCardWasm, u32 } from './queen-card-wasm'
import {
  NODE_HEARTBEAT_SECONDS,
  NODE_TTL_SECONDS,
} from './queen-netlink-card.gen'

const port = globalThis as unknown as {
  onmessage: ((event: MessageEvent) => void) | null
  postMessage: (value: unknown) => void
}

port.onmessage = (event: MessageEvent) => {
  const { url, node, inc, host, shared } = event.data as {
    url: string
    node: number
    inc: number
    host: string
    shared: SharedArrayBuffer
  }
  const slots = new BigInt64Array(shared)
  const card = loadCardWasm('queen/netlink.wasm')
  // one connection; a renewal that has not answered within a heartbeat failed
  const pool = createQueenPool(url, {
    max: 1,
    statement_timeout: NODE_HEARTBEAT_SECONDS * 1000,
  })
  let busy = false
  const beat = async () => {
    if (busy) return
    busy = true
    try {
      const stall = (monoNow() - Number(Atomics.load(slots, BEAT_TICK))) / 1000
      if (card.call('beat_renews', u32(stall)) === 0) {
        port.postMessage({ stalled: stall })
        return
      }
      const sentAt = monoNow()
      const r = await pool.query(RENEW_SQL, [node, inc, host, NODE_TTL_SECONDS])
      if (r.rowCount === 1)
        Atomics.store(slots, BEAT_RENEWED, BigInt(Math.floor(sentAt)))
      else {
        Atomics.store(slots, BEAT_REFUSED, 1n)
        port.postMessage({ refused: true })
      }
    } catch (error) {
      // an unanswered renewal is no renewal; the loop's must_fence counts it
      port.postMessage({
        failed: error instanceof Error ? error.message : String(error),
      })
    } finally {
      busy = false
    }
  }
  setInterval(() => void beat(), NODE_HEARTBEAT_SECONDS * 1000)
}
