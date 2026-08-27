use super::*;
use crate::config::ConfigBuilder;
use crate::plugins::plugins_manager_for_config;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::McpServerContribution;
use codex_extension_api::McpServerContributionContext;
use codex_extension_api::McpServerContributor;
use codex_login::test_support::auth_manager_from_optional_auth;
use codex_runtime_profile::RuntimePreset;
use pretty_assertions::assert_eq;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tempfile::tempdir;

struct CountingMcpContributor {
    calls: Arc<AtomicUsize>,
}

impl McpServerContributor<Config> for CountingMcpContributor {
    fn id(&self) -> &'static str {
        "counting_mcp_contributor"
    }

    fn contribute<'a>(
        &'a self,
        _context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Vec::new()
        })
    }
}

#[tokio::test]
async fn coding_projection_skips_automatic_mcp_contributors_and_servers() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let coding = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .runtime_preset(RuntimePreset::Coding)
        .build()
        .await?;
    let full = ConfigBuilder::without_managed_config_for_tests()
        .codex_home(codex_home.path().to_path_buf())
        .runtime_preset(RuntimePreset::Full)
        .build()
        .await?;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut extensions = ExtensionRegistryBuilder::new();
    extensions.mcp_server_contributor(Arc::new(CountingMcpContributor {
        calls: Arc::clone(&calls),
    }));
    let extensions = Arc::new(extensions.build());
    let plugins_manager = Arc::new(plugins_manager_for_config(
        &coding,
        auth_manager_from_optional_auth(/*auth*/ None),
    ));
    let manager = McpManager::new_with_extensions(
        plugins_manager,
        extensions,
        ConnectorRuntimeManager::default(),
    );

    assert_eq!(manager.runtime_servers(&coding).await, HashMap::new());
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    manager.runtime_servers(&full).await;
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    Ok(())
}
