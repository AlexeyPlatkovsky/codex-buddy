use codex_core::config::Config;
#[cfg(feature = "plugin-runtime")]
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
#[cfg(feature = "plugin-runtime")]
use codex_extension_api::McpServerContribution;
#[cfg(feature = "plugin-runtime")]
use codex_extension_api::McpServerContributionContext;
#[cfg(feature = "plugin-runtime")]
use codex_extension_api::McpServerContributor;
#[cfg(feature = "plugin-runtime")]
use codex_mcp::CODEX_APPS_MCP_SERVER_NAME;
#[cfg(feature = "plugin-runtime")]
use codex_mcp::hosted_plugin_runtime_mcp_server_config;

#[cfg(feature = "plugin-runtime")]
mod executor_plugin;

#[cfg(feature = "plugin-runtime")]
struct HostedPluginRuntimeExtension;

#[cfg(feature = "plugin-runtime")]
impl McpServerContributor<Config> for HostedPluginRuntimeExtension {
    fn id(&self) -> &'static str {
        "hosted_plugin_runtime"
    }

    fn contribute<'a>(
        &'a self,
        context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            let config = context.config();
            let name = CODEX_APPS_MCP_SERVER_NAME.to_string();
            if !config.features.enabled(codex_features::Feature::Apps) {
                return vec![McpServerContribution::Remove { name }];
            }

            vec![McpServerContribution::HostedApps {
                config: Box::new(hosted_plugin_runtime_mcp_server_config(
                    &config.chatgpt_base_url,
                    config.apps_mcp_product_sku.as_deref(),
                    context.originator(),
                )),
            }]
        })
    }
}

#[cfg(feature = "plugin-runtime")]
pub fn install(builder: &mut ExtensionRegistryBuilder<Config>) {
    builder.mcp_server_contributor(std::sync::Arc::new(HostedPluginRuntimeExtension));
}

#[cfg(not(feature = "plugin-runtime"))]
pub fn install(_builder: &mut ExtensionRegistryBuilder<Config>) {}

/// Installs discovery for MCP servers declared by thread-selected executor plugins.
#[cfg(feature = "plugin-runtime")]
pub fn install_executor_plugins(
    builder: &mut ExtensionRegistryBuilder<Config>,
    environment_manager: std::sync::Arc<codex_exec_server::EnvironmentManager>,
) {
    builder.mcp_server_contributor(std::sync::Arc::new(
        executor_plugin::SelectedExecutorPluginMcpContributor::new(environment_manager),
    ));
}

#[cfg(all(test, feature = "plugin-runtime"))]
#[path = "lib_tests.rs"]
mod tests;
