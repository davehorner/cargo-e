// src/e_process_manager.rs

use crate::e_cargocommand_ext::{CargoProcessHandle, CargoProcessResult};
use crate::e_command_builder::TerminalError;
use crate::e_target::CargoTarget;
use crate::Cli;
use chrono::Local;
use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::{Duration, SystemTime};
use sysinfo::System;
// use std::io::Write;
#[cfg(feature = "tui")]
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};
use std::sync::atomic::Ordering;
use std::sync::Mutex as StdMutex;
#[cfg(unix)]
use {
    nix::sys::signal::{kill as nix_kill, Signal},
    nix::unistd::Pid,
    std::os::unix::process::ExitStatusExt,
};

impl ProcessObserver for ProcessManager {
    fn on_spawn(&self, pid: u32, handle: CargoProcessHandle) {
        self.processes
            .lock()
            .unwrap()
            .insert(pid, Arc::new(Mutex::new(handle)));
    }
    // let pid = handle.lock().unwrap().pid;
    // self.processes.lock().unwrap().insert(pid, handle);
    // Ok(())
}

// #[cfg(feature = "uses_async")]
// use tokio::sync::Notify;

// pub static PROCESS_MANAGER: Lazy<ProcessManager> = Lazy::new(ProcessManager::new);

pub trait ProcessObserver: Send + Sync + 'static {
    fn on_spawn(&self, pid: u32, handle: CargoProcessHandle);
}
pub trait SignalTimeTracker {
    /// Returns the time when the last signal was received, if any.
    fn last_signal_time(&self) -> Option<SystemTime>;
    /// Returns the duration between the last two signals, if at least two signals were received.
    fn time_between_signals(&self) -> Option<Duration>;
}

pub struct SignalTimes {
    times: StdMutex<Vec<SystemTime>>,
}

impl SignalTimes {
    pub fn new() -> Self {
        Self {
            times: StdMutex::new(Vec::new()),
        }
    }
    pub fn record_signal(&self) {
        let mut times = self.times.lock().unwrap();
        times.push(SystemTime::now());
    }
}

impl SignalTimeTracker for SignalTimes {
    fn last_signal_time(&self) -> Option<SystemTime> {
        let times = self.times.lock().unwrap();
        times.last().cloned()
    }
    fn time_between_signals(&self) -> Option<Duration> {
        let times = self.times.lock().unwrap();
        if times.len() >= 2 {
            let last = times[times.len() - 1];
            let prev = times[times.len() - 2];
            last.duration_since(prev).ok()
        } else {
            None
        }
    }
}
#[derive()]
pub struct ProcessManager {
    signalled_count: AtomicUsize,
    signal_tx: Sender<()>,
    processes: Mutex<HashMap<u32, Arc<Mutex<CargoProcessHandle>>>>,
    results: Mutex<Vec<CargoProcessResult>>,
    signal_times: SignalTimes, // <-- Add this line
}

impl Drop for ProcessManager {
    fn drop(&mut self) {}
}

impl std::fmt::Debug for ProcessManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let processes_len = self.processes.lock().map(|p| p.len()).unwrap_or(0);
        let results_len = self.results.lock().map(|r| r.len()).unwrap_or(0);
        let signalled_count = self.signalled_count.load(Ordering::SeqCst);
        let signal_times = self.signal_times.times.lock().map(|v| v.len()).unwrap_or(0);
        f.debug_struct("ProcessManager")
            .field("signalled_count", &signalled_count)
            .field("signal_tx", &"Sender<()>")
            .field("processes.len", &processes_len)
            .field("results.len", &results_len)
            .field("signal_times.count", &signal_times)
            .finish()
    }
}

impl ProcessManager {
    pub fn new(_cli: &Cli) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Self {
            signalled_count: AtomicUsize::new(0),
            signal_tx: tx.clone(),
            processes: Mutex::new(HashMap::new()),
            results: Mutex::new(Vec::new()),
            signal_times: SignalTimes::new(), // <-- Add this line
        });
        ProcessManager::install_handler(Arc::clone(&manager), rx);
        crate::GLOBAL_MANAGER.get_or_init(|| Arc::clone(&manager));
        manager
    }

    pub fn last_signal_time(&self) -> Option<SystemTime> {
        self.signal_times.last_signal_time()
    }
    pub fn time_between_signals(&self) -> Option<Duration> {
        self.signal_times.time_between_signals()
    }
    pub fn reset_signalled(&self) {
        self.signalled_count.store(0, Ordering::SeqCst);
    }
    pub fn has_signalled(&self) -> usize {
        self.signalled_count.load(Ordering::SeqCst)
    }

    fn install_handler(self_: Arc<Self>, rx: Receiver<()>) {
        match ctrlc::set_handler({
            let tx = self_.signal_tx.clone();
            move || {
                let _ = tx.send(());
            }
        }) {
            Ok(_) => {
                thread::spawn(move || {
                    while rx.recv().is_ok() {
                        self_.signalled_count.fetch_add(1, Ordering::SeqCst);
                        println!(
                            "ctrlc> signal received.  {}",
                            self_.signalled_count.load(Ordering::SeqCst)
                        );
                        self_.handle_signal();
                    }
                });
            }
            Err(e) => {
                eprintln!("ctrlc> Failed to install Ctrl+C handler: {}", e);
                return;
            }
        }
    }

    fn handle_signal(&self) {
        self.signal_times.record_signal();
        println!("ctrlc>");
        let mut processes = self.processes.lock().unwrap();
        for (pid, handle) in processes.iter() {
            println!("ctrlc> Terminating process with PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
                let final_diagnostics = {
                    let diag_lock = h.diagnostics.lock().unwrap();
                    diag_lock.clone()
                };
                h.result.diagnostics = final_diagnostics.clone();

                if let Some(exit_status) = h.child.try_wait().ok().flatten() {
                    h.result.exit_status = Some(exit_status);
                }

                h.result.end_time = Some(SystemTime::now());
                if let (Some(start), Some(end)) = (h.result.start_time, h.result.end_time) {
                    h.result.elapsed_time = Some(end.duration_since(start).unwrap_or_default());
                }
                self.record_result(h.result.clone());
            }
        }
        processes.clear();
    }

    /// Updates the status line in the terminal.
    /// When `raw_mode` is true, it uses crossterm commands to clear the current line.
    /// Otherwise, it uses the carriage return (`\r`) trick.
    pub fn update_status_line(output: &str, raw_mode: bool) -> io::Result<()> {
        let mut stdout = io::stdout();
        if raw_mode {
            // Move cursor to beginning and clear the current line.
            #[cfg(feature = "tui")]
            {
                execute!(
                    stdout,
                    cursor::MoveToColumn(0),
                    Clear(ClearType::CurrentLine)
                )?;
                print!("\r{}\r", output);
            }
            #[cfg(not(feature = "tui"))]
            print!("\r{}\r", output);
        } else {
            // In non-raw mode, the \r trick can work.
            print!("\r{}\r", output);
        }
        stdout.flush()
    }

    pub fn register(&self, handle: CargoProcessHandle) -> u32 {
        let pid = handle.pid;
        self.processes
            .lock()
            .unwrap()
            .insert(pid, Arc::new(Mutex::new(handle)));

        // #[cfg(feature = "uses_async")]
        // self.notifier.notify_waiters();

        pid
    }

    pub fn take(&self, pid: u32) -> Option<Arc<Mutex<CargoProcessHandle>>> {
        self.processes.lock().unwrap().remove(&pid)
    }

    pub fn remove(&self, pid: u32) {
        if let Some(handle_arc) = self.processes.lock().unwrap().remove(&pid) {
            let mut h = handle_arc.lock().unwrap();
            let final_diagnostics = {
                let diag_lock = h.diagnostics.lock().unwrap();
                diag_lock.clone()
            };
            h.result.diagnostics = final_diagnostics.clone();

            // Ensure `es` is properly defined or assigned
            if let Some(exit_status) = h.child.try_wait().ok().flatten() {
                h.result.exit_status = Some(exit_status);
            }

            h.result.end_time = Some(SystemTime::now());
            if let (Some(start), Some(end)) = (h.result.start_time, h.result.end_time) {
                h.result.elapsed_time = Some(end.duration_since(start).unwrap_or_default());
            }
            self.record_result(h.result.clone());
            drop(h);
            // This was the only Arc reference, so dropping it here will run CargoProcessHandle::drop()
            drop(handle_arc);
        }
    }

    pub fn try_wait(&self, pid: u32) -> anyhow::Result<Option<ExitStatus>> {
        // 1. Lock the processes map just long enough to clone the Arc.
        let handle_arc = {
            let processes = self.processes.lock().unwrap();
            // Clone the Arc to keep the handle in the map while getting your own reference.
            processes
                .get(&pid)
                .ok_or_else(|| anyhow::anyhow!("Process handle with PID {} not found", pid))?
                .clone()
        };

        // 2. Lock the individual process handle to perform try_wait.
        let mut handle = handle_arc.lock().unwrap();
        // Here, try_wait returns a Result<Option<ExitStatus>, std::io::Error>.
        // The '?' operator will convert any std::io::Error to anyhow::Error automatically.
        let status = handle.child.try_wait()?;

        // Return the exit status (or None) wrapped in Ok.
        Ok(status)
    }

    pub fn get(&self, pid: u32) -> Option<Arc<Mutex<CargoProcessHandle>>> {
        self.processes.lock().unwrap().get(&pid).cloned()
    }

    pub fn list(&self) -> Vec<u32> {
        self.processes.lock().unwrap().keys().cloned().collect()
    }

    pub fn status(&self) {
        let processes = self.processes.lock().unwrap();
        if processes.is_empty() {
            println!("No active cargo processes.");
        } else {
            println!("Active processes:");
            for pid in processes.keys() {
                println!(" - PID: {}", pid);
            }
        }
    }

    // /// Attempts to kill the process corresponding to the provided PID.
    // /// Returns Ok(true) if the process was found and successfully killed (or had already exited),
    // /// Ok(false) if the process was not found or did not exit within the maximum attempts,
    // /// or an error if something went wrong.
    // pub fn kill_by_pid(&self, pid: u32) -> anyhow::Result<bool> {
    //     // Retrieve a clone of the handle (Arc) for the given PID without removing it.
    //     let handle = {
    //         let processes = self.processes.lock().unwrap();
    //         processes.get(&pid).cloned()
    //     };

    //     if let Some(handle) = handle {
    //         eprintln!("Attempting to kill PID: {}", pid);

    //         let max_attempts = 3;
    //         let mut attempts = 0;
    //         let mut process_exited = false;

    //         loop {
    //             // Lock the process handle for this iteration.
    //             if let Ok(mut h) = handle.lock() {
    //                 // Check if the process has already exited.
    //                 match h.child.try_wait() {
    //                     Ok(Some(status)) => {
    //                         eprintln!("Process {} already exited with status: {:?}", pid, status);
    //                         process_exited = true;
    //                         break;
    //                     }
    //                     Ok(None) => {
    //                         // Process is still running.
    //                         if attempts == 0 {
    //                             #[cfg(not(target_os = "windows"))] {
    //                                 eprintln!("Sending initial Ctrl+C signal to PID: {}", pid);
    //                                 crate::e_runall::send_ctrl_c(&mut h.child)?;
    //                             }
    //                             #[cfg(target_os = "windows")] {
    //                                 eprintln!("Sending initial kill signal to PID: {}", pid);
    //                             }
    //                         } else {
    //                             eprintln!("Attempt {}: Forcing kill for PID: {}", attempts, pid);
    //                         }
    //                         // Attempt to kill the process.
    //                         if let Err(e) = h.kill() {
    //                             eprintln!("Failed to send kill signal to PID {}: {}", pid, e);
    //                         }
    //                         // Mark that an exit was requested.
    //                         h.requested_exit = true;
    //                     }
    //                     Err(e) => {
    //                         eprintln!("Error checking exit status for PID {}: {}", pid, e);
    //                         break;
    //                     }
    //                 }
    //             } else {
    //                 eprintln!("Could not lock process handle for PID {}", pid);
    //                 break;
    //             }

    //             attempts += 1;
    //             // Allow some time for the process to exit.
    //             sleep(Duration::from_millis(2000));

    //             // Re-check if the process has exited.
    //             if let Ok(mut h) = handle.lock() {
    //                 match h.child.try_wait() {
    //                     Ok(Some(status)) => {
    //                         eprintln!("Process {} exited with status: {:?}", pid, status);
    //                         process_exited = true;
    //                         break;
    //                     }
    //                     Ok(None) => {
    //                         eprintln!("Process {} still running after attempt {}.", pid, attempts);
    //                     }
    //                     Err(e) => {
    //                         eprintln!("Error rechecking process {}: {}", pid, e);
    //                         break;
    //                     }
    //                 }
    //             }

    //             if attempts >= max_attempts {
    //                 eprintln!("Maximum kill attempts reached for PID {}.", pid);
    //                 break;
    //             }
    //         }

    //         // If the process exited, remove it from the map.
    //         if process_exited {
    //             let mut processes = self.processes.lock().unwrap();
    //             processes.remove(&pid);
    //             eprintln!("Process {} removed from map after exit.", pid);
    //         } else {
    //             eprintln!(
    //                 "Process {} did not exit after {} attempts; it remains in the map for future handling.",
    //                 pid, attempts
    //             );
    //         }
    //         Ok(process_exited)
    //     } else {
    //         eprintln!("Process handle with PID {} not found.", pid);
    //         Ok(false)
    //     }
    // }

    /// Attempts to kill the process corresponding to the provided PID.
    /// Returns Ok(true) if the process was found and exited (even via signal),
    /// Ok(false) if the process wasn’t found or didn’t exit after trying
    /// all signals (in which case we drop the handle), or Err if something went wrong.
    pub fn kill_by_pid(&self, pid: u32) -> anyhow::Result<bool> {
        // Grab the handle, if any.
        let handle_opt = {
            let procs = self.processes.lock().unwrap();
            procs.get(&pid).cloned()
        };

        if let Some(handle) = handle_opt {
            eprintln!("Attempting to kill PID: {}", pid);
            #[cfg(unix)]
            let signals = [
                Signal::SIGHUP,
                Signal::SIGINT,
                Signal::SIGQUIT,
                Signal::SIGABRT,
                Signal::SIGKILL,
                Signal::SIGALRM,
                Signal::SIGTERM,
            ];
            #[cfg(unix)]
            let max_attempts = signals.len();

            #[cfg(windows)]
            let max_attempts = 3; // arbitrary, since Child::kill() is always SIGKILL

            let mut attempts = 0;
            let mut did_exit = false;

            while attempts < max_attempts {
                // 1) Check status
                if let Ok(mut h) = handle.lock() {
                    match h.child.try_wait() {
                        Ok(Some(status)) => {
                            // Child has exited—report how.
                            #[cfg(unix)]
                            {
                                if let Some(sig) = status.signal() {
                                    eprintln!("Process {} terminated by signal {}", pid, sig);
                                } else if let Some(code) = status.code() {
                                    eprintln!("Process {} exited with code {}", pid, code);
                                } else {
                                    eprintln!(
                                        "Process {} exited with unknown status: {:?}",
                                        pid, status
                                    );
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                if let Some(code) = status.code() {
                                    eprintln!("Process {} exited with code {}", pid, code);
                                } else {
                                    eprintln!(
                                        "Process {} exited with unknown status: {:?}",
                                        pid, status
                                    );
                                }
                            }
                            did_exit = true;
                            break;
                        }
                        Ok(None) => {
                            // Still running → send the next signal
                            #[cfg(unix)]
                            {
                                let sig = signals[attempts];
                                eprintln!(
                                    "Attempt {}: sending {:?} to PID {}",
                                    attempts + 1,
                                    sig,
                                    pid
                                );
                                nix_kill(Pid::from_raw(pid as i32), sig)?;
                            }
                            #[cfg(windows)]
                            {
                                // // Remove the handle so it drops (and on Windows that will kill if still alive)
                                // {
                                //     let mut procs = self.processes.lock().unwrap();
                                //     procs.remove(&pid);
                                // }

                                eprintln!("Attempt {}: killing PID {}", attempts + 1, pid);
                                if let Err(e) = h.child.kill() {
                                    eprintln!("Failed to kill PID {}: {}", pid, e);
                                }
                                _ = std::process::Command::new("taskkill")
                                    .args(["/F", "/PID", &pid.to_string()])
                                    .spawn();
                            }
                            h.requested_exit = true;
                        }
                        Err(e) => {
                            eprintln!("Error checking status for PID {}: {}", pid, e);
                            break;
                        }
                    }
                } else {
                    eprintln!("Could not lock handle for PID {}", pid);
                    break;
                }

                attempts += 1;
                if did_exit {
                    break;
                }

                // Give it a moment before retrying
                thread::sleep(Duration::from_secs(2));
            }
            // Remove the handle so it drops (and on Windows that will kill if still alive)
            {
                let mut procs = self.processes.lock().unwrap();
                if let Some(handle_arc) = procs.remove(&pid) {
                    let mut handle = handle_arc.lock().unwrap();
                    let final_diagnostics = {
                        let diag_lock = handle.diagnostics.lock().unwrap();
                        diag_lock.clone()
                    };
                    handle.result.diagnostics = final_diagnostics.clone();

                    // Ensure `es` is properly defined or assigned
                    if let Some(exit_status) = handle.child.try_wait().ok().flatten() {
                        handle.result.exit_status = Some(exit_status);
                    }

                    handle.result.end_time = Some(SystemTime::now());
                    if let (Some(start), Some(end)) =
                        (handle.result.start_time, handle.result.end_time)
                    {
                        handle.result.elapsed_time =
                            Some(end.duration_since(start).unwrap_or_default());
                    }
                    self.record_result(handle.result.clone());
                } else {
                    eprintln!("No process found with PID: {}", pid);
                }
            }

            if did_exit {
                eprintln!("Process {} removed from map after exit.", pid);
            } else {
                eprintln!(
                    "Dropped handle for PID {} after {} attempts (process may still be running).",
                    pid, attempts
                );
            }

            Ok(did_exit)
        } else {
            eprintln!("Process handle with PID {} not found.", pid);
            Ok(false)
        }
    }

    //     /// Attempts to kill the process corresponding to the provided PID.
    // /// Returns Ok(true) if the process was found and successfully killed
    // /// (or had already exited), Ok(false) if the process was not found
    // /// or did not exit within the maximum attempts (in which case we drop
    // /// the handle), or an error if something went wrong.
    // pub fn kill_by_pid(&self, pid: u32) -> anyhow::Result<bool> {
    //     // Grab a clone of the Arc<Mutex<ProcessHandle>> if it exists
    //     let handle_opt = {
    //         let processes = self.processes.lock().unwrap();
    //         processes.get(&pid).cloned()
    //     };

    //     if let Some(handle) = handle_opt {
    //         eprintln!("Attempting to kill PID: {}", pid);

    //         let max_attempts = 3;
    //         let mut attempts = 0;
    //         let mut process_exited = false;

    //         loop {
    //             // 1) Check status / send signal
    //             if let Ok(mut h) = handle.lock() {
    //                 match h.child.try_wait() {
    //                     Ok(Some(status)) => {
    //                         // Already exited
    //                         eprintln!(
    //                             "Process {} already exited with status: {:?}",
    //                             pid, status
    //                         );
    //                         process_exited = true;
    //                         break;
    //                     }
    //                     Ok(None) => {
    //                         // Still running → send signal
    //                         if attempts == 0 {
    //                             #[cfg(not(target_os = "windows"))]
    //                             {
    //                                 eprintln!("Sending initial Ctrl+C to PID: {}", pid);
    //                                 crate::e_runall::send_ctrl_c(&mut h.child)?;
    //                             }
    //                             #[cfg(target_os = "windows")]
    //                             {
    //                                 eprintln!("Sending initial kill to PID: {}", pid);
    //                             }
    //                         } else {
    //                             eprintln!("Attempt {}: Forcing kill for PID: {}", attempts, pid);
    //                         }

    //                         if let Err(e) = h.kill() {
    //                             eprintln!("Failed to send kill to PID {}: {}", pid, e);
    //                         }
    //                         h.requested_exit = true;
    //                     }
    //                     Err(e) => {
    //                         eprintln!("Error checking status for PID {}: {}", pid, e);
    //                         break;
    //                     }
    //                 }
    //             } else {
    //                 eprintln!("Could not lock handle for PID {}", pid);
    //                 break;
    //             }

    //             attempts += 1;
    //             if attempts >= max_attempts {
    //                 eprintln!("Maximum kill attempts reached for PID {}. Dropping handle.", pid);
    //                 break;
    //             }

    //             // 2) Wait a bit before re-checking
    //             thread::sleep(Duration::from_millis(2_000));

    //             // 3) Re-check exit status
    //             if let Ok(mut h) = handle.lock() {
    //                 match h.child.try_wait() {
    //                     Ok(Some(status)) => {
    //                         eprintln!("Process {} exited with status: {:?}", pid, status);
    //                         process_exited = true;
    //                         break;
    //                     }
    //                     Ok(None) => {
    //                         eprintln!("Process {} still running after attempt {}.", pid, attempts);
    //                     }
    //                     Err(e) => {
    //                         eprintln!("Error rechecking process {}: {}", pid, e);
    //                         break;
    //                     }
    //                 }
    //             }
    //         }

    //         // Remove the handle (dropping it) whether or not the process exited
    //         {
    //             let mut processes = self.processes.lock().unwrap();
    //             processes.remove(&pid);
    //         }

    //         if process_exited {
    //             eprintln!("Process {} removed from map after exit.", pid);
    //         } else {
    //             eprintln!(
    //                 "Dropped handle for PID {} after {} attempts (process may still be running).",
    //                 pid, attempts
    //             );
    //         }

    //         Ok(process_exited)
    //     } else {
    //         eprintln!("Process handle with PID {} not found.", pid);
    //         Ok(false)
    //     }
    // }

    /// Attempts to kill one process.
    /// Returns Ok(true) if a process was found and killed, Ok(false) if none found,
    /// or an error if something went wrong.
    pub fn kill_one(&self) -> anyhow::Result<bool> {
        // First, lock the map briefly to pick one process handle.
        let maybe_entry = {
            let processes = self.processes.lock().unwrap();
            // Clone the Arc so that we don’t take ownership from the map.
            processes
                .iter()
                .next()
                .map(|(&pid, handle)| (pid, handle.clone()))
        };

        if let Some((pid, handle)) = maybe_entry {
            eprintln!("Attempting to kill PID: {}", pid);

            // We'll attempt to kill the process up to `max_attempts` times.
            let max_attempts = 3;
            let mut attempts = 0;
            let mut process_exited = false;

            loop {
                // Lock the process handle for this iteration.
                if let Ok(mut h) = handle.lock() {
                    // Check if the process has already exited.
                    match h.child.try_wait() {
                        Ok(Some(status)) => {
                            eprintln!("Process {} already exited with status: {:?}", pid, status);
                            process_exited = true;
                            sleep(Duration::from_millis(3_000));
                            break;
                        }
                        Ok(None) => {
                            // Process is still running. On the first attempt, or forcefully on later attempts.
                            if attempts == 0 {
                                eprintln!("Sending initial kill signal to PID: {}", pid);
                            } else {
                                eprintln!("Attempt {}: Forcing kill for PID: {}", attempts, pid);
                            }
                            sleep(Duration::from_millis(3_000));
                            // Try to kill the process. Handle errors by printing a debug message.
                            if let Err(e) = h.kill() {
                                eprintln!("Failed to send kill signal to PID {}: {}", pid, e);
                                sleep(Duration::from_millis(3_000));
                            }
                        }
                        Err(e) => {
                            eprintln!("Error checking exit status for PID {}: {}", pid, e);
                            sleep(Duration::from_millis(3_000));
                            break;
                        }
                    }
                } else {
                    eprintln!("Could not lock process handle for PID {}", pid);
                    sleep(Duration::from_millis(3_000));
                    break;
                }

                attempts += 1;
                // Allow some time for the process to exit.
                sleep(Duration::from_millis(3_000));

                // Check again after the sleep.
                if let Ok(mut h) = handle.lock() {
                    match h.child.try_wait() {
                        Ok(Some(status)) => {
                            eprintln!("Process {} exited with status: {:?}", pid, status);
                            sleep(Duration::from_millis(3_000));
                            process_exited = true;
                            break;
                        }
                        Ok(None) => {
                            eprintln!("Process {} still running after attempt {}.", pid, attempts);
                            sleep(Duration::from_millis(3_000));
                        }
                        Err(e) => {
                            eprintln!("Error rechecking process {}: {}", pid, e);
                            sleep(Duration::from_millis(3_000));
                            break;
                        }
                    }
                }

                if attempts >= max_attempts {
                    eprintln!("Maximum kill attempts reached for PID {}.", pid);
                    sleep(Duration::from_millis(3_000));
                    break;
                }
            }

            // 4) In all cases, remove the handle so it drops
            {
                let mut ps = self.processes.lock().unwrap();
                ps.remove(&pid);
            }
            if process_exited {
                eprintln!("Process {} removed from map after exit.", pid);
            } else {
                eprintln!(
                    "Dropped handle for PID {} after {} attempts (process may still be running).",
                    pid, attempts
                );
            }
            sleep(Duration::from_millis(3_000));
            Ok(process_exited)
        } else {
            println!("No processes to kill.");
            sleep(Duration::from_millis(3_000));
            Ok(false)
        }
    }
    // pub fn kill_one(&self) {
    //     let mut processes = self.processes.lock().unwrap();
    //     if let Some((&pid, handle)) = processes.iter().next() {
    //         eprintln!("Killing PID: {}", pid);
    //         if let Ok(mut h) = handle.lock() {
    //             let _ = h.kill();
    //         }
    //         processes.remove(&pid);
    //     } else {
    //         println!("No processes to kill.");
    //     }
    // }

    pub fn kill_all(&self) {
        let mut processes = self.processes.lock().unwrap();
        for (pid, handle) in processes.drain() {
            eprintln!("Killing PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
            }
        }
    }

    // Returns the terminal error for a given PID.
    pub fn get_terminal_error(&self, pid: u32) -> Option<TerminalError> {
        // Lock the process map
        let processes = self.processes.lock().unwrap();

        // Check if the process exists
        if let Some(handle) = processes.get(&pid) {
            // Lock the handle to access the terminal error flag
            let handle = handle.lock().unwrap();
            // Return the terminal error flag value
            return Some(handle.terminal_error_flag.lock().unwrap().clone());
        }

        // If no process is found for the given PID, return None
        None
    }

    //     pub fn install_ctrlc_handler(self: Arc<Self>) {
    //     let manager = Arc::clone(&self);
    //     ctrlc::set_handler(move || {
    //         eprintln!("CTRL-C detected. Killing all processes.");
    //         manager.kill_all();
    //         std::process::exit(1);
    //     })
    //     .expect("Failed to install ctrl-c handler");
    // }

    /// Wait for the process to finish, show interactive status, then return a result
    pub fn wait(
        &self,
        pid: u32,
        _duration: Option<Duration>,
    ) -> anyhow::Result<CargoProcessResult> {
        // Hide the cursor and ensure it’s restored on exit
        #[allow(dead_code)]
        struct CursorHide;
        impl Drop for CursorHide {
            fn drop(&mut self) {
                #[cfg(feature = "tui")]
                {
                    let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
                }
            }
        }
        #[cfg(feature = "tui")]
        let _cursor_hide = {
            let mut out = std::io::stdout();
            crossterm::execute!(out, crossterm::cursor::Hide)?;
            CursorHide
        };

        // 1. Remove the handle from the map
        let handle_arc = {
            let mut map = self.processes.lock().unwrap();
            map.remove(&pid).ok_or_else(|| {
                let result = CargoProcessResult {
                    target_name: String::new(), // Placeholder, should be set properly in actual use
                    cmd: String::new(),         // Placeholder, should be set properly in actual use
                    args: Vec::new(),           // Placeholder, should be set properly in actual use
                    pid,
                    exit_status: None,
                    diagnostics: Vec::new(),
                    start_time: None,
                    end_time: Some(SystemTime::now()),
                    elapsed_time: None,
                    terminal_error: None, // Placeholder, should be set properly in actual use
                    build_finished_time: None, // Placeholder, should be set properly in actual use
                    build_elapsed: None,  // Placeholder, should be set properly in actual use
                    runtime_elapsed: None, // Placeholder, should be set properly in actual use
                    stats: crate::e_cargocommand_ext::CargoStats::default(), // Provide a default instance of CargoStats
                    build_output_size: 0,        // Default value set to 0
                    runtime_output_size: 0, // Placeholder, should be set properly in actual use
                    is_filter: false,       // Placeholder, should be set properly in actual use
                    is_could_not_compile: false, // Placeholder, should be set properly in actual use
                };
                self.record_result(result.clone());
                anyhow::anyhow!("Process handle with PID {} not found", pid)
            })?
        };

        // 2. Unwrap Arc<Mutex<...>> to get the handle
        let mut handle = Arc::try_unwrap(handle_arc)
            .map_err(|_| anyhow::anyhow!("Process handle for PID {} still shared", pid))?
            .into_inner()
            .unwrap();

        // 3. Interactive polling loop
        let mut system = if handle.is_filter {
            Some(System::new_all())
        } else {
            None
        };
        const POLL: Duration = Duration::from_secs(1);
        let mut loop_cnter = 0;
        loop {
            loop_cnter += 1;
            if handle.is_filter && loop_cnter % 2 == 0 {
                if let Some(ref mut sys) = system {
                    sys.refresh_all();
                }
            }

            if handle.is_filter {
                if let Some(ref sys) = system {
                    if let Some(process) = sys.process((pid as usize).into()) {
                        let status = handle.format_status(Some(process));
                        if !status.is_empty() {
                            print!("\r{}\r", status);
                        }
                    }
                }
                std::io::stdout().flush().unwrap();
            }

            if let Some(es) = handle.child.try_wait()? {
                let final_diagnostics = {
                    let diag_lock = handle.diagnostics.lock().unwrap();
                    diag_lock.clone()
                };
                handle.result.diagnostics = final_diagnostics.clone();
                handle.result.exit_status = Some(es);
                handle.result.end_time = Some(SystemTime::now());
                if let (Some(start), Some(end)) = (handle.result.start_time, handle.result.end_time)
                {
                    handle.result.elapsed_time =
                        Some(end.duration_since(start).unwrap_or_default());
                }
                println!(
                    "\nProcess with PID {} finished {:?} {}",
                    pid,
                    es,
                    handle.result.diagnostics.len()
                );
                break;
            }
            std::thread::sleep(POLL);
        }

        if handle.is_filter {
            // 4. Extract diagnostics out of Arc<Mutex<_>>
            let diagnostics = Arc::try_unwrap(handle.diagnostics)
                .map(|m| m.into_inner().unwrap())
                .unwrap_or_else(|arc| arc.lock().unwrap().clone());

            // 5. Move them into the final result
            handle.result.diagnostics = diagnostics;
        }
        self.record_result(handle.result.clone());
        Ok(handle.result)
    }

    pub fn record_result(&self, result: CargoProcessResult) {
        let mut results = self.results.lock().unwrap();
        results.push(result);
    }

    pub fn generate_report(&self, create_gist: bool) {
        let results = self.results.lock().unwrap();
        let report = crate::e_reports::generate_markdown_report(&results);
        if let Err(e) = crate::e_reports::save_report_to_file(&report, "run_report.md") {
            eprintln!("Failed to save report: {}", e);
        }
        if create_gist {
            crate::e_reports::create_gist(&report, "run_report.md").unwrap_or_else(|e| {
                eprintln!("Failed to create Gist: {}", e);
            });
        }
    }

    pub fn format_process_status(
        pid: u32,
        start_time: Option<SystemTime>,
        // system: Arc<Mutex<System>>,
        target: &CargoTarget,
        target_dimensions: (usize, usize),
    ) -> String {
        // let start_dt: chrono::DateTime<Local> =
        //     start_time.unwrap_or_else(|| SystemTime::UNIX_EPOCH).into();
        let start_str = start_time
            .map(|st| {
                chrono::DateTime::<Local>::from(st)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        let colored_start = nu_ansi_term::Color::LightCyan.paint(&start_str).to_string();
        let plain_start = start_str;
        if start_time.is_none() {
            return String::new();
        }
        // Calculate runtime.
        let now = SystemTime::now();
        let runtime_duration = match start_time {
            Some(start) => now
                .duration_since(start)
                .unwrap_or_else(|_| Duration::from_secs(0)),
            None => Duration::from_secs(0),
        };
        if runtime_duration.as_secs() == 0 {
            return String::new();
        }
        let runtime_str = crate::e_fmt::format_duration(runtime_duration);
        // compute the max number of digits in either dimension:
        let max_digits = target_dimensions
            .0
            .max(target_dimensions.1)
            .to_string()
            .len();
        let left_display = format!(
            "{:0width$}of{:0width$} | {} | {} | PID: {}",
            target_dimensions.0,
            target_dimensions.1,
            nu_ansi_term::Color::Green
                .paint(target.display_name.clone())
                .to_string(),
            colored_start,
            pid,
            width = max_digits,
        );
        let left_plain = format!(
            "{:0width$}of{:0width$} | {} | {} | PID: {}",
            target_dimensions.0,
            target_dimensions.1,
            target.display_name,
            plain_start,
            pid,
            width = max_digits,
        );

        // Get terminal width.
        #[cfg(feature = "tui")]
        let (cols, _) = crossterm::terminal::size().unwrap_or((80, 20));
        #[cfg(not(feature = "tui"))]
        let (cols, _) = (80, 20);
        let mut total_width = cols as usize;
        total_width = total_width - 1;
        // Format the runtime with underlining.
        let right_display = nu_ansi_term::Style::new()
            .reset_before_style()
            .underline()
            .paint(&runtime_str)
            .to_string();
        let left_len = left_plain.len();
        let right_len = runtime_str.len();
        let visible_right_len = runtime_str.len();
        let padding = if total_width > left_len + visible_right_len {
            total_width.saturating_sub(left_len + visible_right_len)
        } else {
            0
        };

        let ret = format!("{}{}{}", left_display, " ".repeat(padding), right_display);
        if left_len + visible_right_len > total_width {
            let truncated_left = &left_display[..total_width.saturating_sub(visible_right_len)];
            return format!("{}{}", truncated_left.trim_end(), right_display);
        }
        ret
    }

    /// Print the exact diagnostic output as captured.
    pub fn print_exact_output(&self) {
        let processes = self.processes.lock().unwrap();
        for handle in processes.iter() {
            println!("--- Full Diagnostics for PID {} ---", handle.0);
            let handle_lock = handle.1.lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();
            for diag in diags.iter() {
                // Print the entire diagnostic.
                println!("{:?}: {}", diag.level, diag.message);
            }
        }
    }

    /// Print a one‑line summary per warning, numbered with leading zeros.
    pub fn print_prefixed_summary(&self) {
        // 1. Grab a snapshot of the handles (Arc clones) under the manager lock.
        let guard = self.processes.lock().unwrap();
        let handles: Vec<_> = guard.iter().map(|h| h.clone()).collect();

        // 2. Now we can iterate without holding the manager lock.
        for handle in handles {
            // Lock only the diagnostics for this handle.
            let handle_lock = handle.1.lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();

            // Collect warnings.
            let warnings: Vec<_> = diags.iter().filter(|d| d.level.eq("warning")).collect();

            // Determine width for zero-padding for warnings.
            let warning_width = warnings.len().to_string().len().max(1);
            println!(
                "\n\n--- Warnings for PID {} --- {} {}",
                handle.0,
                warning_width,
                warnings.len()
            );

            for (i, diag) in warnings.iter().enumerate() {
                // Format the index with leading zeros for warnings.
                let index = format!("{:0width$}", i + 1, width = warning_width);
                // Print the warning with the index.
                println!("{}: {}", index, diag.message.trim());
            }

            // Collect errors.
            let errors: Vec<_> = diags.iter().filter(|d| d.level.eq("error")).collect();

            // Determine width for zero-padding for errors.
            let error_width = errors.len().to_string().len().max(1);
            println!(
                "\n\n--- Errors for PID {} --- {} {}",
                handle.0,
                error_width,
                errors.len()
            );

            for (i, diag) in errors.iter().enumerate() {
                // Format the index with leading zeros for errors.
                let index = format!("{:0width$}", i + 1, width = error_width);
                // Print the error with the index.
                println!("{}: {}", index, diag.message.trim());
            }
        }
    }

    /// file:line:col – source_line, colored by level.
    pub fn print_compact(&self) {
        let processes = self.processes.lock().unwrap();
        for handle in processes.iter() {
            println!("--- Compact for PID {} ---", handle.0);
            let handle_lock = handle.1.lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();
            for diag in diags.iter() {
                println!("{}: {} {}", diag.level, diag.lineref, diag.message.trim());
            }
        }
    }
    /// Print a shortened version: warnings first then errors.
    pub fn print_shortened_output(&self) {
        let processes = self.processes.lock().unwrap();
        for handle in processes.iter() {
            println!("\n\n\n--- Summary for PID {} ---", handle.0);
            let handle_lock = handle.1.lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();

            // Filter diagnostics for warnings and errors.
            let warnings: Vec<_> = diags.iter().filter(|d| d.level.eq("warning")).collect();
            let errors: Vec<_> = diags.iter().filter(|d| d.level.eq("error")).collect();

            // Print warnings.
            if !warnings.is_empty() {
                println!("Warnings:");
                for diag in warnings {
                    println!("print_shortened_output:{}", diag.message.trim());
                }
            } else {
                println!("No warnings.");
            }

            // Print errors.
            if !errors.is_empty() {
                println!("Errors:");
                for diag in errors {
                    println!("print_shortened_output: {}", diag.message.trim());
                }
            } else {
                println!("No errors.");
            }
        }
    }
}

// #[cfg(feature = "uses_async")]
// impl ProcessManager {
//     pub async fn wait_for_processes(&self) {
//         loop {
//             {
//                 if self.processes.lock().unwrap().is_empty() {
//                     break;
//                 }
//             }
//             self.notifier.notified().await;
//         }
//     }
// }

pub struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        #[cfg(feature = "tui")]
        {
            let mut stdout = std::io::stdout();
            let _ = crossterm::execute!(stdout, crossterm::cursor::Show);
        }
    }
}
