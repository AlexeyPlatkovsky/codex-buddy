#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
artifact_dir="${repo_root}/codex-rs/target"
delete_guard="${repo_root}/.codex/hooks/permanent_delete.py"

usage() {
  echo "usage: $0 (--dry-run | --confirm)" >&2
  exit 2
}

if [[ "$#" -ne 1 ]]; then
  usage
fi

mode="$1"
if [[ "${mode}" != "--dry-run" && "${mode}" != "--confirm" ]]; then
  usage
fi

if [[ ! -f "${repo_root}/codex-rs/Cargo.toml" || ! -f "${delete_guard}" ]]; then
  echo "refusing cleanup: repository or deletion guard is incomplete" >&2
  exit 2
fi

if [[ ! -e "${artifact_dir}" ]]; then
  echo "no Rust artifacts found at ${artifact_dir}"
  exit 0
fi

if [[ ! -d "${artifact_dir}" || -L "${artifact_dir}" ]]; then
  echo "refusing cleanup: ${artifact_dir} is not a real directory" >&2
  exit 2
fi

active_builds=""
for process_name in cargo cargo-nextest rustc just bazel bazelisk; do
  process_ids="$(pgrep -x "${process_name}" || true)"
  for process_id in ${process_ids}; do
    if [[ "${process_name}" == "cargo" ]]; then
      command_line="$(ps -p "${process_id}" -o command= 2>/dev/null || true)"
      case "${command_line}" in
        cargo\ tree\ *|*/cargo\ tree\ *)
          echo "allowing read-only Cargo tree process ${process_id} during artifact cleanup" >&2
          continue
          ;;
      esac
    fi
    active_builds="${active_builds}${process_name}: ${process_id}"$'\n'
  done
done

if [[ -n "${active_builds}" ]]; then
  echo "refusing cleanup while Rust/Bazel validation processes are active:" >&2
  echo "${active_builds}" >&2
  exit 1
fi

artifact_size="$(du -sh "${artifact_dir}" | awk '{print $1}')"
if [[ "${mode}" == "--dry-run" ]]; then
  echo "would permanently delete ${artifact_dir} (${artifact_size})"
  exit 0
fi

python3 "${delete_guard}" --delete -- "${artifact_dir}"
echo "removed ${artifact_size} of Rust build artifacts without using Trash"
