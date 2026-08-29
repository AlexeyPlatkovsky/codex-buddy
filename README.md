# Codex Buddy ⚡

Codex Buddy is a lightweight, coding-focused fork of the Codex CLI. It keeps the interactive TUI, headless `exec`, review, resume/fork, authentication, sandboxing, apply-patch, and explicitly configured MCP workflows while removing or deferring non-coding runtime surface.

## Try it

Build and run locally:

```shell
cd codex-rs
cargo build --locked --release -p codex-buddy
./target/release/codex-buddy
```

The local macOS DMG is a CLI binary, not a `.app`: mount it, copy `codex-buddy` to a directory on your `PATH`, then run `codex-buddy`.

## What is different? 🧭

| Area | Standard Codex CLI | Codex Buddy |
| --- | --- | --- |
| Focus | Full product surface | Coding workflows first |
| Command | `codex` | `codex-buddy` |
| Code Mode | Available when supplied by the full build | Intentionally excluded; direct tools are used instead |
| Heavy surfaces | May include apps, plugins, browser/computer automation, cloud, voice, and generation features | Removed or not constructed in the coding runtime profile |
| MCP and skills | Full discovery/composition | Explicit configuration and project instructions only |

⚠️ Code Mode cannot be enabled by a configuration key in the compact Buddy binary: it is a compile-time dependency choice. A future full Buddy variant would need a separate build and the `codex-code-mode-host` helper.

## Evidence so far 🧪

All figures below are exploratory macOS arm64 measurements, not release gates or promises. The full ideal comparison still requires the dedicated Linux runner recorded in the roadmap.

| Measure | Standard / baseline | Buddy | Change |
| --- | ---: | ---: | ---: |
| Release binary | 987.3 MB | 967.5 MB | -2.0% |
| First verified TUI frame | 106.7 ms | 107.1 ms | no meaningful change |
| RSS at first TUI output | 23,356 KiB | 23,052 KiB | -1.3% |
| First request payload | 71,691 B | 62,560 B | -12.7% |
| First request with the opt-in minimal root prompt | 71,691 B | 42,417 B | -40.8% |
| Root-instruction portion with the minimal prompt | 21,209 B | 1,066 B | -95.0% |

The minimal prompt is an opt-in experiment, not the default release setting. It reduced the initial request by about 7,318 approximate tokens in this harness.

## Quality checks ✅

Real authenticated-model runs compare installed Codex CLI 0.151.0 with Buddy using deterministic local graders. Buddy passed all repeated, supported cases:

| Scenario, three macOS runs | Codex | Buddy |
| --- | ---: | ---: |
| Multi-step fix, changelog, and tests | 2/3 | 3/3 |
| Retry after a forced transient tool failure | 3/3 | 3/3 |
| Follow a project `AGENTS.md` → `SKILL.md` instruction | 3/3 | 3/3 |
| Nearest scoped `AGENTS.md` instruction | 3/3 | 3/3 |

⚠️ Actual subagent delegation and a real approval/sandbox denial are not yet proven by the local CLI benchmark; follow-up work captures the tool/approval events rather than inferring them from files.

## Versioning 📌

Buddy releases will use Semantic Versioning: patch for compatible fixes, minor for user-visible Buddy features, and major for incompatible changes. Upstream merges are assessed for user-visible impact before the next Buddy release version is chosen.

## References

- [Migration roadmap](MIGRATION_ROADMAP.md)
- [Performance and payload harness](scripts/buddy_release/compare_e2e_performance.py)
- [Prompt-quality benchmark](scripts/buddy_release/compare_prompt_quality.py)

This repository is licensed under the [Apache-2.0 License](LICENSE).
