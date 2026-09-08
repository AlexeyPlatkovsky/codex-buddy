//! Adapts existing config loaders for blocking plugin-upgrade workers.
//! Reloads must use the same normal or fallback entry point as the initial load.

#[cfg(feature = "connectors")]
use crate::config_manager::ConfigManager;
#[cfg(feature = "connectors")]
use codex_core_plugins::ConfigLayerReload;
#[cfg(feature = "connectors")]
use codex_utils_absolute_path::AbsolutePathBuf;
#[cfg(feature = "connectors")]
use std::sync::Arc;

/// The config-loading path used to select marketplaces for startup tasks.
pub(crate) enum PluginStartupConfig {
    Current,
    Defaults,
}

#[cfg(feature = "connectors")]
pub(crate) fn for_cwd(manager: ConfigManager, cwd: AbsolutePathBuf) -> ConfigLayerReload {
    let runtime = tokio::runtime::Handle::current();
    Arc::new(move || runtime.block_on(manager.load_config_layers_for_cwd(cwd.clone())))
}

#[cfg(feature = "connectors")]
pub(crate) fn defaults(manager: ConfigManager) -> ConfigLayerReload {
    let runtime = tokio::runtime::Handle::current();
    Arc::new(move || {
        runtime
            .block_on(manager.load_default_config())
            .map(|config| config.config_layer_stack)
    })
}
