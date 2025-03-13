//! Module: e_version_update
//!
//! This module provides functionality to check for a new version of the crate on crates.io
//! and to update the crate using `cargo install`.
//!
//! It includes functions for version checking and updating.
//!
//! # Examples
//!
//! **Version checking:** (requires features `check-version`, `uses_reqwest`, `uses_serde`, and optionally `uses_semver`)
//! ```rust,no_run
//! use e_crate_version_checker::e_crate_update::version::{get_latest_version, is_newer_version_available, check_for_update};
//!
//! let latest = get_latest_version("cargo-e").expect("Failed to get version");
//! println!("Latest version: {}", latest);
//!
//! if is_newer_version_available(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_NAME")).unwrap() {
//!     println!("A new version is available!");
//! }
//!
//! // Or simply:
//! check_for_update().expect("Failed to check for update");
//! ```
//!
//! **Updating the crate:**  
//! ```rust,no_run
//! use cargo_e::e_version_update::{build_update_args, update_crate};
//! use e_crate_version_checker::e_crate_update::version::{build_update_args, update_crate};
//!
//! let args = build_update_args("cargo-e", "0.1.5");
//! println!("Update arguments: {:?}", args);
//! update_crate("cargo-e", "0.1.5").expect("Update failed");
//! ```

use std::error::Error;
use std::process::Command;

// Use semver for version comparisons.
// #[cfg(feature = "uses_semver")]
// use semver::Version;

// Include the crate's version number from Cargo.toml in the User-Agent.
const USER_AGENT: &str = concat!(
    "cargo-e (https://crates.io/crates/cargo-e) v",
    env!("CARGO_PKG_VERSION")
);

/// --- Version Checking Functions ---
///
/// These functions are provided in the submodule `version`.
/// They are only available when the feature `check-version` is enabled.
#[cfg(any(feature = "check-version", feature = "check-version-program-start"))]
pub mod version {
    use super::*;
    // Only compile the following if the required sub-features are enabled.
    #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
    use reqwest;
    #[cfg(feature = "uses_semver")]
    use semver::Version;
    #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
    use serde::Deserialize;

    /// Structure representing the crate information returned from crates.io.
    #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
    #[derive(Deserialize, Debug)]
    struct CrateInfo {
        /// The latest published version of the crate.
        max_version: String,
    }

    /// Structure representing the API response from crates.io.
    #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
    #[derive(Deserialize, Debug)]
    struct CrateResponse {
        #[serde(rename = "crate")]
        krate: CrateInfo,
    }

    /// Retrieves the latest version of the specified crate from crates.io.
    ///
    /// # Arguments
    ///
    /// * `crate_name` - The name of the crate (e.g., `"cargo-e"`).
    ///
    /// # Returns
    ///
    /// On success, returns the latest version as a `String`.
    pub fn get_latest_version(crate_name: &str) -> Result<String, Box<dyn Error>> {
        #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
        {
            let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
            // println!("[TRACE] Fetching URL: {}", url);
            let client = reqwest::blocking::Client::new();
            let resp = client
                .get(&url)
                .header(reqwest::header::USER_AGENT, "cargo-e")
                .send()?;
            // println!("[TRACE] Received response: {:?}", resp.status());
            let resp = resp; // json() requires a mutable reference.
            let crate_response: CrateResponse = resp.json()?;
            // println!(
            //     "[TRACE] Parsed response: latest version is {}",
            //     crate_response.krate.max_version
            // );
            Ok(crate_response.krate.max_version)
        }
        #[cfg(not(all(feature = "uses_reqwest", feature = "uses_serde")))]
        {
            Err("Required features (uses_reqwest and uses_serde) are not enabled".into())
        }
    }

    /// Checks if a newer version is available compared to the provided current version.
    ///
    /// # Arguments
    ///
    /// * `current_version` - The current version as a string slice.
    /// * `crate_name` - The name of the crate to check.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the latest version is greater than the current version; otherwise returns `Ok(false)`.
    pub fn is_newer_version_available(
        current_version: &str,
        crate_name: &str,
    ) -> Result<bool, Box<dyn Error>> {
        #[cfg(feature = "uses_semver")]
        {
            let latest_version_str = get_latest_version(crate_name)?;
            // println!(
            //     "[TRACE] Comparing current version {} with latest {}",
            //     current_version, latest_version_str
            // );
            let current = Version::parse(current_version)?;
            let latest = Version::parse(&latest_version_str)?;
            Ok(latest > current)
        }
        #[cfg(not(feature = "uses_semver"))]
        {
            let latest_version_str = get_latest_version(crate_name)?;
            Ok(naive_is_newer(current_version, &latest_version_str))
        }
    }

    fn naive_is_newer(current: &str, latest: &str) -> bool {
        // Split the version strings on '.'
        let current_parts: Vec<u32> = current
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();
        let latest_parts: Vec<u32> = latest
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();

        // Compare each corresponding part
        for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
            if l > c {
                return true;
            } else if l < c {
                return false;
            }
        }
        // If all compared parts are equal, the version with more components is considered newer.
        latest_parts.len() > current_parts.len()
    }
    /// Checks for an update for the current crate and prints a message if an update is available.
    pub fn check_for_update() -> Result<(), Box<dyn Error>> {
        let current = env!("CARGO_PKG_VERSION");
        let crate_name = env!("CARGO_PKG_NAME");
        // println!("[TRACE] Current version of {} is {}", crate_name, current);
        if is_newer_version_available(current, crate_name)? {
            let latest = get_latest_version(crate_name)?;
            println!(
                "A new version of {} is available! Current: {}, Latest: {}.",
                crate_name, current, latest
            );
        } else {
            println!("You are running the latest version of {}.", crate_name);
        }
        Ok(())
    }
    pub fn check_for_update_for(
        check_crate_name: &str,
        current_version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // println!(
        //     "[TRACE] Current version of {} is {}",
        //     check_crate_name, current_version
        // );
        if is_newer_version_available(current_version, check_crate_name)? {
            let latest = get_latest_version(check_crate_name)?;
            println!(
                "A new version of {} is available! Current: {}, Latest: {}.",
                check_crate_name, current_version, latest
            );
        } else {
            println!(
                "You are running the latest version of {}.",
                check_crate_name
            );
        }
        Ok(())
    }
}

/// When the feature `check-version` is disabled, provide stub implementations.
#[cfg(not(any(feature = "check-version", feature = "check-version-program-start")))]
pub mod version {
    use super::*;
    pub fn get_latest_version(_crate_name: &str) -> Result<String, Box<dyn Error>> {
        Err("Feature check-version is disabled".into())
    }
    pub fn is_newer_version_available(
        _current_version: &str,
        _crate_name: &str,
    ) -> Result<bool, Box<dyn Error>> {
        Err("Feature check-version is disabled".into())
    }
    pub fn check_for_update() -> Result<(), Box<dyn Error>> {
        Err("Feature check-version is disabled".into())
    }
}

/// --- Update Functions ---
///
/// These functions help update the crate using `cargo install`.

/// Constructs the arguments for updating the crate.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to update.
/// * `latest_version` - The version to update to.
///
/// # Returns
///
/// A vector of arguments that can be used with `cargo install`.
pub fn build_update_args(crate_name: &str, latest_version: &str) -> Vec<String> {
    // println!(
    //     "[TRACE] Building update args for {} to version {}",
    //     crate_name, latest_version
    // );
    vec![
        "install".to_string(),
        crate_name.to_string(),
        "--force".to_string(),
        "--version".to_string(),
        latest_version.to_string(),
    ]
}

/// Updates the specified crate to the given version using `cargo install`.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to update.
/// * `latest_version` - The version to update to.
///
/// # Returns
///
/// A `Result` indicating whether the update succeeded.
pub fn update_crate(crate_name: &str, latest_version: &str) -> Result<(), Box<dyn Error>> {
    let args = build_update_args(crate_name, latest_version);
    // println!("[TRACE] Running cargo install with args: {:?}", args);
    let status = Command::new("cargo").args(&args).status()?;
    if status.success() {
        // println!(
        //     "[TRACE] Successfully updated {} to version {}.",
        //     crate_name, latest_version
        // );
        Ok(())
    } else {
        Err(format!(
            "Failed to update {} to version {}.",
            crate_name, latest_version
        )
        .into())
    }
}
pub fn show_current_version() -> &'static str {
    USER_AGENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_update_args() {
        let args = build_update_args("cargo-e", "0.1.5");
        let expected = vec!["install", "cargo-e", "--force", "--version", "0.1.5"];
        for (a, b) in args.iter().zip(expected.iter()) {
            assert_eq!(a, b);
        }
    }

    // The following tests require that the check-version feature and its sub-features are enabled.
    #[cfg(feature = "check-version")]
    mod version_tests {
        use super::super::version;
        #[test]
        fn test_get_latest_version_valid_crate() {
            let result = version::get_latest_version("cargo-e");
            assert!(result.is_ok());
            let version_str = result.unwrap();
            assert!(
                !version_str.is_empty(),
                "Version string should not be empty"
            );
        }
        #[test]
        fn test_get_latest_version_invalid_crate() {
            let result = version::get_latest_version("non-existent-crate-123456");
            assert!(result.is_err(), "Should return an error for invalid crate");
        }
        #[test]
        fn test_check_for_update() {
            // This test simply calls check_for_update to exercise its functionality.
            let _ = version::check_for_update();
        }
    }
}
