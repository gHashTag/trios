import XCTest
@testable import TriOSKit

final class LocalAuthMonitorTests: XCTestCase {

    private var monitor: LocalAuthMonitor!

    override func setUp() {
        super.setUp()
        clearAuditLog()
        monitor = LocalAuthMonitor()
    }

    override func tearDown() {
        clearAuditLog()
        monitor = nil
        super.tearDown()
    }

    func testInitialStatusIsUnknownAndHealthy() async {
        let (state, meta) = await monitor.status()
        XCTAssertEqual(state, .unknown)
        XCTAssertTrue(meta.isHealthy)
        XCTAssertNil(meta.fetchedAt)
        XCTAssertEqual(meta.refreshCount, 0)
        XCTAssertEqual(meta.retry403Count, 0)
    }

    func testRecordFetchSuccessUpdatesMetadata() async {
        await monitor.recordFetchSuccess()

        let (state, meta) = await monitor.status()
        XCTAssertEqual(state, .cached)
        XCTAssertTrue(meta.isHealthy)
        XCTAssertEqual(meta.refreshCount, 1)
        XCTAssertNotNil(meta.fetchedAt)
    }

    func testRecordFailureMarksUnhealthyAndStoresReason() async {
        await monitor.recordFailure(reason: "http_503")

        let (state, meta) = await monitor.status()
        XCTAssertEqual(state, .failed)
        XCTAssertFalse(meta.isHealthy)
        XCTAssertEqual(meta.lastFailureReason, "http_503")
        XCTAssertNotNil(meta.lastFailureAt)
    }

    func testRecord403RetryIncrementsCounter() async {
        await monitor.record403Retry()
        await monitor.record403Retry()

        let (_, meta) = await monitor.status()
        XCTAssertEqual(meta.retry403Count, 2)
    }

    func testRecordResetClearsAllMetadata() async {
        await monitor.recordFetchSuccess()
        await monitor.record403Retry()
        await monitor.recordFailure(reason: "http_503")

        await monitor.recordReset()

        let (state, meta) = await monitor.status()
        XCTAssertEqual(state, .unknown)
        XCTAssertTrue(meta.isHealthy)
        XCTAssertNil(meta.fetchedAt)
        XCTAssertNil(meta.lastFailureAt)
        XCTAssertNil(meta.lastFailureReason)
        XCTAssertEqual(meta.refreshCount, 0)
        XCTAssertEqual(meta.retry403Count, 0)
    }

    func testRecordFamilyRevokedMarksUnhealthyAndLogsEvent() async throws {
        await monitor.recordFamilyRevoked()

        let (state, meta) = await monitor.status()
        XCTAssertEqual(state, .failed)
        XCTAssertFalse(meta.isHealthy)
        XCTAssertEqual(meta.lastFailureReason, "refresh_family_revoked")
        XCTAssertNotNil(meta.lastFailureAt)

        let content = try await auditContents(containing: "family.revoked")
        XCTAssertTrue(content.contains("family.revoked"))
    }

    func testShouldProactivelyRefreshWhenNeverFetched() async {
        let shouldRefresh = await monitor.shouldProactivelyRefresh(maxAge: 300)
        XCTAssertTrue(shouldRefresh)
    }

    func testShouldProactivelyRefreshWhenStale() async {
        await monitor.recordFetchSuccess()
        // Back-date fetchedAt so the token appears stale.
        let shouldRefresh = await monitor.shouldProactivelyRefresh(maxAge: -1)
        XCTAssertTrue(shouldRefresh)
    }

    func testShouldNotProactivelyRefreshWhenFresh() async {
        await monitor.recordFetchSuccess()
        let shouldRefresh = await monitor.shouldProactivelyRefresh(maxAge: 600)
        XCTAssertFalse(shouldRefresh)
    }

    func testAuditLogDoesNotContainTokenValue() async throws {
        let monitor = LocalAuthMonitor()
        await monitor.recordFetchSuccess()

        let content = try await auditContents(containing: "fetch.success")
        XCTAssertFalse(content.isEmpty)
        XCTAssertFalse(content.contains("secret-token"))
        XCTAssertTrue(content.contains("fetch.success"))
    }

    // MARK: - Audit log helpers

    private var auditURL: URL {
        URL(fileURLWithPath: "\(ProjectPaths.trinity)/state/local-auth-audit.jsonl")
    }

    /// Removes the audit log so a test cannot read another test's writes.
    ///
    /// Clearing only afterwards is not enough: the first run on a clean checkout
    /// behaves differently from every run after it, and a stale file lets an
    /// assertion pass on somebody else's line.
    private func clearAuditLog() {
        try? FileManager.default.removeItem(at: auditURL)
    }

    /// Waits for `marker` to appear in the audit log, then returns the contents.
    ///
    /// `recordFetchSuccess` and friends hand the write to a detached `Task` so
    /// the auth path is never blocked by disk IO, which means they return before
    /// the entry exists. Reading straight after the call is a race: it passed
    /// locally only because an earlier test had already created the file, and
    /// failed on a clean checkout where nothing had.
    ///
    /// Polling for the effect rather than sleeping a fixed interval keeps the
    /// test fast when the write is prompt and still correct when it is not.
    private func auditContents(
        containing marker: String,
        timeout: TimeInterval = 2.0,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async throws -> String {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let content = try? String(contentsOf: auditURL, encoding: .utf8),
               content.contains(marker) {
                return content
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("Audit log never contained '\(marker)' within \(timeout)s", file: file, line: line)
        return ""
    }
}
