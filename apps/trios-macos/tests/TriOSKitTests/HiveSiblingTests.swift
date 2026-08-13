import XCTest
@testable import TriOSKit

/// Sibling budget awareness. Two copies of the Hive run on this machine with
/// independent daily ceilings, so the number in either window is a share and
/// not a total. These tests hold the probe to four states rather than two -
/// the reading that matters is that a state file which could not be read is
/// never reported as a state file that is not there.
final class HiveSiblingTests: XCTestCase {

    private var root: URL!

    override func setUpWithError() throws {
        root = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-sibling-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: root)
    }

    // MARK: - Helpers

    /// A probe pointed at the temporary root through the same env var the
    /// sibling itself honours.
    private func probe() -> HiveSiblingProbe {
        HiveSiblingProbe(environment: ["TRINITY_ROOT": root.path, "HOME": "/nonexistent"])
    }

    private var stateURL: URL {
        root.appendingPathComponent(HiveSiblingProbe.stateSuffix)
    }

    private func write(_ text: String) throws {
        try FileManager.default.createDirectory(
            at: stateURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try text.write(to: stateURL, atomically: true, encoding: .utf8)
    }

    /// A document in the shape the other copy actually writes: the same
    /// `HiveState` encoder, so the probe is tested against the real format
    /// rather than against a hand-written approximation of it.
    private func writeSiblingState(enabled: Bool, ceiling: Double, spentToday: Double) throws {
        var policy = HivePolicy.default
        policy.enabled = enabled
        policy.dailyBudgetUSD = ceiling
        var state = HiveState(policy: policy)
        state.record(spend: spentToday)

        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try FileManager.default.createDirectory(
            at: stateURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try encoder.encode(state).write(to: stateURL, options: .atomic)
    }

    // MARK: - Location

    func testTrinityRootIsPreferredOverTheHomeConvention() {
        let probe = HiveSiblingProbe(environment: ["TRINITY_ROOT": "/opt/tri", "HOME": "/Users/someone"])
        XCTAssertEqual(probe.statePath, "/opt/tri/.trinity/queen/hive.json")
    }

    func testWithoutTrinityRootTheHomeConventionIsUsed() {
        let probe = HiveSiblingProbe(environment: ["HOME": "/Users/someone"])
        XCTAssertEqual(probe.statePath, "/Users/someone/trinity/.trinity/queen/hive.json")
    }

    func testAnEmptyTrinityRootIsTreatedAsUnset() {
        let probe = HiveSiblingProbe(environment: ["TRINITY_ROOT": "", "HOME": "/Users/someone"])
        XCTAssertEqual(probe.statePath, "/Users/someone/trinity/.trinity/queen/hive.json")
    }

    func testWithNoEnvironmentAtAllTheSiblingIsUnlocatableNotAbsent() {
        // Nowhere to look is not the same as having looked and found nothing.
        let report = HiveSiblingProbe(environment: [:]).probe()
        XCTAssertNil(report.path)
        guard case .unreadable = report.state else {
            return XCTFail("expected .unreadable, got \(report.state)")
        }
    }

    // MARK: - State one: absent

    func testNoFileAtThePathIsAbsent() {
        let report = probe().probe()
        XCTAssertEqual(report.state, .absent)
        XCTAssertEqual(report.path, stateURL.path)
        XCTAssertTrue(report.exposureIsBounded)
        XCTAssertEqual(report.combinedExposure(ownCeiling: 25), 25)
    }

    // MARK: - State two: present and disarmed

    func testAPresentDisarmedSiblingCarriesItsCeilingButAddsNothing() throws {
        try writeSiblingState(enabled: false, ceiling: 40, spentToday: 3)
        let report = probe().probe()
        XCTAssertEqual(report.state, .disarmed(dailyBudgetUSD: 40))
        XCTAssertFalse(report.state.isArmed)
        XCTAssertNil(report.state.armedCeiling)
        // Disarmed spends nothing today, so today's exposure is this copy's.
        XCTAssertEqual(report.combinedExposure(ownCeiling: 25), 25)
        // The operator is still told what arming it would cost.
        XCTAssertTrue(report.summary(ownCeiling: 25, ownArmed: true).contains("$65.00"))
    }

    // MARK: - State three: present and armed

    func testAnArmedSiblingReportsItsCeilingAndItsSpendToday() throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 7.5)
        let report = probe().probe()
        XCTAssertEqual(report.state, .armed(dailyBudgetUSD: 40, spentToday: 7.5))
        XCTAssertTrue(report.state.isArmed)
        XCTAssertEqual(report.state.armedCeiling, 40)
    }

    func testCombinedExposureIsBothCeilingsWhenTheSiblingIsArmed() throws {
        try writeSiblingState(enabled: true, ceiling: 25, spentToday: 0)
        let report = probe().probe()
        // The whole finding: a $25 ceiling entered in each window is $50.
        XCTAssertEqual(report.combinedExposure(ownCeiling: 25), 50)
        let text = report.summary(ownCeiling: 25, ownArmed: true)
        XCTAssertTrue(text.contains("Both Hives are armed"), text)
        XCTAssertTrue(text.contains("Combined exposure $50.00"), text)
    }

    func testAnArmedSiblingIsReportedEvenWhileThisCopyIsIdle() throws {
        try writeSiblingState(enabled: true, ceiling: 12, spentToday: 0)
        let text = probe().probe().summary(ownCeiling: 25, ownArmed: false)
        XCTAssertTrue(text.contains("while this copy is idle"), text)
    }

    func testADayWithNoEntryInTheLedgerIsZeroSpendNotUnknown() throws {
        // The writer encodes an unspent day by omitting the key, so this one
        // absence really is a zero - and it is the only one that is.
        try writeSiblingState(enabled: true, ceiling: 30, spentToday: 0)
        XCTAssertEqual(probe().probe().state, .armed(dailyBudgetUSD: 30, spentToday: 0))
    }

    func testAnArmedSiblingWithNoLedgerAtAllHasUnknownSpendNotZero() {
        let state = HiveSiblingProbe.interpret(
            Data(#"{"policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8)
        )
        XCTAssertEqual(state, .armed(dailyBudgetUSD: 30, spentToday: nil))
        XCTAssertTrue(
            HiveSiblingReport(path: "/p", state: state)
                .summary(ownCeiling: 25, ownArmed: true)
                .contains("today's spend not recorded")
        )
    }

    // MARK: - State four: unreadable, which is not absent

    func testAFileThatCannotBeOpenedIsUnreadableNotAbsent() throws {
        try write("{}")
        try FileManager.default.setAttributes([.posixPermissions: 0], ofItemAtPath: stateURL.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: stateURL.path) }
        // Running as root would defeat the permission bits and make this test
        // measure nothing; skip rather than pass on a false reading.
        try XCTSkipIf(getuid() == 0, "root can read a mode-000 file")

        let report = probe().probe()
        XCTAssertNotEqual(report.state, .absent)
        guard case .unreadable = report.state else {
            return XCTFail("expected .unreadable, got \(report.state)")
        }
    }

    func testADirectoryWhereTheStateFileShouldBeIsUnreadableNotAbsent() throws {
        try FileManager.default.createDirectory(at: stateURL, withIntermediateDirectories: true)
        let report = probe().probe()
        XCTAssertNotEqual(report.state, .absent)
        XCTAssertEqual(report.state.label, "UNREADABLE")
    }

    func testMalformedJSONIsUnreadableNotAbsent() throws {
        try write("{ this is not json")
        let report = probe().probe()
        XCTAssertNotEqual(report.state, .absent)
        XCTAssertEqual(report.state.label, "UNREADABLE")
    }

    func testADocumentWithNoPolicyBlockIsUnreadableRatherThanDefaulted() {
        // HiveState's own decoder would turn this into a disarmed hive with a
        // $25 ceiling and report it with confidence. Reading someone else's
        // file, a missing key has to stay missing.
        let state = HiveSiblingProbe.interpret(Data(#"{"tasks":[]}"#.utf8))
        XCTAssertEqual(state.label, "UNREADABLE")
    }

    func testAPolicyWithNoEnabledFlagIsUnreadableRatherThanDisarmed() {
        // Guessing "disarmed" is guessing in the direction that spends money.
        let state = HiveSiblingProbe.interpret(Data(#"{"policy":{"dailyBudgetUSD":25}}"#.utf8))
        XCTAssertEqual(state.label, "UNREADABLE")
        XCTAssertFalse(state.isArmed)
    }

    func testAnArmedPolicyWithNoCeilingIsUnreadableRatherThanUnbudgeted() {
        let state = HiveSiblingProbe.interpret(Data(#"{"policy":{"enabled":true}}"#.utf8))
        XCTAssertEqual(state.label, "UNREADABLE")
    }

    func testAnUnreadableSiblingMakesTheCombinedFigureAFloorNotACeiling() throws {
        try write("{ this is not json")
        let report = probe().probe()
        XCTAssertFalse(report.exposureIsBounded)
        // It still contributes nothing arithmetically - a guessed ceiling
        // would be reported as if it had been measured - but the caller is
        // told the total is not bounded.
        XCTAssertEqual(report.combinedExposure(ownCeiling: 25), 25)
        XCTAssertTrue(report.summary(ownCeiling: 25, ownArmed: true).contains("at least $25.00"))
    }

    // MARK: - Four states, and only four

    func testExactlyOneOfTheFourStatesRaisesTheExposure() {
        let states: [HiveSiblingState] = [
            .absent,
            .disarmed(dailyBudgetUSD: 40),
            .armed(dailyBudgetUSD: 40, spentToday: 1),
            .unreadable("no"),
        ]
        XCTAssertEqual(states.filter(\.isArmed).count, 1)
        XCTAssertEqual(Set(states.map(\.label)).count, 4)
        XCTAssertEqual(
            states.map { HiveSiblingReport(path: "/p", state: $0).combinedExposure(ownCeiling: 25) },
            [25, 25, 65, 25]
        )
    }

    func testOnlyTheUnreadableStateLeavesTheExposureUnbounded() {
        let states: [HiveSiblingState] = [
            .absent,
            .disarmed(dailyBudgetUSD: 40),
            .armed(dailyBudgetUSD: 40, spentToday: 1),
            .unreadable("no"),
        ]
        let bounded = states.filter { HiveSiblingReport(path: "/p", state: $0).exposureIsBounded }
        XCTAssertEqual(bounded.count, 3)
    }

    func testEveryStateProducesASentenceNamingWhereItLooked() {
        let states: [HiveSiblingState] = [
            .absent,
            .disarmed(dailyBudgetUSD: 40),
            .armed(dailyBudgetUSD: 40, spentToday: 1),
            .unreadable("no"),
        ]
        for state in states {
            let text = HiveSiblingReport(path: "/some/where/hive.json", state: state)
                .summary(ownCeiling: 25, ownArmed: true)
            XCTAssertTrue(text.contains("/some/where/hive.json"), "\(state.label): \(text)")
        }
    }

    // MARK: - Runtime wiring

    @MainActor
    func testTheRuntimeHasNoSiblingReadingUntilItHasProbed() {
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        // nil is "not looked yet", which is not "there is none".
        XCTAssertNil(runtime.sibling)
        XCTAssertTrue(runtime.siblingExposureIsBounded)
        XCTAssertEqual(runtime.combinedExposureUSD, runtime.policy.dailyBudgetUSD)
    }

    @MainActor
    func testTheRuntimeRaisesItsExposureOnceItSeesAnArmedSibling() async throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 2)
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()

        XCTAssertTrue(runtime.siblingIsArmed)
        XCTAssertEqual(runtime.sibling?.state, .armed(dailyBudgetUSD: 40, spentToday: 2))
        XCTAssertEqual(runtime.combinedExposureUSD, runtime.policy.dailyBudgetUSD + 40)
        XCTAssertTrue(runtime.events.contains { $0.kind == "sibling_hive" })
    }

    @MainActor
    func testTheRuntimeLogsTheSiblingOnceRatherThanEveryCycle() async throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 0)
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()
        await runtime.refreshSibling()
        await runtime.refreshSibling()
        XCTAssertEqual(runtime.events.filter { $0.kind == "sibling_hive" }.count, 1)

        // A change in the sibling's own state is the thing worth a line.
        try writeSiblingState(enabled: false, ceiling: 40, spentToday: 0)
        await runtime.refreshSibling()
        XCTAssertEqual(runtime.events.filter { $0.kind == "sibling_hive" }.count, 2)
    }

    @MainActor
    func testAnUnreadableSiblingDoesNotBlockTheRuntime() async throws {
        try write("{ not json")
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()
        // Surfaced, never a gate: the dispatch decision has no input from here.
        XCTAssertFalse(runtime.siblingExposureIsBounded)
        XCTAssertFalse(runtime.siblingIsArmed)
        XCTAssertEqual(runtime.combinedExposureUSD, runtime.policy.dailyBudgetUSD)
    }
}
