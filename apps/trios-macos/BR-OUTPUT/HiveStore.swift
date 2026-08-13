import Foundation

/// Durable hive state: one JSON document for tasks and policy, one append-only
/// JSONL for the audit trail. Both live under the app's runtime data root so
/// anything else in the fleet can read the same ground truth.
struct HiveStore {

    let stateRoot: String

    init(stateRoot: String = ProjectPaths.trinity) {
        self.stateRoot = stateRoot
    }

    var stateURL: URL {
        URL(fileURLWithPath: stateRoot).appendingPathComponent("hive/hive.json")
    }

    var eventsURL: URL {
        URL(fileURLWithPath: stateRoot).appendingPathComponent("hive/hive_events.jsonl")
    }

    var transcriptsDirectory: URL {
        URL(fileURLWithPath: stateRoot).appendingPathComponent("hive/transcripts", isDirectory: true)
    }

    private func ensureDirectory() {
        try? FileManager.default.createDirectory(
            at: stateURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
    }

    // MARK: - State

    func load() -> HiveState {
        guard let data = try? Data(contentsOf: stateURL) else { return HiveState() }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        guard var state = try? decoder.decode(HiveState.self, from: data) else {
            return HiveState()
        }
        state.policy = state.policy.sanitized()
        // The rate window is cut to the last hour here, at the one place every
        // load passes through. A window carried whole out of a file written
        // three days ago is not a measurement of this hour, and replaying it
        // would block the loop for an hour after every launch.
        state.spawnWindow = HiveState.prunedSpawnWindow(state.spawnWindow)
        // A bee cannot survive an app restart, so anything left `running` in
        // the file is a crash artefact. It returns to the queue rather than
        // being silently counted as either a success or a failure.
        for index in state.tasks.indices where state.tasks[index].state == .running {
            state.tasks[index].state = .pending
            state.tasks[index].lastError = "interrupted - the app restarted while this bee was working"
        }
        return state
    }

    @discardableResult
    func save(_ state: HiveState) -> Bool {
        ensureDirectory()
        var snapshot = state
        snapshot.updatedAt = Date()
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(snapshot) else { return false }
        return (try? data.write(to: stateURL, options: .atomic)) != nil
    }

    // MARK: - Events

    func append(_ event: HiveEvent) {
        ensureDirectory()
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        guard let data = try? encoder.encode(event),
              let line = String(data: data, encoding: .utf8),
              let payload = (line + "\n").data(using: .utf8) else { return }

        if let handle = try? FileHandle(forWritingTo: eventsURL) {
            defer { try? handle.close() }
            _ = try? handle.seekToEnd()
            try? handle.write(contentsOf: payload)
        } else {
            try? payload.write(to: eventsURL, options: .atomic)
        }
    }

    func recentEvents(limit: Int = 50) -> [HiveEvent] {
        guard let text = try? String(contentsOf: eventsURL, encoding: .utf8) else { return [] }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return text
            .components(separatedBy: "\n")
            .suffix(limit)
            .compactMap { line -> HiveEvent? in
                guard let data = line.data(using: .utf8) else { return nil }
                return try? decoder.decode(HiveEvent.self, from: data)
            }
            .reversed()
    }

    // MARK: - Retention

    /// Prunes bee transcripts older than `days`. A loop that runs for weeks
    /// writes one per bee; without this the state directory grows without end.
    @discardableResult
    func pruneTranscripts(olderThanDays days: Int, now: Date = Date()) -> Int {
        let fm = FileManager.default
        let cutoff = now.addingTimeInterval(-Double(days) * 86_400)
        guard let files = try? fm.contentsOfDirectory(
            at: transcriptsDirectory,
            includingPropertiesForKeys: [.contentModificationDateKey]
        ) else { return 0 }

        var removed = 0
        for url in files where url.pathExtension == "jsonl" {
            let modified = (try? url.resourceValues(forKeys: [.contentModificationDateKey]))?
                .contentModificationDate
            guard let modified, modified < cutoff else { continue }
            if (try? fm.removeItem(at: url)) != nil { removed += 1 }
        }
        return removed
    }
}

// MARK: - Transcripts

/// Reads back what a bee wrote before the runtime lost sight of it.
///
/// `HiveBeeRunner` streams every line to its transcript as it arrives, so a
/// bee interrupted by a crash, a rebuild or a Stop-all leaves its own evidence
/// on disk. That file is the only thing that can tell a reservation apart from
/// a real cost once the process is gone.
enum HiveTranscript {

    static func summarise(at url: URL) -> HiveTranscriptSummary {
        guard let text = try? String(contentsOf: url, encoding: .utf8) else { return .absent }
        var lines = 0
        var lastCost: Double?
        for line in text.components(separatedBy: "\n") where !line.trimmingCharacters(in: .whitespaces).isEmpty {
            lines += 1
            guard line.contains("total_cost_usd"),
                  let data = line.data(using: .utf8),
                  let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let cost = object["total_cost_usd"] as? Double else { continue }
            lastCost = cost
        }
        return HiveTranscriptSummary(lines: lines, lastCostUSD: lastCost)
    }
}

/// Whether a process id still names a running process.
///
/// `kill(pid, 0)` sends nothing; it only asks whether the process exists and
/// could be signalled. The hive deliberately stops there and never signals a
/// pid it read out of a file: pids are reused, and the process wearing this one
/// after a reboot is somebody else's. An orphan is therefore reported and
/// worked around - its worktree name is unique to its session, so no later bee
/// can be dispatched into the directory it is still writing to - rather than
/// killed on a guess.
enum HiveProcessLiveness {
    static func isAlive(_ pid: Int32) -> Bool {
        guard pid > 0 else { return false }
        return kill(pid, 0) == 0 || errno == EPERM
    }
}

// MARK: - Rate limiting

/// Rolling-window rate limiter for bee spawns.
///
/// Seeded from the persisted window and written back on every spawn, so the
/// bound it enforces - at most `maxBeesPerHour` spawns in any rolling hour -
/// is a statement about the machine rather than about one launch of the app.
///
/// It used to be in-memory only, justified as "a restart is a human-initiated
/// event that should not inherit an old window's debt". That justification
/// depends on a fact about the environment, and the environment stopped
/// supplying it: a watchdog relaunches this app within 60s of it dying, so a
/// crash-loop resets the window as fast as it can crash. The bound is now
/// correct by construction instead of correct by assumption, which is what
/// makes auto-resume on launch safe to add.
struct HiveRateLimiter {
    private(set) var spawnTimes: [Date] = []

    init(spawnTimes: [Date] = [], now: Date = Date()) {
        self.spawnTimes = spawnTimes
        prune(now)
    }

    mutating func record(_ now: Date = Date()) {
        spawnTimes.append(now)
        prune(now)
    }

    mutating func prune(_ now: Date = Date()) {
        let cutoff = now.addingTimeInterval(-3600)
        spawnTimes.removeAll { $0 < cutoff }
    }

    func spawnsInLastHour(_ now: Date = Date()) -> Int {
        let cutoff = now.addingTimeInterval(-3600)
        return spawnTimes.filter { $0 >= cutoff }.count
    }

    func allows(limit: Int, now: Date = Date()) -> Bool {
        spawnsInLastHour(now) < limit
    }
}
