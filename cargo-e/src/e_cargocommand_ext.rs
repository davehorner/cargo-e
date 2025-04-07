
use crate::e_command_builder::TerminalError;
use crate::e_eventdispatcher::{CargoDiagnosticLevel, EventDispatcher};
use crate::e_runner::GLOBAL_CHILDREN;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fmt, thread};
use std::time::{SystemTime, Duration};
use std::process::ExitStatus;
use cargo_metadata::Message;
use nu_ansi_term::{AnsiGenericString, Color, Style};
use regex::Regex;
use serde_json;
use tracing::instrument::WithSubscriber;
use std::io::{self, Write};
use crossterm::{execute, cursor, terminal::{Clear, ClearType}};
// enum CaptureMode {
//     Filtering(DispatcherSet),
//     Passthrough { stdout: std::io::Stdout, stderr: std::io::Stderr },
// }
// struct DispatcherSet {
//     stdout: Option<Arc<EventDispatcher>>,
//     stderr: Option<Arc<EventDispatcher>>,
//     progress: Option<Arc<EventDispatcher>>,
//     stage: Option<Arc<EventDispatcher>>,
// }

/// CargoStats tracks counts for different cargo events and also stores the first occurrence times.
#[derive(Debug, Default, Clone)]
pub struct CargoStats {
    pub compiler_message_count: usize,
    pub compiler_artifact_count: usize,
    pub build_script_executed_count: usize,
    pub build_finished_count: usize,
    // Record the first occurrence of each stage.
    pub compiler_message_time: Option<SystemTime>,
    pub compiler_artifact_time: Option<SystemTime>,
    pub build_script_executed_time: Option<SystemTime>,
    pub build_finished_time: Option<SystemTime>,
}
 
#[derive(Clone)]
pub struct CargoDiagnostic {
    pub lineref: String, 
    pub level: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub note: Option<String>,
    pub help: Option<String>,
    pub uses_color: bool,
    pub diag_number: Option<usize>,
    pub diag_num_padding: Option<usize>,
}
impl CargoDiagnostic {
    pub fn new(
        lineref: String,
        level: String,
        message: String,
        suggestion: Option<String>,
        note: Option<String>,
        help: Option<String>,
        uses_color: bool,
        diag_number: Option<usize>,
        diag_num_padding: Option<usize>,
    ) -> Self {
        CargoDiagnostic {
            lineref,
            level,
            message,
            suggestion,
            note,
            help,
            uses_color,
            diag_number,
            diag_num_padding,
        }
    }


    fn update_suggestion_with_lineno(&self, suggestion: &str, file: String, line_number: usize) -> String {
        // Regex to match line number in the suggestion (e.g., "79 | fn clean<S: AsRef<str>>(s: S) -> String {")
        let suggestion_regex = regex::Regex::new(r"(?P<line>\d+)\s*\|\s*(.*)").unwrap();

        // Iterate through suggestion lines and check line numbers
        suggestion
            .lines()
            .filter_map(|line| {
                                let binding = line.replace(|c: char| c == '|' || c == '^', "");
                                let cleaned_line = binding.trim();
                
                // If the line becomes empty after removing | and ^, skip it
                if cleaned_line.is_empty() {
                    return None; // Skip empty lines
                }
                if let Some(caps) = suggestion_regex.captures(line.trim()) {
                    let suggestion_line: usize = caps["line"]
                        .parse()
                        .unwrap_or_else(|_| line_number); // If parsing fails, use the default line number
                    // Replace the line number if it doesn't match the diagnostic's line number
                    if suggestion_line != line_number {
                        return Some(format!(
                            "{}:{} | {}",
                           file,
                            suggestion_line, // Replace with the actual diagnostic line number
                            caps.get(2).map_or("", |m| m.as_str())
                        ));
                    }
                }
                Some(line.to_string())
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl fmt::Debug for CargoDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Capitalize the first letter of the level
        let struct_name = self.level.chars().next().unwrap_or(' ').to_uppercase().to_string();

        // Extract the file and line number from lineref (e.g., "cargo-e\src\e_command_builder.rs:79:8")
        let lineref_regex = regex::Regex::new(r"(?P<file>.*):(?P<line>\d+):(?P<col>\d+)").unwrap();
        let lineref_caps = lineref_regex.captures(&self.lineref);

        let file = lineref_caps
                    .as_ref()
                    .and_then(|caps| caps.name("file").map(|m| m.as_str().to_string()))
                    .unwrap_or_else(|| "unknown file".to_string());
        let line_number: usize = lineref_caps
                    .as_ref()
                    .and_then(|caps| caps.name("line").map(|m| m.as_str().parse().unwrap_or(0))).unwrap_or(0);

                // Print the diagnostic number and level (e.g., W01: or E001:)
        let diag_number = if let Some(dn) = &self.diag_number {
            format!("{:0width$}", dn, width = self.diag_num_padding.unwrap_or(0)) // Apply padding to the number
        } else {
            String::new()
        };

        // Color the diagnostic number based on the level
        let diag_number_colored = match self.level.as_str() {
            "warning" => Color::Yellow.paint(format!("W{}:", diag_number)),
            "error" => Color::Red.paint(format!("E{}:", diag_number)),
            _ => Color::Green.paint(format!("N{}:", diag_number)), // Default to green for notes
        };

        // Print the struct name (capitalized level) and lineref
        write!(f, "{} ", diag_number_colored)?;
        // Print the struct name (capitalized level) and lineref
        // write!(f, "{}: ", struct_name)?;

        // Always show lineref with underline if uses_color is true
        if self.uses_color {
            let underlined_text = Style::new().underline().paint(&self.lineref).to_string();
            write!(f, "\x1b[4m{}\x1b[0m ", underlined_text)?; // Apply underline using ANSI codes
        } else {
            write!(f, "{} ", self.lineref)?; // Plain lineref without color
        }

        // Print the message with color if necessary
        let message = match self.level.as_str() {
            "warning" => Color::Yellow.paint(&self.message).to_string(),
            "error" => Color::Red.paint(&self.message).to_string(),
            _ => Color::Green.paint(&self.message).to_string(),
        };
        write!(f, "{}: ", message)?;

        // Print the suggestion if present, with color if uses_color is true
        if let Some(suggestion) = &self.suggestion {
            let suggestion = self.update_suggestion_with_lineno(suggestion, file, line_number);

            let suggestion_text = if self.uses_color {
                Color::Green.paint(suggestion).to_string()
            } else {
                suggestion.clone()
            };
            write!(f, "{} ", suggestion_text)?;
        }

        // Print the note if present, with color if uses_color is true
        if let Some(note) = &self.note {
            let note_text = if self.uses_color {
                Color::Blue.paint(note).to_string()
            } else {
                note.clone()
            };
            write!(f, "\n{}", note_text)?;
        }

        // Print the help if present, with color if uses_color is true
        if let Some(help) = &self.help {
            let help_text = if self.uses_color {
                Color::LightYellow.paint(help).to_string()
            } else {
                help.clone()
            };
            write!(f, "\n{} ", help_text)?;
        }

        // Finish the debug formatting
        write!(f, "") // No further fields are needed
    }
}



/// CargoProcessResult is returned when the cargo process completes.
#[derive(Debug,Default, Clone)]
pub struct CargoProcessResult {
    pub pid: u32,
    pub terminal_error: Option<TerminalError>,
    pub exit_status: Option<ExitStatus>,
    pub start_time: Option<SystemTime>,
    pub build_finished_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub build_elapsed: Option<Duration>,
    pub runtime_elapsed: Option<Duration>,
    pub stats: CargoStats,
    pub build_output_size: usize,
    pub runtime_output_size: usize,
    pub diagnostics: Vec<CargoDiagnostic>,
}

impl CargoProcessResult {
    /// Print every diagnostic in full detail.
    pub fn print_exact(&self) {
        println!("--- Full Diagnostics for PID {} ---", self.pid);
        for diag in &self.diagnostics {
            println!("{} {:?}: {} {}",diag.lineref, diag.level, diag.message,diag.suggestion.clone().unwrap_or_default());
        }
    }

    /// Print warnings first, then errors, one‐line summary.
    pub fn print_short(&self) {
        let warnings: Vec<_> = self.diagnostics.iter()
            .filter(|d| d.level.eq("warning"))
            .collect();
        let errors: Vec<_> = self.diagnostics.iter()
            .filter(|d| d.level.eq("error"))
            .collect();

        println!("--- Warnings ({} total) ---", warnings.len());
        for d in warnings {
            println!("W: {}:{}:{} – {}", d.lineref, d.level, d.message, d.suggestion.clone().unwrap_or_default());
        }

        println!("--- Errors ({} total) ---", errors.len());
        for d in errors {
            println!("E: {}:{}:{} – {}", d.lineref, d.level, d.message, d.suggestion.clone().unwrap_or_default());
        }
    }

    /// Print a compact, zero‑padded, numbered list of *all* diagnostics.
    pub fn print_compact(&self) {
        let total = self.diagnostics.len();
        println!("--- All Diagnostics ({} total) ---", total);
    }
}

 
/// CargoProcessHandle holds the cargo process and related state.
#[derive(Debug)]
pub struct CargoProcessHandle {
    pub child: Child,
    pub result: CargoProcessResult,
    pub pid: u32,
    pub requested_exit: bool,
    pub stdout_handle: thread::JoinHandle<()>,
    pub stderr_handle: thread::JoinHandle<()>,
    pub start_time: SystemTime,
    pub stats: Arc<Mutex<CargoStats>>,
    pub stdout_dispatcher: Option<Arc<EventDispatcher>>,
    pub stderr_dispatcher: Option<Arc<EventDispatcher>>,
    pub progress_dispatcher: Option<Arc<EventDispatcher>>,
    pub stage_dispatcher: Option<Arc<EventDispatcher>>,
    pub estimate_bytes: Option<usize>,
    // Separate progress counters for build and runtime output.
    pub build_progress_counter: Arc<AtomicUsize>,
    pub runtime_progress_counter: Arc<AtomicUsize>,
    pub terminal_error_flag: Arc<Mutex<TerminalError>>,
    pub diagnostics: Arc<Mutex<Vec<CargoDiagnostic>>>,
}
 
impl CargoProcessHandle {

pub fn print_results(result: &CargoProcessResult) {
    let start_time = result.start_time.unwrap_or(SystemTime::now());
    println!("-------------------------------------------------");
    println!("Process started at: {:?}", result.start_time);
    if let Some(build_time) = result.build_finished_time {
        println!("Build phase ended at: {:?}", build_time);
        println!("Build phase elapsed:  {}", crate::e_fmt::format_duration(build_time.duration_since(start_time).unwrap_or_else(|_| Duration::new(0, 0))));
    } else {
        println!("No BuildFinished timestamp recorded.");
    }
    println!("Process ended at:   {:?}", result.end_time);
    if let Some(runtime_dur) = result.runtime_elapsed {
        println!("Runtime phase elapsed: {}", crate::e_fmt::format_duration(runtime_dur));
    }
    if let Some(build_dur) = result.build_elapsed {
        println!("Build phase elapsed:   {}", crate::e_fmt::format_duration(build_dur));
    }
    if let Some(total_elapsed) = result.end_time.and_then(|end| end.duration_since(start_time).ok()) {
        println!("Total elapsed time:   {}", crate::e_fmt::format_duration(total_elapsed));
    } else {
        println!("No total elapsed time available.");
    }
    println!("Build output size:  {} ({} bytes)", crate::e_fmt::format_bytes(result.build_output_size), result.build_output_size);
    println!("Runtime output size: {} ({} bytes)", crate::e_fmt::format_bytes(result.runtime_output_size), result.runtime_output_size);
    println!("-------------------------------------------------");
}
 
    /// Kill the cargo process if needed.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }

    //     pub fn wait(&mut self) -> std::io::Result<CargoProcessResult> {
    //     // Lock the instance since `self` is an `Arc`
    //     // let mut cargo_process_handle = self.lock().unwrap();  // `lock()` returns a mutable reference

    //     // Call wait on the child process
    //     let status = self.child.wait()?;  // Call wait on the child process

    //     println!("Cargo process finished with status: {:?}", status);
        
    //     let end_time = SystemTime::now();

    //     // Retrieve the statistics from the process handle
    //     let stats = Arc::try_unwrap(self.stats.clone())
    //         .map(|mutex| mutex.into_inner().unwrap())
    //         .unwrap_or_else(|arc| (*arc.lock().unwrap()).clone());

    //     let build_out = self.build_progress_counter.load(Ordering::Relaxed);
    //     let runtime_out = self.runtime_progress_counter.load(Ordering::Relaxed);

    //     // Calculate phase durations if build_finished_time is recorded
    //     let (build_elapsed, runtime_elapsed) = if let Some(build_finished) = stats.build_finished_time {
    //         let build_dur = build_finished.duration_since(self.start_time)
    //             .unwrap_or_else(|_| Duration::new(0, 0));
    //         let runtime_dur = end_time.duration_since(build_finished)
    //             .unwrap_or_else(|_| Duration::new(0, 0));
    //         (Some(build_dur), Some(runtime_dur))
    //     } else {
    //         (None, None)
    //     };

    //     self.result.exit_status = Some(status);
    //     self.result.end_time = Some(end_time);
    //     self.result.build_output_size = self.build_progress_counter.load(Ordering::Relaxed);
    //     self.result.runtime_output_size = self.runtime_progress_counter.load(Ordering::Relaxed);

    //     Ok(self.result.clone())
    //     // Return the final process result
    //     // Ok(CargoProcessResult {
    //     //     pid: self.pid,
    //     //     exit_status: Some(status),
    //     //     start_time: Some(self.start_time),
    //     //     build_finished_time: stats.build_finished_time,
    //     //     end_time: Some(end_time),
    //     //     build_elapsed,
    //     //     runtime_elapsed,
    //     //     stats,
    //     //     build_output_size: build_out,
    //     //     runtime_output_size: runtime_out,
    //     // })
    // }
 
//  pub fn wait(self: Arc<Self>) -> std::io::Result<CargoProcessResult> {
//     let mut global = GLOBAL_CHILDREN.lock().unwrap();
    
//     // Lock and access the CargoProcessHandle inside the Mutex
//     if let Some(cargo_process_handle) = global.iter_mut().find(|handle| {
//         handle.lock().unwrap().pid == self.pid  // Compare the pid to find the correct handle
//     }) {
//         let mut cargo_process_handle = cargo_process_handle.lock().unwrap();  // Mutably borrow the process handle
        
//         let status = cargo_process_handle.child.wait()?;  // Call wait on the child process

//         println!("Cargo process finished with status: {:?}", status);
        
//         let end_time = SystemTime::now();

//         // Retrieve the statistics from the process handle
//         let stats = Arc::try_unwrap(cargo_process_handle.stats.clone())
//             .map(|mutex| mutex.into_inner().unwrap())
//             .unwrap_or_else(|arc| (*arc.lock().unwrap()).clone());

//         let build_out = cargo_process_handle.build_progress_counter.load(Ordering::Relaxed);
//         let runtime_out = cargo_process_handle.runtime_progress_counter.load(Ordering::Relaxed);

//         // Calculate phase durations if build_finished_time is recorded
//         let (build_elapsed, runtime_elapsed) = if let Some(build_finished) = stats.build_finished_time {
//             let build_dur = build_finished.duration_since(cargo_process_handle.start_time)
//                 .unwrap_or_else(|_| Duration::new(0, 0));
//             let runtime_dur = end_time.duration_since(build_finished)
//                 .unwrap_or_else(|_| Duration::new(0, 0));
//             (Some(build_dur), Some(runtime_dur))
//         } else {
//             (None, None)
//         };

//         // Return the final process result
//         Ok(CargoProcessResult {
//             exit_status: status,
//             start_time: cargo_process_handle.start_time,
//             build_finished_time: stats.build_finished_time,
//             end_time,
//             build_elapsed,
//             runtime_elapsed,
//             stats,
//             build_output_size: build_out,
//             runtime_output_size: runtime_out,
//         })
//     } else {
//         Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Process handle not found").into())
//     }
// }

    // Wait for the process and output threads to finish.  
    // Computes elapsed times for the build phase and runtime phase, and returns a CargoProcessResult.
    // pub fn wait(mut self) -> std::io::Result<CargoProcessResult> {
    //     let status = self.child.wait()?;
    //     println!("Cargo process finished with status: {:?}", status);
 
    //     self.stdout_handle.join().expect("stdout thread panicked");
    //     self.stderr_handle.join().expect("stderr thread panicked");
 
    //     let end_time = SystemTime::now();
 
    //     // Retrieve the statistics.
    //     let stats = Arc::try_unwrap(self.stats)
    //         .map(|mutex| mutex.into_inner().unwrap())
    //         .unwrap_or_else(|arc| (*arc.lock().unwrap()).clone());
 
    //     let build_out = self.build_progress_counter.load(Ordering::Relaxed);
    //     let runtime_out = self.runtime_progress_counter.load(Ordering::Relaxed);
 
    //     // Calculate phase durations if build_finished_time is recorded.
    //     let (build_elapsed, runtime_elapsed) = if let Some(build_finished) = stats.build_finished_time {
    //         let build_dur = build_finished.duration_since(self.start_time).unwrap_or_else(|_| Duration::new(0, 0));
    //         let runtime_dur = end_time.duration_since(build_finished).unwrap_or_else(|_| Duration::new(0, 0));
    //         (Some(build_dur), Some(runtime_dur))
    //     } else {
    //         (None, None)
    //     };
 
    //     Ok(CargoProcessResult {
    //         exit_status: status,
    //         start_time: self.start_time,
    //         build_finished_time: stats.build_finished_time,
    //         end_time,
    //         build_elapsed,
    //         runtime_elapsed,
    //         stats,
    //         build_output_size: build_out,
    //         runtime_output_size: runtime_out,
    //     })
    // }




    /// Returns a formatted status string.
    /// If `system` is provided, CPU/memory and runtime info is displayed on the right.
    /// Otherwise, only the start time is shown.
    pub fn format_status(&self, system: Option<&sysinfo::System>) -> String {
        // Ensure the start time is available.
        let start_time = self
            .result
            .start_time
            .expect("start_time should be initialized");
        let start_dt: chrono::DateTime<chrono::Local> = start_time.into();
        let start_str = start_dt.format("%H:%M:%S").to_string();
        // Use ANSI coloring for the left display.
        let colored_start = nu_ansi_term::Color::Green.paint(&start_str).to_string();

        if let Some(system) = system {
            if let Some(process) = system.process((self.pid as usize).into()) {
                let cpu_usage = process.cpu_usage();
                let mem_kb = process.memory();
                let mem_human = if mem_kb >= 1024 {
                    format!("{:.2} MB", mem_kb as f64 / 1024.0)
                } else {
                    format!("{} KB", mem_kb)
                };

                let now = SystemTime::now();
                let runtime_duration = now.duration_since(start_time).unwrap();
                let runtime_str = crate::e_fmt::format_duration(runtime_duration);

                let left_display = format!(
                    "{} | CPU: {:.2}% | Mem: {}",
                    colored_start, cpu_usage, mem_human
                );
                // Use plain text for length calculations.
                let left_plain = format!("{} | CPU: {:.2}% | Mem: {}", start_str, cpu_usage, mem_human);

                // Get terminal width.
                let (cols, _) = crossterm::terminal::size().unwrap_or((80, 20));
                let total_width = cols as usize;

                // Format the runtime info with underlining.
                let right_display = nu_ansi_term::Style::new()
                    .reset_before_style()
                    .underline()
                    .paint(&runtime_str)
                    .to_string();
                let left_len = left_plain.len();
                let right_len = runtime_str.len();
                let padding = if total_width > left_len + right_len {
                    total_width - left_len - right_len
                } else {
                    1
                };

                let ret=format!("{}{}{}", left_display, " ".repeat(padding), right_display);
                if ret.trim().is_empty() {
                    return String::from("No output available");
                } else {
                    return ret;
                }
            } else {
                return format!("Process {} not found",(self.pid as usize));
                return String::new()
            }
        } else {
            // If system monitoring is disabled, just return the start time.
            colored_start
        }
    }
}
 
/// Extension trait to add cargo-specific capture capabilities to Command.
pub trait CargoCommandExt {
    fn spawn_cargo_capture(
        &mut self,
        stdout_dispatcher: Option<Arc<EventDispatcher>>,
        stderr_dispatcher: Option<Arc<EventDispatcher>>,
        progress_dispatcher: Option<Arc<EventDispatcher>>,
        stage_dispatcher: Option<Arc<EventDispatcher>>,
        estimate_bytes: Option<usize>,
    ) -> CargoProcessHandle;
    fn spawn_cargo_passthrough(
        &mut self,
    ) -> CargoProcessHandle;
}
 
impl CargoCommandExt for Command {


        fn spawn_cargo_passthrough(
        &mut self,
    ) -> CargoProcessHandle {
        // Spawn the child process without redirecting stdout and stderr
        let child = self.spawn().expect("Failed to spawn cargo process");

        let pid = child.id();
        let start_time = SystemTime::now();
        let diagnostics = Arc::new(Mutex::new(Vec::<CargoDiagnostic>::new()));

        let stats = Arc::new(Mutex::new(CargoStats::default()));

                // Try to take ownership of the Vec<CargoDiagnostic> out of the Arc.

        // Create the CargoProcessHandle
        let result = CargoProcessResult {
            pid,
            terminal_error: None,
            exit_status: None,
            start_time: Some(start_time),
            build_finished_time: None,
            end_time: None,
            build_elapsed: None,
            runtime_elapsed: None,
            stats: CargoStats::default(),
            build_output_size: 0,
            runtime_output_size: 0,
            diagnostics: Vec::new(),
        };

        // Return the CargoProcessHandle that owns the child process
        CargoProcessHandle {
            child,          // The child process is now owned by the handle
            result,         // The result contains information about the process
            pid,            // The PID of the process
            stdout_handle: thread::spawn(move || {
                // This thread is now unnecessary if we are not capturing anything
                // We can leave it empty or remove it altogether
            }),
            stderr_handle: thread::spawn(move || {
                // This thread is also unnecessary if we are not capturing anything
            }),
            start_time,
            stats,
            stdout_dispatcher: None,    // No dispatcher is needed
            stderr_dispatcher: None,    // No dispatcher is needed
            progress_dispatcher: None, // No dispatcher is needed
            stage_dispatcher: None,    // No dispatcher is needed
            estimate_bytes: None,
            build_progress_counter: Arc::new(AtomicUsize::new(0)),
            runtime_progress_counter: Arc::new(AtomicUsize::new(0)),
            requested_exit: false,
            terminal_error_flag: Arc::new(Mutex::new(TerminalError::NoError)),
            diagnostics: diagnostics,
        }
    }


    fn spawn_cargo_capture(
        &mut self,
        stdout_dispatcher: Option<Arc<EventDispatcher>>,
        stderr_dispatcher: Option<Arc<EventDispatcher>>,
        progress_dispatcher: Option<Arc<EventDispatcher>>,
        stage_dispatcher: Option<Arc<EventDispatcher>>,
        estimate_bytes: Option<usize>,
    ) -> CargoProcessHandle {
        self.stdout(Stdio::piped())
            .stderr(Stdio::piped());
// println!("Spawning cargo process with capture {:?}",self); 
        let mut child = self.spawn().expect("Failed to spawn cargo process");
        let pid= child.id();
        let start_time = SystemTime::now();
        let diagnostics = Arc::new(Mutex::new(Vec::<CargoDiagnostic>::new()));
        let stats = Arc::new(Mutex::new(CargoStats::default()));
 
        // Two separate counters: one for build output and one for runtime output.
 let stderr_compiler_msg = Arc::new(Mutex::new(VecDeque::<String>::new()));
        let build_progress_counter = Arc::new(AtomicUsize::new(0));
        let runtime_progress_counter = Arc::new(AtomicUsize::new(0));
 
        // Clone dispatchers and counters for use in threads.
        let stdout_disp_clone = stdout_dispatcher.clone();
        let progress_disp_clone_stdout = progress_dispatcher.clone();
        let stage_disp_clone = stage_dispatcher.clone();
 
        let stats_clone = Arc::clone(&stats);
        let build_counter_stdout = Arc::clone(&build_progress_counter);
        let runtime_counter_stdout = Arc::clone(&runtime_progress_counter);
 
        // Spawn a thread to process stdout.
        let stderr_compiler_msg_clone = Arc::clone(&stderr_compiler_msg);
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        println!("{}: Capturing stdout", pid);
        let stdout_handle = thread::spawn(move || {
            let stdout_reader = BufReader::new(stdout);
            // This flag marks whether we are still in the build phase.
            let mut in_build_phase = true;
            {

            for line in stdout_reader.lines() {
                if let Ok(line) = line {
// println!("{}: {}", pid, line);
                    // Try to parse the line as a JSON cargo message.
                    match serde_json::from_str::<Message>(&line) {
                        Ok(msg) => {
                            // let msg_str = format!("{:?}", msg);
                            // if let Some(ref disp) = stdout_disp_clone {
                            //     disp.dispatch(&msg_str);
                            // }
                            // Add message length to the appropriate counter.
                            // if in_build_phase {
                            //     build_counter_stdout.fetch_add(msg_str.len(), Ordering::Relaxed);
                            // } else {
                            //     runtime_counter_stdout.fetch_add(msg_str.len(), Ordering::Relaxed);
                            // }
                            if let Some(total) = estimate_bytes {
                                let current = if in_build_phase {
                                    build_counter_stdout.load(Ordering::Relaxed)
                                } else {
                                    runtime_counter_stdout.load(Ordering::Relaxed)
                                };
                                let progress = (current as f64 / total as f64) * 100.0;
                                if let Some(ref pd) = progress_disp_clone_stdout {
                                    pd.dispatch(&format!("Progress: {:.2}%", progress));
                                }
                            }
 
                            let now = SystemTime::now();
                            // Process known cargo message variants.
                            match msg {
                                Message::BuildFinished(_) => {
                                    // Mark the end of the build phase.
                                    if in_build_phase {
                                        in_build_phase = false;
                                        let mut s = stats_clone.lock().unwrap();
                                        s.build_finished_count += 1;
                                        s.build_finished_time.get_or_insert(now);
                                        // self.result.build_finished_time = Some(now);
                                        if let Some(ref sd) = stage_disp_clone {
                                            sd.dispatch(&format!("Stage: BuildFinished occurred at {:?}", now));
                                        }
                                            if let Some(ref sd) = stage_disp_clone {
                                               sd.dispatch("Stage: Switching to runtime passthrough");
                                            }
                                    }
                                }
                                Message::CompilerMessage(msg) => {
                            println!("parsed{}: {:?}", pid, msg);
                                    let mut s = stats_clone.lock().unwrap();
                                    s.compiler_message_count += 1;
                                    if s.compiler_message_time.is_none() {
                                        s.compiler_message_time = Some(now);
                                        if let Some(ref sd) = stage_disp_clone {
                                            sd.dispatch(&format!("Stage: CompilerMessage occurred at {:?}", now));
                                        }
                                    }
                                    let mut msg_vec = stderr_compiler_msg_clone.lock().unwrap();
                                    msg_vec.push_back(msg.message.rendered.unwrap_or_default().to_string());
                                    // let mut diags = diagnostics.lock().unwrap();            
                                    // let diag = crate::e_eventdispatcher::convert_message_to_diagnostic(msg, &msg_str);
                                    // diags.push(diag.clone());
                                    // if let Some(ref sd) = stage_disp_clone {
                                    //     sd.dispatch(&format!("Stage: Diagnostic occurred at {:?}", now));
                                    // }
                                }
                                Message::CompilerArtifact(_) => {
                                    let mut s = stats_clone.lock().unwrap();
                                    s.compiler_artifact_count += 1;
                                    if s.compiler_artifact_time.is_none() {
                                        s.compiler_artifact_time = Some(now);
                                        if let Some(ref sd) = stage_disp_clone {
                                            sd.dispatch(&format!("Stage: CompilerArtifact occurred at {:?}", now));
                                        }
                                    }
                                }
                                Message::BuildScriptExecuted(_) => {
                                    let mut s = stats_clone.lock().unwrap();
                                    s.build_script_executed_count += 1;
                                    if s.build_script_executed_time.is_none() {
                                        s.build_script_executed_time = Some(now);
                                        if let Some(ref sd) = stage_disp_clone {
                                            sd.dispatch(&format!("Stage: BuildScriptExecuted occurred at {:?}", now));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(_err) => {
                            println!("ERROR {} {}: {}",_err, pid, line);
                            // If JSON parsing fails, assume this is plain runtime output.
                            // If still in build phase, we assume the build phase has ended.
                            if in_build_phase {
                                in_build_phase = false;
                                let now = SystemTime::now();
                                let mut s = stats_clone.lock().unwrap();
                                s.build_finished_count += 1;
                                s.build_finished_time.get_or_insert(now);
                                if let Some(ref sd) = stage_disp_clone {
                                    sd.dispatch(&format!("Stage: BuildFinished (assumed) occurred at {:?}", now));
                                }
                            }
                            if let Some(ref disp) = stdout_disp_clone {
                                disp.dispatch(&line);
                            }
                            // Print the runtime output.
                            println!("{}: {}", pid, line);
                            if line.contains("not a terminal") {
                                println!("{}NOT A TERMINAL - MARK AND RUN AGAIN: {}", pid, line);
                            }
                            runtime_counter_stdout.fetch_add(line.len(), Ordering::Relaxed);
                            if let Some(total) = estimate_bytes {
                                let current = runtime_counter_stdout.load(Ordering::Relaxed);
                                let progress = (current as f64 / total as f64) * 100.0;
                                if let Some(ref pd) = progress_disp_clone_stdout {
                                    pd.dispatch(&format!("Progress: {:.2}%", progress));
                                }
                            }
                        }
                    }
                }
            }
        }
        });
 
     let tflag = TerminalError::NoError;
        // Create a flag to indicate if the process is a terminal process.
     let terminal_flag = Arc::new(Mutex::new(TerminalError::NoError));
     let terminal_flag_clone = Arc::clone(&terminal_flag);
        // Spawn a thread to capture stderr.
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let stderr_disp_clone = stderr_dispatcher.clone();
        let build_counter_stderr = Arc::clone(&build_progress_counter);
        let runtime_counter_stderr = Arc::clone(&runtime_progress_counter);
        let progress_disp_clone_stderr = progress_dispatcher.clone();
        let escape_sequence = "\u{1b}[1m\u{1b}[32m";  
        // let diagnostics_clone = Arc::clone(&diagnostics);
            let stderr_compiler_msg_clone = Arc::clone(&stderr_compiler_msg);
            println!("{}: Capturing stderr", pid);
        let stderr_handle = thread::spawn(move || {

            //    let mut msg_vec = stderr_compiler_msg_clone.lock().unwrap();
                       loop {
                        println!("looping stderr thread {}", pid);
            // Lock the deque and pop all messages available in a while loop
            while let Some(message) = {
                let mut guard = stderr_compiler_msg_clone.lock().unwrap();
                guard.pop_front()
            } {

                println!("compiler:{}: {}", pid, message);
                        for line in message.lines() {


                if let Some(ref disp) = stderr_disp_clone {
    // Dispatch the line and receive the Vec<Option<CallbackResponse>>.
    let responses = disp.dispatch(&line);

    // Iterate over the responses.
    for ret in responses {
        if let Some(response) = ret {
            if response.terminal_status == Some(TerminalError::NoTerminal) {
                // If the response indicates a terminal error, set the flag.
                println!("{} IS A TERMINAL PROCESS - {}", pid, line);
            } else if response.terminal_status == Some(TerminalError::NoError) {
                // If the response indicates no terminal error, set the flag to NoError.
            } else if response.terminal_status == Some(TerminalError::NoTerminal) {
                // If the response indicates not a terminal, set the flag to NoTerminal.
                println!("{} IS A TERMINAL PROCESS - {}", pid, line);
            }
            // if let Some(ref msg) = response.message {
            //     println!("DISPATCH RESULT {} {}", pid, msg);
            // }
                    //     let diag = crate::e_eventdispatcher::convert_response_to_diagnostic(response, &line);
                    //     // let mut diags = diagnostics_clone.lock().unwrap();

                    //     let in_multiline = disp.callbacks
                    //     .lock().unwrap()
                    //     .iter()
                    //     .any(|cb| cb.is_reading_multiline.load(Ordering::Relaxed));
                    
                    // if !in_multiline {
                    //     // start of a new diagnostic
                    //     diags.push(diag);
                    // } else {
                    //     // continuation → child of the last diagnostic
                    //     if let Some(parent) = diags.last_mut() {
                    //         parent.children.push(diag);
                    //     } else {
                    //         // no parent yet (unlikely), just push
                    //         diags.push(diag);
                    //     }
                    // }


        } else {
            println!("DISPATCH RESULT {} {}", pid, line);
        }
    }
}
                        }
                // Print each message to stderr
                log::trace!("rawcompiler[{}]", message);
            }
            // Sleep briefly if no messages are available to avoid busy waiting
            thread::sleep(Duration::from_millis(100));
            break;
        }

        println!("{}: dave stderr", pid);
            let mut flag = terminal_flag_clone.lock().unwrap();
            let stderr_reader = BufReader::new(stderr);
            for line in stderr_reader.lines() {
                if let Ok(line) = line {



                    // if line.contains("IO(Custom { kind: NotConnected") {
                    //     println!("{} IS A TERMINAL PROCESS - {}", pid,line);
                    //     continue;
                    // }
                    let line = if line.starts_with(escape_sequence) {
                        // If the line starts with the escape sequence, preserve it and remove leading spaces
                        let rest_of_line = &line[escape_sequence.len()..]; // Get the part of the line after the escape sequence
                        format!("{}{}", escape_sequence, rest_of_line.trim_start()) // Reassemble the escape sequence and the trimmed text
                    } else {
                        line // If it doesn't start with the escape sequence, leave it unchanged
                    };
                    println!("{}", line.trim());//all lines
if let Some(ref disp) = stderr_disp_clone {
    // Dispatch the line and receive the Vec<Option<CallbackResponse>>.
    let responses = disp.dispatch(&line);

    // Iterate over the responses.
    for ret in responses {
        if let Some(response) = ret {
            if response.terminal_status == Some(TerminalError::NoTerminal) {
                // If the response indicates a terminal error, set the flag.
                *flag = TerminalError::NoTerminal;
                println!("{} IS A TERMINAL PROCESS - {}", pid, line);
            } else if response.terminal_status == Some(TerminalError::NoError) {
                // If the response indicates no terminal error, set the flag to NoError.
                *flag = TerminalError::NoError;
            } else if response.terminal_status == Some(TerminalError::NoTerminal) {
                // If the response indicates not a terminal, set the flag to NoTerminal.
                 *flag = TerminalError::NoTerminal;
                println!("{} IS A TERMINAL PROCESS - {}", pid, line);
            }
            // if let Some(ref msg) = response.message {
            //     println!("DISPATCH RESULT {} {}", pid, msg);
            // }
                    //     let diag = crate::e_eventdispatcher::convert_response_to_diagnostic(response, &line);
                    //     // let mut diags = diagnostics_clone.lock().unwrap();

                    //     let in_multiline = disp.callbacks
                    //     .lock().unwrap()
                    //     .iter()
                    //     .any(|cb| cb.is_reading_multiline.load(Ordering::Relaxed));
                    
                    // if !in_multiline {
                    //     // start of a new diagnostic
                    //     diags.push(diag);
                    // } else {
                    //     // continuation → child of the last diagnostic
                    //     if let Some(parent) = diags.last_mut() {
                    //         parent.children.push(diag);
                    //     } else {
                    //         // no parent yet (unlikely), just push
                    //         diags.push(diag);
                    //     }
                    // }


        }
    }
}
                    // if let Some(ref disp) = stderr_disp_clone {
                    //     if let Some(ret) = disp.dispatch(&line) {
                    //         if let Some(ref msg) = ret.message {
                    //             println!("DISPATCH RESULT {} {}", pid, msg);
                    //         }
                    //     }
                    // }
                    // // Here, we assume stderr is less structured. We add its length to runtime counter.
                    runtime_counter_stderr.fetch_add(line.len(), Ordering::Relaxed);
                    if let Some(total) = estimate_bytes {
                        let current = runtime_counter_stderr.load(Ordering::Relaxed);
                        let progress = (current as f64 / total as f64) * 100.0;
                        if let Some(ref pd) = progress_disp_clone_stderr {
                            pd.dispatch(&format!("Progress: {:.2}%", progress));
                        }
                    }
                }
            }
        });

 
        let pid = child.id();
    let result = CargoProcessResult {
        pid: pid,
        exit_status: None,
        start_time: Some(start_time),
        build_finished_time: None,
        end_time: None,
        build_elapsed: None,
        runtime_elapsed: None,
        stats: CargoStats::default(),
        build_output_size: 0,
        runtime_output_size: 0,
        terminal_error: Some(tflag),
        diagnostics: Vec::new(),
    };
        CargoProcessHandle {
            child,
            result,
            pid,
            stdout_handle,
            stderr_handle,
            start_time,
            stats,
            stdout_dispatcher,
            stderr_dispatcher,
            progress_dispatcher,
            stage_dispatcher,
            estimate_bytes,
            build_progress_counter,
            runtime_progress_counter,
            requested_exit: false,
            terminal_error_flag: terminal_flag,
            diagnostics: diagnostics,
        }
    }
}
 