# Codex Buddy release gates

These gates apply before publishing a Codex Buddy build. The upstream `codex`
targets remain buildable and are checked separately for compatibility.

## Required release matrix

| Gate | Requirement |
| --- | --- |
| Functional behavior | Coding-profile integration tests, app-server thread start/resume tests, and legacy rollout fixtures pass |
| Configuration | Generated config schema is current; managed denials cannot be widened by user or project config |
| Dependency graph | `cargo tree -p codex-buddy -e features` contains no excluded Buddy capabilities or release-only tools |
| Platforms | Release builds and smoke tests pass on macOS arm64, macOS x86_64, Linux x86_64, Linux arm64, and Windows x86_64 |
| TUI | 80-, 96-, and 120-column snapshots pass; resize, deep agent trees, Unicode truncation, and empty states are covered |
| Startup | Cold-start measurement is no worse than 5% over the approved baseline on the same runner |
| Size | Stripped Buddy binary is at least 15% smaller than the approved full-runtime baseline, or the exception is documented and approved |
| Packaging | Archives, checksums, install metadata, and completion output are generated for every supported platform |

## Sign-off

The release PR must attach or link the measurements and CI runs used for each
gate. Any exception names the gate, measured value, reason, owner, and expiry.

Schema, Bazel-lock, or snapshot changes are release artifacts: they must be
reviewed in the same PR and must not be regenerated silently during packaging.
Run the cross-platform matrix again after resolving an upstream merge that
touches app-server, core, protocol, configuration, build manifests, or release
scripts.

Do not expose excluded commands or capabilities through the Buddy package while
retaining their upstream targets for synchronization and full-runtime builds.
