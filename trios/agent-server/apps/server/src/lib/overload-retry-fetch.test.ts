/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 */

import { describe, it } from 'bun:test'
import assert from 'node:assert'
import {
  createOverloadRetryFetch,
  streamErrorInFirstEvent,
} from './overload-retry-fetch'

const OVERLOADED =
  'data: {"error":{"message":"Service temporarily overloaded","type":"service_unavailable","code":503}}\n\ndata: [DONE]\n\n'
const GOOD =
  'data: {"id":"x","choices":[{"index":0,"delta":{"role":"assistant","content":"READY"}}]}\n\ndata: [DONE]\n\n'

function sse(body: string, status = 200): Response {
  return new Response(body, {
    status,
    headers: { 'content-type': 'text/event-stream; charset=utf-8' },
  })
}

function sequence(responses: (() => Response)[]) {
  let calls = 0
  const fetchImpl = (async () => {
    const make = responses[Math.min(calls, responses.length - 1)]
    calls++
    return make()
  }) as unknown as typeof fetch
  return { fetchImpl, calls: () => calls }
}

const noSleep = async () => {}

describe('streamErrorInFirstEvent', () => {
  it('names the error the endpoint put in a 200 stream', () => {
    assert.strictEqual(
      streamErrorInFirstEvent(OVERLOADED),
      '[503] Service temporarily overloaded',
    )
  })
  it('passes a real first event', () => {
    assert.strictEqual(streamErrorInFirstEvent(GOOD), null)
  })
  it('passes a non-JSON event', () => {
    assert.strictEqual(streamErrorInFirstEvent('data: [DONE]\n\n'), null)
  })
})

describe('createOverloadRetryFetch', () => {
  it('retries an in-stream overload and hands on the good stream intact', async () => {
    const seq = sequence([() => sse(OVERLOADED), () => sse(OVERLOADED), () => sse(GOOD)])
    const retries: string[] = []
    const f = createOverloadRetryFetch({
      fetchImpl: seq.fetchImpl,
      sleep: noSleep,
      onRetry: (i) => retries.push(i.reason),
    })
    const res = await f('https://example.test/v1/chat/completions', { method: 'POST', body: '{}' })
    assert.strictEqual(res.status, 200)
    assert.strictEqual(await res.text(), GOOD)
    assert.strictEqual(seq.calls(), 3)
    assert.deepStrictEqual(retries, [
      '[503] Service temporarily overloaded',
      '[503] Service temporarily overloaded',
    ])
  })

  it('retries a 5xx status and a 429', async () => {
    const seq = sequence([
      () => new Response('down', { status: 503 }),
      () => new Response('slow', { status: 429 }),
      () => sse(GOOD),
    ])
    const f = createOverloadRetryFetch({ fetchImpl: seq.fetchImpl, sleep: noSleep })
    const res = await f('https://example.test/', {})
    assert.strictEqual(res.status, 200)
    assert.strictEqual(seq.calls(), 3)
  })

  it('gives up as a 503 with the endpoint words after maxAttempts', async () => {
    const seq = sequence([() => sse(OVERLOADED)])
    const f = createOverloadRetryFetch({ fetchImpl: seq.fetchImpl, sleep: noSleep, maxAttempts: 3 })
    const res = await f('https://example.test/', {})
    assert.strictEqual(res.status, 503)
    assert.match(await res.text(), /Service temporarily overloaded/)
    assert.strictEqual(seq.calls(), 3)
  })

  it('does not touch a non-streaming or a 4xx response', async () => {
    const seq = sequence([() => new Response('{"ok":true}', { status: 200, headers: { 'content-type': 'application/json' } })])
    const f = createOverloadRetryFetch({ fetchImpl: seq.fetchImpl, sleep: noSleep })
    assert.strictEqual(await (await f('https://example.test/', {})).text(), '{"ok":true}')
    const bad = sequence([() => new Response('nope', { status: 400 })])
    const g = createOverloadRetryFetch({ fetchImpl: bad.fetchImpl, sleep: noSleep })
    assert.strictEqual((await g('https://example.test/', {})).status, 400)
    assert.strictEqual(bad.calls(), 1)
  })

  it('replays a stream that arrives in many small chunks', async () => {
    const encoder = new TextEncoder()
    const pieces = GOOD.match(/.{1,7}/gs) ?? []
    const body = new ReadableStream<Uint8Array>({
      start(c) {
        for (const p of pieces) c.enqueue(encoder.encode(p))
        c.close()
      },
    })
    const seq = sequence([() => new Response(body, { status: 200, headers: { 'content-type': 'text/event-stream' } })])
    const f = createOverloadRetryFetch({ fetchImpl: seq.fetchImpl, sleep: noSleep })
    assert.strictEqual(await (await f('https://example.test/', {})).text(), GOOD)
  })
})
