//! Codex Apps-specific adapters for the shared MCP tool runtime.

use std::path::Path;
use std::path::PathBuf;

use codex_login::CodexAuth;

use crate::McpToolRuntimeContextKey;
use crate::tool_runtime::mcp_tool_runtime_cache_path;

/// Builds the Codex Apps runtime key for the active auth identity.
pub fn codex_apps_tools_cache_key(auth: Option<&CodexAuth>) -> McpToolRuntimeContextKey {
    let account_id = auth.and_then(CodexAuth::get_account_id);
    let chatgpt_user_id = auth.and_then(CodexAuth::get_chatgpt_user_id);
    if auth.is_some_and(CodexAuth::is_workspace_account) {
        McpToolRuntimeContextKey::workspace(account_id, chatgpt_user_id)
    } else {
        McpToolRuntimeContextKey::personal(account_id, chatgpt_user_id)
    }
}

/// Returns the persisted Codex Apps tools cache path for the active auth identity.
pub fn codex_apps_tools_cache_path(codex_home: &Path, auth: Option<&CodexAuth>) -> PathBuf {
    mcp_tool_runtime_cache_path(codex_home, codex_apps_tools_cache_key(auth))
}
