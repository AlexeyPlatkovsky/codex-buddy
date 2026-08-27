---
name: upstream-sync
description: Inspect, plan, or perform a safe Codex Buddy synchronization with openai/codex while preserving fork-owned composition and roadmap work. Use for upstream fetches, merge rehearsals, conflict analysis, or sync follow-ups; do not push or resolve destructive conflicts without authorization.
---

# Upstream Sync

The expected remotes are `upstream = openai/codex` and `origin = AlexeyPlatkovsky/codex-buddy`. Verify them and the worktree state before changing branches or history.

For analysis-only requests, fetch only when the user asked for current upstream state; otherwise inspect existing refs. Compare the fork merge base, upstream changes, and fork changes. Pay special attention to high-churn orchestration files, Cargo dependency graphs, config schemas, app-server v2 protocol, rollout compatibility, and TUI snapshots.

Prefer an ordinary upstream merge into the fork integration branch. Do not rebase or force-push shared branches unless the user explicitly chooses that workflow. Never discard uncommitted changes. Resolve conflicts by preserving upstream behavior first, then reapplying the fork through its runtime profile, composition boundaries, and fork-owned modules.

After a sync, run checks proportional to the touched crates and follow the repository `AGENTS.md` test rules. Record material conflict patterns or deferred work as comments or tasks in TaskPilot, then run `taskpilot validate`. Report the upstream range, resulting commit, conflicts, validation performed, and whether pushing still requires authorization.
