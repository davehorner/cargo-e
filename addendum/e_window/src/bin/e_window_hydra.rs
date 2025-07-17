use winit::raw_window_handle::HasWindowHandle;
// Helper to convert Win32WindowHandle to HWND
#[cfg(target_os = "windows")]
fn handle_to_hwnd(
    handle: winit::raw_window_handle::Win32WindowHandle,
) -> windows::Win32::Foundation::HWND {
    let hwnd_isize: isize = handle.hwnd.into();
    let hwnd = hwnd_isize as *mut core::ffi::c_void;
    windows::Win32::Foundation::HWND(hwnd)
}

// A minimal demo that spawns a Chrome window with a random Hydra sketch, pins it to the fill grid, and kills Chrome when the window exits.
extern crate e_window;
use e_window::position_grid::PositionGrid;
use e_window::position_grid_manager::PositionGridManager;
use eframe::egui;
use std::io::BufRead;
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "windows")]
#[derive(Debug, Default, Clone)]
struct ChromeWindowInfo {
    hwnd: Option<u32>,
    pid: Option<u32>,
    title: Option<String>,
    target: Option<String>,
    page_url: Option<String>,
}

#[cfg(target_os = "windows")]
pub struct HydraDemoApp {
    script_tempfile: Option<tempfile::NamedTempFile>,
    chrome_launch_request_tx: Option<std::sync::mpsc::Sender<(i32, i32, i32, i32)>>,
    window_info: Arc<Mutex<ChromeWindowInfo>>,
    chrome_pid: Option<u32>,
    last_pinned_rect: Option<(i32, i32, i32, i32)>,
    eframe_hwnd: Option<u32>,
    chrome_spawned: Arc<std::sync::atomic::AtomicBool>,
    initial_chrome_rect: Option<(i32, i32, i32, i32)>,
    fill_grid: PositionGrid,
    grid_manager: PositionGridManager,
    chrome_output_rx: Option<Receiver<String>>,
    pin_request_rx: Option<Receiver<(u32, (i32, i32, i32, i32))>>,
    click_request_rx: Option<Receiver<(usize, usize)>>,
}

#[cfg(target_os = "windows")]
impl HydraDemoApp {
    pub fn with_hwnd(hwnd: Option<u32>) -> Self {
        let dummy_grid = PositionGrid::default();
        let window_info = Arc::new(Mutex::new(ChromeWindowInfo::default()));
        let (chrome_output_tx, chrome_output_rx) = mpsc::channel();
        let (pin_request_tx, pin_request_rx) = mpsc::channel();
        let (chrome_launch_request_tx, chrome_launch_request_rx) = mpsc::channel();
        let (click_request_tx, click_request_rx) = mpsc::channel();
        // Static JS script to inject into Chrome
        const INJECT_SCRIPT: &str = r#"
document.getElementById('close-icon').click();
//document.getElementById('modal').remove();
// Always attempt to click to close at the end
setInterval(() => {
  const el = document.getElementById('close-icon');
  if (el) {
    el.click();
    // Optionally remove the modal as well:
    const modal = document.getElementById('modal');
    if (modal) modal.remove();
  }
}, 16);
"#;

        let script_tempfile = {
            use std::io::Write;
            use tempfile::NamedTempFile;
            let mut file = NamedTempFile::new().expect("Failed to create temp script file");
            file.write_all(INJECT_SCRIPT.as_bytes())
                .expect("Failed to write script file");
            file
        };

        {
            let window_info_clone = window_info.clone();
            let chrome_output_tx_clone = chrome_output_tx.clone();
            let script_file_path = script_tempfile.path().to_path_buf();
            thread::spawn(move || {
                // Wait for grid coordinates from UI thread
                let click_request_tx_clone = click_request_tx.clone();
                let (x, y, w, h) = chrome_launch_request_rx.recv().unwrap_or((0, 0, 800, 600));
                let hydra_url = format!("debugchrome:https://hydra.ojack.xyz/?sketch_id=example&!id=&!openwindow&!x={}&!y={}&!w={}&!h={}", x, y, w, h);
                println!("[HydraDemo] Spawning debugchrome with URL: {}", hydra_url);
                let chrome = Command::new("debugchrome")
                    .arg(&hydra_url)
                    .arg("--script-file")
                    .arg(script_file_path.display().to_string())
                    .stdout(std::process::Stdio::piped())
                    .spawn();
                match chrome {
                    Ok(mut child) => {
                        let pid = child.id();
                        window_info_clone.lock().unwrap().pid = Some(pid);
                        let tx = chrome_output_tx_clone;
                        let window_info_inner = window_info_clone.clone();
                        thread::spawn(move || {
                            let mut info = ChromeWindowInfo::default();
                            let click_tx = click_request_tx_clone;
                            if let Some(stdout) = child.stdout.take() {
                                use std::io::BufReader;
                                let reader = BufReader::new(stdout);
                                for line in reader.lines().flatten() {
                                    let _ = tx.send(format!("[debugchrome stdout] {}", line));
                                    if let Some(hwnd_hex) = line.strip_prefix("HWND: 0x") {
                                        if let Ok(hwnd_val) =
                                            u32::from_str_radix(hwnd_hex.trim(), 16)
                                        {
                                            info.hwnd = Some(hwnd_val);
                                            // Send a click request to cell (15,10) when HWND is parsed
                                            let _ = click_tx.send((15, 10));
                                        }
                                    }
                                    if let Some(pid_str) = line.strip_prefix("PID: ") {
                                        if let Ok(pid_val) = pid_str.trim().parse::<u32>() {
                                            info.pid = Some(pid_val);
                                        }
                                    }
                                    if let Some(title) = line.strip_prefix("TITLE: ") {
                                        info.title = Some(title.trim().to_string());
                                    }
                                    if let Some(target) = line.strip_prefix("TARGET: ") {
                                        info.target = Some(target.trim().to_string());
                                    }
                                    if let Some(page_url) = line.strip_prefix("PAGE_URL: ") {
                                        info.page_url = Some(page_url.trim().to_string());
                                    }
                                    if info.hwnd.is_some() && info.pid.is_some() {
                                        let mut win_lock = window_info_inner.lock().unwrap();
                                        *win_lock = info.clone();
                                    }
                                }
                            } else {
                                let _ = tx.send(
                                    "[HydraDemo] No stdout from debugchrome child process"
                                        .to_string(),
                                );
                            }
                        });
                    }
                    Err(e) => {
                        let _ = chrome_output_tx_clone
                            .send(format!("[HydraDemo] Failed to launch debugchrome: {}", e));
                    }
                }
            });
        }
        let mut grid_manager = PositionGridManager::new();
        if let Some(hwnd) = hwnd {
            grid_manager.host_hwnd = Some(hwnd);
        }
        // Store click_request_rx in struct for main thread polling
        Self {
            script_tempfile: Some(script_tempfile),
            window_info,
            chrome_pid: None,
            last_pinned_rect: None,
            eframe_hwnd: hwnd,
            chrome_spawned: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            initial_chrome_rect: None,
            fill_grid: dummy_grid,
            grid_manager,
            chrome_output_rx: Some(chrome_output_rx),
            pin_request_rx: Some(pin_request_rx),
            chrome_launch_request_tx: Some(chrome_launch_request_tx),
            click_request_rx: Some(click_request_rx),
        }
    }
}

#[cfg(target_os = "windows")]
impl Default for HydraDemoApp {
    fn default() -> Self {
        HydraDemoApp::with_hwnd(None)
    }
}

#[cfg(target_os = "windows")]
impl Drop for HydraDemoApp {
    fn drop(&mut self) {
        // Send WM_CLOSE to pinned_hwnd if set
        let hwnd_opt = self.window_info.lock().unwrap().hwnd.clone();
        if let Some(hwnd_val) = hwnd_opt {
            let hwnd = hwnd_val as HWND;
            unsafe {
                use winapi::um::winuser::PostMessageW;
                use winapi::um::winuser::WM_CLOSE;
                let result = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                println!(
                    "[HydraDemo] Sent WM_CLOSE to pinned HWND: 0x{:X}, result={}",
                    hwnd_val, result
                );
            }
        }
        if let Some(pid) = self.chrome_pid.take() {
            println!("[HydraDemo] Killing Chrome PID: {}", pid);
            let _ = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status();
        }
    }
}

#[cfg(target_os = "windows")]
impl eframe::App for HydraDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            // --- Ensure eframe_hwnd is set ---

            // --- Always set grid and host HWND before diagnostics ---
            let label_height = 32.0;
            let available = ui.available_size();
            let grid_area = egui::vec2(available.x, (available.y - label_height).max(32.0));
            let (mut new_grid, _char_size) = PositionGrid::from_text_style(self.eframe_hwnd,ui, egui::TextStyle::Heading, egui::Color32::LIGHT_GREEN, None);
            new_grid.rect = ui.allocate_exact_size(grid_area, egui::Sense::hover()).0;
            self.fill_grid = new_grid;
            {
                self.grid_manager.grid = Some(&self.fill_grid as *const PositionGrid);
                // Send grid coordinates to Chrome launch thread if not launched yet
                if let Some(tx) = &self.chrome_launch_request_tx {
                    // Only send once: if channel is empty and Chrome not launched
                    if self.chrome_spawned.load(std::sync::atomic::Ordering::SeqCst) == false {
                        if let Some((grid_x, grid_y, grid_w, grid_h)) = self.grid_manager.get_grid_screen_rect() {
                            // Use grid size and position directly for Chrome window
                            let x = grid_x;
                            let y = grid_y;
                            let w = grid_w;
                            let h = grid_h;
                            let _ = tx.send((x, y, w, h));
                            self.chrome_spawned.store(true, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                }
                if let Some(rx) = &self.click_request_rx {
                    for (cell_x, cell_y) in rx.try_iter() {
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        let result = self.fill_grid.send_mouse_click_to_cell(cell_x, cell_y);
                        println!(
                            "[HydraDemo] Mouse click requested at cell ({}, {}) for HWND=0x{:?}, result={:?}",
                            cell_x, cell_y, self.eframe_hwnd, result
                        );
                        unsafe {
                            use winapi::shared::windef::POINT;
                            use winapi::um::winuser::GetCursorPos;
                            let mut pt: POINT = std::mem::zeroed();
                            if GetCursorPos(&mut pt) != 0 {
                                println!("[HydraDemo] Mouse pointer is now at: x={}, y={}", pt.x, pt.y);
                            } else {
                                println!("[HydraDemo] Failed to get mouse pointer position");
                            }
                        }
                    }
                }
            }

            // --- TOP DIAGNOSTICS (single line, only once per frame) ---
            let grid_rect = self.grid_manager.get_grid_screen_rect().unwrap_or((0, 0, 0, 0));
            let host_rect = self.grid_manager.get_host_screen_rect().unwrap_or((0, 0, 0, 0));
            ui.label(format!(
                "Grid: cells={} | grid_rect=({}, {}, {}, {}) | host_rect=({}, {}, {}, {}) | host_hwnd={:?}",
                self.fill_grid.cell_count(),
                grid_rect.0, grid_rect.1, grid_rect.2, grid_rect.3,
                host_rect.0, host_rect.1, host_rect.2, host_rect.3,
                self.grid_manager.host_hwnd
            ));

            // --- GRID DRAW ---
            self.fill_grid.draw(ui);

            // --- BOTTOM DIAGNOSTICS ---
            {
                let hwnd_val_opt = self.window_info.lock().unwrap().hwnd.clone();
                if let Some(hwnd_val) = hwnd_val_opt {
                    ui.label(format!("Pinned HWND: 0x{:X}", hwnd_val));
                } else {
                    ui.label("Waiting for Chrome HWND...");
                }
                if let Some(pid) = self.chrome_pid {
                    ui.label(format!("Chrome PID: {}", pid));
                }
                // Show all lines of debugchrome output from channel
                if let Some(rx) = &self.chrome_output_rx {
                    for msg in rx.try_iter() {
                        println!("{}", msg);
                    }
                }
                // Show pinning status from channel
                if let Some(rx) = &self.pin_request_rx {
                    for (hwnd, rect) in rx.try_iter() {
                        ui.label(format!("Pin request: HWND=0x{:X}, rect={:?}", hwnd, rect));
                        self.window_info.lock().unwrap().hwnd = Some(hwnd);
                        self.last_pinned_rect = Some(rect);
                    }
                }

                // Move Chrome window to grid position only if HWND is valid
                if let Some(hwnd_val) = self.window_info.lock().unwrap().hwnd {
                    let hwnd = hwnd_val as HWND;
                    if PositionGridManager::is_window(hwnd) {
                        let _ = self.grid_manager.move_and_resize(hwnd);
                    } else {
                        println!("[HydraDemo] Target HWND is invalid or closed. Exiting app.");
                        std::process::exit(0);
                    }
                }
            }
        }); // <-- This closes the CentralPanel::show closure
    } // <-- This closes the update function
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::sync::{Arc, Mutex};

        // Create shared window_info for ctrlc handler
        let window_info = Arc::new(Mutex::new(ChromeWindowInfo::default()));
        // Set Ctrl-C handler ONCE
        let window_info_ctrlc = Arc::clone(&window_info);
        ctrlc::set_handler(move || {
            let win = window_info_ctrlc.lock().unwrap();

            if let Some(pid) = win.pid {
                println!("[HydraDemo] Ctrl-C pressed, killing Chrome PID: {}", pid);
                let _ = Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .status();
            }
            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");
        let options = eframe::NativeOptions::default();
        // Pass window_info to HydraDemoApp
        let _ = eframe::run_native(
            "e_window_hydra",
            options,
            Box::new(|cc| {
                let mut hwnd_opt = None;
                #[cfg(target_os = "windows")]
                {
                    use winit::raw_window_handle::RawWindowHandle;
                    let raw = cc.window_handle().unwrap().as_raw();
                    if let RawWindowHandle::Win32(handle) = raw {
                        let hwnd = handle_to_hwnd(handle);
                        hwnd_opt = Some(hwnd.0 as u32);
                    }
                }
                Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                    HydraDemoApp::with_hwnd(hwnd_opt),
                ))
            }),
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("Sorry, e_window_hydra is only supported on Windows targets at this time.");
        println!("This example will compile, but does not run on non-Windows platforms.");
    }
}
