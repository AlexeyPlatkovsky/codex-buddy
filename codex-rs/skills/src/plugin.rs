//! Portable plugin metadata used while discovering skills.

use codex_utils_absolute_path::AbsolutePathBuf;
use std::path::Path;
use std::path::PathBuf;

pub use codex_exec_server_protocol::DISCOVERABLE_PLUGIN_MANIFEST_PATHS;

pub const AGENT_PLUGIN_MANIFEST_RELATIVE_PATH: &str = "plugin.json";
/// Published Agent Plugins v1 manifest schema:
/// https://github.com/agentplugins/agent-plugins-spec/blob/main/schemas/1.0.0/plugin.schema.json
pub const AGENT_PLUGIN_SCHEMA_URI: &str =
    "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
pub const SUPPORTED_AGENT_PLUGIN_SCHEMA_URIS: &[&str] = &[AGENT_PLUGIN_SCHEMA_URI];
pub const AGENT_PLUGIN_SCHEMA_PREFIX: &str = "https://agent-plugins.org/schemas/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentPluginSchemaStatus {
    Supported,
    Unsupported,
    Unrelated,
}

pub fn agent_plugin_schema_status(contents: &str) -> AgentPluginSchemaStatus {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return AgentPluginSchemaStatus::Unrelated;
    };
    let Some(schema) = value.get("$schema").and_then(serde_json::Value::as_str) else {
        return AgentPluginSchemaStatus::Unrelated;
    };
    if SUPPORTED_AGENT_PLUGIN_SCHEMA_URIS.contains(&schema) {
        AgentPluginSchemaStatus::Supported
    } else if schema.starts_with(AGENT_PLUGIN_SCHEMA_PREFIX) {
        AgentPluginSchemaStatus::Unsupported
    } else {
        AgentPluginSchemaStatus::Unrelated
    }
}

pub fn find_plugin_manifest_path(plugin_root: &Path) -> Option<PathBuf> {
    let agent_manifest_path = plugin_root.join(AGENT_PLUGIN_MANIFEST_RELATIVE_PATH);
    match std::fs::symlink_metadata(&agent_manifest_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => {
            return None;
        }
        Ok(_) => {
            if std::fs::read_to_string(&agent_manifest_path)
                .ok()
                .is_some_and(|contents| {
                    agent_plugin_schema_status(&contents) != AgentPluginSchemaStatus::Unrelated
                })
            {
                return Some(agent_manifest_path);
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return None,
    }

    for relative_path in DISCOVERABLE_PLUGIN_MANIFEST_PATHS {
        let manifest_path = plugin_root.join(relative_path);
        let manifest_parent = manifest_path.parent()?;
        match std::fs::symlink_metadata(manifest_parent) {
            Ok(metadata) if !metadata.file_type().is_dir() => return None,
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return None,
        }
        match std::fs::symlink_metadata(&manifest_path) {
            Ok(metadata) if metadata.file_type().is_file() => return Some(manifest_path),
            Ok(_) => return None,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    None
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SkillDiscoveryMode {
    #[default]
    Recursive,
    DirectChildren,
}

/// The local identifier and optional remote identifier for a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginIdentity {
    pub plugin_id: String,
    pub remote_plugin_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginSkillRoot {
    pub path: AbsolutePathBuf,
    pub plugin_identity: PluginIdentity,
    pub plugin_namespace: String,
    pub plugin_root: AbsolutePathBuf,
    pub discovery_mode: SkillDiscoveryMode,
}

const PLUGIN_METADATA_DIR: &str = ".codex-plugin";
const MIGRATED_COMMAND_SKILLS_DIR: &str = "migrated-command-skills";

/// Returns the install-time command migration output directory for a plugin.
pub fn migrated_command_skills_root(plugin_root: &AbsolutePathBuf) -> AbsolutePathBuf {
    plugin_root
        .join(PLUGIN_METADATA_DIR)
        .join(MIGRATED_COMMAND_SKILLS_DIR)
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod tests;
