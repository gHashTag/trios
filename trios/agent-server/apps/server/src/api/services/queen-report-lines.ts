/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The sentences the Queen's report says about dispatch, and the count they
 * hang on.
 *
 * THE COUNT IS DERIVED HERE, AT THE POINT EACH SENTENCE IS BUILT, from the
 * `started` boolean on each dispatch outcome. It is not `outcomes.length` -
 * the array the dispatch loop fills also carries refusals, and counting those
 * as workers is how a round that started nothing came to report "1 bee(s)
 * working" and name the refused issue as started (trios#1379). It is also not
 * a second counter kept alongside the array: two places counting workers is
 * how they come to disagree.
 *
 * NO IMPORTS, DELIBERATELY. A bee's worktree is a fresh `git worktree add`
 * with no `node_modules`, so a module with a single import - `pg`, a logger,
 * even a type from a neighbouring file - is a module its own test cannot
 * load there. Everything here is structural types and string building.
 */

/** What the report needs from a dispatch outcome. Structural, so the richer
 * `DispatchOutcome` in queen-dispatch.ts satisfies it without an import. */
export interface DispatchReportOutcome {
  started?: boolean
  issue?: number
  detail?: string
  /** Set only when the CONTAINER refused the start: which resource had no
   * room, and the guard's own one-sentence account of it. */
  room?: { resource: 'memory' | 'disk'; summary?: string }
}

/**
 * The outcomes that actually started a bee. Refusals carry `started: false`
 * and stay in the array - the report needs to name them, just not count them
 * as workers.
 */
export function dispatchesThatStarted(
  outcomes: ReadonlyArray<DispatchReportOutcome>,
): DispatchReportOutcome[] {
  return outcomes.filter((outcome) => outcome.started === true)
}

/**
 * The sentence naming the bees this round started: "Started 2 bee(s): #11, #12."
 * Empty string when none started, so the caller pushes nothing rather than a
 * sentence about an empty swarm. The wording is load-bearing: unchanged from
 * what it has always been for the rounds that did start bees.
 */
export function startedLine(
  outcomes: ReadonlyArray<DispatchReportOutcome>,
): string {
  const going = dispatchesThatStarted(outcomes)
  if (going.length === 0) return ''
  return (
    `Started ${going.length} bee(s): ` +
    going.map((outcome) => `#${outcome.issue}`).join(', ') +
    '.'
  )
}

/**
 * The sentence for a round that started nothing. A round that started nothing
 * is the case where a summary in anyone's own words would be the least
 * trustworthy thing on the page, so the refusal is quoted verbatim. The
 * wording is unchanged: tests outside this task assert it byte for byte.
 */
export function nothingStartedLine(
  // `null` as well as `undefined`: `QueendChoice.refusal` is `string | null`,
  // and both spellings mean the same thing to the `??` below. Narrowing this
  // to `undefined` made two call sites in queen-tick.ts a type error that no
  // gate in CI was running to catch.
  refusal: string | null | undefined,
  candidates: number,
): string {
  return (
    `Started nothing. ${refusal ?? 'No reason given'}. ` +
    `${candidates} issue(s) were on the table.`
  )
}

/** How long a refusal detail is shown before it is cut. The stray line caps
 * its list at 8 entries; prose in this codebase is capped at 200 characters
 * (queen-dispatch.ts truncates every detail it builds that way), so 200 is
 * the cap the details themselves already obey. */
const DETAIL_SHOWN = 200

/**
 * A line naming each dispatch that was refused and why, so the operator
 * learns the cause and not only the absence. Without this, a refusal was
 * invisible twice over: not counted as started, and the fact of it suppressed
 * the one sentence that would have explained it.
 *
 * Truncated the way the stray line truncates: shown up to the cap, an
 * ellipsis when there was more, and the sentence left closed - with its own
 * punctuation when the detail carries it, a period added when it does not.
 */
export function refusedLines(
  outcomes: ReadonlyArray<DispatchReportOutcome>,
): string[] {
  return outcomes
    .filter((outcome) => outcome.started !== true)
    .map((outcome) => {
      const detail = String(outcome.detail ?? '')
      const shown =
        detail.slice(0, DETAIL_SHOWN) +
        (detail.length > DETAIL_SHOWN ? '...' : '')
      // Details are sentences and mostly end in their own period; the stray
      // line's closing '.' is added only when the sentence is not already
      // closed, so an operator never reads "swarm..".
      const closed = /[.!?]$/.test(shown) ? shown : `${shown}.`
      return `Refused #${outcome.issue}: ${closed}`
    })
}

/**
 * Why a round that `queend` allowed still started nothing, when the reason is
 * the container.
 *
 * `queend` refuses for the policy's reasons - the worker limit, the budget,
 * nothing to choose - and those reach the headline. A refusal made by the
 * container guard in DISPATCH did not: `queend` had said yes, so the report
 * read "Started nothing. No reason given." under the headline "nothing to do",
 * while the actual sentence sat one line lower where a headline reader never
 * looks.
 *
 * TWO STRINGS, BECAUSE THEY GO TO TWO PLACES. The headline is served to a
 * browser by `/queen/needs-you`, whose contract is no path, no branch and no
 * connection detail - so it gets a label from a closed vocabulary and never
 * the sentence, which names the workspace path. The body is the operator's,
 * and it gets the guard's own SUMMARY: one sentence that carries every number
 * and is written to fit (see `judgeBeeRoom`). Only outcomes the guard marked
 * with `room` are read, so git output and provider bodies - which are also
 * details of dispatches that did not start - can reach neither string.
 *
 * The summary is a FIELD and is never cut out of `detail`: the prose names a
 * path, and a path may hold ". " - "/Volumes/Ext. Drive" ended the sentence
 * after "Ext" while this function still split on it.
 *
 * It comes without a closing period because `nothingStartedLine` adds exactly
 * one. One that is too long anyway is cut at a word and closed with two dots,
 * which that period makes the ellipsis this file uses.
 */
export function containerRefusal(
  outcomes: ReadonlyArray<DispatchReportOutcome>,
): { headline: string; sentence: string } | null {
  const refused = outcomes.find(
    (outcome) => outcome.started !== true && outcome.room !== undefined,
  )
  if (!refused?.room) return null
  const headline = `no room in the container: ${refused.room.resource}`
  const summary = String(refused.room.summary ?? '')
    .trim()
    .replace(/[.!?\s]+$/, '')
  const sentence =
    summary.length <= DETAIL_SHOWN
      ? summary
      : `${summary.slice(0, DETAIL_SHOWN - 3).replace(/\s+\S*$/, '')}..`
  return { headline, sentence: sentence || headline }
}

/**
 * The headline: escalations first, because a decision waiting on a human
 * outranks everything else; then the bees actually working, counted from the
 * `started` booleans; and only when neither applies, the refusal - which is
 * the honest headline for a round that started nothing.
 */
export function reportHeadline(
  escalatedCount: number,
  outcomes: ReadonlyArray<DispatchReportOutcome>,
  refusal: string | null | undefined,
): string {
  if (escalatedCount > 0) return `${escalatedCount} waiting on you`
  const working = dispatchesThatStarted(outcomes).length
  if (working > 0) return `${working} bee(s) working`
  return refusal ?? 'nothing to do'
}
