import Foundation

// ===========================================================================
// SIBLING BUDGET AWARENESS - what the other Hive is allowed to spend.
//
// Two copies of this loop exist on this machine. They keep their state in
// different files and so never race on disk, but their daily ceilings are
// independent: arming both spends up to twice what the operator entered in
// either window, and neither window says so. The ceiling looks like a total
// and is really a per-copy share.
//
// This probe reads the other copy's state file and reports what it found.
// It never blocks a dispatch. A budget that two programs enforce separately
// cannot be enforced from here anyway; what can be fixed is that the operator
// is told the real number before arming the second one.
// ===========================================================================

/// What is known about the sibling Hive.
///
/// Four cases, not two, and the pair that carries the whole point is `absent`
/// against `unreadable`. A file that could not be opened is not a file that is
/// not there. Folding the second into the first would report an armed sibling
/// as no sibling at all, which is the one direction of error that costs money.
enum HiveSiblingState: Equatable {
    /// Looked, and there is no state file. The sibling has never run here.
    case absent
    /// The file is there and its loop is switched off. Its ceiling is carried
    /// anyway, since arming it is one click away.
    case disarmed(dailyBudgetUSD: Double?)
    /// The file is there and its loop is armed. This is the case that doubles
    /// the operator's exposure.
    case armed(dailyBudgetUSD: Double, spentToday: Double?)
    /// Something is at that path but its contents could not be turned into an
    /// answer. Nothing may be inferred from this - least of all a zero.
    case unreadable(String)

    var isArmed: Bool {
        if case .armed = self { return true }
        return false
    }

    /// The sibling's daily ceiling when it is armed, and `nil` in every other
    /// case. Deliberately not "0 when unknown": an unknown ceiling that reads
    /// as zero is the mistake this file exists to prevent.
    var armedCeiling: Double? {
        if case .armed(let ceiling, _) = self { return ceiling }
        return nil
    }

    var label: String {
        switch self {
        case .absent: return "ABSENT"
        case .disarmed: return "DISARMED"
        case .armed: return "ARMED"
        case .unreadable: return "UNREADABLE"
        }
    }
}

/// One reading of the sibling's state file, with the path it was read from so
/// the operator can check the probe's own assumption about where to look.
struct HiveSiblingReport: Equatable {
    /// Where the probe looked. `nil` only when no candidate path could be
    /// formed at all, which is itself reported as `unreadable`.
    let path: String?
    let state: HiveSiblingState
    let checkedAt: Date

    init(path: String?, state: HiveSiblingState, checkedAt: Date = Date()) {
        self.path = path
        self.state = state
        self.checkedAt = checkedAt
    }

    /// The total both copies may spend in one local day: this copy's ceiling
    /// plus the sibling's, counted only while the sibling is armed. A disarmed
    /// sibling spends nothing, and an unknown one contributes nothing here
    /// because a guess would be reported as a measurement - `exposureIsBounded`
    /// is what says whether this number can be trusted as a total.
    func combinedExposure(ownCeiling: Double) -> Double {
        ownCeiling + (state.armedCeiling ?? 0)
    }

    /// False when the sibling could not be read, meaning `combinedExposure` is
    /// a floor rather than a ceiling.
    var exposureIsBounded: Bool {
        if case .unreadable = state { return false }
        return true
    }

    /// One sentence for the operator. Kept here rather than in the view so the
    /// wording is covered by tests and cannot drift away from the numbers.
    func summary(ownCeiling: Double, ownArmed: Bool) -> String {
        let location = path ?? "an undetermined path"
        switch state {
        case .absent:
            return "No sibling Hive state file at \(location). This copy's $\(money(ownCeiling))/day is the whole exposure."
        case .disarmed(let ceiling):
            let its = ceiling.map { "$\(money($0))/day" } ?? "an unrecorded ceiling"
            return "The sibling Hive at \(location) is present and disarmed (\(its)). "
                + "Arming it would raise combined exposure to $\(money(combinedExposureIfArmed(ownCeiling: ownCeiling, ceiling: ceiling)))/day."
        case .armed(let ceiling, let spent):
            let spendText = spent.map { "$\(money($0)) spent today" } ?? "today's spend not recorded"
            let head = ownArmed
                ? "Both Hives are armed."
                : "The sibling Hive is armed while this copy is idle."
            return head
                + " This copy: $\(money(ownCeiling))/day. Sibling at \(location): $\(money(ceiling))/day, \(spendText)."
                + " Combined exposure $\(money(combinedExposure(ownCeiling: ownCeiling)))/day."
        case .unreadable(let why):
            return "The sibling Hive state at \(location) could not be read: \(why). "
                + "Its ceiling is unknown, so combined exposure is at least $\(money(ownCeiling))/day and may be higher."
        }
    }

    private func combinedExposureIfArmed(ownCeiling: Double, ceiling: Double?) -> Double {
        ownCeiling + (ceiling ?? 0)
    }

    private func money(_ value: Double) -> String {
        String(format: "%.2f", value)
    }
}

/// Locates and reads the other Hive's state file.
///
/// The environment is injected rather than read at each call so a test can
/// describe a machine that has no `HOME` at all, and so no user name is ever
/// written into this source.
struct HiveSiblingProbe {

    /// Where the sibling keeps its state, relative to its root. The other copy
    /// writes `$TRINITY_ROOT/.trinity/queen/hive.json`; this is that tail.
    static let stateSuffix = ".trinity/queen/hive.json"

    /// The conventional location of the sibling root when `TRINITY_ROOT` is
    /// unset: `$HOME/trinity`.
    static let defaultRootName = "trinity"

    let environment: [String: String]

    init(environment: [String: String] = ProcessInfo.processInfo.environment) {
        self.environment = environment
    }

    /// The sibling root: the env var the other copy itself honours, then the
    /// conventional directory under the operator's home. `nil` when neither is
    /// available, which is reported as unreadable rather than absent - a path
    /// that could not be formed was never looked at.
    var siblingRoot: String? {
        if let root = environment["TRINITY_ROOT"], !root.isEmpty {
            return root
        }
        if let home = environment["HOME"], !home.isEmpty {
            return URL(fileURLWithPath: home)
                .appendingPathComponent(Self.defaultRootName)
                .path
        }
        return nil
    }

    var statePath: String? {
        siblingRoot.map { root in
            URL(fileURLWithPath: root).appendingPathComponent(Self.stateSuffix).path
        }
    }

    func probe(now: Date = Date()) -> HiveSiblingReport {
        guard let path = statePath else {
            return HiveSiblingReport(
                path: nil,
                state: .unreadable("neither TRINITY_ROOT nor HOME is set, so the sibling could not be located"),
                checkedAt: now
            )
        }

        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory) else {
            return HiveSiblingReport(path: path, state: .absent, checkedAt: now)
        }
        if isDirectory.boolValue {
            return HiveSiblingReport(
                path: path,
                state: .unreadable("a directory sits where the state file should be"),
                checkedAt: now
            )
        }

        let data: Data
        do {
            data = try Data(contentsOf: URL(fileURLWithPath: path))
        } catch {
            // The file is there. Whatever stopped the read - permissions, a
            // dead symlink, a full-disk-access prompt nobody answered - the
            // honest report is that it exists and was not read.
            return HiveSiblingReport(
                path: path,
                state: .unreadable(error.localizedDescription),
                checkedAt: now
            )
        }

        return HiveSiblingReport(path: path, state: Self.interpret(data, now: now), checkedAt: now)
    }

    /// Reads the sibling's document by key rather than by decoding it into
    /// `HiveState`.
    ///
    /// `HiveState`'s decoder fills every missing field with a default, which is
    /// right for loading one's own file and wrong for reading someone else's:
    /// an empty object would decode into a disarmed hive with a $25 ceiling and
    /// report a confident answer about a file that said nothing. Here a missing
    /// key is a missing key.
    static func interpret(_ data: Data, now: Date = Date()) -> HiveSiblingState {
        guard let parsed = try? JSONSerialization.jsonObject(with: data),
              let root = parsed as? [String: Any] else {
            return .unreadable("the file is not a JSON object")
        }
        guard let policy = root["policy"] as? [String: Any] else {
            return .unreadable("the document has no policy block")
        }
        guard let enabled = policy["enabled"] as? Bool else {
            // Without this flag armed and disarmed cannot be told apart, and
            // guessing "disarmed" is guessing in the expensive direction.
            return .unreadable("the policy records no enabled flag")
        }

        let ceiling = (policy["dailyBudgetUSD"] as? NSNumber)?.doubleValue

        guard enabled else { return .disarmed(dailyBudgetUSD: ceiling) }
        guard let ceiling else {
            return .unreadable("the sibling is armed but records no dailyBudgetUSD")
        }
        return .armed(dailyBudgetUSD: ceiling, spentToday: spentToday(in: root, now: now))
    }

    /// Today's spend from the sibling's ledger.
    ///
    /// `nil` when the document carries no ledger at all - that is unknown, not
    /// zero. A ledger that exists but has no entry for today genuinely is zero:
    /// that is exactly how the writer encodes a day with no spend. Both copies
    /// key the ledger by local day through the same model, so the key computed
    /// here is the key the sibling wrote.
    private static func spentToday(in root: [String: Any], now: Date) -> Double? {
        guard let ledger = root["spendByDay"] as? [String: Any] else { return nil }
        guard let entry = ledger[HiveState.dayKey(now)] as? NSNumber else { return 0 }
        return entry.doubleValue
    }
}
