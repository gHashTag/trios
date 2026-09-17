/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * WHY THIS EXISTS.
 *
 * 2026-09-17: the container is limited to 24 GB and held 14.7 GB with sixteen
 * bees running - about 0.85 GB a bee. The same day the worker ceiling stopped
 * being a constant and the credential list reached thirty-four lanes. A full
 * swarm would have asked for about thirty GB, and the kernel answers that by
 * killing the container with every running bee in it. Nothing in dispatch
 * looked at memory at all.
 *
 * These pin the judge with the container in states a test machine never is in:
 * every reading is injected, so none of this needs Linux, a cgroup or a full
 * disk to be true.
 */
import { afterEach, describe, expect, it } from 'bun:test'
import {
  describeReading,
  diskLineUsedPercent,
  judgeBeeRoom,
  type MemoryReading,
  noteBeeEnded,
  noteBeeStarted,
  readContainerMemory,
  resetYoungBees,
  resourceSettings,
  volumeSpace,
  youngBeeCount,
} from '../../src/api/services/queen-resources'

// DECIMAL, like every number the module prints and the platform sells: the
// Railway container is 24000000000 bytes and everyone calls it 24 GB.
const GB = 1_000_000_000
const files = (map: Record<string, string>) => (path: string) =>
  path in map ? map[path] : null
const measured = (usedGb: number, limitGb = 24): MemoryReading => ({
  kind: 'measured',
  usedBytes: usedGb * GB,
  limitBytes: limitGb * GB,
  source: 'cgroup v2',
  limitSource: 'cgroup',
})
const roomy = { totalBytes: 50 * GB, freeBytes: 19 * GB }
const defaults = resourceSettings({})

afterEach(() => resetYoungBees())

describe('reading the container', () => {
  it('counts what the kernel cannot give back, not the cache it will', () => {
    const reading = readContainerMemory({
      platform: 'linux',
      env: {},
      readFile: files({
        '/sys/fs/cgroup/memory.current': '16000000000\n',
        '/sys/fs/cgroup/memory.max': '25769803776\n',
        '/sys/fs/cgroup/memory.stat':
          'anon 9\ninactive_file 2000000000\nactive_file 5\n',
      }),
    })
    expect(reading).toEqual({
      kind: 'measured',
      usedBytes: 16_000_000_000 - 2_000_000_000 - 5,
      limitBytes: 25_769_803_776,
      source: 'cgroup v2',
      limitSource: 'cgroup',
    })
  })

  it('does not refuse an idle container for a day of page cache', () => {
    // No bee running: 2.5 GB of heap, and 16 GB of ACTIVE cache plus 2.5 GB of
    // reclaimable slab left by a day of clones and installs. Counted as the
    // kubelet counts (current - inactive_file) that is 21 GB "in use", past the
    // line, and a memory refusal frees nothing - so every round refused until a
    // redeploy.
    const reading = readContainerMemory({
      platform: 'linux',
      env: {},
      readFile: files({
        '/sys/fs/cgroup/memory.current': '23000000000',
        '/sys/fs/cgroup/memory.max': '24000000000',
        '/sys/fs/cgroup/memory.stat':
          'anon 2500000000\nfile 18000000000\ninactive_file 2000000000\n' +
          'active_file 16000000000\nslab_reclaimable 2500000000\nslab_unreclaimable 100\n',
      }),
    })
    expect(reading).toMatchObject({ usedBytes: 2_500_000_000 })
    expect(
      judgeBeeRoom({
        memory: reading,
        volume: { totalBytes: 50 * GB, freeBytes: 19 * GB },
        youngBees: 0,
        settings: resourceSettings({}),
        volumeDir: '/workspace',
      }),
    ).toEqual({ ok: true })
  })

  it('takes the host as the ceiling when the cgroup says max, and the variable when stated', () => {
    const readFile = files({
      '/sys/fs/cgroup/memory.current': '3000000000',
      '/sys/fs/cgroup/memory.max': 'max',
      '/proc/meminfo':
        'MemTotal:       67108864 kB\nMemAvailable:   60000000 kB\n',
    })
    const host = readContainerMemory({ platform: 'linux', env: {}, readFile })
    expect(host).toMatchObject({
      limitBytes: 67_108_864 * 1024,
      limitSource: 'host MemTotal',
    })
    // The variable replaces the LIMIT only. Use is still measured.
    const stated = readContainerMemory({
      platform: 'linux',
      env: { TRIOS_QUEEN_MEMORY_LIMIT_MB: '24576' },
      readFile,
    })
    expect(stated).toMatchObject({
      usedBytes: 3_000_000_000,
      limitBytes: 24_576 * 1024 * 1024,
      source: 'cgroup v2',
      limitSource: 'TRIOS_QUEEN_MEMORY_LIMIT_MB',
    })
  })

  it('reads cgroup v1, and treats its near-2^63 limit as no limit', () => {
    const reading = readContainerMemory({
      platform: 'linux',
      env: {},
      readFile: files({
        '/sys/fs/cgroup/memory/memory.usage_in_bytes': '5000000000',
        '/sys/fs/cgroup/memory/memory.limit_in_bytes': '9223372036854771712',
        '/sys/fs/cgroup/memory/memory.stat': 'total_inactive_file 1000000000\n',
        '/proc/meminfo': 'MemTotal: 33554432 kB\nMemAvailable: 31457280 kB\n',
      }),
    })
    expect(reading).toMatchObject({
      usedBytes: 4_000_000_000,
      limitBytes: 33_554_432 * 1024,
      source: 'cgroup v1',
      limitSource: 'host MemTotal',
    })
  })

  it('judges the MACHINE when the cgroup says max, because everything else on it uses it too', () => {
    // An unlimited container on a shared 16 GB machine: this cgroup holds one
    // GB, the rest of the machine holds fifteen. Its own use against the
    // host's total admitted thirteen more bees into half a GB.
    const reading = readContainerMemory({
      platform: 'linux',
      env: {},
      readFile: files({
        '/sys/fs/cgroup/memory.current': '1000000000',
        '/sys/fs/cgroup/memory.max': 'max',
        '/proc/meminfo': 'MemTotal: 16000000 kB\nMemAvailable: 500000 kB\n',
      }),
    })
    expect(reading).toMatchObject({
      usedBytes: 15_500_000 * 1024,
      limitBytes: 16_000_000 * 1024,
      source: '/proc/meminfo',
      limitSource: 'host MemTotal',
    })
  })

  it('lets a stated limit lower the line and never raise it past what the kernel enforces', () => {
    const railway = files({
      '/sys/fs/cgroup/memory.current': '23500000000',
      '/sys/fs/cgroup/memory.max': '24000000000',
      '/proc/meminfo': 'MemTotal: 338368112 kB\nMemAvailable: 109626516 kB\n',
    })
    // 24576 MiB is 25.8 GB: the natural way to write "24 GB", and above the
    // kernel's 24000000000. Taken at its word it admitted a bee at 23.5 GB.
    const above = readContainerMemory({
      platform: 'linux',
      env: { TRIOS_QUEEN_MEMORY_LIMIT_MB: '24576' },
      readFile: railway,
    })
    expect(above).toMatchObject({
      limitBytes: 24_000_000_000,
      limitSource: 'cgroup',
    })
    if (above.kind !== 'measured') throw new Error('expected a reading')
    expect(above.note).toContain(
      'states 25.8 GB, above the 24.0 GB the kernel enforces',
    )
    expect(
      judgeBeeRoom({
        memory: above,
        volume: { totalBytes: 50 * GB, freeBytes: 19 * GB },
        youngBees: 0,
        settings: resourceSettings({}),
        volumeDir: '/workspace',
      }).ok,
    ).toBe(false)
    // Below it, the variable is the line and says so.
    const below = readContainerMemory({
      platform: 'linux',
      env: { TRIOS_QUEEN_MEMORY_LIMIT_MB: '16384' },
      readFile: railway,
    })
    expect(below).toMatchObject({
      limitBytes: 16_384 * 1024 * 1024,
      limitSource: 'TRIOS_QUEEN_MEMORY_LIMIT_MB',
    })
    if (below.kind === 'measured') expect(below.note).toBeUndefined()
  })

  it('falls back to /proc/meminfo, floors use at zero, and says unknown when nothing reads', () => {
    expect(
      readContainerMemory({
        platform: 'linux',
        env: {},
        readFile: files({
          '/proc/meminfo': 'MemTotal: 1000 kB\nMemAvailable: 400 kB\n',
        }),
      }),
    ).toMatchObject({ usedBytes: 600 * 1024, source: '/proc/meminfo' })
    expect(
      readContainerMemory({
        platform: 'linux',
        env: { TRIOS_QUEEN_MEMORY_LIMIT_MB: '1024' },
        readFile: files({
          '/sys/fs/cgroup/memory.current': '100',
          '/sys/fs/cgroup/memory.stat': 'inactive_file 500\n',
        }),
      }),
    ).toMatchObject({ usedBytes: 0 })
    expect(
      readContainerMemory({ platform: 'linux', env: {}, readFile: () => null }),
    ).toEqual({
      kind: 'unknown',
      tried: ['cgroup v2', 'cgroup v1', '/proc/meminfo'],
    })
  })

  it('will not hold the HOST against a stated container limit', () => {
    // Measured inside a Railway container on 2026-09-17: memory.max reads
    // 24000000000 while /proc/meminfo reads the machine - MemTotal 338 GB, 228
    // of them in use. Judged against a stated 24 GB that is "full" forever.
    const railwayHost =
      'MemTotal:       338368112 kB\nMemAvailable:   109626516 kB\n'
    const reading = readContainerMemory({
      platform: 'linux',
      env: { TRIOS_QUEEN_MEMORY_LIMIT_MB: '24576' },
      readFile: files({ '/proc/meminfo': railwayHost }),
    })
    expect(reading.kind).toBe('unknown')
    const refusal = judgeBeeRoom({
      memory: reading,
      volume: { totalBytes: 50 * GB, freeBytes: 19 * GB },
      youngBees: 0,
      settings: resourceSettings({}),
      volumeDir: '/workspace',
    })
    expect(refusal.ok).toBe(false)
    if (!refusal.ok) {
      expect(refusal.detail).toContain('it measures the whole host')
      expect(refusal.detail).toContain('TRIOS_QUEEN_RESOURCE_GUARD=off')
    }
    // With nothing stated the same files ARE the machine, and that is judged.
    expect(
      readContainerMemory({
        platform: 'linux',
        env: {},
        readFile: files({ '/proc/meminfo': railwayHost }),
      }),
    ).toMatchObject({ kind: 'measured', limitSource: 'host MemTotal' })
    // And what Railway really shows is read as the container it is.
    const container = readContainerMemory({
      platform: 'linux',
      env: {},
      readFile: files({
        '/sys/fs/cgroup/memory.current': '26374144',
        '/sys/fs/cgroup/memory.max': '24000000000',
        '/sys/fs/cgroup/memory.stat': 'inactive_file 2363392\n',
        '/proc/meminfo': railwayHost,
      }),
    })
    expect(container).toMatchObject({
      usedBytes: 26_374_144 - 2_363_392,
      limitBytes: 24_000_000_000,
      limitSource: 'cgroup',
    })
    expect(describeReading(container)).toBe(
      'memory from cgroup v2, limit 24.0 GB from cgroup',
    )
  })

  it('has nothing to read on a machine that is not Linux, and says which', () => {
    expect(readContainerMemory({ platform: 'darwin' })).toEqual({
      kind: 'unsupported',
      platform: 'darwin',
    })
  })

  it('parses df -Pk, and returns null - not zero - when it cannot', () => {
    const line =
      'Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/x 52428800 32505856 19922944 62% /workspace\n'
    expect(volumeSpace('/workspace', () => line)).toEqual({
      totalBytes: 52_428_800 * 1024,
      freeBytes: 19_922_944 * 1024,
    })
    expect(volumeSpace('/workspace', () => 'garbage')).toBeNull()
    expect(
      volumeSpace('/workspace', () => {
        throw new Error('df is not there')
      }),
    ).toBeNull()
    expect(volumeSpace(process.cwd())?.freeBytes).toBeGreaterThan(0)
    expect(volumeSpace('/definitely/not/a/path/on/this/machine')).toBeNull()
  })
})

describe('the variables', () => {
  it('have defaults that protect a deployment which sets none of them', () => {
    expect(defaults).toMatchObject({
      guardOn: true,
      beeMemoryBytes: 1024 ** 3,
      memoryHeadroomPercent: 10,
      warmupSeconds: 600,
      beeDiskBytes: 2 * 1024 ** 3,
      diskHeadroomPercent: 10,
      memoryLimitBytes: null,
      notes: [],
    })
  })

  it('replace an unusable value by the default and SAY SO', () => {
    for (const bad of ['abc', '0', '-5', '999999999', '1.5']) {
      const settings = resourceSettings({ TRIOS_QUEEN_BEE_MEMORY_MB: bad })
      expect(settings.beeMemoryBytes).toBe(1024 ** 3)
      expect(settings.notes.join(' ')).toContain('TRIOS_QUEEN_BEE_MEMORY_MB')
    }
    expect(
      resourceSettings({ TRIOS_QUEEN_RESOURCE_GUARD: 'maybe' }),
    ).toMatchObject({ guardOn: true })
    expect(
      resourceSettings({ TRIOS_QUEEN_RESOURCE_GUARD: 'maybe' }).notes,
    ).toHaveLength(1)
    expect(
      resourceSettings({ TRIOS_QUEEN_RESOURCE_GUARD: 'OFF' }).guardOn,
    ).toBe(false)
  })

  it('never tell an operator the container limit is 0', () => {
    for (const bad of ['24GB', '256', '-1', '1.5']) {
      const settings = resourceSettings({ TRIOS_QUEEN_MEMORY_LIMIT_MB: bad })
      expect(settings.memoryLimitBytes).toBeNull()
      expect(settings.notes).toHaveLength(1)
      expect(settings.notes[0]).toContain(
        'using the limit the container reports',
      )
      expect(settings.notes[0]).not.toContain('using 0')
    }
    // Unset, empty and 0 are three spellings of "not stated": no note at all.
    for (const unset of [undefined, '', ' ', '0']) {
      const settings = resourceSettings({ TRIOS_QUEEN_MEMORY_LIMIT_MB: unset })
      expect(settings.memoryLimitBytes).toBeNull()
      expect(settings.notes).toHaveLength(0)
    }
    expect(
      resourceSettings({ TRIOS_QUEEN_MEMORY_LIMIT_MB: '24576' })
        .memoryLimitBytes,
    ).toBe(24_576 * 1024 * 1024)
  })

  it('are read on every call, so moving the line needs no restart', () => {
    const env: NodeJS.ProcessEnv = {}
    expect(resourceSettings(env).beeMemoryBytes).toBe(1024 ** 3)
    env.TRIOS_QUEEN_BEE_MEMORY_MB = '2048'
    expect(resourceSettings(env).beeMemoryBytes).toBe(2 * 1024 ** 3)
  })
})

describe('does one more bee fit', () => {
  const judge = (memory: MemoryReading, youngBees = 0, volume = roomy) =>
    judgeBeeRoom({
      memory,
      volume,
      youngBees,
      settings: defaults,
      volumeDir: '/workspace/BrowserOS',
    })

  it('admits a bee at the measured sixteen-bee load and refuses near the limit', () => {
    expect(judge(measured(14.7))).toEqual({ ok: true })
    const refused = judge(measured(20.9))
    expect(refused.ok).toBe(false)
    if (refused.ok) return
    expect(refused.resource).toBe('memory')
    expect(refused.detail).toMatch(
      /^container memory is 20\.9 GB of 24\.0 GB in use \(cgroup v2\);/,
    )
    expect(refused.detail).toContain('needs about 1.1 GB and 2.4 GB stays free')
    expect(refused.detail.endsWith('.')).toBe(true)
    // The report quotes `summary`: it is the first sentence of the detail, it
    // carries every number, it names no variable and it fits a report line.
    expect(refused.detail.startsWith(`${refused.summary}. `)).toBe(true)
    expect(refused.summary).toContain('20.9 GB of 24.0 GB')
    expect(refused.summary).toContain('1.1 GB and 2.4 GB stays free')
    expect(refused.summary).not.toContain('TRIOS_')
    expect(refused.summary.length).toBeLessThanOrEqual(200)
  })

  it('reserves for bees that started and have not grown, and says how many', () => {
    const refused = judge(measured(10), 12)
    expect(refused.ok).toBe(false)
    if (refused.ok) return
    expect(refused.detail).toContain(
      '12 bee(s) started in the last 10 min are still growing',
    )
    expect(
      judge(measured(20.9)).ok === false && !judge(measured(20.9)).ok,
    ).toBe(true)
    const calm = judge(measured(20.9))
    if (!calm.ok) expect(calm.detail).not.toContain('still growing')
  })

  it('stops a round of back-to-back starts that all read the same empty container', () => {
    // The reading does not move: a bee takes minutes to allocate what it will
    // hold. Thirty-four starts would each be admitted on the reading alone.
    let started = 0
    const nowMs = 1_000_000
    while (started < 34) {
      const verdict = judgeBeeRoom({
        memory: measured(1),
        volume: roomy,
        youngBees: youngBeeCount(nowMs, defaults.warmupSeconds),
        settings: defaults,
        volumeDir: '/workspace',
      })
      if (!verdict.ok) break
      noteBeeStarted(`turn-${started}`, nowMs)
      started += 1
    }
    // (24 - 2.4 - 1) GB over 1024 MiB a bee.
    expect(started).toBe(19)
  })

  it('forgets a start once the bee has had time to grow', () => {
    noteBeeStarted('a', 0)
    noteBeeStarted('b', 1_000)
    noteBeeStarted('c', 700_000)
    expect(youngBeeCount(701_000, 600)).toBe(1)
    expect(youngBeeCount(701_000, 0)).toBe(0)
    // A count never destroys what a later count needs: the window is read on
    // every call and may have grown since.
    expect(youngBeeCount(701_000, 3600)).toBe(3)
  })

  it('frees the reservation when the bee ends, however young it was', () => {
    // `turn.ok` only means /chat answered 200; a provider refusal arrives as a
    // 200 whose stream then fails. Ten such dead starts in ten minutes used to
    // reserve ten GB for bees that no longer existed.
    for (let i = 0; i < 18; i++) noteBeeStarted(`dead-${i}`, 1_000)
    expect(judge(measured(1.9), youngBeeCount(2_000, 600)).ok).toBe(false)
    for (let i = 0; i < 18; i++) noteBeeEnded(`dead-${i}`)
    expect(youngBeeCount(2_000, 600)).toBe(0)
    expect(judge(measured(1.9), youngBeeCount(2_000, 600))).toEqual({
      ok: true,
    })
    // Ending a turn nobody noted is not an error.
    noteBeeEnded('never-started')
  })

  it('names the window as it was set, not rounded to minutes', () => {
    const at = (warmupSeconds: number) => {
      const refused = judgeBeeRoom({
        memory: measured(10),
        volume: roomy,
        youngBees: 12,
        settings: { ...defaults, warmupSeconds },
        volumeDir: '/workspace',
      })
      return refused.ok ? '' : refused.summary
    }
    expect(at(20)).toContain('started in the last 20 s are')
    expect(at(90)).toContain('started in the last 90 s are')
    expect(at(600)).toContain('started in the last 10 min are')
  })

  it("refuses on disk with the reaper's counts in the sentence", () => {
    const short = { totalBytes: 50 * GB, freeBytes: 5.1 * GB }
    const first = judge(measured(3), 0, short)
    expect(first.ok).toBe(false)
    if (first.ok) return
    expect(first.resource).toBe('disk')
    expect(first.detail).toContain(
      'has 5.1 GB free of 50.0 GB (nothing reaped yet)',
    )
    const after = judgeBeeRoom({
      memory: measured(3),
      volume: short,
      youngBees: 0,
      settings: defaults,
      volumeDir: '/workspace/BrowserOS',
      reaped: { removed: 2, keptDirty: 1, keptRunning: 14 },
    })
    if (after.ok) throw new Error('expected a refusal')
    expect(after.detail).toContain(
      'after a reap (2 removed; kept 1 with uncommitted work and 14 of running bees)',
    )
    expect(after.detail).toContain('needs about 2.1 GB and 5.0 GB stays free')
    // Where the reaper may stop: 2048 MiB + 5 GB of 50 GB is 85.7% used.
    expect(diskLineUsedPercent(short, defaults)).toBe(85)
    // A path may hold ". ", so the report reads the field and never the prose.
    const dotted = judgeBeeRoom({
      memory: measured(3),
      volume: short,
      youngBees: 0,
      settings: defaults,
      volumeDir: '/Volumes/Ext. Drive/ws/BrowserOS',
    })
    if (dotted.ok) throw new Error('expected a refusal')
    expect(dotted.summary).toContain(
      '/Volumes/Ext. Drive/ws/BrowserOS has 5.1 GB',
    )
    expect(dotted.summary.endsWith('GB stays free')).toBe(true)
  })

  it('leaves a checkout that is not there to the code that can say so', () => {
    // The entrypoint starts the server even when the clone failed. `df` on the
    // missing directory gives nothing, and "no room in the container: disk"
    // with nineteen GB free hid the git error that names the real cause.
    expect(
      judgeBeeRoom({
        memory: measured(3),
        volume: null,
        youngBees: 0,
        settings: defaults,
        volumeDir: '/workspace/BrowserOS',
        checkoutMissing: true,
      }),
    ).toEqual({ ok: true })
    // Memory is still judged: a full container is full wherever the checkout is.
    expect(
      judgeBeeRoom({
        memory: measured(23),
        volume: null,
        youngBees: 0,
        settings: defaults,
        volumeDir: '/workspace/BrowserOS',
        checkoutMissing: true,
      }).ok,
    ).toBe(false)
  })

  it('does not let a share of a huge disk refuse a laptop with ninety GB to spare', () => {
    const laptop = { totalBytes: 1000 * GB, freeBytes: 90 * GB }
    expect(
      judge({ kind: 'unsupported', platform: 'darwin' }, 0, laptop),
    ).toEqual({ ok: true })
  })

  it('treats unknown as a refusal and not-Linux as nothing to protect', () => {
    const blind = judge({
      kind: 'unknown',
      tried: ['cgroup v2', 'cgroup v1', '/proc/meminfo'],
    })
    if (blind.ok) throw new Error('expected a refusal')
    expect(blind.detail).toContain('unknown is not room')
    expect(blind.detail).toContain('TRIOS_QUEEN_RESOURCE_GUARD=off')
    const noDisk = judgeBeeRoom({
      memory: measured(3),
      volume: null,
      youngBees: 0,
      settings: defaults,
      volumeDir: '/workspace',
    })
    if (noDisk.ok) throw new Error('expected a refusal')
    expect(noDisk.resource).toBe('disk')
    expect(judge({ kind: 'unsupported', platform: 'darwin' })).toEqual({
      ok: true,
    })
  })

  it('reports one cause, memory first, and leaks nothing of the closed breakdown', () => {
    const both = judge(measured(23.5), 0, {
      totalBytes: 50 * GB,
      freeBytes: 1 * GB,
    })
    if (both.ok) throw new Error('expected a refusal')
    expect(both.resource).toBe('memory')
    for (const word of [
      'connectedCredentials',
      'lanesPerCredential',
      'effectiveCapacity',
      'API_KEY',
    ]) {
      expect(both.detail).not.toContain(word)
    }
  })
})
