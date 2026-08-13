import XCTest
@testable import TriOSKit

/// Standing invariants and the two literature-driven rules they encode.
final class HiveInvariantTests: XCTestCase {

    private func task(
        id: String = "t",
        state: HiveTaskState = .pending,
        attempts: Int = 0,
        verification: String? = nil
    ) -> HiveTask {
        var t = HiveTask(
            id: id, title: "t", module: "m", path: "m", realm: "Swift",
            signalKind: "testGap", reason: "r", score: 1, confidence: 1, prompt: "p"
        )
        t.state = state
        t.attempts = attempts
        t.verification = verification
        return t
    }

    // MARK: - A clean loop has nothing to report

    func testADefaultPolicyWithNoTasksSatisfiesEveryInvariant() {
        XCTAssertTrue(
            HiveInvariants.check(policy: .default, tasks: [], spentToday: 0).isEmpty
        )
    }

    // MARK: - Policy self-consistency

    func testAPerBeeBudgetAboveTheDailyCeilingIsAViolation() {
        var policy = HivePolicy.default
        policy.dailyBudgetUSD = 4
        policy.maxBudgetUSDPerBee = 50   // set directly, bypassing sanitize()
        let ids = HiveInvariants.check(policy: policy, tasks: [], spentToday: 0).map(\.id)
        XCTAssertTrue(ids.contains("budget-ordering"))
    }

    func testOverspendingTheDayIsAViolation() {
        var policy = HivePolicy.default
        policy.dailyBudgetUSD = 10
        let ids = HiveInvariants.check(policy: policy, tasks: [], spentToday: 10.01).map(\.id)
        XCTAssertTrue(ids.contains("daily-ceiling"))
        // Spending exactly the ceiling is not yet a breach.
        XCTAssertFalse(
            HiveInvariants.check(policy: policy, tasks: [], spentToday: 10).map(\.id)
                .contains("daily-ceiling")
        )
    }

    // MARK: - Bookkeeping drift

    func testATaskPastItsAttemptBudgetIsAViolation() {
        var policy = HivePolicy.default
        policy.maxAttemptsPerTask = 3
        let ids = HiveInvariants.check(
            policy: policy, tasks: [task(attempts: 4)], spentToday: 0
        ).map(\.id)
        XCTAssertTrue(ids.contains("attempts-exceeded"))
    }

    func testReachingReviewWithNoVerdictIsAViolationWhileVerificationIsOn() {
        var policy = HivePolicy.default
        policy.verifyBeforeReview = true
        let ids = HiveInvariants.check(
            policy: policy, tasks: [task(state: .review)], spentToday: 0
        ).map(\.id)
        XCTAssertTrue(ids.contains("review-without-verdict"))

        // With a verdict recorded it is fine...
        XCTAssertFalse(
            HiveInvariants.check(
                policy: policy, tasks: [task(state: .review, verification: "VERIFIED: ok")], spentToday: 0
            ).map(\.id).contains("review-without-verdict")
        )
        // ...and with verification switched off the invariant does not apply.
        policy.verifyBeforeReview = false
        XCTAssertFalse(
            HiveInvariants.check(policy: policy, tasks: [task(state: .review)], spentToday: 0)
                .map(\.id).contains("review-without-verdict")
        )
    }

    func testDuplicateTaskIDsAreAViolation() {
        let ids = HiveInvariants.check(
            policy: .default, tasks: [task(id: "same"), task(id: "same")], spentToday: 0
        ).map(\.id)
        XCTAssertTrue(ids.contains("duplicate-tasks"))
    }

    func testMoreRunningTasksThanBeesIsAViolation() {
        var policy = HivePolicy.default
        policy.maxConcurrentBees = 1
        let ids = HiveInvariants.check(
            policy: policy,
            tasks: [task(id: "a", state: .running), task(id: "b", state: .running)],
            spentToday: 0
        ).map(\.id)
        XCTAssertTrue(ids.contains("running-over-limit"))
    }

    func testViolationsCarryAReadableDetailNotJustAnID() {
        var policy = HivePolicy.default
        policy.maxAttemptsPerTask = 1
        let violations = HiveInvariants.check(
            policy: policy, tasks: [task(id: "hive-rings-sr-00-testGap", attempts: 9)], spentToday: 0
        )
        let attemptViolation = violations.first { $0.id == "attempts-exceeded" }
        XCTAssertTrue(attemptViolation?.detail.contains("hive-rings-sr-00-testGap") == true)
    }

    // MARK: - Confidence floor
    //
    // Harness optimisers invent failures that never happened when the input
    // merely resembles a rule they know. Dispatching against a barely-measured
    // module is the same mistake: work proposed on evidence never gathered.

    func testATargetBelowTheConfidenceFloorProducesNoTask() throws {
        // One measured signal out of six: well under the floor.
        var facts = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        facts.churn30d = 5
        let target = try XCTUnwrap(HivePriorityEngine.rank([facts]).first)

        XCTAssertLessThan(target.confidence, HiveInvariants.minimumDispatchConfidence)
        XCTAssertNil(HiveTaskFactory.makeTask(from: target, policy: .default))
        XCTAssertTrue(HiveTaskFactory.rejection(for: target)?.contains("below the") == true)
    }

    func testAWellMeasuredTargetPassesTheFloor() throws {
        var facts = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        facts.lines = 1000
        facts.todos = 30
        facts.churn30d = 5
        facts.testBlocks = 0
        facts.openIssues = 2
        let target = try XCTUnwrap(HivePriorityEngine.rank([facts]).first)

        XCTAssertGreaterThanOrEqual(target.confidence, HiveInvariants.minimumDispatchConfidence)
        XCTAssertNil(HiveTaskFactory.rejection(for: target))
        XCTAssertNotNil(HiveTaskFactory.makeTask(from: target, policy: .default))
    }

    func testTheRejectionReasonNamesTheFloorRatherThanFailingSilently() throws {
        let blank = HiveModuleFacts(module: "m", path: "m", realm: .swiftRing)
        let target = try XCTUnwrap(HivePriorityEngine.rank([blank]).first)
        XCTAssertEqual(HiveTaskFactory.rejection(for: target), "no signal drove the score")
    }

    // MARK: - No anchoring on the previous attempt
    //
    // Handing a model its own failed attempt reproduces a near-identical
    // program in a third to two thirds of retries. Each Hive attempt is a new
    // session with the original prompt, which is blind resampling. This test
    // exists so a later "improvement" that pastes lastError into the prompt
    // fails here instead of quietly degrading every retry.

    func testAGeneratedPromptNeverCarriesThePreviousFailure() throws {
        var facts = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        facts.lines = 1000
        facts.todos = 30
        facts.churn30d = 5
        facts.testBlocks = 0
        facts.openIssues = 2
        let target = try XCTUnwrap(HivePriorityEngine.rank([facts]).first)
        let task = try XCTUnwrap(HiveTaskFactory.makeTask(from: target, policy: .default))

        let previousError = "swift build failed: cannot find 'Foo' in scope at line 42 of Bar.swift"
        XCTAssertTrue(
            HiveInvariants.promptIsAnchorFree(task.prompt, previousError: previousError),
            "the retry prompt must not contain the previous attempt's failure"
        )
    }

    func testTheAnchorCheckActuallyCatchesAnAnchoredPrompt() {
        let previousError = "swift build failed: cannot find 'Foo' in scope at line 42 of Bar.swift"
        let anchored = "Fix this module. Last time you got: \(previousError)"
        XCTAssertFalse(HiveInvariants.promptIsAnchorFree(anchored, previousError: previousError))
    }

    func testNoPreviousErrorMeansNothingToAnchorOn() {
        XCTAssertTrue(HiveInvariants.promptIsAnchorFree("anything", previousError: nil))
        XCTAssertTrue(HiveInvariants.promptIsAnchorFree("anything", previousError: ""))
    }
}
