/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's encryption (gHashTag/trios#1759, design rule 3): age, through
 * its reference command-line tool. Nothing here is cryptography; it starts
 * `age` and `age-keygen` (https://age-encryption.org, X25519 recipients,
 * ChaCha20-Poly1305 payload) and carries bytes to and from them.
 *
 * WHY THE TOOL AND NOT A LIBRARY: trios has no age library and no libsodium
 * (node_modules holds @noble/ciphers only as a transitive dependency of
 * better-auth). Assembling X25519, HKDF and ChaCha20-Poly1305 by hand would
 * be the home-made crypto rule 3 forbids. The tool is the reference
 * implementation, it is what the owner's own export of 2026-10-10 was made
 * with, and an export made here opens with the stock `age -d` on any machine.
 *
 * WHAT NEVER HAPPENS HERE: a plaintext is never an argument (it goes in on
 * stdin), never written to a file, and never part of an error message (an
 * error carries the exit code and age's own stderr, which names no value).
 * An identity is read by age from its file, or from stdin: the one-lease
 * identity a `run` holds in memory, and the vault's own identity when it
 * comes from the platform's variable (policy.t27 IDENTITY_VAR). An identity
 * on stdin is never written to disk, and no identity is ever an argument.
 * The node binaries are found on PATH, or at TRIOS_VAULT_AGE and
 * TRIOS_VAULT_AGE_KEYGEN; the server's image must carry them.
 */

import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

export class AgeError extends Error {}

/** The binaries: TRIOS_VAULT_AGE and TRIOS_VAULT_AGE_KEYGEN, else from PATH. */
export function ageBinaries(env = process.env): {
  age: string
  keygen: string
} {
  return {
    age: env.TRIOS_VAULT_AGE?.trim() || 'age',
    keygen: env.TRIOS_VAULT_AGE_KEYGEN?.trim() || 'age-keygen',
  }
}

interface Ran {
  code: number
  stdout: Uint8Array
  stderr: string
}

async function run(argv: string[], stdin?: Uint8Array): Promise<Ran> {
  let proc: ReturnType<typeof Bun.spawn>
  try {
    proc = Bun.spawn(argv, {
      stdin: stdin ?? 'ignore',
      stdout: 'pipe',
      stderr: 'pipe',
    })
  } catch (error) {
    throw new AgeError(
      `cannot start ${argv[0]}: ${error instanceof Error ? error.message : String(error)}`,
    )
  }
  const [out, err, code] = await Promise.all([
    new Response(proc.stdout as ReadableStream).arrayBuffer(),
    new Response(proc.stderr as ReadableStream).text(),
    proc.exited,
  ])
  return { code, stdout: new Uint8Array(out), stderr: err }
}

const fail = (what: string, r: Ran): never => {
  throw new AgeError(
    `${what} failed (exit ${r.code}): ${r.stderr.trim().slice(0, 300)}`,
  )
}

const RECIPIENT = /^age1[0-9a-z]{58}$/

export const isRecipient = (text: string): boolean => RECIPIENT.test(text)

/**
 * A new identity in a file the tool creates (mode 0600); returns its
 * recipient. The secret half is never read into this process.
 */
export async function keygenToFile(
  path: string,
  env = process.env,
): Promise<string> {
  const { keygen } = ageBinaries(env)
  const r = await run([keygen, '-o', path])
  if (r.code !== 0) fail('age-keygen', r)
  return recipientOfFile(path, env)
}

export async function recipientOfFile(
  path: string,
  env = process.env,
): Promise<string> {
  const { keygen } = ageBinaries(env)
  const r = await run([keygen, '-y', path])
  if (r.code !== 0) fail('age-keygen -y', r)
  const recipient =
    new TextDecoder().decode(r.stdout).trim().split('\n')[0] ?? ''
  if (!isRecipient(recipient))
    throw new AgeError('age-keygen -y printed no recipient')
  return recipient
}

/**
 * A one-lease identity, held in memory by the process that asked for the
 * lease: the vault encrypts the lease's values to its recipient, so the
 * response body is ciphertext, and the identity dies with the process.
 */
export async function ephemeralIdentity(
  env = process.env,
): Promise<{ identity: Uint8Array; recipient: string }> {
  const { keygen } = ageBinaries(env)
  const r = await run([keygen])
  if (r.code !== 0) fail('age-keygen', r)
  const text = new TextDecoder().decode(r.stdout)
  const recipient = /public key: (age1[0-9a-z]{58})/.exec(text)?.[1]
  if (!recipient) throw new AgeError('age-keygen printed no public key')
  return { identity: r.stdout, recipient }
}

/** Encrypt to every recipient given. Binary output. */
export async function encrypt(
  plaintext: Uint8Array,
  recipients: readonly string[],
  env = process.env,
): Promise<Uint8Array> {
  if (recipients.length === 0) throw new AgeError('no recipient to encrypt to')
  for (const r of recipients)
    if (!isRecipient(r))
      throw new AgeError('a recipient is not an age X25519 recipient')
  const { age } = ageBinaries(env)
  const argv = [age, '--encrypt']
  for (const r of recipients) argv.push('-r', r)
  const r = await run(argv, plaintext)
  if (r.code !== 0) fail('age --encrypt', r)
  return r.stdout
}

/** Decrypt with the identity file at `identityPath`. */
export async function decryptWithFile(
  ciphertext: Uint8Array,
  identityPath: string,
  env = process.env,
): Promise<Uint8Array> {
  const { age } = ageBinaries(env)
  const r = await run([age, '--decrypt', '-i', identityPath], ciphertext)
  if (r.code !== 0) fail('age --decrypt', r)
  return r.stdout
}

/**
 * Whether a ciphertext opens with the identity file, without the plaintext
 * ever entering this process: age writes it to /dev/null.
 */
export async function opensWith(
  ciphertext: Uint8Array,
  identityPath: string,
  env = process.env,
): Promise<boolean> {
  const { age } = ageBinaries(env)
  const r = await run(
    [age, '--decrypt', '-i', identityPath, '-o', '/dev/null'],
    ciphertext,
  )
  return r.code === 0
}

/** age --decrypt with the identity on stdin and the ciphertext in a private temporary file. */
async function decryptStdinIdentity(
  ciphertext: Uint8Array,
  identity: Uint8Array,
  out: string[],
  env: Record<string, string | undefined>,
): Promise<Ran> {
  const { age } = ageBinaries(env)
  const dir = await mkdtemp(join(tmpdir(), 'trios-vault-'))
  try {
    const file = join(dir, 'in.age')
    await writeFile(file, ciphertext, { mode: 0o600 })
    return await run([age, '--decrypt', '-i', '-', ...out, file], identity)
  } finally {
    await rm(dir, { recursive: true, force: true })
  }
}

/**
 * Decrypt with an identity held in memory. age reads the identity on stdin
 * (`-i -`), so the ciphertext goes through a file of its own in a fresh
 * private directory, removed at once. Ciphertext on disk reveals nothing.
 */
export async function decryptWithIdentity(
  ciphertext: Uint8Array,
  identity: Uint8Array,
  env: Record<string, string | undefined> = process.env,
): Promise<Uint8Array> {
  const r = await decryptStdinIdentity(ciphertext, identity, [], env)
  if (r.code !== 0) fail('age --decrypt', r)
  return r.stdout
}

/**
 * The vault's own identity: a file (the owner's local vault, `init`), or the
 * text of the platform's variable held in memory (the server).
 */
export type VaultIdentity = { file: string } | { text: string }

export async function decryptWith(
  ciphertext: Uint8Array,
  identity: VaultIdentity,
  env: Record<string, string | undefined> = process.env,
): Promise<Uint8Array> {
  if ('file' in identity)
    return decryptWithFile(ciphertext, identity.file, env as NodeJS.ProcessEnv)
  return decryptWithIdentity(
    ciphertext,
    new TextEncoder().encode(identity.text),
    env,
  )
}

/** Whether a ciphertext opens with the identity; the plaintext goes to /dev/null. */
export async function opensWithIdentity(
  ciphertext: Uint8Array,
  identity: VaultIdentity,
  env: Record<string, string | undefined> = process.env,
): Promise<boolean> {
  if ('file' in identity)
    return opensWith(ciphertext, identity.file, env as NodeJS.ProcessEnv)
  const r = await decryptStdinIdentity(
    ciphertext,
    new TextEncoder().encode(identity.text),
    ['-o', '/dev/null'],
    env,
  )
  return r.code === 0
}

/** Whether `age` and `age-keygen` can be started (policy.t27 START_NO_TOOL). */
export async function ageAvailable(
  env: Record<string, string | undefined> = process.env,
): Promise<boolean> {
  const { age, keygen } = ageBinaries(env as NodeJS.ProcessEnv)
  try {
    const [a, k] = await Promise.all([
      run([age, '--version']),
      run([keygen, '--version']),
    ])
    return a.code === 0 && k.code === 0
  } catch {
    return false
  }
}

/**
 * The recipient of an identity held in memory (`age-keygen -y` reading
 * stdin), or null when the text is not an age identity. The text is never
 * part of an error.
 */
export async function recipientOfText(
  text: string,
  env: Record<string, string | undefined> = process.env,
): Promise<string | null> {
  const { keygen } = ageBinaries(env as NodeJS.ProcessEnv)
  const r = await run([keygen, '-y'], new TextEncoder().encode(text.trim()))
  const recipient =
    new TextDecoder().decode(r.stdout).trim().split('\n')[0] ?? ''
  return r.code === 0 && isRecipient(recipient) ? recipient : null
}

/**
 * The number of recipient stanzas in an age header (`-> X25519 ...` lines
 * before the `---` line). Reading the header is format, not cryptography:
 * it says how many recipients a file was encrypted to, never who they are.
 */
export function stanzaCount(ciphertext: Uint8Array): number {
  const head = new TextDecoder('latin1').decode(ciphertext.subarray(0, 4096))
  const lines = head.split('\n')
  if (lines[0] !== 'age-encryption.org/v1') return 0
  let n = 0
  for (const line of lines.slice(1)) {
    if (line.startsWith('---')) return n
    if (line.startsWith('-> X25519 ')) n++
    else if (line.startsWith('-> ')) return 0
  }
  return 0
}
