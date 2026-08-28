#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}/codex-rs"

cargo check --locked -p codex-chatgpt --no-default-features
cargo check --locked -p codex-app-server --no-default-features
cargo check --locked -p codex-core --no-default-features
cargo check --locked -p codex-buddy
cargo check --locked -p codex-external-agent-migration

tools_tree="$(cargo tree --locked -p codex-tools -e normal --prefix none)"
if grep -Eq '^codex-connectors v' <<<"${tools_tree}"; then
  echo 'codex-tools unexpectedly depends on codex-connectors'
  echo "${tools_tree}"
  exit 1
fi

mcp_tree="$(cargo tree --locked -p codex-mcp -e normal --prefix none)"
if grep -Eq '^codex-connectors v' <<<"${mcp_tree}"; then
  echo 'codex-mcp unexpectedly depends on codex-connectors'
  echo "${mcp_tree}"
  exit 1
fi

if grep -Eq '^codex-plugin v' <<<"${mcp_tree}"; then
  echo 'codex-mcp unexpectedly depends on codex-plugin'
  echo "${mcp_tree}"
  exit 1
fi

for package in codex-analytics codex-hooks; do
  package_tree="$(cargo tree --locked -p "${package}" -e normal --prefix none)"
  if grep -Eq '^codex-plugin v' <<<"${package_tree}"; then
    echo "${package} unexpectedly depends on codex-plugin"
    echo "${package_tree}"
    exit 1
  fi
done

buddy_tree="$(cargo tree --locked -p codex-buddy -e normal --prefix none)"
if grep -Eq '^codex-connectors v' <<<"${buddy_tree}"; then
  echo 'codex-buddy unexpectedly depends on codex-connectors'
  echo "${buddy_tree}"
  exit 1
fi

for dependency in codex-core-plugins codex-plugin; do
  if grep -Eq "^${dependency} v" <<<"${buddy_tree}"; then
    echo "codex-buddy unexpectedly depends on ${dependency}"
    echo "${buddy_tree}"
    exit 1
  fi
done

for dependency in codex-agent-extension codex-queue-extension; do
  if grep -Eq "^${dependency} v" <<<"${buddy_tree}"; then
    echo "codex-buddy unexpectedly depends on ${dependency}"
    echo "${buddy_tree}"
    exit 1
  fi
done

for dependency in \
  codex-utils-audio \
  codex-memories-extension \
  codex-memories-read \
  codex-memories-write \
  codex-image-generation-extension \
  codex-cloud-tasks \
  codex-cloud-tasks-client \
  codex-cloud-tasks-mock-client; do
  if grep -Eq "^${dependency} v" <<<"${buddy_tree}"; then
    echo "codex-buddy unexpectedly depends on ${dependency}"
    echo "${buddy_tree}"
    exit 1
  fi
done

full_app_server_tree="$(cargo tree --locked -p codex-app-server -e normal --prefix none)"
for dependency in codex-agent-extension codex-queue-extension; do
  if ! grep -Eq "^${dependency} v" <<<"${full_app_server_tree}"; then
    echo "full codex-app-server must depend on ${dependency}"
    echo "${full_app_server_tree}"
    exit 1
  fi
done

for dependency in \
  codex-utils-audio \
  codex-memories-extension \
  codex-memories-read \
  codex-memories-write \
  codex-image-generation-extension; do
  if ! grep -Eq "^${dependency} v" <<<"${full_app_server_tree}"; then
    echo "full codex-app-server must depend on ${dependency}"
    echo "${full_app_server_tree}"
    exit 1
  fi
done

full_cli_tree="$(cargo tree --locked -p codex-cli -e normal --prefix none)"
for dependency in codex-cloud-tasks codex-cloud-tasks-client codex-cloud-tasks-mock-client; do
  if ! grep -Eq "^${dependency} v" <<<"${full_cli_tree}"; then
    echo "full codex-cli must depend on ${dependency}"
    echo "${full_cli_tree}"
    exit 1
  fi
done

workspace_metadata="$(cargo metadata --locked --no-deps --format-version=1)"
for dependency in codex-agent-extension codex-connectors codex-core-plugins codex-plugin codex-queue-extension; do
  if ! jq -e --arg dependency "${dependency}" '
    .packages[]
    | select(.name == "codex-app-server")
    | .dependencies[]
    | select(.name == $dependency and .kind == null)
    | .optional == true
  ' <<<"${workspace_metadata}" >/dev/null; then
    echo "codex-app-server dependency ${dependency} must be optional"
    exit 1
  fi
done

for dependency in codex-core-plugins codex-plugin; do
  if ! jq -e --arg dependency "${dependency}" '
    .packages[]
    | select(.name == "codex-core")
    | .dependencies[]
    | select(.name == $dependency and .kind == null)
    | .optional == true
  ' <<<"${workspace_metadata}" >/dev/null; then
    echo "codex-core dependency ${dependency} must be optional"
    exit 1
  fi
done

buddy_chatgpt_features="$(cargo tree --locked -p codex-buddy -e features -i codex-chatgpt)"
if [[ "${buddy_chatgpt_features}" == *'codex-chatgpt feature "connectors"'* ]]; then
  echo 'codex-buddy unexpectedly enables codex-chatgpt/connectors'
  echo "${buddy_chatgpt_features}"
  exit 1
fi

buddy_mcp_features="$(cargo tree --locked -p codex-buddy -e features -i codex-mcp-extension)"
if [[ "${buddy_mcp_features}" == *'codex-mcp-extension feature "plugin-runtime"'* ]]; then
  echo 'codex-buddy unexpectedly enables codex-mcp-extension/plugin-runtime'
  echo "${buddy_mcp_features}"
  exit 1
fi

for package in codex-app-server codex-core codex-api; do
  buddy_features="$(cargo tree --locked -p codex-buddy -e features -i "${package}")"
  if [[ "${buddy_features}" == *"${package} feature \"realtime\""* ]]; then
    echo "codex-buddy unexpectedly enables ${package}/realtime"
    echo "${buddy_features}"
    exit 1
  fi
done

for package in codex-app-server codex-core codex-api; do
  full_cli_features="$(cargo tree --locked -p codex-cli -e features -i "${package}")"
  if [[ "${full_cli_features}" != *"${package} feature \"realtime\""* ]]; then
    echo "full codex-cli must enable ${package}/realtime"
    echo "${full_cli_features}"
    exit 1
  fi
done
