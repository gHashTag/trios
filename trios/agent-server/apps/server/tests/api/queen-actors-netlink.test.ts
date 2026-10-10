/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * THE NODE LINK, MADE SAFE (gHashTag/t27 specs/queen/netlink.t27, trios#1712
 * lane 6), on the memory net and a virtual clock. Each defect the code
 * reading found is reproduced here first; the Postgres half is in
 * tests/pglive/queen-actors-netlink-live.test.ts.
 */

import { describe, expect, it } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import {
  actorChild,
  createActorSystem,
  type Down,
  incOf,
  nodeOf,
  type Pid,
  supervisor,
} from '../../src/api/services/queen-actors'
import * as actorsCard from '../../src/api/services/queen-actors-card.gen'
import {
  NODE_TTL_SECONDS,
  STRAT_ONE_FOR_ONE,
  X_NOCONNECTION,
} from '../../src/api/services/queen-actors-card.gen'
import { createMemoryNet } from '../../src/api/services/queen-actors-net'
import * as netlinkCard from '../../src/api/services/queen-netlink-card.gen'
import { SELF_FENCE_SECONDS } from '../../src/api/services/queen-netlink-card.gen'
import { DEFAULT_SPECS_ROOT } from '../../src/inngest/spec-catalog'
import { VirtualClock } from './queen-virtual-clock'

const sha = (file: string) =>
  createHash('sha256')
    .update(readFileSync(join(DEFAULT_SPECS_ROOT, file)))
    .digest('hex')

describe('the vendored netlink card is the one PIN names', () => {
  it('netlink.t27 and netlink.wasm match', () => {
    const pin = readFileSync(join(DEFAULT_SPECS_ROOT, 'PIN'), 'utf8')
    expect(pin).toContain(
      `queen/netlink.t27 sha256 ${sha('queen/netlink.t27')}`,
    )
    expect(pin).toContain(
      `queen/netlink.wasm sha256 ${sha('queen/netlink.wasm')}`,
    )
  })

  it("keeps section 9's numbers: both cards say the same", () => {
    expect([
      netlinkCard.NODE_HEARTBEAT_SECONDS,
      netlinkCard.NODE_TTL_SECONDS,
      netlinkCard.NODE_BITS,
      netlinkCard.LOCAL_SLOT_BITS,
      netlinkCard.GEN_MASK,
    ]).toEqual([
      actorsCard.NODE_HEARTBEAT_SECONDS,
      actorsCard.NODE_TTL_SECONDS,
      actorsCard.NODE_BITS,
      actorsCard.LOCAL_SLOT_BITS,
      actorsCard.GEN_MASK,
    ])
    expect(netlinkCard.NODE_TTL_SECONDS).toBe(20)
    expect(netlinkCard.NODE_HEARTBEAT_SECONDS).toBe(5)
  })
})

describe('PgLink defects, reproduced on the memory net', () => {
  it('a node-down resolves only the spawns that waited on that node', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock, { latencyMs: 5 })
    const sys = [0, 1, 2].map((n) =>
      createActorSystem(clock, { node: n, link: net.link(n) }),
    )
    sys[1].registerKind('w', () => ({ name: 'w', receive: () => {} }))
    net.crash(2)
    // node 2's lease lapses at NODE_TTL_SECONDS; a spawn on node 1 is in
    // flight across that moment
    await clock.runUntil(NODE_TTL_SECONDS * 1000 - 3)
    let got: Pid | null | undefined
    void sys[0].spawnOn(1, 'w', null).then((p) => {
      got = p
    })
    await clock.runUntil(NODE_TTL_SECONDS * 1000 + 100)
    expect(sys[0].up(2)).toBe(false)
    expect(got).not.toBeNull()
    expect(got).toBeDefined()
    expect(nodeOf(got as Pid)).toBe(1)
  })

  it('a restarted node never hands out a pid its last incarnation had', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    const a = createActorSystem(clock, { node: 0, link: net.link(0) })
    const b1 = createActorSystem(clock, { node: 1, link: net.link(1) })
    const old = b1.spawn<unknown>({ name: 'x', receive: () => {} })
    net.crash(1)
    await clock.runUntil(1000)
    // the process comes back on the same node id
    const b2 = createActorSystem(clock, { node: 1, link: net.link(1) })
    const got: unknown[] = []
    const fresh = b2.spawn<unknown>({
      name: 'x',
      receive: (m) => void got.push(m),
    })
    expect(fresh).not.toBe(old)
    a.send(old, 'for the old incarnation')
    await clock.runUntil(2000)
    expect(got).toEqual([])
  })

  it('a node cut off from the store stops its actors before any peer sees it down', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    const sys = [0, 1, 2].map((n) =>
      createActorSystem(clock, { node: n, link: net.link(n) }),
    )
    const target = sys[2].spawn<number>({ name: 't', receive: () => {} })
    await clock.runUntil(100)
    net.crash(2)
    // the last second in which the peers still see node 2 up
    await clock.runUntil(100 + (NODE_TTL_SECONDS - 1) * 1000)
    expect(sys[0].up(2)).toBe(true)
    expect(sys[2].alive(target)).toBe(false)
    await clock.runUntil(100 + (NODE_TTL_SECONDS + 1) * 1000)
    expect(sys[0].up(2)).toBe(false)
    expect(sys[2].alive(target)).toBe(false)
  })
})

describe('the node link, made safe (netlink.t27)', () => {
  it('a pid carries its incarnation, and a lone node keeps section 1 pids', () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    const b1 = createActorSystem(clock, { node: 1, link: net.link(1) })
    const b2 = createActorSystem(clock, { node: 1, link: net.link(1) })
    expect(b1.incarnation).toBe(1)
    expect(b2.incarnation).toBe(2)
    expect(incOf(b1.spawn({ name: 'x', receive: () => {} }))).toBe(1)
    expect(incOf(b2.spawn({ name: 'x', receive: () => {} }))).toBe(2)
    const lone = createActorSystem(clock)
    expect(lone.incarnation).toBe(0)
    // slot 1 at generation 1: section 1's pid, unchanged
    expect(lone.spawn({ name: 'x', receive: () => {} })).toBe(2n ** 32n + 1n)
  })

  it('a restart inside the TTL is its last incarnation down at once', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    const a = createActorSystem(clock, { node: 0, link: net.link(0) })
    const b1 = createActorSystem(clock, { node: 1, link: net.link(1) })
    const old = b1.spawn<number>({ name: 'x', receive: () => {} })
    const downs: Down[] = []
    const watcher = a.spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    a.monitor(watcher, old)
    await clock.runUntil(100)
    // the process comes back before its lease could lapse
    const b2 = createActorSystem(clock, { node: 1, link: net.link(1) })
    const fresh = b2.spawn<number>({ name: 'x', receive: () => {} })
    a.monitor(watcher, fresh)
    await clock.runUntil(200)
    expect(downs).toEqual([{ kind: 'DOWN', pid: old, reason: X_NOCONNECTION }])
    // mail written for the last incarnation is expired, not delivered
    a.send(old, 1)
    await clock.runUntil(300)
    expect(net.expired.count).toBe(1)
    // the monitor of the new incarnation still waits
    await clock.runUntil(100 + (NODE_TTL_SECONDS + 5) * 1000)
    expect(downs.length).toBe(1)
  })

  it('a fenced node starts nothing, sends nothing, and its supervisor restarts nothing', async () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock)
    const sys = [0, 1].map((n) =>
      createActorSystem(clock, { node: n, link: net.link(n) }),
    )
    let starts = 0
    supervisor(
      sys[1],
      {
        name: 'sup',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 10,
        periodSeconds: 60,
      },
      [
        actorChild(sys[1], {
          name: 'c',
          init: () => void starts++,
          receive: () => {},
        }),
      ],
    ).start(() => {})
    const sink = sys[0].spawn<number>({ name: 's', receive: () => {} })
    expect(starts).toBe(1)
    net.crash(1)
    await clock.runUntil((SELF_FENCE_SECONDS - 1) * 1000)
    expect(sys[1].fenced()).toBe(false)
    await clock.runUntil(SELF_FENCE_SECONDS * 1000 + 1)
    expect(sys[1].fenced()).toBe(true)
    expect(sys[1].stats.fencedStops).toBe(1)
    expect(sys[1].spawn({ name: 'late', receive: () => {} })).toBe(0n)
    const dead = sys[1].stats.deadLetters
    sys[1].send(sink, 1)
    expect(sys[1].stats.deadLetters).toBe(dead + 1)
    expect(sys[1].stats.remoteSent).toBe(0)
    await clock.runUntil(120_000)
    expect(starts).toBe(1)
    expect(sys[1].up(1)).toBe(false)
  })
})
