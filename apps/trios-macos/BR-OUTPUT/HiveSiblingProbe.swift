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
//
// What it does with that reading is the middle of three options, and the two
// it is not are worth naming. A hard block on "the sibling is armed" would
// make the ceiling meaningful and would also hand a second, independent
// program a veto over this one: a state file left behind by a process that
// died still says `enabled: true`, and this copy would sit blocked forever on
// a corpse's promise. Warning only - what this file did until now - keeps the
// two loops independent and leaves the stated ceiling a per-copy share, which
// is to say not a ceiling.
//
// The middle path, implemented here: charge the sibling's already-committed
// spend for today against THIS copy's ceiling. The pair then shares one
// ceiling instead of getting one each, no state file can veto a dispatch on
// its own (an idle sibling has committed nothing and so costs nothing), and
// the arithmetic is a subtraction rather than a gate.
//
// The honest limit on that claim: only this copy is changed. It stops
// contributing once the pair's observed total reaches the number in this
// window; the sibling keeps enforcing its own. So the pair's combined spend is
// bounded by the LARGER of the two ceilings plus what is in flight, not by
// their sum - "shared rather than doubled", which is what was asked for, and
// not "jointly enforced", which one process cannot deliver for two.
//
// A debit is only honoured while the sibling's file proves its writer is still
// running (see `HiveSiblingFreshness`). A reservation is a claim about a bee
// that is running right now; once the process holding it is provably gone, the
// claim describes money that may never be spent, and honouring it forever
// would be the deadlock the hard block was rejected for.
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

/// Whether the sibling's state file proves that the process writing it is
/// still running.
///
/// An armed Hive rewrites its file every cycle, in flight bees or not, so
/// silence past a few cycles is the ordinary evidence that its process is
/// gone. This is deliberately a separate axis from `HiveSiblingState`: the
/// state is what the document says, and freshness is whether anyone is still
/// saying it. Folding the two together would have produced a fifth state whose
/// meaning changed with the clock.
///
/// It governs exactly one thing - whether the sibling's committed spend is
/// charged against this copy's ceiling. It deliberately does not change
/// `combinedExposure`: an over-stated exposure costs the operator nothing, and
/// an over-honoured debit costs the loop its ability to run at all.
enum HiveSiblingFreshness: Equatable {
    /// Written within the liveness horizon. The only state that proves life.
    case fresh(age: TimeInterval, horizon: TimeInterval)
    /// Older than the horizon. The writer is presumed gone.
    case stale(age: TimeInterval, horizon: TimeInterval)
    /// No usable timestamp: none recorded, none parseable, or one far enough
    /// in the future that the two clocks cannot be reconciled. Not proof of
    /// life, and not proof of death either - just no proof.
    case unestablished(String)

    /// True only for `fresh`. Every other case, including the two that merely
    /// failed to establish an answer, counts as no proof - which is the cheap
    /// direction: it costs a warning, never a stuck loop.
    var provesLife: Bool {
        if case .fresh = self { return true }
        return false
    }

    var age: TimeInterval? {
        switch self {
        case .fresh(let age, _), .stale(let age, _): return age
        case .unestablished: return nil
        }
    }

    var horizon: TimeInterval? {
        switch self {
        case .fresh(_, let horizon), .stale(_, let horizon): return horizon
        case .unestablished: return nil
        }
    }

    var label: String {
        switch self {
        case .fresh: return "FRESH"
        case .stale: return "STALE"
        case .unestablished: return "UNDATED"
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
    /// Whether the file proves its writer is alive.
    let freshness: HiveSiblingFreshness
    /// The sibling ledger's entry for today, read whenever the document could
    /// be parsed at all. `nil` is unknown, never zero.
    let recordedSpendToday: Double?
    let checkedAt: Date

    init(
        path: String?,
        state: HiveSiblingState,
        freshness: HiveSiblingFreshness = .unestablished("the state file carried no usable timestamp"),
        recordedSpendToday: Double? = nil,
        checkedAt: Date = Date()
    ) {
        self.path = path
        self.state = state
        self.freshness = freshness
        self.recordedSpendToday = recordedSpendToday
        self.checkedAt = checkedAt
    }

    /// Dollars the sibling has already committed today that this copy charges
    /// against its own daily ceiling.
    ///
    /// Three conditions, all required, and each of them is a refusal to guess:
    /// the sibling must say it is armed (a disarmed loop is not racing anyone
    /// to the ceiling), its ledger must have been read (an unknown spend is
    /// not a zero and is not a ceiling either, so it buys nothing), and its
    /// file must prove its writer is alive (money reserved by a dead process
    /// is money that may never be spent).
    ///
    /// Zero is therefore the answer in every case where something could not be
    /// established, which is the failure direction that costs a doubled ceiling
    /// rather than a loop that can never dispatch again.
    var committedSpendUSD: Double {
        guard state.isArmed, freshness.provesLife, let spent = recordedSpendToday else { return 0 }
        return max(0, spent)
    }

    /// What is left of `ownCeiling` once this copy's own spend and the
    /// sibling's committed spend are both taken off it. Never below zero.
    func sharedHeadroom(ownCeiling: Double, ownSpentToday: Double) -> Double {
        max(0, ownCeiling - ownSpentToday - committedSpendUSD)
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
                + " " + sharedCeilingNote(ownCeiling: ownCeiling)
        case .unreadable(let why):
            return "The sibling Hive state at \(location) could not be read: \(why). "
                + "Its ceiling is unknown, so combined exposure is at least $\(money(ownCeiling))/day and may be higher."
        }
    }

    /// One sentence saying what this copy charges itself on the sibling's
    /// behalf, and - when it charges nothing - which of the three conditions
    /// was not met. Kept beside the arithmetic so the wording cannot drift
    /// away from the number, and so a "$0.00 charged" can never be printed
    /// without the reason it is zero.
    func sharedCeilingNote(ownCeiling: Double) -> String {
        guard state.isArmed else {
            switch state {
            case .unreadable:
                return "Nothing is charged against this copy's ceiling: the sibling could not be read, "
                    + "so the two ceilings are not shared and this copy's may be spent in full on top of it."
            default:
                return "Nothing is charged against this copy's ceiling: the sibling is \(state.label.lowercased()) "
                    + "and is not dispatching."
            }
        }
        guard freshness.provesLife else {
            let why: String
            switch freshness {
            case .stale(let age, let horizon):
                why = "its file last changed \(Self.duration(age)) ago, past the \(Self.duration(horizon)) liveness horizon"
            case .unestablished(let reason):
                why = reason
            case .fresh:
                why = ""
            }
            return "The sibling says it is armed but does not prove it is running (\(why)), "
                + "so nothing is charged against this copy's ceiling - a state file cannot block this loop "
                + "on its own. Treat the combined figure as a warning, not a bound."
        }
        guard let spent = recordedSpendToday else {
            return "The sibling is armed and running but records no ledger, so there is no committed spend "
                + "to charge - the ceiling below is this copy's alone."
        }
        return "$\(money(max(0, spent))) of it is charged against this copy's $\(money(ownCeiling)) ceiling "
            + "(the sibling's committed spend today), leaving $\(money(max(0, ownCeiling - max(0, spent)))) "
            + "before this copy stops dispatching."
    }

    private func combinedExposureIfArmed(ownCeiling: Double, ceiling: Double?) -> Double {
        ownCeiling + (ceiling ?? 0)
    }

    private func money(_ value: Double) -> String {
        String(format: "%.2f", value)
    }

    /// Whole minutes and hours, ASCII only. `TimeInterval` prints as seconds
    /// with a fraction, which is unreadable in a sentence about a clock.
    static func duration(_ seconds: TimeInterval) -> String {
        let total = Int(seconds.rounded())
        guard total >= 60 else { return "\(max(0, total))s" }
        let minutes = total / 60
        guard minutes >= 60 else { return "\(minutes)m" }
        let hours = minutes / 60
        let rest = minutes % 60
        return rest == 0 ? "\(hours)h" : "\(hours)h \(rest)m"
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

        // The file's own modification date is the fallback proof of life, used
        // only when the document does not carry a usable timestamp of its own.
        let modifiedAt = (try? URL(fileURLWithPath: path)
            .resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate

        let reading = Self.read(data, now: now, modifiedAt: modifiedAt)
        return HiveSiblingReport(
            path: path,
            state: reading.state,
            freshness: reading.freshness,
            recordedSpendToday: reading.recordedSpendToday,
            checkedAt: now
        )
    }

    /// Everything one reading of the document yields.
    struct Reading: Equatable {
        let state: HiveSiblingState
        let freshness: HiveSiblingFreshness
        let recordedSpendToday: Double?
    }

    /// Reads the sibling's document by key rather than by decoding it into
    /// `HiveState`.
    ///
    /// `HiveState`'s decoder fills every missing field with a default, which is
    /// right for loading one's own file and wrong for reading someone else's:
    /// an empty object would decode into a disarmed hive with a $25 ceiling and
    /// report a confident answer about a file that said nothing. Here a missing
    /// key is a missing key.
    static func read(_ data: Data, now: Date = Date(), modifiedAt: Date? = nil) -> Reading {
        guard let parsed = try? JSONSerialization.jsonObject(with: data),
              let root = parsed as? [String: Any] else {
            return Reading(
                state: .unreadable("the file is not a JSON object"),
                freshness: .unestablished("the file is not a JSON object, so it carries no timestamp"),
                recordedSpendToday: nil
            )
        }

        let freshness = self.freshness(in: root, now: now, modifiedAt: modifiedAt)
        let spent = spentToday(in: root, now: now)

        func reading(_ state: HiveSiblingState) -> Reading {
            Reading(state: state, freshness: freshness, recordedSpendToday: spent)
        }

        guard let policy = root["policy"] as? [String: Any] else {
            return reading(.unreadable("the document has no policy block"))
        }
        guard let enabled = policy["enabled"] as? Bool else {
            // Without this flag armed and disarmed cannot be told apart, and
            // guessing "disarmed" is guessing in the expensive direction.
            return reading(.unreadable("the policy records no enabled flag"))
        }

        let ceiling = (policy["dailyBudgetUSD"] as? NSNumber)?.doubleValue

        guard enabled else { return reading(.disarmed(dailyBudgetUSD: ceiling)) }
        guard let ceiling else {
            return reading(.unreadable("the sibling is armed but records no dailyBudgetUSD"))
        }
        return reading(.armed(dailyBudgetUSD: ceiling, spentToday: spent))
    }

    /// The state alone, for callers that only classify the document.
    static func interpret(_ data: Data, now: Date = Date()) -> HiveSiblingState {
        read(data, now: now).state
    }

    // MARK: - Liveness

    /// Missed cycles before the writer is presumed gone.
    ///
    /// Three, the ordinary heartbeat convention: one missed write is a slow
    /// cycle, two is a long scan, three in a row is a process that is not
    /// coming back. An armed Hive persists at the end of every cycle whether
    /// or not a bee is in flight, so its cycle interval is its heartbeat.
    static let missedCyclesBeforePresumedDead: Double = 3

    /// Used when the sibling's document does not record its cycle interval.
    /// The same default the policy itself carries.
    static let defaultCycleIntervalSeconds: TimeInterval = 900

    /// Floor and ceiling on the horizon, both there to stop a number the
    /// sibling chose from making this copy's behaviour absurd.
    ///
    /// The floor stops a sibling configured with a 30-second cycle from being
    /// declared dead during one slow scan. The ceiling stops a sibling with a
    /// 24-hour cycle from holding a debit for three days: past six hours the
    /// evidence of life is too thin to charge money against, and the day
    /// rollover would clear the ledger entry anyway.
    static let minimumStalenessHorizon: TimeInterval = 1800
    static let maximumStalenessHorizon: TimeInterval = 6 * 3600

    static func stalenessHorizon(cycleIntervalSeconds: Double?) -> TimeInterval {
        let cycle = cycleIntervalSeconds.flatMap { $0 > 0 ? $0 : nil } ?? defaultCycleIntervalSeconds
        return min(
            max(missedCyclesBeforePresumedDead * cycle, minimumStalenessHorizon),
            maximumStalenessHorizon
        )
    }

    /// When the document says it was last written, and whether that is recent
    /// enough to prove the writer is still there.
    ///
    /// The document's own `updatedAt` is preferred over the file's
    /// modification date: it is the writer's own statement, written on every
    /// persist, and it survives a copy that resets mtime. The mtime is the
    /// fallback, not the primary, because a plain `cp` of an old state file
    /// would otherwise make a corpse look alive.
    static func freshness(
        in root: [String: Any],
        now: Date,
        modifiedAt: Date?
    ) -> HiveSiblingFreshness {
        let horizon = stalenessHorizon(
            cycleIntervalSeconds: ((root["policy"] as? [String: Any])?["cycleIntervalSeconds"] as? NSNumber)?
                .doubleValue
        )

        let parsed = parseUpdatedAt(root["updatedAt"])
        let stamp: Date
        if let declared = parsed.date {
            stamp = declared
        } else if let modifiedAt {
            stamp = modifiedAt
        } else {
            return .unestablished(parsed.why)
        }

        let age = now.timeIntervalSince(stamp)
        if age > horizon {
            return .stale(age: age, horizon: horizon)
        }
        if age < -horizon {
            // Dated well into the future. Two clocks that disagree by more
            // than the horizon cannot be used to measure it.
            return .unestablished(
                "its timestamp is \(duration(-age)) in the future, so the two clocks cannot be compared"
            )
        }
        return .fresh(age: max(0, age), horizon: horizon)
    }

    /// Both copies write `updatedAt` as an ISO 8601 string. A number is
    /// refused rather than guessed at: `JSONEncoder` counts from 2001 by
    /// default and from 1970 on request, and reading the wrong one would
    /// misdate the file by thirty-one years in whichever direction happened to
    /// be wrong.
    private static func parseUpdatedAt(_ value: Any?) -> (date: Date?, why: String) {
        guard let value else {
            return (nil, "the document records no updatedAt")
        }
        guard let text = value as? String else {
            return (nil, "the document records updatedAt as a number, whose epoch cannot be determined")
        }
        if let date = fractionalISO8601.date(from: text) ?? plainISO8601.date(from: text) {
            return (date, "")
        }
        return (nil, "the document's updatedAt is not an ISO 8601 timestamp")
    }

    private static let fractionalISO8601: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()

    private static let plainISO8601: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter
    }()

    private static func duration(_ seconds: TimeInterval) -> String {
        HiveSiblingReport.duration(seconds)
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
