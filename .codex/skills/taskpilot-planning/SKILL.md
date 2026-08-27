---
name: taskpilot-planning
description: Plan and maintain Codex Buddy work as validated TaskPilot epics, features, tasks, dependencies, and status updates. Use when decomposing roadmap work or recording implementation progress; do not use as a substitute for inspecting the relevant code.
---

# TaskPilot Planning

Use the repository-local `taskpilot` CLI. Treat `.taskpilot/` as canonical project data.

Before creating items, run `taskpilot --json item list` and reuse or update matching items instead of creating duplicates. Use this hierarchy:

- Epic: a product or architecture outcome spanning multiple independently deliverable features.
- Feature: a coherent reviewable capability under an epic.
- Task: an implementation, test, migration, or measurement step under a feature. Direct epic tasks are acceptable only for cross-cutting work that does not form a feature.
- Bug: a defect discovered while executing a task; parent it to the owning task.

Descriptions should record scope, important constraints, observable acceptance criteria, and compatibility considerations. Keep them implementation-oriented and avoid copying the whole design report into every child.

Use `taskpilot item blocks` for real sequencing constraints and `taskpilot item relates` for non-blocking relationships. Add comments for decisions or discoveries that affect later work. Mark an item `in_progress` only when work has started and `done` only after its acceptance criteria are satisfied.

After mutations, run `taskpilot validate` and summarize the resulting hierarchy and blockers. Do not edit generated indexes, caches, or registry state by hand.
