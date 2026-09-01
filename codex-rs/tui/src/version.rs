/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "buddy-branding")]
pub(crate) const PRODUCT_DISPLAY_NAME: &str = "Codex Buddy";
#[cfg(feature = "buddy-branding")]
pub(crate) const PRODUCT_DISPLAY_VERSION: &str = "1.0.9";

#[cfg(not(feature = "buddy-branding"))]
pub(crate) const PRODUCT_DISPLAY_NAME: &str = "OpenAI Codex";
#[cfg(not(feature = "buddy-branding"))]
pub(crate) const PRODUCT_DISPLAY_VERSION: &str = CODEX_CLI_VERSION;
