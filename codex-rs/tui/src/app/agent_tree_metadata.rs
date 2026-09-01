//! Presentation-only model, elapsed-time, and task-boundary state for the agent tree.
//!
//! This cache deliberately does not affect thread navigation or lifecycle ownership. It only
//! determines which locally known agents belong to the current root task and enriches their rows
//! with the model requested at spawn time and elapsed runtime.

use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort;
use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Default)]
pub(super) struct AgentTreeMetadataState {
    models: HashMap<ThreadId, AgentModel>,
    runtimes: HashMap<ThreadId, AgentRuntime>,
    visible_from_order: usize,
}

#[derive(Debug)]
struct AgentModel {
    model: String,
    effort: ReasoningEffort,
}

#[derive(Debug)]
struct AgentRuntime {
    started_at: Instant,
    finished_at: Option<Instant>,
}

impl AgentTreeMetadataState {
    /// Begins a new panel-only task scope without disturbing picker navigation or thread state.
    pub(super) fn begin_root_task(
        &mut self,
        order_len: usize,
        primary_thread_id: ThreadId,
        model: String,
        effort: ReasoningEffort,
    ) {
        self.visible_from_order = order_len;
        self.record_model(primary_thread_id, model, effort);
        self.start_runtime(primary_thread_id);
    }

    pub(super) fn record_model(
        &mut self,
        thread_id: ThreadId,
        model: String,
        effort: ReasoningEffort,
    ) {
        if !model.trim().is_empty() {
            self.models.insert(thread_id, AgentModel { model, effort });
        }
    }

    pub(super) fn start_runtime(&mut self, thread_id: ThreadId) {
        let now = Instant::now();
        match self.runtimes.get_mut(&thread_id) {
            Some(runtime) if runtime.finished_at.is_some() => {
                *runtime = AgentRuntime {
                    started_at: now,
                    finished_at: None,
                };
            }
            Some(_) => {}
            None => {
                self.runtimes.insert(
                    thread_id,
                    AgentRuntime {
                        started_at: now,
                        finished_at: None,
                    },
                );
            }
        }
    }

    pub(super) fn finish_runtime(&mut self, thread_id: ThreadId) {
        if let Some(runtime) = self.runtimes.get_mut(&thread_id) {
            runtime.finished_at.get_or_insert_with(Instant::now);
        }
    }

    pub(super) fn should_include(
        &self,
        index: usize,
        thread_id: ThreadId,
        primary_thread_id: Option<ThreadId>,
    ) -> bool {
        Some(thread_id) == primary_thread_id || index >= self.visible_from_order
    }

    pub(super) fn model_label(&self, thread_id: ThreadId) -> Option<String> {
        self.models
            .get(&thread_id)
            .map(|model| super::agent_tree::compact_agent_model_label(&model.model, &model.effort))
    }

    pub(super) fn elapsed(&self, thread_id: ThreadId) -> Option<Duration> {
        self.runtimes.get(&thread_id).map(|runtime| {
            runtime
                .finished_at
                .unwrap_or_else(Instant::now)
                .saturating_duration_since(runtime.started_at)
        })
    }

    pub(super) fn clear(&mut self) {
        self.models.clear();
        self.runtimes.clear();
        self.visible_from_order = 0;
    }

    pub(super) fn remove(&mut self, thread_id: ThreadId) {
        self.models.remove(&thread_id);
        self.runtimes.remove(&thread_id);
    }
}
