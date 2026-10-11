/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's store (gHashTag/trios#1759): one JSON file on a volume,
 * TRIOS_VAULT_DIR/store.json, mode 0600. It holds ciphertext rows and the
 * names around them -- grants, bindings, aliases, leases -- and never a
 * value: every `ct` is an age file encrypted to the vault's recipient and the
 * owner's recovery recipient (vault-age.ts), so the file alone moves to a new
 * machine and opens there with the recovery identity.
 *
 * The vault's own identity is a separate file, TRIOS_VAULT_DIR/identity
 * (age-keygen's, mode 0600), read by age and never by this process. It is not
 * in the store and never in an export.
 *
 * WHY A FILE AND NOT POSTGRES: the store must survive the Railway account it
 * is replacing (gHashTag/t27#8675). A file on the owner's own volume needs no
 * database to open. The audit goes to the events bus when there is one
 * (vault-audit.ts), and an act it cannot record is refused (policy.t27
 * deliver_code), so the vault never acts unrecorded.
 *
 * Writes replace the file whole (a temporary file, fsync, rename), so a
 * crash leaves the old store or the new one, never half of either. One
 * process owns a store: the vault serializes its own writes (vault-service.ts)
 * and does not lock against a second process on the same directory.
 *
 * THE SERVER'S STORE is Postgres (createPgStore): the same state in one row of
 * queen_vault_store (pg-migrate.ts QUEEN_VAULT_SQL), written whole in one
 * statement, and only over the version it was read at. A second process that
 * wrote in between makes the write fail (StoreConflict); the vault then reads
 * the store again (vault-service.ts handleNow). The rows inside are the same
 * age files, so a dump of that table and the owner's recovery identity are
 * enough to open every value without this code.
 */

import { open, readFile, rename } from 'node:fs/promises'
import { join } from 'node:path'
import type { Pool } from 'pg'

export interface Provenance {
  /** merge.t27 OWNER_SOURCE for a put, else the import's --source label. */
  source: string
  /** The service a railway-kv entry came under; null otherwise. */
  service: string | null
  /** The name the value had there. */
  name: string
}

export interface SecretRow {
  /** policy.t27 CLASS_* */
  class: number
  /** The age file, base64. */
  ct: string
  /** Unix seconds. */
  createdAt: number
  rotatedAt: number
  /** A break-glass lease touched it: due for rotation at once. */
  brokenGlass: boolean
  provenance: Provenance[]
}

export interface Binding {
  scope: string
  /** The environment variable name the value becomes. */
  env: string
  /** A secret id, or an alias of one. */
  id: string
}

export interface Grant {
  /** host.t27 TIER_* */
  tier: number
  scopes: string[]
  grantedAt: number
}

export interface Lease {
  id: string
  /** The holder's public key, hex. */
  key: string
  /** The scope leased; empty for a break-glass lease. */
  scope: string
  /** The secret ids (aliases resolved) whose values the lease delivered. */
  ids: string[]
  issuedAt: number
  expiresAt: number
  renewals: number
  revoked: boolean
  /** A value it holds changed since it was issued (policy.t27 section 6). */
  rotated: boolean
  breakGlass: boolean
}

export interface VaultState {
  version: 1
  vaultRecipient: string
  recoveryRecipient: string
  secrets: Record<string, SecretRow>
  /** alias id -> secret id */
  aliases: Record<string, string>
  bindings: Binding[]
  /** public key hex -> grant; the owner's keys are grants of TIER_OWNER */
  grants: Record<string, Grant>
  leases: Record<string, Lease>
  /** secret id -> unix seconds of its last rotation alarm */
  alarms: Record<string, number>
  /** Unix seconds of the last sweep; 0 before the first. */
  sweptAt: number
}

export interface VaultStore {
  load(): Promise<VaultState | null>
  save(state: VaultState): Promise<void>
}

export const STORE_FILE = 'store.json'
export const IDENTITY_FILE = 'identity'

export function emptyState(
  vaultRecipient: string,
  recoveryRecipient: string,
): VaultState {
  return {
    version: 1,
    vaultRecipient,
    recoveryRecipient,
    secrets: {},
    aliases: {},
    bindings: [],
    grants: {},
    leases: {},
    alarms: {},
    sweptAt: 0,
  }
}

export function createFileStore(dir: string): VaultStore {
  const path = join(dir, STORE_FILE)
  return {
    async load() {
      let text: string
      try {
        text = await readFile(path, 'utf8')
      } catch (error) {
        if ((error as { code?: string }).code === 'ENOENT') return null
        throw error
      }
      const state = JSON.parse(text) as VaultState
      if (state.version !== 1)
        throw new Error(`${STORE_FILE} has version ${state.version}`)
      return state
    },
    async save(state) {
      const tmp = `${path}.${process.pid}.tmp`
      const fh = await open(tmp, 'w', 0o600)
      try {
        await fh.writeFile(`${JSON.stringify(state, null, 1)}\n`)
        await fh.sync()
      } finally {
        await fh.close()
      }
      await rename(tmp, path)
    },
  }
}

/** In memory, for tests and benches: a deep copy each way, as the file would give. */
export function createMemoryStore(): VaultStore & { text(): string } {
  let saved: string | null = null
  return {
    async load() {
      return saved === null ? null : (JSON.parse(saved) as VaultState)
    },
    async save(state) {
      saved = JSON.stringify(state)
    },
    text: () => saved ?? '',
  }
}

export class StoreConflict extends Error {}

/** The server's store: queen_vault_store, one row, optimistic on its version. */
export function createPgStore(pool: Pool): VaultStore {
  let version: number | null = null
  return {
    async load() {
      const r = await pool.query(
        'SELECT state, version FROM queen_vault_store WHERE id = 1',
      )
      const row = r.rows[0] as
        | { state: VaultState; version: string | number }
        | undefined
      if (!row) {
        version = null
        return null
      }
      version = Number(row.version)
      if (row.state.version !== 1)
        throw new Error(`queen_vault_store has version ${row.state.version}`)
      return row.state
    },
    async save(state) {
      const json = JSON.stringify(state)
      const r =
        version === null
          ? await pool.query(
              'INSERT INTO queen_vault_store (id, state, version) VALUES (1, $1::jsonb, 1) ON CONFLICT (id) DO NOTHING',
              [json],
            )
          : await pool.query(
              'UPDATE queen_vault_store SET state = $1::jsonb, version = version + 1, updated_at = now() WHERE id = 1 AND version = $2',
              [json, version],
            )
      if (r.rowCount !== 1)
        throw new StoreConflict(
          'the vault store was written by another process',
        )
      version = version === null ? 1 : version + 1
    },
  }
}
