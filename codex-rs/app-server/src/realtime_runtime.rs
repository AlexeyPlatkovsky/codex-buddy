use codex_app_server_protocol::JSONRPCErrorError;

#[cfg(not(feature = "realtime"))]
use crate::error_code::invalid_request;

#[cfg(not(feature = "realtime"))]
const REALTIME_UNAVAILABLE_MESSAGE: &str = "realtime conversation is unavailable in this build";

/// Build-specific realtime availability injected into the v2 request processor.
#[derive(Clone, Copy, Default)]
pub(crate) struct RealtimeRuntime;

impl RealtimeRuntime {
    pub(crate) fn ensure_available(self) -> Result<(), JSONRPCErrorError> {
        #[cfg(feature = "realtime")]
        {
            Ok(())
        }
        #[cfg(not(feature = "realtime"))]
        {
            Err(invalid_request(REALTIME_UNAVAILABLE_MESSAGE))
        }
    }
}
