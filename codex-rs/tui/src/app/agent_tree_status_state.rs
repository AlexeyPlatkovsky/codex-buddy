//! Bounded lifecycle cache for the future sub-agent tree.
//!
//! The cache is intentionally owned by [`super::agent_navigation::AgentNavigationState`]: it
//! follows the same thread identity lifecycle as picker metadata but keeps presentation status
//! separate from picker liveness. Event consumers update this state directly, without polling or
//! adding app-server protocol surface.

use super::agent_tree::AgentTreeLifecycleEvent;
use super::agent_tree::AgentTreeStatus;
use codex_protocol::ThreadId;
use std::collections::HashMap;
use std::collections::VecDeque;

/// Bounds retained lifecycle state even if a long-running session sees many agent threads.
pub(super) const MAX_AGENT_TREE_STATUS_ENTRIES: usize = 1_024;

#[derive(Debug, Default)]
pub(super) struct AgentTreeStatusState {
    statuses: HashMap<ThreadId, AgentTreeStatus>,
    insertion_order: VecDeque<ThreadId>,
}

impl AgentTreeStatusState {
    pub(super) fn initialize(&mut self, thread_id: ThreadId, initial_status: AgentTreeStatus) {
        self.insert_if_missing(thread_id, initial_status);
    }

    pub(super) fn observe(
        &mut self,
        thread_id: ThreadId,
        initial_status: AgentTreeStatus,
        event: AgentTreeLifecycleEvent,
    ) {
        self.insert_if_missing(thread_id, initial_status);
        if let Some(status) = self.statuses.get_mut(&thread_id) {
            *status = status.transition(event);
        }
    }

    pub(super) fn get(&self, thread_id: &ThreadId) -> Option<AgentTreeStatus> {
        self.statuses.get(thread_id).copied()
    }

    pub(super) fn remove(&mut self, thread_id: ThreadId) {
        if self.statuses.remove(&thread_id).is_some() {
            self.insertion_order
                .retain(|candidate| *candidate != thread_id);
        }
    }

    pub(super) fn clear(&mut self) {
        self.statuses.clear();
        self.insertion_order.clear();
    }

    fn insert_if_missing(&mut self, thread_id: ThreadId, status: AgentTreeStatus) {
        if !self.statuses.contains_key(&thread_id) {
            if self.statuses.len() == MAX_AGENT_TREE_STATUS_ENTRIES
                && let Some(evicted_thread_id) = self.insertion_order.pop_front()
            {
                self.statuses.remove(&evicted_thread_id);
            }
            self.statuses.insert(thread_id, status);
            self.insertion_order.push_back(thread_id);
        }
    }
}

#[cfg(test)]
#[path = "agent_tree_status_state_tests.rs"]
mod tests;
