
use crate::e_eventdispatcher::EventDispatcher;
use crate::e_runner::GLOBAL_CHILDREN;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{SystemTime, Duration};
use std::process::ExitStatus;
use cargo_metadata::Message;
use serde_json;
use tracing::instrument::WithSubscriber;
enum CaptureMode {
    Filtering(DispatcherSet),
    Passthrough { stdout: std::io::Stdout, stderr: std::io::Stderr },
}
struct DispatcherSet {
    stdout: Option<Arc<EventDispatcher>>,
    stderr: Option<Arc<EventDispatcher>>,
    progress: Option<Arc<EventDispatcher>>,
    stage: Option<Arc<EventDispatcher>>,
}

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
 
/// CargoProcessResult is returned when the cargo process completes.
#[derive(Debug,Default, Clone)]
pub struct CargoProcessResult {
    pub pid: u32,
    pub exit_status: Option<ExitStatus>,
    pub start_time: Option<SystemTime>,
    pub build_finished_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,
    pub build_elapsed: Option<Duration>,
    pub runtime_elapsed: Option<Duration>,
    pub stats: CargoStats,
    pub build_output_size: usize,
    pub runtime_output_size: usize,
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
}
 
impl CargoProcessHandle {

/// Helper: Format a Duration in a humanedable way.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    format!("{}.{:03} seconds", secs, millis)
}
 
/// Helper: Format a byte count in humanedable form.
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
 
pub fn print_results(result: &CargoProcessResult) {
    let start_time = result.start_time.unwrap_or(SystemTime::now());
    println!("-------------------------------------------------");
    println!("Process started at: {:?}", result.start_time);
    if let Some(build_time) = result.build_finished_time {
        println!("Build phase ended at: {:?}", build_time);
        println!("Build phase elapsed:  {}", Self::format_duration(build_time.duration_since(start_time).unwrap_or_else(|_| Duration::new(0, 0))));
    } else {
        println!("No BuildFinished timestamp recorded.");
    }
    println!("Process ended at:   {:?}", result.end_time);
    if let Some(runtime_dur) = result.runtime_elapsed {
        println!("Runtime phase elapsed: {}", Self::format_duration(runtime_dur));
    }
    if let Some(build_dur) = result.build_elapsed {
        println!("Build phase elapsed:   {}", Self::format_duration(build_dur));
    }
    if let Some(total_elapsed) = result.end_time.and_then(|end| end.duration_since(start_time).ok()) {
        println!("Total elapsed time:   {}", Self::format_duration(total_elapsed));
    } else {
        println!("No total elapsed time available.");
    }
    println!("Build output size:  {} ({} bytes)", Self::format_bytes(result.build_output_size), result.build_output_size);
    println!("Runtime output size: {} ({} bytes)", Self::format_bytes(result.runtime_output_size), result.runtime_output_size);
    println!("-------------------------------------------------");
}
 
    /// Kill the cargo process if needed.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }

        pub fn wait(&mut self) -> std::io::Result<CargoProcessResult> {
        // Lock the instance since `self` is an `Arc`
        // let mut cargo_process_handle = self.lock().unwrap();  // `lock()` returns a mutable reference

        // Call wait on the child process
        let status = self.child.wait()?;  // Call wait on the child process

        println!("Cargo process finished with status: {:?}", status);
        
        let end_time = SystemTime::now();

        // Retrieve the statistics from the process handle
        let stats = Arc::try_unwrap(self.stats.clone())
            .map(|mutex| mutex.into_inner().unwrap())
            .unwrap_or_else(|arc| (*arc.lock().unwrap()).clone());

        let build_out = self.build_progress_counter.load(Ordering::Relaxed);
        let runtime_out = self.runtime_progress_counter.load(Ordering::Relaxed);

        // Calculate phase durations if build_finished_time is recorded
        let (build_elapsed, runtime_elapsed) = if let Some(build_finished) = stats.build_finished_time {
            let build_dur = build_finished.duration_since(self.start_time)
                .unwrap_or_else(|_| Duration::new(0, 0));
            let runtime_dur = end_time.duration_since(build_finished)
                .unwrap_or_else(|_| Duration::new(0, 0));
            (Some(build_dur), Some(runtime_dur))
        } else {
            (None, None)
        };

        self.result.exit_status = Some(status);
        self.result.end_time = Some(end_time);
        self.result.build_output_size = self.build_progress_counter.load(Ordering::Relaxed);
        self.result.runtime_output_size = self.runtime_progress_counter.load(Ordering::Relaxed);

        Ok(self.result.clone())
        // Return the final process result
        // Ok(CargoProcessResult {
        //     pid: self.pid,
        //     exit_status: Some(status),
        //     start_time: Some(self.start_time),
        //     build_finished_time: stats.build_finished_time,
        //     end_time: Some(end_time),
        //     build_elapsed,
        //     runtime_elapsed,
        //     stats,
        //     build_output_size: build_out,
        //     runtime_output_size: runtime_out,
        // })
    }
 
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
        let mut child = self.spawn().expect("Failed to spawn cargo process");

        let pid = child.id();
        let start_time = SystemTime::now();

        let stats = Arc::new(Mutex::new(CargoStats::default()));

        // Create the CargoProcessHandle
        let result = CargoProcessResult {
            pid,
            exit_status: None,
            start_time: Some(start_time),
            build_finished_time: None,
            end_time: None,
            build_elapsed: None,
            runtime_elapsed: None,
            stats: CargoStats::default(),
            build_output_size: 0,
            runtime_output_size: 0,
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
        let mut capture_mode = CaptureMode::Filtering(DispatcherSet {
    stdout: stdout_dispatcher.clone(),
    stderr: stderr_dispatcher.clone(),
    progress: progress_dispatcher.clone(),
    stage: stage_dispatcher.clone(),
});
        self.stdout(Stdio::piped())
            .stderr(Stdio::piped());
 
        let mut child = self.spawn().expect("Failed to spawn cargo process");
        let pid= child.id();
        let start_time = SystemTime::now();
        let stats = Arc::new(Mutex::new(CargoStats::default()));
 
        // Two separate counters: one for build output and one for runtime output.
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
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stdout_handle = thread::spawn(move || {
            let stdout_reader = BufReader::new(stdout);
            // This flag marks whether we are still in the build phase.
            let mut in_build_phase = true;
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    // Try to parse the line as a JSON cargo message.
                    match serde_json::from_str::<Message>(&line) {
                        Ok(msg) => {
                            let msg_str = format!("{:?}", msg);
                            if let Some(ref disp) = stdout_disp_clone {
                                disp.dispatch(&msg_str);
                            }
                            // Add message length to the appropriate counter.
                            if in_build_phase {
                                build_counter_stdout.fetch_add(msg_str.len(), Ordering::Relaxed);
                            } else {
                                runtime_counter_stdout.fetch_add(msg_str.len(), Ordering::Relaxed);
                            }
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
                                            capture_mode = CaptureMode::Passthrough {
        stdout: std::io::stdout(),
        stderr: std::io::stderr(),
    };
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
                                Message::CompilerMessage(_) => {
                                    let mut s = stats_clone.lock().unwrap();
                                    s.compiler_message_count += 1;
                                    if s.compiler_message_time.is_none() {
                                        s.compiler_message_time = Some(now);
                                        if let Some(ref sd) = stage_disp_clone {
                                            sd.dispatch(&format!("Stage: CompilerMessage occurred at {:?}", now));
                                        }
                                    }
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
                            // Print the runtime output.
                            println!("{}RUNTIME: {}", pid, line);
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
        });
 
        // Spawn a thread to capture stderr.
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let stderr_disp_clone = stderr_dispatcher.clone();
        let build_counter_stderr = Arc::clone(&build_progress_counter);
        let runtime_counter_stderr = Arc::clone(&runtime_progress_counter);
        let progress_disp_clone_stderr = progress_dispatcher.clone();
        let escape_sequence = "\u{1b}[1m\u{1b}[32m";  
        let stderr_handle = thread::spawn(move || {
            let stderr_reader = BufReader::new(stderr);
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    if line.contains("IO(Custom { kind: NotConnected") {
                        println!("{} IS A TERMINAL PROCESS - {}", pid,line);
                        continue;
                    }
                                let line = if line.starts_with(escape_sequence) {
                // If the line starts with the escape sequence, preserve it and remove leading spaces
                let rest_of_line = &line[escape_sequence.len()..]; // Get the part of the line after the escape sequence
                format!("{}{}", escape_sequence, rest_of_line.trim_start()) // Reassemble the escape sequence and the trimmed text
            } else {
                line // If it doesn't start with the escape sequence, leave it unchanged
            };
                    println!("{}", line.trim());
                    if let Some(ref disp) = stderr_disp_clone {
                        disp.dispatch(&line);
                    }
                    // Here, we assume stderr is less structured. We add its length to runtime counter.
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
            requested_exit: false
        }
    }
}
 