/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Starting the vault inside the Queen's server (gHashTag/trios#1759,
 * specs/vault/policy.t27 section 13). Whether it starts is start_code; this
 * file gathers the facts, in the spec's order:
 *   1. TRIOS_VAULT unset or not `on`: nothing else is read -- no variable, no
 *      database, no table. The routes answer 503 and the server is as before.
 *   2. The variables, before the database: TRIOS_VAULT_IDENTITY (the vault's
 *      age identity, the one secret it takes from the platform),
 *      TRIOS_VAULT_RECOVERY (the owner's recovery recipient) and
 *      TRIOS_VAULT_OWNER_KEYS (the owner's Ed25519 public keys). A missing or
 *      malformed one refuses the start before any table is created.
 *   3. The database: the vault's table (pg-migrate.ts QUEEN_VAULT_SQL), the
 *      store in it, and whether that store was written under this identity
 *      and this recovery recipient.
 * A refusal is one log line naming the start word and the variable, never a
 * value. The identity is read once and DELETED from the environment it came
 * from, so no process this server starts afterwards inherits it.
 */

import type { Pool } from 'pg'
import { QUEEN_VAULT_SQL, vaultFlagOn } from '../../lib/db/pg-migrate'
import { createQueenPool } from '../../lib/db/queen-pool'
import { logger } from '../../lib/logger'
import { EV_GRANT } from './queen-vault-audit-card.gen'
import {
  IDENTITY_VAR,
  OWNER_KEYS_VAR,
  RECOVERY_VAR,
  START_BAD_IDENTITY,
  START_NAMES,
  START_NO_DATABASE,
  START_NO_IDENTITY,
  START_NO_OWNER,
  START_NO_RECOVERY,
  START_NO_TOOL,
  START_OFF,
  START_ON,
  TIER_OWNER,
} from './queen-vault-policy-card.gen'
import { ageAvailable, isRecipient, recipientOfText } from './vault-age'
import { type AuditSink, busAuditSink } from './vault-audit'
import { serves, startCode } from './vault-cards'
import { Vault } from './vault-service'
import { createPgStore, emptyState, type VaultStore } from './vault-store'
import { isPublicHex, keyIdOf } from './vault-wire'

export interface VaultStart {
  code: number
  vault: Vault | null
}

const OFF: VaultStart = { code: START_OFF, vault: null }
let current: VaultStart = OFF

/** The vault the routes serve, and whether they serve at all (policy.t27 serves). */
export function currentVault(): Vault | null {
  return serves(current.code) ? current.vault : null
}

export function currentStart(): number {
  return current.code
}

/** The variable each refusal is about: what the log line names. */
const ABOUT: Record<number, string> = {
  [START_NO_DATABASE]: 'DATABASE_URL',
  [START_NO_TOOL]: 'the age and age-keygen binaries on PATH',
  [START_NO_IDENTITY]: IDENTITY_VAR,
  [START_BAD_IDENTITY]: IDENTITY_VAR,
  [START_NO_RECOVERY]: RECOVERY_VAR,
  [START_NO_OWNER]: OWNER_KEYS_VAR,
}

export interface StartDeps {
  env?: Record<string, string | undefined>
  /** The database; createQueenPool(DATABASE_URL) when not given. */
  pool?: Pool
  /** The store and audit over that pool; Postgres when not given. */
  store?: VaultStore
  audit?: AuditSink
  now?: () => number
  /** Where age is looked for (vault-age.ts ageBinaries); this process's environment when not given. */
  ageEnv?: Record<string, string | undefined>
}

function refuse(code: number): VaultStart {
  const about = ABOUT[code]
  logger.error(
    `vault refused to start: ${START_NAMES[code] ?? code}${about ? ` (check ${about})` : ''}`,
  )
  current = { code, vault: null }
  return current
}

/**
 * Start the vault, or decide it stays off. Never throws: a database that
 * cannot be read is START_NO_DATABASE. Safe to call again (a test, a restart).
 */
export async function startVault(deps: StartDeps = {}): Promise<VaultStart> {
  const env = deps.env ?? process.env
  const flagOn = vaultFlagOn(env)
  if (!flagOn) {
    current = OFF
    return current
  }
  const identity = (env[IDENTITY_VAR] ?? '').trim()
  delete env[IDENTITY_VAR]
  const tool = await ageAvailable(deps.ageEnv)
  const recipient =
    identity === '' || !tool
      ? null
      : await recipientOfText(identity, deps.ageEnv)
  const recovery = (env[RECOVERY_VAR] ?? '').trim()
  const owners = (env[OWNER_KEYS_VAR] ?? '')
    .split(/[\s,]+/)
    .filter((k) => k !== '')
  const url = env.DATABASE_URL?.trim()
  const before = startCode({
    flagOn,
    database:
      Boolean(url) || deps.pool !== undefined || deps.store !== undefined,
    tool,
    identitySet: identity !== '',
    identityOk: recipient !== null,
    recoveryOk: isRecipient(recovery),
    ownersOk: owners.length > 0 && owners.every((k) => isPublicHex(k)),
    sameIdentity: true,
    sameRecovery: true,
  })
  if (before !== START_ON) return refuse(before)

  try {
    const pool =
      deps.store && deps.audit
        ? null
        : (deps.pool ?? createQueenPool(url as string))
    if (pool) await pool.query(QUEEN_VAULT_SQL)
    const store = deps.store ?? createPgStore(pool as Pool)
    const audit = deps.audit ?? busAuditSink(pool as Pool)
    const now = deps.now ?? (() => Math.floor(Date.now() / 1000))
    let state = await store.load()
    const after = startCode({
      flagOn,
      database: true,
      tool: true,
      identitySet: true,
      identityOk: true,
      recoveryOk: true,
      ownersOk: true,
      sameIdentity: state === null || state.vaultRecipient === recipient,
      sameRecovery: state === null || state.recoveryRecipient === recovery,
    })
    if (after !== START_ON) return refuse(after)
    let changed = state === null
    state ??= emptyState(recipient as string, recovery)
    for (const key of owners) {
      if (state.grants[key]?.tier === TIER_OWNER) continue
      const id = keyIdOf(key)
      await audit.append(EV_GRANT, { key: id, tier: TIER_OWNER, subject: id })
      state.grants[key] = {
        tier: TIER_OWNER,
        scopes: state.grants[key]?.scopes ?? [],
        grantedAt: now(),
      }
      changed = true
    }
    if (changed) await store.save(state)
    const vault = new Vault({
      identity: { text: identity },
      store,
      audit,
      now,
      env: deps.ageEnv,
    })
    current = { code: START_ON, vault }
    logger.info(
      `vault on: ${Object.keys(state.secrets).length} secrets, ${owners.length} owner keys`,
    )
    return current
  } catch (error) {
    logger.error(
      'vault refused to start: no-database (the store could not be read or written)',
      {
        error: error instanceof Error ? error.name : 'unknown',
      },
    )
    current = { code: START_NO_DATABASE, vault: null }
    return current
  }
}

/** For tests: forget the started vault. */
export function stopVault(): void {
  current = OFF
}
