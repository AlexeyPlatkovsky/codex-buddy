use super::*;
use pretty_assertions::assert_eq;

#[test]
fn picker_metadata_preserves_running_waiting_and_closed_states() {
    assert_eq!(
        AgentTreeStatus::from_picker_metadata(true, false),
        AgentTreeStatus::Running
    );
    assert_eq!(
        AgentTreeStatus::from_picker_metadata(false, false),
        AgentTreeStatus::Waiting
    );
    assert_eq!(
        AgentTreeStatus::from_picker_metadata(false, true),
        AgentTreeStatus::Completed
    );
    assert_eq!(
        AgentTreeStatus::from_picker_metadata(true, true),
        AgentTreeStatus::Completed
    );
}

#[test]
fn existing_turn_statuses_map_to_distinct_terminal_tree_states() {
    assert_eq!(
        AgentTreeLifecycleEvent::from_turn_status(
            codex_app_server_protocol::TurnStatus::InProgress
        ),
        AgentTreeLifecycleEvent::TurnStarted
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_turn_status(codex_app_server_protocol::TurnStatus::Completed),
        AgentTreeLifecycleEvent::Completed
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_turn_status(codex_app_server_protocol::TurnStatus::Failed),
        AgentTreeLifecycleEvent::Failed
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_turn_status(
            codex_app_server_protocol::TurnStatus::Interrupted
        ),
        AgentTreeLifecycleEvent::Interrupted
    );
}

#[test]
fn thread_statuses_capture_waiting_and_interactive_states() {
    use codex_app_server_protocol::ThreadActiveFlag;
    use codex_app_server_protocol::ThreadStatus;

    assert_eq!(
        AgentTreeLifecycleEvent::from_thread_status(&ThreadStatus::Idle),
        AgentTreeLifecycleEvent::Waiting
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_thread_status(&ThreadStatus::SystemError),
        AgentTreeLifecycleEvent::Failed
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_thread_status(&ThreadStatus::Active {
            active_flags: vec![ThreadActiveFlag::WaitingOnApproval],
        }),
        AgentTreeLifecycleEvent::NeedsApproval
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_thread_status(&ThreadStatus::Active {
            active_flags: vec![ThreadActiveFlag::WaitingOnUserInput],
        }),
        AgentTreeLifecycleEvent::NeedsInput
    );
}

#[test]
fn activity_liveness_hints_map_to_running_or_waiting() {
    assert_eq!(
        AgentTreeLifecycleEvent::from_activity_hint(true),
        AgentTreeLifecycleEvent::Activity
    );
    assert_eq!(
        AgentTreeLifecycleEvent::from_activity_hint(false),
        AgentTreeLifecycleEvent::Waiting
    );
}

#[test]
fn interactive_states_are_reached_and_resolved_without_losing_terminal_outcomes() {
    let status = AgentTreeStatus::Waiting
        .transition(AgentTreeLifecycleEvent::TurnStarted)
        .transition(AgentTreeLifecycleEvent::NeedsApproval)
        .transition(AgentTreeLifecycleEvent::Waiting)
        .transition(AgentTreeLifecycleEvent::InteractiveResolved);
    assert_eq!(status, AgentTreeStatus::Waiting);
    assert!(!status.needs_attention());

    let status = AgentTreeStatus::Running
        .transition(AgentTreeLifecycleEvent::NeedsInput)
        .transition(AgentTreeLifecycleEvent::Completed)
        .transition(AgentTreeLifecycleEvent::InteractiveResolved);
    assert_eq!(status, AgentTreeStatus::Completed);
    assert!(status.is_terminal());
}

#[test]
fn terminal_events_are_distinct_and_close_does_not_rewrite_failure_or_interrupt() {
    let statuses = [
        (
            AgentTreeLifecycleEvent::Completed,
            AgentTreeStatus::Completed,
        ),
        (AgentTreeLifecycleEvent::Failed, AgentTreeStatus::Failed),
        (
            AgentTreeLifecycleEvent::Interrupted,
            AgentTreeStatus::Interrupted,
        ),
    ];

    for (event, expected) in statuses {
        let status = AgentTreeStatus::Waiting
            .transition(event)
            .transition(AgentTreeLifecycleEvent::ThreadClosed);
        assert_eq!(status, expected);
        assert!(status.is_terminal());
    }
}

#[test]
fn stale_activity_cannot_revive_a_terminal_agent() {
    let status = AgentTreeStatus::Failed
        .transition(AgentTreeLifecycleEvent::Activity)
        .transition(AgentTreeLifecycleEvent::Waiting);

    assert_eq!(status, AgentTreeStatus::Failed);
}

#[test]
fn a_new_turn_revives_a_terminal_agent() {
    let status = AgentTreeStatus::Interrupted.transition(AgentTreeLifecycleEvent::TurnStarted);

    assert_eq!(status, AgentTreeStatus::Running);
}

#[test]
fn stale_activity_cannot_clear_pending_attention() {
    let status = AgentTreeStatus::NeedsInput.transition(AgentTreeLifecycleEvent::Activity);

    assert_eq!(status, AgentTreeStatus::NeedsInput);
    assert!(status.needs_attention());
}
