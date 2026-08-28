use codex_config::ConfigLayerStack;

use crate::AppInfo;

pub use codex_config::AppToolPolicy;
pub use codex_config::AppToolPolicyInput;
pub use codex_config::app_is_enabled;
pub use codex_config::apps_config_from_layer_stack;

/// Connector-facing policy evaluator that projects config policy onto app metadata.
pub struct AppToolPolicyEvaluator<'a> {
    inner: codex_config::AppToolPolicyEvaluator<'a>,
}

impl<'a> AppToolPolicyEvaluator<'a> {
    pub fn new(config_layer_stack: &'a ConfigLayerStack) -> Self {
        Self {
            inner: codex_config::AppToolPolicyEvaluator::new(config_layer_stack),
        }
    }

    pub fn policy(&self, input: AppToolPolicyInput<'_>) -> AppToolPolicy {
        self.inner.policy(input)
    }

    /// Returns the effective local and managed enablement for one connector.
    pub fn app_enabled(&self, connector_id: &str) -> bool {
        self.inner.app_enabled(connector_id)
    }

    /// Applies app policy without overriding source state for unconfigured apps.
    pub fn apply_app_enabled_state(&self, mut apps: Vec<AppInfo>) -> Vec<AppInfo> {
        for app in &mut apps {
            if self.inner.has_app_enabled_policy(app.id.as_str()) {
                app.is_enabled = self.app_enabled(app.id.as_str());
            }
        }

        apps
    }
}

#[cfg(test)]
#[path = "app_tool_policy_tests.rs"]
mod tests;
