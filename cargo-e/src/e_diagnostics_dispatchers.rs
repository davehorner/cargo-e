//! Diagnostic dispatcher setup for cargo-e
// Provides functions to create configured stdout and stderr EventDispatchers for diagnostics

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::SystemTime;

use crate::e_cargocommand_ext::CargoDiagnostic;
use crate::e_command_builder::{resolve_file_path, TerminalError};
use crate::e_eventdispatcher::{
    CallbackResponse, CallbackType, CargoDiagnosticLevel, EventDispatcher,
};
use open;
use regex::Regex;
// --- Dispatcher creation helpers for diagnostics (no struct, just functions) ---

/// Create a configured EventDispatcher for stdout diagnostics.
pub fn create_stdout_dispatcher() -> EventDispatcher {
    let mut dispatcher = EventDispatcher::new();

    dispatcher.add_callback(
        r"listening on",
        Box::new(|line, _captures, _state, stats| {
            println!("(STDOUT) Dispatcher caught: {}", line);
            if let Ok(url_regex) = Regex::new(r"(http://[^\s]+)") {
                if let Some(url_caps) = url_regex.captures(line) {
                    if let Some(url_match) = url_caps.get(1) {
                        let url = url_match.as_str();
                        if let Err(e) = open::that_detached(url) {
                            eprintln!("Failed to open URL: {}. Error: {}", url, e);
                        } else {
                            println!("Opened URL: {}", url);
                        }
                    }
                }
            } else {
                eprintln!("Failed to create URL regex");
            }
            let mut stats = stats.lock().unwrap();
            if stats.build_finished_time.is_none() {
                let now = SystemTime::now();
                stats.build_finished_time = Some(now);
            }
            None
        }),
    );
    dispatcher.add_callback(
        r"BuildFinished",
        Box::new(|line, _captures, _state, stats| {
            println!("******* {}", line);
            let mut stats = stats.lock().unwrap();
            if stats.build_finished_time.is_none() {
                let now = SystemTime::now();
                stats.build_finished_time = Some(now);
            }
            None
        }),
    );
    dispatcher.add_callback(
        r"server listening at:",
        Box::new(|line, _captures, state, stats| {
            if !state.load(Ordering::Relaxed) {
                println!("Matched 'server listening at:' in: {}", line);
                state.store(true, Ordering::Relaxed);
                Some(CallbackResponse {
                    callback_type: CallbackType::Note,
                    message: Some(format!("Started multiline mode after: {}", line)),
                    file: None,
                    line: None,
                    column: None,
                    suggestion: None,
                    terminal_status: None,
                })
            } else {
                println!("Multiline callback received: {}", line);
                let url_regex = match Regex::new(r"(http://[^\s]+)") {
                    Ok(regex) => regex,
                    Err(e) => {
                        eprintln!("Failed to create URL regex: {}", e);
                        return None;
                    }
                };
                if let Some(url_caps) = url_regex.captures(line) {
                    let url = url_caps.get(1).unwrap().as_str();
                    match open::that_detached(url) {
                        Ok(_) => println!("Opened URL: {}", url),
                        Err(e) => eprintln!("Failed to open URL: {}. Error: {}", url, e),
                    }
                    let mut stats = stats.lock().unwrap();
                    if stats.build_finished_time.is_none() {
                        let now = SystemTime::now();
                        stats.build_finished_time = Some(now);
                    }
                    state.store(false, Ordering::Relaxed);
                    Some(CallbackResponse {
                        callback_type: CallbackType::Note,
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

    dispatcher
}

/// Create a configured EventDispatcher for stderr diagnostics.
/// You must provide all shared state (Arc/Mutex) needed for diagnostics collection.
#[allow(clippy::too_many_arguments)]
pub fn create_stderr_dispatcher(
    diagnostics: Arc<Mutex<Vec<CargoDiagnostic>>>,
    manifest_path: String,
) -> EventDispatcher {
    let mut dispatcher = EventDispatcher::new();

    let suggestion_mode = Arc::new(AtomicBool::new(false));
    let suggestion_regex = Regex::new(r"^\s*(\d+)\s*\|\s*(.*)$").unwrap();
    let warning_location: Arc<Mutex<Option<CallbackResponse>>> = Arc::new(Mutex::new(None));
    let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::new(Mutex::new(None));
    let diagnostic_counts: Arc<Mutex<HashMap<CargoDiagnosticLevel, usize>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let pending_d: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
    let counts: Arc<Mutex<HashMap<CargoDiagnosticLevel, usize>>> = Arc::clone(&diagnostic_counts);

    let diagnostics_arc: Arc<Mutex<Vec<CargoDiagnostic>>> = Arc::clone(&diagnostics);

    dispatcher.add_callback(
        r"^thread '([^']+)' panicked at (.+):([^\s:]+):(\d+):(\d+)",
        Box::new(|line, captures, _state, _stats| {
            if let Some(caps) = captures {
                let thread = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                let message = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown panic");
                let file = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown file");
                let line_num = caps
                    .get(4)
                    .map(|m| m.as_str())
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
                let col_num = caps
                    .get(5)
                    .map(|m| m.as_str())
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
                println!("\n\n\n");
                println!("{}", line);
                println!(
                    "Panic detected: thread='{}', message='{}', file='{}:{}:{}'",
                    thread, message, file, line_num, col_num
                );
                println!("\n\n\n");
                Some(CallbackResponse {
                    callback_type: CallbackType::Error,
                    message: Some(format!(
                        "thread '{}' panicked at {} ({}:{}:{})",
                        thread, message, file, line_num, col_num
                    )),
                    file: Some(file.to_string()),
                    line: Some(line_num),
                    column: Some(col_num),
                    suggestion: None,
                    terminal_status: None,
                })
            } else {
                None
            }
        }),
    );

    dispatcher.add_callback(
        r"error: could not compile `(?P<crate_name>.+)` \((?P<due_to>.+)\) due to (?P<error_count>\d+) previous errors; (?P<warning_count>\d+) warnings emitted",
        Box::new(|line, captures, _state, stats| {
            println!("{}", line);
            if let Some(caps) = captures {
                let crate_name = caps.name("crate_name").map(|m| m.as_str()).unwrap_or("unknown");
                let due_to = caps.name("due_to").map(|m| m.as_str()).unwrap_or("unknown");
                let error_count: usize = caps
                    .name("error_count")
                    .map(|m| m.as_str().parse().unwrap_or(0))
                    .unwrap_or(0);
                let warning_count: usize = caps
                    .name("warning_count")
                    .map(|m| m.as_str().parse().unwrap_or(0))
                    .unwrap_or(0);

                println!(
                    "Detected compilation failure: crate=`{}`, due_to=`{}`, errors={}, warnings={}",
                    crate_name, due_to, error_count, warning_count
                );

                let mut stats = stats.lock().unwrap();
                stats.is_could_not_compile = true;
            }
            None
        }),
    );

    let diagnostics_arc_for_diag: Arc<Mutex<Vec<CargoDiagnostic>>> = Arc::clone(&diagnostics_arc);
    dispatcher.add_callback(
        r"^(?P<level>\w+)(\[(?P<error_code>E\d+)\])?:\s+(?P<msg>.+)$",
        Box::new(move |_line, caps, _multiline_flag, _stats| {
            if let Some(caps) = caps {
                let mut counts = counts.lock().unwrap();
                let mut pending_diag = pending_d.lock().unwrap();
                let mut last_lineref = String::new();
                if let Some(existing_diag) = pending_diag.take() {
                    let mut diags = diagnostics_arc_for_diag.lock().unwrap();
                    last_lineref = existing_diag.lineref.clone();
                    diags.push(existing_diag.clone());
                }
                log::trace!("Diagnostic line: {}", _line);
                let level = caps["level"].to_string();
                let message = caps["msg"].to_string();
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
                        return None;
                    }
                };
                *counts.entry(diag_level).or_insert(0) += 1;

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

                *pending_diag = Some(diag);

                return Some(CallbackResponse {
                    callback_type: CallbackType::LevelMessage,
                    message: None,
                    file: None,
                    line: None,
                    column: None,
                    suggestion: None,
                    terminal_status: None,
                });
            } else {
                println!("No captures found in line: {}", _line);
                None
            }
        }),
    );

    let look_behind = Arc::new(Mutex::new(Vec::<String>::new()));
    {
        let look_behind = Arc::clone(&look_behind);
        dispatcher.add_callback(
            r"^(?P<msg>.*)$",
            Box::new(move |line, _captures, _state, _stats| {
                let mut buf = look_behind.lock().unwrap();
                if line.trim().is_empty() {
                    return None;
                }
                buf.push(line.to_string());
                if buf.len() > 6 {
                    buf.remove(0);
                }
                None
            }),
        );
    }

    {
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        let diagnostics_arc: Arc<Mutex<Vec<CargoDiagnostic>>> = Arc::clone(&diagnostics_arc);
        let backtrace_mode = Arc::new(AtomicBool::new(false));
        let backtrace_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let look_behind = Arc::clone(&look_behind);
        let stored_lines_behind = Arc::new(Mutex::new(Vec::<String>::new()));

        {
            let backtrace_mode = Arc::clone(&backtrace_mode);
            let backtrace_lines = Arc::clone(&backtrace_lines);
            let stored_lines_behind = Arc::clone(&stored_lines_behind);
            let look_behind = Arc::clone(&look_behind);
            dispatcher.add_callback(
                r"stack backtrace:",
                Box::new(move |_line, _captures, _state, _stats| {
                    backtrace_mode.store(true, Ordering::Relaxed);
                    backtrace_lines.lock().unwrap().clear();
                    {
                        let look_behind_buf = look_behind.lock().unwrap();
                        let mut stored = stored_lines_behind.lock().unwrap();
                        *stored = look_behind_buf.clone();
                    }
                    None
                }),
            );
        }

        {
            let backtrace_mode = Arc::clone(&backtrace_mode);
            let backtrace_lines = Arc::clone(&backtrace_lines);
            let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
            let diagnostics_arc: Arc<Mutex<Vec<CargoDiagnostic>>> = Arc::clone(&diagnostics_arc);
            let look_behind = Arc::clone(&look_behind);

            let re_number_type = Regex::new(r"^\s*(\d+):\s+(.*)$").unwrap();
            let re_at_path = Regex::new(r"^\s*at\s+([^\s:]+):(\d+)").unwrap();

            dispatcher.add_callback(
                r"^(?P<msg>.*)$",
                Box::new(move |mut line, _captures, _state, _stats| {
                    if backtrace_mode.load(Ordering::Relaxed) {
                        line = line.trim();
                        if line.trim().is_empty()
                            || line.starts_with("note:")
                            || line.starts_with("error:")
                        {
                            let mut bt_lines = Vec::new();
                            let mut skip_next = false;
                            let mut last_number_type: Option<(String, String)> = None;
                            for l in backtrace_lines.lock().unwrap().iter() {
                                if let Some(caps) = re_number_type.captures(l) {
                                    last_number_type =
                                        Some((caps[1].to_string(), caps[2].to_string()));
                                    skip_next = true;
                                } else if skip_next && re_at_path.is_match(l) {
                                    let path_caps = re_at_path.captures(l).unwrap();
                                    let path = path_caps.get(1).unwrap().as_str();
                                    let line_num = path_caps.get(2).unwrap().as_str();
                                    if path.starts_with("/rustc")
                                        || path.contains(".cargo")
                                        || path.contains(".rustup")
                                    {
                                        // skip
                                    } else {
                                        if let Some((num, typ)) = last_number_type.take() {
                                            let path = match std::fs::canonicalize(path) {
                                                Ok(canon) => canon.display().to_string(),
                                                Err(_) => path.to_string(),
                                            };
                                            bt_lines.push(format!(
                                                "{}: {} @ {}:{}",
                                                num, typ, path, line_num
                                            ));
                                        }
                                    }
                                    skip_next = false;
                                } else if let Some((num, typ)) = last_number_type.take() {
                                    bt_lines.push(format!("{}: {}", num, typ));
                                    if !l.trim().is_empty() {
                                        bt_lines.push(l.clone());
                                    }
                                    skip_next = false;
                                } else if !l.trim().is_empty() {
                                    bt_lines.push(l.clone());
                                    skip_next = false;
                                }
                            }
                            if !bt_lines.is_empty() {
                                let mut pending_diag = pending_diag.lock().unwrap();
                                if let Some(ref mut diag) = *pending_diag {
                                    let stored_lines = {
                                        let buf = look_behind.lock().unwrap();
                                        buf.clone()
                                    };
                                    let note = diag.note.get_or_insert_with(String::new);
                                    if !stored_lines.is_empty() {
                                        note.push_str(&stored_lines.join("\n"));
                                        note.push('\n');
                                    }
                                    note.push_str(&bt_lines.join("\n"));
                                    let mut diags = diagnostics_arc.lock().unwrap();
                                    diags.push(diag.clone());
                                }
                            }
                            backtrace_mode.store(false, Ordering::Relaxed);
                            backtrace_lines.lock().unwrap().clear();
                            return None;
                        }

                        if re_number_type.is_match(line) || re_at_path.is_match(line) {
                            backtrace_lines.lock().unwrap().push(line.to_string());
                        }
                        return None;
                    }
                    None
                }),
            );
        }
    }

    {
        let location_lock_clone = Arc::clone(&warning_location);
        let suggestion_m = Arc::clone(&suggestion_mode);

        dispatcher.add_callback(
            r"^(?P<msg>.*)$",
            Box::new(move |line, _captures, _multiline_flag, _stats| {
                if suggestion_m.load(Ordering::Relaxed) {
                    if let Some(caps) = suggestion_regex.captures(line.trim()) {
                        let code = caps[2].to_string();

                        if let Ok(mut lock) = location_lock_clone.lock() {
                            if let Some(mut loc) = lock.take() {
                                let mut msg = loc.message.unwrap_or_default();
                                msg.push_str(&format!("\n{}", code));
                                loc.message = Some(msg.clone());
                                lock.replace(loc);
                            }
                        }
                    }
                }
                None
            }),
        );
    }
    {
        let suggestion_m = Arc::clone(&suggestion_mode);
        let pending_diag_clone: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        let diagnostics_arc: Arc<Mutex<Vec<CargoDiagnostic>>> = Arc::clone(&diagnostics_arc);
        dispatcher.add_callback(
            r"^\s*$",
            Box::new(move |_line, _captures, _multiline_flag, _stats| {
                suggestion_m.store(false, Ordering::Relaxed);
                if let Some(pending_diag) = pending_diag_clone.lock().unwrap().take() {
                    let mut diags = diagnostics_arc.lock().unwrap();
                    diags.push(pending_diag.clone());
                }
                None
            }),
        );
    }

    {
        let location_lock = Arc::clone(&warning_location);
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        let suggestion_mode = Arc::clone(&suggestion_mode);
        dispatcher.add_callback(
            r"^(?P<msg>.*)$",
            Box::new(move |line, _captures, _multiline_flag, _stats| {
                if let Ok(location_guard) = location_lock.lock() {
                    if let Some(loc) = location_guard.as_ref() {
                        let file = loc.file.clone().unwrap_or_default();
                        let line_num = loc.line.unwrap_or(0);
                        let col = loc.column.unwrap_or(0);

                        if line.trim().starts_with('|') || line.trim().starts_with(char::is_numeric)
                        {
                            let suggestion = line.trim();

                            let mut pending_diag = match pending_diag.lock() {
                                Ok(lock) => lock,
                                Err(e) => {
                                    eprintln!("Failed to acquire lock: {}", e);
                                    return None;
                                }
                            };
                            if let Some(diag) = pending_diag.take() {
                                let mut diag = diag;
                                if let Some(ref mut existing) = diag.suggestion {
                                    diag.suggestion = Some(format!("{}\n{}", existing, suggestion));
                                } else {
                                    diag.suggestion = Some(suggestion.to_string());
                                }
                                *pending_diag = Some(diag.clone());
                                return Some(CallbackResponse {
                                    callback_type: CallbackType::Suggestion,
                                    message: Some(diag.clone().suggestion.clone().unwrap().clone()),
                                    file: Some(file),
                                    line: Some(line_num),
                                    column: Some(col),
                                    suggestion: diag.clone().suggestion.clone(),
                                    terminal_status: None,
                                });
                            }
                        } else {
                            if line.trim().is_empty() {
                                suggestion_mode.store(false, Ordering::Relaxed);
                                return None;
                            }
                        }
                    }
                }
                None
            }),
        );
    }

    {
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        let warning_location = Arc::clone(&warning_location);
        let location_lock = Arc::clone(&warning_location);
        let suggestion_mode = Arc::clone(&suggestion_mode);
        let manifest_path = manifest_path.clone();
        dispatcher.add_callback(
            r"^\s*-->\s+(?P<file>.+?)(?::(?P<line>\d+))?(?::(?P<col>\d+))?\s*$",
            Box::new(move |_line, caps, _multiline_flag, _stats| {
                log::trace!("Location line: {}", _line);
                if let Some(caps) = caps {
                    let file = caps["file"].to_string();
                    let manifest_path_buf = PathBuf::from(&manifest_path);
                    let resolved_path = resolve_file_path(&manifest_path_buf, &file);
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
                    let mut pending_diag = pending_diag.lock().unwrap();
                    if let Some(diag) = pending_diag.take() {
                        let mut diag = diag;
                        diag.lineref = format!("{}:{}:{}", file, line, column);
                        *pending_diag = Some(diag);
                    }
                    *warning_location.lock().unwrap() = Some(resp.clone());
                    *location_lock.lock().unwrap() = Some(resp.clone());
                    suggestion_mode.store(true, Ordering::Relaxed);
                    return Some(resp.clone());
                } else {
                    println!("No captures found in line: {}", _line);
                }
                None
            }),
        );
    }

    {
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        dispatcher.add_callback(
            r"^\s*=\s*note:\s*(?P<msg>.+)$",
            Box::new(move |_line, caps, _state, _stats| {
                if let Some(caps) = caps {
                    let mut pending_diag = pending_diag.lock().unwrap();
                    if let Some(ref mut resp) = *pending_diag {
                        let new_note = format!("note: {}", caps["msg"].to_string());
                        if let Some(existing_note) = &resp.note {
                            resp.note = Some(format!("{}\n{}", existing_note, new_note));
                        } else {
                            resp.note = Some(new_note);
                        }
                    }
                }
                None
            }),
        );
    }

    {
        let pending_diag: Arc<Mutex<Option<CargoDiagnostic>>> = Arc::clone(&pending_diag);
        dispatcher.add_callback(
            r"^\s*(?:\=|\|)\s*help:\s*(?P<msg>.+)$",
            Box::new(move |_line, caps, _state, _stats| {
                if let Some(caps) = caps {
                    let mut pending_diag = pending_diag.lock().unwrap();
                    if let Some(ref mut resp) = *pending_diag {
                        let new_help =
                            format!("\x1b[38;5;214mhelp: {}\x1b[0m", caps["msg"].to_string());
                        if let Some(existing_help) = &resp.help {
                            resp.help = Some(format!("{}\n{}", existing_help, new_help));
                        } else {
                            resp.help = Some(new_help);
                        }
                    }
                }
                None
            }),
        );
    }

    dispatcher.add_callback(
        r"(?:\x1b\[[0-9;]*[A-Za-z])*\s*Serving(?:\x1b\[[0-9;]*[A-Za-z])*\s+at\s+(http://[^\s]+)",
        Box::new(|line, captures, _state, stats| {
            if let Some(caps) = captures {
                let url = caps.get(1).unwrap().as_str();
                let url = url.replace("0.0.0.0", "127.0.0.1");
                println!("(STDERR) Captured URL: {}", url);
                match open::that_detached(&url) {
                    Ok(_) => println!("(STDERR) Opened URL: {}", &url),
                    Err(e) => eprintln!("(STDERR) Failed to open URL: {}. Error: {:?}", url, e),
                }
                let mut stats = stats.lock().unwrap();
                if stats.build_finished_time.is_none() {
                    let now = SystemTime::now();
                    stats.build_finished_time = Some(now);
                }
                Some(CallbackResponse {
                    callback_type: CallbackType::OpenedUrl,
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
    {
        let finished_flag = Arc::clone(&finished_flag);
        dispatcher.add_callback(
            r"^Finished\s+`(?P<profile>[^`]+)`\s+profile\s+\[(?P<opts>[^\]]+)\]\s+target\(s\)\s+in\s+(?P<dur>[0-9.]+s)$",
            Box::new(move |_line, caps, _multiline_flag, stats | {
                if let Some(caps) = caps {
                    finished_flag.store(true, Ordering::Relaxed);
                    let profile = &caps["profile"];
                    let opts    = &caps["opts"];
                    let dur     = &caps["dur"];
                    let mut stats = stats.lock().unwrap();
                    if stats.build_finished_time.is_none() {
                        let now = SystemTime::now();
                        stats.build_finished_time = Some(now);
                    }
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
        dispatcher.add_callback(
            r"^(?P<level>warning|error):\s+`(?P<name>[^`]+)`\s+\((?P<otype>lib|bin)\)\s+generated\s+(?P<count>\d+)\s+(?P<kind>warnings|errors).*run\s+`(?P<cmd>[^`]+)`\s+to apply\s+(?P<fixes>\d+)\s+suggestions",
            Box::new(move |_line, caps, multiline_flag, _stats | {
                let summary_flag = Arc::clone(&summary_flag);
                if let Some(caps) = caps {
                    summary_flag.store(true, Ordering::Relaxed);
                    multiline_flag.store(false, Ordering::Relaxed);

                    let level    = &caps["level"];
                    let name     = &caps["name"];
                    let otype    = &caps["otype"];
                    let count: usize = caps["count"].parse().unwrap_or(0);
                    let kind     = &caps["kind"];
                    let cmd      = caps["cmd"].to_string();
                    let fixes: usize = caps["fixes"].parse().unwrap_or(0);

                    println!("SUMMARIZATION CALLBACK {}",
                        &format!("{}: `{}` ({}) generated {} {}; run `{}` to apply {} fixes",
                        level, name, otype, count, kind, cmd, fixes));
                    Some(CallbackResponse {
                        callback_type: CallbackType::Note,
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

    dispatcher.add_callback(
        r"IO\(Custom \{ kind: NotConnected",
        Box::new(move |line, _captures, _state, _stats| {
            println!("(STDERR) Terminal error detected: {:?}", &line);
            let result = if line.contains("NotConnected") {
                TerminalError::NoTerminal
            } else {
                TerminalError::NoError
            };
            // let sender = sender.lock().unwrap();
            // sender.send(result).ok();
            Some(CallbackResponse {
                callback_type: CallbackType::Warning,
                message: Some(format!("Terminal Error: {}", line)),
                file: None,
                line: None,
                column: None,
                suggestion: None,
                terminal_status: None,
            })
        }),
    );
    dispatcher.add_callback(
        r".*",
        Box::new(|line, _captures, _state, _stats| {
            log::trace!("stdraw[{:?}]", line);
            println!("{}", line);
            None
        }),
    );

    dispatcher
}
