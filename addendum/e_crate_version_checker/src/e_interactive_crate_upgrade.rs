use std::io::IsTerminal;
//use crate::e_crate_update::update_crate;
//use e_crate_version_checker::get_latest_version;
//use crate::e_crate_update::version::get_latest_version;
//use e_crate_version_checker::e_crate_update;
//use crate::e_crate_update::version::get_latest_version;
//use crate::e_crate_update::update_crate;
// If the build script sets the inlined flag:
// When inlined, the addendum code is available under the external module path.
//
#[cfg(feature = "addendum_inline")]
use e_crate_version_checker::e_crate_update::version::get_latest_version;
//#[cfg(feature = "addendum_inline")]
//use e_crate_version_checker::e_crate_update;
#[cfg(feature = "addendum_inline")]
use e_crate_version_checker::e_crate_update::update_crate;

// Otherwise, use the default path:
#[cfg(not(feature = "addendum_inline"))]
use crate::e_crate_update::version::get_latest_version;
#[cfg(not(feature = "addendum_inline"))]
use crate::e_crate_update::update_crate;


use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

fn should_check_for_update() -> bool {
    // If CARGOE_NO_UPDATE is set to "1" or "true", then skip checking for update.
    !std::env::var("CARGOE_NO_UPDATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}


pub fn interactive_crate_upgrade(
    crate_name: &str,
    current_version: &str,
    wait: u64,
) -> Result<(), Box<dyn std::error::Error>> {

    if should_check_for_update() {
        return Ok(());
    }
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
            println!(
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
        );
    } else {
        println!("'{}' up-to-date! {}", crate_name, current_version);
        return Ok(());
    }

    // Compare versions and prompt the user accordingly.
    print!(" want to install? [Y/n] (wait {} seconds)",wait);
    std::io::Write::flush(&mut std::io::stdout())?;

    // let mut input = String::new();
    // if std::io::stdin().read_line(&mut input).is_ok() {
    if let Some(input) = read_line_with_timeout(Duration::from_secs(wait)) {
        let input = input.trim().to_lowercase();
        if input == "y" || input.is_empty() {
            match update_crate(crate_name, &latest_version) {
                Ok(()) => println!("Update complete."),
                Err(e) => eprintln!("Update failed: {}", e),
            }
        } else {
            println!(" no update applied.");
        }
    } else {
        eprintln!(" ok.");
        std::process::exit(1);
    }
    Ok(())
}
