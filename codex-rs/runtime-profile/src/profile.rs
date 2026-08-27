use crate::CapabilityDecision;
use crate::ExternalSource;
use crate::ExternalSourcePolicy;
use crate::RuntimeExtension;
use crate::RuntimePreset;
use crate::RuntimeService;
use crate::ToolCapability;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Capabilities present in a particular compiled product.
///
/// Build entry points start from [`Self::full`] and remove capability groups
/// that were not linked. Resolution never enables a capability above this
/// ceiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompileCeiling {
    tools: BTreeSet<ToolCapability>,
    extensions: BTreeSet<RuntimeExtension>,
    services: BTreeSet<RuntimeService>,
    external_sources: BTreeSet<ExternalSource>,
}

impl RuntimeCompileCeiling {
    /// Declares that every capability known to this crate is compiled in.
    pub fn full() -> Self {
        Self {
            tools: ToolCapability::ALL.into_iter().collect(),
            extensions: RuntimeExtension::ALL.into_iter().collect(),
            services: RuntimeService::ALL.into_iter().collect(),
            external_sources: ExternalSource::ALL.into_iter().collect(),
        }
    }

    /// Removes a model-tool capability from the compile ceiling.
    pub fn without_tool(mut self, capability: ToolCapability) -> Self {
        self.tools.remove(&capability);
        self
    }

    /// Removes an extension capability from the compile ceiling.
    pub fn without_extension(mut self, extension: RuntimeExtension) -> Self {
        self.extensions.remove(&extension);
        self
    }

    /// Removes a service capability from the compile ceiling.
    pub fn without_service(mut self, service: RuntimeService) -> Self {
        self.services.remove(&service);
        self
    }

    /// Removes an external source from the compile ceiling.
    pub fn without_external_source(mut self, source: ExternalSource) -> Self {
        self.external_sources.remove(&source);
        self
    }
}

impl Default for RuntimeCompileCeiling {
    fn default() -> Self {
        Self::full()
    }
}

/// Runtime policy restrictions applied after product and compile-time policy.
///
/// The API exposes only denial and restriction operations, so less-trusted
/// configuration layers cannot re-enable a capability denied by an earlier
/// layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimePolicyPatch {
    denied_tools: BTreeSet<ToolCapability>,
    denied_extensions: BTreeSet<RuntimeExtension>,
    denied_services: BTreeSet<RuntimeService>,
    source_restrictions: BTreeMap<ExternalSource, ExternalSourcePolicy>,
}

impl RuntimePolicyPatch {
    /// Denies a model-tool capability.
    pub fn deny_tool(mut self, capability: ToolCapability) -> Self {
        self.denied_tools.insert(capability);
        self
    }

    /// Denies an extension capability.
    pub fn deny_extension(mut self, extension: RuntimeExtension) -> Self {
        self.denied_extensions.insert(extension);
        self
    }

    /// Denies a service capability.
    pub fn deny_service(mut self, service: RuntimeService) -> Self {
        self.denied_services.insert(service);
        self
    }

    /// Narrows the loading policy for an external source.
    pub fn restrict_external_source(
        mut self,
        source: ExternalSource,
        restriction: ExternalSourcePolicy,
    ) -> Self {
        self.source_restrictions
            .entry(source)
            .and_modify(|current| *current = current.restricted_by(restriction))
            .or_insert(restriction);
        self
    }

    /// Combines two policy layers without allowing either layer to widen access.
    pub fn restricted_by(mut self, restriction: Self) -> Self {
        self.denied_tools.extend(restriction.denied_tools);
        self.denied_extensions.extend(restriction.denied_extensions);
        self.denied_services.extend(restriction.denied_services);
        for (source, policy) in restriction.source_restrictions {
            self = self.restrict_external_source(source, policy);
        }
        self
    }
}

/// Immutable result of resolving product, compile-time, and runtime policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeProfile {
    preset: RuntimePreset,
    tools: BTreeMap<ToolCapability, CapabilityDecision>,
    extensions: BTreeMap<RuntimeExtension, CapabilityDecision>,
    services: BTreeMap<RuntimeService, CapabilityDecision>,
    external_sources: BTreeMap<ExternalSource, ExternalSourcePolicy>,
}

impl ResolvedRuntimeProfile {
    /// Resolves the complete upstream runtime surface.
    pub fn full(ceiling: &RuntimeCompileCeiling, patch: &RuntimePolicyPatch) -> Self {
        Self::resolve(RuntimePreset::Full, ceiling, patch)
    }

    /// Resolves the coding-focused Codex Buddy runtime surface.
    pub fn coding(ceiling: &RuntimeCompileCeiling, patch: &RuntimePolicyPatch) -> Self {
        Self::resolve(RuntimePreset::Coding, ceiling, patch)
    }

    /// Resolves a product preset against its compiled ceiling and policy patch.
    pub fn resolve(
        preset: RuntimePreset,
        ceiling: &RuntimeCompileCeiling,
        patch: &RuntimePolicyPatch,
    ) -> Self {
        let preset_tools: BTreeSet<_> = match preset {
            RuntimePreset::Full => ToolCapability::ALL.into_iter().collect(),
            RuntimePreset::Coding => ToolCapability::CODING.into_iter().collect(),
        };
        let preset_extensions: BTreeSet<_> = match preset {
            RuntimePreset::Full => RuntimeExtension::ALL.into_iter().collect(),
            RuntimePreset::Coding => RuntimeExtension::CODING.into_iter().collect(),
        };
        let preset_services: BTreeSet<_> = match preset {
            RuntimePreset::Full => RuntimeService::ALL.into_iter().collect(),
            RuntimePreset::Coding => RuntimeService::CODING.into_iter().collect(),
        };

        let tools = ToolCapability::ALL
            .into_iter()
            .map(|capability| {
                let decision = decide(
                    preset_tools.contains(&capability),
                    ceiling.tools.contains(&capability),
                    patch.denied_tools.contains(&capability),
                );
                (capability, decision)
            })
            .collect();
        let extensions = RuntimeExtension::ALL
            .into_iter()
            .map(|extension| {
                let decision = decide(
                    preset_extensions.contains(&extension),
                    ceiling.extensions.contains(&extension),
                    patch.denied_extensions.contains(&extension),
                );
                (extension, decision)
            })
            .collect();
        let services = RuntimeService::ALL
            .into_iter()
            .map(|service| {
                let decision = decide(
                    preset_services.contains(&service),
                    ceiling.services.contains(&service),
                    patch.denied_services.contains(&service),
                );
                (service, decision)
            })
            .collect();
        let external_sources = ExternalSource::ALL
            .into_iter()
            .map(|source| {
                let preset_policy = match preset {
                    RuntimePreset::Full => ExternalSourcePolicy::Automatic,
                    RuntimePreset::Coding => ExternalSourcePolicy::ExplicitOnly,
                };
                let policy = if ceiling.external_sources.contains(&source) {
                    patch
                        .source_restrictions
                        .get(&source)
                        .copied()
                        .map_or(preset_policy, |restriction| {
                            preset_policy.restricted_by(restriction)
                        })
                } else {
                    ExternalSourcePolicy::Disabled
                };
                (source, policy)
            })
            .collect();

        Self {
            preset,
            tools,
            extensions,
            services,
            external_sources,
        }
    }

    /// Returns the product preset from which this profile was resolved.
    pub fn preset(&self) -> RuntimePreset {
        self.preset
    }

    /// Returns the decision for a model-tool capability.
    pub fn tool(&self, capability: ToolCapability) -> CapabilityDecision {
        self.tools[&capability]
    }

    /// Returns the decision for an extension capability.
    pub fn extension(&self, extension: RuntimeExtension) -> CapabilityDecision {
        self.extensions[&extension]
    }

    /// Returns the decision for a service capability.
    pub fn service(&self, service: RuntimeService) -> CapabilityDecision {
        self.services[&service]
    }

    /// Returns the effective loading policy for an external source.
    pub fn external_source(&self, source: ExternalSource) -> ExternalSourcePolicy {
        self.external_sources[&source]
    }
}

fn decide(preset_enabled: bool, compiled: bool, denied: bool) -> CapabilityDecision {
    if !compiled {
        CapabilityDecision::NotCompiled
    } else if denied {
        CapabilityDecision::DeniedByPolicy
    } else if preset_enabled {
        CapabilityDecision::Enabled
    } else {
        CapabilityDecision::ExcludedByPreset
    }
}
