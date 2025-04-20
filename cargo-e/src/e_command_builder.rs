use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::time::SystemTime;
use which::which;

use crate::e_cargocommand_ext::CargoProcessResult;
use crate::e_cargocommand_ext::{CargoCommandExt, CargoDiagnostic, CargoProcessHandle};
use crate::e_eventdispatcher::{
    CallbackResponse, CallbackType, CargoDiagnosticLevel, EventDispatcher,
};
use crate::e_runner::GLOBAL_CHILDREN;
use crate::e_target::{CargoTarget, TargetKind, TargetOrigin};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum TerminalError {
    NotConnected,
    NoTerminal,
    NoError,
}

impl Default for TerminalError {
    fn default() -> Self {
        TerminalError::NoError
    }
}

/// A builder that constructs a Cargo command for a given target.
#[derive(Clone)]
pub struct CargoCommandBuilder {
    pub manifest_path: PathBuf,
    pub args: Vec<String>,
    pub subcommand: String,
    pub pid: Option<u32>,
    pub alternate_cmd: Option<String>,
    pub execution_dir: Option<PathBuf>,
    pub suppressed_flags: HashSet<String>,
    pub stdout_dispatcher: Option<Arc<EventDispatcher>>,
    pub stderr_dispatcher: Option<Arc<EventDispatcher>>,
    pub progress_dispatcher: Option<Arc<EventDispatcher>>,
    pub stage_dispatcher: Option<Arc<EventDispatcher>>,
    pub terminal_error_flag: Arc<Mutex<bool>>,
    pub sender: Option<Arc<Mutex<Sender<TerminalError>>>>,
    pub diagnostics: Arc<Mutex<Vec<CargoDiagnostic>>>,
    pub is_filter: bool,
}
impl Default for CargoCommandBuilder {
    fn default() -> Self {
        Self::new(&PathBuf::from("Cargo.toml"), "run".into(), false)
    }
}
impl CargoCommandBuilder {
    /// Creates a new, empty builder.
    pub fn new(manifest: &PathBuf, subcommand: &str, is_filter: bool) -> Self {
        let (sender, _receiver) = channel::<TerminalError>();
        let sender = Arc::new(Mutex::new(sender));
        let mut builder = CargoCommandBuilder {
            manifest_path: manifest.clone(),
            args: Vec::new(),
            subcommand: subcommand.to_string(),
            pid: None,
            alternate_cmd: None,
            execution_dir: None,
            suppressed_flags: HashSet::new(),
            stdout_dispatcher: None,
            stderr_dispatcher: None,
            progress_dispatcher: None,
            stage_dispatcher: None,
            terminal_error_flag: Arc::new(Mutex::new(false)),
            sender: Some(sender),
            diagnostics: Arc::new(Mutex::new(Vec::<CargoDiagnostic>::new())),
            is_filter: is_filter,
        };
        builder.set_default_dispatchers();

        builder
    }

    // Switch to passthrough mode when the terminal error is detected
    fn switch_to_passthrough_mode<F>(self: Arc<Self>, on_spawn: F) -> anyhow::Result<u32>
    where
        F: FnOnce(u32, CargoProcessHandle),
    {
        let mut command = self.build_command();

        // Now, spawn the cargo process in passthrough mode
        let cargo_process_handle = command.spawn_cargo_passthrough(Arc::clone(&self));
        let pid = cargo_process_handle.pid;
        // Notify observer
        on_spawn(pid, cargo_process_handle);

        Ok(pid)
    }

    // Set up the default dispatchers, which includes error detection
    fn set_default_dispatchers(&mut self) {
        if !self.is_filter {
            // If this is a filter, we don't need to set up dispatchers
            return;
        }
        let sender = self.sender.clone().unwrap();

        let mut stdout_dispatcher = EventDispatcher::new();
        stdout_dispatcher.add_callback(
            r"listening on",
            Box::new(|line, _captures, _state| {
                println!("(STDOUT) Dispatcher caught: {}", line);
                // Use a regex to capture a URL from the line.
                if let Ok(url_regex) = Regex::new(r"(http://[^\s]+)") {
                    if let Some(url_caps) = url_regex.captures(line) {
                        if let Some(url_match) = url_caps.get(1) {
                            let url = url_match.as_str();
                            // Call open::that on the captured URL.
                            if let Err(e) = open::that(url) {
                                eprintln!("Failed to open URL: {}. Error: {}", url, e);
                            } else {
                                println!("Opened URL: {}", url);
                            }
                        }
                    }
                } else {
                    eprintln!("Failed to create URL regex");
                }
                None
            }),
        );
        stdout_dispatcher.add_callback(
            r"BuildFinished",
            Box::new(|line, _captures, _state| {
                println!("******* {}", line);
                None
            }),
        );
        stdout_dispatcher.add_callback(
            r"server listening at:",
            Box::new(|line, _captures, state| {
                // If we're not already in multiline mode, this is the initial match.
                if !state.load(Ordering::Relaxed) {
                    println!("Matched 'server listening at:' in: {}", line);
                    state.store(true, Ordering::Relaxed);
                    Some(CallbackResponse {
                        callback_type: CallbackType::Note, // Choose as appropriate
                        message: Some(format!("Started multiline mode after: {}", line)),
                        file: None,
                        line: None,
                        column: None,
                        suggestion: None,
                        terminal_status: None,
                    })
                } else {
                    // We are in multiline mode; process subsequent lines.
                    println!("Multiline callback received: {}", line);
                    // Use a regex to capture a URL from the line.
                    let url_regex = match Regex::new(r"(http://[^\s]+)") {
                        Ok(regex) => regex,
                        Err(e) => {
                            eprintln!("Failed to create URL regex: {}", e);
                            return None;
                        }
                    };
                    if let Some(url_caps) = url_regex.captures(line) {
                        let url = url_caps.get(1).unwrap().as_str();
                        // Call open::that on the captured URL.
                        match open::that(url) {
                            Ok(_) => println!("Opened URL: {}", url),
                            Err(e) => eprintln!("Failed to open URL: {}. Error: {}", url, e),
                        }
                        // End multiline mode.
                        state.store(false, Ordering::Relaxed);
                        Some(CallbackResponse {
                            callback_type: CallbackType::Note, // Choose as appropriate
                            message: Some(format!("Captured and opened URL: {}", url)),
                            file: None,
                            line: None,
                            column: None,
                            suggestion: None,
                            terminal_status: None,
                        })
                    } else {
                        None
                    }
                }
            }),
        );

        let mut stderr_dispatcher = EventDispatcher::new();

        let suggestion_mode = Arc::new(AtomicBool::new(false));
        let suggestion_regex = Regex::new(r"^\s*(\d+)\s*\|\s*(.*)$").unwrap();
        let warning_location: Arc<Mutex<Option<CallbackResponse>>> = Arc::new(Mutex::new(None));
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::new(Mutex::new(None));
        let diagnostic_counts: Arc<Mutex<HashMap<CargoDiagnosticLevel, usize>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_d = Arc::clone(&pending_diag);
        let counts = Arc::clone(&diagnostic_counts);

        let diagnostics_arc = Arc::clone(&self.diagnostics);
        stderr_dispatcher.add_callback(
            r"^(?P<level>\w+)(\[(?P<error_code>E\d+)\])?:\s+(?P<msg>.+)$", // Regex for diagnostic line
            Box::new(move |_line, caps, _multiline_flag| {
                if let Some(caps) = caps {
                    let mut counts = counts.lock().unwrap();
                    // Create a PendingDiag and save the message
                    let mut pending_diag = pending_d.lock().unwrap();
                    let mut last_lineref = String::new();
                    if let Some(existing_diag) = pending_diag.take() {
                        let mut diags = diagnostics_arc.lock().unwrap();
                        last_lineref = existing_diag.lineref.clone();
                        //                 let diag_level = match existing_diag.level.as_str() {
                        //     "error" => CargoDiagnosticLevel::Error,
                        //     "warning" => CargoDiagnosticLevel::Warning,
                        //     "help" => CargoDiagnosticLevel::Help,
                        //     "note" => CargoDiagnosticLevel::Note,
                        //     _ => {
                        //         println!("Unknown diagnostic level: {}", existing_diag.level);
                        //         return None; // Ignore unknown levels
                        //     }
                        // };
                        // *counts.entry(diag_level).or_insert(0) += 1;
                        // let current_count = counts.get(&diag_level).unwrap_or(&0);
                        // existing_diag.diag_number = Some(*current_count);
                        println!("{:?}", existing_diag);
                        diags.push(existing_diag.clone());
                    }
                    log::trace!("Diagnostic line: {}", _line);
                    let level = caps["level"].to_string(); // e.g., "warning", "error"
                                                           // If the diagnostic level is "note" or "help", skip processing.
                                                           // if level == "note" || level == "help" {
                                                           //     println!("Skipping note/help diagnostic: {}", _line);
                                                           //     return None;
                                                           // }
                    let message = caps["msg"].to_string();
                    // If the message contains "generated" followed by one or more digits,
                    // then ignore this diagnostic by returning None.
                    //
                    // Using a regex to check for "generated\s+\d+"
                    let re_generated = regex::Regex::new(r"generated\s+\d+").unwrap();
                    if re_generated.is_match(&message) {
                        log::trace!("Skipping generated diagnostic: {}", _line);
                        return None;
                    }

                    let error_code = caps.name("error_code").map(|m| m.as_str().to_string());
                    let diag_level = match level.as_str() {
                        "error" => CargoDiagnosticLevel::Error,
                        "warning" => CargoDiagnosticLevel::Warning,
                        "help" => CargoDiagnosticLevel::Help,
                        "note" => CargoDiagnosticLevel::Note,
                        _ => {
                            println!("Unknown diagnostic level: {}", level);
                            return None; // Ignore unknown levels
                        }
                    };
                    // Increment the count for this level
                    *counts.entry(diag_level).or_insert(0) += 1;
                    // // Check if the previous diagnostic

                    let current_count = counts.get(&diag_level).unwrap_or(&0);
                    let diag = CargoDiagnostic {
                        error_code: error_code.clone(),
                        lineref: last_lineref.clone(),
                        level: level.clone(),
                        message,
                        suggestion: None,
                        help: None,
                        note: None,
                        uses_color: true,
                        diag_num_padding: Some(2),
                        diag_number: Some(*current_count),
                    };

                    // // Check if the previous diagnostic has a callback response
                    // if let Some(existing_diag) = pending_diag.take() {
                    //     // Save the current diagnostic's callback response
                    //     if let Some(response) = existing_diag.callback_response {
                    //         // Save it into the new diagnostic
                    //         diag.save_callback_response(response);
                    //     }
                    // }

                    // Save the new diagnostic
                    *pending_diag = Some(diag);

                    // Track the count of diagnostics for each level
                    return Some(CallbackResponse {
                        callback_type: CallbackType::LevelMessage, // Treat subsequent lines as warnings
                        message: None,
                        file: None,
                        line: None,
                        column: None,
                        suggestion: None, // This is the suggestion part
                        terminal_status: None,
                    });
                } else {
                    println!("No captures found in line: {}", _line);
                    None
                }
            }),
        );

        {
            let location_lock_clone = Arc::clone(&warning_location);
            let suggestion_m = Arc::clone(&suggestion_mode);

            // Suggestion callback that adds subsequent lines as suggestions
            stderr_dispatcher.add_callback(
                r"^(?P<msg>.*)$", // Capture all lines following the location
                Box::new(move |line, _captures, _multiline_flag| {
                    if suggestion_m.load(Ordering::Relaxed) {
                        // Only process lines that match the suggestion format
                        if let Some(caps) = suggestion_regex.captures(line.trim()) {
                            // Capture the line number and code from the suggestion line
                            // let line_num = caps[1].parse::<usize>().unwrap_or(0);
                            let code = caps[2].to_string();

                            // Lock the pending_diag to add the suggestion
                            if let Ok(mut lock) = location_lock_clone.lock() {
                                if let Some(mut loc) = lock.take() {
                                    // let file = loc.file.clone().unwrap_or_default();
                                    // let col = loc.column.unwrap_or(0);

                                    // Concatenate the suggestion line to the message
                                    let mut msg = loc.message.unwrap_or_default();
                                    msg.push_str(&format!("\n{}", code));

                                    // Print the concatenated suggestion for debugging
                                    // println!("daveSuggestion for {}:{}:{} - {}", file, line_num, col, msg);

                                    // Update the location with the new concatenated message
                                    loc.message = Some(msg.clone());
                                    // println!("Updating location lock with new suggestion: {}", msg);
                                    // Save the updated location back to shared state
                                    // if let Ok(mut lock) = location_lock_clone.lock() {
                                    // println!("Updating location lock with new suggestion: {}", msg);
                                    lock.replace(loc);
                                    // } else {
                                    //     eprintln!("Failed to acquire lock for location_lock_clone");
                                    // }
                                }
                                // return Some(CallbackResponse {
                                //     callback_type: CallbackType::Warning, // Treat subsequent lines as warnings
                                //     message: Some(msg.clone()),
                                //     file: Some(file),
                                //     line: Some(line_num),
                                //     column: Some(col),
                                //     suggestion: Some(msg),  // This is the suggestion part
                                //     terminal_status: None,
                                // });
                            }
                        }
                    } else {
                        // println!("Suggestion mode is not active. Ignoring line: {}", line);
                    }

                    None
                }),
            );
        }
        {
            let suggestion_m = Arc::clone(&suggestion_mode);
            let pending_diag_clone = Arc::clone(&pending_diag);
            let diagnostics_arc = Arc::clone(&self.diagnostics);
            // Callback for handling when an empty line or new diagnostic is received
            stderr_dispatcher.add_callback(
                r"^\s*$", // Regex to capture empty line
                Box::new(move |_line, _captures, _multiline_flag| {
                    // println!("Empty line detected: {}", line);
                    suggestion_m.store(false, Ordering::Relaxed);
                    // End of current diagnostic: take and process it.
                    if let Some(pending_diag) = pending_diag_clone.lock().unwrap().take() {
                        println!("{:?}", pending_diag);
                        // Use diagnostics_arc instead of self.diagnostices
                        let mut diags = diagnostics_arc.lock().unwrap();
                        diags.push(pending_diag.clone());
                    } else {
                        // println!("No pending diagnostic to process.");
                    }
                    // Handle empty line scenario to end the current diagnostic processing
                    // if let Some(pending_diag) = pending_diag_clone.lock().unwrap().take() {
                    //     println!("{:?}", pending_diag);
                    //     let mut diags = self.diagnostics.lock().unwrap();
                    //     diags.push(pending_diag.clone());
                    //                             // let diag = crate::e_eventdispatcher::convert_message_to_diagnostic(msg, &msg_str);
                    //                             // diags.push(diag.clone());
                    //                             // if let Some(ref sd) = stage_disp_clone {
                    //                             //     sd.dispatch(&format!("Stage: Diagnostic occurred at {:?}", now));
                    //                             // }
                    //     // Handle the saved PendingDiag and its CallbackResponse
                    //     // if let Some(callback_response) = pending_diag.callback_response {
                    //     //     println!("End of Diagnostic: {:?}", callback_response);
                    //     // }
                    // } else {
                    //     println!("No pending diagnostic to process.");
                    // }

                    None
                }),
            );
        }

        // {
        //     let pending_diag = Arc::clone(&pending_diag);
        //     let location_lock = Arc::clone(&warning_location);
        //     let suggestion_m = Arc::clone(&suggestion_mode);

        // let suggestion_regex = Regex::new(r"^\s*(\d+)\s*\|\s*(.*)$").unwrap();

        //     stderr_dispatcher.add_callback(
        //     r"^\s*(\d+)\s*\|\s*(.*)$",  // Match suggestion line format
        //     Box::new(move |line, _captures, _multiline_flag| {
        //         if suggestion_m.load(Ordering::Relaxed) {
        //             // Only process lines that match the suggestion format
        //             if let Some(caps) = suggestion_regex.captures(line.trim()) {
        //                 // Capture the line number and code from the suggestion line
        //                 let line_num = caps[1].parse::<usize>().unwrap_or(0);
        //                 let code = caps[2].to_string();

        //                 // Lock the pending_diag to add the suggestion
        //                 if let Some(mut loc) = location_lock.lock().unwrap().take() {
        //                     println!("Suggestion line: {}", line);
        //                     let file = loc.file.clone().unwrap_or_default();
        //                     let col = loc.column.unwrap_or(0);

        //                     // Concatenate the suggestion line to the message
        //                     let mut msg = loc.message.unwrap_or_default();
        //                     msg.push_str(&format!("\n{} | {}", line_num, code));  // Append the suggestion properly

        //                     // Print the concatenated suggestion for debugging
        //                     println!("Suggestion for {}:{}:{} - {}", file, line_num, col, msg);

        //                     // Update the location with the new concatenated message
        //                     loc.message = Some(msg.clone());

        //                     // Save the updated location back to shared state
        //                     location_lock.lock().unwrap().replace(loc);

        //                     // return Some(CallbackResponse {
        //                     //     callback_type: CallbackType::Warning, // Treat subsequent lines as warnings
        //                     //     message: Some(msg.clone()),
        //                     //     file: Some(file),
        //                     //     line: Some(line_num),
        //                     //     column: Some(col),
        //                     //     suggestion: Some(msg),  // This is the suggestion part
        //                     //     terminal_status: None,
        //                     // });
        //                 } else {
        //                     println!("No location information available for suggestion line: {}", line);
        //                 }
        //             } else {
        //                 println!("Suggestion line does not match expected format: {}", line);
        //             }
        //         } else {
        //             println!("Suggestion mode is not active. Ignoring line: {}", line);
        //         }

        //         None
        //     }),
        // );

        // }

        {
            let location_lock = Arc::clone(&warning_location);
            let pending_diag = Arc::clone(&pending_diag);
            let suggestion_mode = Arc::clone(&suggestion_mode);
            stderr_dispatcher.add_callback(
                r"^(?P<msg>.*)$", // Capture all lines following the location
                Box::new(move |line, _captures, _multiline_flag| {
                    // Lock the location to fetch the original diagnostic info
                    if let Ok(location_guard) = location_lock.lock() {
                        if let Some(loc) = location_guard.as_ref() {
                            let file = loc.file.clone().unwrap_or_default();
                            let line_num = loc.line.unwrap_or(0);
                            let col = loc.column.unwrap_or(0);
                            // println!("SUGGESTION: Suggestion for {}:{}:{} {}", file, line_num, col, line);

                            // Only treat lines starting with | or numbers as suggestion lines
                            if line.trim().starts_with('|')
                                || line.trim().starts_with(char::is_numeric)
                            {
                                // Get the existing suggestion and append the new line
                                let suggestion = line.trim();

                                // Print the suggestion for debugging
                                // println!("Suggestion for {}:{}:{} - {}", file, line_num, col, suggestion);

                                // Lock the pending_diag and update its callback_response field
                                let mut pending_diag = match pending_diag.lock() {
                                    Ok(lock) => lock,
                                    Err(e) => {
                                        eprintln!("Failed to acquire lock: {}", e);
                                        return None; // Handle the error appropriately
                                    }
                                };
                                if let Some(diag) = pending_diag.take() {
                                    // If a PendingDiag already exists, update the existing callback response with the new suggestion
                                    let mut diag = diag;

                                    // Append the new suggestion to the existing one
                                    if let Some(ref mut existing) = diag.suggestion {
                                        diag.suggestion =
                                            Some(format!("{}\n{}", existing, suggestion));
                                    } else {
                                        diag.suggestion = Some(suggestion.to_string());
                                    }

                                    // Update the shared state with the new PendingDiag
                                    *pending_diag = Some(diag.clone());
                                    return Some(CallbackResponse {
                                        callback_type: CallbackType::Suggestion, // Treat subsequent lines as warnings
                                        message: Some(
                                            diag.clone().suggestion.clone().unwrap().clone(),
                                        ),
                                        file: Some(file),
                                        line: Some(line_num),
                                        column: Some(col),
                                        suggestion: diag.clone().suggestion.clone(), // This is the suggestion part
                                        terminal_status: None,
                                    });
                                } else {
                                    // println!("No pending diagnostic to process for suggestion line: {}", line);
                                }
                            } else {
                                // If the line doesn't match the suggestion format, just return it as is
                                if line.trim().is_empty() {
                                    // Ignore empty lines
                                    suggestion_mode.store(false, Ordering::Relaxed);
                                    return None;
                                }
                            }
                        } else {
                            // println!("No location information available for suggestion line: {}", line);
                        }
                    }
                    None
                }),
            );
        }

        // 2) Location callback stores its response into that shared state
        {
            let pending_diag = Arc::clone(&pending_diag);
            let warning_location = Arc::clone(&warning_location);
            let location_lock = Arc::clone(&warning_location);
            let suggestion_mode = Arc::clone(&suggestion_mode);
            let manifest_path = self.manifest_path.clone();
            stderr_dispatcher.add_callback(
                // r"^\s*-->\s+(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+)$",
                r"^\s*-->\s+(?P<file>.+?)(?::(?P<line>\d+))?(?::(?P<col>\d+))?\s*$",
                Box::new(move |_line, caps, _multiline_flag| {
                    log::trace!("Location line: {}", _line);
                    // if multiline_flag.load(Ordering::Relaxed) {
                    if let Some(caps) = caps {
                        let file = caps["file"].to_string();
                        let resolved_path = resolve_file_path(&manifest_path, &file);
                        let file = resolved_path.to_str().unwrap_or_default().to_string();
                        let line = caps["line"].parse::<usize>().unwrap_or(0);
                        let column = caps["col"].parse::<usize>().unwrap_or(0);
                        let resp = CallbackResponse {
                            callback_type: CallbackType::Location,
                            message: format!("{}:{}:{}", file, line, column).into(),
                            file: Some(file.clone()),
                            line: Some(line),
                            column: Some(column),
                            suggestion: None,
                            terminal_status: None,
                        };
                        // Lock the pending_diag and update its callback_response field
                        let mut pending_diag = pending_diag.lock().unwrap();
                        if let Some(diag) = pending_diag.take() {
                            // If a PendingDiag already exists, save the new callback response in the existing PendingDiag
                            let mut diag = diag;
                            diag.lineref = format!("{}:{}:{}", file, line, column); // Update the lineref
                                                                                    // diag.save_callback_response(resp.clone()); // Save the callback response
                                                                                    // Update the shared state with the new PendingDiag
                            *pending_diag = Some(diag);
                        }
                        // Save it for the generic callback to see
                        *warning_location.lock().unwrap() = Some(resp.clone());
                        *location_lock.lock().unwrap() = Some(resp.clone());
                        // Set suggestion mode to true as we've encountered a location line
                        suggestion_mode.store(true, Ordering::Relaxed);
                        return Some(resp.clone());
                    } else {
                        println!("No captures found in line: {}", _line);
                    }
                    // }
                    None
                }),
            );
        }

        // // 3) Note callback — attach note to pending_diag
        {
            let pending_diag = Arc::clone(&pending_diag);
            stderr_dispatcher.add_callback(
                r"^\s*=\s*note:\s*(?P<msg>.+)$",
                Box::new(move |_line, caps, _state| {
                    if let Some(caps) = caps {
                        let mut pending_diag = pending_diag.lock().unwrap();
                        if let Some(ref mut resp) = *pending_diag {
                            // Prepare the new note with the blue prefix
                            let new_note =
                                format!("\x1b[34m{}\x1b[0m: {}", "note", caps["msg"].to_string());

                            // Append or set the note
                            if let Some(existing_note) = &resp.note {
                                // If there's already a note, append with newline and the new note
                                resp.note = Some(format!("{}\n{}", existing_note, new_note));
                            } else {
                                // If no existing note, just set the new note
                                resp.note = Some(new_note);
                            }
                        }
                    }
                    None
                }),
            );
        }

        // 4) Help callback — attach help to pending_diag
        {
            let pending_diag = Arc::clone(&pending_diag);
            stderr_dispatcher.add_callback(
                r"^\s*(?:\=|\|)\s*help:\s*(?P<msg>.+)$", // Regex to match both '=' and '|' before help:
                Box::new(move |_line, caps, _state| {
                    if let Some(caps) = caps {
                        let mut pending_diag = pending_diag.lock().unwrap();
                        if let Some(ref mut resp) = *pending_diag {
                            // Create the new help message with the orange "h:" prefix
                            let new_help =
                                format!("\x1b[38;5;214mhelp: {}\x1b[0m", caps["msg"].to_string());

                            // Append or set the help message
                            if let Some(existing_help) = &resp.help {
                                // If there's already a help message, append with newline
                                resp.help = Some(format!("{}\n{}", existing_help, new_help));
                            } else {
                                // If no existing help message, just set the new one
                                resp.help = Some(new_help);
                            }
                        }
                    }
                    None
                }),
            );
        }

        stderr_dispatcher.add_callback(
    r"(?:\x1b\[[0-9;]*[A-Za-z])*\s*Serving(?:\x1b\[[0-9;]*[A-Za-z])*\s+at\s+(http://[^\s]+)",
    Box::new(|line, captures, _state| {
        if let Some(caps) = captures {
            let url = caps.get(1).unwrap().as_str();
            println!("(STDERR) Captured URL: {}", url);
            match open::that(url) {
                Ok(_) => println!("(STDERR) Opened URL: {}", url),
                Err(e) => eprintln!("(STDERR) Failed to open URL: {}. Error: {:?}", url, e),
            }
            Some(CallbackResponse {
                callback_type: CallbackType::OpenedUrl, // Choose as appropriate
                message: Some(format!("Captured and opened URL: {}", url)),
                file: None,
                line: None,
                column: None,
                suggestion: None,
                terminal_status: None,
            })
        } else {
            println!("(STDERR) No URL captured in line: {}", line);
            None
        }
    }),
);

        let finished_flag = Arc::new(AtomicBool::new(false));

        // 0) Finished‐profile summary callback
        {
            let finished_flag = Arc::clone(&finished_flag);
            stderr_dispatcher.add_callback(
        r"^Finished\s+`(?P<profile>[^`]+)`\s+profile\s+\[(?P<opts>[^\]]+)\]\s+target\(s\)\s+in\s+(?P<dur>[0-9.]+s)$",
        Box::new(move |_line, caps, _multiline_flag| {
            if let Some(caps) = caps {
                finished_flag.store(true, Ordering::Relaxed);
                let profile = &caps["profile"];
                let opts    = &caps["opts"];
                let dur     = &caps["dur"];
                Some(CallbackResponse {
                    callback_type: CallbackType::Note,
                    message: Some(format!("Finished `{}` [{}] in {}", profile, opts, dur)),
                    file: None, line: None, column: None, suggestion: None, terminal_status: None,
                })
            } else {
                None
            }
        }),
    );
        }

        let summary_flag = Arc::new(AtomicBool::new(false));
        {
            let summary_flag = Arc::clone(&summary_flag);
            stderr_dispatcher.add_callback(
    r"^(?P<level>warning|error):\s+`(?P<name>[^`]+)`\s+\((?P<otype>lib|bin)\)\s+generated\s+(?P<count>\d+)\s+(?P<kind>warnings|errors).*run\s+`(?P<cmd>[^`]+)`\s+to apply\s+(?P<fixes>\d+)\s+suggestions",
    Box::new(move |_line, caps, multiline_flag| {
        let summary_flag = Arc::clone(&summary_flag);
        if let Some(caps) = caps {
            summary_flag.store(true, Ordering::Relaxed);
            // Always start fresh
            multiline_flag.store(false, Ordering::Relaxed);

            let level    = &caps["level"];
            let name     = &caps["name"];
            let otype    = &caps["otype"];
            let count: usize = caps["count"].parse().unwrap_or(0);
            let kind     = &caps["kind"];   // "warnings" or "errors"
            let cmd      = caps["cmd"].to_string();
            let fixes: usize = caps["fixes"].parse().unwrap_or(0);

            println!("SUMMARIZATION CALLBACK {}",
                    &format!("{}: `{}` ({}) generated {} {}; run `{}` to apply {} fixes",
                    level, name, otype, count, kind, cmd, fixes));
            Some(CallbackResponse {
                callback_type: CallbackType::Note,  // treat as informational
                message: Some(format!(
                    "{}: `{}` ({}) generated {} {}; run `{}` to apply {} fixes",
                    level, name, otype, count, kind, cmd, fixes
                )),
                file: None,
                line: None,
                column: None,
                suggestion: Some(cmd),
                terminal_status: None,
            })
        } else {
            None
        }
    }),
    );
        }

        // {
        //     let summary_flag = Arc::clone(&summary_flag);
        //     let finished_flag = Arc::clone(&finished_flag);
        //     let warning_location = Arc::clone(&warning_location);
        //     // Warning callback for stdout.
        //     stderr_dispatcher.add_callback(
        //         r"^warning:\s+(?P<msg>.+)$",
        //         Box::new(
        //             move |line: &str, captures: Option<regex::Captures>, multiline_flag: Arc<AtomicBool>| {
        //                             // If summary or finished just matched, skip
        //             if summary_flag.swap(false, Ordering::Relaxed)
        //                 || finished_flag.swap(false, Ordering::Relaxed)
        //             {
        //                 return None;
        //             }

        //         // 2) If this line *matches* the warning regex, handle as a new warning
        //         if let Some(caps) = captures {
        //             let msg = caps.name("msg").unwrap().as_str().to_string();
        //                    // 1) If a location was saved, print file:line:col – msg
        //             // println!("*WARNING detected: {:?}", msg);
        //                 multiline_flag.store(true, Ordering::Relaxed);
        //         if let Some(loc) = warning_location.lock().unwrap().take() {
        //                 let file = loc.file.unwrap_or_default();
        //                 let line_num = loc.line.unwrap_or(0);
        //                 let col  = loc.column.unwrap_or(0);
        //                 println!("{}:{}:{} - {}", file, line_num, col, msg);
        //                 return Some(CallbackResponse {
        //                     callback_type: CallbackType::Warning,
        //                     message: Some(msg.to_string()),
        //                     file: None, line: None, column: None, suggestion: None, terminal_status: None,
        //                 });
        //         }
        //             return Some(CallbackResponse {
        //                 callback_type: CallbackType::Warning,
        //                 message: Some(msg),
        //                 file: None,
        //                 line: None,
        //                 column: None,
        //                 suggestion: None,
        //                 terminal_status: None,
        //             });
        //         }

        //                 // 3) Otherwise, if we’re in multiline mode, treat as continuation
        //         if multiline_flag.load(Ordering::Relaxed) {
        //             let text = line.trim();
        //             if text.is_empty() {
        //                 multiline_flag.store(false, Ordering::Relaxed);
        //                 return None;
        //             }
        //             // println!("   - {:?}", text);
        //             return Some(CallbackResponse {
        //                 callback_type: CallbackType::Warning,
        //                 message: Some(text.to_string()),
        //                 file: None,
        //                 line: None,
        //                 column: None,
        //                 suggestion: None,
        //                 terminal_status: None,
        //             });
        //         }
        //                     None
        //             },
        //         ),
        //     );
        // }

        stderr_dispatcher.add_callback(
            r"IO\(Custom \{ kind: NotConnected",
            Box::new(move |line, _captures, _state| {
                println!("(STDERR) Terminal error detected: {:?}", &line);
                let result = if line.contains("NotConnected") {
                    TerminalError::NoTerminal
                } else {
                    TerminalError::NoError
                };
                let sender = sender.lock().unwrap();
                sender.send(result).ok();
                Some(CallbackResponse {
                    callback_type: CallbackType::Warning, // Choose as appropriate
                    message: Some(format!("Terminal Error: {}", line)),
                    file: None,
                    line: None,
                    column: None,
                    suggestion: None,
                    terminal_status: None,
                })
            }),
        );
        stderr_dispatcher.add_callback(
            r".*",
            Box::new(|line, _captures, _state| {
                log::trace!("stdraw[{:?}]", line);
                None // We're just printing, so no callback response is needed.
            }),
        );
        self.stderr_dispatcher = Some(Arc::new(stderr_dispatcher));

        // let mut progress_dispatcher = EventDispatcher::new();
        // progress_dispatcher.add_callback(r"Progress", Box::new(|line, _captures,_state| {
        //     println!("(Progress) {}", line);
        //     None
        // }));
        // self.progress_dispatcher = Some(Arc::new(progress_dispatcher));

        // let mut stage_dispatcher = EventDispatcher::new();
        // stage_dispatcher.add_callback(r"Stage:", Box::new(|line, _captures, _state| {
        //     println!("(Stage) {}", line);
        //     None
        // }));
        // self.stage_dispatcher = Some(Arc::new(stage_dispatcher));
    }

    pub fn run<F>(self: Arc<Self>, on_spawn: F) -> anyhow::Result<u32>
    where
        F: FnOnce(u32, CargoProcessHandle),
    {
        if !self.is_filter {
            return self.switch_to_passthrough_mode(on_spawn);
        }
        let mut command = self.build_command();

        let mut cargo_process_handle = command.spawn_cargo_capture(
            self.clone(),
            self.stdout_dispatcher.clone(),
            self.stderr_dispatcher.clone(),
            self.progress_dispatcher.clone(),
            self.stage_dispatcher.clone(),
            None,
        );
        cargo_process_handle.diagnostics = Arc::clone(&self.diagnostics);
        let pid = cargo_process_handle.pid;

        // Notify observer
        on_spawn(pid, cargo_process_handle);

        Ok(pid)
    }

    // pub fn run(self: Arc<Self>) -> anyhow::Result<u32> {
    //     // Build the command using the builder's configuration
    //     let mut command = self.build_command();

    //     // Spawn the cargo process handle
    //     let cargo_process_handle = command.spawn_cargo_capture(
    //         self.stdout_dispatcher.clone(),
    //         self.stderr_dispatcher.clone(),
    //         self.progress_dispatcher.clone(),
    //         self.stage_dispatcher.clone(),
    //         None,
    //     );
    // let pid = cargo_process_handle.pid;
    // let mut global = GLOBAL_CHILDREN.lock().unwrap();
    // global.insert(pid, Arc::new(Mutex::new(cargo_process_handle)));
    //     Ok(pid)
    // }

    pub fn wait(self: Arc<Self>, pid: Option<u32>) -> anyhow::Result<CargoProcessResult> {
        let mut global = GLOBAL_CHILDREN.lock().unwrap();
        if let Some(pid) = pid {
            // Lock the global list of processes and attempt to find the cargo process handle directly by pid
            if let Some(cargo_process_handle) = global.get_mut(&pid) {
                let mut cargo_process_handle = cargo_process_handle.lock().unwrap();

                // Wait for the process to finish and retrieve the result
                // println!("Waiting for process with PID: {}", pid);
                // let result = cargo_process_handle.wait();
                // println!("Process with PID {} finished", pid);
                loop {
                    println!("Waiting for process with PID: {}", pid);

                    // Attempt to wait for the process, but don't block indefinitely
                    let status = cargo_process_handle.child.try_wait()?;

                    // If the status is `Some(status)`, the process has finished
                    if let Some(status) = status {
                        // Check the terminal error flag and update the result if there is an error
                        if *cargo_process_handle.terminal_error_flag.lock().unwrap()
                            != TerminalError::NoError
                        {
                            let terminal_error =
                                *cargo_process_handle.terminal_error_flag.lock().unwrap();
                            cargo_process_handle.result.terminal_error = Some(terminal_error);
                        }

                        let final_diagnostics = {
                            let diag_lock = self.diagnostics.lock().unwrap();
                            diag_lock.clone()
                        };
                        cargo_process_handle.result.diagnostics = final_diagnostics.clone();
                        cargo_process_handle.result.exit_status = Some(status);
                        cargo_process_handle.result.end_time = Some(SystemTime::now());
                        cargo_process_handle.result.elapsed_time = Some(
                            cargo_process_handle
                                .result
                                .end_time
                                .unwrap()
                                .duration_since(cargo_process_handle.result.start_time.unwrap())
                                .unwrap(),
                        );
                        println!(
                            "Process with PID {} finished {:?} {}",
                            pid,
                            status,
                            final_diagnostics.len()
                        );
                        return Ok(cargo_process_handle.result.clone());
                        // return Ok(CargoProcessResult { exit_status: status, ..Default::default() });
                    }

                    // Sleep briefly to yield control back to the system and avoid blocking
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }

                // Return the result
                // match result {
                //     Ok(res) => Ok(res),
                //     Err(e) => Err(anyhow::anyhow!("Failed to wait for cargo process: {}", e).into()),
                // }
            } else {
                Err(anyhow::anyhow!(
                    "Process handle with PID {} not found in GLOBAL_CHILDREN",
                    pid
                )
                .into())
            }
        } else {
            Err(anyhow::anyhow!("No PID provided for waiting on cargo process").into())
        }
    }

    // pub fn run_wait(self: Arc<Self>) -> anyhow::Result<CargoProcessResult> {
    //     // Run the cargo command and get the process handle (non-blocking)
    //     let pid = self.clone().run()?; // adds to global list of processes
    //     let result = self.wait(Some(pid)); // Wait for the process to finish
    //     // Remove the completed process from GLOBAL_CHILDREN
    //     let mut global = GLOBAL_CHILDREN.lock().unwrap();
    //     global.remove(&pid);

    //     result
    // }

    /// Runs the cargo command using the builder's configuration.
    // pub fn run(&self) -> anyhow::Result<CargoProcessResult> {
    //     // Build the command using the builder's configuration
    //     let mut command = self.build_command();

    //     // Now use the `spawn_cargo_capture` extension to run the command
    //     let mut cargo_process_handle = command.spawn_cargo_capture(
    //         self.stdout_dispatcher.clone(),
    //         self.stderr_dispatcher.clone(),
    //         self.progress_dispatcher.clone(),
    //         self.stage_dispatcher.clone(),
    //         None,
    //     );

    //     // Wait for the process to finish and retrieve the results
    //     cargo_process_handle.wait().context("Failed to execute cargo process")
    // }

    /// Configure the command based on the target kind.
    pub fn with_target(mut self, target: &CargoTarget) -> Self {
        if let Some(origin) = target.origin.clone() {
            println!("Target origin: {:?}", origin);
        } else {
            println!("Target origin is not set");
        }
        match target.kind {
            TargetKind::Unknown | TargetKind::Plugin => {
                return self;
            }
            TargetKind::Bench => {
                // // To run benchmarks, use the "bench" command.
                //  let exe_path = match which("bench") {
                //     Ok(path) => path,
                //     Err(err) => {
                //         eprintln!("Error: 'trunk' not found in PATH: {}", err);
                //         return self;
                //     }
                // };
                // self.alternate_cmd = Some("bench".to_string())
                self.args.push("bench".into());
                self.args.push(target.name.clone());
            }
            TargetKind::Test => {
                self.args.push("test".into());
                // Pass the target's name as a filter to run specific tests.
                self.args.push(target.name.clone());
            }
            TargetKind::UnknownExample
            | TargetKind::UnknownExtendedExample
            | TargetKind::Example
            | TargetKind::ExtendedExample => {
                self.args.push(self.subcommand.clone());
                //self.args.push("--message-format=json".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::UnknownBinary
            | TargetKind::UnknownExtendedBinary
            | TargetKind::Binary
            | TargetKind::ExtendedBinary => {
                self.args.push(self.subcommand.clone());
                self.args.push("--bin".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::Manifest => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push(self.subcommand.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ManifestTauriExample => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push(self.subcommand.clone());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ScriptScriptisto => {
                let exe_path = match which("scriptisto") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'scriptisto' not found in PATH: {}", err);
                        return self;
                    }
                };
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                let candidate_opt = match &target.origin {
                    Some(TargetOrigin::SingleFile(path))
                    | Some(TargetOrigin::DefaultBinary(path)) => Some(path),
                    _ => None,
                };
                if let Some(candidate) = candidate_opt {
                    self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                    self.args.push(candidate.to_string_lossy().to_string());
                } else {
                    println!("No scriptisto origin found for: {:?}", target);
                }
            }
            TargetKind::ScriptRustScript => {
                let exe_path = match which("rust-script") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'rust-script' not found in PATH: {}", err);
                        return self;
                    }
                };
                let candidate_opt = match &target.origin {
                    Some(TargetOrigin::SingleFile(path))
                    | Some(TargetOrigin::DefaultBinary(path)) => Some(path),
                    _ => None,
                };
                if let Some(candidate) = candidate_opt {
                    self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                    if self.is_filter {
                        self.args.push("-c".into()); // ask for cargo output
                    }
                    self.args.push(candidate.to_string_lossy().to_string());
                } else {
                    println!("No rust-script origin found for: {:?}", target);
                }
            }
            TargetKind::ManifestTauri => {
                self.suppressed_flags.insert("quiet".to_string());
                // Helper closure to check for tauri.conf.json in a directory.
                let has_tauri_conf = |dir: &Path| -> bool { dir.join("tauri.conf.json").exists() };

                // Try candidate's parent (if origin is SingleFile or DefaultBinary).
                let candidate_dir_opt = match &target.origin {
                    Some(TargetOrigin::SingleFile(path))
                    | Some(TargetOrigin::DefaultBinary(path)) => path.parent(),
                    _ => None,
                };

                if let Some(candidate_dir) = candidate_dir_opt {
                    if has_tauri_conf(candidate_dir) {
                        println!("Using candidate directory: {}", candidate_dir.display());
                        self.execution_dir = Some(candidate_dir.to_path_buf());
                    } else if let Some(manifest_parent) = target.manifest_path.parent() {
                        if has_tauri_conf(manifest_parent) {
                            println!("Using manifest parent: {}", manifest_parent.display());
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        } else if let Some(grandparent) = manifest_parent.parent() {
                            if has_tauri_conf(grandparent) {
                                println!("Using manifest grandparent: {}", grandparent.display());
                                self.execution_dir = Some(grandparent.to_path_buf());
                            } else {
                                println!("No tauri.conf.json found in candidate, manifest parent, or grandparent; defaulting to manifest parent: {}", manifest_parent.display());
                                self.execution_dir = Some(manifest_parent.to_path_buf());
                            }
                        } else {
                            println!("No grandparent for manifest; defaulting to candidate directory: {}", candidate_dir.display());
                            self.execution_dir = Some(candidate_dir.to_path_buf());
                        }
                    } else {
                        println!(
                            "No manifest parent found for: {}",
                            target.manifest_path.display()
                        );
                    }
                } else if let Some(manifest_parent) = target.manifest_path.parent() {
                    if has_tauri_conf(manifest_parent) {
                        println!("Using manifest parent: {}", manifest_parent.display());
                        self.execution_dir = Some(manifest_parent.to_path_buf());
                    } else if let Some(grandparent) = manifest_parent.parent() {
                        if has_tauri_conf(grandparent) {
                            println!("Using manifest grandparent: {}", grandparent.display());
                            self.execution_dir = Some(grandparent.to_path_buf());
                        } else {
                            println!(
                                "No tauri.conf.json found; defaulting to manifest parent: {}",
                                manifest_parent.display()
                            );
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        }
                    }
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.args.push("tauri".into());
                self.args.push("dev".into());
            }
            TargetKind::ManifestLeptos => {
                let readme_path = target
                    .manifest_path
                    .parent()
                    .map(|p| p.join("README.md"))
                    .filter(|p| p.exists())
                    .or_else(|| {
                        target
                            .manifest_path
                            .parent()
                            .map(|p| p.join("readme.md"))
                            .filter(|p| p.exists())
                    });

                if let Some(readme) = readme_path {
                    if let Ok(mut file) = std::fs::File::open(&readme) {
                        let mut contents = String::new();
                        if file.read_to_string(&mut contents).is_ok()
                            && contents.contains("cargo leptos watch")
                        {
                            // Use cargo leptos watch
                            println!("Detected 'cargo leptos watch' in {}", readme.display());
                            self.execution_dir =
                                target.manifest_path.parent().map(|p| p.to_path_buf());
                            self.execution_dir =
                                Some(target.manifest_path.parent().unwrap().to_path_buf());
                            self.alternate_cmd = Some("cargo".to_string());
                            self.args.push("leptos".into());
                            self.args.push("watch".into());
                            self = self.with_required_features(&target.manifest_path, target);
                            return self;
                        }
                    }
                }

                // fallback to trunk
                let exe_path = match which("trunk") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'trunk' not found in PATH: {}", err);
                        return self;
                    }
                };

                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }

                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self.args.push("--open".into());
                self.args.push("--color".into());
                self.args.push("always".into());
                self = self.with_required_features(&target.manifest_path, target);
            }
            TargetKind::ManifestDioxus => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self = self.with_required_features(&target.manifest_path, target);
            }
            TargetKind::ManifestDioxusExample => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self = self.with_required_features(&target.manifest_path, target);
            }
        }
        self
    }

    /// Configure the command using CLI options.
    pub fn with_cli(mut self, cli: &crate::Cli) -> Self {
        if cli.quiet && !self.suppressed_flags.contains("quiet") {
            // Insert --quiet right after "run" if present.
            if let Some(pos) = self.args.iter().position(|arg| arg == &self.subcommand) {
                self.args.insert(pos + 1, "--quiet".into());
            } else {
                self.args.push("--quiet".into());
            }
        }
        if cli.release {
            // Insert --release right after the initial "run" command if applicable.
            // For example, if the command already contains "run", insert "--release" after it.
            if let Some(pos) = self.args.iter().position(|arg| arg == &self.subcommand) {
                self.args.insert(pos + 1, "--release".into());
            } else {
                // If not running a "run" command (like in the Tauri case), simply push it.
                self.args.push("--release".into());
            }
        }
        // Append extra arguments (if any) after a "--" separator.
        if !cli.extra.is_empty() {
            self.args.push("--".into());
            self.args.extend(cli.extra.iter().cloned());
        }
        self
    }
    /// Append required features based on the manifest, target kind, and name.
    /// This method queries your manifest helper function and, if features are found,
    /// appends "--features" and the feature list.
    pub fn with_required_features(mut self, manifest: &PathBuf, target: &CargoTarget) -> Self {
        if let Some(features) = crate::e_manifest::get_required_features_from_manifest(
            manifest,
            &target.kind,
            &target.name,
        ) {
            self.args.push("--features".to_string());
            self.args.push(features);
        }
        self
    }

    /// Appends extra arguments to the command.
    pub fn with_extra_args(mut self, extra: &[String]) -> Self {
        if !extra.is_empty() {
            // Use "--" to separate Cargo arguments from target-specific arguments.
            self.args.push("--".into());
            self.args.extend(extra.iter().cloned());
        }
        self
    }

    /// Builds the final vector of command-line arguments.
    pub fn build(self) -> Vec<String> {
        self.args
    }

    /// Optionally, builds a std::process::Command.
    pub fn build_command(&self) -> Command {
        let mut is_cargo = false;
        let mut new_args = self.args.clone();
        let supported_subcommands = [
            "run", "build", "test", "bench", "clean", "doc", "publish", "update",
        ];

        let mut cmd = if let Some(alternate) = &self.alternate_cmd {
            Command::new(alternate)
        } else {
            is_cargo = true;
            Command::new("cargo")
        };
        if is_cargo && self.is_filter {
            if let Some(pos) = new_args
                .iter()
                .position(|arg| supported_subcommands.contains(&arg.as_str()))
            {
                // If the command is "cargo run", insert the JSON output format and color options.
                new_args.insert(pos + 1, "--message-format=json".into());
                new_args.insert(pos + 2, "--color".into());
                new_args.insert(pos + 3, "always".into());
            }
        }
        cmd.args(new_args);
        if let Some(dir) = &self.execution_dir {
            cmd.current_dir(dir);
        }
        cmd
    }
}
/// Resolves a file path by:
///   1. If the path is relative, try to resolve it relative to the current working directory.
///   2. If that file does not exist, try to resolve it relative to the parent directory of the manifest path.
///   3. Otherwise, return the original relative path.
fn resolve_file_path(manifest_path: &PathBuf, file_str: &str) -> PathBuf {
    let file_path = Path::new(file_str);
    if file_path.is_relative() {
        // 1. Try resolving relative to the current working directory.
        if let Ok(cwd) = env::current_dir() {
            let cwd_path = cwd.join(file_path);
            if cwd_path.exists() {
                return cwd_path;
            }
        }
        // 2. Try resolving relative to the parent of the manifest path.
        if let Some(manifest_parent) = manifest_path.parent() {
            let parent_path = manifest_parent.join(file_path);
            if parent_path.exists() {
                return parent_path;
            }
        }
        // 3. Neither existed; return the relative path as-is.
        return file_path.to_path_buf();
    }
    file_path.to_path_buf()
}

// --- Example usage ---
#[cfg(test)]
mod tests {
    use crate::e_target::TargetOrigin;

    use super::*;

    #[test]
    fn test_command_builder_example() {
        let target = CargoTarget {
            name: "my_example".to_string(),
            display_name: "My Example".to_string(),
            manifest_path: "Cargo.toml".into(),
            kind: TargetKind::Example,
            extended: true,
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(PathBuf::from(
                "examples/my_example.rs",
            ))),
        };

        let extra_args = vec!["--flag".to_string(), "value".to_string()];

        let manifest_path = PathBuf::from("Cargo.toml");
        let args = CargoCommandBuilder::new(&manifest_path, &"run".to_string(), false)
            .with_target(&target)
            .with_extra_args(&extra_args)
            .build();

        // For an example target, we expect something like:
        // cargo run --example my_example --manifest-path Cargo.toml -- --flag value
        assert!(args.contains(&"--example".to_string()));
        assert!(args.contains(&"my_example".to_string()));
        assert!(args.contains(&"--manifest-path".to_string()));
        assert!(args.contains(&"Cargo.toml".to_string()));
        assert!(args.contains(&"--".to_string()));
        assert!(args.contains(&"--flag".to_string()));
        assert!(args.contains(&"value".to_string()));
    }
}
