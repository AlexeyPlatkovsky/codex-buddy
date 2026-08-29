# Codex Buddy migration history and performance follow-up

Last updated: 2026-08-29

This file is the compact restart handoff for the Codex Buddy fork. The completed migration is
historical record; open work belongs in TaskPilot. General product documentation does not belong
in `docs/`, so this repository-specific record remains at the root.

## Canonical state

| Item | Value |
|---|---|
| Repository | `git@github.com:AlexeyPlatkovsky/codex-buddy.git` |
| Fork / upstream | `origin/main` / `upstream/main` |
| Planning source | `.taskpilot/` |
| Closed migration | `CB-1` through `CB-52`; 52/52 items done |
| Closure commit | `26b8a1e90e38` (`Merge migration release closure`) |
| Optional performance follow-up | `CB-53` through `CB-63` |
| Release requirements | `.github/CODEX_BUDDY_RELEASE_GATES.md` |
| Upstream procedure | `.github/CODEX_BUDDY_MIGRATION.md` |

The migration closed on 2026-08-29. Coding is a deny-by-default runtime, the lightweight
distribution and subagent tree are complete, supported native lanes passed, compatibility evidence
is recorded, and the accepted final-slice stripped-binary reduction is 1.978%.

## Product boundary

| Retain in Coding | Exclude unless explicitly configured or internally required |
|---|---|
| Shell, file inspection/editing, and local search | Packaged/default plugins and marketplace runtime |
| Web search and local image viewing | Connector/Apps runtime and browser/computer automation |
| Explicit MCP, skills, and instructions | Image generation, realtime voice/audio, and memories |
| TUI, non-interactive Exec, auth, sandbox, review | Cloud tasks, decorative runtimes, and Code Mode/V8 |
| Core subagents and right-side agent tree | Queue and detached-review extensions |
| In-process app-server/client lifecycle | Automatically injected optional capabilities or context |

Managed requirements and mandatory safety context always take precedence. Historical protocol and
rollout items remain readable even when their executable runtime is absent.

## Decisions that must survive future upstream merges

| Decision | Historical reason |
|---|---|
| Keep app-server in Buddy | TUI and Exec share `InProcessAppServerClient` lifecycle, MCP, tools, resume, onboarding, and events. Removal requires a client/process redesign. |
| Keep generic MCP, skills, web search, Guardian, and Core subagents | They are coding capabilities; only implicit/plugin-owned sources were removed. |
| Keep image viewing but not generation | Viewing is a coding/file capability; generation is outside the product boundary. |
| Cargo proves Coding and Full; Bazel remains Full | Bazel has one workspace-global feature composition. |
| Keep compatibility DTOs and reexports where dependency direction permits | Coding must resume old rollouts without restoring heavy runtimes. |
| MCP provenance uses string connector IDs | Using plugin-owned IDs would recreate the MCP-to-plugin dependency. |
| `codex_core::connectors` is opt-in | External consumers that use it must explicitly enable `codex-core/connectors`. |
| Keep changes reviewable | Target under 500 logical lines for complex work and under 800 lines for non-mechanical changes. |

## Completed migration history

| Scope | TaskPilot | Result |
|---|---|---|
| Explicit runtime policy | `CB-1`–`CB-11` | Typed, source-aware capability policy; empty Coding config exposes only the coding baseline. |
| Lightweight client | `CB-12`–`CB-22` | Coding CLI/TUI composition, runtime pruning, measurement, and native release evidence completed. |
| Subagent tree | `CB-23`–`CB-33` | Bounded event projection, statuses, responsive rendering, navigation, and snapshots completed. |
| Maintainable fork | `CB-34`–`CB-44` | Ordinary upstream merges, compatibility gates, rollout/config/API preservation, and release checks completed. |
| Heavy-runtime closure | `CB-45`–`CB-52` | Queue, detached review, realtime, personality, decorative TUI, and Code Mode/V8 boundaries completed. |

Key dependency-boundary commits:

| Commit | Boundary |
|---|---|
| `75413ac555` | App-tool policy moved to connector-neutral configuration ownership. |
| `94a6e11742` / `750f6d2856` | MCP runtime/cache ownership and canonical `McpToolRuntime*` APIs moved out of connectors. |
| `cc78653261` / `3884ce56dd` | Connectors became opt-in and MCP stopped depending on the plugin crate. |
| `a110593338` / `77e483d3dd` | Neutral plugin types were extracted; full plugin runtimes became optional and non-default. |
| `90401379fc` / `53ee214a32` | Coding/Full composition spine added; queue and detached review became Full-only. |
| `a8749464c8` / `c09a15e5bc` | Realtime became Full-only; new Coding turns stopped injecting personality. |
| `820c16b71f` | Pet and Full-only memories UI implementation was removed from Coding builds. |
| `bc31f45c2f` / `61af7c0113` | Neutral Code Mode types were extracted; execution, transport, host, and V8 became Full-only. |
| `d031b8ba20` | Guarded, read-only upstream synchronization preflight completed. |

## Closed dependency and compatibility boundary

The Coding normal graph excludes connectors, full plugin runtimes, queue/detached-review
extensions, executable Code Mode/protocol/host/runtime/V8, audio, memory, image-generation,
cloud-task, and realtime execution roots. Full app-server/CLI compositions retain the required
positive roots. `scripts/buddy_release/dependency_preflight.sh` is the regression guard.

Compatibility retained:

- app-server v2 method and DTO shapes, with stable unavailable errors in Coding;
- CLI/config parsing needed to read shared configuration;
- managed policy, auth, approvals, sandbox, rollout projection, and resume;
- historical plugin, realtime, queue, memory, and code-cell items;
- inline review, configured MCP, explicit skills/instructions, web search, and Core subagents.

The top-level `runtime` config key is reserved for the `[runtime]` table. Older ignored scalar
values must be removed; strict parsing prevents an invalid policy from broadening capabilities.

## Closure evidence

Hosted run `33205340928` compared `021111061d` with `326747461d` using Linux x86_64,
Rust 1.95.0, the Cargo-default linker, `CARGO_INCREMENTAL=0`, and the same thin-LTO release
profile.

| Metric | Baseline | Current | Delta |
|---|---:|---:|---:|
| Unique normal dependency nodes | 1,331 | 1,330 | -1 |
| Unstripped release binary | 987,346,640 B | 967,450,136 B | -19,896,504 B (-2.015%) |
| Temporarily stripped binary | 203,310,872 B | 199,288,736 B | -4,022,136 B (-1.978%) |
| First parser-process launch | 7.676 ms | 7.649 ms | -0.027 ms |
| Five-run warm parser mean | 7.500 ms | 7.517 ms | +0.017 ms |
| First qualified 80x24 TUI frame | 106.701 ms | 107.094 ms | +0.393 ms |
| RSS at first TUI output | 23,356 KiB | 23,052 KiB | -304 KiB |

The timing differences are noise-scale. The product owner permanently accepted the reproducible
1.978% stripped reduction for migration closure. It covers only the final runtime/Code Mode slices:
the baseline already contained earlier CLI, plugin, connector, memory, and extension pruning.

Native run `33203764981` passed release build, host-target verification, `--version`, and
`exec --help` on macOS arm64, Linux x86_64, Linux arm64, and Windows x86_64. Native Intel macOS
is not a release requirement. A later aggregate Windows job stopped without a completed step or
log; the standalone Windows evidence remains authoritative.

Focused migration coverage is green. Broad Core and TUI runs encountered existing helper-binary,
PTY/timing, plugin/reset-memory, and lifecycle failures; do not describe those broad suites as
green. No focused migration test failed.

## Maintenance rules

- Start from clean `main == origin/main`; run
  `scripts/buddy_release/upstream_sync_preflight.sh --fetch` before a new sync.
- Use an ordinary sync-only merge. Never rebase shared history, force-push, or mix feature work
  with conflict resolution.
- Re-run the four-platform matrix after upstream changes to core, protocol, configuration,
  app-server, manifests, or release scripts.
- Use `scripts/buddy_release/clean_rust_artifacts.sh --confirm` only after build writers stop and
  required tests/fix/fmt finish.
- Keep release measurements reproducible; attach raw evidence, hashes, exact commits, and declared
  exceptions to the release PR.

## Deferred architecture

Removing the in-process app-server remains a separate product/architecture decision. Create a new
epic before choosing among a protocol/client versus host split, a smaller shared session runtime,
or a true out-of-process client. Do not attempt it through Cargo feature flags alone.

## End-to-end performance comparison plan

TaskPilot `CB-53` is an optional post-migration epic related to historical measurement `CB-21`.
It compares the pre-pruning baseline `92529d95fd` with a frozen current commit; it does not reopen
the migration.

### Planned work

| Order | Items | Deliverable | Blocked by |
|---:|---|---|---|
| 1 | `CB-54`, `CB-57`, `CB-58` | Scenario, metric/schema, isolation, and sampling contract | — |
| 2 | `CB-55`, `CB-59`, `CB-60` | Python build/lifecycle orchestration and paired statistics | Order 1 |
| 3 | `CB-61` | Unit, failure-path, cleanup, dry-run CI, and workflow integration tests | Order 2 |
| 4 | `CB-56`, `CB-62` | Ideal dedicated-runner execution | Order 3 |
| 5 | `CB-63` | Reviewable evidence and release decision | Order 4 |

### Repository entry point

```bash
python3 scripts/buddy_release/compare_e2e_performance.py \
  --baseline 92529d95fd \
  --current HEAD \
  --target x86_64-unknown-linux-gnu \
  --cold-pairs 10 \
  --warm-pairs 30 \
  --seed 20260829 \
  --enable-cold-cache-eviction \
  --require-ideal-host \
  --output "${RUNNER_TEMP}/buddy-e2e-performance.json"
```

`scripts/buddy_release/compare_e2e_performance.py` uses the Python standard library and the
existing permanent-delete guard. It supports `--dry-run`, resolves both revisions to full hashes,
refuses active Cargo/Rust/Bazel writers, validates free space and host controls, removes real
credentials and proxies, binds its deterministic Responses SSE server to loopback, and cleans the
isolated worktrees and targets. The ideal runner must enforce outbound-network denial separately;
the harness configures no external endpoint.

### Scenarios and timestamps

| Scenario | Required evidence |
|---|---|
| Parser startup | Spawn to successful `--version` exit; retained for continuity, not the primary E2E gate. |
| TUI startup | Spawn to first verified 80x24 frame, then stable pre-prompt RSS over a declared window. |
| Headless first turn | Spawn `exec`, submit fixed prompt, receive exactly one deterministic mock request/response, render the assistant result, persist the rollout, and exit successfully. |
| Interactive first turn | Spawn PTY, verify frame, submit fixed prompt, observe mock request, deterministic response completion, first assistant render, and clean shutdown. |

Use monotonic timestamps at process spawn, first frame, prompt submission, mock request arrival,
first SSE byte, first assistant output, turn completion, and process exit. Report total E2E time and
separate client-before-request, controlled server delay, and client-after-first-byte time. A live
model can be an observational canary, but never the deterministic release gate.

### Ideal execution controls

- Dedicated, otherwise idle Linux x86_64 runner; fixed CPU governor/power mode and recorded thermal,
  CPU, memory, kernel, filesystem, toolchain, target, linker, release profile, and environment.
- Build both commits once outside timed regions with `--locked --release`,
  `CARGO_INCREMENTAL=0`, isolated worktrees/targets, and identical build settings.
- Give every sample a fresh 0700 `CODEX_HOME` and work directory cloned from one bounded fixture.
- Run five unmeasured warmups, then at least 30 warm pairs in seeded balanced baseline/current
  order. Run at least 10 true-cold pairs only when privileged page-cache eviction is available;
  otherwise mark cold evidence unavailable.
- Abort or classify the run inconclusive when load/thermal limits, semantic equivalence, mock
  request counts, frame/turn success, timeout, cleanup, or host-control checks fail.
- Sample root-process RSS at a fixed interval and never infer process-tree or in-process service
  initialization from it. Add bounded opt-in structured trace events before making attribution
  claims.

### Statistics, output, and acceptance

The schema-versioned JSON must contain host/build metadata, full commit hashes, scenario fixture and
binary hashes, bounded stdout/stderr evidence, every raw sample, failures/timeouts, cleanup deltas,
and derived median, p90, p95, MAD, paired percentage deltas, and deterministic bootstrap 95%
confidence intervals.

Primary gates are successful headless and interactive first-turn completion. Current is
non-regressing only when the upper bound of the paired 95% confidence interval is at most +5%.
An improvement may be claimed only when the entire interval is below 0%. Otherwise report
`inconclusive` and rerun; never turn missing or noisy evidence into a pass.

Keep the historical 1.978% result unchanged. If the new full-range run is accepted, link its raw
artifact and decision from `.github/CODEX_BUDDY_RELEASE_GATES.md`, then complete `CB-63`.
