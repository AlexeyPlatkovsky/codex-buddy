//! Transport-independent code-mode contracts shared by Codex runtime consumers.

mod response;
mod runtime;
mod session;
mod tool;

pub use response::DEFAULT_IMAGE_DETAIL;
pub use response::FunctionCallOutputContentItem;
pub use response::ImageDetail;
pub use runtime::CodeModeNestedToolCall;
pub use runtime::DEFAULT_EXEC_YIELD_TIME_MS;
pub use runtime::DEFAULT_MAX_OUTPUT_TOKENS_PER_EXEC_CALL;
pub use runtime::DEFAULT_WAIT_YIELD_TIME_MS;
pub use runtime::ExecuteRequest;
pub use runtime::ExecuteToPendingOutcome;
pub use runtime::RuntimeResponse;
pub use runtime::WaitOutcome;
pub use runtime::WaitRequest;
pub use runtime::WaitToPendingOutcome;
pub use runtime::WaitToPendingRequest;
pub use session::CellId;
pub use session::CodeModeSession;
pub use session::CodeModeSessionCellExecutionLimits;
pub use session::CodeModeSessionDelegate;
pub use session::CodeModeSessionProvider;
pub use session::CodeModeSessionProviderFuture;
pub use session::CodeModeSessionResultFuture;
pub use session::NoopCodeModeSessionDelegate;
pub use session::NotificationFuture;
pub use session::StartedCell;
pub use session::ToolInvocationFuture;
pub use tool::CodeModeToolKind;
pub use tool::ToolDefinition;

pub const PUBLIC_TOOL_NAME: &str = "exec";
pub const WAIT_TOOL_NAME: &str = "wait";
