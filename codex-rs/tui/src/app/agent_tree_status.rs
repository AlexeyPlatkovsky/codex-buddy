//! Lifecycle state for the sub-agent tree presentation model.
//!
//! The app-server already reports the events needed to distinguish ordinary work, idle agents,
//! interactive requests, and terminal outcomes. This module keeps the presentation vocabulary
//! separate from that protocol and provides a small deterministic reducer for callers that cache
//! those events.

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadActiveFlag;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::TurnStatus;

/// User-visible lifecycle state for one agent-tree row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentTreeStatus {
    /// The agent is actively processing a turn or reporting activity.
    Running,
    /// The agent is alive but has no currently active turn or request.
    Waiting,
    /// The agent is paused until the user approves an operation.
    NeedsApproval,
    /// The agent is paused until the user answers a question.
    NeedsInput,
    /// The agent completed its work successfully.
    Completed,
    /// The agent stopped because its turn failed.
    Failed,
    /// The agent stopped because its turn was interrupted.
    Interrupted,
}

impl AgentTreeStatus {
    /// Derives the best available initial state from cached picker liveness metadata.
    pub(crate) fn from_picker_metadata(is_running: bool, is_closed: bool) -> Self {
        if is_closed {
            Self::Completed
        } else if is_running {
            Self::Running
        } else {
            Self::Waiting
        }
    }

    /// Applies one lifecycle observation without allowing stale generic events to erase a
    /// terminal outcome or a still-pending interactive request.
    pub(crate) fn transition(self, event: AgentTreeLifecycleEvent) -> Self {
        match event {
            AgentTreeLifecycleEvent::TurnStarted => Self::Running,
            AgentTreeLifecycleEvent::Activity => {
                if self.is_terminal() || self.needs_attention() {
                    self
                } else {
                    Self::Running
                }
            }
            AgentTreeLifecycleEvent::Waiting => match self {
                Self::NeedsApproval | Self::NeedsInput => self,
                Self::Completed | Self::Failed | Self::Interrupted => self,
                Self::Running | Self::Waiting => Self::Waiting,
            },
            AgentTreeLifecycleEvent::InteractiveResolved => match self {
                Self::Completed | Self::Failed | Self::Interrupted => self,
                Self::Running | Self::Waiting | Self::NeedsApproval | Self::NeedsInput => {
                    Self::Waiting
                }
            },
            AgentTreeLifecycleEvent::NeedsApproval => {
                if self.is_terminal() {
                    self
                } else {
                    Self::NeedsApproval
                }
            }
            AgentTreeLifecycleEvent::NeedsInput => {
                if self.is_terminal() {
                    self
                } else {
                    Self::NeedsInput
                }
            }
            AgentTreeLifecycleEvent::Completed => Self::Completed,
            AgentTreeLifecycleEvent::Failed => Self::Failed,
            AgentTreeLifecycleEvent::Interrupted => Self::Interrupted,
            AgentTreeLifecycleEvent::ThreadClosed => match self {
                Self::Failed | Self::Interrupted | Self::Completed => self,
                Self::Running | Self::Waiting | Self::NeedsApproval | Self::NeedsInput => {
                    Self::Completed
                }
            },
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Interrupted)
    }

    pub(crate) fn needs_attention(self) -> bool {
        matches!(self, Self::NeedsApproval | Self::NeedsInput)
    }
}

/// Internal lifecycle observations used to derive [`AgentTreeStatus`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentTreeLifecycleEvent {
    TurnStarted,
    Activity,
    Waiting,
    NeedsApproval,
    NeedsInput,
    InteractiveResolved,
    Completed,
    Failed,
    Interrupted,
    ThreadClosed,
}

impl AgentTreeLifecycleEvent {
    /// Converts the existing sub-agent activity liveness hint into a tree lifecycle event.
    pub(crate) fn from_activity_hint(is_running_hint: bool) -> Self {
        if is_running_hint {
            Self::Activity
        } else {
            Self::Waiting
        }
    }

    /// Converts the existing app-server turn status into the tree's internal lifecycle event.
    pub(crate) fn from_turn_status(status: TurnStatus) -> Self {
        match status {
            TurnStatus::InProgress => Self::TurnStarted,
            TurnStatus::Completed => Self::Completed,
            TurnStatus::Failed => Self::Failed,
            TurnStatus::Interrupted => Self::Interrupted,
        }
    }

    /// Converts a thread-level status update into the most specific tree observation it carries.
    pub(crate) fn from_thread_status(status: &ThreadStatus) -> Self {
        match status {
            ThreadStatus::NotLoaded | ThreadStatus::Idle => Self::Waiting,
            ThreadStatus::SystemError => Self::Failed,
            ThreadStatus::Active { active_flags } => {
                if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval) {
                    Self::NeedsApproval
                } else if active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) {
                    Self::NeedsInput
                } else {
                    Self::TurnStarted
                }
            }
        }
    }

    /// Extracts the lifecycle transition carried by a thread-scoped notification.
    pub(crate) fn from_server_notification(notification: &ServerNotification) -> Option<Self> {
        match notification {
            ServerNotification::ThreadStarted(notification) => {
                Some(Self::from_thread_status(&notification.thread.status))
            }
            ServerNotification::ThreadStatusChanged(notification) => {
                Some(Self::from_thread_status(&notification.status))
            }
            ServerNotification::TurnStarted(_) => Some(Self::TurnStarted),
            ServerNotification::TurnCompleted(notification) => {
                Some(Self::from_turn_status(notification.turn.status.clone()))
            }
            ServerNotification::ThreadClosed(_) => Some(Self::ThreadClosed),
            _ => None,
        }
    }

    /// Converts an existing interactive app-server request into an attention event.
    pub(crate) fn from_server_request(request: &ServerRequest) -> Option<Self> {
        match request {
            ServerRequest::ToolRequestUserInput { .. }
            | ServerRequest::McpServerElicitationRequest { .. } => Some(Self::NeedsInput),
            ServerRequest::CommandExecutionRequestApproval { .. }
            | ServerRequest::FileChangeRequestApproval { .. }
            | ServerRequest::PermissionsRequestApproval { .. }
            | ServerRequest::ApplyPatchApproval { .. }
            | ServerRequest::ExecCommandApproval { .. } => Some(Self::NeedsApproval),
            ServerRequest::DynamicToolCall { .. }
            | ServerRequest::AttestationGenerate { .. }
            | ServerRequest::CurrentTimeRead { .. }
            | ServerRequest::ChatgptAuthTokensRefresh { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "agent_tree_status_tests.rs"]
mod tests;
