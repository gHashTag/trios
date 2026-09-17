/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * The #1379 regression test: a round whose dispatches were all refused must
 * report that nothing started, not report a bee working.
 *
 * Imports `bun:test`, the module under test and one neighbour that itself
 * imports only Node builtins. The module has no imports by design - a bee's
 * worktree has no `node_modules`, so this test has to run with neither a
 * database nor the `queend` binary to hand.
 */
import { describe, expect, it } from 'bun:test'
import {
  containerRefusal,
  dispatchesThatStarted,
  nothingStartedLine,
  refusedLines,
  reportHeadline,
  startedLine,
} from '../../src/api/services/queen-report-lines'
// Builtins only, like the module above: it loads in a bare worktree too.
import {
  judgeBeeRoom,
  resourceSettings,
} from '../../src/api/services/queen-resources'

/** The refusal `dispatchBee` returns when every provider key is busy - the
 * routine case from the issue, not an exotic failure. */
const KEY_EXHAUSTED_DETAIL =
  'all 1 provider key(s) are already in use by bees in flight. Add another ' +
  'with ZAI_API_KEY_2 (or the equivalent for your provider) to widen the ' +
  'swarm.'

const REFUSAL = 'every provider key is busy; the swarm is a key short'

/** The dispatch lines of a report body, assembled the way `report()` assembles
 * them: the started sentence, then one line per refused dispatch, then the
 * nothing-started sentence when nothing started. */
function dispatchBody(
  outcomes: Parameters<typeof startedLine>[0],
  refusal: string | undefined,
  candidates: number,
): string {
  const lines: string[] = []
  const started = startedLine(outcomes)
  if (started !== '') lines.push(started)
  lines.push(...refusedLines(outcomes))
  if (dispatchesThatStarted(outcomes).length === 0) {
    lines.push(nothingStartedLine(refusal, candidates))
  }
  return lines.join('\n')
}

describe('queen report dispatch lines', () => {
  it('reports a round whose only dispatch was refused as started nothing', () => {
    const outcomes = [
      { started: false, issue: 1234, detail: KEY_EXHAUSTED_DETAIL },
    ]

    // The count is the booleans, not the array length: one refusal is not a
    // worker, however many entries the array holds.
    expect(dispatchesThatStarted(outcomes)).toHaveLength(0)

    const headline = reportHeadline(0, outcomes, REFUSAL)
    expect(headline).not.toContain('bee(s) working')
    expect(headline).toBe(REFUSAL)

    const body = dispatchBody(outcomes, REFUSAL, 3)
    // No sentence claims a bee was started.
    expect(body).not.toContain('Started 1 bee(s)')
    // The refusal is carried verbatim, and the refused issue and its detail
    // are named - the operator reads why, not only that nothing happened.
    expect(body).toContain(REFUSAL)
    expect(body).toContain('#1234')
    expect(body).toContain(KEY_EXHAUSTED_DETAIL)
    expect(body).toContain('3 issue(s) were on the table.')
  })

  it('counts only the dispatch that started in a mixed round', () => {
    const outcomes = [
      { started: true, issue: 11, detail: 'worktree ready; zai/glm-4.6' },
      { started: false, issue: 22, detail: KEY_EXHAUSTED_DETAIL },
    ]

    expect(dispatchesThatStarted(outcomes)).toHaveLength(1)

    // The started sentence is byte-for-byte what it has always been for a
    // round that started one bee, and names only the started issue.
    expect(startedLine(outcomes)).toBe('Started 1 bee(s): #11.')
    expect(startedLine(outcomes)).not.toContain('#22')

    // The refused dispatch is reported separately, with its cause. The
    // detail is a sentence and closes itself; no second period is added.
    expect(refusedLines(outcomes)).toEqual([
      `Refused #22: ${KEY_EXHAUSTED_DETAIL}`,
    ])

    expect(reportHeadline(0, outcomes, REFUSAL)).toBe('1 bee(s) working')
  })

  it('builds an empty round exactly as before', () => {
    expect(dispatchesThatStarted([])).toHaveLength(0)
    expect(startedLine([])).toBe('')
    expect(refusedLines([])).toEqual([])

    // The nothing-started sentence is unchanged, word for word - with a
    // refusal given and with none.
    expect(nothingStartedLine(REFUSAL, 3)).toBe(
      `Started nothing. ${REFUSAL}. 3 issue(s) were on the table.`,
    )
    expect(nothingStartedLine(undefined, 0)).toBe(
      'Started nothing. No reason given. 0 issue(s) were on the table.',
    )

    // An empty round's headline is the refusal, or its default.
    expect(reportHeadline(0, [], REFUSAL)).toBe(REFUSAL)
    expect(reportHeadline(0, [], undefined)).toBe('nothing to do')

    // Escalations still outrank the started count in the headline.
    expect(reportHeadline(2, [{ started: true, issue: 11 }], REFUSAL)).toBe(
      '2 waiting on you',
    )
  })

  it('truncates a long refusal detail the way the stray line truncates', () => {
    const longDetail = 'x'.repeat(400)
    const [line] = refusedLines([
      { started: false, issue: 7, detail: longDetail },
    ])
    // 200 characters shown, '...' marks the cut - and the ellipsis already
    // closes the sentence, so no further period is appended.
    expect(line).toBe(`Refused #7: ${'x'.repeat(200)}...`)

    // A short detail that is not a sentence still gets its closing period.
    const [short] = refusedLines([{ started: false, issue: 8, detail: 'no' }])
    expect(short).toBe('Refused #8: no.')
  })
})

describe('a round queend allowed and the container refused', () => {
  const GB = 1_000_000_000
  const settings = resourceSettings({})
  /** What the guard REALLY returns, at the sizes that make it longest: a
   * hand-shortened sample is how a cut in the middle of a number went unseen. */
  const refusal = (room: ReturnType<typeof judgeBeeRoom>, issue = 1500) => {
    if (room.ok) throw new Error('the fixture was meant to have no room')
    return {
      started: false,
      issue,
      detail: room.detail,
      room: { resource: room.resource, summary: room.summary },
    }
  }
  const memoryFull = () =>
    judgeBeeRoom({
      memory: {
        kind: 'measured',
        usedBytes: 1234.5 * GB,
        limitBytes: 4096 * GB,
        source: '/proc/meminfo',
        limitSource: 'TRIOS_QUEEN_MEMORY_LIMIT_MB',
      },
      volume: { totalBytes: 50 * GB, freeBytes: 19 * GB },
      youngBees: 1000,
      settings: { ...settings, beeMemoryBytes: 64 * GB, warmupSeconds: 3599 },
      volumeDir: '/workspace/BrowserOS',
    })
  const diskShort = (
    volumeDir = '/workspace/BrowserOS',
    reaped = { removed: 1234, keptDirty: 567, keptRunning: 1024 },
  ) =>
    judgeBeeRoom({
      memory: { kind: 'unsupported', platform: 'darwin' },
      volume: { totalBytes: 50 * GB, freeBytes: 5.1 * GB },
      youngBees: 0,
      settings,
      volumeDir,
      reaped,
    })

  it('quotes the whole summary in the line that read "No reason given"', () => {
    for (const room of [memoryFull(), diskShort()]) {
      const said = containerRefusal([refusal(room)])
      // Whole: it ends where the guard's sentence ends, not mid-word.
      expect(said?.sentence.endsWith('GB stays free')).toBe(true)
      expect(said?.sentence.length).toBeLessThanOrEqual(200)
      const line = nothingStartedLine(said?.sentence, 4)
      expect(line).not.toContain('No reason given')
      expect(line).toContain('GB stays free. 4 issue(s) were on the table.')
    }
  })

  it('is not fooled by a path that holds a period and a space', () => {
    // The sentence used to be cut out of the prose at the first ". ", and the
    // prose names the workspace: the report read "volume /Volumes/Ext."
    const said = containerRefusal([
      refusal(
        diskShort('/Volumes/Ext. Drive/ws/BrowserOS', {
          removed: 2,
          keptDirty: 1,
          keptRunning: 14,
        }),
      ),
    ])
    expect(said?.sentence).toContain(
      '/Volumes/Ext. Drive/ws/BrowserOS has 5.1 GB',
    )
    expect(said?.sentence.endsWith('GB stays free')).toBe(true)
  })

  it('gives the headline a label and never the sentence', () => {
    const outcomes = [refusal(diskShort())]
    const headline = reportHeadline(
      0,
      outcomes,
      containerRefusal(outcomes)?.headline,
    )
    expect(headline).toBe('no room in the container: disk')
    // /queen/needs-you serves this string to a browser: no path, no numbers.
    expect(headline).not.toContain('/workspace')
  })

  it('reads no other refusal, because those details carry git and provider output', () => {
    expect(
      containerRefusal([
        {
          started: false,
          issue: 7,
          detail:
            "git worktree add failed: fatal: '/workspace/BrowserOS/.worktrees/queen-7' already exists",
        },
        { started: false, issue: 8, detail: KEY_EXHAUSTED_DETAIL },
      ]),
    ).toBeNull()
    expect(
      containerRefusal([{ started: true, issue: 1, detail: 'fresh worktree' }]),
    ).toBeNull()
    expect(containerRefusal([])).toBeNull()
  })

  it('closes a summary that is too long anyway with the ellipsis this file uses', () => {
    const said = containerRefusal([
      {
        started: false,
        issue: 9,
        detail: 'irrelevant',
        room: {
          resource: 'disk',
          summary: `volume /${'deep/'.repeat(60)} has 1.0 GB free`,
        },
      },
    ])
    expect(said?.sentence.length).toBeLessThanOrEqual(200)
    expect(nothingStartedLine(said?.sentence, 1)).toContain('... 1 issue(s)')
    // And a refusal that somehow carries no summary still says something true.
    expect(
      containerRefusal([
        { started: false, issue: 1, room: { resource: 'memory' } },
      ])?.sentence,
    ).toBe('no room in the container: memory')
  })
})
