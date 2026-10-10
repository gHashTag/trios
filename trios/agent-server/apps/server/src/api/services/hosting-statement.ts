/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The hosts' epoch statement (gHashTag/trios#1761, slice 1b): the leaves, the
 * RFC 6962 tree over them, the signed statement, and the check a host runs on
 * its own leaf. Shared by the Queen, which builds and signs it, and the host
 * agent, which verifies it.
 *
 * THIN GLUE. Every rule is gHashTag/t27 specs/hosting/statement.t27, called
 * through hosting-cards.ts: the leaf's kinds and order, the tree's split
 * (corpus_receipt.t27 split_point), each sibling's side, the proof's shape and
 * the order of a host's checks. This file only hashes (SHA-256, as slice 1's
 * wire does) and signs (Ed25519), which a card cannot.
 */

import { createHash } from 'node:crypto'
import {
  leafBefore,
  leafKindKnown,
  proofShapeOk,
  siblingLeft,
  splitPoint,
  verifyCode,
} from './hosting-cards'
import { messageOf, sha256Hex, signHex, verifyHex } from './hosting-wire'
import {
  KIND_WORDS,
  LEAF_DOMAIN,
  LEAF_FIELDS,
  LEAF_PREFIX,
  NODE_PREFIX,
  STATEMENT_DOMAIN,
  STATEMENT_FIELDS,
} from './queen-hosting-statement-card.gen'

export interface Leaf {
  epoch: number
  host: string
  kind: string
  mtri: number
  receipts: string[]
}

/** One money row as the statement reads it: credits and slashes, never strikes. */
export interface MoneyRow {
  hostId: string
  feeKind: number
  mtri: number
  receipt: string | null
}

export const VERIFY_WORDS = [
  'ok',
  'bad_signature',
  'not_my_leaf',
  'bad_shape',
  'root_mismatch',
] as const

// --- the tree (RFC 6962 2.1, corpus_receipt.t27) -------------------------------

const hash = (...parts: Buffer[]): Buffer =>
  createHash('sha256').update(Buffer.concat(parts)).digest()

export const leafHashOf = (text: string): Buffer =>
  hash(Buffer.from([LEAF_PREFIX]), Buffer.from(text))

const nodeOf = (left: Buffer, right: Buffer): Buffer =>
  hash(Buffer.from([NODE_PREFIX]), left, right)

export function rootOf(hashes: readonly Buffer[]): Buffer {
  if (hashes.length === 0) return hash()
  if (hashes.length === 1) return hashes[0] as Buffer
  const k = splitPoint(hashes.length)
  return nodeOf(rootOf(hashes.slice(0, k)), rootOf(hashes.slice(k)))
}

/** The audit path of leaf m, nearest sibling first (RFC 6962 2.1.1). */
export function pathOf(m: number, hashes: readonly Buffer[]): Buffer[] {
  if (hashes.length <= 1) return []
  const k = splitPoint(hashes.length)
  return m < k
    ? [...pathOf(m, hashes.slice(0, k)), rootOf(hashes.slice(k))]
    : [...pathOf(m - k, hashes.slice(k)), rootOf(hashes.slice(0, k))]
}

/** The root a leaf and its path give, each sibling on the side the card names. */
export function rootFromPath(
  leaf: Buffer,
  index: number,
  size: number,
  path: readonly Buffer[],
): Buffer {
  let r = leaf
  path.forEach((p, i) => {
    r = siblingLeft(index, size, i) ? nodeOf(p, r) : nodeOf(r, p)
  })
  return r
}

// --- the leaves and the statement -----------------------------------------------

export const leafText = (leaf: Leaf): string =>
  messageOf(LEAF_DOMAIN, LEAF_FIELDS, { ...leaf })

const kindWord = (kind: number): string => KIND_WORDS[kind] ?? ''
const kindOfWord = (word: string): number =>
  (KIND_WORDS as readonly string[]).indexOf(word)

/** One leaf per (host, kind) with rows, in statement.t27's order. */
export function leavesOf(epoch: number, rows: readonly MoneyRow[]): Leaf[] {
  const by = new Map<string, Leaf>()
  for (const r of rows) {
    if (!leafKindKnown(r.feeKind)) continue
    const key = `${r.hostId}\u0000${r.feeKind}`
    const leaf = by.get(key) ?? {
      epoch,
      host: r.hostId,
      kind: kindWord(r.feeKind),
      mtri: 0,
      receipts: [],
    }
    leaf.mtri += r.mtri
    if (r.receipt && !leaf.receipts.includes(r.receipt))
      leaf.receipts.push(r.receipt)
    by.set(key, leaf)
  }
  const leaves = [...by.values()]
  for (const l of leaves) l.receipts.sort()
  return leaves.sort((a, b) => {
    const order = a.host < b.host ? -1 : a.host > b.host ? 1 : 0
    const ka = kindOfWord(a.kind)
    const kb = kindOfWord(b.kind)
    if (leafBefore(order, ka, kb)) return -1
    if (leafBefore(-order, kb, ka)) return 1
    return 0
  })
}

export const statementText = (fields: Record<string, unknown>): string =>
  messageOf(STATEMENT_DOMAIN, STATEMENT_FIELDS, fields)

/** The fields of a signed statement text, as JSON values. */
export function statementFields(text: string): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const line of text.split('\n').slice(1)) {
    const at = line.indexOf('=')
    if (at > 0) out[line.slice(0, at)] = JSON.parse(line.slice(at + 1))
  }
  return out
}

export const signStatement = (privatePem: string, text: string): string =>
  signHex(privatePem, text)

export const statementHash = (text: string): string => sha256Hex(text)

// --- a host's check of its own leaf ------------------------------------------------

export interface EpochProof {
  statement: string
  signature: string
  tree_size: number
  leaves: Array<{ index: number; leaf: Leaf; path: string[] }>
}

export interface LeafCheck {
  index: number
  kind: string
  mtri: number
  code: number
  word: string
}

/**
 * statement.t27 verify_code, for each of this host's leaves: the statement's
 * signature under the Queen's key, the leaf names this host, the path's
 * shape, and the root the leaf and path give.
 */
export function checkOwnLeaves(
  proof: EpochProof,
  queenKeyHex: string,
  hostId: string,
): LeafCheck[] {
  const signatureOk = verifyHex(queenKeyHex, proof.statement, proof.signature)
  const root = String(statementFields(proof.statement).root ?? '')
  return proof.leaves.map(({ index, leaf, path }) => {
    const shapeOk = proofShapeOk(index, proof.tree_size, path.length)
    const got = shapeOk
      ? rootFromPath(
          leafHashOf(leafText(leaf)),
          index,
          proof.tree_size,
          path.map((p) => Buffer.from(p, 'hex')),
        ).toString('hex')
      : ''
    const code = verifyCode(
      signatureOk,
      leaf.host === hostId,
      shapeOk,
      got === root,
    )
    return {
      index,
      kind: leaf.kind,
      mtri: leaf.mtri,
      code,
      word: VERIFY_WORDS[code] ?? String(code),
    }
  })
}

/** Paths for every leaf of one host, from the statement's stored leaves. */
export function proofOf(
  statement: { message: string; signature: string; leaves: Leaf[] },
  hostId: string,
): EpochProof {
  const hashes = statement.leaves.map((l) => leafHashOf(leafText(l)))
  return {
    statement: statement.message,
    signature: statement.signature,
    tree_size: hashes.length,
    leaves: statement.leaves
      .map((leaf, index) => ({ leaf, index }))
      .filter(({ leaf }) => leaf.host === hostId)
      .map(({ leaf, index }) => ({
        index,
        leaf,
        path: pathOf(index, hashes).map((h) => h.toString('hex')),
      })),
  }
}
