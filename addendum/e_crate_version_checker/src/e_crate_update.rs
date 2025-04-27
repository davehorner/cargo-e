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

pub mod user_agent {
    use std::sync::OnceLock;
    static USER_AGENT_OVERRIDE: OnceLock<String> = OnceLock::new();

    /// Returns the current user agent string.
    /// If the user hasn’t registered a custom crate name, falls back to the default.
    pub fn get_user_agent() -> String {
        USER_AGENT_OVERRIDE.get().cloned().unwrap_or_else(|| {
            format!(
                "e_crate_version_checker (https://crates.io/crates/e_crate_version_checker) v{}",
                crate::LIB_VERSION
            )
        })
    }
    pub fn get_user_agent_checked() -> String {
        let ua = get_user_agent();
        if !ua.contains("[used by") {
            panic!("User agent not overridden. Please call register_user_crate!() in your crate.");
        }
        ua
    }

    /// Sets the user agent string to include the caller’s crate name.
    /// This function is intended to be used via the `register_user_crate!()` macro.
    pub fn set_user_agent_override(ua: String) {
        let _ = USER_AGENT_OVERRIDE.set(ua);
    }
}

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
                .header(
                    reqwest::header::USER_AGENT,
                    user_agent::get_user_agent_checked(),
                )
                .send()?;
            let status = resp.status();
            if !status.is_success() {
                // Handle crate not found vs other HTTP errors
                if status.as_u16() == 404 {
                    return Err(format!("crate '{}' not found on crates.io", crate_name).into());
                } else {
                    return Err(format!(
                        "HTTP error {} fetching crate info for '{}'",
                        status, crate_name
                    )
                    .into());
                }
            }
            // Parse JSON body for crate info
            let crate_response: CrateResponse = resp.json()?;
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

#[cfg(windows)]
use std::{env, fs, io::Write};

// #[cfg(windows)]
// /// Spawns a helper process (a temporary batch file) that waits for the current process
// /// to exit and then runs `cargo install --force <crate_name> --version <latest_version>`.
// /// The batch file deletes itself afterward.
// fn spawn_self_update(crate_name: &str, latest_version: &str) -> Result<(), Box<dyn Error>> {
//     // Get the current process ID (which we'll wait to disappear).
//     let parent_pid = std::process::id();

//     // Create a temporary file path for the batch file.
//     let mut batch_path = env::temp_dir();
//     // We add a random component if needed to avoid collisions.
//     batch_path.push(format!("cargo_e_update_{}.bat", parent_pid));

//     // The batch file will loop until the parent process is gone, then run the install command,
//     // and finally delete itself.
//     let batch_contents = format!(
//         "@echo off\r\n\
//          :wait_loop\r\n\
//          tasklist /FI \"PID eq {}\" | findstr /I \"{}\" >nul\r\n\
//          if %ERRORLEVEL%==0 (\r\n\
//            timeout /T 1 >nul\r\n\
//            goto wait_loop\r\n\
//          )\r\n\
//          echo Parent process exited. Running update...\r\n\
//          cargo install --force {} --version {}\r\n\
//          del \"%~f0\"\r\n",
//         parent_pid, parent_pid, crate_name, latest_version
//     );

//     // Write the batch file.
//     {
//         let mut file = fs::File::create(&batch_path)?;
//         file.write_all(batch_contents.as_bytes())?;
//     }

//     // Spawn the batch file in a detached process.
//     // The empty string after "start" sets the window title.
//     Command::new("cmd")
//         .args(&["/C", "start", "", batch_path.to_str().unwrap()])
//         .spawn()?;

//     Ok(())
// }

#[cfg(not(windows))]
pub fn update_crate(crate_name: &str, latest_version: &str) -> Result<(), Box<dyn Error>> {
    let args = build_update_args(crate_name, latest_version);
    let status = Command::new("cargo").args(&args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to update {} to version {}.",
            crate_name, latest_version
        )
        .into())
    }
}

#[cfg(windows)]
pub fn update_crate(crate_name: &str, latest_version: &str) -> Result<(), Box<dyn Error>> {
    // Instead of attempting to update directly (which will fail because the binary is locked),
    // we spawn a helper process that waits for the current process to exit and then performs the update.
    spawn_self_update(crate_name, latest_version)?;
    Err(format!(
        "Spawned updater for {} version {}. Please allow the {} update to complete.\nIf you have trouble, perform the update manually with `cargo install --force {}`.",
        crate_name, latest_version, crate_name, crate_name
    )
    .into())
}

#[cfg(windows)]
fn spawn_self_update(crate_name: &str, latest_version: &str) -> Result<(), Box<dyn Error>> {
    let parent_pid = std::process::id();

    // Construct the PowerShell command to wait for the parent process to exit, then run the update.
    let ps_command = format!(
        "while (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ Start-Sleep -Seconds 1 }}; \
         Write-Output 'Parent process exited. Running update...'; \
         cargo install --force {crate} --version {version}",
        pid = parent_pid,
        crate = crate_name,
        version = latest_version
    );

    // Launch PowerShell in its own window via cmd /C start.
    let ps_result = Command::new("cmd")
        .args(&[
            "/C",
            "start",
            "", // This is the window title; change as desired.
            "powershell",
            "-NoProfile",
            "-Command",
            &ps_command,
        ])
        .spawn();

    match ps_result {
        Ok(_child) => Ok(()),
        Err(e) => {
            eprintln!(
                "PowerShell failed to spawn: {}. Falling back to batch file method.",
                e
            );

            let mut batch_path = env::temp_dir();
            batch_path.push(format!("update_{}_{}.bat", crate_name, parent_pid));

            let batch_contents = format!(
                "@echo off\r\n\
                 :wait_loop\r\n\
                 tasklist /FI \"PID eq {}\" | findstr /I \"{}\" >nul\r\n\
                 if %ERRORLEVEL%==0 (\r\n\
                   timeout /T 1 >nul\r\n\
                   goto wait_loop\r\n\
                 )\r\n\
                 echo Parent process exited. Running update...\r\n\
                 cargo install --force {} --version {}\r\n",
                parent_pid, parent_pid, crate_name, latest_version
            );

            eprintln!("Updater batch file written to: {}", batch_path.display());
            {
                let mut file = fs::File::create(&batch_path)?;
                file.write_all(batch_contents.as_bytes())?;
            }

            let batch_path_str = format!("{}", batch_path.display());
            // Launch the batch file in its own window via cmd /C start.
            Command::new("cmd")
                .args(&["/C", "start", "", &batch_path_str])
                .spawn()?;

            Ok(())
        }
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_build_update_args() {
//         let args = build_update_args("cargo-e", "0.1.5");
//         let expected = vec!["install", "cargo-e", "--force", "--version", "0.1.5"];
//         for (a, b) in args.iter().zip(expected.iter()) {
//             assert_eq!(a, b);
//         }
//     }

//     // The following tests require that the check-version feature and its sub-features are enabled.
//     #[cfg(feature = "check-version")]
//     mod version_tests {
//         use super::super::version;
//         #[test]
//         fn test_get_latest_version_valid_crate() {
// 	    crate::register_user_crate!();
//             let result = version::get_latest_version("cargo-e");
//             assert!(result.is_ok());
//             let version_str = result.unwrap();
//             assert!(
//                 !version_str.is_empty(),
//                 "Version string should not be empty"
//             );
//         }
//         #[test]
//         fn test_get_latest_version_invalid_crate() {
// 	    crate::register_user_crate!();
//             let result = version::get_latest_version("non-existent-crate-123456");
//             assert!(result.is_err(), "Should return an error for invalid crate");
//         }
//         #[test]
//         fn test_check_for_update() {
// 	    crate::register_user_crate!();
//             // This test simply calls check_for_update to exercise its functionality.
//             let _ = version::check_for_update();
//         }
//     }
// }
