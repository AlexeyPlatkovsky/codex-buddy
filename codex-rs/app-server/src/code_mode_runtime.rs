use std::io::ErrorKind;
use std::io::Result as IoResult;
use std::sync::Arc;

use codex_code_mode_types::CodeModeSessionProvider;
use codex_core::config::Config;

use crate::CodeModeHostTransport;

pub(crate) fn preflight_transport(transport: &CodeModeHostTransport) -> IoResult<()> {
    #[cfg(feature = "code-mode")]
    {
        let _ = transport;
        Ok(())
    }

    #[cfg(not(feature = "code-mode"))]
    match transport {
        CodeModeHostTransport::Local => Ok(()),
        CodeModeHostTransport::Grpc(_) => Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "code mode is unavailable in this build",
        )),
    }
}

pub(crate) fn session_provider(
    transport: &CodeModeHostTransport,
    config: &Config,
) -> IoResult<Option<Arc<dyn CodeModeSessionProvider>>> {
    match transport {
        CodeModeHostTransport::Local => Ok(None),
        CodeModeHostTransport::Grpc(url) => grpc_session_provider(url, config),
    }
}

#[cfg(feature = "code-mode")]
fn grpc_session_provider(
    url: &url::Url,
    config: &Config,
) -> IoResult<Option<Arc<dyn CodeModeSessionProvider>>> {
    if !config
        .features
        .enabled(codex_features::Feature::CodeModeHost)
    {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "remote code-mode host requires the code_mode_host feature to be enabled",
        ));
    }

    Ok(Some(Arc::new(
        codex_code_mode::GrpcCodeModeSessionProvider::with_http_client_factory(
            url.to_string(),
            config.http_client_factory(),
        ),
    )))
}

#[cfg(test)]
#[path = "code_mode_runtime_tests.rs"]
mod tests;

#[cfg(not(feature = "code-mode"))]
fn grpc_session_provider(
    _url: &url::Url,
    _config: &Config,
) -> IoResult<Option<Arc<dyn CodeModeSessionProvider>>> {
    Err(std::io::Error::new(
        ErrorKind::InvalidInput,
        "code mode is unavailable in this build",
    ))
}
