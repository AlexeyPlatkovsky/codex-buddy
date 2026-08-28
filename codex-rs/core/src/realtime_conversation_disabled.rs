use crate::session::session::Session;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::models::MessagePhase;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::ConversationAudioParams;
use codex_protocol::protocol::ConversationSpeechParams;
use codex_protocol::protocol::ConversationStartParams;
use codex_protocol::protocol::ConversationTextParams;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use std::sync::Arc;

const REALTIME_UNAVAILABLE_MESSAGE: &str = "realtime conversations are unavailable in this runtime";

pub(crate) struct RealtimeConversationManager;

#[derive(Clone, Debug)]
pub(crate) struct RealtimeModeInstructions {
    pub(crate) start: Option<String>,
    pub(crate) end: Option<String>,
}

impl RealtimeConversationManager {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) async fn mode_instructions(&self) -> Option<RealtimeModeInstructions> {
        None
    }

    pub(crate) async fn running_state(&self) -> Option<()> {
        None
    }

    pub(crate) async fn register_handoff_stream_item(
        &self,
        _item_id: String,
        _phase: Option<MessagePhase>,
        _initial_text: String,
    ) {
    }

    pub(crate) async fn stream_handoff_delta(
        &self,
        _item_id: &str,
        _delta: String,
    ) -> CodexResult<()> {
        Ok(())
    }

    pub(crate) async fn finish_handoff_stream_item(&self, _item_id: &str) -> bool {
        false
    }

    pub(crate) async fn handoff_out(
        &self,
        _output_text: String,
        _phase: Option<MessagePhase>,
    ) -> CodexResult<()> {
        Ok(())
    }

    pub(crate) async fn handoff_complete(&self) -> CodexResult<()> {
        Ok(())
    }

    pub(crate) async fn clear_active_handoff(&self) {}

    pub(crate) async fn shutdown(&self) -> CodexResult<()> {
        Ok(())
    }
}

pub(crate) async fn handle_start(
    _sess: &Arc<Session>,
    _sub_id: String,
    _params: ConversationStartParams,
) -> CodexResult<()> {
    Err(realtime_unavailable())
}

pub(crate) async fn handle_audio(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationAudioParams,
) {
    send_realtime_unavailable(sess, sub_id).await;
}

pub(crate) async fn handle_text(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationTextParams,
) {
    send_realtime_unavailable(sess, sub_id).await;
}

pub(crate) async fn handle_speech(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationSpeechParams,
) {
    send_realtime_unavailable(sess, sub_id).await;
}

pub(crate) async fn handle_close(sess: &Arc<Session>, sub_id: String) {
    send_realtime_unavailable(sess, sub_id).await;
}

pub(crate) async fn send_realtime_unavailable(sess: &Session, sub_id: String) {
    sess.send_event_raw(Event {
        id: sub_id,
        msg: EventMsg::Error(ErrorEvent {
            misalignment: None,
            message: REALTIME_UNAVAILABLE_MESSAGE.to_string(),
            codex_error_info: Some(CodexErrorInfo::BadRequest),
        }),
    })
    .await;
}

fn realtime_unavailable() -> CodexErr {
    CodexErr::InvalidRequest(REALTIME_UNAVAILABLE_MESSAGE.to_string())
}
