use std::io::IsTerminal;

use crate::e_crate_update::update_crate;
use crate::e_crate_update::version::get_latest_version;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
#[cfg(feature = "fortune")]
use rand::{seq::IteratorRandom, rng};
// Use parse-changelog to extract changelog sections when feature enabled
#[cfg(feature = "changelog")]
use parse_changelog::parse;
/// Embed consumer's changelog when "changelog" feature is enabled; path via E_CRATE_CHANGELOG_PATH env var
#[cfg(feature = "changelog")]
pub const FULL_CHANGELOG: &str = include_str!(env!("E_CRATE_CHANGELOG_PATH"));

// use crossterm::{
//     event::{poll, read, Event, KeyCode},
//     terminal::{enable_raw_mode, disable_raw_mode},
// };

// /// Reads a single key press (without waiting for a newline) from stdin
// /// with a given timeout, using crossterm in raw mode.
// ///
// /// Returns `Some(char)` if a key is pressed within the timeout,
// /// or `None` otherwise.
// ///
// /// # Example
// ///
// /// ```rust,no_run
// /// use std::time::Duration;
// ///
// /// let timeout = Duration::from_secs(5);
// /// if let Some(key) = read_key_with_timeout(timeout) {
// ///     println!("Key pressed: {}", key);
// /// } else {
// ///     println!("No key pressed within the timeout.");
// /// }
// /// ```
// fn read_key_with_timeout(timeout: Duration) -> Option<char> {
//     // Enable raw mode so key presses are captured immediately.
//     if enable_raw_mode().is_err() {
//         return None;
//     }

//     let result = if poll(timeout).ok()? {
//         // We have an event; try to read it.
//         if let Ok(Event::Key(event)) = read() {
//             match event.code {
//                 // Return the character if it's a regular char key.
//                 KeyCode::Char(c) => Some(c),
//                 // You can also handle other key codes here if needed.
//                 _ => None,
//             }
//         } else {
//             None
//         }
//     } else {
//         // No event was received within the timeout.
//         None
//     };

//     // Disable raw mode before returning.
//     let _ = disable_raw_mode();
//     result
// }

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
    wait: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Allow overriding the current version for testing (e.g., force update flow)
    let current_version = std::env::var("E_CRATE_CURRENT_VERSION").unwrap_or_else(|_| current_version.to_string());
    // Skip terminal check if forced (for testing)
    if std::env::var("E_CRATE_FORCE_INTERACTIVE").is_err() && !std::io::stdin().is_terminal() {
        println!("Non-interactive mode detected; skipping update prompt.");
        return Ok(());
    }
    // If fortune feature is enabled, display a random line from the consumer's fortunes file
    #[cfg(feature = "fortune")]
    {
        let data = include_str!(env!("E_CRATE_FORTUNE_PATH"));
        let mut rng = rng();
        if let Some(line) = data.lines().filter(|l| !l.trim().is_empty()).choose(&mut rng) {
            println!("{}", line);
        }
    }

    // Retrieve the latest version from crates.io, handling missing crates gracefully
    let latest_version = match get_latest_version(crate_name) {
        Ok(v) => v,
        Err(e) => {
            // Print only the error message
            eprintln!("{}", e);
            return Ok(());
        }
    };
    if current_version == "0.0.0" {
        print!("'{}' is not installed.", crate_name);
    } else if let (
        Some((cur_major, cur_minor, cur_patch)),
        Some((lat_major, lat_minor, lat_patch)),
    ) = (
        parse_version(&current_version),
        parse_version(&latest_version),
    ) {
        let current_tuple = (cur_major, cur_minor, cur_patch);
        let latest_tuple = (lat_major, lat_minor, lat_patch);
                #[cfg(feature = "changelog")]
                {
    if latest_version != current_version {
                    let changelog_str = FULL_CHANGELOG;
                    match parse(changelog_str) {
                        Ok(cl) => {
                            if let Some(release) = cl.get(latest_version.as_str()) {
                                let filtered_notes = release
    .notes
    .lines()
    .filter(|line| !line.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n");
                                println!("---\nversion {}:\n{}\n---", latest_version, filtered_notes);
                            } else {
                                println!("\nCould not find changelog section for version {}", latest_version);
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to parse changelog: {}", err);
                        }
                    }
                }
            }
        if current_tuple == latest_tuple {
            // // Up-to-date: either print fortune or notice depending on feature
            // #[cfg(feature = "fortune")]
            // {
            //     let data = include_str!(env!("E_CRATE_FORTUNE_PATH"));
            //     let mut rng = thread_rng();
            //     if let Some(line) = data.lines().filter(|l| !l.trim().is_empty()).choose(&mut rng) {
            //         println!("{}", line);
            //     }
            // }
            // #[cfg(not(feature = "fortune"))]
            // {
                println!("'{}' {} is latest.", crate_name, current_version);
            // }
            return Ok(());
        } else if current_tuple > latest_tuple {
            println!(
                "ahead of the latest published version for {}: {} > {}",
                crate_name, current_version, latest_version
            );
            return Ok(());
        } else if lat_major > cur_major {
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
    } else if latest_version != current_version {
        print!(
            "'{}' new version available. current({}) -> latest({})",
            crate_name, current_version, latest_version
        );
    } else {
            println!("'{}' up-to-date! {}", crate_name, current_version);
            return Ok(());
    }

    // Compare versions and prompt the user accordingly.
    println!(" want to install? [Yes/no] (wait {} seconds)", wait);
    std::io::Write::flush(&mut std::io::stdout())?;

    // let mut input = String::new();
    // if std::io::stdin().read_line(&mut input).is_ok() {
    if let Some(input) = read_line_with_timeout(Duration::from_secs(wait)) {
        let input = input.trim().to_lowercase();
        if input == "y" || input.is_empty() {
            // Support dry-run via E_CRATE_DRY_RUN
            let dry_run = std::env::var("E_CRATE_DRY_RUN").is_ok();
            let _success = if dry_run {
                println!("Update complete (dry-run).");
                true
            } else {
                match update_crate(crate_name, &latest_version) {
                    Ok(()) => {
                        println!("Update complete.");
                        true
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        false
                    }
                }
            };
            std::process::exit(0);
        } else {
            println!("Update canceled.");
        }
    }
    Ok(())
}
