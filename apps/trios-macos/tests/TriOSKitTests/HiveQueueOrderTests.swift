import XCTest
@testable import TriOSKit

/// The key the queue is ACTUALLY ordered on.
///
/// This suite exists because the no-imputation invariant was asserted on
/// `score` while the queue was ordered on it too - self-imputation, which
/// rewards ignorance multiplicatively and hands the top of the queue to
/// whichever module the Queen knows least about. The rule and the sort had no
/// test in common. Every assertion here is written against
/// `HivePriorityEngine.ordered` or the key it reads, never against a number
/// that merely sits beside it on the screen.
final class HiveQueueOrderTests: XCTestCase {

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

    /// Measured on four of six signals: churn and open issues went unread, so
    /// 38% of the weight is left open and the interval is that wide.
    private func halfReadFacts() -> HiveModuleFacts {
        var half = HiveModuleFacts(module: "rings/half", path: "rings/half", realm: .swiftRing)
        half.lines = 1000
        half.todos = 8
        half.testBlocks = 3
        half.declaredStatus = "active"
        half.unmeasuredReasons[.churn] = "git log exited 128"
        half.unmeasuredReasons[.openIssues] = "issues snapshot absent"
        return half
    }

    // MARK: - The test that was missing

    func testAFailedProbeIsNotScoredAsZeroOnTheKeyTheSortUses() throws {
        // Two modules, identical on every signal either of them measured. One
        // had its churn probe fail; the other measured churn and it really was
        // zero. The instrument's first rule says these two are not the same
        // reading, so the key the sort uses must not give them the same number.
        var blind = facts("rings/blind", todos: 10, churn: nil, tests: 0, issues: 2)
        blind.unmeasuredReasons[.churn] = "git log exited 128"
        let zeroed = facts("rings/zeroed", todos: 10, churn: 0, tests: 0, issues: 2)

        let ranked = HivePriorityEngine.rank([blind, zeroed])
        let blindTarget = try XCTUnwrap(ranked.first { $0.module == "rings/blind" })
        let zeroedTarget = try XCTUnwrap(ranked.first { $0.module == "rings/zeroed" })

        // The zero-imputed key: a failed probe and a measured zero are
        // indistinguishable. Kept as a live assertion so the defect cannot come
        // back unnoticed under a different name.
        XCTAssertEqual(blindTarget.zeroImputedScore, zeroedTarget.zeroImputedScore)

        // The key the sort uses: they are not the same reading.
        XCTAssertGreaterThan(blindTarget.priorImputedScore, zeroedTarget.priorImputedScore)

        // And the order that comes out of the sort, not merely the property.
        let order = HivePriorityEngine.ordered([zeroedTarget, blindTarget]).map(\.module)
        XCTAssertEqual(order, ["rings/blind", "rings/zeroed"])
    }

    func testEverySignalDeclaresAPriorStrictlyAboveZero() {
        // A prior of zero is zero-imputation wearing the new key's name. This
        // is the single line that stops the whole design decaying back into
        // the defect it replaced.
        for kind in HiveSignal.Kind.allCases {
            XCTAssertGreaterThan(kind.priorWhenUnread, 0, "\(kind.rawValue) declares a zero prior")
            XCTAssertLessThanOrEqual(kind.priorWhenUnread, 1, "\(kind.rawValue) is above full scale")
        }
        XCTAssertEqual(HiveSignal.Kind.totalWeight, 1.0, accuracy: 1e-12)
    }

    func testTheQueueIsOrderedDescendingOnThePriorImputedScore() {
        let ranked = HivePriorityEngine.rank([
            facts("rings/a", todos: 30, churn: 12, tests: 0, issues: 5),
            facts("rings/b", todos: 2, churn: 1, tests: 20, issues: 0),
            facts("rings/c", todos: 14, churn: 4, tests: 3, issues: 2),
            facts("rings/d", todos: 0, churn: 0, tests: 40, issues: 0),
        ])
        let keys = ranked.map(\.priorImputedScore)
        XCTAssertEqual(keys, keys.sorted(by: >))
    }

    func testAProbeFailureDoesNotMoveTheKeyWhenTheDeclaredPriorIsRight() throws {
        // Neutrality OF THE KEY with respect to probe failure. Named narrowly
        // on purpose: an adversarial review measured what the old name implied
        // and found the RANK moves in 46.7% of single-probe failures, because
        // the key is unchanged only when the true reading equals the declared
        // prior exactly - and even then the tie-break reorders the pair. The
        // key is the neutral quantity; the ranking is not, and the test that
        // says so is directly below.
        //
        // Nothing about the module changed here, only whether the scanner
        // managed to read it. churn's
        // declared prior is 0.15 of full scale, and full scale is 20 commits,
        // so a module that really churned 3 times is the module the prior
        // describes. Its key must not move when the probe fails.
        let read = facts("rings/read", todos: 10, churn: 3, tests: 0, issues: 2)
        var unread = facts("rings/unread", todos: 10, churn: nil, tests: 0, issues: 2)
        unread.unmeasuredReasons[.churn] = "git history not read"

        let ranked = HivePriorityEngine.rank([read, unread])
        let readTarget = try XCTUnwrap(ranked.first { $0.module == "rings/read" })
        let unreadTarget = try XCTUnwrap(ranked.first { $0.module == "rings/unread" })

        // Exact, not approximate. Two mathematically equal keys that differ in
        // the last bit skip the declared tie-breaks, and the target with the
        // FAILED probe lands above the measured one for a reason nobody wrote
        // down - which is how the previous key was found to be wrong.
        XCTAssertTrue(readTarget.priorImputedScore == unreadTarget.priorImputedScore)

        // Equal keys, so the declared tie-break decides: the better-measured
        // module goes first.
        XCTAssertEqual(
            HivePriorityEngine.ordered([unreadTarget, readTarget]).map(\.module),
            ["rings/read", "rings/unread"]
        )
    }

    func testTheKeyIsNeitherOfTheTwoRejectedCandidates() throws {
        // A set built so all three candidate keys disagree. `score` divides by
        // the measured weight only and hands the top to the least-known
        // module; `zeroImputedScore` reads every failed probe as health.
        var thin = HiveModuleFacts(module: "rings/thin", path: "rings/thin", realm: .swiftRing)
        thin.lines = 900
        thin.todos = 18
        thin.unmeasuredReasons[.churn] = "git log exited 128"
        thin.unmeasuredReasons[.testGap] = "test blocks not counted"
        thin.unmeasuredReasons[.declaredIncomplete] = "no status declared"
        thin.unmeasuredReasons[.openIssues] = "issues snapshot absent"

        let solid = facts("rings/solid", todos: 12, churn: 6, tests: 1, issues: 3)
        let ranked = HivePriorityEngine.rank([thin, solid])

        let thinTarget = try XCTUnwrap(ranked.first { $0.module == "rings/thin" })
        let solidTarget = try XCTUnwrap(ranked.first { $0.module == "rings/solid" })

        // score rewards ignorance...
        XCTAssertGreaterThan(thinTarget.score, solidTarget.score)
        // ...zeroImputedScore punishes it...
        XCTAssertLessThan(thinTarget.zeroImputedScore, solidTarget.zeroImputedScore)
        // ...and the key the queue uses is neither: it is the declared prior
        // that decides, and here it leaves the fully measured module on top.
        XCTAssertLessThan(thinTarget.priorImputedScore, solidTarget.priorImputedScore)
    }

    /// One signal, already normalised. `nil` means its probe failed.
    private func signal(_ kind: HiveSignal.Kind, _ value: Double?) -> HiveSignal {
        let state: HiveSignalState = value.map { .measured($0) } ?? .unmeasured("probe failed")
        return HiveSignal(kind: kind, raw: state, normalized: state)
    }

    /// A target built from normalised readings directly, with `score`,
    /// `confidence` and `weightedTotal` derived the way the engine derives
    /// them. Some readings cannot be produced from facts - `sizeRisk` is
    /// measured whenever the module has a line count, and `todoDensity` needs
    /// that same line count - so the sharpest cases have to be stated here.
    private func handBuilt(_ module: String, _ signals: [HiveSignal]) -> HiveTarget {
        var weighted = 0.0
        var measuredWeight = 0.0
        for signal in signals {
            guard let value = signal.normalized.value else { continue }
            weighted += value * signal.kind.weight
            measuredWeight += signal.kind.weight
        }
        return HiveTarget(
            module: module,
            path: module,
            realm: .swiftRing,
            signals: signals,
            score: measuredWeight > 0 ? weighted / measuredWeight : 0,
            confidence: measuredWeight / HiveSignal.Kind.totalWeight,
            weightedTotal: weighted
        )
    }

    private func uniformlyMeasured(_ module: String, at value: Double) -> HiveTarget {
        handBuilt(module, HiveSignal.Kind.allCases.map { signal($0, value) })
    }

    /// Read on 58% of the weight and genuinely bad there. Above the confidence
    /// floor, so it is ranked rather than gated.
    private func partlyRead(_ module: String) -> HiveTarget {
        handBuilt(module, [
            signal(.todoDensity, 0.5),
            signal(.churn, nil),
            signal(.testGap, 1.0),
            signal(.sizeRisk, nil),
            signal(.declaredIncomplete, nil),
            signal(.openIssues, 0.5),
        ])
    }

    /// The three candidate keys produce three different orders on one set, and
    /// the queue follows exactly one of them.
    func testTheQueueFollowsThePriorImputedKeyAndNeitherOfTheOthers() {
        let high = uniformlyMeasured("a-high", at: 0.55)
        let low = uniformlyMeasured("b-low", at: 0.42)
        let partial = partlyRead("c-partial")
        let all = [low, partial, high]

        // What the queue actually does.
        XCTAssertEqual(
            HivePriorityEngine.ordered(all).map(\.module),
            ["a-high", "c-partial", "b-low"]
        )

        // `score`, which this copy used to order on, divides by the measured
        // weight only, so the least-known module goes to the top.
        XCTAssertEqual(
            all.sorted { $0.score > $1.score }.map(\.module),
            ["c-partial", "a-high", "b-low"]
        )

        // `zeroImputedScore` reads three failed probes as three zeroes, so the
        // same module falls to the bottom.
        XCTAssertEqual(
            all.sorted { $0.zeroImputedScore > $1.zeroImputedScore }.map(\.module),
            ["a-high", "b-low", "c-partial"]
        )
    }

    // MARK: - The gate

    func testATargetBelowTheConfidenceFloorLeavesTheRankedQueue() {
        var thin = HiveModuleFacts(module: "rings/thin", path: "rings/thin", realm: .swiftRing)
        thin.churn30d = 5
        let solid = facts("rings/solid", todos: 12, churn: 6, tests: 1, issues: 3)

        let queue = HivePriorityEngine.queue([thin, solid])

        XCTAssertEqual(queue.ranked.map(\.target.module), ["rings/solid"])
        XCTAssertEqual(queue.instrumentFaults.map(\.module), ["rings/thin"])
        // It is not dropped: the operator has to see that the instrument, not
        // the module, is what needs work.
        XCTAssertLessThan(
            queue.instrumentFaults.first?.confidence ?? 1,
            HiveInvariants.minimumDispatchConfidence
        )
    }

    func testAnInstrumentFaultNamesTheProbesThatFailed() throws {
        var thin = HiveModuleFacts(module: "rings/thin", path: "rings/thin", realm: .swiftRing)
        thin.churn30d = 5
        thin.unmeasuredReasons[.testGap] = "test blocks not counted"

        let queue = HivePriorityEngine.queue([thin])
        let fault = try XCTUnwrap(queue.instrumentFaults.first)
        XCTAssertTrue(fault.unreadProbeDetail.contains("test blocks not counted"))
        XCTAssertTrue(fault.unreadProbeDetail.contains("Test gap"))
        XCTAssertTrue(queue.ranked.isEmpty)
    }

    func testRankStillReturnsEveryTargetWithTheFaultsLast() {
        var thin = HiveModuleFacts(module: "rings/aaa-thin", path: "rings/aaa-thin", realm: .swiftRing)
        thin.churn30d = 5
        let solid = facts("rings/zzz-solid", todos: 12, churn: 6, tests: 1, issues: 3)

        // Alphabetically the fault sorts first; it must still come last,
        // because it is not ranked at all.
        let ranked = HivePriorityEngine.rank([thin, solid]).map(\.module)
        XCTAssertEqual(ranked, ["rings/zzz-solid", "rings/aaa-thin"])
    }

    // MARK: - What the evidence settles

    func testTheIntervalBracketsTheKeyAndItsLowerEndIsTheZeroImputedScore() throws {
        var partial = HiveModuleFacts(module: "rings/partial", path: "rings/partial", realm: .swiftRing)
        partial.lines = 800
        partial.todos = 12
        partial.testBlocks = 0
        partial.churn30d = 4
        partial.unmeasuredReasons[.openIssues] = "issues snapshot absent"
        partial.unmeasuredReasons[.declaredIncomplete] = "no status declared"

        let target = try XCTUnwrap(HivePriorityEngine.rank([partial]).first)
        XCTAssertEqual(target.lowerBound, target.zeroImputedScore)
        XCTAssertLessThanOrEqual(target.lowerBound, target.priorImputedScore)
        XCTAssertLessThanOrEqual(target.priorImputedScore, target.upperBound)
        XCTAssertEqual(target.upperBound - target.lowerBound, 1 - target.confidence, accuracy: 1e-12)
    }

    func testAFullyMeasuredTargetHasAPointInterval() throws {
        let target = try XCTUnwrap(HivePriorityEngine.rank([facts("rings/full", todos: 5)]).first)
        XCTAssertEqual(target.confidence, 1.0)
        XCTAssertEqual(target.lowerBound, target.upperBound)
        XCTAssertEqual(target.priorImputedScore, target.lowerBound)
    }

    func testNeighboursWhoseIntervalsOverlapAreMarkedNotSettled() throws {
        // A fully measured module and a half-read one whose unread weight is
        // wide enough to reach across the gap.
        let queue = HivePriorityEngine.queue([
            halfReadFacts(),
            facts("rings/solid", todos: 12, churn: 6, tests: 1, issues: 3),
        ])
        XCTAssertEqual(queue.ranked.count, 2)
        // The first row has nothing above it, so it is settled by definition.
        XCTAssertEqual(queue.ranked.first?.separation, .settled)

        let second = try XCTUnwrap(queue.ranked.last)
        guard case .notSettled = second.separation else {
            return XCTFail("an overlapping pair must be reported as unsettled, not silently ordered")
        }
        XCTAssertTrue(queue.hasUnsettledPairs)
    }

    func testTwoFullyMeasuredNeighboursAreSettledByTheEvidence() {
        let worse = facts("rings/worse", todos: 40, churn: 18, tests: 0, issues: 8)
        let better = facts("rings/better", todos: 0, churn: 0, tests: 30, issues: 0)

        let queue = HivePriorityEngine.queue([better, worse])
        XCTAssertEqual(queue.ranked.map(\.target.module), ["rings/worse", "rings/better"])
        // Both intervals are points, and the points differ.
        XCTAssertTrue(queue.ranked.allSatisfy { $0.separation == .settled })
        XCTAssertFalse(queue.hasUnsettledPairs)
    }

    func testTheBreakEvenIsTheReadingThatWouldDrawTheLowerRowLevel() throws {
        let queue = HivePriorityEngine.queue([
            halfReadFacts(),
            facts("rings/solid", todos: 12, churn: 6, tests: 1, issues: 3),
        ])
        let lower = try XCTUnwrap(queue.ranked.last)
        guard case .notSettled(let maybeBreakEven) = lower.separation,
              let breakEven = maybeBreakEven else {
            return XCTFail("this pair overlaps and the lower row has unread weight")
        }

        // Substituting the break-even reading for every unread signal must
        // reproduce the key of the row above, to arithmetic precision.
        let above = try XCTUnwrap(queue.ranked.first).target
        let rebuilt = lower.target.lowerBound
            + breakEven.normalized * lower.target.unmeasuredShare
        XCTAssertEqual(rebuilt, above.priorImputedScore, accuracy: 1e-9)

        // And it is reported in the unread signals' own units, not as a bare
        // fraction: churn's full scale is 20 commits.
        let churn = try XCTUnwrap(breakEven.readings.first { $0.kind == .churn })
        XCTAssertEqual(churn.inOwnUnits, breakEven.normalized * 20, accuracy: 1e-9)
        XCTAssertEqual(breakEven.readings.map(\.kind.rawValue).sorted(), ["churn", "openIssues"])
        XCTAssertTrue(breakEven.summary.contains("of 20"))
    }

    func testAnUnsettledPairWhoseLowerRowIsFullyMeasuredReportsNoBreakEven() throws {
        // The doubt belongs to the row ABOVE: the lower row has nothing left
        // unread, so no reading of its own could flip the pair, and inventing
        // a number for it would be the imputation this whole wave removed.
        let queue = HivePriorityEngine.queue([
            facts("rings/measured", todos: 6, churn: 2, tests: 4, issues: 3),
            halfReadFacts(),
        ])
        XCTAssertEqual(queue.ranked.map(\.target.module), ["rings/half", "rings/measured"])

        let lower = try XCTUnwrap(queue.ranked.last)
        XCTAssertEqual(lower.target.confidence, 1.0)
        guard case .notSettled(let breakEven) = lower.separation else {
            return XCTFail("the row above has unread weight reaching below this one")
        }
        XCTAssertNil(breakEven)
    }

    // MARK: - Determinism

    func testIdenticalFactsGiveAnIdenticalQueue() {
        var thin = HiveModuleFacts(module: "rings/thin", path: "rings/thin", realm: .swiftRing)
        thin.churn30d = 5
        let input = [
            facts("rings/b", todos: 12, churn: 6, tests: 1, issues: 3),
            thin,
            facts("rings/a", todos: 12, churn: 6, tests: 1, issues: 3),
        ]
        XCTAssertEqual(HivePriorityEngine.queue(input), HivePriorityEngine.queue(input))
        // Equal keys and equal confidence fall back to the module name.
        XCTAssertEqual(
            HivePriorityEngine.queue(input).ranked.map(\.target.module),
            ["rings/a", "rings/b"]
        )
    }

    // MARK: - What the operator is told

    func testTheOrderingSentenceNamesTheAssumptionItMakes() {
        let sentence = HiveQueue.orderingSentence
        XCTAssertTrue(sentence.contains("never zero"))
        XCTAssertTrue(sentence.contains("not settled"))
        XCTAssertTrue(sentence.allSatisfy { $0.isASCII })
    }

    // MARK: - A score is a property of the module

    /// The fixed scale, ported from the other copy in this wave. Without it a
    /// declared prior is meaningless: a prior is a claim about a reading, and a
    /// set-relative reading is a claim about whichever modules were scanned
    /// beside it.
    func testAScoreDoesNotMoveWhenAnUnrelatedModuleIsAddedToTheScan() throws {
        let subject = facts("rings/subject", todos: 12, churn: 6, tests: 1, issues: 3)
        let alone = try XCTUnwrap(HivePriorityEngine.rank([subject]).first)

        let noisy = facts("rings/noisy", todos: 400, churn: 90, tests: 0, issues: 40)
        let together = try XCTUnwrap(
            HivePriorityEngine.rank([subject, noisy]).first { $0.module == "rings/subject" }
        )

        XCTAssertEqual(alone.score, together.score)
        XCTAssertEqual(alone.priorImputedScore, together.priorImputedScore)
    }
}
