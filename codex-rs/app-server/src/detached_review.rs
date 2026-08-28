use std::sync::Arc;
use std::sync::Weak;

use codex_app_server_protocol::JSONRPCErrorError;
use codex_core::CodexThread;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_protocol::ThreadId;
use codex_protocol::protocol::W3cTraceContext;

#[cfg(feature = "detached-review")]
use crate::error_code::internal_error;
use crate::error_code::invalid_request;

const DETACHED_REVIEW_UNAVAILABLE_MESSAGE: &str = "detached review is unavailable in this build";

#[cfg_attr(not(feature = "detached-review"), allow(dead_code))]
pub(crate) struct DetachedReviewInvocation {
    pub(crate) config: Config,
    pub(crate) prompt: String,
    pub(crate) parent_trace: Option<W3cTraceContext>,
}

pub(crate) struct DetachedReviewRun {
    pub(crate) thread_id: ThreadId,
    pub(crate) turn_id: String,
    pub(crate) thread: Arc<CodexThread>,
}

/// Build-specific detached-review implementation injected into the turn processor.
#[derive(Clone)]
pub(crate) struct DetachedReviewRunner {
    #[cfg(feature = "detached-review")]
    runner: codex_agent_extension::AgentRunner,
}

impl DetachedReviewRunner {
    pub(crate) fn new(thread_manager: Weak<ThreadManager>) -> Self {
        #[cfg(feature = "detached-review")]
        {
            Self {
                runner: codex_agent_extension::AgentRunner::new(thread_manager),
            }
        }
        #[cfg(not(feature = "detached-review"))]
        {
            let _ = thread_manager;
            Self {}
        }
    }

    pub(crate) fn ensure_available(&self) -> Result<(), JSONRPCErrorError> {
        if cfg!(feature = "detached-review") {
            Ok(())
        } else {
            Err(invalid_request(DETACHED_REVIEW_UNAVAILABLE_MESSAGE))
        }
    }

    pub(crate) async fn start(
        &self,
        parent_thread_id: ThreadId,
        invocation: DetachedReviewInvocation,
    ) -> Result<DetachedReviewRun, JSONRPCErrorError> {
        #[cfg(feature = "detached-review")]
        {
            let DetachedReviewInvocation {
                config,
                prompt,
                parent_trace,
            } = invocation;
            let run = self
                .runner
                .start(
                    parent_thread_id,
                    codex_agent_extension::AgentInvocation {
                        config,
                        prompt,
                        parent_trace,
                    },
                )
                .await
                .map_err(|err| internal_error(format!("failed to start detached review: {err}")))?;
            Ok(DetachedReviewRun {
                thread_id: run.thread_id,
                turn_id: run.turn_id,
                thread: run.thread,
            })
        }
        #[cfg(not(feature = "detached-review"))]
        {
            let _ = (parent_thread_id, invocation);
            Err(invalid_request(DETACHED_REVIEW_UNAVAILABLE_MESSAGE))
        }
    }
}
