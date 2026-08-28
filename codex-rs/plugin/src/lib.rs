//! Shared plugin package models, source providers, identifiers, and telemetry summaries.

use std::collections::HashSet;

pub use codex_skills::mention_syntax;

mod load_outcome;
pub mod manifest;
mod plugin_app_config;
mod provider;

pub use codex_plugin_types::AppConnectorId;
pub use codex_plugin_types::ExecutorPluginHookSource;
pub use codex_plugin_types::PluginCapabilitySummary;
pub use codex_plugin_types::PluginHookSource;
pub use codex_plugin_types::PluginId;
pub use codex_plugin_types::PluginIdError;
pub use codex_plugin_types::PluginTelemetryMetadata;
pub use codex_plugin_types::validate_plugin_segment;
pub use load_outcome::LoadedPlugin;
pub use load_outcome::PluginLoadOutcome;
pub use load_outcome::prompt_safe_plugin_description;
pub use plugin_app_config::parse_plugin_app_config;
pub use plugin_app_config::parse_plugin_app_config_value;
pub use provider::PluginProvider;
pub use provider::PluginResourceLocator;
pub use provider::ResolvedPlugin;
pub use provider::ResolvedPluginError;
pub use provider::ResolvedPluginLocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDeclaration {
    pub name: String,
    pub connector_id: AppConnectorId,
    pub category: Option<String>,
}

pub fn app_connector_ids_from_declarations<'a>(
    app_declarations: impl IntoIterator<Item = &'a AppDeclaration>,
) -> Vec<AppConnectorId> {
    let mut connector_ids = Vec::new();
    let mut seen_connector_ids = HashSet::new();
    for app in app_declarations {
        if seen_connector_ids.insert(&app.connector_id) {
            connector_ids.push(app.connector_id.clone());
        }
    }
    connector_ids
}
