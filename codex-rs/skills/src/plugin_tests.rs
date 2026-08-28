use super::*;
use pretty_assertions::assert_eq;
use std::fs;
use tempfile::tempdir;

const CLAUDE_PLUGIN_MANIFEST: &str = ".claude-plugin/plugin.json";
const CURSOR_PLUGIN_MANIFEST: &str = ".cursor-plugin/plugin.json";

#[test]
fn root_agent_manifest_takes_precedence_over_legacy_manifests() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let root_manifest = plugin_root.join(AGENT_PLUGIN_MANIFEST_RELATIVE_PATH);
    let legacy_manifest = plugin_root.join(".codex-plugin/plugin.json");
    fs::create_dir_all(legacy_manifest.parent().expect("legacy manifest parent"))
        .expect("create legacy manifest parent");
    fs::write(
        &root_manifest,
        format!(r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"sample"}}"#),
    )
    .expect("write root manifest");
    fs::write(&legacy_manifest, r#"{"name":"legacy"}"#).expect("write legacy manifest");

    assert_eq!(find_plugin_manifest_path(plugin_root), Some(root_manifest));
}

#[test]
fn unrelated_root_manifest_falls_back_to_legacy_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let root_manifest = plugin_root.join(AGENT_PLUGIN_MANIFEST_RELATIVE_PATH);
    let legacy_manifest = plugin_root.join(".codex-plugin/plugin.json");
    fs::create_dir_all(legacy_manifest.parent().expect("legacy manifest parent"))
        .expect("create legacy manifest parent");
    fs::write(&root_manifest, r#"{"name":"unrelated"}"#).expect("write root manifest");
    fs::write(&legacy_manifest, r#"{"name":"legacy"}"#).expect("write legacy manifest");

    assert_eq!(
        find_plugin_manifest_path(plugin_root),
        Some(legacy_manifest)
    );
}

#[test]
fn classifies_agent_plugin_schema_versions() {
    assert_eq!(
        [
            format!(r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}"}}"#),
            r#"{"$schema":"https://agent-plugins.org/schemas/2.0.0/plugin.schema.json"}"#
                .to_string(),
            r#"{"name":"unrelated"}"#.to_string(),
        ]
        .map(|contents| agent_plugin_schema_status(&contents)),
        [
            AgentPluginSchemaStatus::Supported,
            AgentPluginSchemaStatus::Unsupported,
            AgentPluginSchemaStatus::Unrelated,
        ]
    );
}

#[test]
fn rejects_nonregular_root_plugin_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let legacy_manifest = plugin_root.join(".codex-plugin/plugin.json");
    fs::create_dir_all(plugin_root.join(AGENT_PLUGIN_MANIFEST_RELATIVE_PATH))
        .expect("root manifest directory");
    fs::create_dir_all(legacy_manifest.parent().expect("legacy manifest parent"))
        .expect("create legacy manifest parent");
    fs::write(&legacy_manifest, r#"{"name":"legacy"}"#).expect("write legacy manifest");

    assert_eq!(find_plugin_manifest_path(plugin_root), None);
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_root_plugin_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path().join("plugin");
    let manifest_target = temp.path().join("manifest.json");
    let legacy_manifest = plugin_root.join(".codex-plugin/plugin.json");
    fs::create_dir_all(&plugin_root).expect("create plugin root");
    fs::write(
        &manifest_target,
        format!(r#"{{"$schema":"{AGENT_PLUGIN_SCHEMA_URI}","name":"sample"}}"#),
    )
    .expect("write manifest target");
    std::os::unix::fs::symlink(
        &manifest_target,
        plugin_root.join(AGENT_PLUGIN_MANIFEST_RELATIVE_PATH),
    )
    .expect("create root manifest symlink");
    fs::create_dir_all(legacy_manifest.parent().expect("legacy manifest parent"))
        .expect("create legacy manifest parent");
    fs::write(&legacy_manifest, r#"{"name":"legacy"}"#).expect("write legacy manifest");

    assert_eq!(find_plugin_manifest_path(&plugin_root), None);
}

#[test]
fn rejects_nonregular_legacy_manifest_before_lower_precedence_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let codex_manifest = plugin_root.join(".codex-plugin/plugin.json");
    let claude_manifest = plugin_root.join(CLAUDE_PLUGIN_MANIFEST);
    fs::create_dir_all(&codex_manifest).expect("nonregular Codex manifest");
    fs::create_dir_all(claude_manifest.parent().expect("Claude manifest parent"))
        .expect("create Claude manifest parent");
    fs::write(&claude_manifest, r#"{"name":"claude"}"#).expect("write Claude manifest");

    assert_eq!(find_plugin_manifest_path(plugin_root), None);
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_legacy_manifest_before_lower_precedence_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let codex_manifest = plugin_root.join(".codex-plugin/plugin.json");
    let claude_manifest = plugin_root.join(CLAUDE_PLUGIN_MANIFEST);
    fs::create_dir_all(codex_manifest.parent().expect("Codex manifest parent"))
        .expect("create Codex manifest parent");
    fs::create_dir_all(claude_manifest.parent().expect("Claude manifest parent"))
        .expect("create Claude manifest parent");
    fs::write(plugin_root.join("benign.json"), r#"{"name":"sample"}"#)
        .expect("write benign manifest");
    fs::write(&claude_manifest, r#"{"name":"claude"}"#).expect("write Claude manifest");
    std::os::unix::fs::symlink("../benign.json", &codex_manifest)
        .expect("create Codex manifest symlink");

    assert_eq!(find_plugin_manifest_path(plugin_root), None);
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_legacy_manifest_directory_before_lower_precedence_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path().join("plugin");
    let manifest_directory = temp.path().join("manifest-directory");
    let claude_manifest = plugin_root.join(CLAUDE_PLUGIN_MANIFEST);
    fs::create_dir_all(&manifest_directory).expect("create manifest target directory");
    fs::create_dir_all(claude_manifest.parent().expect("Claude manifest parent"))
        .expect("create Claude manifest parent");
    fs::write(
        manifest_directory.join("plugin.json"),
        r#"{"name":"sample"}"#,
    )
    .expect("write target manifest");
    fs::write(&claude_manifest, r#"{"name":"claude"}"#).expect("write Claude manifest");
    std::os::unix::fs::symlink(&manifest_directory, plugin_root.join(".codex-plugin"))
        .expect("create Codex manifest directory symlink");

    assert_eq!(find_plugin_manifest_path(&plugin_root), None);
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_claude_manifest_before_cursor_manifest() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let claude_manifest = plugin_root.join(CLAUDE_PLUGIN_MANIFEST);
    let cursor_manifest = plugin_root.join(CURSOR_PLUGIN_MANIFEST);
    fs::create_dir_all(claude_manifest.parent().expect("Claude manifest parent"))
        .expect("create Claude manifest parent");
    fs::create_dir_all(cursor_manifest.parent().expect("Cursor manifest parent"))
        .expect("create Cursor manifest parent");
    let manifest_target = plugin_root.join("benign.json");
    fs::write(&manifest_target, r#"{"name":"sample"}"#).expect("write benign manifest");
    fs::write(&cursor_manifest, r#"{"name":"cursor"}"#).expect("write Cursor manifest");
    std::os::unix::fs::symlink(&manifest_target, &claude_manifest)
        .expect("create Claude manifest symlink");

    assert_eq!(find_plugin_manifest_path(plugin_root), None);
}

#[test]
fn preserves_codex_claude_cursor_legacy_precedence() {
    let temp = tempdir().expect("tempdir");
    let plugin_root = temp.path();
    let codex_manifest = plugin_root.join(".codex-plugin/plugin.json");
    let claude_manifest = plugin_root.join(CLAUDE_PLUGIN_MANIFEST);
    let cursor_manifest = plugin_root.join(CURSOR_PLUGIN_MANIFEST);
    for manifest in [&codex_manifest, &claude_manifest, &cursor_manifest] {
        fs::create_dir_all(manifest.parent().expect("manifest parent"))
            .expect("create manifest parent");
        fs::write(manifest, r#"{"name":"sample"}"#).expect("write manifest");
    }

    assert_eq!(
        find_plugin_manifest_path(plugin_root),
        Some(codex_manifest.clone())
    );
    fs::remove_file(codex_manifest).expect("remove Codex manifest");
    assert_eq!(
        find_plugin_manifest_path(plugin_root),
        Some(claude_manifest)
    );
}
