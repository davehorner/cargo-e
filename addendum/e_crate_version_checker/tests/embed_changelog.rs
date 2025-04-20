//! Integration test to ensure the embedded changelog matches the source file
#![cfg(feature = "changelog")]

use std::fs;
use std::env;

use e_crate_version_checker::e_interactive_crate_upgrade::FULL_CHANGELOG;

#[test]
fn embed_matches_source_changelog() {
    // E_CRATE_CHANGELOG_PATH set by build script via rustc-env at compile time
    let path = env::var("E_CRATE_CHANGELOG_PATH")
        .expect("E_CRATE_CHANGELOG_PATH must be set at compile time");
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read source changelog file at '{}': {}", path, e));
    assert_eq!(FULL_CHANGELOG, src,
        "Embedded FULL_CHANGELOG did not match the contents of {}", path);
}