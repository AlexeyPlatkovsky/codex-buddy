use std::path::PathBuf;
use std::sync::Arc;

use codex_code_mode_types::CodeModeSessionDelegate;
use codex_code_mode_types::CodeModeSessionProvider;
use codex_code_mode_types::CodeModeSessionProviderFuture;

use crate::config::Config;

pub(crate) const CODE_MODE_UNAVAILABLE_ERROR: &str = "code mode is unavailable in this build";

struct UnavailableCodeModeSessionProvider;

impl CodeModeSessionProvider for UnavailableCodeModeSessionProvider {
    fn availability(&self) -> Result<(), String> {
        Err(CODE_MODE_UNAVAILABLE_ERROR.to_string())
    }

    fn create_session<'a>(
        &'a self,
        _delegate: Arc<dyn CodeModeSessionDelegate>,
    ) -> CodeModeSessionProviderFuture<'a> {
        Box::pin(async { Err(CODE_MODE_UNAVAILABLE_ERROR.to_string()) })
    }
}

pub(crate) fn unavailable_provider() -> Arc<dyn CodeModeSessionProvider> {
    Arc::new(UnavailableCodeModeSessionProvider)
}

#[cfg(feature = "code-mode")]
pub(crate) fn default_provider(config: &Config) -> Arc<dyn CodeModeSessionProvider> {
    if config
        .features
        .enabled(codex_features::Feature::CodeModeHost)
        || config.code_mode.disable_in_process_fallback
    {
        Arc::new(codex_code_mode::ProcessOwnedCodeModeSessionProvider::default())
    } else {
        Arc::new(codex_code_mode::DisabledCodeModeSessionProvider)
    }
}

#[cfg(not(feature = "code-mode"))]
pub(crate) fn default_provider(_config: &Config) -> Arc<dyn CodeModeSessionProvider> {
    unavailable_provider()
}

#[cfg(feature = "code-mode")]
pub(crate) fn provider_with_host_program(
    host_program: PathBuf,
) -> Arc<dyn CodeModeSessionProvider> {
    Arc::new(codex_code_mode::ProcessOwnedCodeModeSessionProvider::with_host_program(host_program))
}

#[cfg(not(feature = "code-mode"))]
pub(crate) fn provider_with_host_program(
    _host_program: PathBuf,
) -> Arc<dyn CodeModeSessionProvider> {
    unavailable_provider()
}
