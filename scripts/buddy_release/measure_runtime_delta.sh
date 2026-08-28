#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
delete_guard="${repo_root}/.codex/hooks/permanent_delete.py"
probe="${repo_root}/scripts/buddy_release/runtime_measurement_probe.py"
baseline_revision="021111061d"
current_revision="326747461d"
target_triple=""
measurement_root=""
cargo_version="deferred until the active-build guard passes"

usage() {
  echo "usage: $0 [--baseline REV] [--current REV] [--target TRIPLE] [--dry-run]" >&2
  exit 2
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --baseline)
      baseline_revision="${2:-}"
      shift 2
      ;;
    --current)
      current_revision="${2:-}"
      shift 2
      ;;
    --target)
      target_triple="${2:-}"
      shift 2
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    *)
      usage
      ;;
  esac
done

dry_run="${dry_run:-false}"

if [[ ! -f "${repo_root}/codex-rs/Cargo.toml" || ! -f "${delete_guard}" || ! -f "${probe}" ]]; then
  echo "measurement harness prerequisites are incomplete" >&2
  exit 2
fi

if [[ -L "${repo_root}" || -L "${repo_root}/codex-rs" ]]; then
  echo "refusing measurement from a symlinked repository path" >&2
  exit 2
fi

for revision in "${baseline_revision}" "${current_revision}"; do
  git -C "${repo_root}" rev-parse --verify "${revision}^{commit}" >/dev/null
done

if [[ -z "${target_triple}" ]]; then
  target_triple="$(rustc -vV | awk '/^host: / { print $2 }')"
fi
if [[ -z "${target_triple}" || "${target_triple}" == *"/"* || "${target_triple}" == *".."* ]]; then
  echo "refusing invalid target triple: ${target_triple}" >&2
  exit 2
fi

target_env_name="CARGO_TARGET_$(printf '%s' "${target_triple}" | tr '[:lower:]-' '[:upper:]_')_LINKER"
linker="${!target_env_name:-cargo-default}"

metadata_json() {
  jq -n \
    --arg baseline "$(git -C "${repo_root}" rev-parse "${baseline_revision}^{commit}")" \
    --arg current "$(git -C "${repo_root}" rev-parse "${current_revision}^{commit}")" \
    --arg os "$(uname -srm)" \
    --arg target "${target_triple}" \
    --arg rustc "$(rustc -Vv)" \
    --arg cargo "${cargo_version}" \
    --arg linker "${linker}" \
    --arg linker_env "${target_env_name}" \
    '{baseline: $baseline, current: $current, host: $os, target_triple: $target, rustc: $rustc, cargo: $cargo, linker: $linker, linker_environment: $linker_env, cargo_build: ["build", "--locked", "--release", "-p", "codex-buddy", "--target", $target], cargo_incremental: "0", post_build_strip: "strip -S -x on a temporary copy when supported"}'
}

active_builds() {
  local process_name process_ids
  for process_name in cargo cargo-nextest rustc just bazel bazelisk; do
    process_ids="$(pgrep -x "${process_name}" || true)"
    if [[ -n "${process_ids}" ]]; then
      printf '%s: %s\n' "${process_name}" "${process_ids}"
    fi
  done
}

require_no_active_builds() {
  local builds
  if builds="$(active_builds)"; [[ -n "${builds}" ]]; then
    echo "refusing measurement while Rust/Bazel processes are active:" >&2
    echo "${builds}" >&2
    exit 1
  fi
}

cleanup() {
  local status="$?"
  trap - EXIT INT TERM
  if [[ -n "${measurement_root}" && -d "${measurement_root}" && ! -L "${measurement_root}" ]]; then
    if [[ "${measurement_root}" != "${repo_root}"/.buddy_runtime_measurement.* ]]; then
      echo "refusing cleanup outside the measurement root: ${measurement_root}" >&2
      status=1
    elif ! python3 "${delete_guard}" --delete -- "${measurement_root}" >&2; then
      echo "failed to permanently clean measurement artifacts: ${measurement_root}" >&2
      status=1
    else
      git -C "${repo_root}" worktree prune
    fi
  fi
  exit "${status}"
}

if [[ "${dry_run}" == true ]]; then
  metadata_json | jq '. + {mode: "dry-run", note: "No Cargo command, worktree, target directory, or process probe was started."}'
  exit 0
fi

require_no_active_builds
cargo_version="$(cargo -V)"

measurement_root="$(mktemp -d "${repo_root}/.buddy_runtime_measurement.XXXXXX")"
trap cleanup EXIT INT TERM

if [[ ! -d "${measurement_root}" || -L "${measurement_root}" ]]; then
  echo "refusing unsafe measurement root: ${measurement_root}" >&2
  exit 2
fi

graph_forbidden_json() {
  local worktree="$1"
  local graph_file="$2"
  local forbidden_file="$3"
  local dependency
  local -a forbidden=(
    codex-connectors codex-plugin codex-core-plugins codex-agent-extension codex-queue-extension
    codex-code-mode codex-code-mode-host codex-code-mode-protocol codex-code-mode-runtime v8
    codex-utils-audio codex-memories-extension codex-memories-read codex-memories-write
    codex-image-generation-extension codex-cloud-tasks codex-cloud-tasks-client
    codex-cloud-tasks-mock-client
  )

  (
    cd "${worktree}/codex-rs"
    cargo tree --locked -p codex-buddy -e normal --prefix none --format '{p}' \
      | sed 's/ (\*)$//' | sort -u >"${graph_file}"
  )
  : >"${forbidden_file}"
  for dependency in "${forbidden[@]}"; do
    if grep -Eq "^${dependency} v" "${graph_file}"; then
      printf '%s\n' "${dependency}" >>"${forbidden_file}"
    fi
  done
  jq -n --rawfile graph "${graph_file}" --rawfile forbidden "${forbidden_file}" \
    '{unique_normal_nodes: ($graph | split("\n") | map(select(length > 0)) | length), forbidden_present: ($forbidden | split("\n") | map(select(length > 0)))}'
}

file_size_bytes() {
  if stat -f '%z' "$1" >/dev/null 2>&1; then
    stat -f '%z' "$1"
  else
    stat -c '%s' "$1"
  fi
}

measure_revision() {
  local label="$1"
  local revision="$2"
  local worktree="${measurement_root}/${label}-worktree"
  local target_dir="${measurement_root}/${label}-target"
  local graph_json="${measurement_root}/${label}-graph.json"
  local probe_json="${measurement_root}/${label}-probe.json"
  local binary binary_copy stripped_bytes strip_status graph

  git -C "${repo_root}" worktree add --detach "${worktree}" "${revision}" >/dev/null
  if [[ ! -d "${worktree}" || -L "${worktree}" || ! -d "${target_dir}" && -e "${target_dir}" ]]; then
    echo "refusing unsafe worktree or target directory for ${label}" >&2
    exit 2
  fi

  require_no_active_builds
  graph_forbidden_json "${worktree}" "${measurement_root}/${label}-graph.txt" "${measurement_root}/${label}-forbidden.txt" >"${graph_json}"
  require_no_active_builds
  (
    cd "${worktree}/codex-rs"
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="${target_dir}" \
      cargo build --locked --release -p codex-buddy --target "${target_triple}"
  )

  binary="${target_dir}/${target_triple}/release/codex-buddy"
  if [[ "${target_triple}" == *windows* ]]; then
    binary+=".exe"
  fi
  if [[ ! -f "${binary}" || -L "${binary}" ]]; then
    echo "missing regular release binary for ${label}: ${binary}" >&2
    exit 1
  fi

  binary_copy="${measurement_root}/${label}-stripped-copy"
  cp "${binary}" "${binary_copy}"
  if strip -S -x "${binary_copy}" >/dev/null 2>&1; then
    stripped_bytes="$(file_size_bytes "${binary_copy}")"
    strip_status="strip -S -x"
  else
    stripped_bytes=""
    strip_status="unavailable or unsupported on this host"
  fi
  python3 "${probe}" --binary "${binary}" >"${probe_json}"
  graph="$(cat "${graph_json}")"
  jq -n \
    --arg revision "$(git -C "${repo_root}" rev-parse "${revision}^{commit}")" \
    --arg binary "${binary}" \
    --arg strip_status "${strip_status}" \
    --argjson graph "${graph}" \
    --argjson probe "$(cat "${probe_json}")" \
    --argjson binary_bytes "$(file_size_bytes "${binary}")" \
    --arg stripped_bytes "${stripped_bytes}" \
    '{revision: $revision, graph: $graph, binary: {release_bytes: $binary_bytes, strip_command: $strip_status, stripped_bytes: (if $stripped_bytes == "" then null else ($stripped_bytes | tonumber) end)}, process_and_tui: $probe}'
}

baseline_json="$(measure_revision baseline "${baseline_revision}")"
current_json="$(measure_revision current "${current_revision}")"
metadata_json | jq --argjson baseline "${baseline_json}" --argjson current "${current_json}" \
  '. + {mode: "measurement", baseline_measurement: $baseline, current_measurement: $current}'
