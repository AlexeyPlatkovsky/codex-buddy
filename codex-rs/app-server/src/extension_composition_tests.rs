use super::*;
use codex_runtime_profile::RuntimeCompileCeiling;
use codex_runtime_profile::RuntimePolicyPatch;

#[test]
fn full_profile_preserves_upstream_construction_order() {
    let profile = ResolvedRuntimeProfile::full(
        &RuntimeCompileCeiling::full(),
        &RuntimePolicyPatch::default(),
    );

    let composition = ExtensionComposition::from_profile(&profile);

    assert_eq!(composition.components(), UPSTREAM_EXTENSION_ORDER);
    assert!(composition.uses_executor_skill_provider());
    assert!(composition.uses_orchestrator_skill_provider());
    assert!(composition.starts_plugin_tasks());
    assert!(composition.constructs_apps_service());
}

#[test]
fn coding_profile_constructs_only_coding_extensions_and_host_skills() {
    let profile = ResolvedRuntimeProfile::coding(
        &RuntimeCompileCeiling::full(),
        &RuntimePolicyPatch::default(),
    );

    let composition = ExtensionComposition::from_profile(&profile);

    assert_eq!(
        composition.components(),
        [
            ExtensionComponent::GitAttribution,
            ExtensionComponent::Guardian,
            ExtensionComponent::Mcp,
            ExtensionComponent::WebSearch,
            ExtensionComponent::Skills,
        ]
    );
    assert!(!composition.uses_executor_skill_provider());
    assert!(!composition.uses_orchestrator_skill_provider());
    assert!(!composition.starts_plugin_tasks());
    assert!(!composition.constructs_apps_service());
}

#[test]
fn runtime_denials_remove_constructors_from_the_plan() {
    let profile = ResolvedRuntimeProfile::full(
        &RuntimeCompileCeiling::full(),
        &RuntimePolicyPatch::default()
            .deny_extension(RuntimeExtension::GitAttribution)
            .deny_extension(RuntimeExtension::Skills)
            .deny_service(RuntimeService::Plugins),
    );

    let composition = ExtensionComposition::from_profile(&profile);

    assert!(!composition.installs(ExtensionComponent::GitAttribution));
    assert!(!composition.installs(ExtensionComponent::Skills));
    assert!(!composition.installs(ExtensionComponent::ExecutorPlugins));
    assert!(!composition.uses_executor_skill_provider());
    assert!(!composition.uses_orchestrator_skill_provider());
    assert!(!composition.starts_plugin_tasks());
}
