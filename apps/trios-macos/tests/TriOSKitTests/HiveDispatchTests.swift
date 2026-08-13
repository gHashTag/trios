import XCTest
@testable import TriOSKit

/// The decision the loop makes every cycle. In the copy this was ported from
/// the same logic sat inside a MainActor view model and had no tests at all,
/// so every guardrail was asserted about the policy struct rather than about
/// the code that reads it.
final class HiveDispatchTests: XCTestCase {

    private func task(
        id: String = "t",
        score: Double = 1,
        state: HiveTaskState = .pending,
        attempts: Int = 0,
        module: String = "rings/SR-00",
        created: Date = Date(timeIntervalSince1970: 1_000_000)
    ) -> HiveTask {
        var t = HiveTask(
            id: id, title: id, module: module, path: module, realm: "Swift",
            signalKind: "testGap", reason: "r", score: score, confidence: 1, prompt: "p"
        )
        t.state = state
        t.attempts = attempts
        t.createdAt = created
        return t
    }

    private func context(
        policy: HivePolicy? = nil,
        tasks: [HiveTask] = [],
        liveBees: Int = 0,
        spentToday: Double = 0,
        spawnsInLastHour: Int = 0,
        consecutiveFailures: Int = 0,
        auth: HiveAuthState? = .loggedIn(method: "oauth"),
        siblingCommittedUSD: Double = 0
    ) -> HiveDispatchContext {
        var p = policy ?? HivePolicy.default
        if policy == nil { p.enabled = true }
        return HiveDispatchContext(
            policy: p,
            tasks: tasks,
            liveBees: liveBees,
            spentToday: spentToday,
            spawnsInLastHour: spawnsInLastHour,
            consecutiveFailures: consecutiveFailures,
            auth: auth,
            siblingCommittedUSD: siblingCommittedUSD
        )
    }

    // MARK: - The happy path

    func testAnArmedLoopWithWorkDispatchesTheHighestScoringTask() {
        let decision = HiveDispatch.decide(
            context(tasks: [task(id: "low", score: 0.2), task(id: "high", score: 0.9)])
        )
        // Compared by identity, not by whole-struct equality: HiveTask carries
        // an `updatedAt` stamped at construction, so building an expected value
        // to compare against can only match by luck.
        guard case .dispatch(let chosen) = decision else {
            return XCTFail("expected a dispatch, got \(decision)")
        }
        XCTAssertEqual(chosen.id, "high")
    }

    func testADisarmedLoopIsIdleNotBlocked() {
        // "Not armed" is an ordinary state, not a fault. Reporting it as a
        // block would bury real blocks in noise.
        var policy = HivePolicy.default
        policy.enabled = false
        let decision = HiveDispatch.decide(context(policy: policy, tasks: [task()]))
        XCTAssertEqual(decision, .idle("the loop is not armed"))
    }

    func testAnEmptyQueueIsIdle() {
        guard case .idle(let why) = HiveDispatch.decide(context()) else {
            return XCTFail("an empty queue is idle")
        }
        XCTAssertTrue(why.contains("nothing schedulable"))
    }

    // MARK: - Refusals, in priority order

    func testAnUnprobedCLIBlocksRatherThanBeingAssumedSignedIn() {
        guard case .blocked(let why) = HiveDispatch.decide(context(tasks: [task()], auth: nil)) else {
            return XCTFail("an unprobed CLI must block")
        }
        XCTAssertTrue(why.contains("not been probed"))
    }

    func testASignedOutCLIBlocksWithTheFix() {
        guard case .blocked(let why) = HiveDispatch.decide(
            context(tasks: [task()], auth: .loggedOut)
        ) else {
            return XCTFail("a signed-out CLI must block")
        }
        XCTAssertTrue(why.contains("claude auth login"))
    }

    func testATrippedBreakerBlocks() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.maxConsecutiveFailures = 2
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], consecutiveFailures: 2)
        ) else {
            return XCTFail("the breaker must block")
        }
        XCTAssertTrue(why.contains("circuit breaker"))
    }

    func testTheDailyCeilingBlocks() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 10
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spentToday: 10)
        ) else {
            return XCTFail("the ceiling must block")
        }
        XCTAssertTrue(why.contains("daily ceiling"))
    }

    // MARK: - The ceiling the two Hives share
    //
    // Two copies of this loop run on this machine with a daily ceiling each,
    // so the number in either window is a share and not a total. What is
    // charged here is the sibling's already-committed spend, and only when its
    // state file proved its writer alive - the probe reduces everything it
    // could not establish to zero before the number reaches this decision.

    func testTheSiblingsCommittedSpendIsChargedAgainstThisCopysCeiling() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spentToday: 10, siblingCommittedUSD: 15)
        ) else {
            return XCTFail("the shared ceiling must block once the pair reaches it")
        }
        XCTAssertTrue(why.contains("shared daily ceiling reached"), why)
        // Both numbers, so the operator can see which copy spent what.
        XCTAssertTrue(why.contains("$10.00"), why)
        XCTAssertTrue(why.contains("$15.00"), why)
        XCTAssertTrue(why.contains("$25.00"), why)
    }

    func testTheSharedCeilingBlocksEvenWhenThisCopyHasSpentNothing() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spentToday: 0, siblingCommittedUSD: 40)
        ) else {
            return XCTFail("a sibling past the whole ceiling must block this copy")
        }
        XCTAssertTrue(why.contains("shared daily ceiling reached"), why)
    }

    func testASiblingThatHasCommittedNothingChangesNothing() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        // Absent, disarmed, unreadable or presumed dead all arrive as zero,
        // and zero must leave the loop exactly as it was.
        XCTAssertTrue(
            HiveDispatch.decide(
                context(policy: policy, tasks: [task()], spentToday: 24, siblingCommittedUSD: 0)
            ).isDispatch
        )
    }

    func testTheSharedCeilingIsAdjacentToTheCeilingRatherThanOnIt() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        // Strictly below the ceiling still dispatches; the guard is `<`, the
        // same comparison this copy's own ceiling uses.
        XCTAssertTrue(
            HiveDispatch.decide(
                context(policy: policy, tasks: [task()], spentToday: 10, siblingCommittedUSD: 14.99)
            ).isDispatch
        )
    }

    func testTheSharedCeilingIsNotReportedAsDriftedBookkeeping() {
        // The daily-ceiling *invariant* is about this copy's own ledger. A
        // sibling pushing the pair over is normal operation, and must not be
        // dressed up as evidence that this copy's state machine has drifted.
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spentToday: 5, siblingCommittedUSD: 30)
        ) else {
            return XCTFail("the shared ceiling must block")
        }
        XCTAssertFalse(why.contains("standing invariant"), why)
        XCTAssertTrue(
            HiveInvariants.check(policy: policy, tasks: [task()], spentToday: 5).isEmpty
        )
    }

    func testANegativeSiblingChargeCannotBuyBackThisCopysOwnCeiling() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.dailyBudgetUSD = 25
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spentToday: 25, siblingCommittedUSD: -100)
        ) else {
            return XCTFail("this copy's own ceiling must still block")
        }
        XCTAssertTrue(why.contains("daily ceiling reached"), why)
    }

    func testAViolatedStandingInvariantBlocksBeforeAnythingElse() {
        // A duplicate task id is a bookkeeping breach; the loop must stop
        // rather than dispatch against drifted state.
        var policy = HivePolicy.default
        policy.enabled = true
        guard case .blocked(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task(id: "same"), task(id: "same")])
        ) else {
            return XCTFail("an invariant breach must block")
        }
        XCTAssertTrue(why.contains("duplicate-tasks"))
    }

    func testASignedOutCLIIsNotMaskedByAnEmptyQueue() {
        // Order matters: the dangerous refusal must win over "nothing to do".
        guard case .blocked = HiveDispatch.decide(context(tasks: [], auth: .loggedOut)) else {
            return XCTFail("a signed-out CLI must block even with no work queued")
        }
    }

    // MARK: - Slots and rate

    func testAFullSlateIsIdleNotBlocked() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.maxConcurrentBees = 2
        guard case .idle(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], liveBees: 2)
        ) else {
            return XCTFail("a full slate is idle")
        }
        XCTAssertTrue(why.contains("bee slots are busy"))
    }

    func testTheHourlyRateLimitIsIdleNotBlocked() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.maxBeesPerHour = 3
        guard case .idle(let why) = HiveDispatch.decide(
            context(policy: policy, tasks: [task()], spawnsInLastHour: 3)
        ) else {
            return XCTFail("the rate limit is idle")
        }
        XCTAssertTrue(why.contains("already started this hour"))
    }

    // MARK: - Task selection

    func testASkippedModuleIsNeverSelected() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.skippedModules["rings/SR-00"] = "not worth it"
        let decision = HiveDispatch.decide(
            context(policy: policy, tasks: [task(id: "a", module: "rings/SR-00")])
        )
        XCTAssertFalse(decision.isDispatch)
    }

    func testATaskAtItsAttemptBudgetIsNeverSelected() {
        var policy = HivePolicy.default
        policy.enabled = true
        policy.maxAttemptsPerTask = 2
        let decision = HiveDispatch.decide(
            context(policy: policy, tasks: [task(id: "spent", state: .failed, attempts: 2)])
        )
        XCTAssertFalse(decision.isDispatch)
    }

    func testRunningAndReviewTasksAreNotReselected() {
        let decision = HiveDispatch.decide(
            context(tasks: [task(id: "a", state: .running), task(id: "b", state: .review)])
        )
        XCTAssertFalse(decision.isDispatch)
    }

    func testEqualScoresFallBackToTheOlderTaskSoNothingStarves() {
        let older = task(id: "older", score: 0.5, created: Date(timeIntervalSince1970: 1))
        let newer = task(id: "newer", score: 0.5, created: Date(timeIntervalSince1970: 99))
        XCTAssertEqual(
            HiveDispatch.nextTask(in: context(tasks: [newer, older]))?.id, "older"
        )
    }

    // MARK: - Outcome state machine

    func testAFailedBeeCostsAnAttemptAndGoesToxicWhenExhausted() {
        var policy = HivePolicy.default
        policy.maxAttemptsPerTask = 3

        let midway = HiveDispatch.outcomeState(
            for: task(attempts: 1), verdict: nil, beeSucceeded: false, policy: policy
        )
        XCTAssertEqual(midway.state, .failed)
        XCTAssertTrue(midway.countsAsFailure)

        let exhausted = HiveDispatch.outcomeState(
            for: task(attempts: 3), verdict: nil, beeSucceeded: false, policy: policy
        )
        XCTAssertEqual(exhausted.state, .toxic)
    }

    func testAPassingCheckSendsWorkToReviewAndBreaksNoStreak() {
        let result = HiveDispatch.outcomeState(
            for: task(), verdict: .passed("ok"), beeSucceeded: true, policy: .default
        )
        XCTAssertEqual(result.state, .review)
        XCTAssertFalse(result.countsAsFailure)
    }

    func testAFailingCheckOverridesTheBeesOwnClaimOfSuccess() {
        // The bee said it succeeded; the checks disagree. Checks win.
        let result = HiveDispatch.outcomeState(
            for: task(attempts: 1), verdict: .failed("build broke"),
            beeSucceeded: true, policy: .default
        )
        XCTAssertEqual(result.state, .failed)
        XCTAssertTrue(result.countsAsFailure)
    }

    func testAnUnavailableCheckReachesReviewWithoutCountingAsFailure() {
        // No checker exists. That is the Queen's gap, not the bee's fault, and
        // it must not clear a streak it never earned either.
        let result = HiveDispatch.outcomeState(
            for: task(), verdict: .unavailable("no checker"), beeSucceeded: true, policy: .default
        )
        XCTAssertEqual(result.state, .review)
        XCTAssertFalse(result.countsAsFailure)
    }

    func testWithVerificationOffSuccessGoesStraightToReview() {
        var policy = HivePolicy.default
        policy.verifyBeforeReview = false
        let result = HiveDispatch.outcomeState(
            for: task(), verdict: nil, beeSucceeded: true, policy: policy
        )
        XCTAssertEqual(result.state, .review)
        XCTAssertFalse(result.countsAsFailure)
    }
}

/// The bee process contract: the command line, the stream, and the fate.
final class HiveBeeRunnerTests: XCTestCase {

    private func configuration(worktree: String? = "hive-x") -> HiveBeeRunner.Configuration {
        HiveBeeRunner.Configuration(
            executable: "/usr/bin/true",
            workingDirectory: "/tmp",
            prompt: "do the thing",
            sessionID: "11111111-2222-3333-4444-555555555555",
            model: "sonnet",
            permissionMode: "acceptEdits",
            maxBudgetUSD: 5,
            timeoutSeconds: 60,
            worktreeName: worktree,
            displayName: "bee/hive-x"
        )
    }

    func testArgumentsPinTheSessionSoTheChatIsResumable() throws {
        let args = HiveBeeRunner.arguments(for: configuration())
        let index = try XCTUnwrap(args.firstIndex(of: "--session-id"))
        XCTAssertEqual(args[index + 1], "11111111-2222-3333-4444-555555555555")
        XCTAssertTrue(args.contains("stream-json"))
        XCTAssertTrue(args.contains("--verbose"))
    }

    func testArgumentsCarryTheBudgetAndTheWorktree() throws {
        let args = HiveBeeRunner.arguments(for: configuration())
        let budget = try XCTUnwrap(args.firstIndex(of: "--max-budget-usd"))
        XCTAssertEqual(args[budget + 1], "5.00")
        let worktree = try XCTUnwrap(args.firstIndex(of: "--worktree"))
        XCTAssertEqual(args[worktree + 1], "hive-x")
    }

    func testNoWorktreeFlagWhenIsolationIsOff() {
        XCTAssertFalse(HiveBeeRunner.arguments(for: configuration(worktree: nil)).contains("--worktree"))
    }

    func testDecodesAToolUseAsAToolEvent() throws {
        let line = #"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit"}]}}"#
        XCTAssertEqual(try XCTUnwrap(HiveBeeRunner.decode(line)).kind, .tool("Edit"))
    }

    func testDecodesAssistantText() throws {
        let line = #"{"type":"assistant","message":{"content":[{"type":"text","text":"measured 14 TODOs"}]}}"#
        let event = try XCTUnwrap(HiveBeeRunner.decode(line))
        XCTAssertEqual(event.kind, .assistant)
        XCTAssertEqual(event.text, "measured 14 TODOs")
    }

    func testMalformedLineBecomesRawRatherThanBeingDropped() throws {
        XCTAssertEqual(try XCTUnwrap(HiveBeeRunner.decode("not json at all")).kind, .raw)
    }

    func testSuccessRequiresBothACleanExitAndASuccessResult() {
        let box = HiveResultBox()
        box.record(
            line: #"{"type":"result","subtype":"success","is_error":false,"result":"done","total_cost_usd":0.42,"duration_ms":120000,"session_id":"s1"}"#
        )
        let outcome = HiveBeeRunner.outcome(
            exitCode: 0, resultBox: box, stderr: "", fallbackSessionID: "fallback"
        )
        XCTAssertEqual(outcome.status, .succeeded)
        XCTAssertEqual(outcome.costUSD, 0.42)
        XCTAssertEqual(outcome.sessionID, "s1")
    }

    func testIsErrorWinsOverASuccessSubtype() {
        // A signed-out CLI reports exactly this shape: subtype "success" with
        // is_error true. Reading only the subtype would call it a win.
        let box = HiveResultBox()
        box.record(
            line: #"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in"}"#
        )
        let outcome = HiveBeeRunner.outcome(
            exitCode: 0, resultBox: box, stderr: "", fallbackSessionID: "f"
        )
        XCTAssertEqual(outcome.status, .failed)
        XCTAssertTrue(outcome.summary.contains("Not logged in"))
    }

    func testNonZeroExitIsNeverASuccess() {
        let box = HiveResultBox()
        box.record(line: #"{"type":"result","subtype":"success","is_error":false,"result":"done"}"#)
        XCTAssertEqual(
            HiveBeeRunner.outcome(exitCode: 2, resultBox: box, stderr: "", fallbackSessionID: "f").status,
            .failed
        )
    }

    func testAMissingResultLineIsAFailureNotAnAssumedSuccess() {
        let outcome = HiveBeeRunner.outcome(
            exitCode: 0, resultBox: HiveResultBox(), stderr: "", fallbackSessionID: "f"
        )
        XCTAssertEqual(outcome.status, .failed)
        XCTAssertTrue(outcome.summary.contains("without reporting a result"))
    }

    func testTimeoutOutranksWhateverTheStreamSaid() {
        let box = HiveResultBox()
        box.record(line: #"{"type":"result","subtype":"success","is_error":false}"#)
        box.markTimedOut()
        XCTAssertEqual(
            HiveBeeRunner.outcome(exitCode: 0, resultBox: box, stderr: "", fallbackSessionID: "f").status,
            .timedOut
        )
    }

    func testStderrIsSurfacedWhenTheProcessDiesWithoutAResult() {
        let outcome = HiveBeeRunner.outcome(
            exitCode: 1, resultBox: HiveResultBox(),
            stderr: "error: not a git repository", fallbackSessionID: "f"
        )
        XCTAssertTrue(outcome.summary.contains("not a git repository"))
    }

    func testTranscriptAppendsRatherThanOverwrites() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("hive-tr-\(UUID().uuidString)/bee.jsonl")
        defer { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }

        HiveBeeRunner.appendTranscript(#"{"a":1}"#, to: url)
        HiveBeeRunner.appendTranscript(#"{"b":2}"#, to: url)

        let lines = try String(contentsOf: url, encoding: .utf8)
            .components(separatedBy: "\n").filter { !$0.isEmpty }
        XCTAssertEqual(lines.count, 2)
    }
}
