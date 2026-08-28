#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
delete_guard="${repo_root}/.codex/hooks/permanent_delete.py"
origin_ref="origin/main"
upstream_ref="upstream/main"
fetch_refs=false

usage() {
  cat >&2 <<'EOF'
usage: upstream_sync_preflight.sh [--fetch]

Rehearses an ordinary upstream/main merge into origin/main without changing the
index or worktree. --fetch refreshes only those two remote refs first.
EOF
  exit 2
}

if [[ "${1:-}" == "--fetch" ]]; then
  fetch_refs=true
  shift
fi
if [[ "$#" -ne 0 ]]; then
  usage
fi

if [[ "$(git -C "${repo_root}" rev-parse --show-toplevel)" != "${repo_root}" ]]; then
  echo "refusing preflight: script is not running in its repository root" >&2
  exit 2
fi
if [[ ! -f "${delete_guard}" ]]; then
  echo "refusing preflight: permanent deletion guard is missing" >&2
  exit 2
fi

normalize_github_remote() {
  local remote_url="$1"
  case "${remote_url}" in
    git@github.com:*) remote_url="${remote_url#git@github.com:}" ;;
    ssh://git@github.com/*) remote_url="${remote_url#ssh://git@github.com/}" ;;
    https://github.com/*) remote_url="${remote_url#https://github.com/}" ;;
    *) printf '%s\n' "${remote_url}"; return ;;
  esac
  printf '%s\n' "${remote_url%.git}"
}

origin_url="$(git -C "${repo_root}" remote get-url origin 2>/dev/null || true)"
upstream_url="$(git -C "${repo_root}" remote get-url upstream 2>/dev/null || true)"
if [[ -z "${origin_url}" || -z "${upstream_url}" ]]; then
  echo "refusing preflight: both origin and upstream remotes are required" >&2
  exit 2
fi
if [[ "$(normalize_github_remote "${upstream_url}")" != "openai/codex" ]]; then
  echo "refusing preflight: upstream must resolve to github.com/openai/codex" >&2
  exit 2
fi
if [[ "$(normalize_github_remote "${origin_url}")" == "openai/codex" ]]; then
  echo "refusing preflight: origin must be the fork, not openai/codex" >&2
  exit 2
fi

if [[ "${fetch_refs}" == true ]]; then
  git -C "${repo_root}" fetch --prune origin main
  git -C "${repo_root}" fetch --prune upstream main
fi

for ref in "${origin_ref}" "${upstream_ref}"; do
  if ! git -C "${repo_root}" rev-parse --verify --quiet "${ref}^{commit}" >/dev/null; then
    echo "refusing preflight: missing commit ref ${ref}" >&2
    exit 2
  fi
done

current_branch="$(git -C "${repo_root}" branch --show-current)"
head_commit="$(git -C "${repo_root}" rev-parse HEAD)"
origin_commit="$(git -C "${repo_root}" rev-parse "${origin_ref}")"
upstream_commit="$(git -C "${repo_root}" rev-parse "${upstream_ref}")"
merge_base="$(git -C "${repo_root}" merge-base "${origin_ref}" "${upstream_ref}")"
dirty_paths="$(git -C "${repo_root}" status --porcelain --untracked-files=normal)"
read -r fork_only upstream_only < <(
  git -C "${repo_root}" rev-list --left-right --count "${origin_ref}...${upstream_ref}"
)

fork_paths="$(git -C "${repo_root}" diff --name-only "${merge_base}..${origin_ref}" | LC_ALL=C sort -u)"
upstream_paths="$(git -C "${repo_root}" diff --name-only "${merge_base}..${upstream_ref}" | LC_ALL=C sort -u)"
overlap_paths="$(comm -12 <(printf '%s\n' "${fork_paths}") <(printf '%s\n' "${upstream_paths}"))"
overlap_count="$(printf '%s\n' "${overlap_paths}" | sed '/^$/d' | wc -l | tr -d ' ')"

rehearsal_output="$(mktemp "${repo_root}/.buddy_upstream_preflight.XXXXXX")"
cleanup() {
  if [[ -e "${rehearsal_output}" ]]; then
    python3 "${delete_guard}" --delete -- "${rehearsal_output}" >/dev/null
  fi
}
trap cleanup EXIT

set +e
git -C "${repo_root}" merge-tree --write-tree --messages \
  "${origin_ref}" "${upstream_ref}" >"${rehearsal_output}" 2>&1
rehearsal_status=$?
set -e
if [[ "${rehearsal_status}" -gt 1 ]]; then
  echo "upstream merge rehearsal failed unexpectedly:" >&2
  sed -n '1,80p' "${rehearsal_output}" >&2
  exit 2
fi

conflict_count="$(grep -c '^CONFLICT' "${rehearsal_output}" || true)"
ready=true

printf 'upstream sync preflight\n'
printf '  branch: %s\n' "${current_branch:-DETACHED}"
printf '  origin/main: %s\n' "${origin_commit}"
printf '  upstream/main: %s\n' "${upstream_commit}"
printf '  merge base: %s\n' "${merge_base}"
printf '  fork-only commits: %s\n' "${fork_only}"
printf '  upstream-only commits: %s\n' "${upstream_only}"
printf '  overlapping changed paths: %s\n' "${overlap_count}"
printf '  rehearsal conflicts: %s\n' "${conflict_count}"

if [[ -n "${overlap_paths}" ]]; then
  printf '  first overlapping paths:\n'
  printf '%s\n' "${overlap_paths}" | sed -n '1,25p' | sed 's/^/    /'
fi
if [[ "${rehearsal_status}" -eq 1 ]]; then
  printf '  first conflicts:\n'
  grep '^CONFLICT' "${rehearsal_output}" | sed -n '1,25p' | sed 's/^/    /'
  ready=false
fi

if [[ "${current_branch}" != "main" ]]; then
  echo "  blocker: switch to main before synchronization"
  ready=false
fi
if [[ -n "${dirty_paths}" ]]; then
  echo "  blocker: worktree contains tracked or untracked changes"
  ready=false
fi
if [[ "${head_commit}" != "${origin_commit}" ]]; then
  echo "  blocker: local HEAD does not match origin/main"
  ready=false
fi

if [[ "${ready}" != true ]]; then
  echo "result: not ready"
  exit 1
fi

echo "result: ready for an ordinary sync-only merge"
