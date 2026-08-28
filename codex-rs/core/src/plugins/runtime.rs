//! Compile-time boundary between Core and the optional plugin runtime.

#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::PLUGIN_METRICS_OUTPUT_ENV_VAR;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::PluginCommandAttribution;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::PluginLoadOutcome;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::PluginMetricsSidecar;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::PluginsConfigInput;
#[cfg(feature = "plugins")]
pub use codex_core_plugins::PluginsManager;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::RecommendedPluginCandidatesInput;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::ResolvedPluginMetricsOperation;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::TrustedPluginRoots;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::executor_plugin_hook_sources;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::recognize_artifact_operation;
#[cfg(feature = "plugins")]
pub(crate) use codex_core_plugins::strip_output_env;

#[cfg(not(feature = "plugins"))]
mod disabled {
    use codex_config::ConfigLayerStack;
    use codex_config::types::McpServerConfig;
    use codex_exec_server::Environment;
    use codex_exec_server::ExecutorCapabilityDiscoverySnapshot;
    use codex_exec_server::ExecutorFileSystem;
    use codex_login::AuthManager;
    use codex_login::CodexAuth;
    use codex_plugin_types::AppDeclaration;
    use codex_plugin_types::ExecutorPluginHookSource;
    use codex_plugin_types::PluginCapabilitySummary;
    use codex_plugin_types::PluginHookSource;
    use codex_plugin_types::PluginId;
    use codex_protocol::auth::AuthMode;
    use codex_protocol::models::AdditionalPermissionProfile;
    use codex_skills::PluginSkillRoot;
    use codex_skills::SkillRootSnapshots;
    use codex_tools::DiscoverableTool;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use codex_utils_path_uri::PathUri;
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::Arc;

    pub(crate) const PLUGIN_METRICS_OUTPUT_ENV_VAR: &str = "CODEX_PLUGIN_METRICS_OUTPUT";

    #[derive(Debug, Clone)]
    pub struct PluginsConfigInput {
        pub plugins_enabled: bool,
    }

    impl PluginsConfigInput {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            _config_layer_stack: ConfigLayerStack,
            _model_provider_id: String,
            plugins_enabled: bool,
            _remote_plugin_enabled: bool,
            _chatgpt_base_url: String,
            _http_client_factory: codex_http_client::HttpClientFactory,
        ) -> Self {
            Self { plugins_enabled }
        }
    }

    pub(crate) struct RecommendedPluginCandidatesInput<'a> {
        pub(crate) plugins_config: &'a PluginsConfigInput,
        pub(crate) loaded_plugins: &'a PluginLoadOutcome,
        pub(crate) auth: Option<&'a CodexAuth>,
        pub(crate) disabled_tools: &'a [codex_config::types::ToolSuggestDisabledTool],
        pub(crate) app_server_client_name: Option<&'a str>,
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct PluginLoadOutcome {
        plugins: Vec<LoadedPlugin>,
        capability_summaries: Vec<PluginCapabilitySummary>,
    }

    impl PluginLoadOutcome {
        pub fn effective_plugin_skill_roots(&self) -> Vec<PluginSkillRoot> {
            Vec::new()
        }

        pub fn effective_plugin_hook_sources(&self) -> Vec<PluginHookSource> {
            Vec::new()
        }

        pub fn effective_plugin_hook_warnings(&self) -> Vec<String> {
            Vec::new()
        }

        pub fn capability_summaries(&self) -> &[PluginCapabilitySummary] {
            &self.capability_summaries
        }

        pub fn plugins(&self) -> &[LoadedPlugin] {
            &self.plugins
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LoadedPlugin {
        pub config_name: String,
        pub root: AbsolutePathBuf,
        pub mcp_servers: HashMap<String, McpServerConfig>,
        pub apps: Vec<AppDeclaration>,
    }

    impl LoadedPlugin {
        pub fn is_active(&self) -> bool {
            false
        }

        pub fn display_name(&self) -> &str {
            &self.config_name
        }

        pub fn is_agent_plugin(&self) -> bool {
            false
        }
    }

    pub struct PluginsManager {
        auth_manager: Arc<AuthManager>,
    }

    impl PluginsManager {
        pub fn new<T>(
            _codex_home: std::path::PathBuf,
            auth_manager: Arc<AuthManager>,
            _skill_root_loader: Arc<T>,
        ) -> Self
        where
            T: ?Sized,
        {
            Self { auth_manager }
        }

        pub fn new_with_options<T, U>(
            _codex_home: std::path::PathBuf,
            _restriction_product: Option<T>,
            auth_manager: Arc<AuthManager>,
            _skill_root_loader: Arc<U>,
        ) -> Self
        where
            U: ?Sized,
        {
            Self { auth_manager }
        }

        pub fn auth_mode(&self) -> Option<AuthMode> {
            self.auth_manager.get_api_auth_mode()
        }

        pub fn clear_cache(&self) {}

        pub async fn plugins_for_config(&self, _config: &PluginsConfigInput) -> PluginLoadOutcome {
            PluginLoadOutcome::default()
        }

        pub fn plugin_skill_snapshots_for_config(
            &self,
            _config: &PluginsConfigInput,
        ) -> Option<SkillRootSnapshots<PluginSkillRoot>> {
            None
        }

        pub fn set_analytics_events_client(
            &self,
            _analytics_events_client: codex_analytics::AnalyticsEventsClient,
        ) {
        }

        pub(crate) async fn recommended_plugin_candidates_for_config(
            &self,
            input: RecommendedPluginCandidatesInput<'_>,
        ) -> Option<Vec<DiscoverableTool>> {
            let RecommendedPluginCandidatesInput {
                plugins_config,
                loaded_plugins,
                auth,
                disabled_tools,
                app_server_client_name,
            } = input;
            let _ = (
                plugins_config,
                loaded_plugins,
                auth,
                disabled_tools,
                app_server_client_name,
            );
            None
        }

        pub(crate) fn telemetry_metadata_for_capability_summary(
            &self,
            _summary: &PluginCapabilitySummary,
        ) -> Option<codex_plugin_types::PluginTelemetryMetadata> {
            None
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct PluginCommandAttribution {
        pub(crate) plugin_id: PluginId,
        pub(crate) normalized_relative_path: String,
    }

    impl PluginCommandAttribution {
        pub(crate) fn serialized_fields(&self) -> (String, String) {
            (
                self.plugin_id.as_key(),
                self.normalized_relative_path.clone(),
            )
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub(crate) struct TrustedPluginRoots;

    impl TrustedPluginRoots {
        pub(crate) fn from_plugin_load_outcome(
            _loaded_plugins: &PluginLoadOutcome,
            _codex_home: &Path,
        ) -> Self {
            Self
        }

        pub(crate) fn resolve_attribution(
            &self,
            _command: &[String],
            _cwd: &AbsolutePathBuf,
        ) -> Option<PluginCommandAttribution> {
            None
        }

        pub(crate) async fn resolve_executor_attribution(
            &self,
            _command: &[String],
            _cwd: &PathUri,
            _file_system: &dyn ExecutorFileSystem,
        ) -> Option<PluginCommandAttribution> {
            None
        }

        pub(crate) fn resolve_metrics_operation(
            &self,
            _command: &[String],
            _cwd: &AbsolutePathBuf,
        ) -> Option<ResolvedPluginMetricsOperation> {
            None
        }

        pub(crate) async fn resolve_metrics_operation_in_filesystem(
            &self,
            _command: &[String],
            _cwd: &PathUri,
            _file_system: &dyn ExecutorFileSystem,
        ) -> Option<ResolvedPluginMetricsOperation> {
            None
        }
    }

    pub(crate) struct ResolvedPluginMetricsOperation;

    pub(crate) struct PluginMetricsSidecar;

    impl Drop for PluginMetricsSidecar {
        fn drop(&mut self) {}
    }

    impl PluginMetricsSidecar {
        pub(crate) fn create(_resolved: ResolvedPluginMetricsOperation) -> Option<Self> {
            None
        }

        pub(crate) async fn create_remote(
            _environment: &Environment,
            _resolved: ResolvedPluginMetricsOperation,
        ) -> Option<Self> {
            None
        }

        pub(crate) fn install_output_env(&self, _env: &mut HashMap<String, String>) {}

        pub(crate) fn additional_permissions(&self) -> AdditionalPermissionProfile {
            AdditionalPermissionProfile::default()
        }

        pub(crate) async fn finish(self, _exit_code: i32) -> Option<PluginMeasurementBatch> {
            None
        }
    }

    pub(crate) struct PluginMeasurementBatch {
        pub(crate) plugin_id: String,
        pub(crate) execution_id: String,
        pub(crate) operation: String,
        pub(crate) rows: Vec<codex_analytics::PluginMeasurementRow>,
    }

    pub(crate) fn strip_output_env(env: &mut HashMap<String, String>) {
        if cfg!(windows) {
            env.retain(|key, _| !key.eq_ignore_ascii_case(PLUGIN_METRICS_OUTPUT_ENV_VAR));
        } else {
            env.remove(PLUGIN_METRICS_OUTPUT_ENV_VAR);
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct ArtifactOperation {
        pub(crate) plugin_name: &'static str,
        pub(crate) script_path: &'static str,
        pub(crate) artifact_type: &'static str,
        pub(crate) operation_kind: &'static str,
        pub(crate) expected_output_count: u32,
        pub(crate) output_format: &'static str,
    }

    pub(crate) fn recognize_artifact_operation(
        _attribution: Option<&PluginCommandAttribution>,
        _command: &[String],
    ) -> Option<ArtifactOperation> {
        None
    }

    pub(crate) fn executor_plugin_hook_sources(
        _snapshot: &ExecutorCapabilityDiscoverySnapshot,
    ) -> Vec<ExecutorPluginHookSource> {
        Vec::new()
    }
}

#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::PLUGIN_METRICS_OUTPUT_ENV_VAR;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::PluginCommandAttribution;
#[cfg(not(feature = "plugins"))]
pub use disabled::PluginLoadOutcome;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::PluginMetricsSidecar;
#[cfg(not(feature = "plugins"))]
pub use disabled::PluginsConfigInput;
#[cfg(not(feature = "plugins"))]
pub use disabled::PluginsManager;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::RecommendedPluginCandidatesInput;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::ResolvedPluginMetricsOperation;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::TrustedPluginRoots;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::executor_plugin_hook_sources;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::recognize_artifact_operation;
#[cfg(not(feature = "plugins"))]
pub(crate) use disabled::strip_output_env;
