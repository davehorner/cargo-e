<<<<<<< HEAD
use std::io::{IsTerminal, Write};
use crate::e_crate_update::update_crate;
use crate::e_crate_update::version::get_latest_version;


use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn read_line_with_timeout(timeout: Duration) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let _ = tx.send(input);
    });
    rx.recv_timeout(timeout).ok()
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    let patch = parts[2].parse::<u32>().ok()?;
    Some((major, minor, patch))
}


pub fn interactive_crate_upgrade(
    crate_name: &str,
    current_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {

    if !std::io::stdin().is_terminal() {
        println!("Non-interactive mode detected; skipping update prompt.");
        return Ok(());
    }

    // Retrieve the latest version from crates.io.
    let latest_version = get_latest_version(crate_name)?;
    if current_version == "0.0.0" {
        print!("'{}' is not installed.", crate_name);
    } else if let (Some((cur_major, cur_minor, cur_patch)), Some((lat_major, lat_minor, lat_patch))) =
        (parse_version(current_version), parse_version(&latest_version))
    {
        let current_tuple = (cur_major, cur_minor, cur_patch);
        let latest_tuple = (lat_major, lat_minor, lat_patch);
        if current_tuple == latest_tuple {
            println!("'{}' {} is latest.", crate_name, current_version);
            return Ok(());
        } else if current_tuple > latest_tuple {
            print!(
                "ahead of the latest published version for {}: {} > {}",
                crate_name, current_version, latest_version
            );
            return Ok(());
        } else {
            if lat_major > cur_major {
                print!(
                    "major update for {}: {} -> {}",
                    crate_name, current_version, latest_version
                );
            } else if lat_minor > cur_minor {
                print!(
                    "minor update for {}: {} -> {}",
                    crate_name, current_version, latest_version
                );
            } else if lat_patch > cur_patch {
                print!(
                    "patch update for {}: {} -> {}",
                    crate_name, current_version, latest_version
                );
            }
        }
    } else if latest_version != current_version {
        print!(
            "'{}' new version available. {} -> {}",
            crate_name, latest_version, current_version
=======
use std::error::Error;
use std::io::{self, Write};
use std::process;

// Assume these functions are available from your module.
use crate::e_crate_update::update_crate;
use crate::e_crate_update::version::get_latest_version;

/// Interactively checks for a newer version of a crate and prompts the user to update it.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to check.
/// * `current_version` - The current version of the crate.
///
/// # Returns
///
/// Returns `Ok(())` if the process completes successfully, or an error otherwise.
///
/// # Example
///
/// ```rust,no_run
/// use e_crate_version_checker::e_interactive_crate_upgrade::interactive_crate_upgrade; 
/// interactive_crate_upgrade("mkcmt", "0.1.0").expect("Upgrade process failed");
/// ```
pub fn interactive_crate_upgrade(
    crate_name: &str,
    current_version: &str,
) -> Result<(), Box<dyn Error>> {
    // Retrieve the latest version from crates.io.
    let latest_version = get_latest_version(crate_name)?;
    if current_version == "0.0.0" {
        println!("'{}' is not installed.", crate_name);
    } else if latest_version != current_version {
        println!(
            "'{}'  new version available. {}",
            crate_name, latest_version
>>>>>>> develop
        );
    } else {
        println!("'{}' up-to-date! {}", crate_name, current_version);
        return Ok(());
    }

    // Compare versions and prompt the user accordingly.
<<<<<<< HEAD
    println!("want to install? [Y/n] (wait 3 seconds)");
    std::io::stdout().flush()?;

    // let mut input = String::new();
    // if std::io::stdin().read_line(&mut input).is_ok() {
if let Some(input) = read_line_with_timeout(Duration::from_secs(3)) {
=======
    println!("Do you want to install it? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
>>>>>>> develop
        let input = input.trim().to_lowercase();
        if input == "y" || input.is_empty() {
            match update_crate(crate_name, &latest_version) {
                Ok(()) => println!("Update complete."),
                Err(e) => eprintln!("Update failed: {}", e),
            }
        } else {
            println!("Update canceled.");
        }
    } else {
        eprintln!("Failed to read input.");
<<<<<<< HEAD
        std::process::exit(1);
    }
    Ok(())
}



// pub fn interactive_crate_upgrade(
//     crate_name: &str,
//     current_version: &str,
// ) -> Result<(), Box<dyn std::error::Error>> {

//     // Check if running in an interactive terminal.
//     if !atty::is(atty::Stream::Stdin) {
//         println!("Non-interactive mode detected; skipping update prompt.");
//         return Ok(());
//     }

//     // Retrieve the latest version from crates.io.
//     let latest_version = get_latest_version(crate_name)?;
//     if current_version == "0.0.0" {
//         print!("'{}' is not installed.", crate_name);
//     } else if let (Some((cur_major, cur_minor, cur_patch)), Some((lat_major, lat_minor, lat_patch))) =
//         (parse_version(current_version), parse_version(&latest_version))
//     {
//         if lat_major > cur_major {
//             print!("Major update available for {}: {} -> {}", crate_name, current_version, latest_version);
//         } else if lat_minor > cur_minor {
//             print!("Minor update available for {}: {} -> {}", crate_name, current_version, latest_version);
//         } else if lat_patch > cur_patch {
//             print!("Patch update available for {}: {} -> {}", crate_name, current_version, latest_version);
//         } else {
//             println!("'{}' up-to-date! {}", crate_name, current_version);
//             return Ok(());
//         }
//     } else if latest_version != current_version {
//         print!(
//             "'{}'  new version available. {} -> {}",
//             crate_name, latest_version, current_version
//         );
//     } else {
//         print!("'{}' up-to-date! {}", crate_name, current_version);
//         return Ok(());
//     }

//     // Compare versions and prompt the user accordingly.
//     println!(" want to install it? [Y/n] (wait 3 seconds) ");
//     std::io::stdout().flush()?;


//     // Wait for input for up to 10 seconds.
//     if let Some(input) = read_line_with_timeout(Duration::from_secs(3)) {
//         let input = input.trim().to_lowercase();
//         if input == "y" || input.is_empty() {
//             match update_crate(crate_name, &latest_version) {
//                 Ok(()) => println!("Update complete."),
//                 Err(e) => eprintln!("Update failed: {}", e),
//             }
//         } else {
//             println!("ok.");
//         }


//     // let mut input = String::new();
//     // if std::io::stdin().read_line(&mut input).is_ok() {
//     //     let input = input.trim().to_lowercase();
//     //     if input == "y" || input.is_empty() {
//     //         match update_crate(crate_name, &latest_version) {
//     //             Ok(()) => println!("Update complete."),
//     //             Err(e) => eprintln!("Update failed: {}", e),
//     //         }
//     //     } else {
//     //         println!("Update canceled.");
//     //     }
//     }
//     Ok(())
// }


// // /// Interactively checks for a newer version of a crate and prompts the user to update it.
// // ///
// // /// # Arguments
// // ///
// // /// * `crate_name` - The name of the crate to check.
// // /// * `current_version` - The current version of the crate.
// // ///
// // /// # Returns
// // ///
// // /// Returns `Ok(())` if the process completes successfully, or an error otherwise.
// // ///
// // /// # Example
// // ///
// // /// ```rust,no_run
// // /// use e_crate_version_checker::e_interactive_crate_upgrade::interactive_crate_upgrade; 
// // /// interactive_crate_upgrade("mkcmt", "0.1.0").expect("Upgrade process failed");
// // /// ```
// // pub fn interactive_crate_upgrade(
// //     crate_name: &str,
// //     current_version: &str,
// // ) -> Result<(), Box<dyn Error>> {
// //     // Retrieve the latest version from crates.io.
// //     let latest_version = get_latest_version(crate_name)?;
// //     if current_version == "0.0.0" {
// //         println!("'{}' is not installed.", crate_name);
// //     } else if latest_version != current_version {
// //         println!(
// //             "'{}'  new version available. {} -> {}",
// //             crate_name, latest_version, current_version
// //         );
// //     } else {
// //         println!("'{}' up-to-date! {}", crate_name, current_version);
// //         return Ok(());
// //     }

// //     // Compare versions and prompt the user accordingly.
// //     println!("Do you want to install it? [Y/n] ");
// //     io::stdout().flush()?;

// //     let mut input = String::new();
// //     if io::stdin().read_line(&mut input).is_ok() {
// //         let input = input.trim().to_lowercase();
// //         if input == "y" || input.is_empty() {
// //             match update_crate(crate_name, &latest_version) {
// //                 Ok(()) => println!("Update complete."),
// //                 Err(e) => eprintln!("Update failed: {}", e),
// //             }
// //         } else {
// //             println!("Update canceled.");
// //         }
// //     } else {
// //         eprintln!("Failed to read input.");
// //         process::exit(1);
// //     }
// //     Ok(())
// // }


=======
        process::exit(1);
    }
    Ok(())
}
>>>>>>> develop
