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
            Text(hive.policy.enabled ? "ARMED" : "IDLE")
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(hive.policy.enabled ? .green : .grokDim)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(Color.grokElevated)
                .triosBubble(radius: 9)
        }
    }

    private var subtitle: String {
        var parts: [String] = []
        if let scanned = hive.lastScanAt {
            parts.append("ranked \(hive.targets.count) modules \(elapsed(since: scanned)) ago")
        }
        if let next = hive.nextCycleAt, hive.policy.enabled {
            parts.append("next cycle in \(elapsed(until: next))")
        }
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
            if hive.policy.enabled {
                pill("Pause", "pause.fill") { hive.disarm() }
                pill("Stop bees", "stop.fill") { hive.stopAllBees() }
            } else {
                pill("Run 24/7", "play.fill") { hive.arm() }
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
                ForEach(Array(hive.eligibleTargets.prefix(12).enumerated()), id: \.element.id) { index, target in
                    targetRow(index: index, target: target)
                }
            }
        }
    }

    private func targetRow(index: Int, target: HiveTarget) -> some View {
        let isExpanded = expandedTarget == target.id
        return VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Text("\(index + 1)")
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
                    Text(String(format: "%.2f", target.score))
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundColor(.grokText)
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
