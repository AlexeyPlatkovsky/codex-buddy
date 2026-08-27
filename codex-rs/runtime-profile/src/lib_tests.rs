use super::*;
use pretty_assertions::assert_eq;

#[test]
fn resolution_intersects_preset_ceiling_and_runtime_policy() {
    let ceiling = RuntimeCompileCeiling::full()
        .without_tool(ToolCapability::ApplyPatch)
        .without_extension(RuntimeExtension::WebSearch)
        .without_service(RuntimeService::ExecServer);
    let patch = RuntimePolicyPatch::default()
        .deny_tool(ToolCapability::Shell)
        .deny_extension(RuntimeExtension::Skills)
        .deny_service(RuntimeService::McpRuntime);

    let profile = ResolvedRuntimeProfile::coding(&ceiling, &patch);

    assert_eq!(
        (
            profile.tool(ToolCapability::Shell),
            profile.tool(ToolCapability::ApplyPatch),
            profile.tool(ToolCapability::ComputerUse),
            profile.extension(RuntimeExtension::Skills),
            profile.extension(RuntimeExtension::WebSearch),
            profile.extension(RuntimeExtension::Connectors),
            profile.service(RuntimeService::McpRuntime),
            profile.service(RuntimeService::ExecServer),
            profile.service(RuntimeService::Plugins),
        ),
        (
            CapabilityDecision::DeniedByPolicy,
            CapabilityDecision::NotCompiled,
            CapabilityDecision::ExcludedByPreset,
            CapabilityDecision::DeniedByPolicy,
            CapabilityDecision::NotCompiled,
            CapabilityDecision::ExcludedByPreset,
            CapabilityDecision::DeniedByPolicy,
            CapabilityDecision::NotCompiled,
            CapabilityDecision::ExcludedByPreset,
        )
    );
}

#[test]
fn policy_layers_are_monotonic() {
    let project_policy = RuntimePolicyPatch::default()
        .deny_tool(ToolCapability::WebSearch)
        .restrict_external_source(ExternalSource::Mcp, ExternalSourcePolicy::ExplicitOnly);
    let managed_policy = RuntimePolicyPatch::default()
        .deny_service(RuntimeService::Plugins)
        .restrict_external_source(ExternalSource::Mcp, ExternalSourcePolicy::Disabled);
    let attempted_widening = RuntimePolicyPatch::default()
        .restrict_external_source(ExternalSource::Mcp, ExternalSourcePolicy::Automatic);

    let patch = project_policy
        .restricted_by(managed_policy)
        .restricted_by(attempted_widening);
    let profile = ResolvedRuntimeProfile::full(&RuntimeCompileCeiling::full(), &patch);

    assert_eq!(
        (
            profile.tool(ToolCapability::WebSearch),
            profile.service(RuntimeService::Plugins),
            profile.external_source(ExternalSource::Mcp),
        ),
        (
            CapabilityDecision::DeniedByPolicy,
            CapabilityDecision::DeniedByPolicy,
            ExternalSourcePolicy::Disabled,
        )
    );
}

#[test]
fn coding_sources_require_explicit_grants_and_honor_compile_ceiling() {
    let ceiling =
        RuntimeCompileCeiling::full().without_external_source(ExternalSource::ClientTools);
    let patch = RuntimePolicyPatch::default().restrict_external_source(
        ExternalSource::Instructions,
        ExternalSourcePolicy::Automatic,
    );

    let profile = ResolvedRuntimeProfile::coding(&ceiling, &patch);

    assert_eq!(
        (
            profile.external_source(ExternalSource::Mcp),
            profile.external_source(ExternalSource::Skills),
            profile.external_source(ExternalSource::Instructions),
            profile.external_source(ExternalSource::ClientTools),
        ),
        (
            ExternalSourcePolicy::ExplicitOnly,
            ExternalSourcePolicy::ExplicitOnly,
            ExternalSourcePolicy::ExplicitOnly,
            ExternalSourcePolicy::Disabled,
        )
    );
}
