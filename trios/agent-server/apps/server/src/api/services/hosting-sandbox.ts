/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The OS sandbox a host runs a job in (gHashTag/trios#1761, lane A). A shard
 * is code: t27c compiles the spec and zig builds and runs its tests on this
 * machine. Plumbing only. Which sandbox this machine uses, the level it
 * reports and which job needs which level are host.t27 section 8 card calls;
 * the macOS profile and the bwrap and unshare arguments are data in
 * toolchain.t27 section 6 (queen-hosting-toolchain.gen). This file finds the
 * tools, fills the paths in and runs the planted job.
 *
 * THE PLANTED JOB: before a host registers, it compiles a small program with
 * the job's own zig, inside the sandbox, the way a shard's tests are built,
 * and runs it inside the sandbox. The program opens a TCP connection to a
 * listener this process holds on 127.0.0.1 and creates a file in the
 * operator's real home directory. The level the host reports is
 * isolation_of(sandbox, connect failed, write failed): measured, not
 * assumed. Run without the sandbox, the same program connects and writes,
 * which is the negative control. WHY C: the planted job has to make the two
 * system calls a malicious spec's test binary would make, and no t27 spec can
 * open a socket; C is what `zig cc` compiles with no library of its own.
 */

import { spawnSync } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import {
  existsSync,
  mkdirSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createServer } from 'node:net'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { isolationOf, sandboxChoice } from './hosting-cards'
import {
  SANDBOX_BWRAP,
  SANDBOX_EXEC,
  SANDBOX_NONE,
  SANDBOX_UNSHARE,
} from './queen-hosting-host-card.gen'
import {
  BWRAP_ARGS,
  BWRAP_BINDS,
  SANDBOX_EXEC_PARAMS,
  SANDBOX_EXEC_PROFILE,
  UNSHARE_ARGS,
} from './queen-hosting-toolchain.gen'

export const SANDBOX_WORDS = ['none', 'sandbox-exec', 'bwrap', 'unshare']
export const ISOLATION_WORDS = ['none', 'no-network', 'job-dir-only', 'vm']

/** The paths a job may reach, by toolchain.t27 SANDBOX_EXEC_PARAMS name. */
export interface SandboxPaths {
  T27C: string
  T27B: string
  ZIG: string
  JOB: string
  CACHE: string
}

const which = (tool: string): string | null => {
  const r = spawnSync('/bin/sh', ['-c', `command -v ${tool}`], {
    encoding: 'utf8',
  })
  const path = r.stdout.trim()
  return r.status === 0 && path ? path : null
}

/** The sandbox this machine uses (host.t27 sandbox_choice). */
export function detectSandbox(turnedOff = false): number {
  return sandboxChoice({
    macos: process.platform === 'darwin',
    hasSandboxExec: existsSync('/usr/bin/sandbox-exec'),
    hasBwrap: process.platform === 'linux' && which('bwrap') !== null,
    hasUnshare: process.platform === 'linux' && which('unshare') !== null,
    turnedOff,
  })
}

const fill = (arg: string, paths: SandboxPaths) =>
  arg.replace(
    /\{(T27C|T27B|ZIG|JOB|CACHE)\}/g,
    (_, name: keyof SandboxPaths) => paths[name],
  )

/**
 * WHY REAL PATHS: both sandboxes match the path the kernel resolved. On macOS
 * the temporary directory is /var/folders/..., a link to /private/var/...,
 * and a rule naming the link matches nothing.
 */
const real = (path: string): string => {
  try {
    return realpathSync(path)
  } catch {
    return path
  }
}

/** `argv` as the sandbox runs it. */
export function sandboxed(
  sandbox: number,
  given: SandboxPaths,
  argv: string[],
): string[] {
  const paths: SandboxPaths = {
    T27C: real(given.T27C),
    T27B: real(given.T27B),
    ZIG: real(given.ZIG),
    JOB: real(given.JOB),
    CACHE: real(given.CACHE),
  }
  if (sandbox === SANDBOX_EXEC) {
    const params = (SANDBOX_EXEC_PARAMS as readonly string[]).flatMap(
      (name) => ['-D', `${name}=${paths[name as keyof SandboxPaths]}`],
    )
    return [
      '/usr/bin/sandbox-exec',
      ...params,
      '-p',
      SANDBOX_EXEC_PROFILE.join('\n'),
      ...argv,
    ]
  }
  if (sandbox === SANDBOX_BWRAP)
    return [
      'bwrap',
      ...[...BWRAP_ARGS, ...BWRAP_BINDS].map((a) => fill(a, paths)),
      '--',
      ...argv,
    ]
  if (sandbox === SANDBOX_UNSHARE)
    return ['unshare', ...UNSHARE_ARGS, '--', ...argv]
  return argv
}

/** The planted job's source: a connect and a write, each attempt's errno printed. */
const PROBE_C = `#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <unistd.h>
int main(int argc, char **argv) {
  if (argc != 4) return 64;
  int fd = open(argv[1], O_WRONLY | O_CREAT | O_EXCL, 0600);
  int werr = fd < 0 ? errno : 0;
  if (fd >= 0) close(fd);
  int s = socket(AF_INET, SOCK_STREAM, 0);
  int cerr = s < 0 ? errno : 0;
  if (s >= 0) {
    struct sockaddr_in a = {0};
    a.sin_family = AF_INET;
    a.sin_port = htons((unsigned short)atoi(argv[3]));
    inet_pton(AF_INET, argv[2], &a.sin_addr);
    if (connect(s, (struct sockaddr *)&a, sizeof a) != 0) cerr = errno;
    close(s);
  }
  printf("write %d\\nconnect %d\\n", werr, cerr);
  return 0;
}
`

export interface ProbeResult {
  sandbox: number
  /** The planted job was built and ran to its last line. */
  ran: boolean
  connected: boolean
  wrote: boolean
  /** The level to report (host.t27 isolation_of). */
  isolation: number
  /** What the planted job printed, or why it did not run. */
  detail: string
  millis: number
}

/**
 * Run the planted job under `sandbox` (SANDBOX_NONE runs it bare: the
 * negative control). The file it tries to create is in the operator's real
 * home, and is removed again if it was made.
 */
export async function probeIsolation(opts: {
  sandbox: number
  zig: string
  /** Where the probe's job directory is made; it is removed afterwards. */
  workRoot: string
  cacheDir: string
  t27c?: string
  t27b?: string
  home?: string
}): Promise<ProbeResult> {
  const started = Date.now()
  const job = join(opts.workRoot, `probe-${randomBytes(6).toString('hex')}`)
  mkdirSync(join(job, 'tmp'), { recursive: true })
  mkdirSync(opts.cacheDir, { recursive: true })
  const target = join(
    opts.home ?? homedir(),
    `.trios-host-probe-${randomBytes(6).toString('hex')}`,
  )
  const paths: SandboxPaths = {
    T27C: opts.t27c ?? opts.zig,
    T27B: opts.t27b ?? opts.t27c ?? opts.zig,
    ZIG: dirname(opts.zig),
    JOB: job,
    CACHE: opts.cacheDir,
  }
  const env = {
    PATH: `${dirname(opts.zig)}:/usr/bin:/bin`,
    HOME: job,
    TMPDIR: join(job, 'tmp'),
    ZIG_GLOBAL_CACHE_DIR: opts.cacheDir,
    ZIG_LOCAL_CACHE_DIR: join(job, 'zig-local'),
  }
  let connected = false
  const server = createServer((socket) => {
    connected = true
    socket.destroy()
  })
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  const port = (server.address() as { port: number }).port
  try {
    writeFileSync(join(job, 'probe.c'), PROBE_C)
    const [cc, ...ccArgs] = sandboxed(opts.sandbox, paths, [
      opts.zig,
      'cc',
      '-O1',
      '-o',
      join(job, 'probe'),
      join(job, 'probe.c'),
    ])
    const build = spawnSync(cc as string, ccArgs, {
      cwd: job,
      env,
      encoding: 'utf8',
      timeout: 300_000,
    })
    if (build.status !== 0) {
      const why = (build.stderr || build.stdout || String(build.error)).trim()
      return finish(
        false,
        `the planted job did not build: ${why.split('\n')[0]}`,
      )
    }
    const argv = sandboxed(opts.sandbox, paths, [
      join(job, 'probe'),
      target,
      '127.0.0.1',
      String(port),
    ])
    const run = await new Promise<{ status: number | null; out: string }>(
      (resolve) => {
        // async, so the listener above can accept while the job runs
        const p = Bun.spawn(argv, {
          cwd: job,
          env,
          stdout: 'pipe',
          stderr: 'pipe',
        })
        void Promise.all([new Response(p.stdout).text(), p.exited]).then(
          ([out, status]) => resolve({ status, out }),
        )
      },
    )
    const ran =
      run.status === 0 && /^write -?\d+\nconnect -?\d+\n$/.test(run.out)
    return finish(ran, run.out.trim().replace(/\n/g, ', '))
  } finally {
    server.close()
    rmSync(job, { recursive: true, force: true })
    rmSync(target, { force: true })
  }

  function finish(ran: boolean, detail: string): ProbeResult {
    const wrote = existsSync(target)
    return {
      sandbox: opts.sandbox,
      ran,
      connected,
      wrote,
      // a planted job that never ran proves nothing: report none
      isolation: isolationOf(opts.sandbox, ran && !connected, ran && !wrote),
      detail,
      millis: Date.now() - started,
    }
  }
}

export { SANDBOX_NONE }
