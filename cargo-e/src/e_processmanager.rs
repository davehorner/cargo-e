// src/e_process_manager.rs

use crate::e_cargocommand_ext::{CargoProcessHandle, CargoProcessResult};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use std::sync::atomic::Ordering;

impl ProcessObserver for ProcessManager {
    fn on_spawn(&self, pid: u32, handle: CargoProcessHandle) {
        self.processes.lock().unwrap().insert(pid, Arc::new(Mutex::new(handle)));
    }
        // let pid = handle.lock().unwrap().pid;
        // self.processes.lock().unwrap().insert(pid, handle);
        // Ok(())
}

#[cfg(feature = "uses_async")]
use tokio::sync::Notify;

// pub static PROCESS_MANAGER: Lazy<ProcessManager> = Lazy::new(ProcessManager::new);


pub trait ProcessObserver: Send + Sync + 'static {
    fn on_spawn(&self, pid: u32, handle: CargoProcessHandle);
}

pub struct ProcessManager {
    signalled_count: AtomicUsize,
    signal_tx: Sender<()>,
    processes: Mutex<HashMap<u32, Arc<Mutex<CargoProcessHandle>>>>,

    #[cfg(feature = "uses_async")]
    notifier: Notify,
}

impl ProcessManager {
    // pub fn new() -> Self {
    //     Self {
    //         processes: Mutex::new(HashMap::new()),

    //         #[cfg(feature = "uses_async")]
    //         notifier: Notify::new(),
    //     }
    // }

        pub fn new() -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Self {
            signalled_count: AtomicUsize::new(0),
            signal_tx: tx.clone(),
            processes: Mutex::new(HashMap::new()),
        });
        ProcessManager::install_handler(Arc::clone(&manager), rx);
        manager
    }

    pub fn has_signalled(&self) -> usize {
        self.signalled_count.load(Ordering::Relaxed)
    }

    fn install_handler(self_: Arc<Self>, rx: Receiver<()>) {
        ctrlc::set_handler({
            let tx = self_.signal_tx.clone();
            move || {
                let _ = tx.send(());
            }
        }).expect("Failed to install Ctrl+C handler");

        thread::spawn(move || {
            while rx.recv().is_ok() {
                self_.signalled_count.fetch_add(1, Ordering::Relaxed);
                println!("ctrlc> signal received.");
                self_.handle_signal();
            }
        });
    }

    fn handle_signal(&self) {
        let mut processes = self.processes.lock().unwrap();
        for (pid, handle) in processes.iter() {
            println!("ctrlc> Terminating process with PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
            }
        }
        processes.clear();
    }



    pub fn register(&self, handle: CargoProcessHandle) -> u32 {
        let pid = handle.pid;
        self.processes
            .lock()
            .unwrap()
            .insert(pid, Arc::new(Mutex::new(handle)));

        #[cfg(feature = "uses_async")]
        self.notifier.notify_waiters();

        pid
    }

    pub fn take(&self, pid: u32) -> Option<Arc<Mutex<CargoProcessHandle>>> {
        self.processes.lock().unwrap().remove(&pid)
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

    pub fn kill_one(&self) {
        let mut processes = self.processes.lock().unwrap();
        if let Some((&pid, handle)) = processes.iter().next() {
            eprintln!("Killing PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
            }
            processes.remove(&pid);
        } else {
            println!("No processes to kill.");
        }
    }

    pub fn kill_all(&self) {
        let mut processes = self.processes.lock().unwrap();
        for (pid, handle) in processes.drain() {
            eprintln!("Killing PID: {}", pid);
            if let Ok(mut h) = handle.lock() {
                let _ = h.kill();
            }
        }
    }

    pub fn install_ctrlc_handler(&'static self) {
        ctrlc::set_handler(move || {
            eprintln!("CTRL-C detected. Killing all processes.");
            self.kill_all();
            std::process::exit(1);
        })
        .expect("Failed to install ctrl-c handler");
    }

    pub fn wait(&self, pid: u32) -> anyhow::Result<CargoProcessResult> {
        let mut processes = self.processes.lock().unwrap();
        if let Some(handle) = processes.get_mut(&pid) {
            let mut handle = handle.lock().unwrap();

            loop {
                println!("Waiting for process with PID: {}", pid);

                let status = handle.child.try_wait()?;

                if let Some(status) = status {
                    handle.result.exit_status = Some(status);
                    handle.result.end_time = Some(SystemTime::now());
                    println!("Process with PID {} finished", pid);
                    return Ok(handle.result.clone());
                }

                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        } else {
            Err(anyhow::anyhow!("Process handle with PID {} not found", pid))
        }
}



}

#[cfg(feature = "uses_async")]
impl ProcessManager {
    pub async fn wait_for_processes(&self) {
        loop {
            {
                if self.processes.lock().unwrap().is_empty() {
                    break;
                }
            }
            self.notifier.notified().await;
        }
    }
}
