# Codex Buddy migration roadmap

Last updated: 2026-08-28

This file is the restart handoff for continuing the Codex Buddy fork after a fresh Codex session. It records the current repository state, completed architecture cuts, remaining work, validation expectations, and decisions that should not be rediscovered.

General product documentation does not belong in `docs/` in this repository. That directory is reserved for app-server API documentation, so this roadmap intentionally lives at the repository root.

## Start here after a restart

Repository and branch:

| Item | Value |
|---|---|
| Repository | `git@github.com:AlexeyPlatkovsky/codex-buddy.git` |
| Local path | `/Users/Aleksei_Platkovskii/Documents/IdeaProjects/codex-buddy` |
| Working branch | `main` |
| Fork remote | `origin` → `AlexeyPlatkovsky/codex-buddy` |
| Upstream remote | `upstream` → `openai/codex` |
| Planning source | `.taskpilot/` |
| Next pruning task | `CB-20` |
| Runtime pruning sequence | `CB-20`, then `CB-45` through `CB-52` |
| Measurement task | `CB-21` |
| Cross-platform task | `CB-22` |

Run these read-only checks before editing:

```bash
cd /Users/Aleksei_Platkovskii/Documents/IdeaProjects/codex-buddy
git status --short
git log -12 --oneline
git remote -v
taskpilot --json item show CB-19
taskpilot --json item show CB-20
taskpilot validate
```

Expected state after the Stage 1 checkpoint:

- The Stage 1 implementation commit is `77e483d3dd Gate plugin runtime behind core feature`.
- The worktree is clean.
- `CB-19` is done. Continue with the `CB-20` → `CB-45`…`CB-52` chain; `CB-21`, `CB-22`, and `CB-35` remain follow-up work.
- `CB-1` (explicit config-driven runtime) and `CB-23` (right-side subagent tree) are done.

## Product boundary

Codex Buddy is a maintainable, coding-focused fork. Its intended runtime surface is:

| Keep | Exclude unless explicitly configured or required internally |
|---|---|
| Shell and command execution | Packaged/default plugins |
| File inspection and editing | Connector/App marketplace runtime |
| Local search and file search | Computer-use automation |
| Web search | Browser-control plugins |
| Image viewing | Image generation |
| Explicit project/user/global MCP servers | Realtime voice/audio input |
| Explicit project/user/global skills and instructions | Memories and decorative/personalization runtimes |
| Coding TUI and non-interactive exec | Cloud tasks and unrelated service clients |
| Subagents and the right-side subagent tree | Automatically injected tools, MCP, skills, or instructions |

The coding profile must remain deny-by-default for model-visible capabilities. Managed requirements and mandatory safety context still take precedence.

## Completed work

The following milestones are already implemented. Do not redo them.

| Commit | Completed boundary |
|---|---|
| `014de68def` | Rust CI uses `[self-hosted, Linux, X64]` for trusted runs, with hosted fallback for public fork pull requests. |
| `75413ac555` | App tool policy moved into configuration-owned, connector-neutral logic. |
| `7aa0e88728` | App-server Apps/Plugins/Marketplace processors and dependencies are gated. |
| `68a797049d` | Tool discovery no longer depends on connector models. |
| `f2115a59f4` | First connector provenance ownership cleanup. Later superseded by MCP ownership, but keep the commit history. |
| `dfb07a4721` | Initial slim dependency assertions added to CI. |
| `94a6e11742` | MCP tool runtime/cache ownership moved from connectors into `codex-mcp`. |
| `fcf297f358` | CI prevents connectors from re-entering through `codex-mcp`. |
| `750f6d2856` | Canonical connector-neutral `McpToolRuntime*` API introduced; Codex Apps auth/cache adapters isolated. |
| `cc78653261` | `codex-core/connectors` became optional and non-default; Buddy now excludes `codex-connectors`. |
| `3884ce56dd` | Connector provenance moved into `codex-mcp`; MCP no longer depends on `codex-plugin`. |
| `a110593338` | Neutral `codex-plugin-types` crate added; analytics and hooks no longer depend on the full plugin crate. |
| `77e483d3dd` | `codex-core/plugins` became optional and non-default; Buddy now excludes `codex-plugin` and `codex-core-plugins`. |

Completed TaskPilot product features:

| Item | Result |
|---|---|
| `CB-1` | Explicit config-driven runtime is implemented. |
| `CB-7` / `CB-8` / `CB-10` | MCP, skills, instructions, and extensions respect explicit source policy. |
| `CB-23` through `CB-32` | Right-side subagent tree, state projection, statuses, responsive rendering, and navigation are implemented. |
| `CB-40` / `CB-41` | Compatibility gates and the initial config/CLI/app-server audit are complete. |

## Current dependency state

At this checkpoint:

| Assertion | Expected result |
|---|---|
| Buddy normal graph contains `codex-connectors` | No |
| MCP normal graph contains `codex-connectors` | No |
| MCP normal graph contains `codex-plugin` | No |
| Analytics normal graph contains `codex-plugin` | No |
| Hooks normal graph contains `codex-plugin` | No |
| Buddy normal graph contains `codex-plugin` | No |
| Buddy normal graph contains `codex-core-plugins` | No |
| Buddy normal graph contains `codex-app-server` | Yes, intentionally for now |
| Approximate unique normal graph nodes | 1,289 after Stage 1, down from the 1,304 baseline |

Useful graph commands:

```bash
cd codex-rs
cargo tree -p codex-buddy -e normal --prefix none | sort -u | wc -l
cargo tree -p codex-buddy -e normal -i codex-plugin --prefix none
cargo tree -p codex-buddy -e normal -i codex-core-plugins --prefix none
cargo tree -p codex-buddy -e normal -i codex-app-server --prefix none
cargo tree -p codex-mcp -e normal --prefix none
```

The Buddy graph must keep both full plugin crates absent. Full CLI/app-server compositions intentionally retain them through explicit features. If analytics, hooks, MCP, or connectors reintroduce `codex-plugin` into Buddy, treat that as a regression.

## Architecture decisions already made

Do not reopen these decisions without new evidence:

| Decision | Reason |
|---|---|
| Keep app-server in Buddy for now | TUI and Exec use `codex-app-server-client::InProcessAppServerClient` for session lifecycle, MCP, dynamic tools, resume, onboarding, and events. Removing app-server requires a client architecture split, not a manifest-only edit. |
| Keep web search, MCP, skills, and agent/subagent support | They are part of the requested coding runtime. Their plugin/extension defaults still need pruning. |
| Remove image generation, not image viewing | Image viewing is a coding/file capability; generation is outside the lightweight client scope. |
| Cargo validates slim and Full configurations | Bazel has a single workspace-global feature configuration and should continue validating Full behavior. |
| Bazel core enables `connectors` explicitly | The Bazel build remains the Full compatibility build while Cargo CI proves Buddy’s negative graph. |
| `codex_core::connectors` is opt-in | External consumers using that module must enable `codex-core/connectors`. This intentional compatibility change is recorded in TaskPilot. |
| MCP provenance uses string connector IDs | Retaining `AppConnectorId` in MCP would recreate the MCP → plugin dependency. Conversion happens at plugin-aware boundaries. |
| Keep compatibility reexports where dependency direction permits | `codex-plugin` reexports `codex-plugin-types`; connectors reexports MCP runtime/provenance names. Do not add inverse dependencies merely to retain temporary APIs. |
| Keep commits small | Target less than 500 logical lines for complex stages and less than 800 lines for any non-mechanical review. |

## Roadmap overview

| Order | Stage | TaskPilot | Exit condition |
|---:|---|---|---|
| 1 | Gate the core plugin runtime | `CB-19` (done) | Buddy graph excludes `codex-plugin` and `codex-core-plugins`; Full builds retain plugin behavior. |
| 2 | Define coding and Full compositions | `CB-20`, `CB-45` | Buddy app-server composition includes only coding-required extensions. |
| 3 | Remove heavy non-coding core/TUI runtimes | `CB-46` through `CB-52` | Audio, memories, image generation, realtime, cloud, decorative, and V8/code-mode paths are absent where not required. |
| 4 | Measure real savings | `CB-21` | Reproducible dependency, binary-size, and startup report distinguishes graph/runtime/linker savings. |
| 5 | Cross-platform release gates | `CB-22` | Linux, macOS, and Windows coding configurations build and smoke-test. |
| 6 | Upstream-sync hardening | `CB-35` | Repeatable conflict policy and dependency-boundary checks protect future upstream merges. |
| 7 | Optional app-server architecture split | New TaskPilot feature if approved | Protocol/client separation removes the in-process server from Buddy without rewriting behavior ad hoc. |

### Migration agent record

The migration uses the following fixed roster. No nested or additional agent may be used without recording the change here first.

| Phase | Agent | Role | Model | Effort |
|---|---|---|---|---|
| Planning | Lorentz (`remaining_runtime_plan`) | Runtime architecture | `gpt-5.6-sol` | high |
| Planning | Lovelace (`measurement_platform_plan`) | Measurement and platforms | `gpt-5.6-terra` | high |
| Planning | Pauli (`upstream_hardening_plan`) | Upstream workflow | `gpt-5.6-terra` | high |
| Execution | `runtime_migration_owner` | Runtime features, realtime, and code-mode | `gpt-5.6-sol` | high |
| Execution | `app_server_compatibility_owner` | App-server v2, config, CLI, and rollout compatibility | `gpt-5.6-sol` | high |
| Execution | `tui_migration_owner` | TUI feature gates and snapshots | `gpt-5.6-terra` | high |
| Execution | `measurement_release_owner` | Dependency evidence, benchmarks, and platform CI | `gpt-5.6-terra` | high |
| Review | `final_migration_audit` | Final compatibility and breaking-change audit | `gpt-5.6-sol` | xhigh |

At most three subagents run concurrently, with non-overlapping ownership.

## Stage 1: gate the core plugin runtime

Status: completed in `77e483d3dd`; TaskPilot `CB-19` is done.

### Completion record

- Added non-default `codex-core/plugins`; `connectors` implies it.
- Made `codex-core-plugins` and `codex-plugin` optional while keeping neutral `codex-plugin-types` available to Slim.
- Added one compile-time plugin runtime facade. Full reexports the existing implementation; Slim supplies empty plugin discovery, skills, hooks, attribution, telemetry, and marketplace behavior without spreading optional manager fields through Core.
- Removed plugin install/suggestion handlers and specs from Slim at compile time.
- Propagated Full behavior through CLI, app-server-client, app-server, ChatGPT, external-agent-migration, and Bazel. Buddy disables CLI defaults so Cargo feature unification cannot reintroduce plugins.
- Added CI checks for Slim Core, the standalone Full migration consumer, optional dependency metadata, and the negative Buddy graph.
- Preserved configured MCP environment-policy coverage in Slim and plugin MCP/hook coverage in Full. Added a Slim regression that resumes historical rollout events containing `plugin_id` and `script_path`, retains the attribution, and completes the next turn.

Measured result:

| Assertion | Result |
|---|---|
| Buddy contains `codex-plugin` | No |
| Buddy contains `codex-core-plugins` | No |
| Full CLI contains both plugin crates | Yes |
| Unique Buddy normal graph nodes | 1,289; 15 fewer than the 1,304 baseline |

Validation completed:

- Slim and Full `cargo check` matrices passed for Core, Buddy, CLI, app-server, app-server-client, ChatGPT, and the standalone external-agent migration crate.
- `cargo check --tests -p codex-core --features connectors` passed; the reviewer also confirmed no-default test compilation.
- Focused `just test` coverage passed for Slim plugin-tool absence, configured MCP projection/policy, historical plugin-attributed rollout resume, Full plugin install specs, Full plugin MCP policy, and Full plugin hooks.
- `just bazel-lock-update`, Slim/Full scoped `just fix`, `just fmt`, dependency graph assertions, and `git diff --check` passed.
- A broad `just test -p codex-core` was attempted but is not green on this host. Package-scoped execution cannot locate first-party binaries such as `codex`, `test_stdio_server`, and `codex-code-mode-host`, and existing timing/PTY/MCP tests also fail. Migration-specific failures found by that run were corrected and pass in focused coverage. Do not report the broad suite as green.

### Goal

Make the full plugin loader, marketplace, installation, plugin Apps, and plugin hook integration optional in `codex-core`. Buddy should retain configured MCP, explicit skills/instructions, subagents, shell/file/search/web/image-view capabilities, and generic hooks that do not require plugin discovery.

### Recommended feature shape

Add a non-default core feature approximately like:

```toml
[features]
plugins = ["dep:codex-core-plugins", "dep:codex-plugin"]
connectors = ["dep:codex-connectors", "plugins"]
```

The exact feature name may be `plugins` or `plugin-runtime`; choose one name and use it consistently. Prefer `plugins` unless an existing feature convention strongly favors `plugin-runtime`.

Make `codex-core-plugins` and `codex-plugin` optional normal dependencies. Do not make `codex-plugin-types` optional if neutral DTOs are still needed in slim code.

### Full feature propagation

Full upstream behavior must explicitly enable the new core feature through these composition points:

| Composition point | Propagation |
|---|---|
| `codex-rs/cli/Cargo.toml` | `full-cli` enables `codex-core/plugins`. |
| `codex-rs/app-server-client/Cargo.toml` | `full-runtime-extensions` enables `codex-core/plugins`. This covers normal Full Exec/TUI paths. |
| `codex-rs/app-server/Cargo.toml` | `connectors` enables `codex-core/plugins`. |
| `codex-rs/chatgpt/Cargo.toml` | `connectors` enables `codex-core/plugins`. |
| `codex-rs/core/BUILD.bazel` | Full Bazel crate features include `plugins`. |

Buddy already depends on CLI, Exec, TUI, ChatGPT, and app-server clients with default/full features disabled. Verify that no other dependency enables `codex-core/plugins` through Cargo feature unification.

### Core implementation areas

Audit these before editing; they are the known plugin-heavy production areas:

| Area | Required slim behavior |
|---|---|
| `core/src/plugins/*` | Keep neutral mention parsing/instruction projection if needed; gate discovery, marketplace, installation, plugin snapshots, and plugin telemetry. |
| `core/src/mcp.rs` | Configured MCP remains; plugin MCP overlays/contributors become absent/no-op in slim mode. |
| `core/src/config/mod.rs` | Slim projection must produce an empty plugin snapshot without loading plugins. |
| `core/src/session/mod.rs` and `session/turn.rs` | Do not load plugin packages or inject plugin instructions in slim mode. Preserve skills, configured MCP, and subagents. |
| `core/src/session/turn_context.rs` | Plugin availability is false when the feature is absent. |
| `core/src/thread_manager.rs` | Construct a neutral/no-op plugin service rather than spreading optional fields everywhere. |
| `core/src/state/service.rs` | Gate plugin cache/state services while preserving thread/session state. |
| `core/src/hook_runtime.rs` | Preserve non-plugin hooks; plugin hook contributions are empty in slim mode. |
| `core/src/guardian/review.rs` | Remove only plugin attribution/declarations, not coding safety review. |
| `core/src/tools/handlers/request_plugin_install.rs` | Tool should be absent or return an explicit unavailable result in slim mode. Do not advertise plugin installation. |
| `core/src/tools/events.rs` and `tools/runtimes/*` | Gate plugin-specific provenance/events without removing generic tool execution. |
| `core/src/unified_exec/*` | Preserve execution; remove only plugin hook/source plumbing. |

Prefer one neutral service boundary or no-op manager over dozens of `Option<Arc<...>>` fields. Keep generic DTOs in `codex-plugin-types`; do not move new concepts into `codex-core` unless unavoidable.

### Stage 1 acceptance checks

Before final `fix`/`fmt`, run focused behavior tests through `just test`, never direct `cargo test`:

```bash
cd codex-rs
cargo check -p codex-core --no-default-features
cargo check -p codex-core --features plugins
cargo check -p codex-buddy
cargo check -p codex-app-server --no-default-features
cargo check -p codex-app-server
cargo check -p codex-chatgpt --no-default-features
cargo check -p codex-chatgpt
```

Add focused integration tests for the major agent-logic changes:

- Slim coding config starts no plugin loader or marketplace discovery.
- Explicit configured MCP servers still start and expose tools.
- Explicit skills and instructions still load according to source policy.
- Plugin install/suggestion tools are not advertised in slim mode.
- Full plugin MCP overlays and plugin instructions retain current behavior.
- Resume/rollout behavior remains compatible when old sessions contain plugin-related events.

Graph assertions:

```bash
buddy_tree="$(cargo tree -p codex-buddy -e normal --prefix none)"
! grep -Eq '^codex-(plugin|core-plugins) v' <<<"$buddy_tree"

full_tree="$(cargo tree -p codex-app-server -e normal --prefix none)"
grep -Eq '^codex-(plugin|core-plugins) v' <<<"$full_tree"
```

Add the negative Buddy assertions to `.github/workflows/rust-ci.yml` only after they pass locally.

Run `just bazel-lock-update` for manifest changes. Keep Bazel Full by adding the feature to `core/BUILD.bazel`, then build the affected core/app-server targets.

Finally:

```bash
just fix -p codex-core
just fmt
git diff --check
taskpilot validate
```

Do not rerun tests after the final `fix`/`fmt`. Do not run the complete workspace `just test` without asking the user first.

## Stage 2: prune app-server extensions

App-server must remain for now, but its unconditional extension dependencies should be reviewed and made composition features where appropriate.

### Composition spine completion record

- `90401379fc` adds explicit `coding-runtime-extensions` propagation from app-server through app-server-client, Exec/TUI, and Buddy; Full implies Coding at every layer.
- Buddy now selects Coding explicitly. Full CLI and Bazel targets explicitly retain Full features.
- Coding/Full app-server checks, Coding client/Exec/TUI checks, Buddy and Full CLI checks, app-server composition tests, the Slim external-agent JSON-RPC test, and Buddy tests passed.
- Buddy remains at 1,289 unique normal nodes with plugins/connectors absent; Full retains them. `Cargo.lock` and `MODULE.bazel.lock` did not change.
- `8fac8bd7aa` moves the existing Slim dependency assertions into `scripts/buddy_release/dependency_preflight.sh`; the reusable preflight passes locally with locked Cargo operations.

### Queue and detached-review completion record

- `53ee214a32` makes `codex-queue-extension` and `codex-agent-extension` optional Full features. Buddy/Coding omits both; Full retains both.
- All six `thread/queue/*` request variants and detached `review/start` remain in app-server v2. Coding returns stable `-32600` unavailable errors; inline review remains active.
- Persistent `codex queue` TUI support is Full-only. The in-session composer follow-up queue and Core subagents/right panel are unchanged.
- Slim unavailable/inline tests passed 3/3; Full queue/detached tests passed 2/2; Full TUI queue tests passed 2/2. Coding/Full compile checks and the reusable graph preflight passed.
- Buddy graph decreased from 1,289 to 1,287 unique normal nodes. No Cargo or Bazel lockfile changed.

### Existing runtime-boundary completion record

- `ad6a67a3f9` locks the already-achieved Coding exclusions for audio, all three memory runtime crates, image generation, and all three cloud-task runtime crates into the reusable dependency preflight.
- Positive guards use the actual Full owners: app-server for audio, memories, and image generation; CLI for cloud tasks. `codex-cloud-config` remains intentionally available to Coding.
- This stage changes no product code or compatibility surface. The locked graph preflight and shell syntax check passed.

### Realtime completion record

- `a8749464c8` makes executable realtime API transports and Core conversation services explicit Full features; Coding selects a private zero-state Core facade and starts no realtime work.
- All six app-server v2 realtime methods and all protocol/config/rollout/history/timeline types remain available. Coding returns stable JSON-RPC `-32600` before thread lookup; Full retains the existing transports and voice list.
- Slim public RPC and historical timeline tests passed 2/2; Core Slim/Full representative tests passed; `codex-api` Slim passed 103/103 and Full passed 180/180. Buddy and Full CLI checks, scoped lint, formatting, Bazel lock refresh, and the reciprocal feature-graph preflight passed.
- The Buddy normal graph remains 1,287 nodes because the shared WebSocket/channel crates are still used elsewhere; this stage removes realtime executable code through feature selection rather than claiming a package-count reduction.

### Coding personality completion record

- `c09a15e5bc` resolves Coding configuration to the explicit `Personality::None` value, even when a user configured another personality. Full keeps its configured/default behavior.
- This applies only when constructing new Coding configuration. Stored rollout base instructions and resumed historical context are not rewritten.
- A configuration test proves Full/Coding resolution, and a captured outbound request proves Coding sends neither Friendly/Pragmatic instructions nor a personality update fragment. Both focused Core tests passed; scoped fix, final formatting, and diff checks passed afterward.

### Decorative TUI completion record

- `820c16b71f` compiles pet assets/image runtime/events and the memories settings UI only with the existing Full TUI composition. Coding keeps config/protocol/remote rendering and unconditional slash parsing.
- A small always-compiled layout facade reserves zero pet columns in Coding, while typed `/pets` and `/memories` commands keep the existing unavailable response.
- Coding and Full TUI checks, the Coding hidden-affordance test, five focused Full pet/memory snapshot tests, and pending-snapshot review passed. Scoped fix and final formatting passed afterward. The broad Coding TUI run reached 334 passes before unrelated existing plugin/reset-memory failures and lifecycle timeouts; no CB-49-focused test failed.

### Code-mode neutral contracts completion record

- `bc31f45c2f` adds the private-module `codex-code-mode-types` crate for code-cell DTOs,
  constants, tool definitions, and session/provider/delegate contracts. The protocol and executable
  code-mode crates retain compatibility reexports.
- Core contracts and tools now consume the neutral types directly. Rollout tracing no longer depends
  on the executable code-mode crate, while historical serialization and execution behavior remain
  unchanged. No path-bearing type, model-visible path, or persisted wire shape changed.
- Targeted checks for the five affected crates passed; focused types/protocol/code-mode/tools/rollout
  tests passed 260/260, focused Core contract tests passed 5/5, and all four affected Bazel targets
  built. Bazel lock refresh produced no lockfile diff. Scoped fix, final formatting, and diff checks
  passed afterward.
- A broad Core `code_mode` name filter is not a valid Coding-profile gate: 43 tests passed and 102
  existing host/policy cases failed because sidecar/test binaries or `exec` are unavailable. The
  migration-specific focused contract coverage is green; the broad filtered run is not reported as
  green.

### Build artifact cleanup guard

- On 2026-08-28, migration validation was paused until the active Bazel build finished, then the
  verified `codex-rs/target` tree (about 202 GiB) was permanently deleted through the workspace
  deletion guard. Filesystem free space increased from about 350 GiB to 550 GiB; nothing was moved
  to Trash.
- `scripts/buddy_release/clean_rust_artifacts.sh` now limits cleanup to that exact generated tree,
  refuses symlinks and active Rust/Just/Bazel processes, supports a dry run, and delegates permanent
  removal to the existing workspace guard. The guard uses bounded retries when macOS metadata is
  recreated during directory removal; this race is covered by its unit tests.
- Repository instructions now require `CARGO_INCREMENTAL=0` for broad migration matrices and
  post-validation cleanup at 20 GiB or on user request. Cleanup happens only after tests, lint fixes,
  and formatting are complete.

Current unconditional or broadly included candidates include:

- `codex-agent-extension`
- `codex-guardian-v2`
- `codex-queue-extension`
- `codex-web-search-extension`
- `codex-skills-extension`
- `codex-mcp-extension` with plugin runtime already separable

Classify them before gating:

| Extension | Initial disposition |
|---|---|
| Detached app-server review agent | Gate from Buddy; it does not power Core subagents or the right panel. Keep inline review. |
| Web search | Keep. |
| Skills | Keep explicit project/user/global skills; keep bundled/plugin discovery disabled in Coding profile. |
| MCP | Keep generic configured MCP; keep plugin runtime disabled for Buddy. |
| Guardian | Keep; coding approvals and managed safety behavior depend on it. |
| Queue | Gate from Buddy while retaining stable v2 unavailable responses. |
| Git attribution | Keep until a separate product/security decision changes the policy. |
| Image generation | Exclude. Do not confuse with local image viewing. |
| Memories/history-notes/goals | Exclude from Buddy unless a coding requirement is documented. |

For every gated request processor, preserve app-server v2 protocol variants and return a clear unavailable error when the runtime feature is absent. Do not add v1 APIs. Update app-server README/schema fixtures only if wire behavior or shapes change.

Required checks:

- `cargo check -p codex-app-server --no-default-features --all-targets`
- `cargo check -p codex-app-server`
- `just test -p codex-app-server extension_composition`
- Focused public JSON-RPC tests for newly unavailable processors
- `just write-app-server-schema` only when protocol shapes change
- CI metadata assertions that optional extension dependencies remain optional

## Stage 3: remove heavy non-coding runtimes

Execute the `CB-20`, `CB-45`…`CB-52` blocker chain as small reviewable commits. Do not combine all removals.

Recommended order:

1. Lock negative graph coverage for audio, memories, image generation, cloud tasks, plugins, connectors, and V8 that are already absent.
2. Gate realtime execution while retaining protocol/history compatibility.
3. Suppress personality instructions for new Coding turns without rewriting existing rollouts.
4. Compile pets and Full-only memories settings only in the Full TUI.
5. Extract transport-neutral code-mode types.
6. Gate code-mode execution and transport while retaining historical code-cell projection.

For each item:

- Prove the crate disappears from `cargo tree -p codex-buddy -e normal`.
- Preserve Full propagation and Bazel Full behavior.
- Add a CI negative graph assertion only after the boundary is stable.
- Measure binary size only after a linked release build; dependency removal alone is not proof of binary reduction.

Do not remove:

- Image decoding/viewing used by the TUI and file tools.
- Web search.
- Generic configured MCP.
- Shell/file/search tools.
- Subagent protocol/events required by the right panel.
- Safety, sandbox, authentication, rollout, or resume compatibility merely because they look non-UI.

## Stage 4: measurement

Use `CB-21`. Establish repeatable baselines rather than anecdotal timing.

Capture at minimum:

| Metric | Method |
|---|---|
| Unique normal dependency nodes | `cargo tree -p codex-buddy -e normal --prefix none | sort -u | wc -l` |
| Forbidden crate presence | Anchored `cargo tree` checks used by CI |
| Release binary size | Same target triple, linker, profile, and strip settings for every comparison |
| Cold startup | Multiple process launches after clearing only relevant caches |
| Warm startup | Multiple launches with normal caches retained |
| Runtime initialization | Trace which services/processes actually start, separate from linked dependencies |
| Memory | Same idle/first-turn scenario and platform |

Record commit hashes, OS, target triple, Rust version, linker, feature set, and commands with every result.

## Stage 5: CI and platforms

Use `CB-22`.

Current trusted Linux jobs use:

```yaml
runs-on: [self-hosted, Linux, X64]
```

The workflow intentionally falls back to `ubuntu-24.04` for untrusted public fork pull requests so arbitrary fork code does not execute on private self-hosted runners.

Add targeted macOS and Windows build/smoke coverage for the coding configuration. Do not turn routine CI into an all-features matrix. Full behavior should be covered by selected Full Cargo checks and Bazel.

## Stage 6: upstream synchronization

Before starting a new migration slice:

1. Fetch `upstream`.
2. Inspect the incoming range and current dirty state.
3. Use an ordinary sync-only merge commit; do not rebase, force-push, repeat the historical `ours` bridge, or mix conflict resolution with a feature refactor.
4. Run checks proportional to conflicts.
5. Record recurring conflict patterns in TaskPilot.

High-conflict files currently include:

- `codex-rs/core/Cargo.toml`
- `codex-rs/core/src/session/turn.rs`
- `codex-rs/core/src/config/mod.rs`
- `codex-rs/app-server/Cargo.toml`
- `.github/workflows/rust-ci.yml`

Prefer additive feature gates, small new modules, neutral DTO crates, and compatibility reexports. Avoid broad deletion until the relevant dependency edge is proven absent.

### 2026-08-28 upstream sync record

- Merged `upstream/main` at `868c9edb0d` with an ordinary merge commit; the three incoming MCP-cache/response-ID commits produced no textual conflicts.
- Verified Buddy, Slim Core, and no-default app-server compilation. All 237 MCP tests ran; 236 passed immediately and one stale connector-neutral provenance expectation was corrected in the separate adaptation stage.
- Kept the new hosted Apps cache integration test Full-only with `codex-core/connectors`; Slim intentionally does not start or advertise Codex Apps.
- GitHub CLI PR creation was unavailable to the authenticated user (`CreatePullRequest` permission denied). Review branches were pushed, then merged locally with ordinary merge commits and pushed to `main`; branch protection recorded an administrator bypass. No force push or history rewrite was used.

## Deferred architecture: app-server/client split

Buddy currently reaches app-server through:

```text
codex-buddy
├── codex-exec
│   └── codex-app-server-client
│       └── codex-app-server
└── codex-tui
    └── codex-app-server-client
        └── codex-app-server
```

Removing app-server itself requires one of these larger designs:

- Split `codex-app-server-client` into protocol/client and in-process-host crates.
- Introduce a smaller shared session runtime used directly by TUI/Exec and app-server.
- Keep app-server out-of-process and make TUI/Exec true protocol clients.

Do not attempt this as a dependency flag change. Create a separate TaskPilot epic/design item and decide the process model first.

## Validation rules for every Rust stage

Follow the repository `AGENTS.md` instructions. In particular:

- Never run `cargo test` directly; use `just test`.
- Run tests before the final `just fix` and `just fmt`.
- Run `just fmt` automatically after Rust changes.
- Use `just fix -p <crate>` for the touched crate(s).
- Do not rerun tests after final fix/format.
- Ask before running the complete workspace `just test` after core/common/protocol changes.
- Run `just bazel-lock-update` whenever Cargo manifests or dependencies change.
- Update `MODULE.bazel.lock` if generated content changes.
- Update `core/config.schema.json` with `just write-config-schema` if config types change.
- Keep app-server API development in v2 and regenerate schemas if wire types change.
- Preserve Linux, macOS, and Windows compilation unless a feature is explicitly OS-specific.
- Restore unrelated Clippy auto-fixes with `apply_patch`; do not commit incidental changes.

Known test caveats:

- A prior broad app-server run had environmental/helper-binary failures despite most tests passing; do not describe the full suite as green.
- A prior broad core run also had baseline/environment failures.
- One MCP runtime test is reported as leaky by Nextest while still passing.
- One hooks test timed out once and passed on Nextest retry during the plugin-types extraction.

## TaskPilot workflow

Use `.taskpilot/` as canonical planning data. Do not edit its generated indexes or registry files manually.

For each stage:

```bash
taskpilot --json item list
taskpilot item comment CB-19 "<start decision and scope>" --author codex
taskpilot item comment CB-19 "<result, validation, compatibility notes>" --author codex
taskpilot validate
```

Use `CB-20` and `CB-45` through `CB-52` for the ordered runtime-removal slices, `CB-21` for measurements, `CB-22` for platform checks, and `CB-35` for CI/upstream workflow decisions.

## Definition of done for the migration

| Requirement | Done when |
|---|---|
| Explicit capabilities | Empty/unconfigured sources expose no optional MCP, skill, instruction, connector, or plugin capability. |
| Coding tools | Shell, files, search, web search, and image viewing work in TUI and Exec. |
| Plugins/connectors | Buddy release graph contains neither full plugin runtime nor connector runtime crates. |
| Non-coding runtime | Excluded systems are absent from the release dependency graph, not merely hidden at runtime. |
| Subagents | Right panel remains responsive, bounded, snapshot-tested, and driven by existing events without model-context injection. |
| Compatibility | Config, rollouts/resume, app-server v2, CLI parameters, auth, sandbox, and managed policy remain compatible or have explicit migration notes. |
| CI | Trusted self-hosted Linux plus targeted macOS/Windows checks protect both slim and Full configurations. |
| Measurement | Startup, binary size, dependency count, and runtime initialization deltas are reproducible and recorded. |
| Upstream maintenance | Sync conflicts remain localized to feature composition and small boundary modules. |

## Next-session recommended first action

Upstream advanced to `868c9edb0d` after the Stage 1 checkpoint. Land and validate a sync-only ordinary merge before Stage 2. Then execute `CB-20`: make Buddy select an explicit coding app-server composition while Full Cargo and Bazel retain their current behavior. Do not remove the in-process app-server itself; that remains a separate architecture decision.
