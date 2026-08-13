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
    private func writeSiblingState(
        enabled: Bool,
        ceiling: Double,
        spentToday: Double,
        updatedAt: Date = Date(),
        spentOn: Date = Date()
    ) throws {
        var policy = HivePolicy.default
        policy.enabled = enabled
        policy.dailyBudgetUSD = ceiling
        var state = HiveState(policy: policy)
        state.record(spend: spentToday, on: spentOn)
        // The writer stamps this on every persist; a test that wants a corpse
        // writes an old one rather than waiting three cycles for a real loop
        // to fall silent.
        state.updatedAt = updatedAt

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
        // Surfaced, and it charges nothing: a file that could not be read
        // cannot say how much anyone has spent, so it takes nothing off this
        // copy's ceiling and the loop keeps running.
        XCTAssertFalse(runtime.siblingExposureIsBounded)
        XCTAssertFalse(runtime.siblingIsArmed)
        XCTAssertEqual(runtime.combinedExposureUSD, runtime.policy.dailyBudgetUSD)
        XCTAssertEqual(runtime.siblingCommittedUSD, 0)
        XCTAssertEqual(runtime.dispatchContext.siblingCommittedUSD, 0)
    }

    // MARK: - The liveness horizon
    //
    // The debit below is only honoured while the sibling's file proves the
    // process writing it is alive. An armed Hive rewrites its state at the end
    // of every cycle, so its own cycle interval is its heartbeat and silence
    // past a few of them is the evidence that it is gone.

    func testTheHorizonIsThreeOfTheSiblingsOwnCycles() {
        // The default 15-minute cycle: three missed writes is 45 minutes.
        XCTAssertEqual(HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: 900), 2700)
    }

    func testTheHorizonHasAFloorSoOneSlowScanCannotDeclareALiveSiblingDead() {
        // Three 30-second cycles is 90 seconds, which one `git log` sweep can
        // exceed. The floor is what stops that from freeing the ceiling.
        XCTAssertEqual(
            HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: 30),
            HiveSiblingProbe.minimumStalenessHorizon
        )
    }

    func testTheHorizonHasACeilingSoASlowSiblingCannotHoldADebitForDays() {
        XCTAssertEqual(
            HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: 24 * 3600),
            HiveSiblingProbe.maximumStalenessHorizon
        )
    }

    func testASiblingThatRecordsNoCycleIntervalGetsTheDefaultHorizon() {
        XCTAssertEqual(
            HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: nil),
            HiveSiblingProbe.stalenessHorizon(
                cycleIntervalSeconds: HiveSiblingProbe.defaultCycleIntervalSeconds
            )
        )
        // A zero or negative interval is not a heartbeat either.
        XCTAssertEqual(
            HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: 0),
            HiveSiblingProbe.stalenessHorizon(cycleIntervalSeconds: nil)
        )
    }

    // MARK: - Proof of life

    func testAFileWrittenThisMinuteProvesItsWriterIsAlive() throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 6)
        let report = probe().probe()
        XCTAssertTrue(report.freshness.provesLife)
        XCTAssertEqual(report.freshness.label, "FRESH")
    }

    func testAFileOlderThanTheHorizonIsStaleAndStillReportsArmed() throws {
        try writeSiblingState(
            enabled: true, ceiling: 40, spentToday: 6,
            updatedAt: Date(timeIntervalSinceNow: -4 * 3600)
        )
        let report = probe().probe()
        // The document still says armed - what changed is whether anyone is
        // still saying it. Four states, and freshness is a separate axis.
        XCTAssertEqual(report.state, .armed(dailyBudgetUSD: 40, spentToday: 6))
        XCTAssertEqual(report.freshness.label, "STALE")
        XCTAssertFalse(report.freshness.provesLife)
    }

    func testADocumentWithNoTimestampFallsBackToTheFilesModificationDate() throws {
        try write(#"{"policy":{"enabled":true,"dailyBudgetUSD":30}}"#)
        try FileManager.default.setAttributes(
            [.modificationDate: Date(timeIntervalSinceNow: -5 * 3600)],
            ofItemAtPath: stateURL.path
        )
        XCTAssertEqual(probe().probe().freshness.label, "STALE")

        try FileManager.default.setAttributes(
            [.modificationDate: Date()],
            ofItemAtPath: stateURL.path
        )
        XCTAssertEqual(probe().probe().freshness.label, "FRESH")
    }

    func testWithNeitherATimestampNorAModificationDateLifeIsUnestablished() {
        let reading = HiveSiblingProbe.read(
            Data(#"{"policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8),
            modifiedAt: nil
        )
        XCTAssertEqual(reading.freshness.label, "UNDATED")
        XCTAssertFalse(reading.freshness.provesLife)
    }

    func testANumericTimestampIsRefusedRatherThanGuessedAt() {
        // JSONEncoder counts from 2001 by default and from 1970 on request.
        // Picking one would misdate the file by thirty-one years.
        let reading = HiveSiblingProbe.read(
            Data(#"{"updatedAt":1000000,"policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8),
            modifiedAt: nil
        )
        XCTAssertEqual(reading.freshness.label, "UNDATED")
    }

    func testATimestampFarInTheFutureIsNotProofOfLife() {
        let reading = HiveSiblingProbe.read(
            Data(#"{"updatedAt":"2099-01-01T00:00:00Z","policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8),
            modifiedAt: nil
        )
        // Two clocks that disagree by more than the horizon cannot be used to
        // measure it, and "cannot be measured" never charges money.
        XCTAssertEqual(reading.freshness.label, "UNDATED")
    }

    func testAFractionalSecondsTimestampIsRead() {
        let now = Date()
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let reading = HiveSiblingProbe.read(
            Data(#"{"updatedAt":"\#(formatter.string(from: now))","policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8),
            now: now,
            modifiedAt: nil
        )
        XCTAssertTrue(reading.freshness.provesLife)
    }

    // MARK: - The charge: one shared ceiling rather than two

    func testAnArmedLiveSiblingChargesItsCommittedSpendNotItsCeiling() throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 7.5)
        let report = probe().probe()
        // Its ceiling is $40 and it has committed $7.50. Charging the ceiling
        // would be the hard block by another name; charging the committed
        // spend is what makes the two ceilings one.
        XCTAssertEqual(report.committedSpendUSD, 7.5)
        XCTAssertEqual(report.sharedHeadroom(ownCeiling: 25, ownSpentToday: 10), 7.5)
    }

    func testAnArmedSiblingWhoseProcessIsGoneChargesNothing() throws {
        // The requirement in one test: a state file left behind by a dead
        // process must not block this loop for ever. Its ledger is dominated
        // by reservations for bees that are not running.
        try writeSiblingState(
            enabled: true, ceiling: 40, spentToday: 40,
            updatedAt: Date(timeIntervalSinceNow: -4 * 3600)
        )
        let report = probe().probe()
        XCTAssertEqual(report.committedSpendUSD, 0)
        XCTAssertEqual(report.sharedHeadroom(ownCeiling: 25, ownSpentToday: 0), 25)
        XCTAssertTrue(
            report.sharedCeilingNote(ownCeiling: 25).contains("does not prove it is running"),
            report.sharedCeilingNote(ownCeiling: 25)
        )
        XCTAssertTrue(report.sharedCeilingNote(ownCeiling: 25).contains("4h"))
    }

    func testAnUnreadableSiblingChargesNothingAndSaysTheCeilingIsNotShared() throws {
        try write("{ this is not json")
        let report = probe().probe()
        XCTAssertEqual(report.committedSpendUSD, 0)
        XCTAssertTrue(
            report.sharedCeilingNote(ownCeiling: 25).contains("could not be read"),
            report.sharedCeilingNote(ownCeiling: 25)
        )
    }

    func testADisarmedSiblingChargesNothingEvenWithSpendRecordedToday() throws {
        // A disarmed loop is not racing anyone to the ceiling. Its earlier
        // spend today is real and is deliberately not charged: this copy
        // cannot tell settled spend from an abandoned reservation in a ledger
        // nobody is maintaining, and the shared ceiling is a bound on joint
        // dispatch, not a retrospective audit.
        try writeSiblingState(enabled: false, ceiling: 40, spentToday: 9)
        let report = probe().probe()
        XCTAssertEqual(report.recordedSpendToday, 9)
        XCTAssertEqual(report.committedSpendUSD, 0)
    }

    func testAnAbsentSiblingChargesNothing() {
        let report = probe().probe()
        XCTAssertEqual(report.state, .absent)
        XCTAssertEqual(report.committedSpendUSD, 0)
        XCTAssertNil(report.recordedSpendToday)
    }

    func testAnArmedSiblingWithNoLedgerChargesNothingRatherThanAnInventedNumber() {
        let reading = HiveSiblingProbe.read(
            Data(#"{"policy":{"enabled":true,"dailyBudgetUSD":30}}"#.utf8),
            modifiedAt: Date()
        )
        let report = HiveSiblingReport(
            path: "/p",
            state: reading.state,
            freshness: reading.freshness,
            recordedSpendToday: reading.recordedSpendToday
        )
        XCTAssertTrue(report.freshness.provesLife)
        XCTAssertNil(report.recordedSpendToday)
        XCTAssertEqual(report.committedSpendUSD, 0)
        XCTAssertTrue(
            report.sharedCeilingNote(ownCeiling: 25).contains("records no ledger"),
            report.sharedCeilingNote(ownCeiling: 25)
        )
    }

    func testASiblingLedgerEntryFromYesterdayIsNotChargedToday() throws {
        try writeSiblingState(
            enabled: true, ceiling: 40, spentToday: 20,
            spentOn: Date(timeIntervalSinceNow: -86_400)
        )
        let report = probe().probe()
        // The ceiling is a daily one and the ledger is keyed by local day, so
        // yesterday's spend leaves today's headroom whole.
        XCTAssertEqual(report.recordedSpendToday, 0)
        XCTAssertEqual(report.committedSpendUSD, 0)
    }

    func testTheChargeCannotBeNegative() {
        let report = HiveSiblingReport(
            path: "/p",
            state: .armed(dailyBudgetUSD: 40, spentToday: -5),
            freshness: .fresh(age: 1, horizon: 2700),
            recordedSpendToday: -5
        )
        XCTAssertEqual(report.committedSpendUSD, 0)
    }

    func testTheArmedSummaryCarriesTheChargeSoTheOperatorSeesTheSharedCeiling() throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 7.5)
        let text = probe().probe().summary(ownCeiling: 25, ownArmed: true)
        XCTAssertTrue(text.contains("$7.50 of it is charged"), text)
        XCTAssertTrue(text.contains("leaving $17.50"), text)
    }

    func testDurationsAreWrittenInHoursAndMinutes() {
        XCTAssertEqual(HiveSiblingReport.duration(45), "45s")
        XCTAssertEqual(HiveSiblingReport.duration(2700), "45m")
        XCTAssertEqual(HiveSiblingReport.duration(7200), "2h")
        XCTAssertEqual(HiveSiblingReport.duration(4 * 3600 + 12 * 60), "4h 12m")
    }

    // MARK: - Runtime wiring
    //
    // A proof about `HiveDispatch` stops being a proof about the loop at the
    // line that fills its context, so these look at exactly what this copy
    // hands the decision.

    @MainActor
    func testTheRuntimeChargesAnArmedLiveSiblingAgainstItsOwnCeiling() async throws {
        try writeSiblingState(enabled: true, ceiling: 40, spentToday: 12)
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()

        XCTAssertEqual(runtime.siblingCommittedUSD, 12)
        XCTAssertEqual(runtime.dispatchContext.siblingCommittedUSD, 12)
        XCTAssertEqual(runtime.sharedHeadroomUSD, runtime.policy.dailyBudgetUSD - 12)
    }

    @MainActor
    func testTheRuntimeChargesNothingForASiblingWhoseProcessIsGone() async throws {
        try writeSiblingState(
            enabled: true, ceiling: 40, spentToday: 40,
            updatedAt: Date(timeIntervalSinceNow: -4 * 3600)
        )
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()

        XCTAssertTrue(runtime.siblingIsArmed)
        XCTAssertEqual(runtime.siblingCommittedUSD, 0)
        XCTAssertEqual(runtime.sharedHeadroomUSD, runtime.policy.dailyBudgetUSD)
    }

    @MainActor
    func testASiblingPastTheSharedCeilingBlocksThisCopysNextDispatch() async throws {
        // The sibling has committed more than this copy's whole ceiling, so
        // the pair is over it before this copy has spent a cent.
        try writeSiblingState(enabled: true, ceiling: 100, spentToday: 30)
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()

        var context = runtime.dispatchContext
        context.policy.enabled = true
        context.policy.dailyBudgetUSD = 25
        context.auth = .loggedIn(method: "oauth")
        context.tasks = [
            {
                var t = HiveTask(
                    id: "t", title: "t", module: "rings/SR-00", path: "rings/SR-00",
                    realm: "Swift", signalKind: "testGap", reason: "r",
                    score: 1, confidence: 1, prompt: "p"
                )
                t.state = .pending
                return t
            }(),
        ]

        let decision = HiveDispatch.decide(context)
        XCTAssertFalse(decision.isDispatch)
        XCTAssertTrue(decision.reason.contains("shared daily ceiling reached"), decision.reason)
    }

    @MainActor
    func testTheSameCopyDispatchesOnceTheSiblingIsPresumedGone() async throws {
        // Identical to the test above except for the age of the file. The
        // dead process must not hold this loop shut.
        try writeSiblingState(
            enabled: true, ceiling: 100, spentToday: 30,
            updatedAt: Date(timeIntervalSinceNow: -4 * 3600)
        )
        let runtime = HiveRuntime(
            store: HiveStore(stateRoot: root.appendingPathComponent("own").path),
            siblingProbe: probe()
        )
        await runtime.refreshSibling()

        var context = runtime.dispatchContext
        context.policy.enabled = true
        context.policy.dailyBudgetUSD = 25
        context.auth = .loggedIn(method: "oauth")
        context.tasks = [
            {
                var t = HiveTask(
                    id: "t", title: "t", module: "rings/SR-00", path: "rings/SR-00",
                    realm: "Swift", signalKind: "testGap", reason: "r",
                    score: 1, confidence: 1, prompt: "p"
                )
                t.state = .pending
                return t
            }(),
        ]

        XCTAssertTrue(HiveDispatch.decide(context).isDispatch)
    }
}
