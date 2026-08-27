/// Product defaults used as the starting point for capability resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePreset {
    /// Preserve the complete upstream Codex runtime surface.
    Full,
    /// Limit the runtime to Codex Buddy's coding-focused surface.
    Coding,
}

/// Groups of tools that may be exposed to a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolCapability {
    /// Run local shell commands.
    Shell,
    /// Write to or poll an existing process.
    ProcessInput,
    /// Apply structured file patches.
    ApplyPatch,
    /// Search live or cached web content.
    WebSearch,
    /// Inspect a local image.
    ViewImage,
    /// Spawn, inspect, and communicate with subagents.
    MultiAgent,
    /// Request structured input from the user.
    UserInput,
    /// Request additional sandbox permissions or approvals.
    Permissions,
    /// Use tools contributed by explicitly configured MCP servers.
    Mcp,
    /// Use tools supplied dynamically by a client.
    ClientTools,
    /// Control a browser or desktop through computer use.
    ComputerUse,
    /// Generate or edit images.
    ImageGeneration,
    /// Use realtime audio or voice tools.
    Realtime,
    /// Run JavaScript through code mode.
    CodeMode,
    /// Read or write long-lived memories.
    Memories,
    /// Create and update persistent goals.
    Goals,
    /// Operate the asynchronous work queue.
    Queue,
    /// Discover, install, or invoke plugins and apps.
    Plugins,
}

impl ToolCapability {
    pub(crate) const ALL: [Self; 18] = [
        Self::Shell,
        Self::ProcessInput,
        Self::ApplyPatch,
        Self::WebSearch,
        Self::ViewImage,
        Self::MultiAgent,
        Self::UserInput,
        Self::Permissions,
        Self::Mcp,
        Self::ClientTools,
        Self::ComputerUse,
        Self::ImageGeneration,
        Self::Realtime,
        Self::CodeMode,
        Self::Memories,
        Self::Goals,
        Self::Queue,
        Self::Plugins,
    ];

    pub(crate) const CODING: [Self; 10] = [
        Self::Shell,
        Self::ProcessInput,
        Self::ApplyPatch,
        Self::WebSearch,
        Self::ViewImage,
        Self::MultiAgent,
        Self::UserInput,
        Self::Permissions,
        Self::Mcp,
        Self::ClientTools,
    ];
}

/// First-party extension groups that can contribute runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeExtension {
    /// Multi-agent collaboration.
    Agent,
    /// Apps and connector integrations.
    Connectors,
    /// Git attribution attached to generated changes.
    GitAttribution,
    /// Persistent goal management.
    Goals,
    /// Guardian policy and review behavior.
    Guardian,
    /// Long-lived history notes.
    HistoryNotes,
    /// Image generation and editing.
    ImageGeneration,
    /// Core extension-backed item types.
    Items,
    /// MCP server and tool contributions.
    Mcp,
    /// Memory extraction and consolidation.
    Memories,
    /// Asynchronous queued work.
    Queue,
    /// Skill discovery and invocation.
    Skills,
    /// Standalone web search.
    WebSearch,
}

impl RuntimeExtension {
    pub(crate) const ALL: [Self; 13] = [
        Self::Agent,
        Self::Connectors,
        Self::GitAttribution,
        Self::Goals,
        Self::Guardian,
        Self::HistoryNotes,
        Self::ImageGeneration,
        Self::Items,
        Self::Mcp,
        Self::Memories,
        Self::Queue,
        Self::Skills,
        Self::WebSearch,
    ];

    pub(crate) const CODING: [Self; 7] = [
        Self::Agent,
        Self::GitAttribution,
        Self::Guardian,
        Self::Items,
        Self::Mcp,
        Self::Skills,
        Self::WebSearch,
    ];
}

/// Long-lived service groups that a product may construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeService {
    /// The embedded or remote app-server boundary.
    AppServer,
    /// Local and remote command execution infrastructure.
    ExecServer,
    /// Authentication and credential lifecycle.
    Authentication,
    /// Sandbox enforcement.
    Sandbox,
    /// User and managed-policy approval handling.
    Approvals,
    /// Rollout persistence and resume support.
    RolloutStore,
    /// Thread metadata and lifecycle persistence.
    ThreadStore,
    /// Explicitly configured MCP server lifecycle.
    McpRuntime,
    /// Plugin discovery and execution.
    Plugins,
    /// Apps and connector providers.
    Apps,
    /// Browser and computer-use infrastructure.
    Browser,
    /// Realtime audio and voice infrastructure.
    Realtime,
    /// Memory extraction and consolidation workers.
    Memories,
    /// Persistent goal workers.
    Goals,
    /// Asynchronous queue workers.
    Queue,
    /// Cloud task orchestration.
    CloudTasks,
    /// Desktop integration.
    Desktop,
    /// Remote-control infrastructure.
    RemoteControl,
    /// JavaScript code-mode host infrastructure.
    CodeMode,
    /// Image generation infrastructure.
    ImageGeneration,
}

impl RuntimeService {
    pub(crate) const ALL: [Self; 20] = [
        Self::AppServer,
        Self::ExecServer,
        Self::Authentication,
        Self::Sandbox,
        Self::Approvals,
        Self::RolloutStore,
        Self::ThreadStore,
        Self::McpRuntime,
        Self::Plugins,
        Self::Apps,
        Self::Browser,
        Self::Realtime,
        Self::Memories,
        Self::Goals,
        Self::Queue,
        Self::CloudTasks,
        Self::Desktop,
        Self::RemoteControl,
        Self::CodeMode,
        Self::ImageGeneration,
    ];

    pub(crate) const CODING: [Self; 8] = [
        Self::AppServer,
        Self::ExecServer,
        Self::Authentication,
        Self::Sandbox,
        Self::Approvals,
        Self::RolloutStore,
        Self::ThreadStore,
        Self::McpRuntime,
    ];
}

/// Optional external inputs that can extend the runtime surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExternalSource {
    /// MCP servers and their tools.
    Mcp,
    /// Skills and their supporting resources.
    Skills,
    /// Optional instruction files and fragments.
    Instructions,
    /// Client-supplied dynamic tool definitions.
    ClientTools,
}

impl ExternalSource {
    pub(crate) const ALL: [Self; 4] = [
        Self::Mcp,
        Self::Skills,
        Self::Instructions,
        Self::ClientTools,
    ];
}

/// Effective availability of a tool, extension, or service capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityDecision {
    /// The capability is available to downstream runtime checks.
    Enabled,
    /// The selected product preset does not include the capability.
    ExcludedByPreset,
    /// The product was compiled without the capability.
    NotCompiled,
    /// A runtime policy explicitly denied the capability.
    DeniedByPolicy,
}

impl CapabilityDecision {
    /// Returns whether the capability is available after all policy layers.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

/// Effective loading policy for an external source.
///
/// The ordering is intentionally monotonic: automatic discovery can be
/// restricted to explicit grants or disabled, but a patch cannot widen the
/// policy selected by the product preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalSourcePolicy {
    /// Product defaults and implicit discovery may contribute the source.
    Automatic,
    /// Only a source granted by a recognized explicit configuration origin is allowed.
    ExplicitOnly,
    /// The source is unavailable.
    Disabled,
}

impl ExternalSourcePolicy {
    /// Returns the more restrictive of two source policies.
    pub fn restricted_by(self, restriction: Self) -> Self {
        match (self, restriction) {
            (Self::Disabled, _) | (_, Self::Disabled) => Self::Disabled,
            (Self::ExplicitOnly, Self::Automatic)
            | (Self::Automatic, Self::ExplicitOnly)
            | (Self::ExplicitOnly, Self::ExplicitOnly) => Self::ExplicitOnly,
            (Self::Automatic, Self::Automatic) => Self::Automatic,
        }
    }
}
