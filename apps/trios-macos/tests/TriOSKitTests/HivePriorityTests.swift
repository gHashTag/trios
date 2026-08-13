import XCTest
@testable import TriOSKit

/// The ranking engine, and the one rule it exists to enforce: a signal that
/// was not measured is dropped from the score, never counted as zero.
final class HivePriorityTests: XCTestCase {

    private func facts(
        _ module: String,
        lines: Int? = 1000,
        todos: Int? = 0,
        churn: Int? = 0,
        tests: Int? = 6,
        status: String? = "active",
        issues: Int? = 0
    ) -> HiveModuleFacts {
        var f = HiveModuleFacts(module: module, path: module, realm: .swiftRing)
        f.lines = lines
        f.todos = todos
        f.churn30d = churn
        f.testBlocks = tests
        f.declaredStatus = status
        f.openIssues = issues
        return f
    }

    func testRanksTheModuleWithMoreProblemsHigher() {
        let ranked = HivePriorityEngine.rank([
            facts("rings/SR-00", todos: 0, churn: 0, tests: 12, issues: 0),
            facts("rings/SR-01", todos: 40, churn: 30, tests: 0, issues: 9),
        ])
        XCTAssertEqual(ranked.first?.module, "rings/SR-01")
        XCTAssertEqual(ranked.last?.module, "rings/SR-00")
        XCTAssertGreaterThan(ranked.first?.score ?? 0, ranked.last?.score ?? 1)
    }

    func testRankingIsDeterministicForEqualInputs() {
        let input = [facts("b"), facts("a"), facts("c")]
        let first = HivePriorityEngine.rank(input).map(\.module)
        XCTAssertEqual(first, HivePriorityEngine.rank(input).map(\.module))
        // Equal score and confidence fall back to the module name, not to
        // whatever order the filesystem happened to hand over.
        XCTAssertEqual(first, ["a", "b", "c"])
    }

    // MARK: - Absent is not zero

    func testUnmeasuredSignalIsExcludedFromTheScoreNotCountedAsZero() throws {
        // Identical measured evidence; one module simply could not have its
        // churn read. It must not be punished for a probe that failed.
        var blind = facts("blind", todos: 20, churn: nil, tests: 0, issues: 4)
        blind.unmeasuredReasons[.churn] = "git history unavailable"
        let sighted = facts("sighted", todos: 20, churn: 0, tests: 0, issues: 4)

        let ranked = HivePriorityEngine.rank([blind, sighted])
        let blindTarget = try XCTUnwrap(ranked.first { $0.module == "blind" })
        let sightedTarget = try XCTUnwrap(ranked.first { $0.module == "sighted" })

        // The sighted module measured churn as a real zero, which drags it
        // down. The blind one loses confidence instead of score.
        XCTAssertGreaterThan(blindTarget.score, sightedTarget.score)
        XCTAssertLessThan(blindTarget.confidence, sightedTarget.confidence)
    }

    func testUnmeasuredSignalCarriesItsReasonThrough() throws {
        var f = facts("opaque", churn: nil)
        f.unmeasuredReasons[.churn] = "git log exited 128"
        let target = try XCTUnwrap(HivePriorityEngine.rank([f]).first)
        let churn = try XCTUnwrap(target.signals.first { $0.kind == .churn })

        XCTAssertFalse(churn.raw.isMeasured)
        XCTAssertFalse(churn.normalized.isMeasured)
        XCTAssertEqual(churn.raw.unmeasuredReason, "git log exited 128")
    }

    func testConfidenceIsTheMeasuredShareOfTotalWeight() throws {
        let full = try XCTUnwrap(HivePriorityEngine.rank([facts("full")]).first)
        XCTAssertEqual(full.confidence, 1.0, accuracy: 0.0001)

        var half = HiveModuleFacts(module: "half", path: "half", realm: .swiftRing)
        half.lines = 500
        half.todos = 5
        let partial = try XCTUnwrap(HivePriorityEngine.rank([half]).first)
        XCTAssertLessThan(partial.confidence, 1.0)
        XCTAssertGreaterThan(partial.confidence, 0.0)
    }

    func testAModuleWithNothingMeasuredScoresZeroWithZeroConfidence() throws {
        let blank = HiveModuleFacts(module: "unknown", path: "unknown", realm: .swiftRing)
        let target = try XCTUnwrap(HivePriorityEngine.rank([blank]).first)

        XCTAssertEqual(target.score, 0)
        XCTAssertEqual(target.confidence, 0)
        XCTAssertEqual(target.measuredCount, 0)
        // And it must say so rather than presenting a confident zero.
        XCTAssertTrue(target.reason.contains("no positive signal measured"))
    }

    func testReasonNamesOnlySignalsItActuallyRead() throws {
        var f = facts("loud", todos: 50, churn: nil, tests: 0, issues: 0)
        f.unmeasuredReasons[.churn] = "git history unavailable"
        let target = try XCTUnwrap(HivePriorityEngine.rank([f]).first)
        XCTAssertTrue(target.reason.contains("confidence"))
        XCTAssertFalse(target.reason.contains("Churn"))
    }

    func testTestGapFallsAsTestDensityRises() {
        let untested = HivePriorityEngine.rawReadings(for: facts("u", tests: 0))[.testGap]
        let tested = HivePriorityEngine.rawReadings(for: facts("t", tests: 60))[.testGap]
        XCTAssertEqual(untested?.value ?? 0, 1.0, accuracy: 0.0001)
        XCTAssertEqual(tested?.value ?? 1, 0.0, accuracy: 0.0001)
    }

    func testDeclaredStubCountsAsIncomplete() {
        let stub = HivePriorityEngine.rawReadings(for: facts("x", status: "stub"))[.declaredIncomplete]
        let active = HivePriorityEngine.rawReadings(for: facts("y", status: "active"))[.declaredIncomplete]
        XCTAssertEqual(stub?.value, 1)
        XCTAssertEqual(active?.value, 0)
    }

    // MARK: - Task synthesis

    func testTaskIDIsStableForTheSameModuleAndWeakness() {
        let a = HiveTaskFactory.taskID(module: "rings/SR-00", kind: .testGap)
        XCTAssertEqual(a, HiveTaskFactory.taskID(module: "rings/SR-00", kind: .testGap))
        XCTAssertNotEqual(a, HiveTaskFactory.taskID(module: "rings/SR-00", kind: .churn))
        XCTAssertEqual(a, "hive-rings-sr-00-testGap")
    }

    func testPromptForbidsPushingWhenPolicyForbidsIt() throws {
        var f = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        f.lines = 1000
        f.todos = 30
        f.churn30d = 5
        f.testBlocks = 0
        f.declaredStatus = "active"
        f.openIssues = 2

        let target = try XCTUnwrap(HivePriorityEngine.rank([f]).first)
        var policy = HivePolicy.default
        policy.allowPush = false

        let task = try XCTUnwrap(HiveTaskFactory.makeTask(from: target, policy: policy))
        XCTAssertTrue(task.prompt.contains("Do NOT push"))
        XCTAssertTrue(task.prompt.contains("Never `git add -A`"))
        XCTAssertTrue(task.prompt.contains("is not a zero"))
        XCTAssertEqual(task.state, .pending)
        XCTAssertEqual(task.attempts, 0)
    }

    func testPromptListsUnmeasuredSignalsAsUnknownNotZero() throws {
        var f = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        f.lines = 1000
        f.todos = 30
        f.unmeasuredReasons[.churn] = "git history unavailable"
        f.unmeasuredReasons[.openIssues] = "issues snapshot absent"

        let target = try XCTUnwrap(HivePriorityEngine.rank([f]).first)
        let task = try XCTUnwrap(HiveTaskFactory.makeTask(from: target, policy: .default))

        XCTAssertTrue(task.prompt.contains("NOT MEASURED"))
        XCTAssertTrue(task.prompt.contains("treat these as unknown, not as zero"))
        XCTAssertTrue(task.prompt.contains("git history unavailable"))
    }

    func testMakeTaskReturnsNilWhenNoSignalDrivesTheScore() throws {
        let blank = HiveModuleFacts(module: "unknown", path: "unknown", realm: .swiftRing)
        let target = try XCTUnwrap(HivePriorityEngine.rank([blank]).first)
        XCTAssertNil(HiveTaskFactory.makeTask(from: target, policy: .default))
    }
}
