use super::*;
use pretty_assertions::assert_eq;

#[test]
fn interactive_and_terminal_transitions_are_retained_per_thread() {
    let thread_id = ThreadId::new();
    let mut statuses = AgentTreeStatusState::default();

    statuses.observe(
        thread_id,
        AgentTreeStatus::Waiting,
        AgentTreeLifecycleEvent::NeedsApproval,
    );
    statuses.observe(
        thread_id,
        AgentTreeStatus::Waiting,
        AgentTreeLifecycleEvent::Activity,
    );
    assert_eq!(
        statuses.get(&thread_id),
        Some(AgentTreeStatus::NeedsApproval)
    );

    statuses.observe(
        thread_id,
        AgentTreeStatus::Waiting,
        AgentTreeLifecycleEvent::Completed,
    );
    statuses.observe(
        thread_id,
        AgentTreeStatus::Waiting,
        AgentTreeLifecycleEvent::InteractiveResolved,
    );
    assert_eq!(statuses.get(&thread_id), Some(AgentTreeStatus::Completed));
}

#[test]
fn oldest_status_is_evicted_at_the_bound() {
    let mut statuses = AgentTreeStatusState::default();
    let first_thread_id = ThreadId::new();
    statuses.initialize(first_thread_id, AgentTreeStatus::Waiting);

    let mut last_thread_id = first_thread_id;
    for _ in 0..MAX_AGENT_TREE_STATUS_ENTRIES {
        last_thread_id = ThreadId::new();
        statuses.initialize(last_thread_id, AgentTreeStatus::Running);
    }

    assert_eq!(statuses.get(&first_thread_id), None);
    assert_eq!(
        statuses.get(&last_thread_id),
        Some(AgentTreeStatus::Running)
    );
}

#[test]
fn removing_a_status_allows_a_new_initial_state() {
    let thread_id = ThreadId::new();
    let mut statuses = AgentTreeStatusState::default();
    statuses.initialize(thread_id, AgentTreeStatus::Completed);
    statuses.remove(thread_id);
    statuses.initialize(thread_id, AgentTreeStatus::Running);

    assert_eq!(statuses.get(&thread_id), Some(AgentTreeStatus::Running));
}
