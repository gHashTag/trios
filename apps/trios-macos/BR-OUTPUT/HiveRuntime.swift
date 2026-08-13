import Combine
import Foundation

// ===========================================================================
// HIVE RUNTIME - the timer, the child processes, and the persisted state.
//
// Deliberately thin. Every decision it makes is delegated to `HiveDispatch`,
// which is pure and tested; what lives here is only the machinery that cannot
// be pure - a repeating timer, running processes, and disk.
//
// Combine rather than SwiftUI, so this file compiles and runs in CI.
// ===========================================================================

/// A bee the runtime is currently supervising.
struct HiveLiveBee: Identifiable, Equatable {
    let taskID: String
    let title: String
    let module: String
    let sessionID: String
    let branch: String?
    let startedAt: Date
    var status: HiveBeeStatus
    var lastLine: String
    var toolCalls: Int

    var id: String { taskID }

    /// The command that reopens this bee's conversation.
    var resumeCommand: String { "claude --resume \(sessionID)" }
}

@MainActor
final class HiveRuntime: ObservableObject {

    static let shared = HiveRuntime()

    // MARK: - Published state

    @Published private(set) var policy: HivePolicy
    @Published private(set) var tasks: [HiveTask] = []
    @Published private(set) var targets: [HiveTarget] = []
    @Published private(set) var bees: [HiveLiveBee] = []
    @Published private(set) var events: [HiveEvent] = []
    @Published private(set) var status: HiveDispatchDecision = .idle("not started")
    @Published private(set) var auth: HiveAuthState?
    @Published private(set) var isScanning = false
    @Published private(set) var lastScanAt: Date?
    @Published private(set) var nextCycleAt: Date?
    @Published private(set) var spentToday: Double = 0
    @Published private(set) var consecutiveFailures = 0
    /// The other Hive's state, re-read once per cycle. `nil` until the first
    /// cycle has run, which is not the same as "no sibling".
    @Published private(set) var sibling: HiveSiblingReport?

    // MARK: - Internals

    private let store: HiveStore
    private let scanner: HiveRepoScanner
    private let verifier: HiveVerifier
    private let siblingProbe: HiveSiblingProbe
    private var spendByDay: [String: Double] = [:]
    /// Refreshed once per cycle rather than per row, so a review list of
    /// twenty tasks does not run twenty `git rev-parse` calls.
    private var currentHead: String?
    private var rateLimiter = HiveRateLimiter()
    private var runners: [String: HiveBeeRunner] = [:]
    private var timer: Timer?
    private var cycleInFlight = false

    init(
        store: HiveStore = HiveStore(),
        scanner: HiveRepoScanner = HiveRepoScanner(),
        verifier: HiveVerifier = HiveVerifier(),
        siblingProbe: HiveSiblingProbe = HiveSiblingProbe()
    ) {
        self.store = store
        self.scanner = scanner
        self.verifier = verifier
        self.siblingProbe = siblingProbe
        let state = store.load()
        self.policy = state.policy
        self.tasks = state.tasks
        self.spendByDay = state.spendByDay
        self.spentToday = state.spent()
        self.events = store.recentEvents(limit: 60)
    }

    // MARK: - Derived

    /// Counted from live runners, not from task state, so a crashed bee cannot
    /// hold a slot forever.
    var liveBeeCount: Int { bees.filter { !$0.status.isTerminal }.count }

    var eligibleTargets: [HiveTarget] {
        targets.filter { policy.skippedModules[$0.module] == nil }
    }

    var reviewCount: Int { tasks.filter { $0.state == .review }.count }
    var doneCount: Int { tasks.filter { $0.state == .done }.count }
    var toxicCount: Int { tasks.filter { $0.state == .toxic }.count }

    /// Whether a task's recorded verdict still describes the current tree.
    func evidenceState(for task: HiveTask) -> HiveEvidenceState {
        guard task.verification != nil else { return .unrecorded }
        return HiveVerifier.evidenceState(
            verifiedAt: task.verifiedAtCommit,
            currentHead: currentHead
        )
    }

    /// What both copies together may spend in one local day. Equal to this
    /// copy's ceiling until the sibling is found armed, at which point the
    /// number the operator entered in this window stops being the total.
    var combinedExposureUSD: Double {
        sibling?.combinedExposure(ownCeiling: policy.dailyBudgetUSD) ?? policy.dailyBudgetUSD
    }

    /// True only while a second armed loop is known to exist. An unreadable
    /// sibling is not counted here - see `siblingExposureIsBounded`.
    var siblingIsArmed: Bool { sibling?.state.isArmed ?? false }

    /// False when the sibling could not be read, so `combinedExposureUSD` is a
    /// lower bound rather than a ceiling.
    var siblingExposureIsBounded: Bool { sibling?.exposureIsBounded ?? true }

    var invariantViolations: [HiveInvariantViolation] {
        HiveInvariants.check(policy: policy, tasks: tasks, spentToday: spentToday)
    }

    private var context: HiveDispatchContext {
        HiveDispatchContext(
            policy: policy,
            tasks: tasks,
            liveBees: liveBeeCount,
            spentToday: spentToday,
            spawnsInLastHour: rateLimiter.spawnsInLastHour(),
            consecutiveFailures: consecutiveFailures,
            auth: auth
        )
    }

    // MARK: - Control

    func arm() {
        var updated = policy
        updated.enabled = true
        // Arming is the operator's statement that the cause was addressed, so
        // it is also the only thing that clears the breaker.
        consecutiveFailures = 0
        apply(updated)
        record("hive_armed", "interval \(policy.cycleIntervalSeconds)s, max \(policy.maxConcurrentBees) bees")
        Task { await runCycle(trigger: "arm") }
    }

    func disarm() {
        var updated = policy
        updated.enabled = false
        apply(updated)
        timer?.invalidate()
        timer = nil
        nextCycleAt = nil
        status = .idle("the loop is not armed")
        record("hive_disarmed", "paused by operator; running bees were left to finish")
    }

    func stopAllBees() {
        disarm()
        for (taskID, runner) in runners {
            runner.terminate()
            record("bee_cancelled", "operator stopped \(taskID)")
        }
    }

    func updatePolicy(_ new: HivePolicy) {
        apply(new)
        if policy.enabled { scheduleTimer() }
    }

    func skip(module: String, why: String) {
        var updated = policy
        updated.skippedModules[module] = why
        apply(updated)
        for index in tasks.indices
        where tasks[index].module == module && tasks[index].isSchedulable {
            tasks[index].state = .toxic
            tasks[index].lastError = "module skipped by operator: \(why)"
            tasks[index].updatedAt = Date()
        }
        persist()
        record("module_skipped", "\(module): \(why)")
    }

    func unskip(module: String) {
        var updated = policy
        updated.skippedModules[module] = nil
        apply(updated)
        record("module_unskipped", module)
    }

    func accept(_ taskID: String) {
        guard let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].state = .done
        tasks[index].updatedAt = Date()
        persist()
        record("task_accepted", taskID)
    }

    func reject(_ taskID: String, why: String) {
        guard let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].state = tasks[index].attempts >= policy.maxAttemptsPerTask ? .toxic : .pending
        tasks[index].lastError = why
        tasks[index].updatedAt = Date()
        persist()
        record("task_rejected", "\(taskID): \(why)")
    }

    func runCycleNow() {
        Task { await runCycle(trigger: "manual") }
    }

    // MARK: - Scan

    func rescan() async {
        guard !isScanning else { return }
        isScanning = true
        let scanner = self.scanner
        let facts = await Task.detached(priority: .utility) { scanner.scan() }.value
        targets = HivePriorityEngine.rank(facts)
        lastScanAt = Date()
        isScanning = false
    }

    /// Re-reads the other Hive's state file, once per cycle.
    ///
    /// Nothing here can gate a dispatch: the sibling enforces its own ceiling
    /// in its own process, and a check from this side would be advisory at
    /// best and a false assurance at worst. What it does is put the real total
    /// in front of the operator, and write one audit line when the answer
    /// changes - only when it changes, or an armed sibling would fill the log
    /// with the same sentence every cycle.
    func refreshSibling() async {
        let probe = siblingProbe
        let report = await Task.detached(priority: .utility) { probe.probe() }.value
        let previous = sibling?.state
        sibling = report
        guard previous != report.state else { return }
        record(
            "sibling_hive",
            "\(report.state.label): " + report.summary(
                ownCeiling: policy.dailyBudgetUSD,
                ownArmed: policy.enabled
            )
        )
    }

    func preflight() async {
        guard let executable = HiveProcess.resolve("claude", overrideEnvKey: "CLAUDE_EXECUTABLE") else {
            auth = .unknown("claude CLI not found on this machine")
            return
        }
        auth = await Task.detached(priority: .utility) {
            HiveAuthProbe.check(executable: executable)
        }.value
    }

    // MARK: - The cycle

    func runCycle(trigger: String) async {
        guard !cycleInFlight else { return }
        cycleInFlight = true
        defer { cycleInFlight = false }

        harvest()
        let verifier = self.verifier
        currentHead = await Task.detached(priority: .utility) {
            verifier.head(at: verifier.projectRoot)
        }.value
        await rescan()
        await refreshSibling()
        materialiseTasks()
        if auth == nil || policy.enabled { await preflight() }

        let decision = HiveDispatch.decide(context)
        status = decision

        switch decision {
        case .dispatch(let task):
            dispatch(task)
        case .blocked(let why):
            record("hive_blocked", why)
        case .idle:
            break
        }

        persist()
        scheduleTimer()
    }

    /// Turns the top-ranked targets into tasks. A target the factory refuses -
    /// too little of it was measured - is recorded rather than dropped, so the
    /// operator can see what the Queen declined to work on and why.
    func materialiseTasks() {
        var created = 0
        for target in eligibleTargets.prefix(policy.targetsPerCycle) {
            guard let candidate = HiveTaskFactory.makeTask(from: target, policy: policy) else {
                if let why = HiveTaskFactory.rejection(for: target) {
                    record("target_declined", "\(target.module): \(why)")
                }
                continue
            }
            if let existing = tasks.firstIndex(where: { $0.id == candidate.id }) {
                tasks[existing].score = candidate.score
                tasks[existing].confidence = candidate.confidence
                tasks[existing].reason = candidate.reason
                tasks[existing].prompt = candidate.prompt
                continue
            }
            tasks.append(candidate)
            created += 1
            record("task_created", "\(candidate.id): \(candidate.reason)")
        }
        if created > 0 { persist() }
    }

    // MARK: - Dispatch

    private func dispatch(_ task: HiveTask) {
        guard let executable = HiveProcess.resolve("claude", overrideEnvKey: "CLAUDE_EXECUTABLE"),
              let index = tasks.firstIndex(where: { $0.id == task.id }) else { return }

        let sessionID = UUID().uuidString
        let worktree = policy.useWorktree ? "hive-\(task.id)" : nil

        tasks[index].state = .running
        tasks[index].attempts += 1
        tasks[index].sessionID = sessionID
        tasks[index].branch = worktree
        tasks[index].updatedAt = Date()

        let configuration = HiveBeeRunner.Configuration(
            executable: executable,
            workingDirectory: scanner.projectRoot,
            // The original prompt, every attempt. Never the previous failure -
            // see HiveInvariants.promptIsAnchorFree.
            prompt: tasks[index].prompt,
            sessionID: sessionID,
            model: policy.model,
            permissionMode: policy.permissionMode,
            maxBudgetUSD: policy.maxBudgetUSDPerBee,
            timeoutSeconds: policy.beeTimeoutSeconds,
            worktreeName: worktree,
            displayName: "bee/\(task.id)"
        )

        let runner = HiveBeeRunner(configuration: configuration)
        runners[task.id] = runner
        rateLimiter.record()

        bees.insert(
            HiveLiveBee(
                taskID: task.id,
                title: task.title,
                module: task.module,
                sessionID: sessionID,
                branch: worktree,
                startedAt: Date(),
                status: .starting,
                lastLine: "spawning",
                toolCalls: 0
            ),
            at: 0
        )

        record(
            "bee_spawned",
            "\(task.id) session \(sessionID)\(worktree.map { ", worktree \($0)" } ?? ""), attempt \(tasks[index].attempts)"
        )
        persist()

        let transcript = store.transcriptsDirectory.appendingPathComponent("\(sessionID).jsonl")
        runner.start(
            transcriptURL: transcript,
            onEvent: { [weak self] event in
                Task { @MainActor in self?.absorb(event, for: task.id) }
            },
            onFinish: { [weak self] outcome in
                Task { @MainActor in await self?.complete(taskID: task.id, outcome: outcome) }
            }
        )
    }

    private func absorb(_ event: HiveBeeEvent, for taskID: String) {
        guard let index = bees.firstIndex(where: { $0.taskID == taskID }) else { return }
        if bees[index].status == .starting { bees[index].status = .working }
        if case .tool = event.kind { bees[index].toolCalls += 1 }
        if !event.text.isEmpty { bees[index].lastLine = String(event.text.prefix(240)) }
    }

    // MARK: - Harvest

    private func complete(taskID: String, outcome: HiveBeeOutcome) async {
        runners[taskID] = nil
        if let beeIndex = bees.firstIndex(where: { $0.taskID == taskID }) {
            bees[beeIndex].status = outcome.status
            if !outcome.summary.isEmpty {
                bees[beeIndex].lastLine = String(outcome.summary.prefix(240))
            }
        }
        guard let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }

        tasks[index].resultSummary = String(outcome.summary.prefix(2000))
        tasks[index].costUSD = outcome.costUSD
        tasks[index].durationMs = outcome.durationMs
        tasks[index].updatedAt = Date()

        if let cost = outcome.costUSD, cost > 0 {
            var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
            state.record(spend: cost)
            spendByDay = state.spendByDay
            spentToday = state.spent()
        }

        var verdict: HiveVerdict?
        if outcome.status == .succeeded && policy.verifyBeforeReview {
            let task = tasks[index]
            let verifier = self.verifier
            record("verify_started", taskID)
            verdict = await Task.detached(priority: .utility) { verifier.verify(task: task) }.value
            if let verdict {
                tasks[index].verification = "\(verdict.label): \(String(verdict.detail.prefix(1500)))"
                tasks[index].verified = verdict.isPass ? true : (verdict.isFail ? false : nil)
                // Stamp the commit the verdict was measured against, so a
                // review opened days later can tell whether the evidence still
                // describes the tree.
                tasks[index].verifiedAtCommit = verifier.head(at: verifier.projectRoot)
                record("verify_\(verdict.isPass ? "passed" : (verdict.isFail ? "failed" : "unavailable"))",
                       "\(taskID): \(String(verdict.detail.prefix(200)))")
            }
        }

        let resolution = HiveDispatch.outcomeState(
            for: tasks[index],
            verdict: verdict,
            beeSucceeded: outcome.status == .succeeded,
            policy: policy
        )
        tasks[index].state = resolution.state
        if resolution.countsAsFailure {
            tasks[index].lastError = String(outcome.summary.prefix(500))
            consecutiveFailures += 1
        } else {
            consecutiveFailures = 0
        }

        record(
            "bee_finished",
            "\(taskID) \(outcome.status.label) -> \(resolution.state.label)"
        )
        persist()

        if policy.enabled { await runCycle(trigger: "slot-free") }
    }

    /// Drops finished bees older than an hour and prunes stale transcripts.
    private func harvest() {
        let cutoff = Date().addingTimeInterval(-3600)
        bees.removeAll { $0.status.isTerminal && $0.startedAt < cutoff }
        let pruned = store.pruneTranscripts(olderThanDays: policy.retainTranscriptDays)
        if pruned > 0 { record("retention", "pruned \(pruned) transcript(s)") }
    }

    // MARK: - Plumbing

    private func apply(_ new: HivePolicy) {
        policy = new.sanitized()
        persist()
    }

    private func scheduleTimer() {
        timer?.invalidate()
        guard policy.enabled else { return }
        let interval = TimeInterval(policy.cycleIntervalSeconds)
        nextCycleAt = Date().addingTimeInterval(interval)
        let timer = Timer(timeInterval: interval, repeats: false) { [weak self] _ in
            Task { @MainActor in await self?.runCycle(trigger: "timer") }
        }
        // .common keeps the loop ticking while a menu or sheet is open.
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    private func persist() {
        var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
        state.updatedAt = Date()
        store.save(state)
    }

    private func record(_ kind: String, _ detail: String) {
        let event = HiveEvent(kind: kind, detail: detail)
        store.append(event)
        events.insert(event, at: 0)
        if events.count > 200 { events.removeLast(events.count - 200) }
    }
}
