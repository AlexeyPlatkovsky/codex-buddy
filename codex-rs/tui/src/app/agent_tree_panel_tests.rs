use super::*;
use crate::app::agent_tree::AgentTreeInput;
use codex_protocol::ThreadId;
use insta::assert_snapshot;
use ratatui::buffer::Buffer;

fn tree() -> AgentTreeSnapshot {
    let main = ThreadId::from_string("00000000-0000-0000-0000-000000000101").unwrap();
    let planner = ThreadId::from_string("00000000-0000-0000-0000-000000000102").unwrap();
    let worker = ThreadId::from_string("00000000-0000-0000-0000-000000000103").unwrap();
    AgentTreeSnapshot::from_inputs(
        [
            AgentTreeInput {
                thread_id: main,
                parent_thread_id: None,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
                status: AgentTreeStatus::Running,
            },
            AgentTreeInput {
                thread_id: planner,
                parent_thread_id: Some(main),
                agent_path: Some("root/planner".to_string()),
                agent_nickname: Some("Ada".to_string()),
                agent_role: Some("planner".to_string()),
                status: AgentTreeStatus::NeedsInput,
            },
            AgentTreeInput {
                thread_id: worker,
                parent_thread_id: Some(planner),
                agent_path: Some("root/planner/worker".to_string()),
                agent_nickname: Some("Grace".to_string()),
                agent_role: Some("worker".to_string()),
                status: AgentTreeStatus::Completed,
            },
        ],
        Some(main),
        Some(planner),
        Some(planner),
    )
}

fn buffer_text(buffer: &Buffer, area: Rect) -> String {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn wide_layout_clamps_the_panel_and_renders_tree_snapshot() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 120, /*height*/ 7,
    );
    let layout = layout_agent_tree_panel(area, /*has_subagents*/ true);
    assert_eq!(layout.chat_area.width, 83);
    assert_eq!(layout.panel_area.map(|area| area.width), Some(36));

    let tree = tree();
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(layout.panel_content_height(), &tree);
    viewport.update(
        &tree,
        tree.rows
            .iter()
            .find(|row| row.is_selected)
            .map(|row| row.thread_id),
    );
    let panel = layout.panel_area.unwrap();
    let mut buffer = Buffer::empty(area);
    render_agent_tree_panel(panel, &tree, &viewport, &mut buffer);
    assert_snapshot!(buffer_text(&buffer, panel), @r"
┌ Subagents ────────────────────────
│  ● Main [default]
│›   ? Ada [planner]
│      ✓ Grace [worker]
│
│
│
");
}

#[test]
fn narrow_layout_preserves_the_full_chat_width() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 99, /*height*/ 24,
    );
    let layout = layout_agent_tree_panel(area, /*has_subagents*/ true);
    assert_snapshot!(format!("{layout:#?}"), @r"
AgentTreePanelLayout {
    chat_area: Rect {
        x: 0,
        y: 0,
        width: 99,
        height: 24,
    },
    panel_area: None,
}
");
}

#[test]
fn panel_snapshot_covers_active_and_terminal_statuses() {
    let statuses = [
        AgentTreeStatus::Running,
        AgentTreeStatus::Waiting,
        AgentTreeStatus::NeedsApproval,
        AgentTreeStatus::NeedsInput,
        AgentTreeStatus::Completed,
        AgentTreeStatus::Failed,
        AgentTreeStatus::Interrupted,
    ];
    let rows = statuses
        .into_iter()
        .enumerate()
        .map(|(index, status)| AgentTreeInput {
            thread_id: ThreadId::from_string(&format!(
                "00000000-0000-0000-0000-{:012}",
                index + 201
            ))
            .expect("valid thread"),
            parent_thread_id: None,
            agent_path: None,
            agent_nickname: Some(format!("Agent {index}")),
            agent_role: Some("worker".to_string()),
            status,
        })
        .collect::<Vec<_>>();
    let active_thread_id = rows[3].thread_id;
    let tree =
        AgentTreeSnapshot::from_inputs(rows, None, Some(active_thread_id), Some(active_thread_id));
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 32, /*height*/ 9,
    );
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(8, &tree);
    viewport.update(&tree, Some(active_thread_id));
    let mut buffer = Buffer::empty(area);
    render_agent_tree_panel(area, &tree, &viewport, &mut buffer);

    assert_snapshot!(buffer_text(&buffer, area), @r"
┌ Subagents ────────────────────
│  ● Agent 0 [worker]
│  ○ Agent 1 [worker]
│  ! Agent 2 [worker]
│› ? Agent 3 [worker]
│  ✓ Agent 4 [worker]
│  × Agent 5 [worker]
│  – Agent 6 [worker]
│
");
}

#[test]
fn panel_snapshot_marks_scroll_direction_without_changing_selected_agent() {
    let tree = AgentTreeSnapshot::from_inputs(
        (0..8).map(|index| AgentTreeInput {
            thread_id: ThreadId::from_string(&format!(
                "00000000-0000-0000-0000-{:012}",
                index + 301
            ))
            .expect("valid thread"),
            parent_thread_id: None,
            agent_path: None,
            agent_nickname: Some(format!("Agent {index}")),
            agent_role: Some("worker".to_string()),
            status: AgentTreeStatus::Waiting,
        }),
        None,
        None,
        None,
    );
    let selected_thread_id = tree.rows[6].thread_id;
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 32, /*height*/ 4,
    );
    let mut viewport = AgentTreeViewport::new();
    viewport.resize(3, &tree);
    viewport.update(&tree, Some(selected_thread_id));
    let mut selected_tree = tree.clone();
    for row in &mut selected_tree.rows {
        row.is_selected = row.thread_id == selected_thread_id;
        row.is_current = row.thread_id == selected_thread_id;
    }
    let mut buffer = Buffer::empty(area);
    render_agent_tree_panel(area, &selected_tree, &viewport, &mut buffer);

    assert_eq!(viewport.selected_thread_id(), Some(selected_thread_id));
    assert_snapshot!(buffer_text(&buffer, area), @r"
┌ Subagents ↕ ──────────────────
│  ○ Agent 4 [worker]
│  ○ Agent 5 [worker]
│› ○ Agent 6 [worker]
");
}
