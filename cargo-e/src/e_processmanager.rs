// src/e_process_manager.rs

use crate::e_cargocommand_ext::{CargoProcessHandle, CargoProcessResult};
use crate::e_command_builder::TerminalError;
use crate::e_target::CargoTarget;
use crate::{Cli, GLOBAL_MANAGER};
use chrono::Local;
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
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
#[cfg(unix)]
use {
    nix::sys::signal::{kill as nix_kill, Signal},
    nix::unistd::Pid,
    std::os::unix::process::ExitStatusExt,
};

impl ProcessObserver for ProcessManager {
    fn on_spawn(&self, pid: u32, handle: CargoProcessHandle) {
        self.processes.insert(pid, Arc::new(Mutex::new(handle)));
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
    processes: dashmap::DashMap<u32, Arc<Mutex<CargoProcessHandle>>>,
    results: dashmap::DashMap<u32, CargoProcessResult>,
    signal_times: SignalTimes, // <-- Add this line
}

impl Drop for ProcessManager {
    fn drop(&mut self) {}
}

impl std::fmt::Debug for ProcessManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let processes_len = self.processes.len();
        let results_len = self.results.len();
        let signalled_count = self.signalled_count.load(Ordering::SeqCst);
        f.debug_struct("ProcessManager")
            .field("signalled_count", &signalled_count)
            .field("signal_tx", &"Sender<()>")
            .field("processes.len", &processes_len)
            .field("results.len", &results_len)
            .finish()
    }
}

impl ProcessManager {
    pub fn new(_cli: &Cli) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Self {
            signalled_count: AtomicUsize::new(0),
            signal_tx: tx.clone(),
            processes: dashmap::DashMap::new(),
            results: dashmap::DashMap::new(),
            signal_times: SignalTimes::new(),
        });
        ProcessManager::install_handler(Arc::clone(&manager), rx);
        crate::GLOBAL_MANAGER.get_or_init(|| Arc::clone(&manager));
        crate::GLOBAL_EWINDOW_PIDS.get_or_init(|| dashmap::DashMap::new());
        manager
    }
    pub fn cleanup(&self) {
        // eprintln!("[ProcessManager::drop] Dropping ProcessManager and cleaning up processes.");
        // Try to kill all managed processes.
        for entry in self.processes.iter() {
            let pid = *entry.key();
            // eprintln!("[ProcessManager::drop] Attempting to kill PID {}", pid);
            if let Ok(mut handle) = entry.value().try_lock() {
                let _ = handle.kill();
            }
            // If you want to avoid locking, you would need to redesign CargoProcessHandle to allow lock-free signaling.
        }
        self.e_window_kill_all();
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
        let processes: Vec<_> = self
            .processes
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();
        for (pid, handle) in processes {
            println!("ctrlc> Terminating process with PID: {}", pid);
            if let Ok(mut h) = handle.try_lock() {
                let _ = h.kill();
                let final_diagnostics = {
                    let diag_lock = match h.diagnostics.try_lock() {
                        Ok(lock) => lock.clone(),
                        Err(e) => {
                            eprintln!("Failed to acquire diagnostics lock for PID {}: {}", pid, e);
                            Vec::new()
                        }
                    };
                    h.result.diagnostics = diag_lock.clone();

                    if let Some(exit_status) = h.child.try_wait().ok().flatten() {
                        h.result.exit_status = Some(exit_status);
                    }

                    h.result.end_time = Some(SystemTime::now());
                    if let (Some(start), Some(end)) = (h.result.start_time, h.result.end_time) {
                        h.result.elapsed_time = Some(end.duration_since(start).unwrap_or_default());
                    }
                    self.record_result(h.result.clone());
                    if let Some(manager) = GLOBAL_MANAGER.get() {
                        manager.e_window_kill_all();
                    }
                };
            }
            //self.processes.remove(&pid);
        }
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
        self.processes.insert(pid, Arc::new(Mutex::new(handle)));
        pid
    }

    pub fn take(&self, pid: u32) -> Option<Arc<Mutex<CargoProcessHandle>>> {
        // self.processes.remove(&pid).map(|(_, handle)| handle)
        self.processes.get(&pid).map(|entry| entry.clone())
    }

    pub fn remove(&self, pid: u32) {
        println!(
            "[ProcessManager::remove] Removing process with PID: {}",
            pid
        );
        if let Some(handle_arc) = self.processes.get(&pid) {
            match handle_arc.try_lock() {
                Ok(mut h) => {
                    h.removed = true;
                    let final_diagnostics = {
                        let diag_lock = match h.diagnostics.try_lock() {
                            Ok(lock) => lock.clone(),
                            Err(_) => Vec::new(),
                        };
                        diag_lock
                    };
                    h.result.diagnostics = final_diagnostics.clone();

                    if let Some(exit_status) = h.child.try_wait().ok().flatten() {
                        h.result.exit_status = Some(exit_status);
                    }

                    h.result.end_time = Some(SystemTime::now());
                    if let (Some(start), Some(end)) = (h.result.start_time, h.result.end_time) {
                        h.result.elapsed_time = Some(end.duration_since(start).unwrap_or_default());
                    }
                    h.result.pid = pid;
                    self.record_result(h.result.clone());
                    drop(h);
                }
                Err(e) => {
                    eprintln!("Failed to acquire lock for PID {}: {}", pid, e);
                }
            }
        }
    }

    pub fn try_wait(&self, pid: u32) -> anyhow::Result<Option<ExitStatus>> {
        // 1. Lock the processes map just long enough to clone the Arc.
        let handle_arc = {
            // Use DashMap's get method directly (no lock needed)
            self.processes
                .get(&pid)
                .map(|entry| entry.clone())
                .ok_or_else(|| anyhow::anyhow!("Process handle with PID {} not found", pid))?
        };

        // 2. Lock the individual process handle to perform try_wait.
        let mut handle = match handle_arc.try_lock() {
            Ok(h) => h,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to acquire process handle lock for PID {}: {}",
                    pid,
                    e
                ))
            }
        };
        // Here, try_wait returns a Result<Option<ExitStatus>, std::io::Error>.
        // The '?' operator will convert any std::io::Error to anyhow::Error automatically.
        let status = handle.child.try_wait()?;
        drop(handle);
        // Return the exit status (or None) wrapped in Ok.
        Ok(status)
    }

    pub fn get(&self, pid: u32) -> Option<Arc<Mutex<CargoProcessHandle>>> {
        self.processes.get(&pid).map(|entry| entry.clone())
    }

    pub fn is_alive(&self, pid: u32) -> bool {
        // Cross-platform check if a PID is still running
        #[cfg(unix)]
        {
            // On Unix, sending signal 0 checks if the process exists
            unsafe { libc::kill(pid as i32, 0) == 0 }
        }

        #[cfg(windows)]
        {
            // Use the windows crate to check if the process exists.

            unsafe {
                match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    Ok(handle) => {
                        if !handle.is_invalid() {
                            let _ = CloseHandle(handle);
                            return true;
                        } else {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
        }
        // if let Some(handle_arc) = self.processes.get(&pid) {
        //     let handle = handle_arc.lock().unwrap();
        //     !handle.removed
        // } else {
        //     false
        // }
    }

    pub fn list(&self) -> Vec<u32> {
        self.processes.iter().map(|entry| *entry.key()).collect()
    }

    pub fn status(&self) {
        if self.processes.is_empty() {
            println!("No active cargo processes.");
        } else {
            println!("Active processes:");
            for pid in self.processes.iter().map(|entry| *entry.key()) {
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
    pub fn kill_try_by_pid(&self, pid: u32) -> anyhow::Result<bool> {
        // Grab the handle, if any.
        let handle_opt = { self.processes.get(&pid).map(|entry| entry.clone()) };
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
                if let Ok(mut h) = handle.try_lock() {
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
                                // On Windows, try to kill child processes first before killing the main process.
                                let mut sys = System::new_all();
                                sys.refresh_all();
                                // let parent_pid = sysinfo::Pid::from(pid as usize);

                                // // Helper function to recursively collect all descendant PIDs
                                // fn collect_descendants(
                                //     sys: &System,
                                //     parent: sysinfo::Pid,
                                //     descendants: &mut Vec<sysinfo::Pid>,
                                // ) {
                                //     let children: Vec<_> = sys
                                //         .processes()
                                //         .values()
                                //         .filter(|p| p.parent() == Some(parent))
                                //         .map(|p| p.pid())
                                //         .collect();
                                //     for child_pid in &children {
                                //         descendants.push(*child_pid);
                                //         collect_descendants(sys, *child_pid, descendants);
                                //     }
                                // }

                                // let mut descendants = Vec::new();
                                // collect_descendants(&sys, parent_pid, &mut descendants);

                                // for &child_pid in &descendants {
                                //     eprintln!("Attempting to kill descendant PID {} of parent PID {}", child_pid, pid);
                                //     let _ = std::process::Command::new("taskkill")
                                //         .args(["/F", "/PID", &child_pid.to_string()])
                                //         .spawn();
                                //     self.e_window_kill(child_pid.as_u32());
                                // }
                                // Only attempt to kill if the child is still alive
                                match h.child.try_wait() {
                                    Ok(None) => {
                                        eprintln!("Attempt {}: killing PID {}", attempts + 1, pid);
                                        if let Err(e) = h.child.kill() {
                                            eprintln!("Failed to kill PID {}: {}", pid, e);
                                        }
                                        // Only call taskkill if the process is still running
                                        if h.child.try_wait()?.is_none() {
                                            _ = std::process::Command::new("taskkill")
                                                .args(["/F", "/PID", &pid.to_string()])
                                                .spawn();
                                        }
                                    }
                                    Ok(Some(status)) => {
                                        eprintln!(
                                            "PID {} already exited with status: {:?}",
                                            pid, status
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("Error checking status for PID {}: {}", pid, e);
                                    }
                                }
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
            }
            eprintln!(
                "Reference count for PID {} before lock: {}",
                pid,
                Arc::strong_count(&handle)
            );

            // println!("doing e_window cleanup for PID {}", pid);
            // // Handle global e_window_pids for this PID
            // if let Some(global) = crate::GLOBAL_EWINDOW_PIDS.get() {
            //     if let Some(e_window_pid_ref) = global.get(&pid) {
            //         let e_window_pid = *e_window_pid_ref.value();
            //         eprintln!(
            //             "[DEBUG] Killing global e_window PID {} for parent PID {}",
            //             e_window_pid, pid
            //         );
            //         let _ = std::process::Command::new("taskkill")
            //             .args(["/F", "/PID", &format!("{}", e_window_pid)])
            //             .spawn();
            //         // Drop the reference before removing to avoid blocking
            //         drop(e_window_pid_ref);
            //         global.remove(&pid);
            //     } else {
            //         eprintln!("[DEBUG] No global e_window PID found for parent PID {}", pid);
            //     }
            // }

            // Remove the handle so it drops (and on Windows that will kill if still alive)
            //                if let Some((_, handle_arc)) = self.processes.remove(&pid) {
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

    pub fn e_window_kill(&self, pid: u32) {
        // Ensure this function is only executed on the main thread
        // if !std::thread::current().name().map_or(false, |name| name == "main") {
        //     eprintln!("[DEBUG] Skipping e_window_kill for PID {} as it is not running on the main thread", pid);
        //     return;
        // }

        // Try to get the e_window PID for this process from GLOBAL_EWINDOW_PIDS
        if let Some(global) = crate::GLOBAL_EWINDOW_PIDS.get() {
            eprintln!(
                "[DEBUG] Searching for e_window PID for parent PID {} in map: {:?}",
                pid, global
            );

            // Extract the e_window_pid and drop the reference to avoid deadlocks
            if let Some(e_window_pid) = global.get(&pid).map(|entry| *entry.value()) {
                eprintln!(
                    "[DEBUG] Killing e_window PID {} for parent PID {}",
                    e_window_pid, pid
                );

                // Check if the process is still running before attempting to kill it
                let mut sys = sysinfo::System::new_all();
                sys.refresh_all();
                if sys
                    .process(sysinfo::Pid::from(e_window_pid as usize))
                    .is_some()
                {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/PID", &format!("{}", e_window_pid)])
                        .spawn();

                    eprintln!("[DEBUG] Successfully killed e_window PID {}", e_window_pid);
                } else {
                    eprintln!("[DEBUG] e_window PID {} is not running", e_window_pid);
                }

                // Remove the entry after handling the PID
                global.remove(&pid);
                eprintln!("[DEBUG] Removed e_window PID {} from map", e_window_pid);
            } else {
                eprintln!("[DEBUG] No e_window PID found for parent PID {}", pid);
            }
        } else {
            eprintln!("[DEBUG] GLOBAL_EWINDOW_PIDS is not initialized or empty");
        }
    }
    /// Kill all e_window processes tracked in GLOBAL_EWINDOW_PIDS.
    pub fn e_window_kill_all(&self) {
        if let Some(global) = crate::GLOBAL_EWINDOW_PIDS.get() {
            // Collect the PIDs first to avoid mutating while iterating
            let pids: Vec<u32> = global.iter().map(|entry| *entry.key()).collect();
            for pid in pids {
                self.e_window_kill(pid);
            }
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
        // Pick one process handle from DashMap (no lock needed).
        let maybe_entry = {
            self.processes
                .iter()
                .next()
                .map(|entry| (*entry.key(), entry.value().clone()))
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
            // {
            //     self.processes.remove(&pid);
            // }
            if process_exited {
                eprintln!("2Process {} removed from map after exit.", pid);
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
        let pids: Vec<u32> = self.processes.iter().map(|entry| *entry.key()).collect();
        println!("Killing all processes: {:?}", pids);
        self.e_window_kill_all();
        for pid in pids {
            println!("Killing PID: {}", pid);
            let _ = self.kill_by_pid(pid);
            //            if let Some((_, handle)) = self.processes.remove(&pid) {
            // if let Some(handle) = self.processes.get(&pid) {

            //     eprintln!("Killing PID: {}", pid);
            //     if let Ok(mut h) = handle.lock() {
            //         // Kill the main process
            //         let _ = h.kill();
            //     }
            // }
        }
    }

    // Returns the terminal error for a given PID.
    pub fn get_terminal_error(&self, pid: u32) -> Option<TerminalError> {
        // Lock the process map
        // Use DashMap's get method directly (no lock needed)
        if let Some(handle) = self.processes.get(&pid) {
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
            self.processes
                .get(&pid)
                .map(|entry| entry.clone())
                .ok_or_else(|| {
                    println!("Process with PID {} not found in the process map.", pid);
                    let result = CargoProcessResult {
                        target_name: String::new(), // Placeholder, should be set properly in actual use
                        cmd: String::new(), // Placeholder, should be set properly in actual use
                        args: Vec::new(),   // Placeholder, should be set properly in actual use
                        pid,
                        exit_status: None,
                        diagnostics: Vec::new(),
                        start_time: None,
                        end_time: Some(SystemTime::now()),
                        elapsed_time: None,
                        terminal_error: None, // Placeholder, should be set properly in actual use
                        build_finished_time: None, // Placeholder, should be set properly in actual use
                        build_elapsed: None, // Placeholder, should be set properly in actual use
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
        eprintln!(
            "Reference count for PID {} before try_unwrap: {}",
            pid,
            Arc::strong_count(&handle_arc)
        );
        // 2. Unwrap Arc<Mutex<...>> to get the handle
        let mut handle = match Arc::try_unwrap(handle_arc) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(h) => h,
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Failed to acquire process handle mutex for PID {}",
                        pid
                    ));
                }
            },
            Err(handle_arc_left) => {
                // If Arc::try_unwrap fails, we still have other references (e.g., in the DashMap).
                // Let's just lock and use the handle for this wait, but don't drop the Arc.
                let mut handle = handle_arc_left.lock().unwrap();
                // Set stats on the result before entering the polling loop
                let stats_clone = handle.stats.lock().unwrap().clone();
                handle.result.stats = stats_clone;

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
                            if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
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
                        if let (Some(start), Some(end)) =
                            (handle.result.start_time, handle.result.end_time)
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
                    let diagnostics = Arc::try_unwrap(handle.diagnostics.clone())
                        .map(|m| m.into_inner().unwrap())
                        .unwrap_or_else(|arc| arc.lock().unwrap().clone());

                    // 5. Move them into the final result
                    handle.result.diagnostics = diagnostics;
                }
                self.record_result(handle.result.clone());
                return Ok(handle.result.clone());
            }
        };

        // Set stats on the result before entering the polling loop
        handle.result.stats = handle.stats.lock().unwrap().clone();
        // 3. Interactive polling loop
        let mut system = if handle.is_filter {
            Some(System::new_all())
        } else {
            None
        };
        const POLL: Duration = Duration::from_secs(1);
        let mut loop_cnter = 0;
        let start_time = SystemTime::now();
        let timeout_duration = _duration.unwrap_or(Duration::from_secs(300)); // Default timeout of 5 minutes

        loop {
            loop_cnter += 1;
            if handle.is_filter && loop_cnter % 2 == 0 {
                if let Some(ref mut sys) = system {
                    sys.refresh_all();
                }
            }

            if handle.is_filter {
                if let Some(ref sys) = system {
                    if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
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

            // Check for timeout
            if start_time.elapsed().unwrap_or_default() > timeout_duration {
                eprintln!("\nProcess with PID {} timed out.", pid);
                handle.child.kill()?; // Terminate the process
                handle.result.exit_status = None;
                handle.result.end_time = Some(SystemTime::now());
                self.e_window_kill(pid);
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
        self.results.insert(result.pid, result);
    }

    pub fn generate_report(&self, create_gist: bool) {
        let results: Vec<_> = self
            .results
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
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
        for entry in self.processes.iter() {
            println!("--- Full Diagnostics for PID {} ---", entry.key());
            let handle_lock = entry.value().lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();
            for diag in diags.iter() {
                // Print the entire diagnostic.
                println!("{:?}: {}", diag.level, diag.message);
            }
        }
    }

    /// Print a one‑line summary per warning, numbered with leading zeros.
    pub fn print_prefixed_summary(&self) {
        // 1. Grab a snapshot of the handles (Arc clones) from DashMap.
        let handles: Vec<_> = self
            .processes
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // 2. Now we can iterate without holding any DashMap guard.
        for (pid, handle_arc) in handles {
            // Lock only the diagnostics for this handle.
            let handle_lock = handle_arc.lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();

            // Collect warnings.
            let warnings: Vec<_> = diags.iter().filter(|d| d.level.eq("warning")).collect();

            // Determine width for zero-padding for warnings.
            let warning_width = warnings.len().to_string().len().max(1);
            if warnings.len() > 0 {
                println!(
                    "\n\n--- Warnings for PID {} --- {} {}",
                    pid,
                    warning_width,
                    warnings.len()
                );

                for (i, diag) in warnings.iter().enumerate() {
                    // Format the index with leading zeros for warnings.
                    let index = format!("{:0width$}", i + 1, width = warning_width);
                    // Print the warning with the index.
                    println!("{}: {}", index, diag.message.trim());
                }
            }
            // Collect errors.
            let errors: Vec<_> = diags.iter().filter(|d| d.level.eq("error")).collect();
            if errors.len() > 0 {
                // Determine width for zero-padding for errors.
                let error_width = errors.len().to_string().len().max(1);
                println!(
                    "\n\n--- Errors for PID {} --- {} {}",
                    pid,
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
    }

    /// file:line:col – source_line, colored by level.
    pub fn print_compact(&self) {
        for entry in self.processes.iter() {
            println!("--- Compact for PID {} ---", entry.key());
            let handle_lock = entry.value().lock().unwrap();
            let diags = handle_lock.diagnostics.lock().unwrap();
            for diag in diags.iter() {
                println!("{}: {} {}", diag.level, diag.lineref, diag.message.trim());
            }
        }
    }
    /// Print a shortened version: warnings first then errors.
    pub fn print_shortened_output(&self) {
        for handle in self.processes.iter() {
            println!("\n\n\n--- Summary for PID {} ---", handle.key());
            let handle_lock = handle.value().lock().unwrap();
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

    pub fn kill_by_pid(&self, pid: u32) -> anyhow::Result<bool> {
        // Check if the process is alive
        if !self.is_alive(pid) {
            eprintln!("Process with PID {} is not running.", pid);
            return Ok(false);
        }

        #[cfg(unix)]
        {
            let signals = [
                Signal::SIGHUP,
                Signal::SIGINT,
                Signal::SIGQUIT,
                Signal::SIGABRT,
                Signal::SIGKILL,
                Signal::SIGALRM,
                Signal::SIGTERM,
            ];
            let mut killed = false;
            for (i, sig) in signals.iter().enumerate() {
                eprintln!("Attempt {}: sending {:?} to PID {}", i + 1, sig, pid);
                if let Err(e) = nix_kill(Pid::from_raw(pid as i32), *sig) {
                    eprintln!("Failed to send {:?} to PID {}: {}", sig, pid, e);
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !self.is_alive(pid) {
                    killed = true;
                    break;
                }
            }
            Ok(killed)
        }

        #[cfg(windows)]
        {
            eprintln!("Attempting to kill PID {} on Windows", pid);
            let output = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
            match output {
                Ok(out) => {
                    if out.status.success() {
                        // Give a moment for the process to terminate
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        Ok(!self.is_alive(pid))
                    } else {
                        eprintln!(
                            "taskkill failed for PID {}: {}",
                            pid,
                            String::from_utf8_lossy(&out.stderr)
                        );
                        Ok(false)
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute taskkill for PID {}: {}", pid, e);
                    Ok(false)
                }
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
