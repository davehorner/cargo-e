// Request type for grid operations from callbacks
#[cfg(target_os = "windows")]
enum GridRequest {
    AlignPinned {
        pinned_hwnd: u32,
        alignment_mode: AlignmentMode,
        host_hwnd: u32,
        src_hwnd: u32,
        event_rect: (i32, i32, i32, i32),
    },
}
#[cfg(target_os = "windows")]
use std::sync::atomic::AtomicU32;
#[cfg(target_os = "windows")]
static HOST_HWND: AtomicU32 = AtomicU32::new(0);
// e_window_orca.rs
// Based on e_window_hydra, but launches https://hundredrabbits.github.io/Orca/ and integrates e_grid like e_window_e_grid_demo01

extern crate dashmap;
extern crate e_window;

use dashmap::DashMap;
use e_window::position_grid::PositionGrid;
use e_window::position_grid_manager::{AlignmentBase, AlignmentMode, PositionGridManager};
use eframe::egui;
use std::io::BufRead;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;

#[cfg(target_os = "windows")]
use e_grid::ipc_protocol::WindowFocusEvent;
#[cfg(target_os = "windows")]
use e_grid::GridClient;

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
pub struct OrcaDemoApp {
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
    grid_client: Option<GridClient>,
    focus_request_rx: Option<Receiver<u64>>,
    alignment_state: Arc<DashMap<&'static str, i32>>, // "mode" (as i32), "offset_x", "offset_y"
    grid_request_rx: Option<Receiver<GridRequest>>,
    grid_request_tx: Option<std::sync::mpsc::Sender<GridRequest>>,
}

#[cfg(target_os = "windows")]
impl OrcaDemoApp {
    pub fn with_hwnd(hwnd: Option<u32>) -> Self {
        let (grid_request_tx, grid_request_rx) = mpsc::channel();
        // ...existing code...
        let alignment_state = Arc::new(DashMap::new());
        alignment_state.insert("mode", 0); // 0: Grid, 1: HostExact, 2: Offset
        alignment_state.insert("offset_x", 0);
        alignment_state.insert("offset_y", 0);
        let dummy_grid = PositionGrid::default();
        let window_info = Arc::new(Mutex::new(ChromeWindowInfo::default()));
        let (chrome_output_tx, chrome_output_rx) = mpsc::channel();
        let (pin_request_tx, pin_request_rx) = mpsc::channel();
        let (chrome_launch_request_tx, chrome_launch_request_rx) = mpsc::channel();
        let (click_request_tx, click_request_rx) = mpsc::channel();
        let (focus_request_tx, focus_request_rx) = mpsc::channel::<u64>();
        // Static JS script to inject into Chrome (optional for Orca)
        const INJECT_SCRIPT: &str = "";
        let script_tempfile = {
            use std::io::Write;
            use tempfile::NamedTempFile;
            let mut file = NamedTempFile::new().expect("Failed to create temp script file");
            file.write_all(INJECT_SCRIPT.as_bytes())
                .expect("Failed to write script file");
            file
        };
        // e_grid client setup
        let mut grid_client: Option<GridClient> = None;
        match GridClient::new() {
            Ok(c) => grid_client = Some(c),
            Err(_) => {
                println!("Grid server not running, starting server in-process...");
                let window_info_for_destroy = window_info.clone();
                // Set the global HOST_HWND for cross-thread access
                if let Some(hwnd) = hwnd {
                    HOST_HWND.store(hwnd as u32, Ordering::SeqCst);
                }
                thread::spawn(move || {
                    use e_grid::ipc_server::start_server_with_mode;
                    use e_grid::EventDispatchMode;
                    use e_grid::{
                        EVENT_TYPE_WINDOW_DESTROYED, EVENT_TYPE_WINDOW_MOVE,
                        EVENT_TYPE_WINDOW_MOVE_START, EVENT_TYPE_WINDOW_MOVE_STOP,
                        EVENT_TYPE_WINDOW_RESIZE, EVENT_TYPE_WINDOW_RESIZE_START,
                        EVENT_TYPE_WINDOW_RESIZE_STOP,
                    };
                    let _ = start_server_with_mode(EventDispatchMode::Open, |server| {
                        if let Some(rx) = server.take_window_event_direct_receiver() {
                            let window_info_for_destroy = window_info_for_destroy.clone();
                            std::thread::spawn(move || {
                                for event in rx {
                                    match event.event_type {
                                        EVENT_TYPE_WINDOW_DESTROYED => {
                                            let pinned_hwnd =
                                                window_info_for_destroy.lock().unwrap().hwnd;
                                            if let Some(pinned) = pinned_hwnd {
                                                if event.hwnd == pinned as u64 {
                                                    println!("[OrcaDemo] 💀 Orca window destroyed (HWND=0x{:X}), exiting...", pinned);
                                                    std::process::exit(0);
                                                }
                                            }
                                        }
                                        EVENT_TYPE_WINDOW_LOCATIONCHANGE => {
                                            let pinned_hwnd =
                                                window_info_for_destroy.lock().unwrap().hwnd;
                                            let host_hwnd = {
                                                let val = HOST_HWND.load(Ordering::SeqCst);
                                                if val != 0 {
                                                    Some(val)
                                                } else {
                                                    None
                                                }
                                            };
                                            if let (Some(pinned), Some(host)) =
                                                (pinned_hwnd, host_hwnd)
                                            {
                                                let src_hwnd = event.hwnd as u32;
                                                let dst_hwnd = if src_hwnd == pinned {
                                                    host
                                                } else if src_hwnd == host {
                                                    pinned
                                                } else {
                                                    0
                                                };
                                                if dst_hwnd != 0 && src_hwnd != dst_hwnd {
                                                    use winapi::um::winuser::{
                                                        SetWindowPos, ShowWindow, SWP_NOZORDER,
                                                        SW_RESTORE,
                                                    };
                                                    unsafe {
                                                        SetWindowPos(
                                                            dst_hwnd as HWND,
                                                            std::ptr::null_mut(),
                                                            event.real_x as i32,
                                                            event.real_y as i32,
                                                            event.real_width as i32,
                                                            event.real_height as i32,
                                                            SWP_NOZORDER,
                                                        );
                                                        // Always restore and raise the pinned window
                                                        if dst_hwnd == pinned {
                                                            ShowWindow(pinned as HWND, SW_RESTORE);
                                                            // Set topmost and above host
                                                            SetWindowPos(
                                                pinned as HWND,
                                                winapi::um::winuser::HWND_TOPMOST,
                                                0, 0, 0, 0,
                                                winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE,
                                            );
                                                            SetWindowPos(
                                                pinned as HWND,
                                                host as HWND,
                                                0, 0, 0, 0,
                                                winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE,
                                            );
                                                        }
                                                    }
                                                    println!("[OrcaDemo] Synced move/resize/locationchange event: src=0x{:X} dst=0x{:X} rect=({}, {}, {}, {})", src_hwnd, dst_hwnd, event.real_x, event.real_y, event.real_width, event.real_height);
                                                }
                                            }
                                        }

                                        EVENT_TYPE_WINDOW_MOVE
                                        | EVENT_TYPE_WINDOW_MOVE_START
                                        | EVENT_TYPE_WINDOW_MOVE_STOP
                                        | EVENT_TYPE_WINDOW_RESIZE
                                        | EVENT_TYPE_WINDOW_RESIZE_START
                                        | EVENT_TYPE_WINDOW_RESIZE_STOP => {
                                            let pinned_hwnd =
                                                window_info_for_destroy.lock().unwrap().hwnd;
                                            let host_hwnd = {
                                                let val = HOST_HWND.load(Ordering::SeqCst);
                                                if val != 0 {
                                                    Some(val)
                                                } else {
                                                    None
                                                }
                                            };
                                            if let (Some(pinned), Some(host)) =
                                                (pinned_hwnd, host_hwnd)
                                            {
                                                let src_hwnd = event.hwnd as u32;
                                                let dst_hwnd = if src_hwnd == pinned {
                                                    host
                                                } else if src_hwnd == host {
                                                    pinned
                                                } else {
                                                    0
                                                };
                                                if dst_hwnd != 0 && src_hwnd != dst_hwnd {
                                                    use winapi::um::winuser::{
                                                        SetWindowPos, ShowWindow, SWP_NOZORDER,
                                                        SW_RESTORE,
                                                    };
                                                    unsafe {
                                                        SetWindowPos(
                                                            dst_hwnd as HWND,
                                                            std::ptr::null_mut(),
                                                            event.real_x as i32,
                                                            event.real_y as i32,
                                                            event.real_width as i32,
                                                            event.real_height as i32,
                                                            SWP_NOZORDER,
                                                        );
                                                        // Always restore and raise the pinned window
                                                        if dst_hwnd == pinned {
                                                            ShowWindow(pinned as HWND, SW_RESTORE);
                                                            // Set topmost and above host
                                                            SetWindowPos(
                                                                pinned as HWND,
                                                                winapi::um::winuser::HWND_TOPMOST,
                                                                0, 0, 0, 0,
                                                                winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE,
                                                            );
                                                            SetWindowPos(
                                                                pinned as HWND,
                                                                host as HWND,
                                                                0, 0, 0, 0,
                                                                winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE,
                                                            );
                                                        }
                                                    }
                                                    println!("[OrcaDemo] Synced move/resize event: src=0x{:X} dst=0x{:X} rect=({}, {}, {}, {})", src_hwnd, dst_hwnd, event.real_x, event.real_y, event.real_width, event.real_height);
                                                }
                                            }
                                        }
                                        _ => {
                                            println!(
                                                "[Server caller] [Direct] Raw window event: {:?}",
                                                event
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    });
                });
                for _ in 0..10 {
                    match GridClient::new() {
                        Ok(c) => {
                            println!("Connected to in-process server!");
                            grid_client = Some(c);
                            break;
                        }
                        Err(_) => thread::sleep(std::time::Duration::from_millis(300)),
                    }
                }
            }
        }
        // No need to call setup_window_events_with_mode on the client; server is started with correct mode

        // Register focus callback and both standard and direct window event callbacks if grid_client is available
        if let Some(ref mut client) = grid_client {
            let alignment_state_cb = alignment_state.clone();
            let grid_request_tx_cb = grid_request_tx.clone();
            let host_hwnd = hwnd;
            let focus_request_tx = focus_request_tx.clone();
            client.set_focus_callback(move |focus_event: WindowFocusEvent| {
                // Only care about host window focus
                if let Some(host_hwnd) = host_hwnd {
                    if focus_event.hwnd == host_hwnd as u64 && focus_event.event_type == 0 {
                        let _ = focus_request_tx.send(focus_event.hwnd);
                        println!(
                            "[OrcaDemo] Host focus event received via callback for HWND=0x{:X}",
                            focus_event.hwnd
                        );
                    }
                }
            });

            // Register window event callback to exit if pinned Orca window is destroyed
            let window_info_for_destroy = window_info.clone();
            use e_grid::EVENT_TYPE_WINDOW_DESTROYED;
            client.set_window_event_callback(move |event| {
                println!("[OrcaDemo] Window event: {:?}", event);
                // Always get latest alignment mode and offsets
                let mode_idx = alignment_state_cb.get("mode").map(|v| *v).unwrap_or(0);
                let offset_x = alignment_state_cb.get("offset_x").map(|v| *v).unwrap_or(0);
                let offset_y = alignment_state_cb.get("offset_y").map(|v| *v).unwrap_or(0);
                let alignment_mode = match mode_idx {
                    0 => AlignmentMode::Grid,
                    1 => AlignmentMode::HostExact,
                    2 => AlignmentMode::Offset { base: AlignmentBase::Host, dx: offset_x, dy: offset_y },
                    _ => AlignmentMode::Grid,
                };
                match event.event_type {
                                        EVENT_TYPE_WINDOW_DESTROYED => {
                                            let pinned_hwnd = window_info_for_destroy.lock().unwrap().hwnd;
                                            if let Some(pinned) = pinned_hwnd {
                                                if event.hwnd == pinned as u64 {
                                                    println!("[OrcaDemo] 💀 Orca window destroyed (HWND=0x{:X}), exiting...", pinned);
                                                    std::process::exit(0);
                                                }
                                            }
                                        }
                                        EVENT_TYPE_WINDOW_LOCATIONCHANGE => {
                                            let pinned_hwnd = window_info_for_destroy.lock().unwrap().hwnd;
                                            let host_hwnd = {
                                                let val = HOST_HWND.load(Ordering::SeqCst);
                                                if val != 0 { Some(val) } else { None }
                                            };
                                            if let (Some(pinned), Some(host)) = (pinned_hwnd, host_hwnd) {
                                                let src_hwnd = event.hwnd as u32;
                                                let dst_hwnd = if src_hwnd == pinned { host } else if src_hwnd == host { pinned } else { 0 };
                                                if dst_hwnd != 0 && src_hwnd != dst_hwnd {
                                                    // Send request to main thread for alignment
                                                    let event_rect = (event.real_x as i32, event.real_y as i32, event.real_width as i32, event.real_height as i32);
                                                    let _ = grid_request_tx_cb.send(GridRequest::AlignPinned {
                                                        pinned_hwnd: pinned,
                                                        alignment_mode: alignment_mode.clone(),
                                                        host_hwnd: host,
                                                        src_hwnd,
                                                        event_rect,
                                                    });
                                                }
                                            }
                                        }
                _ => {
                    println!("[Server caller] [Direct] Raw window event: {:?}", event);
                }
                }
            }).ok();
        }
        {
            let window_info_clone = window_info.clone();
            let chrome_output_tx_clone = chrome_output_tx.clone();
            let script_file_path = script_tempfile.path().to_path_buf();
            thread::spawn(move || {
                let click_request_tx_clone = click_request_tx.clone();
                let (x, y, w, h) = chrome_launch_request_rx.recv().unwrap_or((0, 0, 800, 600));
                let orca_url = format!("debugchrome:https://hundredrabbits.github.io/Orca/?!openwindow&!x={}&!y={}&!w={}&!h={}", x, y, w, h);
                println!("[OrcaDemo] Spawning debugchrome with URL: {}", orca_url);
                let chrome = Command::new("debugchrome")
                    .arg(&orca_url)
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
                                            // No auto click for Orca
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
                                    "[OrcaDemo] No stdout from debugchrome child process"
                                        .to_string(),
                                );
                            }
                        });
                    }
                    Err(e) => {
                        let _ = chrome_output_tx_clone
                            .send(format!("[OrcaDemo] Failed to launch debugchrome: {}", e));
                    }
                }
            });
        }
        // Helper closure to get host_hwnd from grid_manager (must be thread-safe)
        fn grid_manager_host_hwnd() -> Option<u32> {
            // This is a hack: in a real app, you would want to share grid_manager safely.
            // For now, just use the static host_hwnd if available.
            // (You may want to refactor to share grid_manager via Arc<Mutex<...>> if needed.)
            let val = HOST_HWND.load(Ordering::SeqCst);
            if val != 0 {
                Some(val)
            } else {
                None
            }
        }

        let mut grid_manager = PositionGridManager::new();
        if let Some(hwnd) = hwnd {
            grid_manager.host_hwnd = Some(hwnd as u32);
            // Store for cross-thread access
            HOST_HWND.store(hwnd as u32, Ordering::SeqCst);
        }
        let alignment_state = Arc::new(DashMap::new());
        alignment_state.insert("mode", 0); // 0: Grid, 1: HostExact, 2: Offset
        alignment_state.insert("offset_x", 0);
        alignment_state.insert("offset_y", 0);
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
            grid_client,
            focus_request_rx: Some(focus_request_rx),
            alignment_state,
            grid_request_rx: Some(grid_request_rx),
            grid_request_tx: Some(grid_request_tx),
        }
    }
}

#[cfg(target_os = "windows")]
impl eframe::App for OrcaDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process grid requests from callbacks
        if let Some(rx) = &self.grid_request_rx {
            while let Ok(req) = rx.try_recv() {
                match req {
                    GridRequest::AlignPinned {
                        pinned_hwnd,
                        alignment_mode,
                        host_hwnd,
                        src_hwnd,
                        event_rect,
                    } => {
                        // Use grid_manager to get target rect and move pinned window
                        if let Some(rect) = unsafe {
                            self.grid_manager
                                .get_target_rect_for_alignment(pinned_hwnd as HWND, alignment_mode)
                        } {
                            let (x, y, w, h) = rect;
                            use winapi::um::winuser::{
                                SetWindowPos, ShowWindow, SWP_NOZORDER, SW_RESTORE,
                            };
                            unsafe {
                                SetWindowPos(
                                    pinned_hwnd as HWND,
                                    std::ptr::null_mut(),
                                    x,
                                    y,
                                    w,
                                    h,
                                    SWP_NOZORDER,
                                );
                                ShowWindow(pinned_hwnd as HWND, SW_RESTORE);
                                SetWindowPos(
                                    pinned_hwnd as HWND,
                                    winapi::um::winuser::HWND_TOPMOST,
                                    0,
                                    0,
                                    0,
                                    0,
                                    winapi::um::winuser::SWP_NOMOVE
                                        | winapi::um::winuser::SWP_NOSIZE,
                                );
                                SetWindowPos(
                                    pinned_hwnd as HWND,
                                    host_hwnd as HWND,
                                    0,
                                    0,
                                    0,
                                    0,
                                    winapi::um::winuser::SWP_NOMOVE
                                        | winapi::um::winuser::SWP_NOSIZE,
                                );
                            }
                        }
                    }
                }
            }
        }
        // Restore pinned window if host focus event received via channel
        if let Some(rx) = &mut self.focus_request_rx {
            for _ in rx.try_iter() {
                if let Some(hwnd_val) = self.window_info.lock().unwrap().hwnd {
                    let hwnd = hwnd_val as HWND;
                    if unsafe { PositionGridManager::is_window(hwnd) } {
                        unsafe {
                            use winapi::um::winuser::{ShowWindow, SW_RESTORE};
                            ShowWindow(hwnd, SW_RESTORE);
                        }
                        println!(
                            "[OrcaDemo] Host focus event: restoring pinned HWND=0x{:X}",
                            hwnd_val
                        );
                    }
                }
            }
        }
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- ALIGNMENT MODE SELECTION ---
            let alignment_mode_names = ["Grid (client area)", "HostExact (outer rect)", "Offset (custom)"];
            let mut mode_idx = self.alignment_state.get("mode").map(|v| *v).unwrap_or(0);
            ui.horizontal(|ui| {
                ui.label("Alignment Mode:");
                for (i, name) in alignment_mode_names.iter().enumerate() {
                    let i32_idx = i as i32;
                    if ui.radio_value(&mut mode_idx, i32_idx, *name).changed() {
                        self.alignment_state.insert("mode", i32_idx);
                    }
                }
            });
            if mode_idx == 2 {
                let mut offset_x = self.alignment_state.get("offset_x").map(|v| *v).unwrap_or(0);
                let mut offset_y = self.alignment_state.get("offset_y").map(|v| *v).unwrap_or(0);
                ui.horizontal(|ui| {
                    ui.label("Offset X:");
                    if ui.add(egui::Slider::new(&mut offset_x, -1000..=1000).text("px")).changed() {
                        self.alignment_state.insert("offset_x", offset_x);
                    }
                    ui.label("Offset Y:");
                    if ui.add(egui::Slider::new(&mut offset_y, -1000..=1000).text("px")).changed() {
                        self.alignment_state.insert("offset_y", offset_y);
                    }
                });
            }
            // --- TOP: Chrome/Orca info ---
            ui.heading("OrcaDemoApp (Orca Grid + e_grid sync)");
            if let Ok(win) = self.window_info.lock() {
                ui.label(format!("Chrome HWND: {:?}", win.hwnd));
                ui.label(format!("Chrome PID: {:?}", win.pid));
                ui.label(format!("Title: {:?}", win.title));
                ui.label(format!("Target: {:?}", win.target));
                ui.label(format!("Page URL: {:?}", win.page_url));
            }
            ui.separator();
            if let Some(rx) = &self.chrome_output_rx {
                while let Ok(msg) = rx.try_recv() {
                    ui.label(msg);
                }
            }

            // --- GRID DIAGNOSTICS ---
            let label_height = 32.0;
            let available = ui.available_size();
            let grid_area = egui::vec2(available.x, (available.y - label_height).max(32.0));
            let (mut new_grid, _char_size) = PositionGrid::from_text_style(self.eframe_hwnd, ui, egui::TextStyle::Heading, egui::Color32::LIGHT_GREEN, None);
            new_grid.rect = ui.allocate_exact_size(grid_area, egui::Sense::hover()).0;
            self.fill_grid = new_grid;
            self.grid_manager.grid = Some(&self.fill_grid as *const PositionGrid);

            // Send grid coordinates to Chrome launch thread if not launched yet
            if let Some(tx) = &self.chrome_launch_request_tx {
                if self.chrome_spawned.load(std::sync::atomic::Ordering::SeqCst) == false {
                    if let Some((grid_x, grid_y, grid_w, grid_h)) = unsafe { self.grid_manager.get_grid_screen_rect() } {
                        let x = grid_x;
                        let y = grid_y;
                        let w = grid_w;
                        let h = grid_h;
                        let _ = tx.send((x, y, w, h));
                        self.chrome_spawned.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            }

            let grid_rect = unsafe { self.grid_manager.get_grid_screen_rect().unwrap_or((0, 0, 0, 0)) };
            let host_rect = unsafe { self.grid_manager.get_host_screen_rect().unwrap_or((0, 0, 0, 0)) };
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
                // Show pinning status from channel
                if let Some(rx) = &self.pin_request_rx {
                    for (hwnd, rect) in rx.try_iter() {
                        ui.label(format!("Pin request: HWND=0x{:X}, rect={:?}", hwnd, rect));
                        self.window_info.lock().unwrap().hwnd = Some(hwnd);
                        self.last_pinned_rect = Some(rect);
                        // After pin, set topmost and above host ONCE
                        let hwnd = hwnd as HWND;
                        if unsafe { PositionGridManager::is_window(hwnd) } {
                            let _ = unsafe { self.grid_manager.move_and_resize(hwnd) };
                            // Ensure window is visible and not minimized
                            unsafe {
                                use winapi::um::winuser::{ShowWindow, SW_RESTORE};
                                ShowWindow(hwnd, SW_RESTORE);
                            }
                            // Set pinned topmost, host not topmost, and set z-order
                            let topmost_res = unsafe { PositionGridManager::set_topmost(hwnd, true) };
                            println!("[OrcaDemo] Set pinned HWND=0x{:X} topmost: {}", hwnd as u32, topmost_res);
                            if let Some(host_hwnd) = self.grid_manager.host_hwnd {
                                let _ = unsafe { PositionGridManager::set_topmost(host_hwnd as HWND, false) };
                                let zorder_res = unsafe { PositionGridManager::set_zorder_above(hwnd, host_hwnd as HWND) };
                                println!("[OrcaDemo] Set pinned HWND=0x{:X} above host HWND=0x{:X}: {}", hwnd as u32, host_hwnd, zorder_res);
                            }
                        }
                    }
                }
            }
        });
    }
}

#[cfg(target_os = "windows")]
impl Drop for OrcaDemoApp {
    fn drop(&mut self) {
        // Send WM_CLOSE to pinned_hwnd if set
        // Always send WM_CLOSE to pinned HWND on exit
        let hwnd_opt = self.window_info.lock().unwrap().hwnd.clone();
        if let Some(hwnd_val) = hwnd_opt {
            let hwnd = hwnd_val as HWND;
            unsafe {
                use winapi::um::winuser::PostMessageW;
                use winapi::um::winuser::WM_CLOSE;
                let result = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                println!(
                    "[OrcaDemo] Sent WM_CLOSE to pinned HWND: 0x{:X}, result={}",
                    hwnd_val, result
                );
            }
        }
        if let Some(pid) = self.chrome_pid.take() {
            println!("[OrcaDemo] Killing Chrome PID: {}", pid);
            let _ = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status();
        }
    }
}

#[cfg(target_os = "windows")]
fn handle_to_hwnd(
    handle: winit::raw_window_handle::Win32WindowHandle,
) -> windows::Win32::Foundation::HWND {
    let hwnd_isize: isize = handle.hwnd.into();
    let hwnd = hwnd_isize as *mut core::ffi::c_void;
    windows::Win32::Foundation::HWND(hwnd)
}

#[cfg(target_os = "windows")]
fn main() {
    use std::process::Command;
    use std::sync::{Arc, Mutex};
    let window_info = Arc::new(Mutex::new(ChromeWindowInfo::default()));
    let window_info_ctrlc = Arc::clone(&window_info);
    ctrlc::set_handler(move || {
        let win = window_info_ctrlc.lock().unwrap();
        if let Some(pid) = win.pid {
            println!("[OrcaDemo] Ctrl-C pressed, killing Chrome PID: {}", pid);
            let _ = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .status();
        }
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "e_window_orca",
        options,
        Box::new(|cc| {
            let mut hwnd_opt = None;
            #[cfg(target_os = "windows")]
            {
                use winit::raw_window_handle::HasWindowHandle;
                use winit::raw_window_handle::RawWindowHandle;

                let raw = cc.window_handle().unwrap().as_raw();
                if let RawWindowHandle::Win32(handle) = raw {
                    let hwnd = handle_to_hwnd(handle);
                    hwnd_opt = Some(hwnd.0 as u32);
                }
            }

            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                OrcaDemoApp::with_hwnd(hwnd_opt),
            ))
        }),
    );
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("Sorry, e_window_orca is only supported on Windows targets at this time.");
}
