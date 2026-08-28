//! Implementation-neutral plugin identifiers and runtime metadata shared across Codex crates.

mod plugin_id;

use codex_config::HookEventsToml;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;
pub use plugin_id::PluginId;
pub use plugin_id::PluginIdError;
pub use plugin_id::validate_plugin_segment;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppConnectorId(pub String);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginCapabilitySummary {
    pub config_name: String,
    pub display_name: String,
    pub plugin_namespace: Option<String>,
    pub description: Option<String>,
    pub has_skills: bool,
    pub mcp_server_names: Vec<String>,
    pub app_connector_ids: Vec<AppConnectorId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHookSource {
    pub plugin_id: PluginId,
    pub plugin_root: AbsolutePathBuf,
    pub plugin_data_root: AbsolutePathBuf,
    pub source_path: AbsolutePathBuf,
    pub source_relative_path: String,
    pub hooks: HookEventsToml,
}

/// Inline plugin hooks whose paths and MCP target belong to an executor environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorPluginHookSource {
    pub plugin_id: PluginId,
    pub environment_id: String,
    pub plugin_root: PathUri,
    pub manifest_path: PathUri,
    pub source_relative_path: String,
    pub hooks: HookEventsToml,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTelemetryMetadata {
    /// Local plugin identifier used by Codex configuration and the plugin cache,
    /// when it has been resolved.
    pub plugin_id: Option<PluginId>,
    /// Optional backend identifier for remote plugins.
    pub remote_plugin_id: Option<String>,
    pub capability_summary: Option<PluginCapabilitySummary>,
}
