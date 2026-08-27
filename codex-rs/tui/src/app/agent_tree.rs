//! Pure projection of cached sub-agent metadata into a stable tree.
//!
//! The tree is deliberately independent from rendering and app-server events. Callers provide
//! entries in the navigation cache's first-seen order; this module only resolves parentage and
//! produces a bounded preorder snapshot for a future view.

use crate::multi_agents::AgentPickerThreadEntry;
use crate::multi_agents::format_agent_picker_item_name;
use codex_protocol::ThreadId;
use std::collections::HashMap;
use std::collections::HashSet;

#[path = "agent_tree_status.rs"]
mod agent_tree_status;

pub(crate) use agent_tree_status::AgentTreeLifecycleEvent;
pub(crate) use agent_tree_status::AgentTreeStatus;

/// Upper bound for a single presentation snapshot. The navigation cache may retain more history,
/// but a malformed or unusually large cache must not cause a render-time allocation to grow
/// without bound.
pub(crate) const MAX_AGENT_TREE_ROWS: usize = 1_024;

/// Cached metadata needed to derive one tree node.
///
/// `parent_thread_id` is optional because older activity and some resume paths only have an
/// agent path. `AgentTreeInput::from_picker_entry` lets callers reuse the existing navigation
/// cache while supplying the parent relation when it is available from the loaded thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentTreeInput {
    pub(crate) thread_id: ThreadId,
    pub(crate) parent_thread_id: Option<ThreadId>,
    pub(crate) agent_path: Option<String>,
    pub(crate) agent_nickname: Option<String>,
    pub(crate) agent_role: Option<String>,
    pub(crate) status: AgentTreeStatus,
}

impl AgentTreeInput {
    pub(crate) fn from_picker_entry(
        thread_id: ThreadId,
        entry: &AgentPickerThreadEntry,
        parent_thread_id: Option<ThreadId>,
    ) -> Self {
        Self {
            thread_id,
            parent_thread_id,
            agent_path: entry.agent_path.clone(),
            agent_nickname: entry.agent_nickname.clone(),
            agent_role: entry.agent_role.clone(),
            status: AgentTreeStatus::from_picker_metadata(entry.is_running, entry.is_closed),
        }
    }

    /// Replaces the picker-derived lifecycle state with a richer observation from the caller.
    pub(crate) fn with_status(mut self, status: AgentTreeStatus) -> Self {
        self.status = status;
        self
    }

    /// Applies a lifecycle observation to cached metadata before it is projected into a row.
    pub(crate) fn apply_status_event(&mut self, event: AgentTreeLifecycleEvent) {
        self.status = self.status.transition(event);
    }
}

/// One row in [`AgentTreeSnapshot`], ordered for preorder display.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentTreeRow {
    pub(crate) thread_id: ThreadId,
    pub(crate) parent_thread_id: Option<ThreadId>,
    pub(crate) depth: usize,
    pub(crate) agent_path: Option<String>,
    pub(crate) label: String,
    pub(crate) status: AgentTreeStatus,
    pub(crate) is_current: bool,
    pub(crate) is_selected: bool,
}

/// Bounded, immutable presentation state for a sub-agent tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AgentTreeSnapshot {
    pub(crate) rows: Vec<AgentTreeRow>,
    /// True when input contained more entries than this snapshot could retain.
    pub(crate) truncated: bool,
}

impl AgentTreeSnapshot {
    /// Projects cached entries into stable spawn-order preorder.
    ///
    /// Sibling order follows the input order, which is the navigation cache's first-seen spawn
    /// order. An exact parent path wins over a thread-id relation; the latter is used when a path
    /// is absent, incomplete, or has no corresponding cached ancestor. Unknown parents become
    /// roots so teardown and resume races cannot hide a row or panic the view.
    pub(crate) fn from_inputs(
        inputs: impl IntoIterator<Item = AgentTreeInput>,
        primary_thread_id: Option<ThreadId>,
        selected_thread_id: Option<ThreadId>,
        current_thread_id: Option<ThreadId>,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut seen_thread_ids = HashSet::new();
        let mut truncated = false;
        for input in inputs {
            if nodes.len() == MAX_AGENT_TREE_ROWS {
                truncated = true;
                break;
            }
            // A duplicate can occur when a refresh races a close event. Keep the first-seen
            // position and metadata, matching AgentNavigationState's stable ordering contract.
            if !seen_thread_ids.insert(input.thread_id) {
                continue;
            }
            nodes.push(input);
        }

        let index_by_id: HashMap<ThreadId, usize> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.thread_id, index))
            .collect();
        let index_by_path: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let path = node.agent_path.as_deref()?.trim();
                (!path.is_empty()).then_some((path, index))
            })
            .fold(HashMap::new(), |mut paths, (path, index)| {
                // Duplicate paths are resolved to the earliest observed entry.
                paths.entry(path).or_insert(index);
                paths
            });

        let mut parents = vec![None; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            let path_parent = node
                .agent_path
                .as_deref()
                .and_then(path_parent)
                .and_then(|parent_path| index_by_path.get(parent_path).copied())
                .filter(|parent_index| *parent_index != index);
            let id_parent = node
                .parent_thread_id
                .and_then(|parent| index_by_id.get(&parent).copied())
                .filter(|parent_index| *parent_index != index);
            parents[index] = path_parent.or(id_parent);
        }

        // Parent IDs come from external state and may be stale or cyclic. Promote every member
        // of a cycle to a root; this keeps preorder total and deterministic.
        let mut cyclic = vec![false; parents.len()];
        for index in 0..parents.len() {
            let mut positions = HashMap::new();
            let mut path = Vec::new();
            let mut cursor = Some(index);
            while let Some(current) = cursor {
                if let Some(cycle_start) = positions.get(&current).copied() {
                    for cycle_member in &path[cycle_start..] {
                        cyclic[*cycle_member] = true;
                    }
                    break;
                }
                positions.insert(current, path.len());
                path.push(current);
                cursor = parents[current];
            }
        }
        for (index, is_cyclic) in cyclic.into_iter().enumerate() {
            if is_cyclic {
                parents[index] = None;
            }
        }

        let mut children = vec![Vec::new(); nodes.len()];
        let mut roots = Vec::new();
        for (index, parent) in parents.iter().enumerate() {
            if let Some(parent) = parent {
                children[*parent].push(index);
            } else {
                roots.push(index);
            }
        }

        let mut order = Vec::with_capacity(nodes.len());
        let mut stack = roots
            .into_iter()
            .rev()
            .map(|index| (index, 0usize))
            .collect::<Vec<_>>();
        while let Some((index, depth)) = stack.pop() {
            order.push((index, depth));
            for child in children[index].iter().rev() {
                stack.push((*child, depth.saturating_add(1)));
            }
        }

        let rows = order
            .into_iter()
            .map(|(index, depth)| {
                let input = &nodes[index];
                AgentTreeRow {
                    thread_id: input.thread_id,
                    parent_thread_id: parents[index].map(|parent| nodes[parent].thread_id),
                    depth,
                    agent_path: input.agent_path.clone(),
                    label: format_agent_picker_item_name(
                        input.agent_nickname.as_deref(),
                        input.agent_role.as_deref(),
                        primary_thread_id == Some(input.thread_id),
                    ),
                    status: input.status,
                    is_current: current_thread_id == Some(input.thread_id),
                    is_selected: selected_thread_id == Some(input.thread_id),
                }
            })
            .collect();

        Self { rows, truncated }
    }
}

/// Returns the exact path of an agent's parent, if the path has at least two components.
fn path_parent(path: &str) -> Option<&str> {
    let path = path.trim().trim_end_matches('/');
    let slash = path.rfind('/')?;
    (slash > 0).then_some(&path[..slash])
}

#[cfg(test)]
#[path = "agent_tree_tests.rs"]
mod tests;
