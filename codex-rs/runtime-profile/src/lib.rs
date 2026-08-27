//! Product-level runtime capability policy.
//!
//! This crate intentionally has no dependencies on Codex configuration, core,
//! protocol, or runtime implementations. Product entry points choose a preset,
//! adapters translate configuration into a [`RuntimePolicyPatch`], and runtime
//! consumers query the resulting immutable [`ResolvedRuntimeProfile`].

mod capability;
mod profile;

pub use capability::CapabilityDecision;
pub use capability::ExternalSource;
pub use capability::ExternalSourcePolicy;
pub use capability::RuntimeExtension;
pub use capability::RuntimePreset;
pub use capability::RuntimeService;
pub use capability::ToolCapability;
pub use profile::ResolvedRuntimeProfile;
pub use profile::RuntimeCompileCeiling;
pub use profile::RuntimePolicyPatch;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
