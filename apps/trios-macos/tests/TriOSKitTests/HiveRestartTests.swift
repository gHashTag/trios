import XCTest
@testable import TriOSKit

// ===========================================================================
// THE RESTART TRAP.
//
// Every claim the Hive makes about rates, money and progress is a claim about
// a process. A relaunch is not an exceptional event on this machine - a
// watchdog restores the app within 60s of it dying - so any bound that lives
// only in memory is a bound the machine does not have.
//
// These tests are written against a state file, not against an object, so they
// fail if the persistence is removed rather than if the arithmetic changes.
// ===========================================================================

private func pendingTask(_ id: String = "hive-demo-testGap", score: Double = 0.9) -> HiveTask {
    HiveTask(
        id: id, title: id, module: "demo", path: "demo", realm: "Swift",
        signalKind: "testGap", reason: "r", score: score, confidence: 1, prompt: "p"
    )
}

private func temporaryRoot() -> URL {
    FileManager.default.temporaryDirectory
        .appendingPathComponent("hive-tests-\(UUID().uuidString)", isDirectory: true)
}

final class HiveSpawnWindowPersistenceTests: XCTestCase {

    /// The headline claim: the hourly bound composes across process
    /// lifetimes. Delete the persistence and this test dispatches a seventh
    /// bee in the same hour.
    func testTheHourlyBoundHoldsAcrossARestart() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var policy = HivePolicy.default
        policy.enabled = true
        policy.maxBeesPerHour = 6

        // Six bees went out over the last ten minutes, then the process died.
        var limiter = HiveRateLimiter()
        let now = Date()
        for minute in 0..<6 { limiter.record(now.addingTimeInterval(-Double(minute) * 60)) }
        XCTAssertEqual(limiter.spawnsInLastHour(now), 6)
        XCTAssertTrue(
            store.save(
                HiveState(policy: policy, tasks: [pendingTask()], spawnWindow: limiter.spawnTimes)
            )
        )

        // The watchdog relaunches the app within the minute.
        let reloaded = store.load()
        let restarted = HiveRateLimiter(spawnTimes: reloaded.spawnWindow, now: now)
        XCTAssertEqual(restarted.spawnsInLastHour(now), 6)

        let decision = HiveDispatch.decide(
            HiveDispatchContext(
                policy: policy,
                tasks: reloaded.tasks,
                liveBees: 0,
                spentToday: 0,
                spawnsInLastHour: restarted.spawnsInLastHour(now),
                consecutiveFailures: 0,
                auth: .loggedIn(method: "oauth")
            )
        )
        XCTAssertFalse(decision.isDispatch)
        XCTAssertTrue(decision.reason.contains("already started this hour"))
    }

    /// The reason a persisted window must be pruned on load rather than
    /// trusted: a file written on Monday would otherwise hand Thursday's
    /// limiter six spawns that never happened this hour.
    func testAWindowFromThreeDaysAgoCannotBeReplayed() {
        let now = Date()
        let window = [
            now.addingTimeInterval(-3 * 86_400),
            now.addingTimeInterval(-7200),
            now.addingTimeInterval(-3601),
            now.addingTimeInterval(-1800),
            now.addingTimeInterval(-60),
        ]
        let pruned = HiveState.prunedSpawnWindow(window, now: now)
        XCTAssertEqual(pruned.count, 2)
        XCTAssertEqual(HiveRateLimiter(spawnTimes: pruned, now: now).spawnsInLastHour(now), 2)
    }

    /// A clock that moved backwards must not leave entries that never expire.
    func testAWindowFromTheFutureIsDropped() {
        let now = Date()
        XCTAssertTrue(HiveState.prunedSpawnWindow([now.addingTimeInterval(600)], now: now).isEmpty)
    }

    /// Pruning happens at the one place every load passes through, so nothing
    /// downstream has to remember to do it.
    func testTheStorePrunesTheWindowOnLoad() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)
        let now = Date()

        let stale = HiveState(
            policy: .default,
            tasks: [],
            spawnWindow: [now.addingTimeInterval(-86_400), now.addingTimeInterval(-120)]
        )
        XCTAssertTrue(store.save(stale))
        XCTAssertEqual(store.load().spawnWindow.count, 1)
    }

    /// The key is really written. A round trip that passes because both sides
    /// forgot the field would be no evidence at all.
    func testTheWindowIsNamedInTheStateFile() throws {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)
        XCTAssertTrue(store.save(HiveState(policy: .default, tasks: [], spawnWindow: [Date()])))

        let text = try String(contentsOf: store.stateURL, encoding: .utf8)
        XCTAssertTrue(text.contains("spawnWindow"))
    }

    /// A state file written before the window was persisted must still load.
    /// Throwing here would silently hand the operator a default policy, which
    /// reads exactly like they never configured anything.
    func testAnOlderStateFileWithoutTheWindowStillLoads() throws {
        let json = """
        {
          "policy": { "enabled": true, "maxBeesPerHour": 4 },
          "tasks": [],
          "spendByDay": { "2026-08-13": 3.5 }
        }
        """
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let state = try decoder.decode(HiveState.self, from: Data(json.utf8))
        XCTAssertTrue(state.policy.enabled)
        XCTAssertEqual(state.policy.maxBeesPerHour, 4)
        XCTAssertTrue(state.spawnWindow.isEmpty)
        XCTAssertTrue(state.reservations.isEmpty)
    }

    /// The limiter seeded from disk prunes what it was handed, so a caller
    /// that skips `HiveStore.load` cannot smuggle an old window past it.
    func testTheLimiterPrunesWhatItIsSeededWith() {
        let now = Date()
        let limiter = HiveRateLimiter(
            spawnTimes: [now.addingTimeInterval(-7200), now.addingTimeInterval(-30)],
            now: now
        )
        XCTAssertEqual(limiter.spawnTimes.count, 1)
    }
}

/// The runtime's own wiring, driven through a temporary state root.
///
/// The tests above prove the state file can carry a window and that the
/// limiter honours one. These prove the running object actually reads it and
/// writes it back - the two lines a later refactor is most likely to drop,
/// and which no assertion about `HiveState` alone would notice.
@MainActor
final class HiveRuntimeRestartTests: XCTestCase {

    func testTheWindowIsSeededOnLoadAndWrittenBackOnPersist() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var policy = HivePolicy.default
        policy.maxBeesPerHour = 6
        let now = Date()
        XCTAssertTrue(
            store.save(
                HiveState(
                    policy: policy,
                    tasks: [],
                    spawnWindow: [now.addingTimeInterval(-120), now.addingTimeInterval(-60)]
                )
            )
        )

        let hive = HiveRuntime(store: store)
        // Seeded: the limiter inherited this hour's real spawns.
        XCTAssertEqual(hive.spawnsThisHour, 2)

        // Written back: any persist keeps the window in the file.
        hive.updatePolicy(policy)
        XCTAssertEqual(store.load().spawnWindow.count, 2)
    }

    /// The critical one. A file that says armed, with no clock behind it,
    /// must say so in the audit log and in the badge.
    func testArmedInTheFileWithNoClockAsksForAResume() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        var policy = HivePolicy.default
        policy.enabled = true
        XCTAssertTrue(store.save(HiveState(policy: policy, tasks: [pendingTask()])))

        let hive = HiveRuntime(store: store)
        XCTAssertTrue(hive.policy.enabled)
        XCTAssertEqual(hive.loopStatus, .resumeRequired)
        XCTAssertFalse(hive.loopStatus.isTicking)
        XCTAssertTrue(hive.events.contains { $0.kind == "hive_resume_required" })
    }

    /// A disarmed file says nothing and asks for nothing.
    func testADisarmedFileIsSilent() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)
        XCTAssertTrue(store.save(HiveState(policy: .default, tasks: [pendingTask()])))

        let hive = HiveRuntime(store: store)
        XCTAssertEqual(hive.loopStatus, .idle)
        XCTAssertFalse(hive.events.contains { $0.kind == "hive_resume_required" })
    }

    /// Three restarts with two bees in flight each would otherwise leave $30
    /// charged against a $25 ceiling for about $2.40 of real work, with no
    /// breaker tripped and nothing on screen saying it was never spent.
    func testOrphanedReservationsAreSettledOnLoadNotInherited() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        let day = HiveState.dayKey()
        var state = HiveState(
            policy: .default,
            tasks: [],
            spendByDay: [day: 30],
            reservations: [
                // No transcript exists for either session, and no pid, so
                // both bees provably wrote nothing.
                "a": HiveReservation(taskID: "a", sessionID: UUID().uuidString, day: day, amount: 5, pid: nil),
                "b": HiveReservation(taskID: "b", sessionID: UUID().uuidString, day: day, amount: 5, pid: nil),
            ]
        )
        state.updatedAt = Date()
        XCTAssertTrue(store.save(state))

        let hive = HiveRuntime(store: store)
        XCTAssertEqual(hive.spentToday, 20)
        XCTAssertEqual(hive.events.filter { $0.kind == "reservation_refunded" }.count, 2)
        // Settled reservations do not survive into the new process, so they
        // cannot block a fresh bee on the same task for ever.
        XCTAssertTrue(store.load().reservations.isEmpty)
    }
}

final class HiveLoopStatusTests: XCTestCase {

    /// `policy.enabled` is a persisted wish. Only a live timer is a clock.
    func testArmedWithoutAClockIsItsOwnState() {
        XCTAssertEqual(HiveLoopStatus.of(enabled: false, ticking: false), .idle)
        XCTAssertEqual(HiveLoopStatus.of(enabled: true, ticking: true), .ticking)
        XCTAssertEqual(HiveLoopStatus.of(enabled: true, ticking: false), .resumeRequired)
        // A timer with the loop disarmed is still not a running loop.
        XCTAssertEqual(HiveLoopStatus.of(enabled: false, ticking: true), .idle)
    }

    /// The badge, the countdown and the Run button all read this. Only one of
    /// the three states may claim the loop is running.
    func testOnlyTheTickingStateReadsAsRunning() {
        XCTAssertTrue(HiveLoopStatus.ticking.isTicking)
        XCTAssertFalse(HiveLoopStatus.resumeRequired.isTicking)
        XCTAssertFalse(HiveLoopStatus.idle.isTicking)
        XCTAssertTrue(HiveLoopStatus.resumeRequired.needsOperator)
        XCTAssertNotNil(HiveLoopStatus.resumeRequired.advice)
        XCTAssertNil(HiveLoopStatus.ticking.advice)
        XCTAssertNil(HiveLoopStatus.idle.advice)
        // The three labels are distinct, so no state can be mistaken for
        // another on screen.
        XCTAssertEqual(Set([HiveLoopStatus.idle, .ticking, .resumeRequired].map(\.label)).count, 3)
    }
}

final class HiveCycleLatchTests: XCTestCase {

    func testASecondCycleIsDroppedWhileTheFirstRuns() {
        var latch = HiveCycleLatch()
        let start = Date()
        let first = latch.enter(now: start, deadline: 3600)
        XCTAssertNotNil(first.token)
        XCTAssertTrue(latch.isInFlight)

        let second = latch.enter(now: start.addingTimeInterval(5), deadline: 3600)
        XCTAssertNil(second.token)
        XCTAssertEqual(second, .busy(heldFor: 5))
    }

    /// The defect this replaces: a cycle that wedged on a subprocess ignoring
    /// SIGTERM held the loop for ever, and `Cycle now` did nothing at all.
    func testAWedgedCycleIsForcedOpen() {
        var latch = HiveCycleLatch()
        let start = Date()
        _ = latch.enter(now: start, deadline: 3600)

        let later = latch.enter(now: start.addingTimeInterval(3601), deadline: 3600)
        guard case .forced(_, let heldFor) = later else {
            return XCTFail("a cycle held past its deadline must be forced open")
        }
        XCTAssertGreaterThanOrEqual(heldFor, 3600)
        XCTAssertNotNil(later.token)
    }

    /// Forcing the latch open creates two claimants. The wedged cycle, if it
    /// ever returns, must not release a latch the new one holds - otherwise a
    /// third cycle runs concurrently with the second.
    func testAForcedOutCycleCannotReleaseTheLatchItLost() throws {
        var latch = HiveCycleLatch()
        let start = Date()
        let wedged = try XCTUnwrap(latch.enter(now: start, deadline: 60).token)
        let rescuer = try XCTUnwrap(latch.enter(now: start.addingTimeInterval(120), deadline: 60).token)

        latch.leave(token: wedged)
        XCTAssertTrue(latch.isInFlight)

        latch.leave(token: rescuer)
        XCTAssertFalse(latch.isInFlight)
    }

    /// The deadline is derived from the loop's own interval and floored at an
    /// hour, so a slow scan is never mistaken for a wedge.
    func testTheDeadlineIsAtLeastAnHourAndTwoIntervals() {
        XCTAssertEqual(HiveCycleLatch.wedgeDeadline(cycleIntervalSeconds: 30), 3600)
        XCTAssertEqual(HiveCycleLatch.wedgeDeadline(cycleIntervalSeconds: 900), 3600)
        XCTAssertEqual(HiveCycleLatch.wedgeDeadline(cycleIntervalSeconds: 7200), 14400)
    }
}

final class HiveSpawnAdmissionTests: XCTestCase {

    func testATaskWithALiveBeeRefusesASecondOne() {
        let refusal = HiveDispatch.admissionRefusal(
            taskID: "hive-apps-queen-testGap",
            liveTaskIDs: ["hive-apps-queen-testGap"],
            reservedTaskIDs: []
        )
        XCTAssertTrue(refusal?.contains("already working") == true)
    }

    /// A reservation outlives its bee's process. Admitting a second bee while
    /// one is outstanding is how ten bees went out against a two-bee limit.
    func testAnUnsettledReservationAlsoRefuses() {
        let refusal = HiveDispatch.admissionRefusal(
            taskID: "t", liveTaskIDs: [], reservedTaskIDs: ["t"]
        )
        XCTAssertTrue(refusal?.contains("reservation") == true)
    }

    func testAnIdleTaskIsAdmitted() {
        XCTAssertNil(
            HiveDispatch.admissionRefusal(
                taskID: "t", liveTaskIDs: ["other"], reservedTaskIDs: ["another"]
            )
        )
    }

    /// An operator's button crosses the same envelope as the cycle: it chooses
    /// which task runs, not whether the ceiling applies.
    func testTheOperatorPathMeetsTheSameGuardrails() {
        var policy = HivePolicy.default
        policy.enabled = false          // the loop is paused; the human is not
        policy.dailyBudgetUSD = 25

        let refusal = HiveDispatch.guardrails(
            HiveDispatchContext(
                policy: policy,
                tasks: [pendingTask()],
                liveBees: 0,
                spentToday: 25,
                spawnsInLastHour: 0,
                consecutiveFailures: 0,
                auth: .loggedIn(method: "oauth")
            )
        )
        XCTAssertTrue(refusal?.isBlocked == true)
        XCTAssertTrue(refusal?.reason.contains("daily ceiling reached") == true)
    }

    /// A clear envelope returns nothing at all, so the caller can dispatch.
    func testAClearEnvelopeRefusesNothing() {
        var policy = HivePolicy.default
        policy.enabled = true
        XCTAssertNil(
            HiveDispatch.guardrails(
                HiveDispatchContext(
                    policy: policy,
                    tasks: [pendingTask()],
                    liveBees: 0,
                    spentToday: 0,
                    spawnsInLastHour: 0,
                    consecutiveFailures: 0,
                    auth: .loggedIn(method: "oauth")
                )
            )
        )
    }
}

final class HiveReservationRepairTests: XCTestCase {

    func testAReservationIsWrittenToTheStateFile() {
        let root = temporaryRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let store = HiveStore(stateRoot: root.path)

        let reservation = HiveReservation(
            taskID: "t", sessionID: "s", day: HiveState.dayKey(), amount: 5, pid: 4242
        )
        XCTAssertTrue(
            store.save(HiveState(policy: .default, tasks: [], reservations: ["t": reservation]))
        )
        // Compared field by field: the ISO8601 encoding the store uses drops
        // sub-second precision, so whole-struct equality would fail on the
        // timestamp while every fact that matters round-tripped correctly.
        let reloaded = store.load().reservations["t"]
        XCTAssertEqual(reloaded?.taskID, "t")
        XCTAssertEqual(reloaded?.sessionID, "s")
        XCTAssertEqual(reloaded?.day, HiveState.dayKey())
        XCTAssertEqual(reloaded?.amount, 5)
        XCTAssertEqual(reloaded?.pid, 4242)
    }

    /// A bee that wrote nothing never ran, and that is the one absence which
    /// may honestly be read as a zero.
    func testABeeThatWroteNothingIsRefundedInFull() {
        XCTAssertEqual(HiveDispatch.repair(.absent), .refundInFull)
        XCTAssertEqual(
            HiveDispatch.repair(HiveTranscriptSummary(lines: 0, lastCostUSD: nil)), .refundInFull
        )
    }

    /// Real output and no cost line: what it spent is unknown, and unknown is
    /// not zero. This is the case the conservative rule was written for.
    func testOutputWithoutACostLineKeepsTheReservation() {
        XCTAssertEqual(HiveDispatch.repair(HiveTranscriptSummary(lines: 42, lastCostUSD: nil)), .keep)
    }

    /// The transcript reported a cost, so the guess is replaced by a reading.
    func testAReportedCostIsChargedExactly() {
        XCTAssertEqual(
            HiveDispatch.repair(HiveTranscriptSummary(lines: 9, lastCostUSD: 0.41)), .charge(0.41)
        )
        // A negative cost is nonsense; it is clamped rather than trusted.
        XCTAssertEqual(
            HiveDispatch.repair(HiveTranscriptSummary(lines: 9, lastCostUSD: -1)), .charge(0)
        )
    }

    /// The reader takes the LAST cost line: a bee that reported twice has
    /// spent the later figure, not the earlier one.
    func testTheTranscriptReaderTakesTheLastReportedCost() throws {
        let root = temporaryRoot()
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let url = root.appendingPathComponent("session.jsonl")

        let lines = [
            #"{"type":"assistant","message":{"content":[{"type":"text","text":"working"}]}}"#,
            #"{"type":"result","subtype":"success","total_cost_usd":0.12}"#,
            "",
            #"{"type":"result","subtype":"success","total_cost_usd":0.37}"#,
        ].joined(separator: "\n")
        try Data(lines.utf8).write(to: url)

        let summary = HiveTranscript.summarise(at: url)
        XCTAssertEqual(summary.lines, 3)
        XCTAssertEqual(summary.lastCostUSD, 0.37)
        XCTAssertEqual(HiveDispatch.repair(summary), .charge(0.37))
    }

    func testAnAbsentTranscriptIsAbsentNotEmpty() {
        XCTAssertEqual(
            HiveTranscript.summarise(at: temporaryRoot().appendingPathComponent("nothing.jsonl")),
            .absent
        )
    }

    /// The arithmetic the repair drives: six bees stopped after three seconds
    /// would otherwise hold $30 against a $25 ceiling until local midnight.
    func testSixCancelledBeesNoLongerBlockTheDay() {
        var state = HiveState(policy: .default, tasks: [])
        let day = HiveState.dayKey()
        for _ in 0..<6 { state.record(spend: 5) }
        XCTAssertEqual(state.spent(), 30)

        for _ in 0..<6 {
            guard case .refundInFull = HiveDispatch.repair(.absent) else {
                return XCTFail("a bee that wrote nothing must be refunded")
            }
            state.refund(5, forDay: day)
        }
        XCTAssertEqual(state.spent(), 0)
    }

    /// A refund never drives the ledger below zero, whatever it is handed.
    func testARefundNeverGoesNegative() {
        var state = HiveState(policy: .default, tasks: [])
        state.record(spend: 2)
        state.refund(50, forDay: HiveState.dayKey())
        XCTAssertEqual(state.spent(), 0)
    }
}
