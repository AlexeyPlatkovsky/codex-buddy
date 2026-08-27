//! Bounded viewport state for the sub-agent tree.
//!
//! This module intentionally contains no rendering code. It turns a complete tree snapshot into
//! a stable visible slice while retaining selection by thread ID as snapshots change.

use super::agent_tree::AgentTreeRow;
use super::agent_tree::AgentTreeSnapshot;
use super::agent_tree::MAX_AGENT_TREE_ROWS;
use codex_protocol::ThreadId;

/// A visible slice of an agent tree and its navigation indicators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AgentTreeVisibleRows<'a> {
    pub(crate) rows: &'a [AgentTreeRow],
    pub(crate) above: bool,
    pub(crate) below: bool,
    pub(crate) truncated: bool,
}

/// Bounded scrolling and selection state for an [`AgentTreeSnapshot`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AgentTreeViewport {
    offset: usize,
    height: usize,
    selected_thread_id: Option<ThreadId>,
}

impl AgentTreeViewport {
    /// Creates an empty viewport. The height is zero until the first layout measurement.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Updates the available row height and keeps the selected row visible.
    pub(crate) fn resize(&mut self, height: usize, snapshot: &AgentTreeSnapshot) {
        self.height = height.min(MAX_AGENT_TREE_ROWS);
        self.reconcile(snapshot, None);
    }

    /// Applies a new snapshot, preserving selection by thread ID when possible.
    ///
    /// `preferred_selection` is used when the caller explicitly selected a different thread. If
    /// it is absent or stale, the previous selection is retained; current or first-row fallback
    /// makes removal of the selected node deterministic.
    pub(crate) fn update(
        &mut self,
        snapshot: &AgentTreeSnapshot,
        preferred_selection: Option<ThreadId>,
    ) {
        self.reconcile(snapshot, preferred_selection);
    }

    /// Returns only the bounded visible slice and navigation indicators.
    pub(crate) fn visible<'a>(&self, snapshot: &'a AgentTreeSnapshot) -> AgentTreeVisibleRows<'a> {
        let start = self.offset.min(snapshot.rows.len());
        let end = start.saturating_add(self.height).min(snapshot.rows.len());
        AgentTreeVisibleRows {
            rows: &snapshot.rows[start..end],
            above: start > 0,
            below: end < snapshot.rows.len(),
            truncated: snapshot.truncated,
        }
    }

    pub(crate) fn selected_thread_id(&self) -> Option<ThreadId> {
        self.selected_thread_id
    }

    fn reconcile(&mut self, snapshot: &AgentTreeSnapshot, preferred_selection: Option<ThreadId>) {
        if snapshot.rows.is_empty() {
            self.selected_thread_id = None;
            self.offset = 0;
            return;
        }
        let selected = preferred_selection
            .filter(|id| snapshot.rows.iter().any(|row| row.thread_id == *id))
            .or_else(|| {
                self.selected_thread_id
                    .filter(|id| snapshot.rows.iter().any(|row| row.thread_id == *id))
            })
            .or_else(|| {
                snapshot
                    .rows
                    .iter()
                    .find(|row| row.is_current)
                    .or(snapshot.rows.first())
                    .map(|row| row.thread_id)
            });
        self.selected_thread_id = selected;
        self.ensure_visible(
            self.selected_index(snapshot).unwrap_or(0),
            snapshot.rows.len(),
        );
    }

    fn selected_index(&self, snapshot: &AgentTreeSnapshot) -> Option<usize> {
        self.selected_thread_id
            .and_then(|id| snapshot.rows.iter().position(|row| row.thread_id == id))
    }

    fn ensure_visible(&mut self, selected: usize, row_count: usize) {
        if self.height == 0 {
            self.offset = self.offset.min(row_count);
            return;
        }
        if selected < self.offset {
            self.offset = selected;
        } else if selected >= self.offset.saturating_add(self.height) {
            self.offset = selected + 1 - self.height;
        }
        self.offset = self.offset.min(row_count.saturating_sub(self.height));
    }
}

#[cfg(test)]
#[path = "agent_tree_viewport_tests.rs"]
mod tests;
