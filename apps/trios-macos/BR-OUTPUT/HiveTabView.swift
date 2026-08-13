import SwiftUI

/// The Hive workspace: what the Queen measured, what she ranked, which bees
/// are out, and what is waiting on a human.
///
/// Colours come from `TriosTheme` and paths from `ProjectPaths` per L6 - this
/// view introduces neither a palette nor a path of its own.
struct HiveTabView: View {
    @StateObject private var hive = HiveRuntime.shared
    @State private var showPolicy = false
    @State private var expandedTarget: String?
    @State private var tick = Date()

    private let ticker = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                statusBanner
                siblingBanner
                controls
                stats
                if showPolicy { policyPanel }
                liveBees
                reviewQueue
                ranking
                auditLog
            }
            .padding(20)
        }
        .background(Color.grokBackground)
        .onReceive(ticker) { tick = $0 }
        .task {
            if hive.targets.isEmpty { await hive.rescan() }
            if hive.auth == nil { await hive.preflight() }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Image(systemName: "circle.hexagongrid.fill")
                .font(.system(size: 20, weight: .semibold))
                .foregroundColor(.grokText)
            VStack(alignment: .leading, spacing: 2) {
                Text("HIVE")
                    .font(.system(size: 16, weight: .bold))
                    .foregroundColor(.grokText)
                Text(subtitle)
                    .font(.system(size: 11))
                    .foregroundColor(.grokMuted)
            }
            Spacer()
            // Reads the clock, not the wish. `policy.enabled` is persisted and
            // survives a restart; the timer does not, so a badge derived from
            // the policy showed ARMED over a loop with a queue, no bees and no
            // clock, for as long as the operator left it.
            Text(hive.loopStatus.label)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(badgeColour)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Color.grokElevated)
                .triosBubble(radius: 9)
        }
    }

    private var badgeColour: Color {
        switch hive.loopStatus {
        case .ticking: return .green
        case .resumeRequired: return .orange
        case .idle: return .grokDim
        }
    }

    private var subtitle: String {
        var parts: [String] = []
        if let scanned = hive.lastScanAt {
            parts.append("ranked \(hive.targets.count) modules \(elapsed(since: scanned)) ago")
        }
        // The countdown is shown only when a clock exists. Derived from the
        // policy flag it kept counting down over a loop that had no timer.
        if let next = hive.nextCycleAt, hive.loopStatus.isTicking {
            parts.append("next cycle in \(elapsed(until: next))")
        }
        if let advice = hive.loopStatus.advice { parts.append(advice) }
        return parts.isEmpty ? "not scanned yet" : parts.joined(separator: " - ")
    }

    // MARK: - Status

    /// The loop's own account of what it is doing, or refusing to do.
    /// `idle` and `blocked` are drawn differently on purpose: an unarmed loop
    /// is an ordinary state, a signed-out CLI is a fault.
    @ViewBuilder
    private var statusBanner: some View {
        switch hive.status {
        case .blocked(let why):
            banner(icon: "exclamationmark.triangle.fill", tint: .orange, text: why)
        case .idle(let why):
            banner(icon: "pause.circle", tint: .grokDim, text: why)
        case .dispatch(let task):
            banner(icon: "arrow.up.forward.circle", tint: .green, text: "dispatching \(task.id)")
        }
    }

    /// The second Hive on this machine, and what it does to the number in the
    /// policy panel.
    ///
    /// The daily ceiling reads like a total and is only this copy's share, so
    /// an armed sibling is shown whether or not this copy is armed. An
    /// unreadable sibling is shown too: not knowing is a state the operator has
    /// to see, and it is the one case where the combined figure is a floor
    /// rather than a ceiling. A sibling that is absent or disarmed adds
    /// nothing to the exposure and so gets no banner - the policy panel still
    /// carries its line.
    @ViewBuilder
    private var siblingBanner: some View {
        if let report = hive.sibling {
            switch report.state {
            case .armed:
                banner(
                    icon: "exclamationmark.triangle.fill",
                    tint: .orange,
                    text: report.summary(ownCeiling: hive.policy.dailyBudgetUSD, ownArmed: hive.policy.enabled)
                )
            case .unreadable:
                banner(
                    icon: "questionmark.circle",
                    tint: .yellow,
                    text: report.summary(ownCeiling: hive.policy.dailyBudgetUSD, ownArmed: hive.policy.enabled)
                )
            case .absent, .disarmed:
                EmptyView()
            }
        }
    }

    private func banner(icon: String, tint: Color, text: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 12))
                .foregroundColor(tint)
            Text(text)
                .font(.system(size: 11))
                .foregroundColor(.grokMuted)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    // MARK: - Controls

    private var controls: some View {
        HStack(spacing: 8) {
            // Run 24/7 is offered whenever no cycle is scheduled, including the
            // armed-but-not-ticking state a restart leaves behind. Keying this
            // off the policy flag hid the only button that could restore the
            // loop precisely when it was the only button that would work.
            if hive.loopStatus.isTicking {
                pill("Pause", "pause.fill") { hive.disarm() }
                pill("Stop bees", "stop.fill") { hive.stopAllBees() }
            } else {
                pill("Run 24/7", "play.fill") { hive.arm() }
                if hive.policy.enabled {
                    pill("Disarm", "pause.fill") { hive.disarm() }
                }
            }
            pill("Cycle now", "arrow.clockwise") { hive.runCycleNow() }
            pill("Rescan", "ruler") { Task { await hive.rescan() } }
            pill("Preflight", "lock.shield") { Task { await hive.preflight() } }
            pill(showPolicy ? "Hide policy" : "Policy", "gearshape") {
                withAnimation { showPolicy.toggle() }
            }
            Spacer()
        }
    }

    private func pill(_ label: String, _ icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 4) {
                Image(systemName: icon).font(.system(size: 9))
                Text(label).font(.system(size: 11, weight: .medium))
            }
            .foregroundColor(.grokText)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(Color.grokElevated)
            .triosBubble(radius: 8)
            .overlay(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .stroke(Color.grokBorder, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }

    // MARK: - Stats

    private var stats: some View {
        HStack(spacing: 8) {
            stat("Bees", "\(hive.liveBeeCount)")
            stat("Review", "\(hive.reviewCount)")
            stat("Done", "\(hive.doneCount)")
            stat("Toxic", "\(hive.toxicCount)")
            stat("Today", String(format: "$%.2f", hive.spentToday))
            stat("Fails", "\(hive.consecutiveFailures)/\(hive.policy.maxConsecutiveFailures)")
            // Only when a second loop is actually armed. A tile that always
            // read the same as the daily ceiling would teach the operator to
            // stop looking at it.
            if hive.siblingIsArmed {
                stat("Both hives", String(format: "$%.2f", hive.combinedExposureUSD))
            }
        }
    }

    private func stat(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label.uppercased())
                .font(.system(size: 8, weight: .semibold))
                .foregroundColor(.grokDim)
            Text(value)
                .font(.system(size: 15, weight: .bold, design: .monospaced))
                .foregroundColor(.grokText)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(10)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    // MARK: - Policy

    private var policyPanel: some View {
        VStack(alignment: .leading, spacing: 6) {
            section("GUARDRAILS")
            stepper("Concurrent bees", hive.policy.maxConcurrentBees, 1...8) {
                var p = hive.policy; p.maxConcurrentBees = $0; hive.updatePolicy(p)
            }
            stepper("Cycle interval (min)", hive.policy.cycleIntervalSeconds / 60, 1...240) {
                var p = hive.policy; p.cycleIntervalSeconds = $0 * 60; hive.updatePolicy(p)
            }
            stepper("Attempts before toxic", hive.policy.maxAttemptsPerTask, 1...10) {
                var p = hive.policy; p.maxAttemptsPerTask = $0; hive.updatePolicy(p)
            }
            stepper("Bees per hour", hive.policy.maxBeesPerHour, 1...60) {
                var p = hive.policy; p.maxBeesPerHour = $0; hive.updatePolicy(p)
            }
            stepper("Daily ceiling ($)", Int(hive.policy.dailyBudgetUSD), 1...1000) {
                var p = hive.policy; p.dailyBudgetUSD = Double($0); hive.updatePolicy(p)
            }
            // The ceiling above governs this copy alone. Whatever the sibling
            // probe last saw belongs directly under it, in all four of its
            // states, so the operator reads the two numbers together.
            if let report = hive.sibling {
                Text(report.summary(ownCeiling: hive.policy.dailyBudgetUSD, ownArmed: hive.policy.enabled))
                    .font(.system(size: 10))
                    .foregroundColor(hive.siblingIsArmed || !hive.siblingExposureIsBounded ? .orange : .grokDim)
                    .fixedSize(horizontal: false, vertical: true)
            } else {
                Text("The sibling Hive has not been probed yet; that is not the same as there being none.")
                    .font(.system(size: 10))
                    .foregroundColor(.grokDim)
                    .fixedSize(horizontal: false, vertical: true)
            }
            stepper("Fails in a row -> pause", hive.policy.maxConsecutiveFailures, 1...20) {
                var p = hive.policy; p.maxConsecutiveFailures = $0; hive.updatePolicy(p)
            }
            toggle("Run the project's checks before review", hive.policy.verifyBeforeReview) {
                var p = hive.policy; p.verifyBeforeReview = $0; hive.updatePolicy(p)
            }
            toggle("Isolate each bee in a git worktree", hive.policy.useWorktree) {
                var p = hive.policy; p.useWorktree = $0; hive.updatePolicy(p)
            }
            toggle("Allow bees to push", hive.policy.allowPush) {
                var p = hive.policy; p.allowPush = $0; hive.updatePolicy(p)
            }

            if !hive.invariantViolations.isEmpty {
                ForEach(hive.invariantViolations) { violation in
                    Text("invariant `\(violation.id)`: \(violation.detail)")
                        .font(.system(size: 10))
                        .foregroundColor(.orange)
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    private func stepper(
        _ label: String, _ value: Int, _ range: ClosedRange<Int>,
        onChange: @escaping (Int) -> Void
    ) -> some View {
        HStack {
            Text(label).font(.system(size: 11)).foregroundColor(.grokMuted)
            Spacer()
            Text("\(value)")
                .font(.system(size: 11, design: .monospaced))
                .foregroundColor(.grokText)
            Stepper("") {
                if value < range.upperBound { onChange(value + 1) }
            } onDecrement: {
                if value > range.lowerBound { onChange(value - 1) }
            }
            .labelsHidden()
        }
    }

    private func toggle(_ label: String, _ isOn: Bool, onChange: @escaping (Bool) -> Void) -> some View {
        Toggle(isOn: Binding(get: { isOn }, set: onChange)) {
            Text(label).font(.system(size: 11)).foregroundColor(.grokMuted)
        }
        .toggleStyle(.switch)
    }

    // MARK: - Bees

    private var liveBees: some View {
        VStack(alignment: .leading, spacing: 6) {
            section("LIVE BEES")
            if hive.bees.isEmpty {
                emptyRow("No bee has been sent out yet.")
            } else {
                ForEach(hive.bees) { bee in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(bee.title)
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundColor(.grokText)
                            Spacer()
                            Text(bee.status.label)
                                .font(.system(size: 9, weight: .bold, design: .monospaced))
                                .foregroundColor(colour(for: bee.status))
                        }
                        Text(bee.lastLine)
                            .font(.system(size: 10))
                            .foregroundColor(.grokMuted)
                            .lineLimit(3)
                        HStack(spacing: 10) {
                            Text("tools \(bee.toolCalls)")
                            Text(elapsed(since: bee.startedAt))
                            if let branch = bee.branch { Text(branch) }
                            Spacer()
                            Text(bee.resumeCommand)
                                .font(.system(size: 9, design: .monospaced))
                                .foregroundColor(.grokDim)
                                .textSelection(.enabled)
                        }
                        .font(.system(size: 9))
                        .foregroundColor(.grokDim)
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.grokSurface)
                    .triosBubble(radius: 10)
                }
            }
        }
    }

    private func colour(for status: HiveBeeStatus) -> Color {
        switch status {
        case .starting, .working: return .yellow
        case .succeeded: return .green
        case .failed, .timedOut: return .red
        case .cancelled: return .grokDim
        }
    }

    // MARK: - Review

    private var reviewQueue: some View {
        let waiting = hive.tasks.filter { $0.state == .review }
        return Group {
            if !waiting.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    section("WAITING ON YOU")
                    ForEach(waiting) { task in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(task.title)
                                    .font(.system(size: 11, weight: .semibold))
                                    .foregroundColor(.grokText)
                                Spacer()
                                Text(verificationLabel(task))
                                    .font(.system(size: 9, weight: .bold, design: .monospaced))
                                    .foregroundColor(verificationColour(task))
                                evidenceBadge(task)
                            }
                            if let verification = task.verification {
                                Text(verification)
                                    .font(.system(size: 9))
                                    .foregroundColor(.grokDim)
                                    .lineLimit(4)
                            }
                            if let summary = task.resultSummary {
                                Text(summary)
                                    .font(.system(size: 10))
                                    .foregroundColor(.grokMuted)
                                    .lineLimit(6)
                            }
                            HStack(spacing: 10) {
                                Button("Accept") { hive.accept(task.id) }
                                    .buttonStyle(.plain)
                                    .font(.system(size: 10, weight: .semibold))
                                    .foregroundColor(.green)
                                Button("Send back") { hive.reject(task.id, why: "rejected on review") }
                                    .buttonStyle(.plain)
                                    .font(.system(size: 10, weight: .semibold))
                                    .foregroundColor(.red)
                                Spacer()
                                if let cost = task.costUSD {
                                    Text(String(format: "$%.3f", cost))
                                        .font(.system(size: 9, design: .monospaced))
                                        .foregroundColor(.grokDim)
                                }
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.grokSurface)
                        .triosBubble(radius: 10)
                    }
                }
            }
        }
    }

    /// Whether the evidence behind the verdict still describes the tree.
    /// A pass measured against a commit the tree has since moved past is not a
    /// pass now, and must not read like one.
    @ViewBuilder
    private func evidenceBadge(_ task: HiveTask) -> some View {
        let state = hive.evidenceState(for: task)
        if !state.isCurrent {
            Text(state.label)
                .font(.system(size: 8, weight: .bold, design: .monospaced))
                .foregroundColor(.orange)
                .padding(.horizontal, 5)
                .padding(.vertical, 1)
                .background(Color.grokElevated)
                .triosBubble(radius: 6)
        }
    }

    /// Three states drawn apart. UNVERIFIED must never look like a pass - it is
    /// the state in which nothing was checked at all.
    private func verificationLabel(_ task: HiveTask) -> String {
        switch task.verified {
        case .some(true): return "VERIFIED"
        case .some(false): return "BROKE THE BUILD"
        case .none: return "UNVERIFIED"
        }
    }

    private func verificationColour(_ task: HiveTask) -> Color {
        switch task.verified {
        case .some(true): return .green
        case .some(false): return .red
        case .none: return .orange
        }
    }

    // MARK: - Ranking

    private var ranking: some View {
        VStack(alignment: .leading, spacing: 6) {
            section("WHAT SHE RANKED")
            if hive.targets.isEmpty {
                emptyRow("Nothing scanned yet.")
            } else {
                // The order makes an assumption that cannot be read off the
                // numbers beside it - three are on display and only one sorts -
                // so the screen says which, in words, above the list.
                Text(HiveQueue.orderingSentence)
                    .font(.system(size: 8))
                    .foregroundColor(.grokDim)
                    .fixedSize(horizontal: false, vertical: true)
                ForEach(hive.eligibleRows.prefix(12)) { row in
                    targetRow(row)
                }
                if !hive.instrumentFaults.isEmpty {
                    instrumentFaultList
                }
            }
        }
    }

    /// Targets the scan could not read well enough to rank.
    ///
    /// Deliberately without a position and without a score. Printing either
    /// beside a module the scanner barely read states a comparison the scan
    /// never made - it ranks how well the instrument worked and presents the
    /// result as a ranking of code.
    private var instrumentFaultList: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text("NOT RANKED - INSTRUMENT FAULT")
                .font(.system(size: 8, weight: .bold, design: .monospaced))
                .foregroundColor(.orange)
            Text("Under \(Int(HiveInvariants.minimumDispatchConfidence * 100))% of the signal "
                + "weight was read on these, so they have no place in the queue. The remedy is "
                + "the probe, not a bee.")
                .font(.system(size: 8))
                .foregroundColor(.grokDim)
                .fixedSize(horizontal: false, vertical: true)
            ForEach(hive.instrumentFaults) { target in
                VStack(alignment: .leading, spacing: 1) {
                    HStack(spacing: 8) {
                        Text(target.module)
                            .font(.system(size: 9, weight: .semibold, design: .monospaced))
                            .foregroundColor(.grokMuted)
                        Spacer()
                        Text("\(Int(target.confidence * 100))% read")
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundColor(.orange)
                    }
                    Text(target.unreadProbeDetail)
                        .font(.system(size: 8))
                        .foregroundColor(.grokDim)
                        .lineLimit(2)
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    /// The pair above this row is not settled by the evidence.
    ///
    /// Three different sentences, because "no break-even" has two different
    /// causes and telling a reader that everything here was read when it was
    /// not would be the same class of untruth this whole wave is about.
    private func unsettledBadge(_ breakEven: HiveBreakEven?, _ target: HiveTarget) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text("NOT SETTLED BY THE EVIDENCE")
                .font(.system(size: 8, weight: .bold, design: .monospaced))
                .foregroundColor(.orange)
            Text(Self.unsettledExplanation(breakEven, target))
                .font(.system(size: 8))
                .foregroundColor(.grokDim)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    static func unsettledExplanation(_ breakEven: HiveBreakEven?, _ target: HiveTarget) -> String {
        if let breakEven {
            return "the unread signals here would only have to read \(breakEven.summary) "
                + "for this row to overtake the one above"
        }
        if target.unmeasuredShare > 0 {
            return "nothing the unread signals here could read would lift this row past the one "
                + "above; the doubt belongs to that row, whose own unread signals could still "
                + "put it below this one"
        }
        return "everything here was read - the doubt belongs to the row above, whose unread "
            + "signals could still put it below this one"
    }

    private func targetRow(_ row: HiveRankedTarget) -> some View {
        let target = row.target
        let isExpanded = expandedTarget == target.id
        return VStack(alignment: .leading, spacing: 6) {
            // When the evidence does not separate this row from the one above,
            // the screen says so. Picking one of two indistinguishable targets
            // and presenting it as a rank is the defect, not the fix.
            if case .notSettled(let breakEven) = row.separation {
                unsettledBadge(breakEven, target)
            }

            HStack(spacing: 8) {
                Text("\(row.position)")
                    .font(.system(size: 9, design: .monospaced))
                    .foregroundColor(.grokDim)
                    .frame(width: 16, alignment: .trailing)
                VStack(alignment: .leading, spacing: 1) {
                    Text(target.module)
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundColor(.grokText)
                    Text(target.reason)
                        .font(.system(size: 9))
                        .foregroundColor(.grokMuted)
                        .lineLimit(1)
                }
                Spacer()
                Text(target.realm.rawValue)
                    .font(.system(size: 8, weight: .medium))
                    .foregroundColor(.grokDim)
                VStack(alignment: .trailing, spacing: 1) {
                    // The key the queue is ordered on.
                    Text(String(format: "%.2f", target.priorImputedScore))
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundColor(.grokText)
                    // What the evidence leaves open, either side of it.
                    Text(String(format: "%.2f-%.2f", target.lowerBound, target.upperBound))
                        .font(.system(size: 8, design: .monospaced))
                        .foregroundColor(.grokDim)
                    // Confidence sits beside the score, never folded into it:
                    // a half-measured module must not read as a confident one.
                    Text("conf \(Int(target.confidence * 100))%")
                        .font(.system(size: 8, design: .monospaced))
                        .foregroundColor(target.confidence >= 0.8 ? .grokDim : .orange)
                }
                Button {
                    withAnimation { expandedTarget = isExpanded ? nil : target.id }
                } label: {
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 9))
                        .foregroundColor(.grokDim)
                }
                .buttonStyle(.plain)
            }

            if isExpanded {
                ForEach(target.signals) { signal in
                    signalRow(signal)
                }
                // All three numbers, each under the name that says what it
                // assumes. Only the first one sorts.
                HStack(spacing: 10) {
                    keyReadout("queue key", target.priorImputedScore)
                    keyReadout("measured only", target.score)
                    keyReadout("zero-imputed", target.zeroImputedScore)
                    Spacer()
                }
                HStack {
                    Text(target.path)
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundColor(.grokDim)
                    Spacer()
                    Button("Not worth it") {
                        hive.skip(module: target.module, why: "operator: not worth a bee")
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 10))
                    .foregroundColor(.grokMuted)
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    private func keyReadout(_ label: String, _ value: Double) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(label)
                .font(.system(size: 8))
                .foregroundColor(.grokDim)
            Text(String(format: "%.3f", value))
                .font(.system(size: 9, design: .monospaced))
                .foregroundColor(.grokText)
        }
    }

    @ViewBuilder
    private func signalRow(_ signal: HiveSignal) -> some View {
        HStack(spacing: 8) {
            Text(signal.kind.label)
                .font(.system(size: 9))
                .foregroundColor(.grokMuted)
                .frame(width: 130, alignment: .leading)
            if let value = signal.raw.value {
                Text(value == value.rounded() ? "\(Int(value))" : String(format: "%.2f", value))
                    .font(.system(size: 9, design: .monospaced))
                    .foregroundColor(.grokText)
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Color.grokBorder)
                        Capsule()
                            .fill(Color.grokText.opacity(0.55))
                            .frame(width: max(2, geo.size.width * min(max(signal.normalized.value ?? 0, 0), 1)))
                    }
                }
                .frame(height: 3)
            } else {
                // Printed in full and in warning colour: a signal she could not
                // read must show as a gap, not blend in as a low value.
                Text("NOT MEASURED - \(signal.raw.unmeasuredReason ?? "unknown")")
                    .font(.system(size: 9))
                    .foregroundColor(.orange)
                Spacer()
            }
        }
    }

    // MARK: - Audit

    private var auditLog: some View {
        VStack(alignment: .leading, spacing: 4) {
            section("AUDIT")
            if hive.events.isEmpty {
                emptyRow("nothing recorded yet")
            } else {
                ForEach(hive.events.prefix(20)) { event in
                    HStack(alignment: .top, spacing: 8) {
                        Text(time(event.timestamp))
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundColor(.grokDim)
                        Text(event.kind)
                            .font(.system(size: 8, weight: .bold, design: .monospaced))
                            .foregroundColor(.grokMuted)
                            .frame(width: 110, alignment: .leading)
                        Text(event.detail)
                            .font(.system(size: 8))
                            .foregroundColor(.grokDim)
                            .lineLimit(2)
                        Spacer()
                    }
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.grokSurface)
        .triosBubble(radius: 10)
    }

    // MARK: - Helpers

    private func section(_ title: String) -> some View {
        Text(title)
            .font(.system(size: 9, weight: .bold))
            .foregroundColor(.grokDim)
    }

    private func emptyRow(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 10))
            .foregroundColor(.grokDim)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.grokSurface)
            .triosBubble(radius: 10)
    }

    private func elapsed(since date: Date) -> String {
        format(tick.timeIntervalSince(date))
    }

    private func elapsed(until date: Date) -> String {
        format(date.timeIntervalSince(tick))
    }

    private func format(_ interval: TimeInterval) -> String {
        let total = Int(max(0, interval))
        return total < 60 ? "\(total)s" : "\(total / 60)m \(total % 60)s"
    }

    private func time(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        return formatter.string(from: date)
    }
}
