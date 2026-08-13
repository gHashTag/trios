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
    /// The last scan, split into what the evidence could rank and what it
    /// could not. The queue - not a flat list - because those two halves have
    /// different remedies and printing them as one list states a comparison
    /// the scan never made.
    @Published private(set) var queue: HiveQueue = .empty
    /// Every target from the last scan, ranked ones first. Kept for counting.
    var targets: [HiveTarget] { queue.allTargets }
    @Published private(set) var bees: [HiveLiveBee] = []
    @Published private(set) var events: [HiveEvent] = []
    @Published private(set) var status: HiveDispatchDecision = .idle("not started")
    @Published private(set) var auth: HiveAuthState?
    @Published private(set) var isScanning = false
    @Published private(set) var lastScanAt: Date?
    @Published private(set) var nextCycleAt: Date?
    @Published private(set) var spentToday: Double = 0
    @Published private(set) var consecutiveFailures = 0
    /// Whether a cycle is actually scheduled, and whether the operator has to
    /// do something about it. Derived from the timer, never from the persisted
    /// policy flag - see `HiveLoopStatus`.
    @Published private(set) var loopStatus: HiveLoopStatus = .idle
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
    /// What each live bee was charged at dispatch, and the day it was charged
    /// to. Persisted, so a restart cannot lose the charge or the way back out
    /// of it.
    private var reservations: [String: HiveReservation] = [:]
    private var timer: Timer?
    private var cycleLatch = HiveCycleLatch()
    /// The fault list as it stood when it was last recorded, so an unchanged
    /// instrument does not write an audit line every cycle.
    private var recordedFaults: [String] = []

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
        // The hourly window is restored, not reset. `store.load()` has already
        // cut it to the last hour, so what is inherited is this hour's real
        // spawns and nothing older.
        self.rateLimiter = HiveRateLimiter(spawnTimes: state.spawnWindow)
        self.reservations = state.reservations
        repairOrphanedReservations()
        syncLoopStatus()
        announceLoopStateOnLoad()
    }

    // MARK: - Load-time repair

    /// Settles every reservation left behind by the process that made it.
    ///
    /// No bee survives a restart, so on arrival each of these describes a bee
    /// the runtime can no longer ask. Rather than guess, it reads the bee's own
    /// transcript: a cost line is charged exactly, an empty transcript is
    /// refunded whole, and real output with no cost line keeps its reservation
    /// because what it spent is unknown. A bee whose pid is still alive is an
    /// orphan that outlived the app - it keeps its reservation, because it may
    /// be spending money right now, and it is named in the audit log rather
    /// than silently signalled.
    private func repairOrphanedReservations() {
        guard !reservations.isEmpty else { return }
        var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
        for (taskID, reservation) in reservations {
            if let pid = reservation.pid, HiveProcessLiveness.isAlive(pid) {
                record(
                    "bee_orphaned",
                    "\(taskID): pid \(pid) from session \(reservation.sessionID) is still running after a "
                        + "restart. Its $\(String(format: "%.2f", reservation.amount)) reservation stands, "
                        + "and its worktree is never reused."
                )
                continue
            }
            let transcript = store.transcriptsDirectory
                .appendingPathComponent("\(reservation.sessionID).jsonl")
            switch HiveDispatch.repair(HiveTranscript.summarise(at: transcript)) {
            case .charge(let cost):
                if cost < reservation.amount {
                    state.refund(reservation.amount - cost, forDay: reservation.day)
                } else if cost > reservation.amount {
                    state.record(spend: cost - reservation.amount, on: reservation.startedAt)
                }
                record(
                    "reservation_settled",
                    String(
                        format: "%@: interrupted bee reported $%.2f against a $%.2f reservation",
                        taskID, cost, reservation.amount
                    )
                )
            case .refundInFull:
                state.refund(reservation.amount, forDay: reservation.day)
                record(
                    "reservation_refunded",
                    String(
                        format: "%@: $%.2f returned - session %@ wrote nothing, so it never spent anything",
                        taskID, reservation.amount, reservation.sessionID
                    )
                )
            case .keep:
                record(
                    "reservation_kept",
                    String(
                        format: "%@: $%.2f stands - session %@ produced output and never reported a cost, "
                            + "and an absent cost is not a zero",
                        taskID, reservation.amount, reservation.sessionID
                    )
                )
            }
            reservations[taskID] = nil
        }
        spendByDay = state.spendByDay
        spentToday = state.spent()
        persist()
    }

    /// Says out loud, once, that a loop the file calls armed is not running.
    ///
    /// Launch does not resume the cycle: the timer is created when an operator
    /// arms the loop, and nothing recreates it on load. That is a deliberate
    /// choice, but it used to be an invisible one - the badge read ARMED off
    /// the policy flag, so a queue with work in it could sit untouched for days
    /// looking exactly like a healthy loop.
    private func announceLoopStateOnLoad() {
        guard policy.enabled else { return }
        status = .idle("armed in the state file, not ticking - press Run 24/7 to start the clock")
        record(
            "hive_resume_required",
            "loaded with enabled=true and no cycle scheduled: \(tasks.filter(\.isSchedulable).count) "
                + "schedulable task(s) are waiting and nothing will dispatch until the loop is re-armed"
        )
    }

    /// Recomputes the published loop status from the two things that decide it.
    private func syncLoopStatus() {
        loopStatus = HiveLoopStatus.of(enabled: policy.enabled, ticking: timer != nil)
    }

    // MARK: - Derived

    /// Counted from live runners, not from task state, so a crashed bee cannot
    /// hold a slot forever.
    var liveBeeCount: Int { bees.filter { !$0.status.isTerminal }.count }

    /// The ranked rows minus the ones the operator has ruled out.
    ///
    /// Positions are the queue's own, so a gap means something above was ruled
    /// out rather than that the numbering is broken.
    var eligibleRows: [HiveRankedTarget] {
        queue.ranked.filter { policy.skippedModules[$0.target.module] == nil }
    }

    /// Ranked targets minus the ones the operator has ruled out. Instrument
    /// faults are NOT in here: they were never ranked.
    var eligibleTargets: [HiveTarget] {
        eligibleRows.map(\.target)
    }

    /// Targets the scan read too little of to rank at all. Their remedy is to
    /// repair the probe, so they are surfaced rather than silently dropped -
    /// and they carry no position and no score, because the scan never
    /// compared them with anything.
    var instrumentFaults: [HiveTarget] {
        queue.instrumentFaults.filter { policy.skippedModules[$0.module] == nil }
    }

    /// Bees started in the last rolling hour, counted across restarts because
    /// the window is persisted.
    var spawnsThisHour: Int { rateLimiter.spawnsInLastHour() }

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

    /// What the sibling has already committed today and this copy therefore
    /// charges against its own ceiling. Zero until the sibling has been probed:
    /// an unprobed sibling is not a debit, it is an unknown.
    var siblingCommittedUSD: Double { sibling?.committedSpendUSD ?? 0 }

    /// What is left of this copy's daily ceiling once both its own spend and
    /// the sibling's committed spend are taken off it. This, not
    /// `dailyBudgetUSD - spentToday`, is what the next dispatch is measured
    /// against.
    var sharedHeadroomUSD: Double {
        max(0, policy.dailyBudgetUSD - spentToday - siblingCommittedUSD)
    }

    var invariantViolations: [HiveInvariantViolation] {
        HiveInvariants.check(policy: policy, tasks: tasks, spentToday: spentToday)
    }

    /// Everything the pure decision needs, assembled from the impure world.
    ///
    /// Internal rather than private: the wiring is where a proof about
    /// `HiveDispatch` stops being a proof about the loop, so a test is allowed
    /// to look at exactly what this copy hands the decision.
    var dispatchContext: HiveDispatchContext {
        HiveDispatchContext(
            policy: policy,
            tasks: tasks,
            liveBees: liveBeeCount,
            spentToday: spentToday,
            spawnsInLastHour: rateLimiter.spawnsInLastHour(),
            consecutiveFailures: consecutiveFailures,
            auth: auth,
            siblingCommittedUSD: siblingCommittedUSD
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
        // The clock starts here rather than at the end of the first cycle, so
        // arming is visible immediately and does not depend on that cycle
        // reaching its tail.
        scheduleTimer()
        Task { await runCycle(trigger: "arm") }
    }

    func disarm() {
        var updated = policy
        updated.enabled = false
        apply(updated)
        timer?.invalidate()
        timer = nil
        nextCycleAt = nil
        syncLoopStatus()
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
        // Unconditionally, because the clock repeats now: a policy that turns
        // the loop off must take the timer with it, or a disarmed loop keeps
        // waking every interval to rescan the whole repository and return.
        scheduleTimer()
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
        queue = HivePriorityEngine.queue(facts)
        lastScanAt = Date()
        isScanning = false

        // One audit line when the instrument's blind spots change, and only
        // then. A module the scanner keeps failing to read is a fault to fix,
        // not a target to rank, and it would otherwise vanish between cycles.
        let faults = queue.instrumentFaults.map(\.module).sorted()
        if faults != recordedFaults {
            recordedFaults = faults
            if !faults.isEmpty {
                let detail = queue.instrumentFaults
                    .prefix(3)
                    .map { "\($0.module) (\(Int($0.confidence * 100))% read: \($0.unreadProbeDetail))" }
                    .joined(separator: "; ")
                record(
                    "instrument_fault",
                    "\(faults.count) module(s) left the ranked queue - too little was measured "
                        + "to rank them, so the remedy is the probe, not a bee: \(detail)"
                )
            }
        }
    }

    /// Re-reads the other Hive's state file, once per cycle and before the
    /// dispatch decision that consumes it.
    ///
    /// The reading is not a veto: no state of the sibling switches this loop
    /// off. What it can do is spend this copy's ceiling - the sibling's
    /// committed dollars are charged here, so the two loops share one daily
    /// ceiling rather than getting one each. Everything that could not be
    /// established charges nothing, which keeps a file whose process died from
    /// blocking a loop that is running.
    ///
    /// One audit line when the answer changes - only when it changes, or an
    /// armed sibling would fill the log with the same sentence every cycle.
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
        // The latch has a deadline. Without one it is a lock nothing can
        // release once the cycle holding it wedges on a subprocess that ignores
        // SIGTERM: `Cycle now` returns silently, Pause-then-Run re-arms a timer
        // that returns at the same guard, and only killing the app recovers.
        let admission = cycleLatch.enter(
            deadline: HiveCycleLatch.wedgeDeadline(cycleIntervalSeconds: policy.cycleIntervalSeconds)
        )
        switch admission {
        case .busy:
            return
        case .forced(_, let heldFor):
            record(
                "cycle_wedged",
                "a cycle held the loop for \(Int(heldFor))s without finishing; it was forced open so the "
                    + "loop could carry on. Its subprocess may still be running."
            )
        case .admitted:
            break
        }
        guard let token = admission.token else { return }
        // Every exit path leaves the latch and re-arms the clock. The loop's
        // ability to run again must not depend on this particular cycle
        // reaching its own last line.
        defer {
            cycleLatch.leave(token: token)
            if policy.enabled && timer == nil { scheduleTimer() }
            nextCycleAt = timer?.fireDate
            syncLoopStatus()
        }

        harvest()
        let verifier = self.verifier
        currentHead = await Task.detached(priority: .utility) {
            verifier.head(at: verifier.projectRoot)
        }.value
        await rescan()
        await refreshSibling()
        materialiseTasks()
        if auth == nil || policy.enabled { await preflight() }

        let decision = HiveDispatch.decide(dispatchContext)
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
        // The clock is not re-armed here. It repeats on its own, and the defer
        // above restores it on every exit path - including the ones that never
        // reach this line.
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

    /// Task ids with a bee still alive. The admission gate's first input.
    private var liveTaskIDs: Set<String> {
        Set(bees.filter { !$0.status.isTerminal }.map(\.taskID))
    }

    private func dispatch(_ task: HiveTask) {
        // One admission gate, whoever asked. Two `claude -p` sessions on one
        // task would edit the same worktree, render two rows with the same
        // identity, and leave a reservation no completion will ever settle.
        if let refusal = HiveDispatch.admissionRefusal(
            taskID: task.id,
            liveTaskIDs: liveTaskIDs,
            reservedTaskIDs: Set(reservations.keys)
        ) {
            record("spawn_refused", "\(task.id): \(refusal)")
            status = .idle(refusal)
            return
        }
        guard let executable = HiveProcess.resolve("claude", overrideEnvKey: "CLAUDE_EXECUTABLE"),
              let index = tasks.firstIndex(where: { $0.id == task.id }) else { return }

        let sessionID = UUID().uuidString
        // The session is part of the directory name, so two attempts on one
        // task can never collide on a worktree - not even across process
        // lifetimes, where a bee that outlived the app is still writing to the
        // directory a fresh bee would otherwise be dispatched into.
        let worktree = policy.useWorktree ? "hive-\(task.id)-\(sessionID.prefix(8).lowercased())" : nil

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

        // Charge the full per-bee budget now, and reconcile downward when the
        // bee reports what it actually cost. Debiting only what a bee reports
        // means a bee that hangs, is killed on the wall clock, or dies without
        // a result line costs the ledger nothing while costing real money at
        // the provider: two such bees an hour spend ten times the daily ceiling
        // while the dashboard reads zero. Absent is not zero here either.
        reserve(policy.maxBudgetUSDPerBee, for: task.id, sessionID: sessionID)

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

        // The pid is written beside the reservation, so a reservation found in
        // the state file after a restart can be told apart from one whose bee
        // is still running and still spending.
        if let pid = runner.processIdentifier {
            reservations[task.id]?.pid = pid
            persist()
        }
    }

    // MARK: - Spend

    /// Charges the full per-bee budget the moment a bee goes out.
    private func reserve(_ amount: Double, for taskID: String, sessionID: String) {
        guard amount > 0 else { return }
        let day = HiveState.dayKey()
        var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
        state.record(spend: amount)
        spendByDay = state.spendByDay
        spentToday = state.spent()
        reservations[taskID] = HiveReservation(
            taskID: taskID,
            sessionID: sessionID,
            day: day,
            amount: amount,
            pid: nil
        )
    }

    /// Settles a reservation against what the bee actually cost.
    ///
    /// `costUSD == nil` does not mean free. It means the bee hung, was killed
    /// on the wall clock, was cancelled by an operator, or died before printing
    /// a result line - and the provider bills for all of those. But it is also
    /// not one absence: the bee's transcript is on disk, so the three cases can
    /// be told apart instead of being collapsed into "keep the whole
    /// reservation". Six bees stopped after three seconds would otherwise
    /// charge $30 against a $25 ceiling and block every dispatch until
    /// midnight, with nothing in the log saying the money was never spent.
    private func reconcile(taskID: String, reported: Double?, sessionID: String?) {
        guard let reservation = reservations.removeValue(forKey: taskID) else {
            guard let reported, reported > 0 else { return }
            var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
            state.record(spend: reported)
            spendByDay = state.spendByDay
            spentToday = state.spent()
            return
        }

        var settled = reported
        if settled == nil {
            let transcript = store.transcriptsDirectory
                .appendingPathComponent("\(sessionID ?? reservation.sessionID).jsonl")
            switch HiveDispatch.repair(HiveTranscript.summarise(at: transcript)) {
            case .charge(let cost):
                settled = cost
            case .refundInFull:
                settled = 0
                record(
                    "reservation_refunded",
                    String(
                        format: "%@: $%.2f returned - the bee wrote nothing, so it never spent anything",
                        taskID, reservation.amount
                    )
                )
            case .keep:
                // Real output, no cost line. Unknown is not zero.
                return
            }
        }
        guard let settled else { return }

        var state = HiveState(policy: policy, tasks: tasks, spendByDay: spendByDay)
        if settled < reservation.amount {
            state.refund(reservation.amount - settled, forDay: reservation.day)
        } else if settled > reservation.amount {
            state.record(spend: settled - reservation.amount)
        }
        spendByDay = state.spendByDay
        spentToday = state.spent()
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
        reconcile(taskID: taskID, reported: outcome.costUSD, sessionID: outcome.sessionID)
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

        // The cost is not recorded here. It was charged in full at dispatch and
        // reconciled downward above: charging only on completion let a bee
        // killed by the wall-clock guard, which prints no result line, cost the
        // ledger nothing while costing real money at the provider.

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
        syncLoopStatus()
        persist()
    }

    /// Arms the clock.
    ///
    /// Repeating, not one-shot. A one-shot timer made the next cycle depend on
    /// this cycle reaching its own tail, so any early return killed the loop
    /// permanently: one `claude auth status` that exceeded its 20s timeout
    /// because the machine was busy returned `.unknown`, the cycle returned
    /// before re-arming, and the Hive sat at ARMED and "next cycle 0s" for ever
    /// while the load that caused it subsided a minute later. Overlapping ticks
    /// are dropped by the cycle latch, which is the right place for that
    /// decision.
    private func scheduleTimer() {
        timer?.invalidate()
        timer = nil
        guard policy.enabled else {
            nextCycleAt = nil
            syncLoopStatus()
            return
        }
        let interval = TimeInterval(policy.cycleIntervalSeconds)
        let timer = Timer(timeInterval: interval, repeats: true) { [weak self] _ in
            Task { @MainActor in await self?.runCycle(trigger: "timer") }
        }
        // .common keeps the loop ticking while a menu or sheet is open.
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
        // Read off the timer rather than recomputed, so the countdown on
        // screen is the moment the clock will actually fire.
        nextCycleAt = timer.fireDate
        syncLoopStatus()
    }

    private func persist() {
        var state = HiveState(
            policy: policy,
            tasks: tasks,
            spendByDay: spendByDay,
            // The rate window and the live reservations travel with the ledger.
            // Both used to live only in this object, so both were erased by the
            // one event they exist to survive.
            spawnWindow: rateLimiter.spawnTimes,
            reservations: reservations
        )
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
