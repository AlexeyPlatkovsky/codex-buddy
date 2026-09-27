# Codex Buddy changelog

## Unreleased

## 1.157.1 — 2026-09-27

Based on [Codex 0.157.1](https://github.com/openai/codex/releases/tag/rust-v0.157.1)
(`36650394c5`), incorporating upstream changes since `16ff14c266`.
Buddy now uses `1.MINOR.PATCH` for the corresponding Codex `0.MINOR.PATCH` release.
Buddy-only changes are recorded under Unreleased until the next upstream release sync.

### Features

- Updated the model catalog with GPT-6 Sol and Luna, including Amazon Bedrock support.
- Integrated fullscreen transcripts, text selection and copying, Shift-click selection
  extension, and Mermaid, math, and table rendering controls.
- Preserved Buddy's pinned agent tree, agent navigation, model and role metadata, and
  context usage display across the upstream transcript changes.
- Integrated upstream conversation forking from sessions open in another app and
  improved recovery of unsent question answers and queued input.
- Added upstream gateway OAuth sign-in and credential management, cloud skill catalog
  refresh, and MCP capability and attribution improvements.
- Integrated upstream Windows MXC sandbox support and network policy enforcement
  across HTTP, WebSocket, authentication, and app-server requests.
- Retained Buddy's coding runtime composition and explicit external-source controls.
  Features gated out of that runtime remain available only in the full Codex build;
  Buddy continues to use its in-process server rather than the shared full-runtime daemon.
- Set project subagents to Sol with high reasoning by default, reserving Astra for
  tasks that justify it.

### Fixes

- Buddy now identifies itself to Herdr inside macOS and Linux panes using Herdr's
  supported agent hint, while preserving its executable name, arguments, and branding.
  An explicitly configured `HERDR_AGENT` value takes precedence.
- Pasted local image paths become image attachment chips in both the main composer
  and question answers. Attachments survive queued-answer editing and draft recovery.
- Reduced stack usage during session initialization and thread start, resume, and
  fork dispatch, fixing embedded-session stack overflows at the normal test stack size.
- Restored per-tool runtime capability enforcement, built-in cleanup-hook trust
  and filtering, and executor MCP hook routing.
- Preserve the declined status of explicitly cancelled zsh subcommands in both
  command completion notifications and saved conversation history.
- Included upstream fixes for Windows daemon process handles, residual job membership,
  and unwanted console windows from MCP and code-mode child processes.
- Included upstream fixes for terminal scrollback over SSH, tmux mouse settings,
  composer draft recovery, and transcript selection.
- Included upstream fixes for transient file uploads, compaction resume boundaries,
  environment changes during turns, and subagent persistence and cancellation.

### Compatibility

- The upstream app-server API removes `thread/rollback`; clients should use
  `thread/revert`.
- Upstream image payloads can reference uploaded files through `fileId` (`file_id`
  in raw events). Inline image URLs remain supported. Raw completed-item events
  no longer expose internal executed-tool warehouse metadata.
- Upstream Windows requirements replace the private-desktop setting with updated
  sandbox implementation choices, including `mxc`. Legacy `friendly` and `pragmatic`
  personality values remain accepted but no longer select a response style.
- Upstream replaces orchestrator skills with host-provided cloud skills.
  `[orchestrator.skills]` is now ignored; use `[cloud.skills]` with a cloud skill
  provider. Skill-list callers must use the `cloud` authority. Old rollouts still
  load, but old orchestrator package aliases are not reusable.
- Version validation now requires Buddy's minor and patch numbers to match the pinned
  upstream release in `scripts/buddy_release/upstream-version.txt`. Individual tasks
  no longer increment the release version.

See the [upstream release history](https://github.com/openai/codex/releases) for the
complete upstream feature and fix lists.
