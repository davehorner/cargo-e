use assert_cmd::Command;
use cargo_e::e_features::{get_feature_flags, get_feature_flags_json};
use predicates::prelude::*;
use predicates::str::contains;

#[test]
fn test_version_feature_flags() {
    let mut cmd = Command::cargo_bin("cargo-e").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        // Check that the output starts with "cargo-e " and contains a JSON array.
        .stdout(contains("cargo-e "))
        .stdout(contains("["))
        .stdout(contains("]"))
        // For equivalent configuration, expect "equivalent" and not "!equivalent".
        .stdout(if cfg!(feature = "equivalent") {
            contains("\"equivalent\"").and(contains("\"!equivalent\"").not())
        } else {
            // Otherwise, expect "!equivalent" and not "equivalent".
            contains("\"!equivalent\"").and(contains("\"equivalent\"").not())
        });
}

#[test]
fn test_feature_flags_compact() {
    let flags = get_feature_flags();
    let json = get_feature_flags_json();

    if cfg!(feature = "equivalent") {
        // When equivalent is enabled, we expect "equivalent" to be present and "!equivalent" not.
        assert!(
            flags.contains(&"equivalent"),
            "Expected 'equivalent' to be present when equivalent is enabled"
        );
        assert!(
            !flags.contains(&"!equivalent"),
            "Did not expect '!equivalent' when equivalent is enabled"
        );
        assert!(
            json.contains("\"equivalent\""),
            "Expected JSON to contain \"equivalent\""
        );
        assert!(
            !json.contains("\"!equivalent\""),
            "Did not expect JSON to contain \"!equivalent\" when equivalent is enabled"
        );
    } else {
        // When equivalent is disabled, we expect "!equivalent" to be present and "equivalent" not.
        assert!(
            flags.contains(&"!equivalent"),
            "Expected '!equivalent' to be present when equivalent is disabled"
        );
        assert!(
            !flags.contains(&"equivalent"),
            "Did not expect 'equivalent' when equivalent is disabled"
        );
        assert!(
            json.contains("\"!equivalent\""),
            "Expected JSON to contain \"!equivalent\""
        );
        assert!(
            !json.contains("\"equivalent\""),
            "Did not expect JSON to contain \"equivalent\" when equivalent is disabled"
        );
    }
}

#[cfg(feature = "equivalent")]
#[test]
fn test_feature_flags_equivalent_enabled() {
    let flags = get_feature_flags();
    // When "equivalent" is enabled, the returned vector should include "equivalent" and not "!equivalent".
    assert!(
        flags.contains(&"equivalent"),
        "Expected 'equivalent' to be present in feature flags when equivalent is enabled, got {:?}",
        flags
    );
    assert!(
        !flags.contains(&"!equivalent"),
        "Expected '!equivalent' not to be present when equivalent is enabled, got {:?}",
        flags
    );
}

#[cfg(not(feature = "equivalent"))]
#[test]
fn test_feature_flags_equivalent_disabled() {
    let flags = get_feature_flags();
    // When "equivalent" is disabled, the returned vector should include "!equivalent" and not "equivalent".
    assert!(
        flags.contains(&"!equivalent"),
        "Expected '!equivalent' to be present in feature flags when equivalent is disabled, got {:?}",
        flags
    );
    assert!(
        !flags.contains(&"equivalent"),
        "Expected 'equivalent' not to be present when equivalent is disabled, got {:?}",
        flags
    );
}
