//! Library for `e_crate_version_checker`.
//!
//! Provides functionality to query crates.io for crate version information.

pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod e_crate_update;
pub mod e_interactive_crate_upgrade;

// Create a prelude module that re-exports key items
pub mod prelude {
    pub use crate::e_crate_update::user_agent::{get_user_agent, set_user_agent_override};
    pub use crate::e_crate_update::version::local_crate_version_via_executable;
    pub use crate::e_crate_update::version::lookup_local_version_via_cargo;
    pub use crate::e_crate_update::*;
    pub use crate::e_interactive_crate_upgrade::*;
    pub use crate::register_user_crate;
    pub use crate::LIB_VERSION;
}

// Tests for changelog parsing
#[cfg(all(test, feature = "changelog"))]
mod tests {
    use super::*;
    use parse_changelog::parse;
    #[test]
    fn test_changelog_contains_latest_section() {
        // FULL_CHANGELOG should be included at compile time
        let changelog = e_interactive_crate_upgrade::FULL_CHANGELOG;
        let parsed = parse(changelog).expect("Failed to parse FULL_CHANGELOG");
        // Expect a known version from the consumer's changelog
        let version = "0.2.14";
        assert!(parsed.get(version).is_some(),
            "Changelog should contain section for version {}", version);
        let notes = &parsed.get(version).unwrap().notes;
        // Check for a known substring from that section
        assert!(notes.contains("rust-script / scriptisto kind detection"),
            "Changelog notes for version {} should contain expected text", version);
    }
}

/// A macro that registers the caller’s crate name in the user agent string.
/// This macro captures the caller's crate name using `env!("CARGO_PKG_NAME")`.
#[macro_export]
macro_rules! register_user_crate {
    () => {{
        let ua = format!(
            "e_crate_version_checker (https://crates.io/crates/e_crate_version_checker) v{} [used by {} v{}]",
            $crate::LIB_VERSION,        // This always uses the library's version.
            env!("CARGO_PKG_NAME"),        // This will still be the consumer's crate name.
            env!("CARGO_PKG_VERSION")     // This will still be the consumer's crate version.
        );
        $crate::e_crate_update::user_agent::set_user_agent_override(ua);
    }};
}
