//! Module: e_version_update
//!
//! This module provides functionality to check for a new version of the crate on crates.io
//! and to update the crate using `cargo install`.
//!
//! It includes functions for version checking and updating.
//!
//! # Examples
//!
//!
#![allow(dead_code)]

use std::error::Error;
use std::process::Command;

// Use semver for version comparisons.
// #[cfg(feature = "uses_semver")]
// use semver::Version;

// Include the crate's version number from Cargo.toml in the User-Agent.
const USER_AGENT: &str = concat!(
    "e_crate_version_checker (https://crates.io/crates/e_crate_version_checker) v",
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
    // #[cfg(feature = "uses_semver")]
    // use semver::Version;
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
        let __url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        #[cfg(all(feature = "uses_reqwest", feature = "uses_serde"))]
        {
            // println!("[TRACE] Fetching URL: {}", url);
            let client = reqwest::blocking::Client::new();
            let resp = client
                .get(&__url)
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
        //#[cfg(feature = "uses_semver")]
        //{
        //    let latest_version_str = get_latest_version(crate_name)?;
        //    // println!(
        //    //     "[TRACE] Comparing current version {} with latest {}",
        //    //     current_version, latest_version_str
        //    // );
        //    let current = Version::parse(current_version)?;
        //    let latest = Version::parse(&latest_version_str)?;
        //    Ok(latest > current)
        // }
        // #[cfg(not(feature = "uses_semver"))]
        // {
        let latest_version_str = get_latest_version(crate_name)?;
        Ok(naive_is_newer(current_version, &latest_version_str))
        // }
    }

    pub fn naive_is_newer(current: &str, latest: &str) -> bool {
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
            match l.cmp(c) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => continue,
            }
        }
        // If all compared parts are equal, the version with more components is considered newer.
        latest_parts.len() > current_parts.len()
    }
    /// Checks for an update for the current crate and prints a message if an update is available.
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    /// Looks up the version of a given crate by running `<crate_name> -v`
    /// and returning a tuple containing the binary name and its version from
    /// the first non-empty line of its output. The expected output format is
    /// "name version".
    ///
    /// Returns `Some((name, version))` if the command executes successfully,
    /// or `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use e_crate_version_checker::prelude::*;
    /// let (name, version) = local_crate_version_via_executable("cargo-e")
    ///     .expect("Could not retrieve the version");
    /// println!("Crate name: {}, version: {}", name, version);
    /// ```
    #[allow(dead_code)]
    pub fn local_crate_version_via_executable(crate_name: &str) -> Option<(String, String)> {
        // Retrieve CARGO_HOME, defaulting to "%USERPROFILE%\.cargo" on Windows
        // or "$HOME/.cargo" otherwise.
        let cargo_home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
            if cfg!(windows) {
                let userprofile = std::env::var("USERPROFILE")
                    .expect("USERPROFILE not set; cannot determine default CARGO_HOME");
                format!("{}\\.cargo", userprofile)
            } else {
                let home = std::env::var("HOME")
                    .expect("HOME not set; cannot determine default CARGO_HOME");
                format!("{}/.cargo", home)
            }
        });

        // Construct the path to the system-installed binary for the given crate.
        let mut bin_path = std::path::PathBuf::from(cargo_home);
        bin_path.push("bin");
        #[cfg(windows)]
        {
            bin_path.push(format!("{}.exe", crate_name));
        }
        #[cfg(not(windows))]
        {
            bin_path.push(crate_name);
        }

        if !bin_path.exists() {
            eprintln!(
                "System binary for {} not found at {:?}",
                crate_name, bin_path
            );
            return None;
        }

        // Run the binary with the -v flag.
        let output = std::process::Command::new(bin_path)
            .args(["-v"])
            .output()
            .ok()?;

        if !output.status.success() {
            eprintln!("{} -v failed", crate_name);
            return None;
        }

        // Convert the output bytes to a string.
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Get the first non-empty line and trim any whitespace.
        let first_line = stdout.lines().find(|line| !line.trim().is_empty())?.trim();

        // Split the first line into parts by whitespace.
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            eprintln!("Unexpected output format: {}", first_line);
            return None;
        }

        // Assume the first part is the binary name and the second part is the version.
        let name = parts[0].to_string();
        let version = parts[1].to_string();

        Some((name, version))
    }

    /// Looks up the local version of a crate in the current workspace by running `cargo metadata`.
    ///
    /// Returns `Some(version)` if a package with the given name is found, or `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `crate_name` - The name of the crate to look up.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use e_crate_version_checker::prelude::*;
    /// let version = lookup_local_version_via_cargo("mkcmt").expect("Crate not found");
    /// println!("Local version of mkcmt is {}", version);
    /// ```
    pub fn lookup_local_version_via_cargo(__crate_name: &str) -> Option<String> {
        // Run `cargo metadata` with no dependencies.
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .output()
            .ok()?;
        if !output.status.success() {
            eprintln!("cargo metadata failed");
            return None;
        }
        #[cfg(feature = "uses_serde")]
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let json: serde_json::Value = serde_json::from_str(&stdout).ok()?;
            let packages = json.get("packages")?.as_array()?;
            packages.iter().find_map(|pkg| {
                if pkg.get("name")?.as_str()? == __crate_name {
                    pkg.get("version")?.as_str().map(String::from)
                } else {
                    None
                }
            })
        }
        #[cfg(not(feature = "uses_serde"))]
        return None;
    }
}

/// When the feature `check-version` is disabled, provide stub implementations.
#[cfg(not(any(feature = "check-version", feature = "check-version-program-start")))]
pub mod version {
    /// ```rust,should_panic(expected = "Feature check-version is disabled")
    /// use e_crate_version_checker::version::get_latest_version;
    /// // This call will panic because the function returns an error.
    /// get_latest_version("any_crate").unwrap();
    /// ```
    pub fn get_latest_version(_crate_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        Err("Feature check-version is disabled".into())
    }
    /// Checks if a newer version is available compared to the provided current version.
    ///
    /// When the feature `check-version` is disabled, this function always returns an error.
    ///
    /// # Example
    ///
    /// ```rust,should_panic(expected = "Feature check-version is disabled")
    /// use e_crate_version_checker::version::is_newer_version_available;
    /// // This call will panic because the function returns an error.
    /// is_newer_version_available("1.0.0", "any_crate").unwrap();
    /// ```
    pub fn is_newer_version_available(
        _current_version: &str,
        _crate_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Err("Feature check-version is disabled".into())
    }
    /// Checks for an update for the current crate and prints a message if an update is available.
    ///
    /// When the feature `check-version` is disabled, this function always returns an error.
    ///
    /// # Example
    ///
    /// ```rust,should_panic(expected = "Feature check-version is disabled")
    /// use e_crate_version_checker::version::check_for_update;
    /// // This call will panic because the function returns an error.
    /// check_for_update().unwrap();
    /// ```
    pub fn check_for_update() -> Result<(), Box<dyn std::error::Error>> {
        Err("Feature check-version is disabled".into())
    }
    /// ```rust,no_run
    /// use e_crate_version_checker::prelude::*;
    /// let result = local_crate_version_via_executable("cargo-e");
    /// // The actual result may depend on your local environment.
    /// assert!(result.is_none());
    /// ```
    pub fn local_crate_version_via_executable(_crate_name: &str) -> Option<(String, String)> {
        None
    }
    /// ```rust,no_run
    /// use e_crate_version_checker::prelude::*;
    /// let result = lookup_local_version_via_cargo("cargo-e");
    /// // The actual result may depend on your local environment.
    /// assert!(result.is_none());
    /// ```
    pub fn lookup_local_version_via_cargo(_crate_name: &str) -> Option<String> {
        None
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
