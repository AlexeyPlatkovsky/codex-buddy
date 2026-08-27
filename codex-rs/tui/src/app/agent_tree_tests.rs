use super::*;
use pretty_assertions::assert_eq;

fn input(
    thread_id: ThreadId,
    parent_thread_id: Option<ThreadId>,
    agent_path: Option<&str>,
    nickname: Option<&str>,
    closed: bool,
) -> AgentTreeInput {
    AgentTreeInput {
        thread_id,
        parent_thread_id,
        agent_path: agent_path.map(str::to_string),
        agent_nickname: nickname.map(str::to_string),
        agent_role: Some("worker".to_string()),
        is_running: !closed,
        is_closed: closed,
    }
}

fn ids(snapshot: &AgentTreeSnapshot) -> Vec<ThreadId> {
    snapshot.rows.iter().map(|row| row.thread_id).collect()
}

#[test]
fn path_hierarchy_is_preorder_and_siblings_keep_spawn_order() {
    let root = ThreadId::from_string("00000000-0000-0000-0000-000000000001").unwrap();
    let first = ThreadId::from_string("00000000-0000-0000-0000-000000000002").unwrap();
    let second = ThreadId::from_string("00000000-0000-0000-0000-000000000003").unwrap();
    let nested = ThreadId::from_string("00000000-0000-0000-0000-000000000004").unwrap();

    // The child is observed before its parent, but hierarchy still requires a parent-before-child
    // preorder. The two siblings retain their first-seen order.
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![
            input(root, None, Some("/root"), None, false),
            input(
                second,
                Some(root),
                Some("/root/second"),
                Some("Second"),
                false,
            ),
            input(first, Some(root), Some("/root/first"), Some("First"), false),
            input(
                nested,
                Some(first),
                Some("/root/first/nested"),
                Some("Nested"),
                false,
            ),
        ],
        Some(root),
        None,
        Some(root),
    );

    assert_eq!(ids(&snapshot), vec![root, second, first, nested]);
    assert_eq!(snapshot.rows[0].depth, 0);
    assert_eq!(snapshot.rows[1].depth, 1);
    assert_eq!(snapshot.rows[2].depth, 1);
    assert_eq!(snapshot.rows[3].depth, 2);
    assert_eq!(snapshot.rows[3].parent_thread_id, Some(first));
    assert_eq!(snapshot.rows[0].label, "Main [default]");
}

#[test]
fn parent_thread_id_fills_missing_or_unmatched_path_parent() {
    let root = ThreadId::new();
    let child = ThreadId::new();
    let grandchild = ThreadId::new();
    let unknown = ThreadId::new();
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![
            input(root, None, None, None, false),
            input(child, Some(root), None, Some("Child"), false),
            input(
                grandchild,
                Some(child),
                Some("/root/child/grandchild"),
                Some("Grand"),
                false,
            ),
            input(
                unknown,
                Some(ThreadId::new()),
                None,
                Some("Detached"),
                false,
            ),
        ],
        Some(root),
        None,
        Some(root),
    );

    assert_eq!(ids(&snapshot), vec![root, child, grandchild, unknown]);
    assert_eq!(snapshot.rows[1].parent_thread_id, Some(root));
    assert_eq!(snapshot.rows[2].parent_thread_id, Some(child));
    assert_eq!(snapshot.rows[3].depth, 0);
}

#[test]
fn exact_path_parent_takes_precedence_over_stale_thread_parent() {
    let first = ThreadId::new();
    let second = ThreadId::new();
    let child = ThreadId::new();
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![
            input(first, None, Some("/root/first"), Some("First"), false),
            input(second, None, Some("/root/second"), Some("Second"), false),
            input(
                child,
                Some(second),
                Some("/root/first/child"),
                Some("Child"),
                false,
            ),
        ],
        None,
        None,
        None,
    );

    assert_eq!(ids(&snapshot), vec![first, child, second]);
    assert_eq!(snapshot.rows[1].parent_thread_id, Some(first));
}

#[test]
fn markers_and_closed_metadata_are_preserved_without_special_cases() {
    let root = ThreadId::new();
    let closed = ThreadId::new();
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![
            input(root, None, None, None, false),
            input(closed, Some(root), None, None, true),
        ],
        Some(root),
        Some(closed),
        Some(root),
    );

    assert!(snapshot.rows[0].is_current);
    assert!(!snapshot.rows[0].is_selected);
    assert!(snapshot.rows[1].is_selected);
    assert!(!snapshot.rows[1].is_current);
    assert!(snapshot.rows[1].is_closed);
    assert!(!snapshot.rows[1].is_running);
}

#[test]
fn snapshot_is_bounded_and_marks_truncation() {
    let inputs = (0..MAX_AGENT_TREE_ROWS + 7)
        .map(|_| input(ThreadId::new(), None, None, None, false))
        .collect::<Vec<_>>();
    let snapshot = AgentTreeSnapshot::from_inputs(inputs, None, None, None);

    assert_eq!(snapshot.rows.len(), MAX_AGENT_TREE_ROWS);
    assert!(snapshot.truncated);
}

#[test]
fn cyclic_parent_metadata_is_promoted_to_roots() {
    let first = ThreadId::new();
    let second = ThreadId::new();
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![
            input(first, Some(second), None, Some("First"), false),
            input(second, Some(first), None, Some("Second"), false),
        ],
        None,
        None,
        None,
    );

    assert_eq!(ids(&snapshot), vec![first, second]);
    assert_eq!(snapshot.rows[0].depth, 0);
    assert_eq!(snapshot.rows[1].depth, 0);
}

#[test]
fn current_detached_agent_is_not_mislabeled_as_main() {
    let detached = ThreadId::new();
    let snapshot = AgentTreeSnapshot::from_inputs(
        vec![input(detached, None, Some("/root/detached"), None, false)],
        None,
        None,
        Some(detached),
    );

    assert_eq!(snapshot.rows[0].label, "[worker]");
    assert!(snapshot.rows[0].is_current);
}
