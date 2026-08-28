#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AppConnectorId(pub(crate) String);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PluginCapabilitySummary {
    pub(crate) config_name: String,
    pub(crate) display_name: String,
    pub(crate) plugin_namespace: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) has_skills: bool,
    pub(crate) mcp_server_names: Vec<String>,
    pub(crate) app_connector_ids: Vec<AppConnectorId>,
}

pub(crate) const TOOL_MENTION_SIGIL: char = '$';
pub(crate) const PLUGIN_TEXT_MENTION_SIGIL: char = '@';

pub(crate) const OPENAI_CURATED_MARKETPLACE_NAME: &str = "openai-curated";
const OPENAI_API_CURATED_MARKETPLACE_NAME: &str = "openai-api-curated";
pub(crate) const REMOTE_GLOBAL_MARKETPLACE_NAME: &str = "openai-curated-remote";
pub(crate) const REMOTE_WORKSPACE_MARKETPLACE_NAME: &str = "workspace-directory";
pub(crate) const REMOTE_WORKSPACE_SHARED_WITH_ME_MARKETPLACE_NAME: &str =
    "workspace-shared-with-me";
pub(crate) const REMOTE_WORKSPACE_SHARED_WITH_ME_PRIVATE_MARKETPLACE_NAME: &str =
    "workspace-shared-with-me-private";
pub(crate) const REMOTE_WORKSPACE_SHARED_WITH_ME_UNLISTED_MARKETPLACE_NAME: &str =
    "workspace-shared-with-me-unlisted";

pub(crate) fn is_openai_curated_marketplace_name(marketplace_name: &str) -> bool {
    marketplace_name == OPENAI_CURATED_MARKETPLACE_NAME
        || marketplace_name == OPENAI_API_CURATED_MARKETPLACE_NAME
}
