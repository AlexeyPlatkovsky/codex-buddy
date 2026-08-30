#!/usr/bin/env sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(git -C "$script_dir/../.." rev-parse --show-toplevel)"
git -C "$repo_root" config --local core.hooksPath "$repo_root/.githooks"
printf 'Installed Codex Buddy Git hooks from %s/.githooks\n' "$repo_root"
