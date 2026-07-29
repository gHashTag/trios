import Foundation

/// Opens and closes the Queen's worker chats.
///
/// This is the half that was missing: the registry recorded delegation and the
/// policy decided whether it was allowed, but nothing actually created a chat or
/// a branch. The Queen delegates by performing three steps atomically enough
/// that a failure never leaves a half-open task:
///
/// 1. refuse the delegation if policy says no,
/// 2. create the worker's own chat,
/// 3. create the GitButler virtual branch that isolates its edits.
///
/// Virtual branches are what let several bees share one checkout - each task's
/// changes are attributed to its own branch in the same working directory, so
/// there is no worktree to duplicate and no checkout to switch.
@MainActor
final class QueenDelegationService {
    /// Creates a conversation and returns its id.
    typealias ChatFactory = (String) async -> UUID?
    /// Creates the isolating virtual branch. Returns false when it could not.
    typealias BranchFactory = (String) async -> Bool
    /// Posts the brief into the worker's chat.
    typealias Briefer = (UUID, String) async -> Void

    private let registry: QueenDelegationRegistry
    private let makeChat: ChatFactory
    private let makeBranch: BranchFactory
    private let brief: Briefer

    init(
        registry: QueenDelegationRegistry,
        makeChat: @escaping ChatFactory,
        makeBranch: @escaping BranchFactory,
        brief: @escaping Briefer
    ) {
        self.registry = registry
        self.makeChat = makeChat
        self.makeBranch = makeBranch
        self.brief = brief
    }

    /// Outcome of a delegation attempt, so callers can report the reason rather
    /// than silently doing nothing.
    enum Outcome: Equatable {
        case delegated(DelegatedTask)
        case refused(String)
    }

    @discardableResult
    func delegate(
        issueText: String,
        title: String,
        worker: String,
        ownedPaths: [String] = []
    ) async -> Outcome {
        guard let issue = IssueReference.parse(issueText) else {
            return refuse("'\(issueText)' is not a GitHub issue. Use owner/repo#123 or the issue URL.")
        }
        // Check policy before creating anything, so a refusal leaves no orphan
        // chat or branch behind.
        if let existing = registry.task(forIssue: issue) {
            return refuse("\(issue.slug) is already delegated to \(existing.worker).")
        }
        if let reason = registry.delegationBlockReason(paths: ownedPaths) {
            return refuse(reason)
        }

        guard let conversationId = await makeChat("\(issue.slug) \(title)") else {
            return refuse("Could not open a chat for \(issue.slug).")
        }

        guard let task = registry.delegate(
            issue: issue,
            title: title,
            worker: worker,
            conversationId: conversationId,
            ownedPaths: ownedPaths
        ) else {
            return refuse(registry.lastError ?? "Delegation was refused.")
        }

        // The branch isolates the work. If it cannot be created the task still
        // exists but must not be told to start, or two bees end up editing the
        // same files on the same branch.
        if let branch = task.virtualBranch {
            let created = await makeBranch(branch)
            if !created {
                registry.transition(taskID: task.id, to: .cancelled)
                return refuse("Could not create the virtual branch \(branch); delegation rolled back.")
            }
        }

        await brief(conversationId, briefing(for: task))
        registry.transition(taskID: task.id, to: .running)
        return .delegated(task)
    }

    /// The brief handed to a worker.
    ///
    /// Deliberately a *subset*: the worker gets its issue, its branch and its
    /// file boundary, not the Queen's conversation. The supervisor pattern's
    /// main failure mode is the orchestrator's context leaking into every
    /// worker until nobody has room to think.
    func briefing(for task: DelegatedTask) -> String {
        var lines = [
            "You are working on \(task.issue.slug).",
            "Issue: \(task.issue.url)",
            "Task: \(task.title)"
        ]
        if let branch = task.virtualBranch {
            lines.append("Your virtual branch: \(branch). All your edits belong to it.")
        }
        if task.ownedPaths.isEmpty {
            lines.append("No file boundary was set. Ask before touching shared files.")
        } else {
            lines.append("You own these paths and only these: \(task.ownedPaths.joined(separator: ", ")).")
        }
        lines.append("Report back when done; the Queen reviews before anything lands.")
        return lines.joined(separator: "\n")
    }

    private func refuse(_ reason: String) -> Outcome {
        TriosLogBus.shared.warn(.queen, "queen.delegate.refused", reason)
        return .refused(reason)
    }
}
