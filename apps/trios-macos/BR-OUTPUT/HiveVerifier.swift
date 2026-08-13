import Foundation

// ===========================================================================
// VERIFICATION - a bee's success is a claim until something executes.
//
// Without this, the Queen's only evidence that a module improved is the bee's
// own closing paragraph. That is the model grading its own homework, and it is
// how an autonomous loop accumulates confident, plausible, wrong work.
//
// Three outcomes, never two. "Could not check" is its own verdict and must
// never collapse into "passed".
// ===========================================================================

enum HiveVerdict: Equatable {
    case passed(String)
    case failed(String)
    /// No runnable check exists for this realm, or the checker itself broke.
    case unavailable(String)

    var label: String {
        switch self {
        case .passed: return "VERIFIED"
        case .failed: return "BROKE THE BUILD"
        case .unavailable: return "UNVERIFIED"
        }
    }

    var detail: String {
        switch self {
        case .passed(let d), .failed(let d), .unavailable(let d): return d
        }
    }

    /// Only a passing check clears a bee's work. Unavailable does not.
    var isPass: Bool {
        if case .passed = self { return true }
        return false
    }

    /// A failing check is the only verdict that costs the bee an attempt.
    /// An absent checker is the Queen's gap, not the bee's fault.
    var isFail: Bool {
        if case .failed = self { return true }
        return false
    }
}

/// Whether the evidence behind a recorded verdict still applies.
enum HiveEvidenceState: Equatable {
    case current
    case stale(measuredAt: String)
    /// A verdict exists but no commit was recorded with it.
    case unrecorded
    case unknown(String)

    var isCurrent: Bool { self == .current }

    var label: String {
        switch self {
        case .current: return "CURRENT"
        case .stale: return "STALE"
        case .unrecorded: return "NO COMMIT RECORDED"
        case .unknown: return "UNKNOWN"
        }
    }
}

struct HiveVerifier {

    /// The app directory (`.../apps/trios-macos`), not the repository root.
    let projectRoot: String
    /// Where the app sits inside the repository, e.g. `apps/trios-macos`.
    ///
    /// A worktree is registered at the *repository* root, so resolving one and
    /// running the checks there looks for `Package.swift` beside `crates/` and
    /// finds nothing. Every worktree bee would come back UNVERIFIED - the gate
    /// built to stop false passes would quietly never fire.
    let appRelativePath: String
    /// Wall-clock ceiling for the whole check.
    var timeout: TimeInterval = 900

    init(projectRoot: String = ProjectPaths.root, appRelativePath: String? = nil) {
        self.projectRoot = projectRoot
        if let appRelativePath {
            self.appRelativePath = appRelativePath
        } else {
            let url = URL(fileURLWithPath: projectRoot)
            let app = url.lastPathComponent
            let parent = url.deletingLastPathComponent().lastPathComponent
            self.appRelativePath = parent.isEmpty ? app : "\(parent)/\(app)"
        }
    }

    // MARK: - Entry point

    func verify(task: HiveTask) -> HiveVerdict {
        // A bee isolated in a worktree changed files there. If that worktree
        // cannot be located, falling back to the main checkout would run the
        // checks against a tree the bee never touched and report a green tick
        // for work nobody examined. Refuse instead.
        guard let root = workingRoot(for: task) else {
            return .unavailable(
                "bee ran in worktree `\(task.branch ?? "?")` but no such worktree is registered - "
                    + "verifying the main checkout would grade the wrong tree"
            )
        }

        switch task.realm {
        case HiveModuleFacts.Realm.swiftRing.rawValue,
             HiveModuleFacts.Realm.surface.rawValue:
            return verifySwiftPackage(at: root)
        case HiveModuleFacts.Realm.rustRing.rawValue:
            return verifyRustRing(module: task.module, at: root)
        default:
            return .unavailable("no check is defined for realm \(task.realm)")
        }
    }

    // MARK: - Swift

    func verifySwiftPackage(at packagePath: String) -> HiveVerdict {
        guard FileManager.default.fileExists(atPath: "\(packagePath)/Package.swift") else {
            return .unavailable("no Package.swift at \(packagePath)")
        }
        guard let swift = HiveProcess.resolve("swift", overrideEnvKey: "SWIFT_EXECUTABLE") else {
            return .unavailable("swift toolchain not found")
        }

        let build = HiveProcess.run(
            executable: swift, arguments: ["build"],
            currentDirectory: packagePath, timeout: timeout
        )
        if build.timedOut {
            return .unavailable("`swift build` exceeded \(Int(timeout))s - verdict unknown, not assumed")
        }
        if build.exitCode != 0 {
            return .failed("`swift build` failed:\n" + Self.tail(build.standardError + build.standardOutput))
        }

        let test = HiveProcess.run(
            executable: swift, arguments: ["test"],
            currentDirectory: packagePath, timeout: timeout
        )
        if test.timedOut {
            return .unavailable("`swift test` exceeded \(Int(timeout))s - verdict unknown, not assumed")
        }
        if test.exitCode != 0 {
            return .failed("`swift test` failed:\n" + Self.tail(test.standardOutput + test.standardError))
        }
        return .passed(Self.testSummary(test.standardOutput) ?? "swift build and tests passed")
    }

    // MARK: - Rust

    /// Rust rings carry their own Cargo manifest, so they get a real checker -
    /// which the tree this was ported from could not offer for its Zig core.
    func verifyRustRing(module: String, at root: String) -> HiveVerdict {
        let ringPath = "\(root)/\(module)"
        guard let manifest = Self.findCargoManifest(under: ringPath) else {
            return .unavailable("no Cargo.toml under \(module) - nothing to run")
        }
        guard let cargo = HiveProcess.resolve("cargo", overrideEnvKey: "CARGO_EXECUTABLE") else {
            return .unavailable("cargo not found")
        }

        let test = HiveProcess.run(
            executable: cargo,
            arguments: ["test", "--manifest-path", manifest],
            currentDirectory: root,
            timeout: timeout
        )
        if test.timedOut {
            return .unavailable("`cargo test` exceeded \(Int(timeout))s - verdict unknown, not assumed")
        }
        if test.exitCode != 0 {
            return .failed("`cargo test` failed:\n" + Self.tail(test.standardOutput + test.standardError))
        }
        return .passed(Self.cargoSummary(test.standardOutput) ?? "cargo test passed")
    }

    /// Nearest manifest at or below the ring root. Rings nest their crates.
    static func findCargoManifest(under path: String) -> String? {
        let fm = FileManager.default
        let direct = "\(path)/Cargo.toml"
        if fm.fileExists(atPath: direct) { return direct }
        guard let walker = fm.enumerator(
            at: URL(fileURLWithPath: path),
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        ) else { return nil }
        var best: String?
        for case let url as URL in walker where url.lastPathComponent == "Cargo.toml" {
            // Shallowest wins, so a workspace root beats a member crate.
            if best == nil || url.pathComponents.count < URL(fileURLWithPath: best!).pathComponents.count {
                best = url.path
            }
        }
        return best
    }

    // MARK: - Worktree resolution

    /// The directory the checks should run in: the app tree inside whichever
    /// checkout the bee worked in.
    ///
    /// Returns nil when the task named a worktree that cannot be found - never
    /// the main checkout as a consolation prize.
    func workingRoot(for task: HiveTask) -> String? {
        guard let branch = task.branch else { return projectRoot }
        let result = HiveProcess.run(
            executable: "/usr/bin/git",
            arguments: ["-C", projectRoot, "worktree", "list", "--porcelain"],
            timeout: 30
        )
        guard result.exitCode == 0 else { return nil }
        guard let repoRoot = Self.worktreePath(named: branch, in: result.standardOutput) else {
            return nil
        }
        // git reports the repository root; the checks live one app down.
        let appRoot = "\(repoRoot)/\(appRelativePath)"
        return FileManager.default.fileExists(atPath: appRoot) ? appRoot : repoRoot
    }

    /// Parses `git worktree list --porcelain`, matching either the branch ref
    /// or the trailing path component against the worktree name.
    static func worktreePath(named name: String, in porcelain: String) -> String? {
        var currentPath: String?
        for line in porcelain.components(separatedBy: "\n") {
            if line.hasPrefix("worktree ") {
                currentPath = String(line.dropFirst("worktree ".count))
                if let currentPath, URL(fileURLWithPath: currentPath).lastPathComponent == name {
                    return currentPath
                }
            } else if line.hasPrefix("branch ") {
                let ref = String(line.dropFirst("branch ".count))
                if ref == "refs/heads/\(name)" || ref.hasSuffix("/\(name)") {
                    return currentPath
                }
            }
        }
        return nil
    }

    // MARK: - Output shaping

    /// Quotes swift-testing's own tally, so the recorded verdict is a number
    /// the tool printed rather than a sentence composed here.
    static func testSummary(_ output: String) -> String? {
        for line in output.components(separatedBy: "\n").reversed()
        where line.contains("Test run with") && line.contains("test") {
            return line.trimmingCharacters(in: .whitespaces)
                .trimmingCharacters(in: CharacterSet(charactersIn: "\u{2714} "))
        }
        return nil
    }

    static func cargoSummary(_ output: String) -> String? {
        for line in output.components(separatedBy: "\n").reversed()
        where line.hasPrefix("test result:") {
            return line.trimmingCharacters(in: .whitespaces)
        }
        return nil
    }

    // MARK: - Evidence currency

    /// The commit a verdict was measured against.
    ///
    /// A lifecycle state like `review` is a claim unless the evidence behind it
    /// is *current*. A verdict recorded on Monday says nothing about a tree
    /// that moved on Tuesday, and without the commit there is no way to tell
    /// the two apart - the same stale-reading failure already fixed for the
    /// issues snapshot, left live in the verification record.
    func head(at root: String) -> String? {
        let result = HiveProcess.run(
            executable: "/usr/bin/git",
            arguments: ["-C", root, "rev-parse", "HEAD"],
            timeout: 20
        )
        guard result.exitCode == 0 else { return nil }
        let head = result.standardOutput.trimmingCharacters(in: .whitespacesAndNewlines)
        return head.isEmpty ? nil : head
    }

    /// Whether a recorded verdict still describes the current tree.
    ///
    /// Four outcomes, not two: the commit may match, differ, or be unknown on
    /// either side. Unknown is never reported as current.
    static func evidenceState(
        verifiedAt: String?,
        currentHead: String?
    ) -> HiveEvidenceState {
        guard let verifiedAt, !verifiedAt.isEmpty else {
            return .unrecorded
        }
        guard let currentHead, !currentHead.isEmpty else {
            return .unknown("could not read the current commit")
        }
        return verifiedAt == currentHead ? .current : .stale(measuredAt: verifiedAt)
    }

    static func tail(_ text: String, lines: Int = 25) -> String {
        let kept = text
            .components(separatedBy: "\n")
            .filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
            .suffix(lines)
        return kept.joined(separator: "\n")
    }
}
