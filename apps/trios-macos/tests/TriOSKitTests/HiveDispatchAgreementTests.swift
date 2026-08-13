import XCTest
@testable import TriOSKit

/// The screen and the dispatcher must agree on which task is first.
///
/// They did not. The ranked queue was ordered on the prior-imputed key while
/// `HiveDispatch.nextTask` still sorted on `score` - the self-imputing key the
/// ordering work had just replaced - so the bee that actually went out was
/// chosen by the old rule. An adversarial probe over 2000 random scans found
/// the two disagreeing on 26.2% of them, and `nextTask` had no test at all.
///
/// This is the second time the same shape of defect landed here: a fix applied
/// to the quantity that is displayed rather than to the quantity that decides.
final class HiveDispatchAgreementTests: XCTestCase {

    private func task(
        id: String,
        score: Double,
        dispatchKey: Double?,
        created: TimeInterval = 1_000_000
    ) -> HiveTask {
        var t = HiveTask(
            id: id, title: id, module: id, path: id, realm: "Swift",
            signalKind: "testGap", reason: "r", score: score, confidence: 1, prompt: "p"
        )
        t.dispatchKey = dispatchKey
        t.createdAt = Date(timeIntervalSince1970: created)
        return t
    }

    private func context(_ tasks: [HiveTask]) -> HiveDispatchContext {
        var p = HivePolicy.default
        p.enabled = true
        return HiveDispatchContext(
            policy: p, tasks: tasks, liveBees: 0, spentToday: 0,
            spawnsInLastHour: 0, consecutiveFailures: 0,
            auth: .loggedIn(method: "oauth")
        )
    }

    func testTheDispatcherPicksByTheKeyNotByScore() throws {
        // The exact shape the probe found: a half-read module whose `score` is
        // higher because it divides by the measured weight only, against a
        // fully-read module that the ranked queue puts first.
        let wellRead = task(id: "well-read", score: 0.55, dispatchKey: 0.55)
        let halfRead = task(id: "half-read", score: 0.60, dispatchKey: 0.39)

        let picked = try XCTUnwrap(HiveDispatch.nextTask(in: context([wellRead, halfRead])))
        XCTAssertEqual(
            picked.id, "well-read",
            "the dispatcher must follow the ranked key, not the self-imputing score"
        )
    }

    func testTheTopOfTheRankedQueueIsTheTaskTheDispatcherPicks() throws {
        // The property that matters, stated directly: whatever the operator is
        // shown at the top of the list is what the next bee works on.
        let tasks = [
            task(id: "a", score: 0.90, dispatchKey: 0.20),
            task(id: "b", score: 0.10, dispatchKey: 0.80),
            task(id: "c", score: 0.50, dispatchKey: 0.50),
        ]
        let byKey = tasks.sorted { ($0.dispatchKey ?? $0.score) > ($1.dispatchKey ?? $1.score) }
        let picked = try XCTUnwrap(HiveDispatch.nextTask(in: context(tasks)))
        XCTAssertEqual(picked.id, byKey.first?.id)
        XCTAssertEqual(picked.id, "b")
    }

    func testATaskFromAnOlderStateFileFallsBackToScoreForOneCycle() throws {
        // A task written before dispatchKey existed carries nil. It must still
        // be orderable rather than crash or sort as zero, and materialiseTasks
        // refreshes it on the first cycle after launch.
        let legacy = task(id: "legacy", score: 0.70, dispatchKey: nil)
        let fresh = task(id: "fresh", score: 0.10, dispatchKey: 0.30)
        let picked = try XCTUnwrap(HiveDispatch.nextTask(in: context([fresh, legacy])))
        XCTAssertEqual(picked.id, "legacy")
    }

    func testEqualKeysStillFallBackToTheOlderTask() throws {
        let older = task(id: "older", score: 0.1, dispatchKey: 0.5, created: 1)
        let newer = task(id: "newer", score: 0.9, dispatchKey: 0.5, created: 99)
        let picked = try XCTUnwrap(HiveDispatch.nextTask(in: context([newer, older])))
        XCTAssertEqual(picked.id, "older", "a tie must not starve a module")
    }

    func testTheKeyTravelsFromTheTargetOntoTheTask() throws {
        var facts = HiveModuleFacts(module: "rings/SR-00", path: "rings/SR-00", realm: .swiftRing)
        facts.lines = 1000
        facts.todos = 30
        facts.churn30d = 5
        facts.testBlocks = 0
        facts.openIssues = 2
        let target = try XCTUnwrap(HivePriorityEngine.rank([facts]).first)
        let made = try XCTUnwrap(HiveTaskFactory.makeTask(from: target, policy: .default))

        // Not recomputed from score and confidence - those cannot reconstruct
        // it, which is why it is carried.
        XCTAssertEqual(made.dispatchKey, target.priorImputedScore)
    }
}
