use codex_protocol::protocol::EventMsg;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RealtimeEventOrder {
    BeforeEvent,
    #[default]
    AfterEvent,
}

#[derive(Debug, Default)]
pub(crate) struct RealtimeEventEffects {
    pub(crate) order: RealtimeEventOrder,
}

/// No-op state retained so non-realtime builds preserve the common session event path.
#[derive(Default)]
pub(crate) struct RealtimeHistoryState;

impl RealtimeHistoryState {
    pub(crate) fn should_observe(&self, _event: &EventMsg) -> bool {
        false
    }

    pub(crate) fn observe(&mut self, _event: &EventMsg) -> RealtimeEventEffects {
        RealtimeEventEffects::default()
    }
}
