/**
 * Whether the container can carry one more bee.
 *
 * THE MEASUREMENT, 2026-09-17. The container is limited to 24 GB. With sixteen
 * bees running it held 14.7 GB, idle it holds one to three: about 0.85 GB per
 * running bee. The same day the worker ceiling stopped being a constant and the
 * credential list grew to thirty-four lanes. A full swarm would therefore have
 * asked for about thirty GB of a twenty-four GB container, and the kernel
 * answers that question by killing the container - every running bee with it,
 * each holding the only copy of its unfinished turn. The volume is the second
 * way to die: 50 GB, 31 in use, 36 at the day's peak.
 *
 * Nothing in dispatch looked at memory at all. The ceiling was a number, the
 * credentials were a number, and the one thing that actually decides how many
 * bees fit - the container - was not asked.
 *
 * So it is asked, before every start, and it is asked AGAIN every time: every
 * variable and every reading below is taken on the call, never cached, so the
 * line moves with the container and with the operator's variables and no
 * restart is ever needed for the guard to notice either.
 *
 * WHAT THIS IS NOT. It is admission control. A bee that was admitted and then
 * grows far past its reserve can still take the container down; the headroom is
 * the only buffer against that. And it caps nothing by count - the policy and
 * the credentials still decide how many bees MAY run; this only decides
 * whether the next one FITS.
 */
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

export type MemoryReading =
  | {
      kind: 'measured'
      usedBytes: number
      limitBytes: number
      /** Where the use was read: 'cgroup v2', 'cgroup v1' or '/proc/meminfo'. */
      source: string
      /** Where the limit came from: 'cgroup', 'host MemTotal' or the variable. */
      limitSource: string
      /** Said once in the log: a stated limit the kernel's own limit overrules. */
      note?: string
    }
  /** Not Linux: a development or test machine has nothing to read. */
  | { kind: 'unsupported'; platform: string }
  /** Linux, and nothing usable. Unknown is not room. */
  | { kind: 'unknown'; tried: string[]; why?: string }

export interface VolumeSpace {
  totalBytes: number
  freeBytes: number
}

export interface ResourceSettings {
  guardOn: boolean
  beeMemoryBytes: number
  memoryHeadroomPercent: number
  warmupSeconds: number
  beeDiskBytes: number
  diskHeadroomPercent: number
  memoryLimitBytes: number | null
  /** One sentence per variable that was set to something unusable. */
  notes: string[]
}

export type BeeRoom =
  | { ok: true }
  | {
      ok: false
      resource: 'memory' | 'disk'
      /**
       * The first sentence of `detail`, without its period: every number, no
       * variable name, under 200 characters. A field rather than something a
       * reader cuts out of the prose, because the prose names the workspace
       * path and a path may contain ". " - "/Volumes/Ext. Drive" ended the
       * sentence after "Ext" when the report split on it.
       */
      summary: string
      detail: string
    }

const MB = 1024 * 1024
/**
 * DECIMAL, for the headroom caps and for every number printed. Railway sells
 * "24 GB" and its cgroup file says 24000000000; printed in binary units that is
 * "22.4 GB", which reads as a wrong limit and invites an operator to "correct"
 * it with TRIOS_QUEEN_MEMORY_LIMIT_MB. The `_MB` variables stay binary, as
 * every memory flag an operator has met is.
 */
const GB = 1_000_000_000
/**
 * The headroom is a share of the limit, and a share of something huge is huge:
 * ten percent of a developer's terabyte disk is a hundred GB "kept free", and
 * the guard would refuse every local dispatch on a laptop with ninety GB to
 * spare. The share protects a SMALL container; past these sizes it stops
 * growing.
 */
const MEMORY_HEADROOM_CAP = 8 * GB
const DISK_HEADROOM_CAP = 10 * GB
/** cgroup v1 writes "no limit" as a number near 2^63. */
const V1_UNLIMITED = 2 ** 60

function gb(bytes: number): string {
  return (bytes / GB).toFixed(1)
}

function readFileOrNull(path: string): string | null {
  try {
    return readFileSync(path, 'utf8')
  } catch {
    return null
  }
}

function statValue(stat: string | null, key: string): number {
  if (!stat) return 0
  const match = new RegExp(`^${key}\\s+(\\d+)$`, 'm').exec(stat)
  return match ? Number(match[1]) : 0
}

function meminfoBytes(meminfo: string | null, key: string): number | null {
  if (!meminfo) return null
  const match = new RegExp(`^${key}:\\s+(\\d+)\\s*kB`, 'm').exec(meminfo)
  return match ? Number(match[1]) * 1024 : null
}

/**
 * Read on every call. A value that cannot be used is replaced by its default
 * AND SAID SO: a variable that is silently ignored is the zero-length-key trap
 * again - it looks configured in the editor and changes nothing.
 */
export function resourceSettings(
  env: NodeJS.ProcessEnv = process.env,
): ResourceSettings {
  const notes: string[] = []
  const bounded = (
    name: string,
    fallback: number,
    low: number,
    high: number,
  ): number => {
    const raw = env[name]?.trim()
    if (!raw) return fallback
    const parsed = Number(raw)
    if (!Number.isInteger(parsed) || parsed < low || parsed > high) {
      notes.push(
        `${name}=${raw.slice(0, 24)} is not an integer from ${low} to ${high}; using ${fallback}`,
      )
      return fallback
    }
    return parsed
  }
  const guardRaw = env.TRIOS_QUEEN_RESOURCE_GUARD?.trim().toLowerCase()
  if (guardRaw && guardRaw !== 'on' && guardRaw !== 'off') {
    notes.push(
      `TRIOS_QUEEN_RESOURCE_GUARD=${guardRaw.slice(0, 24)} is neither on nor off; the guard stays on`,
    )
  }
  // Not `bounded`: this one has no default to fall back TO. Unset, empty and 0
  // all mean "read the limit from the container", and an unusable value must
  // say THAT - "using 0" would tell an operator the container has no memory.
  const limitRaw = env.TRIOS_QUEEN_MEMORY_LIMIT_MB?.trim()
  let limitMb = 0
  if (limitRaw && limitRaw !== '0') {
    const parsed = Number(limitRaw)
    if (Number.isInteger(parsed) && parsed >= 512 && parsed <= 4_194_304) {
      limitMb = parsed
    } else {
      notes.push(
        `TRIOS_QUEEN_MEMORY_LIMIT_MB=${limitRaw.slice(0, 24)} is not an integer from 512 to 4194304; ` +
          'using the limit the container reports',
      )
    }
  }
  return {
    guardOn: guardRaw !== 'off',
    beeMemoryBytes:
      bounded('TRIOS_QUEEN_BEE_MEMORY_MB', 1024, 128, 65_536) * MB,
    memoryHeadroomPercent: bounded(
      'TRIOS_QUEEN_MEMORY_HEADROOM_PERCENT',
      10,
      0,
      50,
    ),
    warmupSeconds: bounded('TRIOS_QUEEN_BEE_WARMUP_SECONDS', 600, 0, 3600),
    beeDiskBytes: bounded('TRIOS_QUEEN_BEE_DISK_MB', 2048, 256, 1_048_576) * MB,
    diskHeadroomPercent: bounded(
      'TRIOS_QUEEN_DISK_HEADROOM_PERCENT',
      10,
      0,
      50,
    ),
    memoryLimitBytes: limitMb > 0 ? limitMb * MB : null,
    notes,
  }
}

/**
 * What the container holds that the kernel CANNOT give back, against its limit.
 *
 * Not `memory.current`: that counts page cache, and a server that clones
 * repositories and links node_modules fills the cache constantly. And not the
 * kubelet's working set either (`current - inactive_file`), which was the first
 * version: it still counts `active_file` and reclaimable slab, and those come
 * back only under pressure AT the limit - which this guard refuses before ever
 * reaching. On a classic-LRU kernel a page read twice is promoted to active and
 * nothing demotes it without pressure, so the figure only grows: a review put
 * 16 GB of active cache in a container with no bee running and the guard
 * answered "19.6 GB in use" every round, for good, because a memory refusal
 * frees nothing. What kills a container is memory that reclaim cannot free, so
 * that is what is counted: current minus the file cache, both lists, minus
 * reclaimable slab. `/proc/meminfo`'s MemAvailable already means the same.
 * Mapped and dirty file pages it under-counts are what the headroom is for.
 *
 * The limit is the cgroup's. When the cgroup says "max" the container is not
 * the thing that is limited and the host's MemTotal is the real ceiling; when
 * even that is not what the platform enforces, TRIOS_QUEEN_MEMORY_LIMIT_MB
 * states it, and replaces the LIMIT only - use is always measured.
 */
export function readContainerMemory(
  readers: {
    readFile?: (path: string) => string | null
    platform?: string
    env?: NodeJS.ProcessEnv
  } = {},
): MemoryReading {
  const platform = readers.platform ?? process.platform
  if (platform !== 'linux') return { kind: 'unsupported', platform }
  const read = readers.readFile ?? readFileOrNull
  const stated = resourceSettings(readers.env ?? process.env).memoryLimitBytes
  const meminfo = read('/proc/meminfo')
  const hostTotal = meminfoBytes(meminfo, 'MemTotal')
  const tried: string[] = []

  const hostUsed =
    hostTotal !== null && meminfoBytes(meminfo, 'MemAvailable') !== null
      ? hostTotal - (meminfoBytes(meminfo, 'MemAvailable') ?? 0)
      : null

  const finish = (
    usedRaw: number,
    cgroupLimit: number | null,
    source: string,
  ): MemoryReading | null => {
    // THE KERNEL'S LIMIT WINS. A stated limit can only LOWER the line: the
    // kernel kills at its own number whatever a variable says, so a stated
    // 24576 MB over a cgroup limit of 24000000000 bytes would have moved the
    // guard's line to within one bee of the kill line, and said nothing.
    const ceiling = cgroupLimit ?? hostTotal
    const limit =
      stated && ceiling ? Math.min(stated, ceiling) : (stated ?? ceiling)
    if (!limit || limit <= 0) return null
    const statedWins = Boolean(stated) && limit === stated
    // The cgroup says "max": the container is not what is limited, the machine
    // is - and the machine is also used by everything else on it. Judging this
    // cgroup's own use against the host's total admitted thirteen more bees
    // into a host with half a GB available.
    const onHost = cgroupLimit === null && !statedWins && hostUsed !== null
    const used = onHost ? Math.max(usedRaw, hostUsed ?? 0) : usedRaw
    return {
      kind: 'measured',
      usedBytes: Math.max(0, used),
      limitBytes: limit,
      source: onHost && used !== usedRaw ? '/proc/meminfo' : source,
      limitSource: statedWins
        ? 'TRIOS_QUEEN_MEMORY_LIMIT_MB'
        : cgroupLimit
          ? 'cgroup'
          : 'host MemTotal',
      ...(stated && !statedWins
        ? {
            note:
              `TRIOS_QUEEN_MEMORY_LIMIT_MB states ${gb(stated)} GB, above the ` +
              `${gb(limit)} GB the ${cgroupLimit ? 'kernel enforces' : 'machine has'}; ` +
              'using the smaller',
          }
        : {}),
    }
  }

  tried.push('cgroup v2')
  const v2Current = read('/sys/fs/cgroup/memory.current')
  if (v2Current !== null && /^\d+\s*$/.test(v2Current)) {
    const max = (read('/sys/fs/cgroup/memory.max') ?? '').trim()
    const stat = read('/sys/fs/cgroup/memory.stat')
    const givenBack =
      statValue(stat, 'inactive_file') +
      statValue(stat, 'active_file') +
      statValue(stat, 'slab_reclaimable')
    const reading = finish(
      Number(v2Current) - givenBack,
      /^\d+$/.test(max) ? Number(max) : null,
      'cgroup v2',
    )
    if (reading) return reading
  }

  tried.push('cgroup v1')
  const v1Usage = read('/sys/fs/cgroup/memory/memory.usage_in_bytes')
  if (v1Usage !== null && /^\d+\s*$/.test(v1Usage)) {
    const limitRaw = Number(
      (read('/sys/fs/cgroup/memory/memory.limit_in_bytes') ?? '').trim(),
    )
    const stat = read('/sys/fs/cgroup/memory/memory.stat')
    const givenBack =
      statValue(stat, 'total_inactive_file') +
      statValue(stat, 'total_active_file')
    const reading = finish(
      Number(v1Usage) - givenBack,
      Number.isFinite(limitRaw) && limitRaw > 0 && limitRaw < V1_UNLIMITED
        ? limitRaw
        : null,
      'cgroup v1',
    )
    if (reading) return reading
  }

  // MEASURED from inside a Railway container, 2026-09-17: memory.max reads
  // 24000000000 and /proc/meminfo reads MemTotal 338 GB - the HOST. So this last
  // resort describes the whole machine, which is the right thing to judge on a
  // machine with no container around the server and the wrong thing to hold
  // against a stated CONTAINER limit: the host's use is always far above it and
  // every start would be refused for memory this container never touched.
  tried.push('/proc/meminfo')
  const available = meminfoBytes(meminfo, 'MemAvailable')
  if (hostTotal !== null && available !== null) {
    if (stated) {
      return {
        kind: 'unknown',
        tried,
        why:
          'TRIOS_QUEEN_MEMORY_LIMIT_MB states a container limit, but only ' +
          '/proc/meminfo is readable and it measures the whole host',
      }
    }
    return {
      kind: 'measured',
      usedBytes: Math.max(0, hostTotal - available),
      limitBytes: hostTotal,
      source: '/proc/meminfo',
      limitSource: 'host MemTotal',
    }
  }
  return { kind: 'unknown', tried }
}

/** One line saying what the guard reads, for the log - not for a refusal. */
export function describeReading(memory: MemoryReading): string {
  if (memory.kind === 'unsupported') {
    return `memory is not measured on ${memory.platform}; only the volume is`
  }
  if (memory.kind === 'unknown') {
    return `memory cannot be measured (tried ${memory.tried.join(', ')})`
  }
  return (
    `memory from ${memory.source}, limit ${gb(memory.limitBytes)} GB ` +
    `from ${memory.limitSource}`
  )
}

/**
 * Free bytes on the volume, from `df -Pk` - the same subprocess, for the same
 * reason, as `volumeUsedPercent`: statfs arithmetic is exact on the Linux
 * container and wrong on an APFS shared container, and a guard that refuses
 * every dispatch in a unit test is a guard somebody switches off.
 */
export function volumeSpace(
  dir: string,
  run: (dir: string) => string = (target) =>
    execFileSync('df', ['-Pk', target], {
      encoding: 'utf8',
      timeout: 10_000,
      stdio: ['ignore', 'pipe', 'ignore'],
    }),
): VolumeSpace | null {
  try {
    const line = run(dir).trim().split('\n').pop() ?? ''
    // Filesystem 1024-blocks Used Available Capacity Mounted-on
    const match = /^\S+\s+(\d+)\s+\d+\s+(\d+)\s+\d+%/.exec(line)
    if (!match) return null
    const totalBytes = Number(match[1]) * 1024
    const freeBytes = Number(match[2]) * 1024
    return totalBytes > 0 ? { totalBytes, freeBytes } : null
  } catch {
    return null
  }
}

/**
 * Bees that have started and not yet grown.
 *
 * The tick starts bees back to back, and a bee takes minutes to allocate what
 * it will hold. Thirty-four starts in one round would each see the same nearly
 * empty container and each be admitted; the reading is true and the conclusion
 * is wrong. So every start inside the warm-up window reserves a bee's memory
 * that the reading cannot show yet.
 *
 * Process-local on purpose: after a restart every bee has been reaped, so an
 * empty list is the correct list. It reserves memory; it caps nothing by count.
 */
const youngBees = new Map<string, number>()
/** No warm-up this module accepts is longer; past it an entry is only garbage. */
const LONGEST_WARMUP_MS = 3600 * 1000

/**
 * Keyed by the turn, because a reservation must end with the bee. `turn.ok`
 * only means /chat answered 200, and a provider refusal arrives exactly that
 * way: 200, then the failure in the stream. Kept as bare timestamps, ten such
 * dead starts in ten minutes reserved ten GB for bees that no longer existed
 * and the guard refused a nearly empty container, saying they were "still
 * growing". `closeDispatch`, where every ending lands, calls `noteBeeEnded`.
 */
export function noteBeeStarted(turn: string, nowMs: number): void {
  youngBees.set(turn, nowMs)
}

export function noteBeeEnded(turn: string): void {
  youngBees.delete(turn)
}

export function youngBeeCount(nowMs: number, warmupSeconds: number): number {
  const horizon = nowMs - warmupSeconds * 1000
  let young = 0
  for (const [turn, startedAt] of youngBees) {
    // Dropped only past the LONGEST window, never past the caller's: the window
    // is read on every call and may grow, and a count must not destroy what a
    // later count would have needed.
    if (nowMs - startedAt > LONGEST_WARMUP_MS) youngBees.delete(turn)
    else if (startedAt > horizon) young += 1
  }
  return warmupSeconds > 0 ? young : 0
}

export function resetYoungBees(): void {
  youngBees.clear()
}

/**
 * One more bee: does it fit?
 *
 * Pure. Memory is judged first because it is the failure that takes everything
 * with it, and ONE cause is reported - a sentence naming two is a sentence an
 * operator acts on half of. Unknown is a refusal: a guard that reads an
 * unmeasurable container as empty disables itself exactly when the container
 * is unwell. Not-Linux passes, because there is no container to protect.
 *
 * The FIRST SENTENCE carries every number, names no variable and stays under
 * 200 characters, because it is the part the round report quotes as the reason
 * nothing started (`containerRefusal` in queen-report-lines.ts). What to do
 * about it comes after.
 */
export function judgeBeeRoom(input: {
  memory: MemoryReading
  volume: VolumeSpace | null
  youngBees: number
  settings: ResourceSettings
  volumeDir: string
  reaped?: { removed: number; keptDirty: number; keptRunning: number }
  /**
   * The workspace directory does not exist: there is no checkout to fill, and
   * `df` on a missing path fails like a sick filesystem does. The entrypoint
   * starts the server even when the clone failed, and "no room in the
   * container: disk" with nineteen GB free sent the operator to the wrong
   * place while hiding the git error that names the real one. So disk is not
   * judged, and `prepareWorktree` says what is actually wrong.
   */
  checkoutMissing?: boolean
}): BeeRoom {
  const { memory, volume, youngBees, settings, volumeDir, reaped } = input

  const refuse = (
    resource: 'memory' | 'disk',
    summary: string,
    advice: string,
  ): BeeRoom => ({
    ok: false,
    resource,
    summary,
    detail: `${summary}. ${advice}`,
  })

  if (memory.kind === 'unknown') {
    return refuse(
      'memory',
      `container memory cannot be measured (tried ${memory.tried.join(', ')}) ` +
        'and unknown is not room',
      `${memory.why ? `${memory.why}. ` : ''}Not starting a bee blind; ` +
        'TRIOS_QUEEN_RESOURCE_GUARD=off dispatches without the guard.',
    )
  }
  if (memory.kind === 'measured') {
    const keepFree = Math.min(
      (memory.limitBytes * settings.memoryHeadroomPercent) / 100,
      MEMORY_HEADROOM_CAP,
    )
    const asked = (youngBees + 1) * settings.beeMemoryBytes
    if (memory.usedBytes + asked > memory.limitBytes - keepFree) {
      // Short on purpose: the first sentence is what a report line shows.
      const where =
        memory.limitSource === 'cgroup'
          ? memory.source
          : memory.limitSource === 'host MemTotal'
            ? `${memory.source}, host total`
            : `${memory.source}, stated limit`
      // The window as it was set: "0 min" for twenty seconds told an operator
      // the count was taken over nothing.
      const window =
        settings.warmupSeconds % 60 === 0
          ? `${settings.warmupSeconds / 60} min`
          : `${settings.warmupSeconds} s`
      const growing =
        youngBees > 0
          ? ` and ${youngBees} bee(s) started in the last ${window} are still growing`
          : ''
      return refuse(
        'memory',
        `container memory is ${gb(memory.usedBytes)} GB of ` +
          `${gb(memory.limitBytes)} GB in use (${where})${growing}; another bee ` +
          `needs about ${gb(settings.beeMemoryBytes)} GB and ${gb(keepFree)} GB ` +
          'stays free',
        'Not starting a bee that would get every running bee killed. Asked ' +
          'again every round, and a finishing bee is what gives memory back; ' +
          'TRIOS_QUEEN_BEE_MEMORY_MB and TRIOS_QUEEN_MEMORY_HEADROOM_PERCENT ' +
          '(MiB and percent) move the line.',
      )
    }
  }

  if (input.checkoutMissing) return { ok: true }
  if (volume === null) {
    return refuse(
      'disk',
      `free space on ${volumeDir} cannot be measured (df gave nothing) and ` +
        'unknown is not room',
      'Not starting a bee blind; TRIOS_QUEEN_RESOURCE_GUARD=off dispatches ' +
        'without the guard.',
    )
  }
  const keepFree = diskKeptFree(volume, settings)
  if (volume.freeBytes < settings.beeDiskBytes + keepFree) {
    const reaping = reaped
      ? `after a reap (${reaped.removed} removed; kept ${reaped.keptDirty} with ` +
        `uncommitted work and ${reaped.keptRunning} of running bees)`
      : '(nothing reaped yet)'
    return refuse(
      'disk',
      `volume ${volumeDir} has ${gb(volume.freeBytes)} GB free of ` +
        `${gb(volume.totalBytes)} GB ${reaping}; another bee needs about ` +
        `${gb(settings.beeDiskBytes)} GB and ${gb(keepFree)} GB stays free`,
      'Not starting a bee that would fill the disk under the running ones. ' +
        'TRIOS_QUEEN_BEE_DISK_MB and TRIOS_QUEEN_DISK_HEADROOM_PERCENT move the line.',
    )
  }
  return { ok: true }
}

function diskKeptFree(volume: VolumeSpace, settings: ResourceSettings): number {
  return Math.min(
    (volume.totalBytes * settings.diskHeadroomPercent) / 100,
    DISK_HEADROOM_CAP,
  )
}

/**
 * The used-percent at which the disk rule is satisfied, for the reaper to stop
 * at. Without it the one reap the guard asks for ran to QUEEN_VOLUME_LOW - 55%
 * - and removed every clean tree it was allowed to, when the guard needed seven
 * GB. Rounded down, so stopping there is never stopping short.
 */
export function diskLineUsedPercent(
  volume: VolumeSpace,
  settings: ResourceSettings,
): number {
  const need = settings.beeDiskBytes + diskKeptFree(volume, settings)
  return Math.max(0, Math.floor(100 * (1 - need / volume.totalBytes)))
}
