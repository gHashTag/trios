import XCTest
@testable import TriOSKit

/// The second unbounded wait on the subprocess path.
///
/// An adversarial review found that escalating SIGTERM to SIGKILL unblocks
/// `waitUntilExit()` and nothing else: one line later `group.wait()` waits on
/// pipe readers, and `readDataToEndOfFile()` returns only when the WRITE end
/// closes. A grandchild that inherited the descriptor keeps it open after its
/// parent is killed, and `claude -p` spawns tool subprocesses routinely.
final class HiveDrainTests: XCTestCase {

    func testAGrandchildHoldingThePipeDoesNotWedgeTheCaller() throws {
        // The child exits immediately; the grandchild it leaves behind holds
        // the inherited stdout for far longer than any grace period. Without a
        // bounded drain this call never returns.
        let started = Date()
        let result = HiveProcess.run(
            executable: "/bin/sh",
            arguments: ["-c", "sleep 60 & exit 0"],
            timeout: 3
        )
        let elapsed = Date().timeIntervalSince(started)

        XCTAssertLessThan(
            elapsed,
            HiveProcess.terminationGraceSeconds + HiveProcess.readerDrainGraceSeconds + 5,
            "the call must return on a bounded schedule even while a grandchild holds the pipe"
        )
        // It returned, which is the whole point; the child itself exited 0.
        XCTAssertEqual(result.exitCode, 0)
    }

    func testAnOrdinaryProcessStillReturnsItsWholeOutput() {
        // The drain bound must not truncate a well-behaved child.
        let result = HiveProcess.run(
            executable: "/bin/echo",
            arguments: ["measured, not assumed"],
            timeout: 10
        )
        XCTAssertEqual(result.exitCode, 0)
        XCTAssertTrue(result.standardOutput.contains("measured, not assumed"))
        XCTAssertFalse(result.timedOut)
    }

    func testAChildThatIgnoresSigtermIsStillReapedWithinTheGracePeriod() {
        let started = Date()
        let result = HiveProcess.run(
            executable: "/bin/sh",
            arguments: ["-c", "trap '' TERM; sleep 60"],
            timeout: 2
        )
        let elapsed = Date().timeIntervalSince(started)

        XCTAssertTrue(result.timedOut)
        XCTAssertLessThan(
            elapsed,
            2 + HiveProcess.terminationGraceSeconds + HiveProcess.readerDrainGraceSeconds + 5,
            "SIGTERM is a request; the escalation to SIGKILL is what bounds this"
        )
    }
}
