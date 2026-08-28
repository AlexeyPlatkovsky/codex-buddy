use codex_connectors::AppInfo;
use codex_connectors::ConnectorMetadata;
use codex_core::config::Config;
use codex_login::CodexAuth;
use codex_plugin::AppConnectorId;

pub(crate) use codex_core::connectors::list_accessible_connectors_from_mcp_tools_with_mcp_manager;
pub(crate) use codex_core::connectors::list_cached_accessible_connectors_from_mcp_tools;

pub(crate) async fn list_cached_all_connectors(
    _config: &Config,
    _plugin_apps: &[AppConnectorId],
) -> Option<Vec<AppInfo>> {
    Some(Vec::new())
}

pub(crate) async fn list_all_connectors_with_options(
    _config: &Config,
    _force_refetch: bool,
    _plugin_apps: &[AppConnectorId],
) -> anyhow::Result<Vec<AppInfo>> {
    Ok(Vec::new())
}

pub(crate) struct ConnectorMetadataReadResult {
    pub(crate) apps: Vec<ConnectorMetadata>,
    pub(crate) missing_app_ids: Vec<String>,
}

pub(crate) async fn read_connector_metadata(
    _config: &Config,
    _auth: &CodexAuth,
    app_ids: &[String],
    _include_tools: bool,
) -> anyhow::Result<ConnectorMetadataReadResult> {
    Ok(ConnectorMetadataReadResult {
        apps: Vec::new(),
        missing_app_ids: app_ids.to_vec(),
    })
}

pub(crate) fn merge_connectors_with_accessible(
    _connectors: Vec<AppInfo>,
    _accessible_connectors: Vec<AppInfo>,
    _all_connectors_loaded: bool,
) -> Vec<AppInfo> {
    Vec::new()
}
