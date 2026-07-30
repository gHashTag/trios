import XCTest
@testable import TriOSKit

final class ProviderCircuitBreakerTests: XCTestCase {
    private let key = ProviderEndpointKey(provider: .openrouter, baseURL: "https://openrouter.ai/api/v1")

    func testInitialStateIsClosed() async {
        let breaker = ProviderCircuitBreaker()
        XCTAssertEqual(await breaker.state(for: key), .closed)
        XCTAssertTrue(await breaker.canSend(to: key))
    }

    func testStaysClosedBelowThreshold() async {
        let breaker = ProviderCircuitBreaker(failureThreshold: 3)
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        XCTAssertEqual(await breaker.state(for: key), .closed)
        XCTAssertTrue(await breaker.canSend(to: key))
    }

    func testTripsOpenAtThreshold() async {
        let breaker = ProviderCircuitBreaker(failureThreshold: 2)
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        XCTAssertEqual(await breaker.state(for: key), .open)
        XCTAssertFalse(await breaker.canSend(to: key))
    }

    func testCooldownTransitionsToHalfOpen() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        XCTAssertFalse(await breaker.canSend(to: key))

        now.addTimeInterval(31)
        XCTAssertTrue(await breaker.canSend(to: key))
        XCTAssertEqual(await breaker.state(for: key), .halfOpen)
    }

    func testRetryAfterOverridesComputedCooldown() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .rateLimit, retryAfter: 120)
        await breaker.recordFailure(key, kind: .rateLimit, retryAfter: 120)

        let nextRetry = await breaker.nextRetryAt(for: key)
        XCTAssertEqual(nextRetry?.timeIntervalSince(now), 120, accuracy: 0.1)

        now.addTimeInterval(119)
        XCTAssertFalse(await breaker.canSend(to: key))
        now.addTimeInterval(2)
        XCTAssertTrue(await breaker.canSend(to: key))
    }

    func testHalfOpenSuccessClosesBreaker() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        now.addTimeInterval(31)
        await breaker.recordSuccess(key)
        XCTAssertEqual(await breaker.state(for: key), .closed)
        XCTAssertTrue(await breaker.canSend(to: key))
        XCTAssertEqual(await breaker.failureStreak(for: key), 0)
    }

    func testHalfOpenFailureReopensBreaker() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        now.addTimeInterval(31)
        await breaker.recordFailure(key, kind: .gateway)
        XCTAssertEqual(await breaker.state(for: key), .open)
        XCTAssertFalse(await breaker.canSend(to: key))
    }

    func testPersistentKindUsesLongerCooldown() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            persistentBackoffMultiplier: 4,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .auth)
        await breaker.recordFailure(key, kind: .auth)
        let authRetry = await breaker.nextRetryAt(for: key)!

        let authKey = ProviderEndpointKey(provider: .anthropic, baseURL: "https://api.anthropic.com")
        now = Date()
        let transientBreaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            transientBackoffMultiplier: 2,
            clock: { now }
        )
        await transientBreaker.recordFailure(authKey, kind: .gateway)
        await transientBreaker.recordFailure(authKey, kind: .gateway)
        let gatewayRetry = await transientBreaker.nextRetryAt(for: authKey)!

        XCTAssertGreaterThan(authRetry.timeIntervalSince(now), gatewayRetry.timeIntervalSince(now))
    }

    func testResetClearsState() async {
        let breaker = ProviderCircuitBreaker(failureThreshold: 1)
        await breaker.recordFailure(key, kind: .balance)
        await breaker.reset(key)
        XCTAssertEqual(await breaker.state(for: key), .closed)
        XCTAssertNil(await breaker.lastFailureKind(for: key))
    }

    func testTransportErrorMapping() {
        let rateLimit = TransportError.serverError(
            statusCode: 429,
            bodySample: "Rate limited",
            url: nil,
            retryAfter: 5
        )
        XCTAssertEqual(rateLimit.circuitBreakerFailureKind, .rateLimit)
        XCTAssertEqual(rateLimit.retryAfter, 5)

        let auth = TransportError.serverError(statusCode: 401, bodySample: "Unauthorized", url: nil)
        XCTAssertEqual(auth.circuitBreakerFailureKind, .auth)

        let balance = TransportError.serverError(statusCode: 402, bodySample: "Insufficient balance", url: nil)
        XCTAssertEqual(balance.circuitBreakerFailureKind, .balance)

        let unavailable = TransportError.serverError(statusCode: 503, bodySample: "Unavailable", url: nil)
        XCTAssertEqual(unavailable.circuitBreakerFailureKind, .gateway)

        let timeout = TransportError.requestTimedOut
        XCTAssertEqual(timeout.circuitBreakerFailureKind, .timeout)
    }

    func testDifferentEndpointsAreIsolated() async {
        let breaker = ProviderCircuitBreaker(failureThreshold: 2)
        let keyA = ProviderEndpointKey(provider: .openai, baseURL: "https://api.openai.com")
        let keyB = ProviderEndpointKey(provider: .anthropic, baseURL: "https://api.anthropic.com")
        await breaker.recordFailure(keyA, kind: .gateway)
        await breaker.recordFailure(keyA, kind: .gateway)
        XCTAssertEqual(await breaker.state(for: keyA), .open)
        XCTAssertEqual(await breaker.state(for: keyB), .closed)
    }

    func testHalfOpenProbeLockAllowsOnlyOneCaller() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        now.addTimeInterval(31)

        let first = await breaker.beginProbe(key)
        let second = await breaker.beginProbe(key)
        XCTAssertTrue(first)
        XCTAssertFalse(second)

        // A caller while a probe is in flight cannot send.
        XCTAssertFalse(await breaker.canSend(to: key))

        await breaker.endProbe(key, success: true)
        XCTAssertEqual(await breaker.state(for: key), .closed)
        XCTAssertTrue(await breaker.canSend(to: key))
    }

    func testStuckProbeReleasesAfterTimeout() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            halfOpenProbeTimeout: 10,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        now.addTimeInterval(31)

        XCTAssertTrue(await breaker.beginProbe(key))
        now.addTimeInterval(11)
        // After the probe timeout a new caller can start a probe.
        XCTAssertTrue(await breaker.canSend(to: key))
        XCTAssertTrue(await breaker.beginProbe(key))
    }

    func testJitterProducesDifferentCooldownsForDifferentEndpoints() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            jitterFactor: 0.5,
            clock: { now }
        )
        let keyA = ProviderEndpointKey(provider: .openai, baseURL: "https://api.openai.com")
        let keyB = ProviderEndpointKey(provider: .anthropic, baseURL: "https://api.anthropic.com")

        await breaker.recordFailure(keyA, kind: .gateway)
        await breaker.recordFailure(keyA, kind: .gateway)
        await breaker.recordFailure(keyB, kind: .gateway)
        await breaker.recordFailure(keyB, kind: .gateway)

        let retryA = await breaker.nextRetryAt(for: keyA)!
        let retryB = await breaker.nextRetryAt(for: keyB)!
        // Jitter of ±50% on a 30s base means the absolute difference should be
        // non-zero for two different endpoint keys.
        XCTAssertNotEqual(retryA.timeIntervalSince(now), retryB.timeIntervalSince(now), accuracy: 0.1)
    }

    func testHalfOpenFailedProbeReopensBreaker() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .gateway)
        await breaker.recordFailure(key, kind: .gateway)
        now.addTimeInterval(31)

        XCTAssertTrue(await breaker.beginProbe(key))
        await breaker.endProbe(key, success: false)
        XCTAssertEqual(await breaker.state(for: key), .open)
    }

    func testBalanceCooldownFloorIsFourTimesBase() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 1,
            baseCooldown: 30,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .balance)

        let nextRetry = await breaker.nextRetryAt(for: key)!
        XCTAssertGreaterThanOrEqual(nextRetry.timeIntervalSince(now), 120, accuracy: 0.1)
    }

    func testContextLengthFailureKindMapping() {
        let error = TransportError.serverError(
            statusCode: 400,
            bodySample: "context_length_exceeded",
            url: nil
        )
        XCTAssertEqual(error.circuitBreakerFailureKind, .contextLength)
    }

    func testContextLengthCooldownIsPersistent() async {
        var now = Date()
        let breaker = ProviderCircuitBreaker(
            failureThreshold: 2,
            baseCooldown: 30,
            persistentBackoffMultiplier: 4,
            clock: { now }
        )
        await breaker.recordFailure(key, kind: .contextLength)
        await breaker.recordFailure(key, kind: .contextLength)
        XCTAssertEqual(await breaker.lastFailureKind(for: key), .contextLength)
        XCTAssertEqual(await breaker.state(for: key), .open)
    }

    func testContextLengthNotEligibleForCrossProviderFailover() {
        let error = TransportError.serverError(
            statusCode: 413,
            bodySample: "Payload Too Large",
            url: nil
        )
        XCTAssertFalse(error.isEligibleForCrossProviderFailover)
    }
}
