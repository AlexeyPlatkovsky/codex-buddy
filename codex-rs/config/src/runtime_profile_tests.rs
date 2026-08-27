use super::*;
use crate::AbsolutePathBuf;
use crate::ConfigLayerEntry;
use crate::ConfigRequirements;
use crate::ConfigRequirementsToml;
use crate::config_toml::ConfigToml;
use codex_runtime_profile::ResolvedRuntimeProfile;
use codex_runtime_profile::RuntimeCompileCeiling;
use tempfile::TempDir;

fn absolute_path(temp_dir: &TempDir, name: &str) -> AbsolutePathBuf {
    AbsolutePathBuf::from_absolute_path(temp_dir.path().join(name))
        .expect("test path should be absolute")
}

fn stack(layers: Vec<ConfigLayerEntry>) -> ConfigLayerStack {
    ConfigLayerStack::new(
        layers,
        ConfigRequirements::default(),
        ConfigRequirementsToml::default(),
    )
    .expect("config stack should be valid")
}

fn layer(source: ConfigLayerSource, config: &str) -> ConfigLayerEntry {
    ConfigLayerEntry::new(
        source,
        toml::from_str(config).expect("config TOML should parse"),
    )
}

#[test]
fn configured_sources_require_an_enabled_explicit_origin() {
    let temp_dir = TempDir::new().expect("tempdir");
    let packaged = layer(
        ConfigLayerSource::PackagedDefaults {
            file: absolute_path(&temp_dir, "defaults.toml"),
        },
        r#"
instructions = "packaged"

[mcp_servers.packaged]
command = "packaged-mcp"

[skills.bundled]
enabled = true
"#,
    );
    let user = layer(
        ConfigLayerSource::User {
            file: absolute_path(&temp_dir, "config.toml"),
            profile: None,
        },
        r#"
developer_instructions = "user"

[mcp_servers.user]
command = "user-mcp"

[skills.bundled]
enabled = true
"#,
    );
    let untrusted_project = ConfigLayerEntry::new_disabled(
        ConfigLayerSource::Project {
            dot_codex_folder: absolute_path(&temp_dir, ".codex"),
        },
        toml::from_str(
            r#"
[mcp_servers.project]
command = "project-mcp"
"#,
        )
        .expect("project config"),
        "project is not trusted",
    );

    let policy = runtime_profile_policy_from_stack(
        &stack(vec![packaged, user, untrusted_project]),
        RuntimePolicyPatch::default(),
    );

    assert_eq!(
        (
            policy.source_is_explicitly_configured(ExternalSource::Mcp),
            policy.source_is_explicitly_configured(ExternalSource::Skills),
            policy.source_is_explicitly_configured(ExternalSource::Instructions),
            policy.source_is_explicitly_configured(ExternalSource::ClientTools),
        ),
        (true, true, true, false)
    );
    assert_eq!(
        (
            policy.config_path_is_explicitly_configured("mcp_servers.packaged"),
            policy.config_path_is_explicitly_configured("mcp_servers.user"),
        ),
        (false, true)
    );
}

#[test]
fn packaged_defaults_never_grant_an_external_source() {
    let temp_dir = TempDir::new().expect("tempdir");
    let policy = runtime_profile_policy_from_stack(
        &stack(vec![layer(
            ConfigLayerSource::PackagedDefaults {
                file: absolute_path(&temp_dir, "defaults.toml"),
            },
            r#"
[mcp_servers.packaged]
command = "packaged-mcp"
"#,
        )]),
        RuntimePolicyPatch::default(),
    );

    assert!(!policy.source_is_explicitly_configured(ExternalSource::Mcp));
}

#[test]
fn managed_restrictions_cannot_be_widened_by_config_layers() {
    let temp_dir = TempDir::new().expect("tempdir");
    let policy = runtime_profile_policy_from_stack(
        &stack(vec![layer(
            ConfigLayerSource::User {
                file: absolute_path(&temp_dir, "config.toml"),
                profile: None,
            },
            r#"
[runtime.sources]
mcp = "inherit"
skills = "explicit-only"
"#,
        )]),
        RuntimePolicyPatch::default()
            .restrict_external_source(ExternalSource::Mcp, ExternalSourcePolicy::Disabled),
    );
    let resolved =
        ResolvedRuntimeProfile::full(&RuntimeCompileCeiling::full(), policy.restrictions());

    assert_eq!(
        (
            resolved.external_source(ExternalSource::Mcp),
            resolved.external_source(ExternalSource::Skills),
        ),
        (
            ExternalSourcePolicy::Disabled,
            ExternalSourcePolicy::ExplicitOnly,
        )
    );
}

#[test]
fn explicit_layer_classifies_admin_user_trusted_project_and_session_inputs() {
    let temp_dir = TempDir::new().expect("tempdir");

    assert_eq!(
        (
            runtime_source_layer_is_explicit(&layer(
                ConfigLayerSource::PackagedDefaults {
                    file: absolute_path(&temp_dir, "defaults.toml"),
                },
                "",
            )),
            runtime_source_layer_is_explicit(&layer(
                ConfigLayerSource::System {
                    file: absolute_path(&temp_dir, "system.toml"),
                },
                "",
            )),
            runtime_source_layer_is_explicit(&layer(
                ConfigLayerSource::User {
                    file: absolute_path(&temp_dir, "config.toml"),
                    profile: Some("buddy".to_string()),
                },
                "",
            )),
            runtime_source_layer_is_explicit(&layer(
                ConfigLayerSource::Project {
                    dot_codex_folder: absolute_path(&temp_dir, ".codex"),
                },
                "",
            )),
            runtime_source_layer_is_explicit(&layer(ConfigLayerSource::SessionFlags, "")),
        ),
        (false, true, true, true, true)
    );

    let untrusted_project = ConfigLayerEntry::new_disabled(
        ConfigLayerSource::Project {
            dot_codex_folder: absolute_path(&temp_dir, ".codex-untrusted"),
        },
        toml::Value::Table(toml::map::Map::new()),
        "project is not trusted",
    );
    assert!(!runtime_source_layer_is_explicit(&untrusted_project));
}

#[test]
fn runtime_source_restrictions_deserialize_from_config_toml() {
    let config: ConfigToml = toml::from_str(
        r#"
[runtime.sources]
mcp = "explicit-only"
skills = "disabled"
instructions = "inherit"
client_tools = "disabled"
"#,
    )
    .expect("runtime source configuration should deserialize");

    assert_eq!(
        config.runtime,
        Some(RuntimeToml {
            sources: Some(RuntimeSourcesToml {
                mcp: Some(RuntimeSourcePolicyToml::ExplicitOnly),
                skills: Some(RuntimeSourcePolicyToml::Disabled),
                instructions: Some(RuntimeSourcePolicyToml::Inherit),
                client_tools: Some(RuntimeSourcePolicyToml::Disabled),
            }),
        })
    );
}
