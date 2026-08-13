import Foundation

/// Reads this app's own tree and reports what it could measure about each
/// module - and, for every field it could not read, why.
///
/// The scanner never substitutes a zero for a failed reading. A module whose
/// git history could not be read reports `churn30d = nil` plus the reason, and
/// the priority engine drops that weight from the denominator.
struct HiveRepoScanner {

    /// Root of the app tree (`apps/trios-macos`), not the whole monorepo.
    let projectRoot: String
    /// Repository root, used only for `git log`, which needs the work tree.
    let gitRoot: String

    static let sourceExtensions: Set<String> = ["swift", "rs", "t27", "tri"]
    /// Files larger than this are counted by line only - no content scan.
    static let maxScannedFileBytes = 2 * 1024 * 1024

    /// Directories that are not this app's source and must not be measured as
    /// if they were. Two separate harms: walking `target/` and `.build/` took
    /// the scan from seconds to over a minute, and counting generated or
    /// vendored code inflates a module's size and TODO density with lines
    /// nobody wrote and no bee should edit.
    static let excludedDirectories: Set<String> = [
        ".build", ".git", ".swiftpm", "target", "vendor", "node_modules",
        "Pods", "DerivedData", "gen", "generated", "_to_delete",
    ]

    static func isExcluded(_ url: URL, relativeTo base: String) -> Bool {
        let path = url.path
        guard path.hasPrefix(base) else { return false }
        return String(path.dropFirst(base.count))
            .components(separatedBy: "/")
            .contains { excludedDirectories.contains($0) }
    }
    /// A snapshot older than this is not a measurement of today's repository.
    static let maxIssueSnapshotAgeDays = 7

    init(projectRoot: String = ProjectPaths.root, gitRoot: String? = nil) {
        self.projectRoot = projectRoot
        // apps/trios-macos -> apps -> repo root
        self.gitRoot = gitRoot
            ?? URL(fileURLWithPath: projectRoot)
                .deletingLastPathComponent()
                .deletingLastPathComponent()
                .path
    }

    /// A realm is a family of modules that share a checker. `rings` holds one
    /// directory per module; `BR-OUTPUT` is a flat file tree treated as one.
    struct RealmRoot {
        let relative: String
        /// Sub-directories are modules. When false the root itself is a module.
        let subdirectoriesAreModules: Bool
        /// Where tests for this realm live, when they live outside the module.
        let externalTests: String?
    }

    var realmRoots: [RealmRoot] {
        [
            RealmRoot(relative: "rings", subdirectoriesAreModules: true, externalTests: "tests"),
            RealmRoot(relative: "BR-OUTPUT", subdirectoriesAreModules: false, externalTests: "tests"),
        ]
    }

    /// Swift rings are `SR-*`, Rust rings are `RUST-*`. The distinction decides
    /// which checker can verify a bee's work, so it is recorded, not guessed.
    static func realm(forModule module: String) -> HiveModuleFacts.Realm {
        let name = URL(fileURLWithPath: module).lastPathComponent
        if name.hasPrefix("RUST-") { return .rustRing }
        if name.hasPrefix("SR-") { return .swiftRing }
        return .surface
    }

    // MARK: - Entry point

    func scan() -> [HiveModuleFacts] {
        let fm = FileManager.default
        let churn = readChurn()
        let issues = readIssueCounts()

        var results: [HiveModuleFacts] = []

        for root in realmRoots {
            let rootPath = "\(projectRoot)/\(root.relative)"
            var isDir: ObjCBool = false
            guard fm.fileExists(atPath: rootPath, isDirectory: &isDir), isDir.boolValue else {
                continue
            }

            let externalTests = root.externalTests.map {
                attributeExternalTests(
                    testsRoot: "\(projectRoot)/\($0)",
                    moduleRoot: rootPath,
                    modulePrefix: root.relative,
                    flat: !root.subdirectoriesAreModules
                )
            }

            var moduleKeys: [(key: String, path: String)] = []
            if root.subdirectoriesAreModules {
                let entries = (try? fm.contentsOfDirectory(atPath: rootPath)) ?? []
                for entry in entries.sorted() where !entry.hasPrefix(".") {
                    var entryIsDir: ObjCBool = false
                    let path = "\(rootPath)/\(entry)"
                    guard fm.fileExists(atPath: path, isDirectory: &entryIsDir),
                          entryIsDir.boolValue else { continue }
                    moduleKeys.append((key: "\(root.relative)/\(entry)", path: path))
                }
            } else {
                moduleKeys.append((key: root.relative, path: rootPath))
            }

            for module in moduleKeys {
                var facts = HiveModuleFacts(
                    module: module.key,
                    path: module.key,
                    realm: Self.realm(forModule: module.key)
                )

                let content = measureContent(at: module.path)
                if content.lines == 0 {
                    let why = "no source files under \(module.key)"
                    facts.unmeasuredReasons[.sizeRisk] = why
                    facts.unmeasuredReasons[.todoDensity] = why
                    facts.unmeasuredReasons[.testGap] = why
                } else {
                    facts.lines = content.lines
                    facts.todos = content.todos
                    facts.testBlocks = content.testBlocks + (externalTests?[module.key] ?? 0)
                }

                switch churn {
                case .success(let table):
                    facts.churn30d = table[module.key] ?? 0
                case .failure(let why):
                    facts.unmeasuredReasons[.churn] = why
                }

                switch issues {
                case .success(let table):
                    facts.openIssues = table[module.key] ?? 0
                case .failure(let why):
                    facts.unmeasuredReasons[.openIssues] = why
                }

                // Nothing in this app declares a per-module status the way
                // trinity's cell.tri did. Saying so beats scoring a zero.
                facts.unmeasuredReasons[.declaredIncomplete] =
                    "this app declares no per-module status"

                results.append(facts)
            }
        }

        return results
    }

    // MARK: - Content

    struct ContentMeasurement: Equatable {
        var lines = 0
        var todos = 0
        var testBlocks = 0
    }

    func measureContent(at directory: String) -> ContentMeasurement {
        var out = ContentMeasurement()
        let fm = FileManager.default
        guard let walker = fm.enumerator(
            at: URL(fileURLWithPath: directory),
            includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey],
            options: [.skipsHiddenFiles]
        ) else { return out }

        let base = directory.hasSuffix("/") ? directory : directory + "/"
        for case let url as URL in walker {
            if Self.excludedDirectories.contains(url.lastPathComponent) {
                walker.skipDescendants()
                continue
            }
            guard Self.sourceExtensions.contains(url.pathExtension) else { continue }
            guard !Self.isExcluded(url, relativeTo: base) else { continue }
            let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
            guard values?.isRegularFile == true else { continue }
            if let size = values?.fileSize, size > Self.maxScannedFileBytes { continue }
            guard let text = try? String(contentsOf: url, encoding: .utf8) else { continue }

            let counts = Self.count(in: text)
            out.lines += counts.lines
            out.todos += counts.todos
            out.testBlocks += counts.testBlocks
        }
        return out
    }

    /// Line, marker and test-block counts for one file's text. Split out so the
    /// counting rule is testable without touching disk.
    static func count(in text: String) -> ContentMeasurement {
        var out = ContentMeasurement()
        text.enumerateLines { line, _ in
            out.lines += 1
            if line.contains("TODO") || line.contains("FIXME")
                || line.contains("XXX") || line.contains("HACK") {
                out.todos += 1
            }
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            // Swift XCTest and swift-testing, plus Rust's #[test].
            if trimmed.hasPrefix("func test")
                || trimmed.hasPrefix("@Test")
                || trimmed.hasPrefix("#[test]")
                || trimmed.hasPrefix("#[tokio::test]") {
                out.testBlocks += 1
            }
        }
        return out
    }

    // MARK: - External tests

    /// Attributes test blocks living outside the module tree back to a module.
    ///
    /// A test file is credited to the module whose source-file stems it names
    /// most often, and to exactly one module - so a shared helper cannot
    /// inflate the coverage of everything it touches. A test file that matches
    /// nothing has its blocks dropped rather than spread around.
    func attributeExternalTests(
        testsRoot: String,
        moduleRoot: String,
        modulePrefix: String,
        flat: Bool
    ) -> [String: Int] {
        let fm = FileManager.default
        guard fm.fileExists(atPath: testsRoot) else { return [:] }

        // Both sides are symlink-resolved: on macOS a temporary directory is
        // reached as /var/... but enumerated as /private/var/..., and comparing
        // the two raw strings silently yields an empty module name.
        let moduleBase = URL(fileURLWithPath: moduleRoot).resolvingSymlinksInPath()
        let basePath = moduleBase.path + "/"
        var stemOwner: [String: String] = [:]

        if let walker = fm.enumerator(
            at: moduleBase,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) {
            for case let url as URL in walker {
                if Self.excludedDirectories.contains(url.lastPathComponent) {
                    walker.skipDescendants()
                    continue
                }
                guard Self.sourceExtensions.contains(url.pathExtension) else { continue }
                let stem = url.deletingPathExtension().lastPathComponent
                if flat {
                    stemOwner[stem] = modulePrefix
                    continue
                }
                let path = url.resolvingSymlinksInPath().path
                guard path.hasPrefix(basePath) else { continue }
                let relative = String(path.dropFirst(basePath.count))
                let components = relative.components(separatedBy: "/")
                guard components.count > 1, let module = components.first, !module.isEmpty else {
                    continue
                }
                stemOwner[stem] = "\(modulePrefix)/\(module)"
            }
        }

        var counts: [String: Int] = [:]
        guard let testWalker = fm.enumerator(
            at: URL(fileURLWithPath: testsRoot),
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return counts }

        for case let url as URL in testWalker {
            if Self.excludedDirectories.contains(url.lastPathComponent) {
                testWalker.skipDescendants()
                continue
            }
            guard Self.sourceExtensions.contains(url.pathExtension) else { continue }
            guard let text = try? String(contentsOf: url, encoding: .utf8) else { continue }
            let blocks = Self.count(in: text).testBlocks
            guard blocks > 0 else { continue }

            // Tokenise the test file once and intersect, rather than running
            // `text.contains(stem)` for every stem. The naive form is
            // stems x files x text substring searches over grapheme-aware
            // Swift strings, and it cost 68s on this tree - more than the git
            // log and the whole file walk put together.
            let tokens = Self.identifiers(in: text)
            var hits: [String: Int] = [:]
            for (stem, module) in stemOwner where tokens.contains(stem) {
                hits[module, default: 0] += 1
            }
            guard let best = hits.max(by: {
                $0.value != $1.value ? $0.value < $1.value : $0.key > $1.key
            }) else { continue }
            counts[best.key, default: 0] += blocks
        }

        return counts
    }

    /// Every identifier-shaped token in a source file, as a set.
    ///
    /// Matching on whole tokens is also more correct than substring search: a
    /// file naming `ChatMessageStore` no longer credits `ChatMessage`.
    static func identifiers(in text: String) -> Set<String> {
        var tokens = Set<String>()
        var current = String.UnicodeScalarView()
        for scalar in text.unicodeScalars {
            if CharacterSet.alphanumerics.contains(scalar) || scalar == "_" {
                current.append(scalar)
            } else if !current.isEmpty {
                tokens.insert(String(current))
                current = String.UnicodeScalarView()
            }
        }
        if !current.isEmpty { tokens.insert(String(current)) }
        return tokens
    }

    // MARK: - Readings

    enum Reading<T> {
        case success(T)
        case failure(String)
    }

    /// Commits per module over the last 30 days, from `git log --name-only`.
    /// Returns a reason rather than an empty table when git cannot be read.
    func readChurn() -> Reading<[String: Int]> {
        let result = HiveProcess.run(
            executable: "/usr/bin/git",
            arguments: [
                "-C", gitRoot,
                "log", "--since=30.days", "--name-only", "--pretty=format:%H",
            ],
            timeout: 30
        )
        // Paths in the log are repo-relative; module keys are app-relative.
        let appPrefix = URL(fileURLWithPath: projectRoot).lastPathComponent
        return Self.churnReading(
            from: result,
            prefixes: realmRoots.map(\.relative),
            pathPrefix: "apps/\(appPrefix)/",
            flatRoots: Set(realmRoots.filter { !$0.subdirectoriesAreModules }.map(\.relative))
        )
    }

    /// Decides whether a `git log` run is a reading at all, then parses it.
    ///
    /// Split from the call so the rule can be tested without a repository -
    /// the rule being the whole point of the file, and the one that was
    /// silently violated from underneath.
    static func churnReading(
        from result: HiveProcess.Result,
        prefixes: [String],
        pathPrefix: String = "",
        flatRoots: Set<String> = []
    ) -> Reading<[String: Int]> {
        guard result.exitCode == 0 else {
            let detail = result.standardError.trimmingCharacters(in: .whitespacesAndNewlines)
            return .failure(detail.isEmpty ? "git log exited \(result.exitCode)" : detail)
        }
        // The exit code is not enough to call this a reading.
        //
        // A log that was only partly read parses into a table that is missing
        // modules, and a missing module is written out as `churn30d = 0` - a
        // measured zero, with its 0.18 of the weight still in the denominator
        // and the confidence reported as if the probe had worked. That is the
        // absent-read-as-zero failure this whole file exists to prevent,
        // entered from underneath: git exits 0, and the truncation happens in
        // the reader. So a bounded read is a failed reading, with the reason
        // carried forward like any other.
        if result.timedOut {
            return .failure("git log did not finish within its timeout - churn unread, not zero")
        }
        if result.outputTruncated {
            return .failure(
                "git log exited 0 but its output was not read to the end - a partial log would "
                    + "report churn 0 for every module it did not reach"
            )
        }
        return .success(
            parseChurn(
                result.standardOutput,
                prefixes: prefixes,
                pathPrefix: pathPrefix,
                flatRoots: flatRoots
            )
        )
    }

    /// One commit counts once per module it touched.
    static func parseChurn(
        _ log: String,
        prefixes: [String],
        pathPrefix: String = "",
        flatRoots: Set<String> = []
    ) -> [String: Int] {
        var counts: [String: Int] = [:]
        var currentModules = Set<String>()

        func flush() {
            for module in currentModules { counts[module, default: 0] += 1 }
            currentModules.removeAll()
        }

        for rawLine in log.components(separatedBy: "\n") {
            if rawLine.isEmpty { continue }
            if rawLine.count == 40, rawLine.allSatisfy({ $0.isHexDigit }) {
                flush()
                continue
            }
            guard rawLine.hasPrefix(pathPrefix) else { continue }
            let line = String(rawLine.dropFirst(pathPrefix.count))
            for prefix in prefixes where line.hasPrefix("\(prefix)/") {
                if flatRoots.contains(prefix) {
                    currentModules.insert(prefix)
                    continue
                }
                let remainder = line.dropFirst(prefix.count + 1)
                guard let segment = remainder.components(separatedBy: "/").first,
                      !segment.isEmpty else { continue }
                currentModules.insert("\(prefix)/\(segment)")
            }
        }
        flush()
        return counts
    }

    /// Open issues attributed to a module by mention.
    ///
    /// An absent snapshot is reported as unreadable, never as "no issues" - and
    /// a *stale* snapshot is reported as unreadable too. In the tree this was
    /// ported from, that file was 116 days old and was being scored as a
    /// current reading, which is the exact failure this instrument prevents.
    func readIssueCounts(now: Date = Date()) -> Reading<[String: Int]> {
        // `self.projectRoot`, not the global ProjectPaths.
        //
        // Every other reading in this scanner is relative to the root it was
        // constructed with; this one reached for the process-wide path instead.
        // A scanner built for an explicit root therefore read a snapshot from
        // somewhere else, or from nowhere - and the failure surfaces as
        // "no .trinity/issues_snapshot.json on disk", which reads as an honest
        // unmeasured signal rather than as a broken probe. The sibling copy in
        // the Queen package always used its own root, so the two had drifted
        // apart here without anything noticing.
        let path = "\(projectRoot)/.trinity/issues_snapshot.json"
        let fm = FileManager.default
        guard let data = fm.contents(atPath: path) else {
            return .failure("no .trinity/issues_snapshot.json on disk")
        }
        guard let modified = (try? fm.attributesOfItem(atPath: path)[.modificationDate]) as? Date else {
            return .failure("issues snapshot has no modification date - age unknown")
        }
        let ageDays = Int(now.timeIntervalSince(modified) / 86_400)
        if ageDays > Self.maxIssueSnapshotAgeDays {
            return .failure("issues snapshot is \(ageDays) days stale - refresh it before this signal counts")
        }
        guard let text = String(data: data, encoding: .utf8) else {
            return .failure("issues_snapshot.json is not utf-8")
        }
        return .success(
            Self.parseIssueMentions(
                text,
                prefixes: realmRoots.map(\.relative),
                flatRoots: Set(realmRoots.filter { !$0.subdirectoriesAreModules }.map(\.relative))
            )
        )
    }

    /// Counts how often each module is named in the snapshot text.
    /// Deliberately crude: it is a weak signal and is labelled as one.
    ///
    /// A flat root is one module, so `BR-OUTPUT/Foo.swift` counts against
    /// `BR-OUTPUT` - not against a `BR-OUTPUT/Foo` key that matches no module
    /// and therefore silently scores zero everywhere.
    static func parseIssueMentions(
        _ text: String,
        prefixes: [String],
        flatRoots: Set<String> = []
    ) -> [String: Int] {
        var counts: [String: Int] = [:]
        for prefix in prefixes {
            var search = text[...]
            while let range = search.range(of: "\(prefix)/") {
                let rest = search[range.upperBound...]
                if flatRoots.contains(prefix) {
                    counts[prefix, default: 0] += 1
                    search = rest
                    continue
                }
                let segment = rest.prefix { $0.isLetter || $0.isNumber || $0 == "_" || $0 == "-" }
                if !segment.isEmpty {
                    counts["\(prefix)/\(segment)", default: 0] += 1
                }
                search = rest
            }
        }
        return counts
    }
}
