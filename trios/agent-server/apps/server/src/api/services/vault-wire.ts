/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * What a vault client and the vault both speak (gHashTag/trios#1759, design
 * rule 5): the Ed25519 keys and the signed request. Plumbing only; whether a
 * signed request is admitted is policy.t27 request_code (vault-cards.ts).
 *
 * THE KEY: Ed25519 (RFC 8032) through node:crypto, in the layout of the
 * self-hosting lane (gHashTag/trios#1756), so a host key there is a workload
 * key here: the private half as PKCS#8 PEM in a file of mode 0600, the public
 * half as 64 lowercase hex characters (its 32 raw bytes), and the key id as
 * KEY_ID_HEX_LEN hex characters of the SHA-256 of those bytes
 * (verified/signed_receipt.t27).
 *
 * THE MESSAGE: REQUEST_DOMAIN, then one `name=value` line per
 * REQUEST_SIGNED_FIELDS, value being the field's JSON text, every line ending
 * in a newline (signed_receipt.t27's layout). It travels as four headers.
 */

import {
  createHash,
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  type KeyObject,
  randomBytes,
  sign,
  verify,
} from 'node:crypto'
import { HASH_HEX_LEN } from './queen-vault-audit-card.gen'
import {
  KEY_ID_HEX_LEN,
  NONCE_MIN_BYTES,
  REQUEST_DOMAIN,
  REQUEST_SIGNED_FIELDS,
} from './queen-vault-policy-card.gen'

export const HEADER_KEY = 'x-vault-key'
export const HEADER_TIME = 'x-vault-time'
export const HEADER_NONCE = 'x-vault-nonce'
export const HEADER_SIGNATURE = 'x-vault-signature'

export const sha256Hex = (data: string | Uint8Array): string =>
  createHash('sha256').update(data).digest('hex')

/** The first HASH_HEX_LEN characters of a SHA-256: an event's `ct`, `req`, `reason_sha`. */
export const shortHash = (data: string | Uint8Array): string =>
  sha256Hex(data).slice(0, HASH_HEX_LEN)

// --- keys -------------------------------------------------------------------

const SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex')
const HEX64 = /^[0-9a-f]{64}$/

export const isPublicHex = (text: unknown): text is string =>
  typeof text === 'string' && HEX64.test(text)

export function generateKey(): { privatePem: string; publicHex: string } {
  const { privateKey } = generateKeyPairSync('ed25519')
  const privatePem = privateKey
    .export({ format: 'pem', type: 'pkcs8' })
    .toString()
  return { privatePem, publicHex: publicHexOf(privatePem) }
}

export function publicHexOf(privatePem: string): string {
  const der = createPublicKey(createPrivateKey(privatePem)).export({
    format: 'der',
    type: 'spki',
  })
  return Buffer.from(der).subarray(SPKI_PREFIX.length).toString('hex')
}

function publicKeyOf(publicHex: string): KeyObject | null {
  if (!HEX64.test(publicHex)) return null
  try {
    return createPublicKey({
      key: Buffer.concat([SPKI_PREFIX, Buffer.from(publicHex, 'hex')]),
      format: 'der',
      type: 'spki',
    })
  } catch {
    return null
  }
}

export const keyIdOf = (publicHex: string): string =>
  sha256Hex(Buffer.from(publicHex, 'hex')).slice(0, KEY_ID_HEX_LEN)

// --- requests ---------------------------------------------------------------

export interface RequestFacts {
  method: string
  path: string
  key: string
  utc_unix: number
  nonce: string
  body_sha256: string
}

export function messageOf(facts: RequestFacts): string {
  let text = `${REQUEST_DOMAIN}\n`
  for (const name of REQUEST_SIGNED_FIELDS) {
    const value = facts[name as keyof RequestFacts]
    if (value === undefined) throw new Error(`no ${name} to sign`)
    text += `${name}=${JSON.stringify(value)}\n`
  }
  return text
}

/** The four headers of a signed request for `body` (the exact bytes sent). */
export function signRequest(
  privatePem: string,
  method: string,
  path: string,
  body: string,
  nowSeconds: number,
): Record<string, string> {
  const key = publicHexOf(privatePem)
  const facts: RequestFacts = {
    method: method.toUpperCase(),
    path,
    key,
    utc_unix: Math.floor(nowSeconds),
    nonce: randomBytes(NONCE_MIN_BYTES).toString('hex'),
    body_sha256: sha256Hex(body),
  }
  const signature = sign(
    null,
    Buffer.from(messageOf(facts)),
    createPrivateKey(privatePem),
  ).toString('hex')
  return {
    [HEADER_KEY]: key,
    [HEADER_TIME]: String(facts.utc_unix),
    [HEADER_NONCE]: facts.nonce,
    [HEADER_SIGNATURE]: signature,
  }
}

export interface SignedHeaders {
  key: string
  utcUnix: number
  nonce: string
  signature: string
}

/** The four headers, or null when one is missing or malformed. */
export function readSignedHeaders(
  get: (name: string) => string | undefined | null,
): SignedHeaders | null {
  const key = get(HEADER_KEY) ?? ''
  const time = get(HEADER_TIME) ?? ''
  const nonce = get(HEADER_NONCE) ?? ''
  const signature = get(HEADER_SIGNATURE) ?? ''
  if (!HEX64.test(key)) return null
  if (!/^\d{1,12}$/.test(time)) return null
  if (!/^[0-9a-f]{2,128}$/.test(nonce) || nonce.length % 2 !== 0) return null
  if (!/^[0-9a-f]{128}$/.test(signature)) return null
  return { key, utcUnix: Number(time), nonce, signature }
}

export function verifyRequest(
  headers: SignedHeaders,
  method: string,
  path: string,
  body: string,
): boolean {
  const key = publicKeyOf(headers.key)
  if (!key) return false
  const message = messageOf({
    method: method.toUpperCase(),
    path,
    key: headers.key,
    utc_unix: headers.utcUnix,
    nonce: headers.nonce,
    body_sha256: sha256Hex(body),
  })
  try {
    return verify(
      null,
      Buffer.from(message),
      key,
      Buffer.from(headers.signature, 'hex'),
    )
  } catch {
    return false
  }
}
