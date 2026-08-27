use super::*;
use crate::app::agent_tree::AgentTreeInput;
use crate::app::agent_tree::AgentTreeStatus;
use pretty_assertions::assert_eq;

fn snapshot(count: usize) -> AgentTreeSnapshot {
    AgentTreeSnapshot::from_inputs(
        (0..count).map(|_| AgentTreeInput {
            thread_id: ThreadId::new(),
            parent_thread_id: None,
            agent_path: None,
            agent_nickname: None,
            agent_role: Some("worker".to_string()),
            status: AgentTreeStatus::Waiting,
        }),
        None,
        None,
        None,
    )
}

fn snapshot_with_ids(ids: &[ThreadId], current: Option<ThreadId>) -> AgentTreeSnapshot {
    AgentTreeSnapshot::from_inputs(
        ids.iter().copied().map(|thread_id| AgentTreeInput {
            thread_id,
            parent_thread_id: None,
            agent_path: None,
            agent_nickname: None,
            agent_role: Some("worker".to_string()),
            status: AgentTreeStatus::Waiting,
        }),
        None,
        None,
        current,
    )
}

#[test]
fn large_tree_exposes_only_visible_rows_and_indicators() {
    let tree = snapshot(1_024);
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(5, &tree);
    viewport.update(&tree, Some(tree.rows[900].thread_id));
    let visible = viewport.visible(&tree);

    assert_eq!(visible.rows.len(), 5);
    assert!(visible.above);
    assert!(visible.below);
    assert!(!visible.truncated);
}

#[test]
fn selection_is_preserved_by_thread_id_when_rows_reorder() {
    let first = ThreadId::new();
    let selected = ThreadId::new();
    let third = ThreadId::new();
    let before = snapshot_with_ids(&[first, selected, third], Some(first));
    let after = snapshot_with_ids(&[third, selected, first], Some(third));
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(2, &before);
    viewport.update(&before, Some(selected));
    viewport.update(&after, None);

    assert_eq!(viewport.selected_thread_id(), Some(selected));
    assert_eq!(viewport.visible(&after).rows[1].thread_id, selected);
}

#[test]
fn resize_clamps_offset_and_keeps_selection_visible() {
    let tree = snapshot(20);
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(3, &tree);
    viewport.update(&tree, Some(tree.rows[15].thread_id));
    viewport.resize(50, &tree);
    let visible = viewport.visible(&tree);

    assert_eq!(visible.rows.len(), 20);
    assert!(!visible.above);
    assert!(!visible.below);
}

#[test]
fn removing_selected_row_falls_back_to_current_then_first() {
    let first = ThreadId::new();
    let selected = ThreadId::new();
    let current = ThreadId::new();
    let before = snapshot_with_ids(&[first, selected, current], Some(current));
    let after = snapshot_with_ids(&[first, current], Some(current));
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(2, &before);
    viewport.update(&before, Some(selected));
    viewport.update(&after, None);

    assert_eq!(viewport.selected_thread_id(), Some(current));
}

#[test]
fn empty_tree_resets_state_and_truncation_is_forwarded() {
    let tree = snapshot(MAX_AGENT_TREE_ROWS + 1);
    assert!(tree.truncated);
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(3, &tree);
    assert!(viewport.visible(&tree).truncated);
    viewport.update(&AgentTreeSnapshot::default(), None);
    assert_eq!(viewport.visible(&AgentTreeSnapshot::default()).rows, &[]);
    assert_eq!(viewport.selected_thread_id(), None);
}

#[test]
fn actual_short_frame_height_reconciles_selection_and_scroll_markers() {
    let tree = snapshot(8);
    let selected_thread_id = tree.rows[6].thread_id;
    let mut viewport = AgentTreeViewport::new();
    // The terminal may choose a shorter frame than the chat widget's desired height. Use the
    // actual panel rectangle rather than the earlier desired-height estimate.
    let actual_panel = super::super::agent_tree_panel::layout_agent_tree_panel(
        ratatui::layout::Rect::new(
            /*x*/ 0, /*y*/ 0, /*width*/ 120, /*height*/ 4,
        ),
        /*has_subagents*/ true,
    )
    .panel_area
    .expect("wide frame has a panel");
    viewport.resize(usize::from(actual_panel.height.saturating_sub(1)), &tree);
    viewport.update(&tree, Some(selected_thread_id));
    let visible = viewport.visible(&tree);

    assert_eq!(visible.rows.len(), 3);
    assert_eq!(
        visible.rows.last().map(|row| row.thread_id),
        Some(selected_thread_id)
    );
    assert!(visible.above);
    assert!(visible.below);
}
