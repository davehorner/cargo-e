use eframe::{egui, Frame};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winapi::um::winuser::GetForegroundWindow;
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            format!("Pool Manager Panic: {}", s)
        } else {
            "Pool Manager Panic: Unknown error".to_string()
        };

        #[cfg(target_os = "windows")]
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            let wide_message: Vec<u16> = OsStr::new(&message)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            winapi::um::winuser::MessageBoxW(
                std::ptr::null_mut(),
                wide_message.as_ptr(),
                OsStr::new("Error")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
                winapi::um::winuser::MB_ICONERROR | winapi::um::winuser::MB_OK,
            );
        }
    }));
}

#[derive(Clone)]
pub struct PoolManagerApp {
    pub pool_size: usize,
    pub rate_ms: u64,
    pub last_spawn: Arc<Mutex<Instant>>,
    pub spawned: Arc<Mutex<usize>>,
    pub children: Arc<Mutex<Vec<std::process::Child>>>,
    pub shutdown: Arc<std::sync::atomic::AtomicBool>, // Add this line
}

impl PoolManagerApp {
    pub fn new(pool_size: usize, rate_ms: u64) -> Self {
        setup_panic_hook();
        Self {
            pool_size,
            rate_ms,
            last_spawn: Arc::new(Mutex::new(Instant::now())),
            spawned: Arc::new(Mutex::new(0)),
            children: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)), // Initialize here
        }
    }
}

impl eframe::App for PoolManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        #[cfg(target_os = "windows")]
        {
            ensure_window_visible(unsafe { GetForegroundWindow() } as usize);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("e_window Pool Manager");
            ui.label(format!("Target pool size: {}", self.pool_size));
            ui.label(format!("Spawn rate: {} ms", self.rate_ms));
            let spawned = *self.spawned.lock().unwrap();
            ui.label(format!("Total windows spawned: {}", spawned));
            let last = *self.last_spawn.lock().unwrap();
            ui.label(format!("Last spawn: {:.1?} ago", last.elapsed()));
            ui.label("This window manages the pool and will keep at least N windows open.");
        });
        ctx.request_repaint_after(Duration::from_millis(500));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            let message = "Pool Manager is exiting.";
            let wide_message: Vec<u16> = OsStr::new(message)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            winapi::um::winuser::MessageBoxW(
                std::ptr::null_mut(),
                wide_message.as_ptr(),
                OsStr::new("Info")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect::<Vec<u16>>()
                    .as_ptr(),
                winapi::um::winuser::MB_OK,
            );
        }
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let handles: Vec<std::thread::JoinHandle<()>> = Vec::new();

        #[cfg(target_os = "windows")]
        use winapi::um::winuser::{PostMessageW, WM_QUIT};

        fn send_quit_to_hwnds_concurrently(hwnds: Vec<usize>) {
            use std::thread;
            hwnds.into_iter().for_each(|hwnd| {
                thread::spawn(move || unsafe {
                    PostMessageW(hwnd as _, WM_QUIT, 0, 0);
                });
            });
        }
        #[cfg(target_os = "windows")]
        {
            let hwnds: Vec<usize> = {
                let children = self.children.lock().unwrap();
                children
                    .iter()
                    .filter_map(|child| get_hwnd_for_child(child))
                    .collect()
            }; // Release the lock here

            send_quit_to_hwnds_concurrently(hwnds);
        }

        let mut handles = Vec::new();
        {
            let mut children = self.children.lock().unwrap();
            for mut child in children.drain(..) {
                let handle = std::thread::spawn(move || {
                    let _ = child.kill();
                });
                handles.push(handle);
            }
        } // Release the lock here
        for handle in handles {
            let _ = handle.join();
        }
    }
}

#[cfg(target_os = "windows")]
fn get_hwnd_for_child(child: &std::process::Child) -> Option<usize> {
    use std::cell::Cell;

    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};

    struct HwndSearch {
        target_pid: u32,
        found_hwnd: Cell<Option<HWND>>,
    }

    unsafe extern "system" fn enum_windows_proc(
        hwnd: HWND,
        lparam: winapi::shared::minwindef::LPARAM,
    ) -> i32 {
        let search = &*(lparam as *const HwndSearch);
        let mut pid = 0u32;
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == search.target_pid {
            search.found_hwnd.set(Some(hwnd));
            return 0; // Stop enumeration
        }
        1 // Continue enumeration
    }

    let search = HwndSearch {
        target_pid: child.id(),
        found_hwnd: Cell::new(None),
    };

    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &search as *const _ as winapi::shared::minwindef::LPARAM,
        );
    }

    search.found_hwnd.get().map(|hwnd| hwnd as usize)
}

#[cfg(target_os = "windows")]
use winapi::um::winuser::{SetForegroundWindow, ShowWindow, SW_RESTORE};

#[cfg(target_os = "windows")]
fn ensure_window_visible(hwnd: usize) {
    unsafe {
        // Restore the window if it is minimized or hidden
        ShowWindow(hwnd as _, SW_RESTORE);
        // Bring the window to the foreground
        SetForegroundWindow(hwnd as _);
    }
}
