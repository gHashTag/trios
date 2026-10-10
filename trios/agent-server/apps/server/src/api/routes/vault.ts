/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The vault's door (gHashTag/trios#1759): POST /vault/<operation>, one per
 * policy.t27 OP_NAMES_TEXT. Every request carries an Ed25519 signature over
 * its method, path, time, nonce and body (vault-wire.ts), judged by
 * policy.t27 request_code before anything else; there is no bearer and no
 * cookie, and no browser Origin to trust, so it is not behind the
 * trusted-origin guard (tools/route-guard-audit.mjs gives the reason).
 *
 * OFF MEANS OFF (policy.t27 section 13): unless the vault started at boot
 * (vault-start.ts: TRIOS_VAULT=on, its three variables, a database), every
 * request answers 503 and nothing else runs. The 503 names no reason; the
 * boot log does, by the start word and the variable, never a value.
 *
 * No response carries a value: a lease answers ciphertext to the requester's
 * one-lease recipient, an export ciphertext to the recovery recipient.
 */

import { Hono } from 'hono'
import { logger } from '../../lib/logger'
import type { Vault } from '../services/vault-service'
import { currentVault } from '../services/vault-start'

/** The routes over one vault; the CLI's local mode mounts the same ones. */
export function vaultRoutes(getVault: () => Vault | null): Hono {
  return new Hono().all('/*', async (c) => {
    const vault = getVault()
    if (!vault)
      return c.json({ ok: false, error: 'the vault is unavailable' }, 503)
    if (c.req.method !== 'POST')
      return c.json({ ok: false, error: 'POST only' }, 405)
    const body = await c.req.text()
    try {
      const r = await vault.handle({
        method: c.req.method,
        path: c.req.path,
        header: (name) => c.req.header(name),
        body,
      })
      return c.json(r.body, r.status as 200, { 'Cache-Control': 'no-store' })
    } catch (error) {
      logger.warn('vault request failed', {
        error: error instanceof Error ? error.name : 'unknown',
      })
      return c.json({ ok: false, error: 'the vault could not answer' }, 500)
    }
  })
}

/** The server's mount: the vault vault-start.ts started at boot, or 503. */
export function createVaultRoute(): Hono {
  return vaultRoutes(currentVault)
}
