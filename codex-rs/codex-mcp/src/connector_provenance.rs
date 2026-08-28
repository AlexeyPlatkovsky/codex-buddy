use std::collections::HashMap;
use std::collections::HashSet;

/// Connector declarations contributed by one plugin package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginConnectorSource {
    plugin_id: String,
    plugin_display_name: String,
    connector_ids: Vec<String>,
}

impl PluginConnectorSource {
    /// Creates one plugin source from connector IDs that were already parsed.
    pub fn from_connector_ids(
        plugin_id: impl Into<String>,
        plugin_display_name: impl Into<String>,
        connector_ids: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut seen_connector_ids = HashSet::new();
        let connector_ids = connector_ids
            .into_iter()
            .filter(|connector_id| !connector_id.trim().is_empty())
            .filter(|connector_id| seen_connector_ids.insert(connector_id.clone()))
            .collect();
        Self {
            plugin_id: plugin_id.into(),
            plugin_display_name: plugin_display_name.into(),
            connector_ids,
        }
    }

    /// Returns the package name shown in connector provenance.
    pub fn plugin_display_name(&self) -> &str {
        &self.plugin_display_name
    }

    /// Returns the connector IDs contributed by this package.
    pub fn connector_ids(&self) -> &[String] {
        &self.connector_ids
    }
}

/// Immutable connector declarations and their plugin provenance.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectorSnapshot {
    sources: Vec<PluginConnectorSource>,
    connector_ids: Vec<String>,
    plugin_display_names_by_connector_id: HashMap<String, Vec<String>>,
}

impl ConnectorSnapshot {
    /// Builds a connector snapshot from package-scoped declarations.
    pub fn from_plugin_sources(sources: impl IntoIterator<Item = PluginConnectorSource>) -> Self {
        let sources = sources
            .into_iter()
            .filter(|source| !source.connector_ids().is_empty())
            .collect::<Vec<_>>();
        let mut connector_ids = Vec::new();
        let mut seen_connector_ids = HashSet::new();
        let mut plugin_display_names_by_connector_id: HashMap<String, Vec<String>> = HashMap::new();

        for source in &sources {
            for connector_id in source.connector_ids() {
                if seen_connector_ids.insert(connector_id.clone()) {
                    connector_ids.push(connector_id.clone());
                }
                plugin_display_names_by_connector_id
                    .entry(connector_id.clone())
                    .or_default()
                    .push(source.plugin_display_name().to_string());
            }
        }
        for plugin_names in plugin_display_names_by_connector_id.values_mut() {
            plugin_names.sort_unstable();
            plugin_names.dedup();
        }

        Self {
            sources,
            connector_ids,
            plugin_display_names_by_connector_id,
        }
    }

    /// Returns the connector IDs in source contribution order.
    pub fn connector_ids(&self) -> &[String] {
        &self.connector_ids
    }

    /// Returns the package display names associated with one connector.
    pub fn plugin_display_names_for_connector_id(&self, connector_id: &str) -> &[String] {
        self.plugin_display_names_by_connector_id
            .get(connector_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Combines two snapshots while preserving source order and provenance.
    pub fn merged_with(&self, other: &Self) -> Self {
        Self::from_plugin_sources(self.sources.iter().chain(&other.sources).cloned())
    }
}

#[cfg(test)]
#[path = "connector_provenance_tests.rs"]
mod tests;
