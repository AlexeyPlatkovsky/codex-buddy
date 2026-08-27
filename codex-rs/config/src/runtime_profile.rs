//! Translation between configuration provenance and runtime-profile policy.

use crate::ConfigLayerEntry;
use crate::ConfigLayerSource;
use crate::ConfigLayerStack;
use codex_runtime_profile::ExternalSource;
use codex_runtime_profile::ExternalSourcePolicy;
use codex_runtime_profile::RuntimePolicyPatch;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;

const EXTERNAL_SOURCES: [ExternalSource; 4] = [
    ExternalSource::Mcp,
    ExternalSource::Skills,
    ExternalSource::Instructions,
    ExternalSource::ClientTools,
];

/// TOML representation of a restriction for one optional external source.
///
/// The setting can only narrow the product policy selected by the runtime
/// preset. It never enables a source that the product or compile ceiling has
/// removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeSourcePolicyToml {
    /// Preserve the product policy for this source.
    Inherit,
    /// Permit the source only when it comes from an explicit trusted origin.
    ExplicitOnly,
    /// Do not load the source.
    Disabled,
}

impl RuntimeSourcePolicyToml {
    fn as_restriction(self) -> Option<ExternalSourcePolicy> {
        match self {
            Self::Inherit => None,
            Self::ExplicitOnly => Some(ExternalSourcePolicy::ExplicitOnly),
            Self::Disabled => Some(ExternalSourcePolicy::Disabled),
        }
    }

    fn from_toml_value(value: &toml::Value) -> Option<Self> {
        match value.as_str()? {
            "inherit" => Some(Self::Inherit),
            "explicit-only" => Some(Self::ExplicitOnly),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

/// Source-specific runtime restrictions under `[runtime.sources]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct RuntimeSourcesToml {
    pub mcp: Option<RuntimeSourcePolicyToml>,
    pub skills: Option<RuntimeSourcePolicyToml>,
    pub instructions: Option<RuntimeSourcePolicyToml>,
    pub client_tools: Option<RuntimeSourcePolicyToml>,
}

/// Runtime configuration under `[runtime]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct RuntimeToml {
    pub sources: Option<RuntimeSourcesToml>,
}

/// Policy and explicit-source grants resolved from enabled config layers.
///
/// `managed_restrictions` is supplied separately because managed policy must
/// remain authoritative even when another config layer attempts to widen it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProfilePolicy {
    restrictions: RuntimePolicyPatch,
    explicitly_configured_paths: BTreeSet<String>,
    explicitly_configured_sources: BTreeSet<ExternalSource>,
}

impl RuntimeProfilePolicy {
    /// Returns the monotonic restrictions that should be passed to runtime
    /// profile resolution.
    pub fn restrictions(&self) -> &RuntimePolicyPatch {
        &self.restrictions
    }

    /// Returns whether an enabled trusted layer explicitly configured a source.
    pub fn source_is_explicitly_configured(&self, source: ExternalSource) -> bool {
        self.explicitly_configured_sources.contains(&source)
    }

    /// Returns whether an explicit trusted layer configured this TOML path.
    ///
    /// A parent path matches any explicit child. For example, an MCP loader
    /// can query `mcp_servers.docs` to distinguish a user-configured server
    /// from a server contributed by packaged defaults.
    pub fn config_path_is_explicitly_configured(&self, config_path: &str) -> bool {
        self.explicitly_configured_paths
            .iter()
            .any(|configured_path| {
                configured_path == config_path
                    || configured_path
                        .strip_prefix(config_path)
                        .is_some_and(|suffix| suffix.starts_with('.'))
            })
    }
}

/// Resolves runtime-source restrictions and explicit grants from a config stack.
///
/// Disabled layers are ignored, which means untrusted project configuration can
/// neither grant a source nor relax a restriction. Packaged defaults may narrow
/// runtime policy but never count as an explicit external-source grant.
pub fn runtime_profile_policy_from_stack(
    config_layer_stack: &ConfigLayerStack,
    managed_restrictions: RuntimePolicyPatch,
) -> RuntimeProfilePolicy {
    let mut restrictions = RuntimePolicyPatch::default();

    for layer in config_layer_stack.layers_low_to_high() {
        for source in EXTERNAL_SOURCES {
            if let Some(restriction) = runtime_source_policy_in_layer(&layer.config, source)
                .and_then(RuntimeSourcePolicyToml::as_restriction)
            {
                restrictions = restrictions.restrict_external_source(source, restriction);
            }
        }
    }

    let explicitly_configured_paths: BTreeSet<String> = config_layer_stack
        .origins()
        .into_iter()
        .filter_map(|(path, origin)| {
            runtime_source_origin_is_explicit(&origin.name).then_some(path)
        })
        .collect();
    let explicitly_configured_sources = EXTERNAL_SOURCES
        .into_iter()
        .filter(|source| {
            explicitly_configured_paths
                .iter()
                .any(|path| runtime_source_config_path_matches(*source, path))
        })
        .collect();

    RuntimeProfilePolicy {
        restrictions: restrictions.restricted_by(managed_restrictions),
        explicitly_configured_paths,
        explicitly_configured_sources,
    }
}

/// Returns whether an enabled configuration layer is an explicit source grant.
///
/// Project layers must be enabled by `ConfigLayerStack`, which already requires
/// the project to be trusted. Packaged defaults are never an explicit grant
/// because they are product-owned rather than user/admin input.
pub fn runtime_source_layer_is_explicit(layer: &ConfigLayerEntry) -> bool {
    !layer.is_disabled() && runtime_source_origin_is_explicit(&layer.name)
}

fn runtime_source_origin_is_explicit(source: &ConfigLayerSource) -> bool {
    match source {
        ConfigLayerSource::PackagedDefaults { .. } => false,
        ConfigLayerSource::Mdm { .. }
        | ConfigLayerSource::System { .. }
        | ConfigLayerSource::EnterpriseManaged { .. }
        | ConfigLayerSource::User { .. }
        | ConfigLayerSource::Project { .. }
        | ConfigLayerSource::SessionFlags
        | ConfigLayerSource::LegacyManagedConfigTomlFromFile { .. }
        | ConfigLayerSource::LegacyManagedConfigTomlFromMdm => true,
    }
}

fn runtime_source_policy_in_layer(
    config: &toml::Value,
    source: ExternalSource,
) -> Option<RuntimeSourcePolicyToml> {
    config
        .get("runtime")?
        .get("sources")?
        .get(runtime_source_config_key(source))
        .and_then(RuntimeSourcePolicyToml::from_toml_value)
}

fn runtime_source_config_path_matches(source: ExternalSource, config_path: &str) -> bool {
    match source {
        ExternalSource::Mcp => path_matches_or_contains(config_path, "mcp_servers"),
        ExternalSource::Skills => path_matches_or_contains(config_path, "skills"),
        ExternalSource::Instructions => matches!(
            config_path,
            "instructions" | "developer_instructions" | "model_instructions_file"
        ),
        // Dynamic client tools are supplied by a session client rather than a
        // TOML value, so callers must validate its `ConfigLayerEntry` directly.
        ExternalSource::ClientTools => false,
    }
}

fn path_matches_or_contains(config_path: &str, parent_path: &str) -> bool {
    config_path == parent_path
        || config_path
            .strip_prefix(parent_path)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn runtime_source_config_key(source: ExternalSource) -> &'static str {
    match source {
        ExternalSource::Mcp => "mcp",
        ExternalSource::Skills => "skills",
        ExternalSource::Instructions => "instructions",
        ExternalSource::ClientTools => "client_tools",
    }
}

#[cfg(test)]
#[path = "runtime_profile_tests.rs"]
mod tests;
