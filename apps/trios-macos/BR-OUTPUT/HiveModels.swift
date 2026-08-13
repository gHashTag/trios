import Foundation

// ===========================================================================
// HIVE - the Queen's worker model for self-directed development.
//
// A *target* is a part of this app. A *task* is one unit of improvement on a
// target. A *bee* is a Claude Code session executing exactly one task. The
// Queen ranks targets, dispatches bees, and holds their work for review.
//
// Ported from the superseded Trinity Queen package (gHashTag/trinity
// apps/queen). Paths go through ProjectPaths per L6; nothing here reaches
// outside apps/trios-macos.
// ===========================================================================

// MARK: - Signals

/// A signal is measured or it is not. It is never silently zero.
/// `unmeasured` carries the reason, so a probe that did not run can never be
/// mistaken for a reading of zero.
enum HiveSignalState: Equatable {
    case measured(Double)
    case unmeasured(String)

    var value: Double? {
        if case .measured(let v) = self { return v }
        return nil
    }

    var isMeasured: Bool { value != nil }

    var unmeasuredReason: String? {
        if case .unmeasured(let why) = self { return why }
        return nil
    }
}

/// One scored dimension of a module's need for work.
struct HiveSignal: Identifiable, Equatable {
    let kind: Kind
    /// Raw reading, before normalization (14 TODOs, 9 commits, and so on).
    let raw: HiveSignalState
    /// Reading mapped to 0...1 across the scanned set. Unmeasured stays so.
    let normalized: HiveSignalState

    var id: String { kind.rawValue }

    enum Kind: String, CaseIterable {
        case todoDensity
        case churn
        case testGap
        case sizeRisk
        case declaredIncomplete
        case openIssues

        /// Weights sum to 1.0 and are renormalized over the measured subset.
        var weight: Double {
            switch self {
            case .testGap: return 0.20
            case .openIssues: return 0.20
            case .todoDensity: return 0.18
            case .churn: return 0.18
            case .sizeRisk: return 0.12
            case .declaredIncomplete: return 0.12
            }
        }

        var label: String {
            switch self {
            case .todoDensity: return "TODO density"
            case .churn: return "Churn (30d)"
            case .testGap: return "Test gap"
            case .sizeRisk: return "Size risk"
            case .declaredIncomplete: return "Declared incomplete"
            case .openIssues: return "Open issues"
            }
        }

        /// What a bee is told to do when this signal dominates.
        var remedy: String {
            switch self {
            case .todoDensity:
                return "resolve or delete the TODO/FIXME markers, closing the ones already done"
            case .churn:
                return "stabilise the code that keeps changing - find the churn's root cause and remove it"
            case .testGap:
                return "add the tests this module does not have, starting with its error paths"
            case .sizeRisk:
                return "split the largest file along its real seams without changing behaviour"
            case .declaredIncomplete:
                return "finish what the module declares as stub/planned, or correct the declaration"
            case .openIssues:
                return "close the oldest open issue bound to this module"
            }
        }
    }
}

// MARK: - Facts

/// Everything measured about one part of the app. `nil` means not measured.
struct HiveModuleFacts: Equatable {
    let module: String
    let path: String
    let realm: Realm

    var lines: Int?
    var todos: Int?
    var churn30d: Int?
    var testBlocks: Int?
    var declaredStatus: String?
    var openIssues: Int?

    /// Why each unmeasured field could not be read, keyed by signal kind.
    var unmeasuredReasons: [HiveSignal.Kind: String] = [:]

    /// Which language a module is in decides which checker can verify a bee's
    /// work on it, so it is carried on the facts rather than guessed later.
    enum Realm: String, Equatable, CaseIterable {
        case swiftRing = "Swift"
        case rustRing = "Rust"
        case surface = "Surface"
    }

    init(module: String, path: String, realm: Realm) {
        self.module = module
        self.path = path
        self.realm = realm
    }
}

// MARK: - Targets

/// A ranked part of the app, with the arithmetic that ranked it exposed.
struct HiveTarget: Identifiable, Equatable {
    let module: String
    let path: String
    let realm: HiveModuleFacts.Realm
    let signals: [HiveSignal]
    /// 0...1, computed over the measured signals only.
    let score: Double
    /// Share of total weight actually measured. 1.0 means fully calibrated.
    let confidence: Double

    var id: String { module }

    var measuredCount: Int { signals.filter { $0.normalized.isMeasured }.count }

    /// The signals that actually drove the score, strongest first.
    var drivers: [HiveSignal] {
        signals
            .filter { ($0.normalized.value ?? 0) > 0 }
            .sorted {
                ($0.normalized.value ?? 0) * $0.kind.weight
                    > ($1.normalized.value ?? 0) * $1.kind.weight
            }
    }

    /// Justification naming the top two drivers and the confidence. It never
    /// names a signal that was not read.
    var reason: String {
        let top = drivers.prefix(2).map { signal -> String in
            let text: String
            switch signal.raw {
            case .measured(let v):
                text = v == v.rounded() ? String(Int(v)) : String(format: "%.2f", v)
            case .unmeasured:
                text = "?"
            }
            return "\(signal.kind.label) \(text)"
        }
        guard !top.isEmpty else {
            return "no positive signal measured (confidence \(Int(confidence * 100))%)"
        }
        return top.joined(separator: ", ") + " - confidence \(Int(confidence * 100))%"
    }

    var dominantKind: HiveSignal.Kind? { drivers.first?.kind }

    var dominantRemedy: String {
        drivers.first?.kind.remedy ?? "audit the module and report what is actually wrong"
    }
}

// MARK: - Tasks

enum HiveTaskState: String, Codable, Equatable {
    case pending
    case running
    case review
    case done
    case failed
    /// Attempt budget exhausted. Never picked again (MNL rule).
    case toxic

    var label: String {
        switch self {
        case .pending: return "PENDING"
        case .running: return "RUNNING"
        case .review: return "REVIEW"
        case .done: return "DONE"
        case .failed: return "FAILED"
        case .toxic: return "TOXIC"
        }
    }
}

/// One unit of work. Exactly one bee, exactly one session, at a time.
struct HiveTask: Codable, Identifiable, Equatable {
    var id: String
    var title: String
    var module: String
    var path: String
    var realm: String
    var signalKind: String
    var reason: String
    var score: Double
    var confidence: Double
    var prompt: String

    var state: HiveTaskState
    var attempts: Int
    var createdAt: Date
    var updatedAt: Date

    /// Claude Code session UUID - this is the chat, resumable by id.
    var sessionID: String?
    var branch: String?
    var lastError: String?
    var resultSummary: String?
    var costUSD: Double?
    var durationMs: Int?
    /// What an executed check said about the bee's work. `nil` = never run.
    var verification: String?
    /// `true` only when a check ran and passed, `false` when it ran and failed,
    /// `nil` when no check exists - which is not the same as either.
    var verified: Bool?

    init(
        id: String,
        title: String,
        module: String,
        path: String,
        realm: String,
        signalKind: String,
        reason: String,
        score: Double,
        confidence: Double,
        prompt: String
    ) {
        self.id = id
        self.title = title
        self.module = module
        self.path = path
        self.realm = realm
        self.signalKind = signalKind
        self.reason = reason
        self.score = score
        self.confidence = confidence
        self.prompt = prompt
        self.state = .pending
        self.attempts = 0
        self.createdAt = Date()
        self.updatedAt = Date()
    }

    /// Backwards-compatible decoding: older state files lack later fields.
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(String.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        module = try c.decode(String.self, forKey: .module)
        path = try c.decodeIfPresent(String.self, forKey: .path) ?? ""
        realm = try c.decodeIfPresent(String.self, forKey: .realm) ?? "Swift"
        signalKind = try c.decodeIfPresent(String.self, forKey: .signalKind) ?? ""
        reason = try c.decodeIfPresent(String.self, forKey: .reason) ?? ""
        score = try c.decodeIfPresent(Double.self, forKey: .score) ?? 0
        confidence = try c.decodeIfPresent(Double.self, forKey: .confidence) ?? 0
        prompt = try c.decodeIfPresent(String.self, forKey: .prompt) ?? ""
        state = try c.decodeIfPresent(HiveTaskState.self, forKey: .state) ?? .pending
        attempts = try c.decodeIfPresent(Int.self, forKey: .attempts) ?? 0
        createdAt = try c.decodeIfPresent(Date.self, forKey: .createdAt) ?? Date()
        updatedAt = try c.decodeIfPresent(Date.self, forKey: .updatedAt) ?? Date()
        sessionID = try c.decodeIfPresent(String.self, forKey: .sessionID)
        branch = try c.decodeIfPresent(String.self, forKey: .branch)
        lastError = try c.decodeIfPresent(String.self, forKey: .lastError)
        resultSummary = try c.decodeIfPresent(String.self, forKey: .resultSummary)
        costUSD = try c.decodeIfPresent(Double.self, forKey: .costUSD)
        durationMs = try c.decodeIfPresent(Int.self, forKey: .durationMs)
        verification = try c.decodeIfPresent(String.self, forKey: .verification)
        verified = try c.decodeIfPresent(Bool.self, forKey: .verified)
    }

    var isTerminal: Bool { state == .done || state == .toxic }

    /// Eligible for a bee when it has never succeeded and has attempts left.
    var isSchedulable: Bool { state == .pending || state == .failed }
}

// MARK: - Policy

/// Every guardrail on the loop, in one place. Persisted with the hive.
struct HivePolicy: Codable, Equatable {
    /// Master kill switch.
    var enabled: Bool = false
    var maxConcurrentBees: Int = 2
    var cycleIntervalSeconds: Int = 900
    /// Failures before a task is marked toxic and never retried (MNL).
    var maxAttemptsPerTask: Int = 3
    var maxBeesPerHour: Int = 6
    var targetsPerCycle: Int = 3
    var beeTimeoutSeconds: Int = 3600
    var maxBudgetUSDPerBee: Double = 5.0
    var model: String = "sonnet"
    var permissionMode: String = "acceptEdits"
    /// Isolate each bee in its own git worktree.
    var useWorktree: Bool = true
    /// Bees never push or open PRs. Flipping this is a deliberate act.
    var allowPush: Bool = false
    /// Execute the project's own checks before a bee's work reaches review.
    var verifyBeforeReview: Bool = true
    /// Consecutive failed bees before the loop trips its breaker and pauses.
    var maxConsecutiveFailures: Int = 3
    /// Ceiling on what every bee together may spend in one local day.
    var dailyBudgetUSD: Double = 25.0
    /// Bee transcripts older than this are pruned.
    var retainTranscriptDays: Int = 14
    /// Modules the operator has ruled out, with the reason.
    ///
    /// The ranking measures *weakness*, which is not *value*: the largest
    /// untested directory wins on arithmetic even when work there is worth
    /// little. Rather than encode that judgement as if it were a measurement,
    /// the human supplies it here and the audit records why.
    var skippedModules: [String: String] = [:]

    static let `default` = HivePolicy()

    /// Clamp entered values into ranges the loop can survive.
    func sanitized() -> HivePolicy {
        var p = self
        p.maxConcurrentBees = min(max(1, p.maxConcurrentBees), 8)
        p.cycleIntervalSeconds = min(max(30, p.cycleIntervalSeconds), 24 * 3600)
        p.maxAttemptsPerTask = min(max(1, p.maxAttemptsPerTask), 10)
        p.maxBeesPerHour = min(max(1, p.maxBeesPerHour), 60)
        p.targetsPerCycle = min(max(1, p.targetsPerCycle), 20)
        p.beeTimeoutSeconds = min(max(60, p.beeTimeoutSeconds), 12 * 3600)
        p.maxBudgetUSDPerBee = min(max(0.5, p.maxBudgetUSDPerBee), 100)
        p.maxConsecutiveFailures = min(max(1, p.maxConsecutiveFailures), 20)
        p.dailyBudgetUSD = min(max(1, p.dailyBudgetUSD), 1000)
        p.retainTranscriptDays = min(max(1, p.retainTranscriptDays), 365)
        // A per-bee budget above the daily ceiling would let one bee consume
        // the whole day before the ceiling could ever be consulted.
        p.maxBudgetUSDPerBee = min(p.maxBudgetUSDPerBee, p.dailyBudgetUSD)
        return p
    }

    private enum CodingKeys: String, CodingKey {
        case enabled, maxConcurrentBees, cycleIntervalSeconds, maxAttemptsPerTask
        case maxBeesPerHour, targetsPerCycle, beeTimeoutSeconds, maxBudgetUSDPerBee
        case model, permissionMode, useWorktree, allowPush, verifyBeforeReview
        case maxConsecutiveFailures, dailyBudgetUSD, retainTranscriptDays, skippedModules
    }

    init() {}

    /// Every field optional on decode, so a state file written by an older
    /// build keeps loading instead of resetting the operator's settings.
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let d = HivePolicy()
        enabled = try c.decodeIfPresent(Bool.self, forKey: .enabled) ?? d.enabled
        maxConcurrentBees = try c.decodeIfPresent(Int.self, forKey: .maxConcurrentBees) ?? d.maxConcurrentBees
        cycleIntervalSeconds = try c.decodeIfPresent(Int.self, forKey: .cycleIntervalSeconds) ?? d.cycleIntervalSeconds
        maxAttemptsPerTask = try c.decodeIfPresent(Int.self, forKey: .maxAttemptsPerTask) ?? d.maxAttemptsPerTask
        maxBeesPerHour = try c.decodeIfPresent(Int.self, forKey: .maxBeesPerHour) ?? d.maxBeesPerHour
        targetsPerCycle = try c.decodeIfPresent(Int.self, forKey: .targetsPerCycle) ?? d.targetsPerCycle
        beeTimeoutSeconds = try c.decodeIfPresent(Int.self, forKey: .beeTimeoutSeconds) ?? d.beeTimeoutSeconds
        maxBudgetUSDPerBee = try c.decodeIfPresent(Double.self, forKey: .maxBudgetUSDPerBee) ?? d.maxBudgetUSDPerBee
        model = try c.decodeIfPresent(String.self, forKey: .model) ?? d.model
        permissionMode = try c.decodeIfPresent(String.self, forKey: .permissionMode) ?? d.permissionMode
        useWorktree = try c.decodeIfPresent(Bool.self, forKey: .useWorktree) ?? d.useWorktree
        allowPush = try c.decodeIfPresent(Bool.self, forKey: .allowPush) ?? d.allowPush
        verifyBeforeReview = try c.decodeIfPresent(Bool.self, forKey: .verifyBeforeReview) ?? d.verifyBeforeReview
        maxConsecutiveFailures = try c.decodeIfPresent(Int.self, forKey: .maxConsecutiveFailures) ?? d.maxConsecutiveFailures
        dailyBudgetUSD = try c.decodeIfPresent(Double.self, forKey: .dailyBudgetUSD) ?? d.dailyBudgetUSD
        retainTranscriptDays = try c.decodeIfPresent(Int.self, forKey: .retainTranscriptDays) ?? d.retainTranscriptDays
        skippedModules = try c.decodeIfPresent([String: String].self, forKey: .skippedModules) ?? d.skippedModules
    }
}

// MARK: - Events and state

/// Append-only audit of everything the hive did.
struct HiveEvent: Codable, Identifiable, Equatable {
    var id: UUID
    var timestamp: Date
    var kind: String
    var taskID: String?
    var detail: String

    init(kind: String, taskID: String? = nil, detail: String) {
        self.id = UUID()
        self.timestamp = Date()
        self.kind = kind
        self.taskID = taskID
        self.detail = detail
    }
}

/// Persisted hive state. One file, atomically written.
struct HiveState: Codable, Equatable {
    var policy: HivePolicy
    var tasks: [HiveTask]
    var updatedAt: Date
    /// Dollars spent per local day, keyed `yyyy-MM-dd`. Survives restarts so a
    /// crash-loop cannot reset the day's ceiling by restarting the app.
    var spendByDay: [String: Double]

    init(policy: HivePolicy = .default, tasks: [HiveTask] = [], spendByDay: [String: Double] = [:]) {
        self.policy = policy
        self.tasks = tasks
        self.updatedAt = Date()
        self.spendByDay = spendByDay
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        policy = try c.decodeIfPresent(HivePolicy.self, forKey: .policy) ?? .default
        tasks = try c.decodeIfPresent([HiveTask].self, forKey: .tasks) ?? []
        updatedAt = try c.decodeIfPresent(Date.self, forKey: .updatedAt) ?? Date()
        spendByDay = try c.decodeIfPresent([String: Double].self, forKey: .spendByDay) ?? [:]
    }

    /// Local-day key. Local, not UTC: the ceiling is a human's daily budget.
    static func dayKey(_ date: Date = Date()) -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }

    func spent(on date: Date = Date()) -> Double {
        spendByDay[Self.dayKey(date)] ?? 0
    }

    mutating func record(spend: Double, on date: Date = Date()) {
        guard spend > 0 else { return }
        spendByDay[Self.dayKey(date), default: 0] += spend
        // Keep a fortnight of history; the rest is noise in a state file.
        let cutoff = Self.dayKey(date.addingTimeInterval(-14 * 86_400))
        spendByDay = spendByDay.filter { $0.key >= cutoff }
    }
}
