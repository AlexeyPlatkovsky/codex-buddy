# Codex Buddy migration workflow

Codex Buddy is maintained as a fork of upstream `main`. Keep the upstream history
available so changes can be synchronized and reviewed as ordinary merge commits.

## Branches and pull requests

- Keep `main` releasable. Start work from a clean, up-to-date local `main` using a
  short-lived branch such as `buddy/<area>-<change>`.
- Update from `origin/main` before opening a pull request. Use ordinary merges when
  synchronizing; do not rebase shared branches or force-push them.
- Resolve conflicts with this priority: security and managed-policy invariants,
  upstream compatibility, then Buddy-specific behavior. Record non-obvious choices
  in the pull request.
- Keep complex changes below 500 changed lines and all other changes below 800 when
  practical. Split independent migration stages into separate pull requests.
- A pull request must state its migration stage, affected compatibility surfaces,
  tests run, and any intentionally updated snapshots or generated files.

## Stage checks

Run the narrowest applicable checks locally, then let CI run the full matrix:

| Change | Required checks |
| --- | --- |
| Rust or configuration | `just fmt`, focused `just test -p <project>`, and `just fix -p <project>` for substantial Rust changes |
| `ConfigToml` or nested config types | `just write-config-schema`; include the schema diff |
| `Cargo.toml` or `Cargo.lock` | `just bazel-lock-update`; include `MODULE.bazel.lock` |
| TUI rendering or text output | focused TUI tests; inspect and accept only intended `insta` snapshots |
| app-server/protocol/resume behavior | focused integration tests plus legacy rollout/resume fixtures |
| release or dependency graph | feature-aware `cargo tree`, package builds, and size/startup measurements |

Do not rewrite protocol history or rollout records to make a test pass. Preserve
legacy deserializers and verify that the full runtime preset remains compatible.

## Synchronization checklist

Before merging a migration stage:

1. Confirm the worktree is clean except for the intended branch changes.
2. Merge the latest `origin/main` and rerun checks for every conflicted area.
3. Check that the Buddy build still uses only its permitted runtime capabilities.
4. Note platform, dependency, schema, Bazel-lock, or snapshot impact in the PR.
