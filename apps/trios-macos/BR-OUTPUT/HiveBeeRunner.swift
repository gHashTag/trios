import Foundation

// ===========================================================================
// BEE - one Claude Code session, one task, one resumable chat.
//
// The Queen spawns `claude -p` with a fixed session id, so the work she starts
// is a real conversation the operator can reopen with `claude --resume <id>`.
// ===========================================================================

enum HiveBeeStatus: String, Equatable {
    case starting
    case working
    case succeeded
    case failed
    case cancelled
    case timedOut

    var label: String {
        switch self {
        case .starting: return "STARTING"
        case .working: return "WORKING"
        case .succeeded: return "DONE"
        case .failed: return "FAILED"
        case .cancelled: return "CANCELLED"
        case .timedOut: return "TIMED OUT"
        }
    }

    var isTerminal: Bool { self != .starting && self != .working }
}

/// One decoded line of a bee's stream.
struct HiveBeeEvent: Equatable {
    let timestamp: Date
    let kind: Kind
    let text: String

    enum Kind: Equatable {
        case system
        case assistant
        case tool(String)
        case toolResult
        case result(success: Bool)
        case raw
    }
}

/// Outcome handed back when the process exits.
struct HiveBeeOutcome: Equatable {
    var status: HiveBeeStatus
    var summary: String
    var costUSD: Double?
    var durationMs: Int?
    var sessionID: String?
}

/// Spawns and drains one `claude -p` process.
///
/// Not an actor: `Process` and its pipe handlers are callback-driven, so the
/// runner keeps its own lock and hands results back on a caller-chosen queue.
final class HiveBeeRunner: @unchecked Sendable {

    struct Configuration: Equatable {
        var executable: String
        var workingDirectory: String
        var prompt: String
        var sessionID: String
        var model: String
        var permissionMode: String
        var maxBudgetUSD: Double
        var timeoutSeconds: Int
        var worktreeName: String?
        var displayName: String
    }

    private let configuration: Configuration
    private let queue = DispatchQueue(label: "com.trios.hive.bee", qos: .utility)
    private let lock = NSLock()
    private var process: Process?
    private var pendingLine = ""

    init(configuration: Configuration) {
        self.configuration = configuration
    }

    /// The command line handed to Claude Code. Pure, so tests can assert on it
    /// without spawning anything.
    static func arguments(for configuration: Configuration) -> [String] {
        var args: [String] = [
            "-p", configuration.prompt,
            "--output-format", "stream-json",
            "--verbose",
            "--session-id", configuration.sessionID,
            "--permission-mode", configuration.permissionMode,
            "--model", configuration.model,
            "--max-budget-usd", String(format: "%.2f", configuration.maxBudgetUSD),
            "-n", configuration.displayName,
        ]
        if let worktree = configuration.worktreeName {
            args.append(contentsOf: ["--worktree", worktree])
        }
        return args
    }

    /// Starts the bee. `onEvent` fires per decoded stream line, `onFinish`
    /// exactly once, both on `deliverOn`.
    func start(
        deliverOn: DispatchQueue = .main,
        transcriptURL: URL? = nil,
        onEvent: @escaping (HiveBeeEvent) -> Void,
        onFinish: @escaping (HiveBeeOutcome) -> Void
    ) {
        let configuration = self.configuration

        let process = Process()
        process.executableURL = URL(fileURLWithPath: configuration.executable)
        process.arguments = Self.arguments(for: configuration)
        process.currentDirectoryURL = URL(fileURLWithPath: configuration.workingDirectory)

        var environment = ProcessInfo.processInfo.environment
        environment["TRIOS_HIVE_TASK"] = configuration.displayName
        process.environment = environment

        let outPipe = Pipe()
        let errPipe = Pipe()
        process.standardOutput = outPipe
        process.standardError = errPipe

        let errorBox = HiveDataBox()
        let resultBox = HiveResultBox()

        outPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty, let chunk = String(data: data, encoding: .utf8) else { return }
            guard let self else { return }
            for line in self.split(chunk) {
                if let transcriptURL { Self.appendTranscript(line, to: transcriptURL) }
                guard let event = Self.decode(line) else { continue }
                if case .result = event.kind { resultBox.record(line: line) }
                deliverOn.async { onEvent(event) }
            }
        }

        errPipe.fileHandleForReading.readabilityHandler = { handle in
            let data = handle.availableData
            guard !data.isEmpty else { return }
            errorBox.value.append(data)
        }

        process.terminationHandler = { finished in
            outPipe.fileHandleForReading.readabilityHandler = nil
            errPipe.fileHandleForReading.readabilityHandler = nil
            let stderrText = String(data: errorBox.value, encoding: .utf8) ?? ""
            let outcome = Self.outcome(
                exitCode: finished.terminationStatus,
                resultBox: resultBox,
                stderr: stderrText,
                fallbackSessionID: configuration.sessionID
            )
            deliverOn.async { onFinish(outcome) }
        }

        do {
            try process.run()
        } catch {
            deliverOn.async {
                onFinish(
                    HiveBeeOutcome(
                        status: .failed,
                        summary: "could not launch claude: \(error.localizedDescription)",
                        costUSD: nil,
                        durationMs: nil,
                        sessionID: configuration.sessionID
                    )
                )
            }
            return
        }

        lock.lock()
        self.process = process
        lock.unlock()

        // Wall-clock guard, independent of anything Claude Code reports.
        queue.asyncAfter(deadline: .now() + .seconds(configuration.timeoutSeconds)) { [weak self] in
            guard let self else { return }
            self.lock.lock()
            let running = self.process?.isRunning ?? false
            self.lock.unlock()
            if running {
                resultBox.markTimedOut()
                self.terminate()
            }
        }
    }

    func terminate() {
        lock.lock()
        let process = self.process
        lock.unlock()
        guard let process, process.isRunning else { return }
        process.terminate()
    }

    // MARK: - Line assembly

    /// Claude Code emits one JSON object per line, but a pipe read can split
    /// mid-line, so a partial line is carried into the next chunk.
    private func split(_ chunk: String) -> [String] {
        lock.lock()
        pendingLine += chunk
        var parts = pendingLine.components(separatedBy: "\n")
        pendingLine = parts.removeLast()
        lock.unlock()
        return parts.filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
    }

    // MARK: - Decoding

    static func decode(_ line: String) -> HiveBeeEvent? {
        guard let data = line.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let type = object["type"] as? String else {
            return HiveBeeEvent(timestamp: Date(), kind: .raw, text: String(line.prefix(240)))
        }

        switch type {
        case "system":
            return HiveBeeEvent(
                timestamp: Date(), kind: .system, text: object["subtype"] as? String ?? "system"
            )
        case "assistant":
            guard let message = object["message"] as? [String: Any],
                  let content = message["content"] as? [[String: Any]] else {
                return HiveBeeEvent(timestamp: Date(), kind: .assistant, text: "")
            }
            for block in content
            where (block["type"] as? String) == "tool_use" {
                if let name = block["name"] as? String {
                    return HiveBeeEvent(timestamp: Date(), kind: .tool(name), text: name)
                }
            }
            let text = content
                .filter { ($0["type"] as? String) == "text" }
                .compactMap { $0["text"] as? String }
                .joined(separator: " ")
            return HiveBeeEvent(timestamp: Date(), kind: .assistant, text: text)
        case "user":
            return HiveBeeEvent(timestamp: Date(), kind: .toolResult, text: "")
        case "result":
            let isError = object["is_error"] as? Bool ?? false
            let subtype = object["subtype"] as? String ?? ""
            return HiveBeeEvent(
                timestamp: Date(),
                kind: .result(success: !isError && subtype == "success"),
                text: object["result"] as? String ?? subtype
            )
        default:
            return nil
        }
    }

    /// Decides the bee's fate from the exit code and the terminal `result`
    /// line. A missing result line is a failure, never an assumed success.
    ///
    /// Note `is_error: true` can arrive alongside `subtype: "success"` - a
    /// signed-out CLI reports exactly that - so both are required.
    static func outcome(
        exitCode: Int32,
        resultBox: HiveResultBox,
        stderr: String,
        fallbackSessionID: String
    ) -> HiveBeeOutcome {
        if resultBox.timedOut {
            return HiveBeeOutcome(
                status: .timedOut,
                summary: "killed after exceeding its wall-clock budget",
                costUSD: resultBox.costUSD,
                durationMs: resultBox.durationMs,
                sessionID: fallbackSessionID
            )
        }

        guard let line = resultBox.line,
              let data = line.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            let detail = stderr.trimmingCharacters(in: .whitespacesAndNewlines)
            return HiveBeeOutcome(
                status: .failed,
                summary: detail.isEmpty
                    ? "exited \(exitCode) without reporting a result"
                    : String(detail.suffix(400)),
                costUSD: nil,
                durationMs: nil,
                sessionID: fallbackSessionID
            )
        }

        let isError = object["is_error"] as? Bool ?? false
        let subtype = object["subtype"] as? String ?? ""
        let succeeded = !isError && subtype == "success" && exitCode == 0
        return HiveBeeOutcome(
            status: succeeded ? .succeeded : .failed,
            summary: object["result"] as? String ?? subtype,
            costUSD: object["total_cost_usd"] as? Double,
            durationMs: object["duration_ms"] as? Int,
            sessionID: object["session_id"] as? String ?? fallbackSessionID
        )
    }

    // MARK: - Transcript

    static func appendTranscript(_ line: String, to url: URL) {
        try? FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true
        )
        guard let data = (line + "\n").data(using: .utf8) else { return }
        if let handle = try? FileHandle(forWritingTo: url) {
            defer { try? handle.close() }
            _ = try? handle.seekToEnd()
            try? handle.write(contentsOf: data)
        } else {
            try? data.write(to: url, options: .atomic)
        }
    }
}

/// Carries the terminal `result` line across threads.
final class HiveResultBox: @unchecked Sendable {
    private let lock = NSLock()
    private var _line: String?
    private var _timedOut = false

    var line: String? {
        lock.lock(); defer { lock.unlock() }
        return _line
    }

    var timedOut: Bool {
        lock.lock(); defer { lock.unlock() }
        return _timedOut
    }

    var costUSD: Double? { field("total_cost_usd") as? Double }
    var durationMs: Int? { field("duration_ms") as? Int }

    func record(line: String) {
        lock.lock(); defer { lock.unlock() }
        _line = line
    }

    func markTimedOut() {
        lock.lock(); defer { lock.unlock() }
        _timedOut = true
    }

    private func field(_ key: String) -> Any? {
        guard let line,
              let data = line.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return nil
        }
        return object[key]
    }
}
