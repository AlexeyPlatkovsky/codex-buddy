use std::sync::Arc;

use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::ThreadQueueAddParams;
use codex_app_server_protocol::ThreadQueueAddResponse;
use codex_app_server_protocol::ThreadQueueDeleteParams;
use codex_app_server_protocol::ThreadQueueDeleteResponse;
use codex_app_server_protocol::ThreadQueueListParams;
use codex_app_server_protocol::ThreadQueueListResponse;
use codex_app_server_protocol::ThreadQueueReorderParams;
use codex_app_server_protocol::ThreadQueueReorderResponse;
use codex_app_server_protocol::ThreadQueueStartParams;
use codex_app_server_protocol::ThreadQueueStartResponse;
use codex_app_server_protocol::ThreadQueueUpdateParams;
use codex_app_server_protocol::ThreadQueueUpdateResponse;
use codex_core::ThreadManager;
use codex_thread_store::ThreadStore;

use crate::outgoing_message::ConnectionRequestId;
use crate::outgoing_message::OutgoingMessageSender;
use crate::queue_runtime::QueueRuntime;

pub(crate) struct ThreadQueueRequestProcessor;

impl ThreadQueueRequestProcessor {
    pub(crate) fn new(
        _thread_manager: Arc<ThreadManager>,
        _thread_store: Arc<dyn ThreadStore>,
        _outgoing: Arc<OutgoingMessageSender>,
        _runtime: QueueRuntime,
    ) -> Self {
        Self
    }

    pub(crate) async fn add(
        &self,
        _params: ThreadQueueAddParams,
    ) -> Result<ThreadQueueAddResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }

    pub(crate) async fn list(
        &self,
        _params: ThreadQueueListParams,
    ) -> Result<ThreadQueueListResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }

    pub(crate) async fn update(
        &self,
        _params: ThreadQueueUpdateParams,
    ) -> Result<ThreadQueueUpdateResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }

    pub(crate) async fn delete(
        &self,
        _params: ThreadQueueDeleteParams,
    ) -> Result<ThreadQueueDeleteResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }

    pub(crate) async fn reorder(
        &self,
        _params: ThreadQueueReorderParams,
    ) -> Result<ThreadQueueReorderResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }

    pub(crate) async fn start(
        &self,
        _request_id: &ConnectionRequestId,
        _params: ThreadQueueStartParams,
    ) -> Result<ThreadQueueStartResponse, JSONRPCErrorError> {
        Err(QueueRuntime::unavailable_error())
    }
}
