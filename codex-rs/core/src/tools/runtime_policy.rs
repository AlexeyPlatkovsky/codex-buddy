use crate::session::turn_context::TurnContext;
use codex_runtime_profile::ExternalSource;
use codex_runtime_profile::ExternalSourcePolicy;
use codex_runtime_profile::RuntimePreset;
use codex_runtime_profile::ToolCapability;

pub(super) fn tool_enabled(turn_context: &TurnContext, capability: ToolCapability) -> bool {
    turn_context
        .config
        .runtime_profile
        .tool(capability)
        .is_enabled()
}

pub(super) fn explicit_source_tool_enabled(
    turn_context: &TurnContext,
    capability: ToolCapability,
    source: ExternalSource,
) -> bool {
    if !tool_enabled(turn_context, capability) {
        return false;
    }

    match turn_context.config.runtime_profile.external_source(source) {
        ExternalSourcePolicy::Automatic => true,
        ExternalSourcePolicy::ExplicitOnly => turn_context
            .config
            .runtime_profile_policy
            .source_is_explicitly_configured(source),
        ExternalSourcePolicy::Disabled => false,
    }
}

pub(super) fn full_tool_surface_enabled(turn_context: &TurnContext) -> bool {
    turn_context.config.runtime_profile.preset() == RuntimePreset::Full
}
