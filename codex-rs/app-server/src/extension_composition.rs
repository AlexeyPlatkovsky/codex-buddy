use codex_runtime_profile::CapabilityDecision;
use codex_runtime_profile::ResolvedRuntimeProfile;
use codex_runtime_profile::RuntimeExtension;
use codex_runtime_profile::RuntimeService;

/// Install-time units whose order defines the upstream extension pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionComponent {
    Queue,
    HistoryNotes,
    Goals,
    GitAttribution,
    Guardian,
    Memories,
    Mcp,
    ExecutorPlugins,
    WebSearch,
    ImageGeneration,
    Skills,
}

const UPSTREAM_EXTENSION_ORDER: [ExtensionComponent; 11] = [
    ExtensionComponent::Queue,
    ExtensionComponent::HistoryNotes,
    ExtensionComponent::Goals,
    ExtensionComponent::GitAttribution,
    ExtensionComponent::Guardian,
    ExtensionComponent::Memories,
    ExtensionComponent::Mcp,
    ExtensionComponent::ExecutorPlugins,
    ExtensionComponent::WebSearch,
    ExtensionComponent::ImageGeneration,
    ExtensionComponent::Skills,
];

/// Immutable construction plan resolved once for the process runtime profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionComposition {
    components: Vec<ExtensionComponent>,
    executor_skill_provider: bool,
    orchestrator_skill_provider: bool,
    plugin_startup_tasks: bool,
    apps_service: bool,
}

impl ExtensionComposition {
    pub(crate) fn from_profile(profile: &ResolvedRuntimeProfile) -> Self {
        let components = UPSTREAM_EXTENSION_ORDER
            .into_iter()
            .filter(|component| component_enabled(profile, *component))
            .collect();
        let skills_enabled = extension_enabled(profile, RuntimeExtension::Skills);
        let plugins_enabled = service_enabled(profile, RuntimeService::Plugins);

        Self {
            components,
            executor_skill_provider: skills_enabled && plugins_enabled,
            orchestrator_skill_provider: skills_enabled && plugins_enabled,
            plugin_startup_tasks: plugins_enabled,
            apps_service: service_enabled(profile, RuntimeService::Apps),
        }
    }

    #[cfg(test)]
    pub(crate) fn components(&self) -> &[ExtensionComponent] {
        &self.components
    }

    pub(crate) fn installs(&self, component: ExtensionComponent) -> bool {
        self.components.contains(&component)
    }

    pub(crate) fn uses_executor_skill_provider(&self) -> bool {
        self.executor_skill_provider
    }

    pub(crate) fn uses_orchestrator_skill_provider(&self) -> bool {
        self.orchestrator_skill_provider
    }

    pub(crate) fn starts_plugin_tasks(&self) -> bool {
        self.plugin_startup_tasks
    }

    pub(crate) fn constructs_apps_service(&self) -> bool {
        self.apps_service
    }
}

fn component_enabled(profile: &ResolvedRuntimeProfile, component: ExtensionComponent) -> bool {
    let extension = match component {
        ExtensionComponent::Queue => RuntimeExtension::Queue,
        ExtensionComponent::HistoryNotes => RuntimeExtension::HistoryNotes,
        ExtensionComponent::Goals => RuntimeExtension::Goals,
        ExtensionComponent::GitAttribution => RuntimeExtension::GitAttribution,
        ExtensionComponent::Guardian => RuntimeExtension::Guardian,
        ExtensionComponent::Memories => RuntimeExtension::Memories,
        ExtensionComponent::Mcp => RuntimeExtension::Mcp,
        ExtensionComponent::ExecutorPlugins => RuntimeExtension::Connectors,
        ExtensionComponent::WebSearch => RuntimeExtension::WebSearch,
        ExtensionComponent::ImageGeneration => RuntimeExtension::ImageGeneration,
        ExtensionComponent::Skills => RuntimeExtension::Skills,
    };
    let service = match component {
        ExtensionComponent::Queue => Some(RuntimeService::Queue),
        ExtensionComponent::Goals => Some(RuntimeService::Goals),
        ExtensionComponent::Guardian => Some(RuntimeService::Approvals),
        ExtensionComponent::Memories => Some(RuntimeService::Memories),
        ExtensionComponent::Mcp => Some(RuntimeService::McpRuntime),
        ExtensionComponent::ExecutorPlugins => Some(RuntimeService::Plugins),
        ExtensionComponent::ImageGeneration => Some(RuntimeService::ImageGeneration),
        ExtensionComponent::HistoryNotes
        | ExtensionComponent::GitAttribution
        | ExtensionComponent::WebSearch
        | ExtensionComponent::Skills => None,
    };
    extension_enabled(profile, extension)
        && service.is_none_or(|service| service_enabled(profile, service))
}

fn extension_enabled(profile: &ResolvedRuntimeProfile, extension: RuntimeExtension) -> bool {
    profile.extension(extension) == CapabilityDecision::Enabled
}

fn service_enabled(profile: &ResolvedRuntimeProfile, service: RuntimeService) -> bool {
    profile.service(service) == CapabilityDecision::Enabled
}

#[cfg(test)]
#[path = "extension_composition_tests.rs"]
mod tests;
