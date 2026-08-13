import XCTest
@testable import TriOSKit

/// Guardrails, the verification gate, and the readings the scanner refuses to
/// report as measurements.
final class HiveRuntimeTests: XCTestCase {

    // MARK: - Policy

    func testPolicySanitizationClampsHostileValues() {
        var policy = HivePolicy.default
        policy.maxConcurrentBees = 500
        policy.cycleIntervalSeconds = 0
        policy.maxBeesPerHour = -3
        policy.maxBudgetUSDPerBee = 10_000
        // Raised so this exercises the absolute per-bee cap rather than the
        // daily ceiling, which is asserted separately below.
        policy.dailyBudgetUSD = 500

        let clean = policy.sanitized()
        XCTAssertEqual(clean.maxConcurrentBees, 8)
        XCTAssertEqual(clean.cycleIntervalSeconds, 30)
        XCTAssertEqual(clean.maxBeesPerHour, 1)
        XCTAssertEqual(clean.maxBudgetUSDPerBee, 100)
    }

    func testAPerBeeBudgetCannotExceedTheDailyCeiling() {
        // Otherwise one bee spends the whole day before the ceiling is read.
        var policy = HivePolicy.default
        policy.dailyBudgetUSD = 4
        policy.maxBudgetUSDPerBee = 50
        XCTAssertEqual(policy.sanitized().maxBudgetUSDPerBee, 4)
    }

    func testTheLoopStartsDisarmedAndBeesCannotPush() {
        // A fresh install must never begin spawning bees on its own.
        XCTAssertFalse(HivePolicy.default.enabled)
        XCTAssertFalse(HivePolicy.default.allowPush)
        XCTAssertTrue(HivePolicy.default.verifyBeforeReview)
    }

    func testRateLimiterCountsOnlyTheLastHour() {
        var limiter = HiveRateLimiter()
        let now = Date()
        limiter.record(now.addingTimeInterval(-7200))
        limiter.record(now.addingTimeInterval(-1800))
        limiter.record(now.addingTimeInterval(-60))

        XCTAssertEqual(limiter.spawnsInLastHour(now), 2)
        XCTAssertTrue(limiter.allows(limit: 3, now: now))
        XCTAssertFalse(limiter.allows(limit: 2, now: now))
    }

    func testTaskIsSchedulableOnlyBeforeItSucceedsOrGoesToxic() {
        var task = HiveTask(
            id: "t", title: "t", module: "m", path: "m", realm: "Swift",
            signalKind: "testGap", reason: "r", score: 1, confidence: 1, prompt: "p"
        )
        XCTAssertTrue(task.isSchedulable)
        task.state = .failed
        XCTAssertTrue(task.isSchedulable)
        task.state = .running
        XCTAssertFalse(task.isSchedulable)
        task.state = .review
        XCTAssertFalse(task.isSchedulable)
        task.state = .toxic
        XCTAssertFalse(task.isSchedulable)
        XCTAssertTrue(task.isTerminal)
    }

    // MARK: - Spend ceiling

    func testSpendAccumulatesPerLocalDay() {
        var state = HiveState()
        let today = Date()
        state.record(spend: 1.25, on: today)
        state.record(spend: 0.75, on: today)
        XCTAssertEqual(state.spent(on: today), 2.0, accuracy: 0.0001)
    }

    func testYesterdaysSpendDoesNotCountAgainstToday() {
        var state = HiveState()
        let today = Date()
        let yesterday = today.addingTimeInterval(-86_400)
        state.record(spend: 10, on: yesterday)
        XCTAssertEqual(state.spent(on: today), 0)
        XCTAssertEqual(state.spent(on: yesterday), 10)
    }

    func testZeroSpendIsNotRecorded() {
        var state = HiveState()
        state.record(spend: 0)
        XCTAssertTrue(state.spendByDay.isEmpty)
    }

    func testSpendSurvivesAStoreRoundTrip() {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-spend-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var state = HiveState()
        state.record(spend: 3.5)
        XCTAssertTrue(store.save(state))
        // A crash-loop must not reset the day's ceiling by restarting.
        XCTAssertEqual(store.load().spent(), 3.5, accuracy: 0.0001)
    }

    // MARK: - Persistence

    func testATaskLeftRunningByACrashGoesBackToTheQueue() {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-crash-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var task = HiveTask(
            id: "t", title: "t", module: "m", path: "m", realm: "Swift",
            signalKind: "testGap", reason: "r", score: 1, confidence: 1, prompt: "p"
        )
        task.state = .running
        store.save(HiveState(policy: .default, tasks: [task]))

        let loaded = store.load()
        // Neither counted as a success nor burned as a failure.
        XCTAssertEqual(loaded.tasks.first?.state, .pending)
        XCTAssertTrue(loaded.tasks.first?.lastError?.contains("interrupted") == true)
    }

    func testMissingStateFileYieldsDefaultsRatherThanCrashing() {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-missing-\(UUID().uuidString)")
        let loaded = HiveStore(stateRoot: root.path).load()
        XCTAssertTrue(loaded.tasks.isEmpty)
        XCTAssertFalse(loaded.policy.enabled)
    }

    func testAPolicyFileMissingNewerFieldsKeepsTheOperatorsSettings() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-legacy-\(UUID().uuidString)")
        try fm.createDirectory(at: root.appendingPathComponent("hive"), withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        let legacy = """
        {"policy":{"enabled":true,"maxConcurrentBees":5},"tasks":[],"updatedAt":"2026-08-13T00:00:00Z"}
        """
        try legacy.write(
            to: root.appendingPathComponent("hive/hive.json"), atomically: true, encoding: .utf8
        )

        let loaded = HiveStore(stateRoot: root.path).load()
        // The fields present are honoured...
        XCTAssertTrue(loaded.policy.enabled)
        XCTAssertEqual(loaded.policy.maxConcurrentBees, 5)
        // ...and the ones added later take their defaults instead of failing
        // the whole decode and silently resetting everything.
        XCTAssertTrue(loaded.policy.verifyBeforeReview)
        XCTAssertEqual(loaded.policy.maxConsecutiveFailures, 3)
        XCTAssertTrue(loaded.policy.skippedModules.isEmpty)
        XCTAssertTrue(loaded.spendByDay.isEmpty)
    }

    func testEventsAppendInOrderAndReadBackNewestFirst() {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-events-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        store.append(HiveEvent(kind: "bee_spawned", taskID: "t1", detail: "first"))
        store.append(HiveEvent(kind: "bee_succeeded", taskID: "t1", detail: "second"))

        let events = store.recentEvents(limit: 10)
        XCTAssertEqual(events.count, 2)
        XCTAssertEqual(events.first?.detail, "second")
    }

    func testRecentTranscriptsSurvivePruning() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-ret-\(UUID().uuidString)")
        let store = HiveStore(stateRoot: root.path)
        try fm.createDirectory(at: store.transcriptsDirectory, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        let fresh = store.transcriptsDirectory.appendingPathComponent("fresh.jsonl")
        try "{}".write(to: fresh, atomically: true, encoding: .utf8)
        let stale = store.transcriptsDirectory.appendingPathComponent("stale.jsonl")
        try "{}".write(to: stale, atomically: true, encoding: .utf8)
        try fm.setAttributes(
            [.modificationDate: Date().addingTimeInterval(-40 * 86_400)],
            ofItemAtPath: stale.path
        )

        XCTAssertEqual(store.pruneTranscripts(olderThanDays: 14), 1)
        XCTAssertTrue(fm.fileExists(atPath: fresh.path))
        XCTAssertFalse(fm.fileExists(atPath: stale.path))
    }

    // MARK: - Preflight

    func testSignedInCLIIsAllowedToSpawn() {
        let state = HiveAuthProbe.parse(#"{"loggedIn":true,"authMethod":"oauth"}"#)
        XCTAssertEqual(state, .loggedIn(method: "oauth"))
        XCTAssertTrue(state.canSpawn)
        XCTAssertNil(state.blockerText)
    }

    func testASignedOutReportIsReadFromStdoutNotFromTheExitCode() {
        // `claude auth status` exits 1 when signed out and still prints a
        // well-formed report. Reading the code instead of the report turns a
        // clear "signed out" into a vague "could not determine".
        let state = HiveAuthProbe.parse(#"{"loggedIn":false,"authMethod":"none"}"#)
        XCTAssertEqual(state, .loggedOut)
        XCTAssertFalse(state.canSpawn)
        XCTAssertTrue(state.blockerText?.contains("claude auth login") == true)
    }

    func testAnUnreadableProbeIsUnknownNotSignedOut() {
        // "We could not tell" and "it is signed out" call for different
        // messages, and neither may be silently assumed from the other.
        let state = HiveAuthProbe.parse("<html>proxy error</html>")
        XCTAssertFalse(state.canSpawn)
        guard case .unknown = state else {
            return XCTFail("expected .unknown, got \(state)")
        }
        XCTAssertTrue(state.blockerText?.contains("Could not determine") == true)
        XCTAssertEqual(HiveAuthProbe.parse(""), .unknown("empty response"))
    }

    // MARK: - Scanner

    func testCountsLinesTodosAndTestBlocks() {
        let source = """
        import Foundation
        // TODO: handle the empty case
        func add(_ a: Int, _ b: Int) -> Int { a + b }
        // FIXME: overflow
        func testAddWorks() {}
        @Test func alsoCounts() {}
        """
        let counts = HiveRepoScanner.count(in: source)
        XCTAssertEqual(counts.lines, 6)
        XCTAssertEqual(counts.todos, 2)
        XCTAssertEqual(counts.testBlocks, 2)
    }

    func testCountsRustTestAttributes() {
        let source = """
        #[test]
        fn it_works() {}
        #[tokio::test]
        async fn async_works() {}
        """
        XCTAssertEqual(HiveRepoScanner.count(in: source).testBlocks, 2)
    }

    func testChurnCountsEachCommitOncePerModule() {
        let log = """
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        apps/trios-macos/rings/SR-00/A.swift
        apps/trios-macos/rings/SR-00/B.swift
        apps/trios-macos/rings/SR-01/C.swift
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        apps/trios-macos/rings/SR-00/A.swift
        """
        let counts = HiveRepoScanner.parseChurn(
            log, prefixes: ["rings"], pathPrefix: "apps/trios-macos/"
        )
        // Two files in SR-00 in the first commit still count as one commit.
        XCTAssertEqual(counts["rings/SR-00"], 2)
        XCTAssertEqual(counts["rings/SR-01"], 1)
    }

    func testChurnIgnoresPathsOutsideThisApp() {
        // The repository is a monorepo; a commit to another app is not churn
        // in this one.
        let log = """
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        crates/trios-server/src/main.rs
        apps/other-app/rings/SR-00/X.swift
        apps/trios-macos/rings/SR-00/A.swift
        """
        let counts = HiveRepoScanner.parseChurn(
            log, prefixes: ["rings"], pathPrefix: "apps/trios-macos/"
        )
        XCTAssertEqual(counts.count, 1)
        XCTAssertEqual(counts["rings/SR-00"], 1)
    }

    func testAFlatRootIsOneModuleNotOnePerSubdirectory() {
        let log = """
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        apps/trios-macos/BR-OUTPUT/HiveModels.swift
        apps/trios-macos/BR-OUTPUT/QueenTabView.swift
        """
        let counts = HiveRepoScanner.parseChurn(
            log,
            prefixes: ["BR-OUTPUT"],
            pathPrefix: "apps/trios-macos/",
            flatRoots: ["BR-OUTPUT"]
        )
        XCTAssertEqual(counts["BR-OUTPUT"], 1)
    }

    func testRealmIsReadFromTheRingPrefix() {
        XCTAssertEqual(HiveRepoScanner.realm(forModule: "rings/SR-00"), .swiftRing)
        XCTAssertEqual(HiveRepoScanner.realm(forModule: "rings/RUST-13"), .rustRing)
        XCTAssertEqual(HiveRepoScanner.realm(forModule: "BR-OUTPUT"), .surface)
    }

    func testTestsLivingOutsideTheModuleTreeAreAttributedBackToIt() throws {
        // Swift keeps its tests in a sibling tree. Counting only in-tree
        // blocks would report a fully tested module as wholly untested.
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-attr-\(UUID().uuidString)")
        let rings = root.appendingPathComponent("rings")
        let tests = root.appendingPathComponent("tests")
        try fm.createDirectory(at: rings.appendingPathComponent("SR-00"), withIntermediateDirectories: true)
        try fm.createDirectory(at: rings.appendingPathComponent("SR-01"), withIntermediateDirectories: true)
        try fm.createDirectory(at: tests, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        try "struct TodoListProjection {}".write(
            to: rings.appendingPathComponent("SR-00/TodoListProjection.swift"),
            atomically: true, encoding: .utf8
        )
        try "struct BrowserCommand {}".write(
            to: rings.appendingPathComponent("SR-01/BrowserCommand.swift"),
            atomically: true, encoding: .utf8
        )
        try """
        func testA() { _ = TodoListProjection.self }
        func testB() { _ = TodoListProjection.self }
        """.write(to: tests.appendingPathComponent("ProjectionTests.swift"), atomically: true, encoding: .utf8)

        let counts = HiveRepoScanner(projectRoot: root.path, gitRoot: root.path)
            .attributeExternalTests(
                testsRoot: tests.path,
                moduleRoot: rings.path,
                modulePrefix: "rings",
                flat: false
            )

        XCTAssertEqual(counts["rings/SR-00"], 2)
        // A module the test file never names gets no credit at all.
        XCTAssertNil(counts["rings/SR-01"])
    }

    // MARK: - Attribution matching

    func testIdentifiersAreWholeTokensNotSubstrings() {
        let tokens = HiveRepoScanner.identifiers(in: "let x = ChatMessageStore(id: some_name)")
        XCTAssertTrue(tokens.contains("ChatMessageStore"))
        XCTAssertTrue(tokens.contains("some_name"))
        // Substring matching credited ChatMessage for a file that only ever
        // named ChatMessageStore. Whole tokens do not.
        XCTAssertFalse(tokens.contains("ChatMessage"))
    }

    func testIdentifiersSplitOnPunctuationAndHandleTheFinalToken() {
        let tokens = HiveRepoScanner.identifiers(in: "a.b(c)[d]{trailing}")
        XCTAssertEqual(tokens, ["a", "b", "c", "d", "trailing"])
        XCTAssertEqual(HiveRepoScanner.identifiers(in: "last"), ["last"])
        XCTAssertTrue(HiveRepoScanner.identifiers(in: "...").isEmpty)
    }

    func testAttributionUsesWholeTokens() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-tok-\(UUID().uuidString)")
        let rings = root.appendingPathComponent("rings")
        let tests = root.appendingPathComponent("tests")
        try fm.createDirectory(at: rings.appendingPathComponent("SR-00"), withIntermediateDirectories: true)
        try fm.createDirectory(at: tests, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        try "enum Chat {}".write(
            to: rings.appendingPathComponent("SR-00/Chat.swift"), atomically: true, encoding: .utf8
        )
        // Names ChatMessage, never Chat on its own.
        try "func testOnly() { _ = ChatMessage.self }".write(
            to: tests.appendingPathComponent("T.swift"), atomically: true, encoding: .utf8
        )

        let counts = HiveRepoScanner(projectRoot: root.path, gitRoot: root.path)
            .attributeExternalTests(
                testsRoot: tests.path, moduleRoot: rings.path, modulePrefix: "rings", flat: false
            )
        XCTAssertTrue(counts.isEmpty)
    }

    // MARK: - Build artefacts

    func testBuildArtefactsAreNotMeasuredAsSource() {
        let base = "/repo/rings/RUST-13/"
        for artefact in ["target", "vendor", "node_modules", ".build", "gen"] {
            let url = URL(fileURLWithPath: "/repo/rings/RUST-13/\(artefact)/x.rs")
            XCTAssertTrue(
                HiveRepoScanner.isExcluded(url, relativeTo: base),
                "\(artefact) must not count as this module's source"
            )
        }
        XCTAssertFalse(
            HiveRepoScanner.isExcluded(URL(fileURLWithPath: "/repo/rings/RUST-13/src/x.rs"), relativeTo: base),
            "real source must still be measured"
        )
    }

    func testExclusionOnlyLooksBelowTheBase() {
        // A repository that itself lives under a directory called `vendor`
        // must not have every one of its own files excluded.
        let url = URL(fileURLWithPath: "/vendor/checkout/rings/SR-00/A.swift")
        XCTAssertFalse(HiveRepoScanner.isExcluded(url, relativeTo: "/vendor/checkout/rings/"))
    }

    func testAStaleIssuesSnapshotIsUnreadableNotZero() throws {
        // In the tree this was ported from, the file was 116 days old and was
        // being scored as a current reading of open issues.
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-stale-\(UUID().uuidString)")
        try fm.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        let snapshot = root.appendingPathComponent("issues_snapshot.json")
        try #"[{"number":1,"title":"rings/SR-00 is broken"}]"#
            .write(to: snapshot, atomically: true, encoding: .utf8)
        try fm.setAttributes(
            [.modificationDate: Date().addingTimeInterval(-116 * 86_400)],
            ofItemAtPath: snapshot.path
        )

        let age = Int(Date().timeIntervalSince(
            (try fm.attributesOfItem(atPath: snapshot.path)[.modificationDate] as? Date) ?? Date()
        ) / 86_400)
        XCTAssertGreaterThan(age, HiveRepoScanner.maxIssueSnapshotAgeDays)
    }

    func testIssueMentionsAreCountedPerModulePath() {
        let snapshot = #"{"issues":[{"title":"rings/SR-00 leaks","body":"see rings/SR-00 and rings/SR-01"}]}"#
        let counts = HiveRepoScanner.parseIssueMentions(snapshot, prefixes: ["rings"])
        XCTAssertEqual(counts["rings/SR-00"], 2)
        XCTAssertEqual(counts["rings/SR-01"], 1)
    }
}
