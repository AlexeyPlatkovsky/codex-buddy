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

/// Immutable connector selection and provenance after applying disabled plugins.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectorSnapshot {
    connector_ids: Vec<String>,
    plugin_display_names_by_connector_id: HashMap<String, Vec<String>>,
    disabled_connector_ids: HashSet<String>,
}

impl ConnectorSnapshot {
    /// Builds the final selection from all plugin sources, preserving contribution order.
    /// An enabled contributor preserves a shared connector unless its canonical owner is disabled.
    pub fn from_plugin_sources(
        sources: impl IntoIterator<Item = PluginConnectorSource>,
        disabled_plugin_ids: &[String],
        canonical_disabled_connector_ids: HashSet<String>,
    ) -> Self {
        let mut connector_ids = Vec::new();
        let mut seen_connector_ids = HashSet::new();
        let mut plugin_display_names_by_connector_id: HashMap<String, Vec<String>> = HashMap::new();
        let mut disabled_connector_ids = HashSet::new();

        for source in sources {
            if disabled_plugin_ids.contains(&source.plugin_id) {
                disabled_connector_ids.extend(source.connector_ids);
                continue;
            }
            for connector_id in source.connector_ids() {
                if canonical_disabled_connector_ids.contains(connector_id) {
                    continue;
                }
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
        disabled_connector_ids.retain(|id| !seen_connector_ids.contains(id));
        disabled_connector_ids.extend(canonical_disabled_connector_ids);

        Self {
            connector_ids,
            plugin_display_names_by_connector_id,
            disabled_connector_ids,
        }
    }

    /// Returns the connector IDs in source contribution order.
    pub fn connector_ids(&self) -> &[String] {
        &self.connector_ids
    }

    /// Connector tools explicitly excluded or excluded by all contributing plugins.
    pub fn disabled_connector_ids(&self) -> &HashSet<String> {
        &self.disabled_connector_ids
    }

    /// Returns the package display names associated with one connector.
    pub fn plugin_display_names_for_connector_id(&self, connector_id: &str) -> &[String] {
        self.plugin_display_names_by_connector_id
            .get(connector_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "connector_provenance_tests.rs"]
mod tests;
