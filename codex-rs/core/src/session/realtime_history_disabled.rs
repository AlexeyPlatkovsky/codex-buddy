use codex_protocol::error::Result as CodexResult;

use crate::realtime_history::RealtimeEventEffects;
use crate::session::session::Session;

impl Session {
    pub(super) async fn send_realtime_history_effects(
        &self,
        _sub_id: &str,
        _effects: RealtimeEventEffects,
    ) -> CodexResult<()> {
        Ok(())
    }
}
