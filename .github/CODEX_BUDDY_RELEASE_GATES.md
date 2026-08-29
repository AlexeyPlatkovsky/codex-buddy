# Codex Buddy release gates

These gates apply before publishing a Codex Buddy build. The upstream `codex`
targets remain buildable and are checked separately for compatibility.

## Required release matrix

| Gate | Requirement |
| --- | --- |
| Functional behavior | Coding-profile integration tests, app-server thread start/resume tests, and legacy rollout fixtures pass |
| Configuration | Generated config schema is current; managed denials cannot be widened by user or project config |
| Dependency graph | `cargo tree -p codex-buddy -e features` contains no excluded Buddy capabilities or release-only tools |
| Platforms | Release builds and smoke tests pass on macOS arm64, Linux x86_64, Linux arm64, and Windows x86_64 |
| TUI | 80-, 96-, and 120-column snapshots pass; resize, deep agent trees, Unicode truncation, and empty states are covered |
| Startup | Cold-start measurement is no worse than 5% over the approved baseline on the same runner |
| Size | Stripped Buddy binary size is measured reproducibly against an approved baseline, and the result is linked and accepted before release |
| Packaging | Archives, checksums, install metadata, and completion output are generated for every supported platform |

## Sign-off

The release PR must attach or link the measurements and CI runs used for each
gate. Any exception names the gate, measured value, reason, owner, and expiry.

Permanent migration-closure decisions approved by the product owner on
2026-08-29:

- Native Intel macOS is not a supported Buddy release platform. The required
  matrix is the four-platform set listed above. This decision has no expiry and
  remains in force until the product owner changes supported platforms.
- The size result for `021111061d` through `326747461d` is an accepted 1.978%
  stripped-binary reduction (203,310,872 B to 199,288,736 B). The comparison is
  reproducible but covers only the final runtime/code-mode slices because the
  baseline already contained earlier heavy pruning. The product owner accepts
  this result permanently for migration closure; it has no expiry. A future
  release still records its own reproducible size rather than assuming this
  historical value.

Schema, Bazel-lock, or snapshot changes are release artifacts: they must be
reviewed in the same PR and must not be regenerated silently during packaging.
Run the cross-platform matrix again after resolving an upstream merge that
touches app-server, core, protocol, configuration, build manifests, or release
scripts.

Do not expose excluded commands or capabilities through the Buddy package while
retaining their upstream targets for synchronization and full-runtime builds.

## Configuration compatibility notes

The top-level `runtime` key is reserved for the `[runtime]` table. Older local
configuration that used `runtime` as an ignored scalar must remove or migrate
that value before starting Codex Buddy. Strict parsing is intentional so an
invalid runtime policy cannot silently broaden the enabled capability set.
