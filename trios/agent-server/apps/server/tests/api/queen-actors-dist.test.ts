/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * PREEMPTION AND NODES (gHashTag/t27 specs/queen/actors.t27 sections 8 and 9,
 * t27#7900), as the wasm card decides them. The numbers each part measures are
 * printed; the asserts check the behaviour, not a machine's speed.
 */

import { describe, expect, it } from 'bun:test'
import {
  createActorSystem,
  type Down,
  holdsAfterKill,
  nodeOf,
  type Pid,
  remoteChild,
  supervisor,
} from '../../src/api/services/queen-actors'
import {
  ISO_LOOP,
  ISO_PROCESS,
  ISO_THREAD,
  NODE_TTL_SECONDS,
  STRAT_ONE_FOR_ONE,
  X_CRASH,
  X_KILLED,
  X_NOCONNECTION,
} from '../../src/api/services/queen-actors-card.gen'
import {
  processWork,
  threadWork,
} from '../../src/api/services/queen-actors-isolate'
import { createMemoryNet } from '../../src/api/services/queen-actors-net'
import { VirtualClock } from './queen-virtual-clock'

const CPU = new URL('./fixtures/queen-cpu-work.ts', import.meta.url).href
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

describe('section 8: a slice between turns', () => {
  // one actor that sends itself the next message: the chain the ring benchmark
  // measured, where every turn is a microtask and nothing else gets to run
  const flood = async (slices: boolean) => {
    const sys = createActorSystem(undefined, { slices })
    const N = 200_000
    let processed = 0
    let firedAt = -1
    let finish: () => void = () => {}
    const done = new Promise<void>((r) => {
      finish = r
    })
    const pid: Pid = sys.spawn<number>({
      name: 'flood',
      receive: (n, self) => {
        processed++
        if (n + 1 >= N) finish()
        else sys.send(pid, n + 1, self)
      },
    })
    setTimeout(() => {
      firedAt = processed
    }, 0)
    sys.send(pid, 0)
    await done
    await sleep(5)
    return { firedAt, yields: sys.stats.yields, N }
  }

  it('a timer runs during a flood of 200 000 turns, not after it', async () => {
    const on = await flood(true)
    const off = await flood(false)
    console.log(
      `\n## slices: a 0 ms timer during 200 000 turns\nwith slices: it ran after ${on.firedAt} turns (${on.yields} yields)\nwithout: after ${off.firedAt} turns`,
    )
    expect(on.firedAt).toBeLessThan(on.N)
    expect(on.yields).toBeGreaterThan(0)
    expect(off.firedAt).toBe(off.N)
  })
})

describe('section 8: a turn the OS preempts and a kill stops', () => {
  it('the card: only a process kill stops the work', () => {
    expect(holdsAfterKill(ISO_LOOP, false)).toBe(true)
    expect(holdsAfterKill(ISO_THREAD, false)).toBe(true)
    expect(holdsAfterKill(ISO_PROCESS, false)).toBe(false)
  })

  const cpuRun = async (isolated: boolean) => {
    const sys = createActorSystem()
    const TURNS = 8
    const MS = 120
    let left = TURNS
    let finish: () => void = () => {}
    const done = new Promise<void>((r) => {
      finish = r
    })
    const workers: Pid[] = []
    for (let i = 0; i < 4; i++)
      workers.push(
        sys.spawn<number>({
          name: `cpu-${i}`,
          isolated: isolated
            ? {
                cpuBound: true,
                foreignCode: false,
                start: (ms) => threadWork(CPU, 'burn', ms),
              }
            : undefined,
          receive: async (ms) => {
            if (!isolated) {
              const { burn } = await import(CPU)
              burn(ms)
            }
            left--
            if (left === 0) finish()
          },
        }),
      )
    // a 10 ms ticker: the longest gap between its runs - counting the start and
    // the end of the run - is how long the loop was held
    const ticks: number[] = []
    const ticker = setInterval(() => ticks.push(performance.now()), 10)
    const t0 = performance.now()
    for (let i = 0; i < TURNS; i++) sys.send(workers[i % 4], MS)
    await done
    const t1 = performance.now()
    const wall = t1 - t0
    clearInterval(ticker)
    const marks = [t0, ...ticks.filter((t) => t > t0 && t < t1), t1]
    let late = 0
    for (let i = 1; i < marks.length; i++)
      late = Math.max(late, marks[i] - marks[i - 1] - 10)
    return { wall: Math.round(wall), late: Math.round(late) }
  }

  it('CPU turns in threads run side by side and leave the loop free', async () => {
    const loop = await cpuRun(false)
    const thread = await cpuRun(true)
    console.log(
      `\n## 8 turns of 120 ms CPU on 4 actors\non the loop: ${loop.wall} ms, the loop held up to ${loop.late} ms\nin threads: ${thread.wall} ms, the loop held up to ${thread.late} ms`,
    )
    expect(thread.late).toBeLessThan(loop.late)
    expect(thread.wall).toBeLessThan(loop.wall)
  }, 60_000)

  it('a thread turn past its bound is killed, but the card knows its work goes on', async () => {
    const sys = createActorSystem()
    const shared = new SharedArrayBuffer(4)
    const counter = new Int32Array(shared)
    let reason = -1
    let stop: () => void = () => {}
    const pid = sys.spawn<SharedArrayBuffer>(
      {
        name: 'spinner',
        turnMaxSeconds: 1,
        isolated: {
          cpuBound: true,
          foreignCode: false,
          start: (buf) => {
            const w = threadWork(CPU, 'spin', buf)
            stop = w.stop
            return w
          },
        },
        receive: () => {},
      },
      (r) => {
        reason = r
      },
    )
    sys.send(pid, shared)
    await sleep(1500)
    expect(sys.alive(pid)).toBe(false)
    expect(reason).toBe(X_KILLED)
    // kill_effect(ISO_THREAD) is KILL_ABANDONS: the runtime does not claim a stop
    expect(sys.stats.stopped).toBe(0)
    expect(holdsAfterKill(ISO_THREAD, false)).toBe(true)
    stop()
    const a = Atomics.load(counter, 0)
    await sleep(200)
    const b = Atomics.load(counter, 0)
    console.log(
      `\n## a terminated worker thread: counter ${a}, then ${b} 200 ms after terminate (it keeps running)`,
    )
    expect(b).toBeGreaterThanOrEqual(a)
  }, 30_000)

  it('a process turn past its bound is killed with SIGKILL', async () => {
    const sys = createActorSystem()
    let failed = ''
    const pid = sys.spawn<number>({
      name: 'sleeper',
      turnMaxSeconds: 1,
      isolated: {
        cpuBound: false,
        foreignCode: true,
        start: (s) => {
          const w = processWork(['sleep', String(s)])
          w.result.catch((e) => {
            failed = String(e)
          })
          return w
        },
      },
      receive: () => {},
    })
    sys.send(pid, 30)
    await sleep(1600)
    expect(sys.alive(pid)).toBe(false)
    expect(sys.stats.stopped).toBe(1)
    expect(failed).toContain('exit')
  }, 30_000)
})

describe('section 9: nodes', () => {
  const three = () => {
    const clock = new VirtualClock()
    const net = createMemoryNet(clock, { latencyMs: 5 })
    const sys = [0, 1, 2].map((n) =>
      createActorSystem(clock, { node: n, link: net.link(n) }),
    )
    return { clock, net, sys }
  }

  it('a pid names its node, and a send crosses to it in order', async () => {
    const { clock, sys } = three()
    const got: number[] = []
    const far = sys[2].spawn<number>({
      name: 'far',
      receive: (n) => void got.push(n),
    })
    expect(nodeOf(far)).toBe(2)
    for (let i = 0; i < 5; i++) sys[0].send(far, i)
    expect(sys[0].stats.remoteSent).toBe(5)
    await clock.runUntil(1000)
    expect(got).toEqual([0, 1, 2, 3, 4])
  })

  it('a monitor across nodes hears the real reason while the node is up', async () => {
    const { clock, sys } = three()
    const downs: Down[] = []
    const watcher = sys[0].spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    const target = sys[1].spawn<number>({
      name: 't',
      receive: () => {
        throw new Error('boom')
      },
    })
    sys[0].monitor(watcher, target)
    await clock.runUntil(100)
    sys[0].send(target, 1)
    await clock.runUntil(1000)
    expect(downs).toEqual([{ kind: 'DOWN', pid: target, reason: X_CRASH }])
  })

  it('a node that stops renewing is down after NODE_TTL_SECONDS, and its monitors hear noconnection', async () => {
    const { clock, net, sys } = three()
    const downs: Down[] = []
    const watcher = sys[0].spawn<Down>({
      name: 'w',
      receive: (m) => void downs.push(m),
    })
    const target = sys[2].spawn<number>({ name: 't', receive: () => {} })
    sys[0].monitor(watcher, target)
    await clock.runUntil(100)
    net.crash(2)
    await clock.runUntil(100 + (NODE_TTL_SECONDS - 1) * 1000)
    expect(downs).toEqual([])
    expect(sys[0].up(2)).toBe(true)
    await clock.runUntil(100 + (NODE_TTL_SECONDS + 1) * 1000)
    expect(sys[0].up(2)).toBe(false)
    expect(downs).toEqual([
      { kind: 'DOWN', pid: target, reason: X_NOCONNECTION },
    ])
    // a send to a node that is down is a dead letter
    const before = sys[0].stats.deadLetters
    sys[0].send(target, 1)
    expect(sys[0].stats.deadLetters).toBe(before + 1)
  })

  it('a supervisor places workers on the roomiest nodes and moves them when a node dies', async () => {
    const { clock, net, sys } = three()
    const workedOn = new Map<number, number>()
    for (const s of sys)
      s.registerKind('worker', () => ({
        name: 'worker',
        receive: () => {},
        init: () => workedOn.set(s.node, (workedOn.get(s.node) ?? 0) + 1),
      }))
    const placedOn = new Map<number, number>()
    const children = Array.from({ length: 6 }, (_, i) =>
      remoteChild(sys[0], {
        name: `w${i}`,
        kind: 'worker',
        arg: null,
        nodes: () => [1, 2],
        room: (n) => 3 - (placedOn.get(n) ?? 0),
        placed: (n) => {
          if (n !== null) placedOn.set(n, (placedOn.get(n) ?? 0) + 1)
        },
      }),
    )
    supervisor(
      sys[0],
      {
        name: 'pool',
        strategy: STRAT_ONE_FOR_ONE,
        maxRestarts: 10,
        periodSeconds: 300,
      },
      children,
    ).start(() => {})
    await clock.runUntil(1000)
    expect(workedOn.get(1)).toBe(3)
    expect(workedOn.get(2)).toBe(3)
    const crashAt = clock.now()
    net.crash(2)
    // three DOWNs at the TTL, three restarts after the card's backoff, all on node 1
    await clock.runUntil(crashAt + (NODE_TTL_SECONDS + 15) * 1000)
    expect(workedOn.get(1)).toBe(6)
    expect(workedOn.get(2)).toBe(3)
    console.log(
      `\n## a node dies under 3 of 6 workers\nall 3 restarted on node 1 within ${NODE_TTL_SECONDS} s (lease TTL) + the card's backoff (<= 10 s)`,
    )
  })
})
