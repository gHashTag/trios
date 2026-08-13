import XCTest
@testable import TriOSKit

/// Evidence currency: a lifecycle state is a claim unless the evidence behind
/// it is *current*. A pass measured against a commit the tree has since moved
/// past is not a pass now.
final class HiveEvidenceTests: XCTestCase {

    private let commitA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    private let commitB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

    func testAVerdictMeasuredAtTheCurrentCommitIsCurrent() {
        let state = HiveVerifier.evidenceState(verifiedAt: commitA, currentHead: commitA)
        XCTAssertEqual(state, .current)
        XCTAssertTrue(state.isCurrent)
        XCTAssertEqual(state.label, "CURRENT")
    }

    func testAVerdictFromAnEarlierCommitIsStale() {
        let state = HiveVerifier.evidenceState(verifiedAt: commitA, currentHead: commitB)
        XCTAssertEqual(state, .stale(measuredAt: commitA))
        XCTAssertFalse(state.isCurrent)
        XCTAssertEqual(state.label, "STALE")
    }

    func testAVerdictWithNoCommitRecordedIsNotCurrent() {
        // The dangerous reading: no commit means "probably fine". It means the
        // opposite - there is no way to tell.
        for missing in [nil, ""] {
            let state = HiveVerifier.evidenceState(verifiedAt: missing, currentHead: commitA)
            XCTAssertEqual(state, .unrecorded)
            XCTAssertFalse(state.isCurrent)
        }
    }

    func testAnUnreadableHeadIsUnknownNotCurrent() {
        for missing in [nil, ""] {
            let state = HiveVerifier.evidenceState(verifiedAt: commitA, currentHead: missing)
            XCTAssertFalse(state.isCurrent)
            guard case .unknown = state else {
                return XCTFail("expected .unknown, got \(state)")
            }
        }
    }

    func testOnlyOneOfTheFourStatesReadsAsCurrent() {
        let states: [HiveEvidenceState] = [
            .current,
            .stale(measuredAt: commitA),
            .unrecorded,
            .unknown("no git"),
        ]
        XCTAssertEqual(states.filter(\.isCurrent).count, 1)
    }

    func testHeadOfANonRepositoryIsNilRatherThanEmpty() {
        // An empty string would compare equal to another empty string and make
        // two unknowns look like a match.
        let verifier = HiveVerifier(projectRoot: "/tmp/not-a-repo-\(UUID().uuidString)")
        XCTAssertNil(verifier.head(at: verifier.projectRoot))
    }

    func testATaskCarriesItsVerificationCommitAcrossPersistence() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-evidence-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var task = HiveTask(
            id: "t", title: "t", module: "m", path: "m", realm: "Swift",
            signalKind: "testGap", reason: "r", score: 1, confidence: 1, prompt: "p"
        )
        task.state = .review
        task.verification = "VERIFIED: 231 tests"
        task.verified = true
        task.verifiedAtCommit = commitA
        XCTAssertTrue(store.save(HiveState(policy: .default, tasks: [task])))

        let loaded = try XCTUnwrap(store.load().tasks.first)
        XCTAssertEqual(loaded.verifiedAtCommit, commitA)
        XCTAssertEqual(
            HiveVerifier.evidenceState(verifiedAt: loaded.verifiedAtCommit, currentHead: commitB),
            .stale(measuredAt: commitA)
        )
    }

    func testAStateFileWrittenBeforeCommitTrackingStillLoads() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-old-\(UUID().uuidString)")
        try fm.createDirectory(at: root.appendingPathComponent("hive"), withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        let legacy = """
        {"policy":{},"tasks":[{"id":"t","title":"t","module":"m","state":"review",\
        "verification":"VERIFIED: ok","verified":true,"score":1,"confidence":1,"attempts":1}],\
        "updatedAt":"2026-08-13T00:00:00Z"}
        """
        try legacy.write(
            to: root.appendingPathComponent("hive/hive.json"), atomically: true, encoding: .utf8
        )

        let loaded = try XCTUnwrap(HiveStore(stateRoot: root.path).load().tasks.first)
        XCTAssertEqual(loaded.verification, "VERIFIED: ok")
        // No commit was recorded back then, and that is reported as such rather
        // than assumed current.
        XCTAssertNil(loaded.verifiedAtCommit)
        XCTAssertEqual(
            HiveVerifier.evidenceState(verifiedAt: loaded.verifiedAtCommit, currentHead: commitA),
            .unrecorded
        )
    }
}

/// Goodhart's law inside the loop: an unsupervised agent optimises the score it
/// is measured by, not the goal behind it.
final class HiveMetricGamingTests: XCTestCase {

    private func target() throws -> HiveTarget {
        var facts = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        facts.lines = 1000
        facts.todos = 30
        facts.churn30d = 5
        facts.testBlocks = 0
        facts.openIssues = 2
        return try XCTUnwrap(HivePriorityEngine.rank([facts]).first)
    }

    func testThePromptForbidsOptimisingTheSelectingSignal() throws {
        let task = try XCTUnwrap(HiveTaskFactory.makeTask(from: target(), policy: .default))
        XCTAssertTrue(task.prompt.contains("Do NOT optimise the signal that selected this task"))
    }

    func testThePromptNamesTheConcreteWaysToGameEachSignal() throws {
        let prompt = try XCTUnwrap(HiveTaskFactory.makeTask(from: target(), policy: .default)).prompt
        // Naming the specific cheat is what makes the rule actionable; "do good
        // work" is not a constraint anything can follow.
        XCTAssertTrue(prompt.contains("Trivial tests that raise coverage"))
        XCTAssertTrue(prompt.contains("TODO markers deleted without resolving"))
        XCTAssertTrue(prompt.contains("files split to lower a size score"))
    }

    func testThePromptStillAllowsReportingThatNothingIsWrong() throws {
        let prompt = try XCTUnwrap(HiveTaskFactory.makeTask(from: target(), policy: .default)).prompt
        // Without this, the only way to satisfy the task is to change
        // something - which is exactly how metric-gaming becomes rational.
        XCTAssertTrue(prompt.contains("no change needed"))
        XCTAssertTrue(prompt.contains("report that there is none"))
    }
}
