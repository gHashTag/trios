import Foundation

// ===========================================================================
// DISPATCH DECISION - "should a bee go out right now, and for what?"
//
// Pure. No clock of its own, no disk, no processes, no actor. The orchestrator
// that owns the timer and the child processes asks this type what to do and
// then does it, which is the only reason the decision can be tested at all.
//
// In the copy this was ported from, the same logic lived inside a MainActor
// ObservableObject and had no tests. Every guardrail was therefore asserted
// about a policy struct rather than about the code that reads it.
// ===========================================================================

enum HiveDispatchDecision: Equatable {
    /// Send a bee for this task.
    case dispatch(HiveTask)
    /// Nothing is wrong; there is simply nothing to do.
    case idle(String)
    /// The loop must not dispatch, and this is why.
    case blocked(String)

    var isDispatch: Bool {
        if case .dispatch = self { return true }
        return false
    }

    var reason: String {
        switch self {
        case .dispatch(let task): return task.id
        case .idle(let why), .blocked(let why): return why
        }
    }
}

struct HiveDispatchContext: Equatable {
    var policy: HivePolicy
    var tasks: [HiveTask]
    /// Bees alive right now, counted by the orchestrator from live processes
    /// rather than from task state, so a crashed bee cannot hold a slot.
    var liveBees: Int
    var spentToday: Double
    var spawnsInLastHour: Int
    var consecutiveFailures: Int
    /// Whether the CLI preflight passed. Three states, so "not yet probed" is
    /// not silently treated as "signed in".
    var auth: HiveAuthState?
}

enum HiveDispatch {

    /// The single decision point. Order matters: the cheapest and most
    /// dangerous refusals are checked first, so a signed-out CLI or a tripped
    /// breaker can never be masked by an empty queue.
    static func decide(_ context: HiveDispatchContext) -> HiveDispatchDecision {
        let policy = context.policy

        guard policy.enabled else {
            return .idle("the loop is not armed")
        }

        let violations = HiveInvariants.check(
            policy: policy,
            tasks: context.tasks,
            spentToday: context.spentToday
        )
        if let first = violations.first {
            return .blocked(
                "standing invariant `\(first.id)` is violated: \(first.detail)"
                    + (violations.count > 1 ? " (and \(violations.count - 1) more)" : "")
            )
        }

        guard let auth = context.auth else {
            return .blocked("the claude CLI has not been probed yet")
        }
        guard auth.canSpawn else {
            return .blocked(auth.blockerText ?? "the claude CLI cannot spawn")
        }

        guard context.consecutiveFailures < policy.maxConsecutiveFailures else {
            return .blocked(
                "circuit breaker: \(context.consecutiveFailures) bees failed in a row "
                    + "(limit \(policy.maxConsecutiveFailures)) - fix the cause, then re-arm"
            )
        }

        guard context.spentToday < policy.dailyBudgetUSD else {
            return .blocked(
                String(
                    format: "daily ceiling reached - $%.2f of $%.2f spent today",
                    context.spentToday, policy.dailyBudgetUSD
                )
            )
        }

        guard context.liveBees < policy.maxConcurrentBees else {
            return .idle("all \(policy.maxConcurrentBees) bee slots are busy")
        }

        guard context.spawnsInLastHour < policy.maxBeesPerHour else {
            return .idle(
                "\(context.spawnsInLastHour) bees already started this hour "
                    + "(limit \(policy.maxBeesPerHour))"
            )
        }

        guard let next = nextTask(in: context) else {
            return .idle("nothing schedulable in the queue")
        }
        return .dispatch(next)
    }

    /// Highest score first among tasks that may still be attempted, skipping
    /// anything the operator has ruled out and anything already running.
    static func nextTask(in context: HiveDispatchContext) -> HiveTask? {
        context.tasks
            .filter(\.isSchedulable)
            .filter { context.policy.skippedModules[$0.module] == nil }
            .filter { $0.attempts < context.policy.maxAttemptsPerTask }
            .sorted {
                // Score first, then the oldest task, so a tie cannot starve one
                // module forever behind another with the same score.
                if $0.score != $1.score { return $0.score > $1.score }
                if $0.createdAt != $1.createdAt { return $0.createdAt < $1.createdAt }
                return $0.id < $1.id
            }
            .first
    }

    /// How a finished bee changes a task. Kept here, beside the dispatch rule,
    /// because the two together are the whole state machine.
    static func outcomeState(
        for task: HiveTask,
        verdict: HiveVerdict?,
        beeSucceeded: Bool,
        policy: HivePolicy
    ) -> (state: HiveTaskState, countsAsFailure: Bool) {
        guard beeSucceeded else {
            let exhausted = task.attempts >= policy.maxAttemptsPerTask
            return (exhausted ? .toxic : .failed, true)
        }
        guard policy.verifyBeforeReview else {
            return (.review, false)
        }
        switch verdict {
        case .some(.passed):
            return (.review, false)
        case .some(.failed):
            // The bee claimed success and the checks disagree. Checks win.
            let exhausted = task.attempts >= policy.maxAttemptsPerTask
            return (exhausted ? .toxic : .failed, true)
        case .some(.unavailable), .none:
            // Neither pass nor fail: it reaches review flagged unchecked, and
            // does not reset a failure streak it never earned.
            return (.review, false)
        }
    }
}
