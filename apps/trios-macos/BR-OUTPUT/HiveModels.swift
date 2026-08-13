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

        /// The declared weight of all six signals together.
        ///
        /// Read, never assumed to be 1.0: `confidence` and both bounds divide
        /// by it, and a seventh signal added by a later wave must move all
        /// three together or they stop being comparable.
        static var totalWeight: Double {
            allCases.reduce(0.0) { $0 + $1.weight }
        }

        /// The declared full-scale reading for this signal, in the signal's own
        /// units. `nil` means the raw reading already lies in 0...1 and needs
        /// no normalisation at all.
        ///
        /// Fixed, not taken from the scanned set. Normalising against the set's
        /// own maximum makes every score a statement about the other modules
        /// scanned alongside it: one added `func test` line moves unrelated
        /// modules' scores, a failed git probe on one module rewrites two
        /// others', and a score stored on a task last month cannot be compared
        /// with one computed today. A declared scale makes the reading a
        /// property of the module - which is also what lets the ordering key's
        /// declared priors mean anything, since a prior is a claim about a
        /// reading and a set-relative reading is a claim about the set. Values
        /// are round numbers chosen to sit at "clearly bad", and they are
        /// guesses about severity, not measurements - which is why they are
        /// written down here where they can be argued with.
        var scale: Double? {
            switch self {
            case .todoDensity: return 20        // markers per kLOC
            case .churn: return 20              // commits touching it in 30d
            case .sizeRisk: return 5000         // lines in one module
            case .openIssues: return 10         // snapshot mentions
            // Already absolute. `testGap` is the one signal that answers "is
            // this bad in itself"; dividing it by a set maximum throws away the
            // only calibration the instrument has.
            case .testGap, .declaredIncomplete: return nil
            }
        }

        /// The normalised reading this signal is ASSUMED to have taken when its
        /// probe failed. Used by the queue's ordering key, and by nothing else.
        ///
        /// PLACEHOLDER VALUES. Every number below is a guess about severity,
        /// exactly like `scale` above, and written down here for the same
        /// reason: so it can be argued with instead of being buried in an
        /// estimator. Each one is meant to be the MEDIAN normalised reading of
        /// that signal over a named scan corpus, computed once, offline, and
        /// frozen here as a literal with its date and commit. No such corpus
        /// has been recorded yet, so these are the author's guesses and must be
        /// replaced by measured medians before anyone calls them calibrated.
        ///
        /// It must never be recomputed per scan. A prior taken from the
        /// scanned set makes a module's rank a function of which other modules
        /// happened to be scanned beside it, which is the exact coupling the
        /// fixed `scale` was introduced to remove.
        ///
        /// It must never be zero. Zero here is `zeroImputedScore` wearing the
        /// new key's name, and reinstates the one rule the instrument exists to
        /// enforce: a probe that failed is not a reading of health.
        var priorWhenUnread: Double {
            switch self {
            // Most modules in this tree are under-tested, so an unread test
            // count is far more likely to be bad news than good.
            case .testGap: return 0.60
            case .churn: return 0.15
            case .todoDensity: return 0.10
            case .sizeRisk: return 0.10
            case .declaredIncomplete: return 0.10
            case .openIssues: return 0.10
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
    /// Sum of `normalized * weight` over the measured signals, before any
    /// division. Carried so the bounds are one division rather than a division
    /// followed by a multiplication - see `zeroImputedScore`.
    let weightedTotal: Double

    var id: String { module }

    var measuredCount: Int { signals.filter { $0.normalized.isMeasured }.count }

    /// The weighted mean computed by substituting ZERO for every signal that
    /// was not measured. REPORTED, never ordered on - it is the lower end of
    /// this target's interval and is exposed as `lowerBound` under that name.
    ///
    /// Named for what it is. Reading it as "at least this bad" is arithmetically
    /// the opposite of what it does: `weighted / totalWeight` is a lower bound
    /// only if you assume every unread signal would have read zero, and refusing
    /// that assumption is the instrument's first documented rule. See
    /// `priorImputedScore` for what the queue is actually ordered on, and why.
    var zeroImputedScore: Double {
        let totalWeight = HiveSignal.Kind.totalWeight
        return totalWeight > 0 ? weightedTotal / totalWeight : 0
    }

    // MARK: - The key the queue is ordered on

    /// THE ORDERING KEY: the weakness actually measured, plus a declared
    /// typical reading for every probe that failed.
    ///
    ///     key = SUM over measured   (normalised * weight)
    ///         + SUM over unmeasured (priorWhenUnread * weight)
    ///         all over the total weight
    ///
    /// WHY THIS AND NOT THE OTHER TWO, so the next reader does not have to
    /// re-litigate it:
    ///
    /// The queue is a top-k selection under a budget, and the cost of getting
    /// it wrong is the regret of spending a bee on module i instead of on the
    /// true worst one: V_worst - V_i. That loss is LINEAR in the ranked
    /// quantity, and for a linear loss the selection that minimises it is
    /// argmax of the POSTERIOR MEAN. So the key has to be a mean. Both keys
    /// this replaced are bounds or ratios, which is why both were wrong, and
    /// wrong in opposite directions.
    ///
    ///   - `zeroImputedScore` is this same expression with every prior set to
    ///     zero. It is the maximin answer - what you get by refusing to say
    ///     anything at all about the unread signals - and its bias against a
    ///     module whose probe failed is exactly -p_true * weight, always
    ///     negative. A failed probe always reads as health, which is a direct
    ///     violation of the instrument's first documented rule.
    ///
    ///   - `score`, which this copy used to order on, is the same expression
    ///     with each prior set to the module's OWN measured mean: lower +
    ///     score * (1 - confidence) = score, algebraically. It is
    ///     self-imputation, its bias is correlated with the module's own
    ///     measured weakness, and the amplification is 1/confidence - between
    ///     2.5x and 4.2x over the two-signal subsets of these six. Ordering on
    ///     it hands the top of the queue to whichever module the Queen knows
    ///     least about.
    ///
    ///   - A declared constant is the only one of the three whose bias can be
    ///     driven to zero, because "is 0.15 the right prior for churn" is a
    ///     checkable claim about a written-down number rather than a structural
    ///     property of the estimator. The condition it is chosen to satisfy is
    ///     NEUTRALITY: the ranking must not move when a probe fails and nothing
    ///     about the module has changed, or the queue is ordered by how well
    ///     the scanner worked rather than by the state of the code.
    ///
    /// Interval dominance was considered as the comparator and rejected: it is
    /// the right STATEMENT of what the evidence settles, but overlap is
    /// intransitive, so it is not a strict weak ordering and `sorted(by:)` may
    /// not be handed it. It is reported instead - see `HiveSeparation`.
    ///
    /// One sum then one division, in `allCases` order. The association is
    /// fixed so that a module measured at exactly the declared prior and a
    /// module whose probe failed produce the same key BIT FOR BIT, and the
    /// declared tie-breaks actually get to run.
    var priorImputedScore: Double {
        let totalWeight = HiveSignal.Kind.totalWeight
        guard totalWeight > 0 else { return 0 }
        var sum = 0.0
        for kind in HiveSignal.Kind.allCases {
            let reading = signals.first { $0.kind == kind }?.normalized
            if let normalized = reading?.value {
                sum += normalized * kind.weight
            } else {
                // Includes a signal the engine never produced at all: absent is
                // not zero here either.
                sum += kind.priorWhenUnread * kind.weight
            }
        }
        return sum / totalWeight
    }

    /// Weight of the signals whose probes failed, as a share of the total.
    var unmeasuredShare: Double {
        let totalWeight = HiveSignal.Kind.totalWeight
        guard totalWeight > 0 else { return 0 }
        var sum = 0.0
        for kind in HiveSignal.Kind.allCases {
            let reading = signals.first { $0.kind == kind }?.normalized
            if reading?.value == nil { sum += kind.weight }
        }
        return sum / totalWeight
    }

    /// The honest lower end of the interval: what this module would score if
    /// every unread signal turned out to be perfectly healthy. It is
    /// `zeroImputedScore` under the name that describes it.
    var lowerBound: Double { zeroImputedScore }

    /// The honest upper end: every unread signal at full scale.
    var upperBound: Double { lowerBound + unmeasuredShare }

    /// True when too little of this module was read for its rank to mean
    /// anything. Such a target leaves the ranked queue entirely: the remedy is
    /// to repair the probe, not to send a bee at the module.
    var isInstrumentFault: Bool {
        confidence < HiveInvariants.minimumDispatchConfidence
    }

    /// The probes that failed, in declared order, each with its own reason.
    var unreadProbes: [(kind: HiveSignal.Kind, why: String)] {
        HiveSignal.Kind.allCases.compactMap { kind in
            guard let signal = signals.first(where: { $0.kind == kind }) else {
                return (kind, "signal not produced by the scan")
            }
            guard case .unmeasured(let why) = signal.normalized else { return nil }
            return (kind, why)
        }
    }

    /// One line naming every probe that failed and why - the operator's
    /// instruction sheet when a target is an instrument fault rather than a
    /// weak module.
    var unreadProbeDetail: String {
        let parts = unreadProbes.map { "\($0.kind.label): \($0.why)" }
        guard !parts.isEmpty else { return "every signal was read" }
        return parts.joined(separator: "; ")
    }

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
    /// The commit the verdict was measured against. Without it, a verdict
    /// recorded days ago is indistinguishable from one recorded just now.
    var verifiedAtCommit: String?

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
        verifiedAtCommit = try c.decodeIfPresent(String.self, forKey: .verifiedAtCommit)
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

// MARK: - Reservations

/// What a bee was charged at dispatch, and everything needed to settle that
/// charge after the process that made it is gone.
///
/// Held in the state file rather than only in memory. A reservation that lives
/// in a variable is cancelled by the very event it exists to survive: three
/// rebuild-or-crash restarts in a morning, two bees in flight each time, and
/// the ledger keeps six full per-bee budgets against about two dollars of real
/// work - with nothing on screen saying the money was never spent.
struct HiveReservation: Codable, Equatable {
    var taskID: String
    var sessionID: String
    /// The local-day key the charge landed on, so a settlement taken after
    /// midnight cannot credit the wrong day.
    var day: String
    var amount: Double
    /// The bee's process id, so a reservation found at load can be told apart
    /// from one whose bee is still running. Never signalled - see
    /// `HiveProcessLiveness`.
    var pid: Int32?
    var startedAt: Date

    init(taskID: String, sessionID: String, day: String, amount: Double, pid: Int32?, startedAt: Date = Date()) {
        self.taskID = taskID
        self.sessionID = sessionID
        self.day = day
        self.amount = amount
        self.pid = pid
        self.startedAt = startedAt
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
    /// When each bee was spawned, so the hourly rate bound survives a restart.
    ///
    /// The window used to be in memory only, on the argument that a restart is
    /// a human-initiated event that should not inherit an old window's debt.
    /// That argument fails the moment anything relaunches the app on its own:
    /// a watchdog that restores the process within 60s turns the rate limit
    /// into a limit per launch, and the limiter's own bound - "at most
    /// maxBeesPerHour spawns in any rolling hour" - stops being true of the
    /// machine. Pruned to the last hour on load, so a window recorded three
    /// days ago cannot be replayed as this hour's spend.
    var spawnWindow: [Date]
    /// Live reservations, keyed by task id. Written before the bee is launched
    /// so a crash between the charge and the launch cannot lose the charge.
    var reservations: [String: HiveReservation]

    init(
        policy: HivePolicy = .default,
        tasks: [HiveTask] = [],
        spendByDay: [String: Double] = [:],
        spawnWindow: [Date] = [],
        reservations: [String: HiveReservation] = [:]
    ) {
        self.policy = policy
        self.tasks = tasks
        self.updatedAt = Date()
        self.spendByDay = spendByDay
        self.spawnWindow = spawnWindow
        self.reservations = reservations
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        policy = try c.decodeIfPresent(HivePolicy.self, forKey: .policy) ?? .default
        tasks = try c.decodeIfPresent([HiveTask].self, forKey: .tasks) ?? []
        updatedAt = try c.decodeIfPresent(Date.self, forKey: .updatedAt) ?? Date()
        spendByDay = try c.decodeIfPresent([String: Double].self, forKey: .spendByDay) ?? [:]
        // Optional, so a state file written before the window was persisted
        // keeps loading instead of throwing and resetting every setting.
        spawnWindow = try c.decodeIfPresent([Date].self, forKey: .spawnWindow) ?? []
        reservations = try c.decodeIfPresent([String: HiveReservation].self, forKey: .reservations) ?? [:]
    }

    /// The rolling window, cut to the last hour.
    ///
    /// Applied on load. Without it a persisted window is worse than none: a
    /// file written days ago would hand the limiter six spawns that never
    /// happened this hour, and the loop would refuse to work for an hour after
    /// every launch.
    static func prunedSpawnWindow(_ window: [Date], now: Date = Date()) -> [Date] {
        let cutoff = now.addingTimeInterval(-3600)
        // Dates in the future are dropped too: a clock that moved backwards
        // would otherwise leave entries that never expire.
        return window.filter { $0 >= cutoff && $0 <= now }.sorted()
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

    /// Returns part of a reservation to the day it was charged against.
    ///
    /// The ledger charges the full per-bee budget the moment a bee is sent out,
    /// because a bee that hangs, is killed on the wall clock, or dies without a
    /// result line still costs real money at the provider and reports nothing.
    /// Waiting for a `total_cost_usd` line to debit anything means an execution
    /// where every bee times out spends without limit while the ceiling reads
    /// zero. Reconciliation runs one way only: down, and never below zero.
    mutating func refund(_ amount: Double, forDay key: String) {
        guard amount > 0, let current = spendByDay[key] else { return }
        spendByDay[key] = max(0, current - amount)
    }
}
