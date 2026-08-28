use std::sync::Arc;
use std::sync::Weak;

use codex_app_server_protocol::JSONRPCErrorError;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_extension_api::ExtensionEventSink;
use codex_extension_api::ExtensionRegistryBuilder;
#[cfg(feature = "queue")]
use codex_queue_extension::QueuedItemService;
use codex_thread_store::QueueStore;

use crate::error_code::invalid_request;

const QUEUE_UNAVAILABLE_MESSAGE: &str = "user message queue is unavailable";

/// Build-specific queue implementation injected into app-server request and extension plumbing.
#[derive(Clone, Default)]
pub(crate) struct QueueRuntime {
    #[cfg(feature = "queue")]
    service: Option<Arc<QueuedItemService>>,
}

impl QueueRuntime {
    pub(crate) fn new(
        queue_store: Option<Arc<dyn QueueStore>>,
        thread_manager: Weak<ThreadManager>,
        event_sink: Arc<dyn ExtensionEventSink>,
    ) -> Self {
        #[cfg(feature = "queue")]
        {
            Self {
                service: queue_store.map(|queue_store| {
                    Arc::new(QueuedItemService::new(
                        queue_store,
                        thread_manager,
                        event_sink,
                    ))
                }),
            }
        }
        #[cfg(not(feature = "queue"))]
        {
            let _ = (queue_store, thread_manager, event_sink);
            Self {}
        }
    }

    pub(crate) fn install(&self, builder: &mut ExtensionRegistryBuilder<Config>) {
        #[cfg(feature = "queue")]
        if let Some(service) = &self.service {
            codex_queue_extension::install(builder, Arc::clone(service));
        }
        #[cfg(not(feature = "queue"))]
        let _ = builder;
    }

    #[cfg(feature = "queue")]
    pub(crate) fn service(&self) -> Option<&QueuedItemService> {
        self.service.as_deref()
    }

    pub(crate) fn unavailable_error() -> JSONRPCErrorError {
        invalid_request(QUEUE_UNAVAILABLE_MESSAGE)
    }
}
