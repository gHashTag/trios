/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * One actor node in its own OS process, for the PgLink tests and chaos runs
 * (trios#1712 lane 6). A test kills it, freezes it (SIGSTOP) or makes it spin,
 * which a node sharing the test's own process cannot be made to do.
 *
 * env: QUEEN_NODE_URL (the scratch database), QUEEN_NODE (its node id),
 *      QUEEN_NODE_SINK (optional: a pid on another node to tick at)
 * stdout: one JSON line {ready, spin, echo, ticker} with its pids, then a line
 *         per event the parent may count.
 */

import {
  createActorSystem,
  type Pid,
} from '../../../src/api/services/queen-actors'
import { createPgLink } from '../../../src/api/services/queen-actors-pg'
import { createQueenPool } from '../../../src/lib/db/queen-pool'

const url = process.env.QUEEN_NODE_URL as string
const node = Number(process.env.QUEEN_NODE)
const pool = createQueenPool(url)
const link = await createPgLink(pool, node, { pollMs: 200 })
const sys = createActorSystem(undefined, { node, link })
const say = (o: unknown) =>
  process.stdout.write(
    `${JSON.stringify(o, (_k, v) => (typeof v === 'bigint' ? v.toString() : v))}\n`,
  )

// a turn that holds the loop: a synchronous spin, as a CPU-heavy turn would
const spin = sys.spawn<number>({
  name: 'spin',
  receive: (ms) => {
    const end = Date.now() + ms
    let x = 0
    while (Date.now() < end) x++
    say({ spun: ms, x: x > 0 })
  },
})
// replies to whoever asks: {from, n} -> n back to `from`
const echo = sys.spawn<{ from: string; n: number }>({
  name: 'echo',
  receive: (m, self) => {
    sys.send(BigInt(m.from) as Pid, m.n, self)
  },
})
// an actor that keeps working: every tick it sends its count to the sink
const sinkText = process.env.QUEEN_NODE_SINK
let n = 0
const ticker = sys.spawn<number>({
  name: 'ticker',
  receive: (_m, self) => {
    if (sinkText) sys.send(BigInt(sinkText) as Pid, n++, self)
  },
})
setInterval(() => sys.send(ticker, 0), 250)
// a link that can fence says so (the chaos run also drives the link from
// before netlink.t27, which has no fence)
;(link as { onFenced?: (h: () => void) => void }).onFenced?.(() =>
  say({ fenced: true, at: Date.now() }),
)
say({
  ready: true,
  spin,
  echo,
  ticker,
  inc: (link as { incarnation?: number }).incarnation,
})
