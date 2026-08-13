import XCTest
@testable import TriOSKit

/// The gate that stops a bee grading its own homework, and the three verdicts
/// it must keep apart.
final class HiveVerifierTests: XCTestCase {

    private func task(realm: String, module: String = "rings/SR-00", branch: String? = nil) -> HiveTask {
        var t = HiveTask(
            id: "t", title: "t", module: module, path: module, realm: realm,
            signalKind: "testGap", reason: "r", score: 1, confidence: 1, prompt: "p"
        )
        t.branch = branch
        return t
    }

    // MARK: - Three outcomes, never two

    func testUnavailableIsNotAPass() {
        let verdict = HiveVerdict.unavailable("no checker")
        XCTAssertFalse(verdict.isPass)
        XCTAssertFalse(verdict.isFail)
        XCTAssertEqual(verdict.label, "UNVERIFIED")
    }

    func testOnlyAFailingCheckCostsTheBeeAnAttempt() {
        XCTAssertTrue(HiveVerdict.failed("build broke").isFail)
        XCTAssertFalse(HiveVerdict.passed("ok").isFail)
        // A missing checker is the Queen's gap, not the bee's fault.
        XCTAssertFalse(HiveVerdict.unavailable("no checker").isFail)
    }

    func testAnUnknownRealmIsUnverifiedNotVerified() {
        let verdict = HiveVerifier(projectRoot: "/tmp").verify(task: task(realm: "Klingon"))
        XCTAssertFalse(verdict.isPass)
        XCTAssertEqual(verdict.label, "UNVERIFIED")
    }

    func testMissingPackageIsUnavailableNotFailed() {
        let verdict = HiveVerifier(projectRoot: "/tmp")
            .verifySwiftPackage(at: "/tmp/not-a-package-\(UUID().uuidString)")
        XCTAssertFalse(verdict.isPass)
        XCTAssertFalse(verdict.isFail)
    }

    func testARustRingWithoutAManifestIsUnavailable() {
        let verdict = HiveVerifier(projectRoot: "/tmp")
            .verifyRustRing(module: "rings/RUST-99", at: "/tmp/nope-\(UUID().uuidString)")
        XCTAssertFalse(verdict.isPass)
        XCTAssertFalse(verdict.isFail)
        XCTAssertTrue(verdict.detail.contains("Cargo.toml"))
    }

    // MARK: - Worktree targeting

    func testATaskWithNoWorktreeVerifiesTheMainCheckout() {
        let verifier = HiveVerifier(projectRoot: "/tmp/repo")
        XCTAssertEqual(verifier.workingRoot(for: task(realm: "Swift")), "/tmp/repo")
    }

    func testAMissingWorktreeRefusesRatherThanGradingTheWrongTree() {
        // The dangerous version returned projectRoot, ran the checks against a
        // tree the bee never touched, and reported a green pass.
        let verifier = HiveVerifier(projectRoot: "/tmp/repo")
        XCTAssertNil(verifier.workingRoot(for: task(realm: "Swift", branch: "hive-nope")))

        let verdict = verifier.verify(task: task(realm: "Swift", branch: "hive-nope"))
        XCTAssertFalse(verdict.isPass)
        XCTAssertTrue(verdict.detail.contains("grade the wrong tree"))
    }

    func testWorktreePathIsFoundByBranchRef() {
        let porcelain = """
        worktree /Users/x/trios-land
        HEAD abc
        branch refs/heads/main

        worktree /Users/x/wt/hive-rings-sr-00
        HEAD def
        branch refs/heads/hive-rings-sr-00
        """
        XCTAssertEqual(
            HiveVerifier.worktreePath(named: "hive-rings-sr-00", in: porcelain),
            "/Users/x/wt/hive-rings-sr-00"
        )
        XCTAssertNil(HiveVerifier.worktreePath(named: "hive-absent", in: porcelain))
    }

    func testWorktreePathIsFoundByDirectoryNameWhenDetached() {
        let porcelain = """
        worktree /Users/x/wt/hive-a
        HEAD abc
        detached
        """
        XCTAssertEqual(HiveVerifier.worktreePath(named: "hive-a", in: porcelain), "/Users/x/wt/hive-a")
    }

    // MARK: - Output shaping

    func testSummaryQuotesTheToolsOwnTally() {
        let swiftOutput = """
        Test Suite 'All tests' started
        Executed 42 tests, with 0 failures
        Test run with 42 tests in 3 suites passed after 0.05 seconds.
        """
        XCTAssertTrue(
            HiveVerifier.testSummary(swiftOutput)?.hasPrefix("Test run with 42 tests") == true
        )
        XCTAssertNil(HiveVerifier.testSummary("nothing here"))
    }

    func testCargoSummaryQuotesCargosOwnTally() {
        let cargoOutput = """
        running 7 tests
        test result: ok. 7 passed; 0 failed; 0 ignored
        """
        XCTAssertEqual(
            HiveVerifier.cargoSummary(cargoOutput),
            "test result: ok. 7 passed; 0 failed; 0 ignored"
        )
        XCTAssertNil(HiveVerifier.cargoSummary("no result line"))
    }

    func testFailureOutputIsTailedNotTruncatedFromTheFront() {
        // The compiler's last lines are the ones that name the error.
        let text = (1...100).map { "line \($0)" }.joined(separator: "\n")
        XCTAssertEqual(HiveVerifier.tail(text, lines: 3), "line 98\nline 99\nline 100")
    }

    func testCargoManifestSearchPrefersTheShallowestManifest() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("hive-cargo-\(UUID().uuidString)")
        let nested = root.appendingPathComponent("crate/src")
        try fm.createDirectory(at: nested, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: root) }

        try "[workspace]".write(
            to: root.appendingPathComponent("Cargo.toml"), atomically: true, encoding: .utf8
        )
        try "[package]".write(
            to: root.appendingPathComponent("crate/Cargo.toml"), atomically: true, encoding: .utf8
        )

        XCTAssertEqual(
            HiveVerifier.findCargoManifest(under: root.path),
            root.appendingPathComponent("Cargo.toml").path
        )
        XCTAssertNil(HiveVerifier.findCargoManifest(under: "/tmp/absent-\(UUID().uuidString)"))
    }
}
