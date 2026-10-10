/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The bounded drain wired to this process (queen-drain.ts, drain.t27): its
 * runner's bees, its reviews, its rounds. Kept apart from queen-drain.ts so
 * that the loop imports neither the tick nor the runner, and the tick can ask
 * the flag without an import cycle.
 */

import { logger } from '../../lib/logger'
import {
  drainBounded,
  drainCapSeconds,
  platformDrainSeconds,
} from './queen-drain'
import {
  activeRunner,
  beesInFlight,
  drainSeconds,
  handBackBee,
} from './queen-runner'
import { reviewsInFlight, stopQueenTickNow } from './queen-tick'

/** For the server's shutdown on SIGTERM, with TRIOS_QUEEN_DRAIN=bounded. */
export async function drainForDeploy(
  env: Record<string, string | undefined> = process.env,
): Promise<void> {
  const runner = activeRunner()
  const capSeconds = drainCapSeconds(
    runner ? drainSeconds(env) : 0,
    platformDrainSeconds(env),
  )
  const result = await drainBounded({
    now: Date.now,
    sleep: (ms) => new Promise<void>((resolve) => setTimeout(resolve, ms)),
    capSeconds,
    roundsStartBeesHere: env.TRIOS_QUEEN_BEES_RUN_ELSEWHERE !== 'on',
    stopClaims: () => runner?.stop(),
    bees: () => (runner ? beesInFlight() : []),
    handBack: async (bee) => {
      const order = beesInFlight().find((o) => o.issue === bee.issue)
      if (runner && order) await handBackBee(runner.pool, runner.runner, order)
    },
    reviews: reviewsInFlight,
    stopRest: stopQueenTickNow,
  })
  logger.info('Queen drained (drain.t27)', {
    capSeconds,
    tookSeconds: Math.round(result.tookMs / 1000),
    heldAtStart: result.heldAtStart,
    handedBack: result.handedBack,
    reviewsStopped: result.reviewsStopped,
  })
}
