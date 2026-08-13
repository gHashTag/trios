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
        XCTAssertFalse(result.outputTruncated)
    }

    /// The bounded drain must hand back what arrived, and say that it is a
    /// prefix.
    ///
    /// This is the theorem the last wave's comment asserted and the code did
    /// not do: the bytes were published by the reader RETURNING, and on this
    /// path the reader has not returned. The payload was written, read by the
    /// kernel, and discarded - exit 0, timedOut false, stdout zero bytes. Every
    /// caller then saw a successful command that produced no output, and the
    /// scanner scored that as a measured churn of zero for the whole tree.
    func testABoundedDrainReturnsWhatArrivedAndSaysItIsAPrefix() {
        let result = HiveProcess.run(
            executable: "/bin/sh",
            arguments: ["-c", "echo '{\"loggedIn\":true}'; sleep 45 & exit 0"],
            timeout: 3
        )

        XCTAssertEqual(result.exitCode, 0)
        // The child exited on its own, well inside the timeout...
        XCTAssertFalse(result.timedOut)
        // ...the grandchild held the pipe past the drain grace...
        XCTAssertTrue(result.outputTruncated)
        // ...and the line the child did write is still here.
        XCTAssertTrue(
            result.standardOutput.contains("loggedIn"),
            "the payload arrived before the bound and must not be discarded"
        )
    }

    /// The scanner's side of the same defect.
    ///
    /// `git log` exiting 0 with a truncated read parses into a table missing
    /// most of the repository, and every module the table does not name is
    /// written out as a MEASURED churn of zero - weight still in the
    /// denominator, confidence reported as if the probe had worked. The rule
    /// the whole file exists to enforce, breached from underneath.
    func testATruncatedGitLogIsAFailedReadingNotAChurnOfZero() {
        let truncated = HiveProcess.Result(
            exitCode: 0,
            standardOutput: "abc\nBR-OUTPUT/HiveModels.swift\n",
            standardError: "",
            timedOut: false,
            outputTruncated: true
        )
        switch HiveRepoScanner.churnReading(from: truncated, prefixes: ["BR-OUTPUT"]) {
        case .success(let table):
            XCTFail("a partial log was accepted as a reading: \(table)")
        case .failure(let why):
            XCTAssertTrue(why.contains("not read to the end"), why)
        }
    }

    func testACompleteGitLogIsStillARealReading() {
        let complete = HiveProcess.Result(
            exitCode: 0,
            standardOutput: String(repeating: "a", count: 40) + "\nBR-OUTPUT/HiveModels.swift\n",
            standardError: "",
            timedOut: false,
            outputTruncated: false
        )
        let reading = HiveRepoScanner.churnReading(
            from: complete,
            prefixes: ["BR-OUTPUT"],
            flatRoots: ["BR-OUTPUT"]
        )
        switch reading {
        case .success(let table):
            XCTAssertEqual(table["BR-OUTPUT"], 1)
        case .failure(let why):
            XCTFail("a complete log must be a reading: \(why)")
        }
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
