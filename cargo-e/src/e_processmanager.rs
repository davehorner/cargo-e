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

pub struct ProcessManager {
    signalled_count: AtomicUsize,
    signal_tx: Sender<()>,
    processes: Mutex<HashMap<u32, Arc<Mutex<CargoProcessHandle>>>>,
    // #[cfg(feature = "uses_async")]
    // notifier: Notify,
}

impl ProcessManager {
    pub fn new(_cli: &Cli) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Self {
            signalled_count: AtomicUsize::new(0),
            signal_tx: tx.clone(),
            processes: Mutex::new(HashMap::new()),
        });
        ProcessManager::install_handler(Arc::clone(&manager), rx);
        manager
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
                println!("ctrlc> Ctrl+C handler installed.");
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
        println!("ctrlc>");
        let mut processes = self.processes.lock().unwrap();
        for (pid, handle) in processes.iter() {
            println!("ctrlc> Terminating process with PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
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
                print!("{}", output);
            }
            #[cfg(not(feature = "tui"))]
            print!("\r{}", output);
        } else {
            // In non-raw mode, the \r trick can work.
            print!("\r{}", output);
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
                                // Remove the handle so it drops (and on Windows that will kill if still alive)
                                {
                                    let mut procs = self.processes.lock().unwrap();
                                    procs.remove(&pid);
                                }

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
                procs.remove(&pid);
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
    //                         eprintln!(
    //                             "Process {} still running after attempt {}.",
    //                             pid, attempts
    //                         );
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
            map.remove(&pid)
                .ok_or_else(|| anyhow::anyhow!("Process handle with PID {} not found", pid))?
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
                            print!("\r{}", status);
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
        Ok(handle.result)
    }

    //     pub fn wait(&self, pid: u32, _duration: Option<Duration>) -> anyhow::Result<CargoProcessResult> {
    //     // Hide the cursor and ensure it is restored on exit.
    //     {
    //         let mut stdout = std::io::stdout();
    //         crossterm::execute!(stdout, crossterm::cursor::Hide)?;
    //     }

    //     let mut processes = self.processes.lock().unwrap();
    //     if let Some(handle) = processes.get_mut(&pid) {
    //         let mut handle = handle.lock().unwrap();
    //         let mut system = System::new_all();

    //         // Initialize start_time if not already set.
    //         let start_time = handle.result.start_time.unwrap_or_else(|| {
    //             let now = SystemTime::now();
    //             handle.result.start_time = Some(now);
    //             now
    //         });

    //         // Main loop.
    //         const POLL_DURATION: Duration = Duration::from_secs(1);
    //         loop {
    //             system.refresh_all();
    //             let maybe_system: Option<&System> = if true { Some(&system) } else { None };
    //             // Get formatted status string.
    //             let output = handle.format_status(maybe_system);
    //             print!("\r{}", output);
    //             std::io::stdout().flush().unwrap();

    //             if let Some(status) = handle.child.try_wait()? {
    //                 handle.result.exit_status = Some(status);
    //                 handle.result.end_time = Some(SystemTime::now());
    //                 println!("\nProcess with PID {} finished", pid);
    //                 return Ok(handle.result.clone());
    //             }
    //             std::thread::sleep(POLL_DURATION);
    //         }
    //     } else {
    //         Err(anyhow::anyhow!("Process handle with PID {} not found", pid))
    //     }
    // }

    // pub fn wait(&self, pid: u32, _duration: Option<Duration>) -> anyhow::Result<CargoProcessResult> {
    //     // Turn off (hide) the cursor.
    //     {
    //         let mut stdout = std::io::stdout();
    //         crossterm::execute!(stdout, crossterm::cursor::Hide)?;
    //     }
    //     // Ensure the cursor is shown when we exit.
    //     let _cursor_guard = CursorGuard;

    //     let mut processes = self.processes.lock().unwrap();
    //     if let Some(handle) = processes.get_mut(&pid) {
    //         let mut handle = handle.lock().unwrap();
    //         let mut system = System::new_all();

    //         // Define the poll duration constant (adjust as needed).
    //         const POLL_DURATION: Duration = Duration::from_secs(1);

    //         // Initialize start_time if not already set.
    //         let start_time = handle.result.start_time.unwrap_or_else(|| {
    //             let now = SystemTime::now();
    //             handle.result.start_time = Some(now);
    //             now
    //         });
    //         // Format the start time with seconds precision.
    //         let start_dt: chrono::DateTime<Local> = start_time.into();
    //         let start_str = start_dt.format("%H:%M:%S").to_string();
    //         // Use ANSI color for the start time.
    //         let colored_start = nu_ansi_term::Color::Green.paint(&start_str).to_string();
    //         // Plain version for spacing calculations.
    //         let plain_start = start_str;

    //         loop {
    //             system.refresh_all();
    //             let now = SystemTime::now();
    //             let runtime_duration = now.duration_since(start_time).unwrap();
    //             let runtime_str = crate::e_fmt::format_duration(runtime_duration);

    //             // Use usize conversion with into()
    //             if let Some(process) = system.process((pid as usize).into()) {
    //                 let cpu_usage = process.cpu_usage();
    //                 let mem_kb = process.memory();
    //                 let mem_human = if mem_kb >= 1024 {
    //                     format!("{:.2} MB", mem_kb as f64 / 1024.0)
    //                 } else {
    //                     format!("{} KB", mem_kb)
    //                 };

    //                 let left_display = format!("{} | CPU: {:.2}% | Mem: {}", colored_start, cpu_usage, mem_human);
    //                 let left_plain = format!("{} | CPU: {:.2}% | Mem: {}", plain_start, cpu_usage, mem_human);

    //                 // Get terminal width with crossterm.
    //                 let (cols, _) = crossterm::terminal::size().unwrap_or((80, 20));
    //                 let total_width = cols as usize;
    //                 // Right side: the elapsed duration, underlined.
    //                 let right_display = nu_ansi_term::Style::new()
    //                     .reset_before_style()
    //                     .underline()
    //                     .paint(&runtime_str)
    //                     .to_string();
    //                 let left_len = left_plain.len();
    //                 let right_len = runtime_str.len();
    //                 let padding = if total_width > left_len + right_len {
    //                     total_width - left_len - right_len
    //                 } else {
    //                     1
    //                 };

    //                 let output = format!("\r{}{}{}", left_display, " ".repeat(padding), right_display);
    //                 print!("{}", output);
    //                 std::io::stdout().flush().unwrap();
    //             } else {
    //                 print!("\rProcess with PID {} not found in sysinfo", pid);
    //                 std::io::stdout().flush().unwrap();
    //             }

    //             if let Some(status) = handle.child.try_wait()? {
    //                 handle.result.exit_status = Some(status);
    //                 handle.result.end_time = Some(SystemTime::now());
    //                 println!("\nProcess with PID {} finished", pid);
    //                 return Ok(handle.result.clone());
    //             }

    //             std::thread::sleep(POLL_DURATION);
    //         }
    //     } else {
    //         Err(anyhow::anyhow!("Process handle with PID {} not found", pid))
    //     }
    // }

    // pub fn wait(&self, pid: u32, max_polls: Option<usize>) -> anyhow::Result<CargoProcessResult> {
    //     let mut processes = self.processes.lock().unwrap();
    //     if let Some(handle) = processes.get_mut(&pid) {
    //         let mut handle = handle.lock().unwrap();
    //         let mut system = System::new_all();

    //         // Initialize start_time if not already set.
    //         let start_time = handle.result.start_time.unwrap_or_else(|| {
    //             let now = SystemTime::now();
    //             handle.result.start_time = Some(now);
    //             now
    //         });
    //         // Format the start time with more precision.
    //         let start_dt: chrono::DateTime<Local> = start_time.into();
    //         let start_str = start_dt.format("%H:%M:%S").to_string();

    //         let mut polls = 0;
    //         loop {
    //             system.refresh_all();

    //             if let Some(process) = system.process((pid as usize).into()) {
    //                 let now = SystemTime::now();
    //                 let runtime = now.duration_since(start_time).unwrap();
    //                 let runtime_str = Self::format_duration(runtime);

    //                 // Get memory usage and convert to a human-readable string.
    //                 let mem_kb = process.memory();
    //                 let mem_human = if mem_kb >= 1024 {
    //                     format!("{:.2} MB", mem_kb as f64 / 1024.0)
    //                 } else {
    //                     format!("{} KB", mem_kb)
    //                 };

    //                 // Build the output string.
    //                 let output = format!(
    //                     "{} | Runtime: {} | Mem: {} | CPU: {:.2}%%",
    //                     start_str,
    //                     runtime_str,
    //                     mem_human,
    //                     process.cpu_usage()
    //                 );
    //                 // Print on one line and pad to clear leftover characters.
    //                 print!("\r{:<80}", output);
    //                 std::io::stdout().flush().unwrap();
    //             } else {
    //                 print!("\rProcess with PID {} not found in sysinfo", pid);
    //                 std::io::stdout().flush().unwrap();
    //             }

    //             // Check if the process has finished.
    //             if let Some(status) = handle.child.try_wait()? {
    //                 handle.result.exit_status = Some(status);
    //                 handle.result.end_time = Some(SystemTime::now());
    //                 println!("\nProcess with PID {} finished", pid);
    //                 return Ok(handle.result.clone());
    //             }

    //             polls += 1;
    //             if let Some(max) = max_polls {
    //                 if polls >= max {
    //                     println!("\nReached maximum polling iterations ({})", max);
    //                     break;
    //                 }
    //             }
    //             std::thread::sleep(Duration::from_secs(1));
    //         }
    //         Err(anyhow::anyhow!("Process did not finish after maximum polls"))
    //     } else {
    //         Err(anyhow::anyhow!("Process handle with PID {} not found", pid))
    //     }
    // }

    //     pub fn wait(&self, pid: u32) -> anyhow::Result<CargoProcessResult> {
    //         let mut processes = self.processes.lock().unwrap();
    //         if let Some(handle) = processes.get_mut(&pid) {
    //             let mut handle = handle.lock().unwrap();

    //             loop {
    //                 println!("Waiting for process with PID: {}", pid);

    //                 let status = handle.child.try_wait()?;

    //                 if let Some(status) = status {
    //                     handle.result.exit_status = Some(status);
    //                     handle.result.end_time = Some(SystemTime::now());
    //                     println!("Process with PID {} finished", pid);
    //                     return Ok(handle.result.clone());
    //                 }

    //                 std::thread::sleep(std::time::Duration::from_secs(1));
    //             }
    //         } else {
    //             Err(anyhow::anyhow!("Process handle with PID {} not found", pid))
    //         }
    // }

    pub fn format_process_status(
        pid: u32,
        start_time: Option<SystemTime>,
        system: Arc<Mutex<System>>,
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
        // Refresh the system stats and look up the process.
        if let Some(process) = system.lock().unwrap().process((pid as usize).into()) {
            let cpu_usage = process.cpu_usage();
            let mem_kb = process.memory();
            let mem_human = if mem_kb >= 1024 {
                format!("{:.2} MB", mem_kb as f64 / 1024.0)
            } else {
                format!("{} KB", mem_kb)
            };

            // Calculate runtime.
            let now = SystemTime::now();
            let runtime_duration = match start_time {
                Some(start) => now
                    .duration_since(start)
                    .unwrap_or_else(|_| Duration::from_secs(0)),
                None => Duration::from_secs(0),
            };
            let runtime_str = crate::e_fmt::format_duration(runtime_duration);
            // compute the max number of digits in either dimension:
            let max_digits = target_dimensions
                .0
                .max(target_dimensions.1)
                .to_string()
                .len();
            let left_display = format!(
                "{:0width$}of{:0width$} | {} | {} | PID: {} | CPU: {:.2}% | Mem: {}",
                target_dimensions.0,
                target_dimensions.1,
                nu_ansi_term::Color::Green
                    .paint(target.display_name.clone())
                    .to_string(),
                colored_start,
                pid,
                cpu_usage,
                mem_human,
                width = max_digits,
            );
            let left_plain = format!(
                "{:0width$}of{:0width$} | {} | {} | PID: {} | CPU: {:.2}% | Mem: {}",
                target_dimensions.0,
                target_dimensions.1,
                target.display_name,
                plain_start,
                pid,
                cpu_usage,
                mem_human,
                width = max_digits,
            );

            // Get terminal width.
            #[cfg(feature = "tui")]
            let (cols, _) = crossterm::terminal::size().unwrap_or((80, 20));
            #[cfg(not(feature = "tui"))]
            let (cols, _) = (80, 20);
            let total_width = cols as usize;

            // Format the runtime with underlining.
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

            format!("{}{}{}", left_display, " ".repeat(padding), right_display)
        } else {
            // String::from("xxx")
            format!("\rProcess with PID {} not found in sysinfo", pid)
        }
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

            // Determine width for zero-padding.
            let width = warnings.len().to_string().len().max(1);
            println!(
                "\n\n--- Warnings for PID {} --- {} {}",
                handle.0,
                width,
                warnings.len()
            );

            for (i, diag) in warnings.iter().enumerate() {
                // Format the index with leading zeros.
                let index = format!("{:0width$}", i + 1, width = width);
                // Print the warning with the index.
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
