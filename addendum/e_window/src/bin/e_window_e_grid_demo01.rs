// e_window_e_grid.rs
// Combines Chrome grid pinning (Hydra demo) with MIDI event playback (e_midi_demo)
// - Launches Chrome pinned to grid
// - Handles move, resize, and focus events for Chrome HWND
// - Plays MIDI songs on window events
// - Includes detailed logging and comments

extern crate e_window;
extern crate e_midi;
extern crate dashmap;

use e_window::position_grid::PositionGrid;
use e_window::position_grid_manager::PositionGridManager;
use eframe::egui;
use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use once_cell::sync::Lazy;

#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;

#[cfg(target_os = "windows")]
use e_grid::ipc_protocol::WindowFocusEvent;
#[cfg(target_os = "windows")]
use e_grid::ipc_server::start_server;
#[cfg(target_os = "windows")]
use e_grid::GridClient;
use e_midi::MidiPlayer;


#[cfg(target_os = "windows")]
static PINNED_HWND_MAP: Lazy<DashMap<&'static str, u32>> = Lazy::new(|| DashMap::new());
#[cfg(target_os = "windows")]
static CHROME_WINDOW_INFO_MAP: Lazy<DashMap<&'static str, ChromeWindowInfo>> = Lazy::new(|| DashMap::new());

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
pub struct EGridDemoApp {
    programmatic_move_in_progress: Arc<std::sync::atomic::AtomicBool>,
    programmatic_resize_in_progress: Arc<std::sync::atomic::AtomicBool>,
    script_tempfile: Option<tempfile::NamedTempFile>,
    chrome_launch_request_tx: Option<std::sync::mpsc::Sender<(i32, i32, i32, i32)>>,
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
    midi_player: Arc<MidiPlayer>,
    midi_sender: Arc<std::sync::mpsc::Sender<e_midi::MidiCommand>>,
    song_map: Arc<DashMap<u64, usize>>,
    next_song: Arc<AtomicUsize>,
    grid_client: Option<GridClient>,
    host_move_rx: Option<Receiver<(i32, i32, i32, i32)>>,
    focus_request_rx: Option<Receiver<(u64)>>,
    grid_offset_x: i32,
    grid_offset_y: i32,
    zorder_toggle_tx: Option<std::sync::mpsc::Sender<bool>>,
    zorder_toggle_rx: Option<std::sync::mpsc::Receiver<bool>>,
    zorder_thread_running: bool,
    zorder_state: bool,
    zorder_delay_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(target_os = "windows")]
impl EGridDemoApp {
    pub fn with_hwnd(hwnd: Option<u32>) -> Self {
        let programmatic_move_in_progress = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let programmatic_resize_in_progress = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dummy_grid = PositionGrid::default();
        CHROME_WINDOW_INFO_MAP.insert("chrome", ChromeWindowInfo::default());
        let (chrome_output_tx, chrome_output_rx) = mpsc::channel();
        let (pin_request_tx, pin_request_rx) = mpsc::channel();
        let (chrome_launch_request_tx, chrome_launch_request_rx) = mpsc::channel();
        let (click_request_tx, click_request_rx) = mpsc::channel();
        // Static JS script to inject into Chrome
        const INJECT_SCRIPT: &str = r#"
document.getElementById('close-icon').click();
setInterval(() => {
  const el = document.getElementById('close-icon');
  if (el) {
    el.click();
    const modal = document.getElementById('modal');
    if (modal) modal.remove();
  }
}, 16);
emitter.emit('ui: hide info');
"#;
        let script_tempfile = {
            use std::io::Write;
            use tempfile::NamedTempFile;
            let mut file = NamedTempFile::new().expect("Failed to create temp script file");
            file.write_all(INJECT_SCRIPT.as_bytes())
                .expect("Failed to write script file");
            file
        };
        // MIDI setup
        let midi_player = Arc::new(MidiPlayer::new().expect("Failed to init MidiPlayer"));
        let midi_sender = Arc::new(midi_player.get_command_sender());
        let song_map = Arc::new(DashMap::<u64, usize>::new());
        let next_song = Arc::new(AtomicUsize::new(0));
        // e_grid client setup
        let mut grid_client: Option<GridClient> = None;
        match GridClient::new() {
            Ok(c) => grid_client = Some(c),
            Err(_) => {
                println!("Grid server not running, starting server in-process...");
                thread::spawn(|| { start_server().unwrap(); });
                for _ in 0..10 {
                    match GridClient::new() {
                        Ok(c) => { println!("Connected to in-process server!"); grid_client = Some(c); break; },
                        Err(_) => thread::sleep(std::time::Duration::from_millis(300)),
                    }
                }
            }
        }
        // Chrome launch thread (same as hydra)
        {
            let chrome_output_tx_clone = chrome_output_tx.clone();
            let script_file_path = script_tempfile.path().to_path_buf();
            thread::spawn(move || {
                print!("[EGridDemo] Spawning debugchrome with script file: {}", script_file_path.display());
                let click_request_tx_clone = click_request_tx.clone();
                let (x, y, w, h) = chrome_launch_request_rx.recv().unwrap_or((0, 0, 800, 600));
                let hydra_url = format!("debugchrome:https://hydra.ojack.xyz/?sketch_id=example&!id=&!openwindow&!x={}&!y={}&!w={}&!h={}", x, y, w, h);
                println!("[EGridDemo] Spawning debugchrome with URL: {}", hydra_url);
                let chrome = Command::new("debugchrome")
                    .arg(&hydra_url)
                    .arg("--script-file")
                    .arg(script_file_path.display().to_string())
                    .stdout(std::process::Stdio::piped())
                    .spawn();
                match chrome {
                    Ok(mut child) => {
                        let pid = child.id();
                        if let Some(mut info) = CHROME_WINDOW_INFO_MAP.get_mut("chrome") {
                            info.pid = Some(pid);
                        }
                        let tx = chrome_output_tx_clone;
                        thread::spawn(move || {
                            let click_tx = click_request_tx_clone;
                            if let Some(stdout) = child.stdout.take() {
                                use std::io::BufReader;
                                let reader = BufReader::new(stdout);
                                for line in reader.lines().flatten() {
                                    println!("[debugchrome stdout] {}", line);
                                    let _ = tx.send(format!("[debugchrome stdout] {}", line));
                                    let mut info = CHROME_WINDOW_INFO_MAP.get_mut("chrome").map(|mut_ref| mut_ref.clone()).unwrap_or_default();
                                    static mut CHROME_HWND_VERIFIED: bool = false;
                                    if let Some(hwnd_hex) = line.strip_prefix("HWND: 0x") {
                                        if let Ok(hwnd_val) = u32::from_str_radix(hwnd_hex.trim(), 16) {
                                            #[cfg(target_os = "windows")]
                                            unsafe {
                                                if !CHROME_HWND_VERIFIED {
                                                    use winapi::um::winuser::GetClassNameW;
                                                    use winapi::shared::windef::HWND;
                                                    use std::ffi::OsString;
                                                    use std::os::windows::ffi::OsStringExt;
                                                    let hwnd_win = hwnd_val as HWND;
                                                    let mut class_buf = [0u16; 256];
                                                    let len = GetClassNameW(hwnd_win, class_buf.as_mut_ptr(), class_buf.len() as i32);
                                                    let class_name = if len > 0 {
                                                        OsString::from_wide(&class_buf[..len as usize]).to_string_lossy().to_string()
                                                    } else {
                                                        String::new()
                                                    };
                                                    println!("[DEBUGCHROME] HWND=0x{:X} class_name='{}'", hwnd_val, class_name);
                                                    if class_name == "Chrome_WidgetWin_1" {
                                                        println!("[DEBUGCHROME] Verified Chrome HWND, pinning.");
                                                        PINNED_HWND_MAP.insert("pinned", hwnd_val);
                                                        info.hwnd = Some(hwnd_val);
                                                        let _ = click_tx.send((15, 10));
                                                    } else {
                                                        println!("[DEBUGCHROME] HWND=0x{:X} is not Chrome_WidgetWin_1, not pinning.", hwnd_val);
                                                    }
                                                    CHROME_HWND_VERIFIED = true;
                                                }
                                            }
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
                                        CHROME_WINDOW_INFO_MAP.insert("chrome", info.clone());
                                    }
                                }
                            } else {
                                let _ = tx.send("[EGridDemo] No stdout from debugchrome child process".to_string());
                            }
                        });
                    }
                    Err(e) => {
                        let _ = chrome_output_tx_clone.send(format!("[EGridDemo] Failed to launch debugchrome: {}", e));
                    }
                }
            });
        }
        let mut grid_manager = PositionGridManager::new();
        if let Some(hwnd) = hwnd {
            grid_manager.host_hwnd = Some(hwnd as u32);
        }
        // Add atomic flag to track when Chrome is pinned to grid
        let pinned_layout_complete = Arc::new(std::sync::atomic::AtomicBool::new(false));
        // Channel for host move requests
        let (host_move_tx, host_move_rx) = mpsc::channel::<(i32, i32, i32, i32)>();
        let (focus_request_tx, focus_request_rx) = mpsc::channel::<u64>();
        // Register window event callbacks for MIDI
        if let Some(ref mut client) = grid_client {
            let midi_sender_focus = Arc::clone(&midi_sender);
            let song_map_for_focus: Arc<DashMap<u64, usize>> = Arc::clone(&song_map);
            let next_song_for_focus = Arc::clone(&next_song);
            let total_songs = midi_player.get_total_song_count();
            println!("[DEBUG] Registering focus callback with grid_client...");
            let focus_request_tx_cb = focus_request_tx.clone();
            client.set_focus_callback(move |focus_event: WindowFocusEvent| {
                println!("[DEBUG] Focus callback invoked: hwnd={}, event_type={}", focus_event.hwnd, focus_event.event_type);
                let hwnd = focus_event.hwnd;

                let focused = focus_event.event_type == 0;
                println!("[DEBUG] Focus event: hwnd={}, focused={} (event_type={})", hwnd, focused, focus_event.event_type);

                // --- MIDI logic ---
                let song_index = if let Some(idx) = song_map_for_focus.get(&hwnd) {
                    println!("🎵 Using assigned song {} for HWND {}", *idx, hwnd);
                    *idx
                } else {
                    let song_index = next_song_for_focus.fetch_add(1, Ordering::SeqCst) % total_songs;
                    song_map_for_focus.insert(hwnd, song_index);
                    song_index
                };
                if focused {
                                                       let _ = focus_request_tx_cb.send(hwnd);
                    println!("[DEBUG] Focused event detected for HWND {}. Stopping and playing song {}", hwnd, song_index);
                    let _ = midi_sender_focus.send(e_midi::MidiCommand::Stop);
                    let _ = midi_sender_focus.send(e_midi::MidiCommand::PlaySongResumeAware {
                        song_index: Some(song_index),
                        position_ms: None,
                        tracks: None,
                        tempo_bpm: None,
                    });
                    println!("▶️ [FOCUS] Queued play song {} for HWND {:?}", song_index, hwnd);
                    // Send focus event to main thread if host gets focus
                    // if let Some(host_hwnd) = grid_manager.host_hwnd {
                    //     if hwnd == host_hwnd as u64 {
                    //         let _ = focus_request_tx_cb.send(hwnd);
                    //         println!("[FOCUS] Sent host focus event to main thread");
                    //     }
                    // }
                } else {
                    println!("[DEBUG] Defocused event detected for HWND {}. Stopping playback.", hwnd);
                    let _ = midi_sender_focus.send(e_midi::MidiCommand::Stop);
                    println!("⏹️ [FOCUS] Queued stop playback for HWND {:?}", hwnd);
                }

                // --- Chrome auto-detect logic ---
                #[cfg(target_os = "windows")]
                {
                    use winapi::um::winuser::{GetClassNameW};
                    use winapi::shared::windef::HWND;
                    use std::ffi::OsString;
                    use std::os::windows::ffi::OsStringExt;
                    let hwnd_win = hwnd as HWND;
                    let mut class_buf = [0u16; 256];
                    let len = unsafe { GetClassNameW(hwnd_win, class_buf.as_mut_ptr(), class_buf.len() as i32) };
                    let class_name = if len > 0 {
                        OsString::from_wide(&class_buf[..len as usize]).to_string_lossy().to_string()
                    } else {
                        String::new()
                    };
                    println!("[FOCUS] HWND=0x{:X} class_name='{}'", hwnd, class_name);
     
                     if let Some(pinned_hwnd) = PINNED_HWND_MAP.get("pinned") {
                            let pinned_val = *pinned_hwnd;
                            println!("[FOCUS] Setting pinned HWND=0x{:X} topmost due to focus", pinned_val);
                            // Use static methods from PositionGridManager or direct winapi calls
                            let _ = PositionGridManager::set_topmost(pinned_val as HWND, true);
                            if let Some(host_hwnd) = grid_manager.host_hwnd {
                                let _ = PositionGridManager::set_zorder_above(pinned_val as HWND, host_hwnd as HWND);
                            }
                            return;
                    }
                    // If this is a Chrome window, set as pinned HWND
                    // Only pin if not already pinned
                    if class_name == "Chrome_WidgetWin_1" {
                        use winapi::um::winuser::{GetWindowTextW, GetWindowTextLengthW};
                        use winapi::um::winuser::{EnumChildWindows, GetClassNameW};
                        use winapi::shared::windef::HWND;
                        use std::ffi::OsString;
                        use std::os::windows::ffi::OsStringExt;
                        let len = unsafe { GetWindowTextLengthW(hwnd_win) };
                        if len > 0 {
                            let mut buf = vec![0u16; (len + 1) as usize];
                            let read = unsafe { GetWindowTextW(hwnd_win, buf.as_mut_ptr(), buf.len() as i32) };
                            if read > 0 {
                                let title = String::from_utf16_lossy(&buf[..read as usize]);
                                println!("[FOCUS] Window title: '{}'", title);
                            }
                        }
                        // Enumerate child windows of hwnd_win and print their class names and window titles
                        unsafe {

                            extern "system" fn enum_child_proc(child_hwnd: HWND, _lparam: isize) -> i32 {
                                let mut class_buf = [0u16; 256];
                                let len = unsafe { GetClassNameW(child_hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32) };
                                let class_name = if len > 0 {
                                    OsString::from_wide(&class_buf[..len as usize]).to_string_lossy().to_string()
                                } else {
                                    String::new()
                                };

                                let text_len = unsafe { GetWindowTextLengthW(child_hwnd) };
                                let window_title = if text_len > 0 {
                                    let mut buf = vec![0u16; (text_len + 1) as usize];
                                    let read = unsafe { GetWindowTextW(child_hwnd, buf.as_mut_ptr(), buf.len() as i32) };
                                    if read > 0 {
                                        String::from_utf16_lossy(&buf[..read as usize])
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                };

                                println!("[FOCUS] Child HWND=0x{:X}, class='{}', title='{}'", child_hwnd as u32, class_name, window_title);

                                // Check for "Who's using Chrome?" in window title
                                if window_title.contains("Who's using Chrome?") {
                                    println!("[FOCUS] Found 'Who's using Chrome?' dialog! HWND=0x{:X}", child_hwnd as u32);
                                }

                                1 // continue enumeration
                            }

                            EnumChildWindows(hwnd_win, Some(enum_child_proc), 0);
                        }
                        println!("[FOCUS] Detected Chrome window on focus. Setting as pinned HWND.");
                        PINNED_HWND_MAP.insert("pinned", hwnd as u32);
                        let mut info = CHROME_WINDOW_INFO_MAP.get_mut("chrome").map(|mut_ref| mut_ref.clone()).unwrap_or_default();
                        info.hwnd = Some(hwnd as u32);
                        CHROME_WINDOW_INFO_MAP.insert("chrome", info);
                    }
                }
            }).unwrap();
            println!("[DEBUG] Focus callback registered.");
            // Move/resize callbacks
            let midi_sender_start = Arc::clone(&midi_sender);
            let host_move_tx_cb = Arc::new(host_move_tx.clone());
            println!("[DIAG] host_move_tx_cb in move_callback: ptr={:p}", &host_move_tx_cb);
            let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress);
            let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress);
            client.set_move_resize_start_callback({
                let host_move_tx_cb = Arc::clone(&host_move_tx_cb);
                let midi_sender_start = Arc::clone(&midi_sender_start);
                let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress_cb);
                let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress_cb);
                move |event| {
                    let pinned_hwnd = PINNED_HWND_MAP.get("pinned").map(|v| *v as u64);
                    let host_hwnd = grid_manager.host_hwnd.map(|v| v as u64);
                    if Some(event.hwnd) == pinned_hwnd {
                        println!("[DEBUG] Move/resize START for pinned HWND: {}", event.hwnd);
                        let _ = midi_sender_start.send(e_midi::MidiCommand::PlaySongResumeAware {
                            song_index: Some(1), position_ms: None, tracks: None, tempo_bpm: None });
                    } else if Some(event.hwnd) == host_hwnd {
                        println!("[DEBUG] Move/resize START for host HWND: {}", event.hwnd);
                        let _ = midi_sender_start.send(e_midi::MidiCommand::PlaySongResumeAware {
                            song_index: Some(1), position_ms: None, tracks: None, tempo_bpm: None });
                        let target_rect = (event.real_x, event.real_y, event.real_width as i32, event.real_height as i32);
                        let _ = host_move_tx_cb.send(target_rect);
                    } else {
                        println!("[IGNORE] Move/resize START for non-pinned/non-host HWND: {}", event.hwnd);
                    }
                }
            }).unwrap();
            client.set_resize_callback({
                let host_move_tx_cb = Arc::clone(&host_move_tx_cb);
                let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress);
                let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress);
                move |event| {
                    println!(
                        "🔥 [DEBUG] Resize callback triggered! HWND: {}, rect=({}, {}, {}, {})",
                        event.hwnd, event.real_x, event.real_y, event.real_width, event.real_height
                    );
                    let pinned_hwnd = PINNED_HWND_MAP.get("pinned").map(|v| *v as u64);
                    let host_hwnd = grid_manager.host_hwnd.map(|v| v as u64);
                    if Some(event.hwnd) == pinned_hwnd {
                        println!("[DEBUG] Resize for pinned HWND: {}", event.hwnd);
                    } else if Some(event.hwnd) == host_hwnd {
                        println!("[DEBUG] Resize for host HWND: {}", event.hwnd);
                        let target_rect = (event.real_x, event.real_y, event.real_width as i32, event.real_height as i32);
                        let _ = host_move_tx_cb.send(target_rect);
                    } else {
                        println!("[IGNORE] Resize for non-pinned/non-host HWND: {}", event.hwnd);
                    }
                }
            }).unwrap();
            client.set_move_callback({
                let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress);
                let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress);
                let host_move_tx_cb = Arc::clone(&host_move_tx_cb);
                move |event| {
                    if programmatic_move_in_progress_cb.load(Ordering::SeqCst) || programmatic_resize_in_progress_cb.load(Ordering::SeqCst) {
                        println!("[SUPPRESS] programmatic_move_in_progress or programmatic_resize_in_progress=true, skipping move callback for HWND: {}", event.hwnd);
                        return;
                    }
                    println!("🔥 [DEBUG] Move callback triggered! HWND: {}, rect=({}, {}, {}, {})", event.hwnd, event.real_x, event.real_y, event.real_width, event.real_height);
                    let pinned_hwnd = PINNED_HWND_MAP.get("pinned").map(|v| *v as u64);
                    let host_hwnd = grid_manager.host_hwnd.map(|v| v as u64);
                    if Some(event.hwnd) == pinned_hwnd {
                        // Track when Chrome (pinned HWND) is first moved to grid position (inside or equal to grid rect)
                        if !pinned_layout_complete.load(Ordering::SeqCst) {
                            println!("[DIAG] Chrome HWND move event for pinned HWND. Setting pinned_layout_complete=true");
                            pinned_layout_complete.store(true, Ordering::SeqCst);
                        }
                        use std::sync::atomic::AtomicU64;
                        use std::time::{SystemTime, UNIX_EPOCH};
                        static LAST_MOVE_SEND_MS: once_cell::sync::Lazy<AtomicU64> = once_cell::sync::Lazy::new(|| AtomicU64::new(0));
                        if pinned_layout_complete.load(Ordering::SeqCst) {
                            let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                            let last_ms = LAST_MOVE_SEND_MS.load(Ordering::SeqCst);
                            if now_ms >= last_ms + 16 {
                                LAST_MOVE_SEND_MS.store(now_ms, Ordering::SeqCst);
                                let target_rect = (event.real_x, event.real_y, event.real_width as i32, event.real_height as i32);
                                let send_result = host_move_tx_cb.send(target_rect);
                                println!("[DIAG] Sent move_host_to_maintain_grid_alignment request to main thread: {:?}, send_result={:?}", target_rect, send_result);
                            } else {
                                println!("[DIAG] Debounced move request, skipping ({}ms since last)", now_ms - last_ms);
                            }
                        } else {
                            println!("[DIAG] pinned_layout_complete is false, not sending move request");
                        }
                    } else if Some(event.hwnd) == host_hwnd {
                        println!("[DEBUG] Move callback for host HWND: {}", event.hwnd);
                    } else {
                        println!("[IGNORE] Move callback for non-pinned/non-host HWND: {}", event.hwnd);
                    }
                }
            }).unwrap();
            let midi_sender_stop = Arc::clone(&midi_sender);
            let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress);
            let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress);
            client.set_move_resize_stop_callback({
                let programmatic_move_in_progress_cb = Arc::clone(&programmatic_move_in_progress_cb);
                let programmatic_resize_in_progress_cb = Arc::clone(&programmatic_resize_in_progress_cb);
                move |event| {
                    // if programmatic_move_in_progress_cb.load(Ordering::SeqCst) || programmatic_resize_in_progress_cb.load(Ordering::SeqCst) {
                    //     println!("[SUPPRESS] programmatic_move_in_progress or programmatic_resize_in_progress=true, skipping move/resize STOP callback for HWND: {}", event.hwnd);
                    //     return;
                    // }
                    programmatic_move_in_progress_cb.store(false, Ordering::SeqCst);
                    programmatic_resize_in_progress_cb.store(false, Ordering::SeqCst);
                    println!("🔥 [DEBUG] Move/resize STOP callback triggered! HWND: {}", event.hwnd);
                    let pinned_hwnd = PINNED_HWND_MAP.get("pinned").map(|v| *v as u64);
                    let host_hwnd = grid_manager.host_hwnd.map(|v| v as u64);
                    if Some(event.hwnd) == pinned_hwnd {
                        let _ = midi_sender_stop.send(e_midi::MidiCommand::Stop);
                        let _ = host_move_tx.send((-1, -1, -1, -1)); // Use (-1, -1, -1, -1) as a signal for alignment
                        println!("[STOP] Sent alignment signal to UI thread for pinned HWND");
                    } else if Some(event.hwnd) == host_hwnd {
                        let _ = midi_sender_stop.send(e_midi::MidiCommand::Stop);
                        let _ = host_move_tx.send((-1, -1, -1, -1));
                        println!("[STOP] Sent alignment signal to UI thread for host HWND");
                    } else {
                        println!("[IGNORE] Move/resize STOP for non-pinned/non-host HWND: {}", event.hwnd);
                    }
                }
            }).unwrap();
            client.start_background_monitoring().unwrap();
        }
        let (zorder_toggle_tx, zorder_toggle_rx) = std::sync::mpsc::channel();
        let zorder_delay_ms = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1000));
        Self {
            script_tempfile: Some(script_tempfile),
            // ...existing code...
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
            midi_player,
            midi_sender,
            song_map,
            next_song,
            grid_client,
            host_move_rx: Some(host_move_rx),
            focus_request_rx: Some(focus_request_rx),
            programmatic_move_in_progress,
            programmatic_resize_in_progress,
            grid_offset_x: 0,
            grid_offset_y: 0,
            zorder_toggle_tx: Some(zorder_toggle_tx),
            zorder_toggle_rx: Some(zorder_toggle_rx),
            zorder_thread_running: false,
            zorder_state: true,
            zorder_delay_ms,
        }
    }
}

// Implement eframe::App for EGridDemoApp
#[cfg(target_os = "windows")]
impl eframe::App for EGridDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- ASYNC Z-ORDER TOGGLE CHANNEL ---
        // All state is now in self
        egui::CentralPanel::default().show(ctx, |ui| {
            // --- TOP: Chrome/MIDI info ---
            ui.heading("EGridDemoApp (Chrome Grid + MIDI)");
            if let Some(win) = CHROME_WINDOW_INFO_MAP.get("chrome") {
                ui.label(format!("Chrome HWND: {:?}", win.hwnd));
                ui.label(format!("Chrome PID: {:?}", win.pid));
                ui.label(format!("Title: {:?}", win.title));
                ui.label(format!("Target: {:?}", win.target));
                ui.label(format!("Page URL: {:?}", win.page_url));
            }
            ui.separator();
            ui.label(format!("MIDI Songs Loaded: {}", self.midi_player.get_total_song_count()));
            ui.label(format!("Next Song Index: {}", self.next_song.load(Ordering::SeqCst)));
            ui.separator();
            if let Some(rx) = &self.chrome_output_rx {
                while let Ok(msg) = rx.try_recv() {
                    ui.label(msg);
                }
            }

            // --- Z ORDER CHECKBOX ---
ctx.memory_mut(|mem| {
    if mem.data.get_temp::<bool>("pinned_above_host".into()).is_none() {
        mem.data.insert_temp("pinned_above_host".into(), true);
    }
});
let mut pinned_above_host = ctx.memory(|mem| mem.data.get_temp::<bool>("pinned_above_host".into()).unwrap_or(true));
if ui.checkbox(&mut pinned_above_host, "Pinned window above host").changed() {
    ctx.memory_mut(|mem| mem.data.insert_temp("pinned_above_host".into(), pinned_above_host));
    if let Some(hwnd_val) = CHROME_WINDOW_INFO_MAP.get("chrome").and_then(|win| win.hwnd) {
        let hwnd_win = hwnd_val as HWND;
        if PositionGridManager::is_window(hwnd_win) {
            let host_hwnd = self.grid_manager.host_hwnd.map(|v| v as HWND);
            if let Some(host_hwnd) = host_hwnd {
                let _ = self.grid_manager.set_pinned_zorder(hwnd_win, pinned_above_host);
                println!("[EGridDemo] Set pinned HWND=0x{:X} z-order above host: {}", hwnd_val, pinned_above_host);
            }
        }
    }
}

            // --- GRID OFFSET SLIDERS & RANDOM DELAY SLIDER ---
            ui.horizontal(|ui| {
                ui.label("Grid Offset X:");
                ui.add(egui::Slider::new(&mut self.grid_offset_x, -5000..=5000).text("px"));
                ui.label("Grid Offset Y:");
                ui.add(egui::Slider::new(&mut self.grid_offset_y, -5000..=5000).text("px"));
            });
            // Add a slider for random delay (milliseconds)
            if !ctx.memory(|mem| mem.data.get_temp::<i32>(egui::Id::new("random_delay_ms")).is_some()) {
                ctx.memory_mut(|mem| mem.data.insert_temp(egui::Id::new("random_delay_ms"), 1000));
            }
            let mut random_delay_ms = ctx.memory(|mem| mem.data.get_temp::<i32>(egui::Id::new("random_delay_ms")).unwrap_or(1000));
            ui.horizontal(|ui| {
                ui.label("Random Delay (ms):");
                if ui.add(egui::Slider::new(&mut random_delay_ms, 0..=30000).text("ms")).changed() {
                    ctx.memory_mut(|mem| mem.data.insert_temp(egui::Id::new("random_delay_ms"), random_delay_ms));
                    self.zorder_delay_ms.store(random_delay_ms as u64, std::sync::atomic::Ordering::Relaxed);
                }
            });


            // --- GRID DIAGNOSTICS ---
            let label_height = 32.0;
            // Get host window rect
            let host_rect = self.grid_manager.get_host_screen_rect().unwrap_or((0, 0, 800, 600));
            // Apply grid offset from sliders
            let grid_rect = (
                host_rect.0 + self.grid_offset_x,
                host_rect.1 + self.grid_offset_y,
                host_rect.2,
                host_rect.3
            );
            let (mut new_grid, _char_size) = PositionGrid::from_text_style(self.eframe_hwnd, ui, egui::TextStyle::Heading, egui::Color32::LIGHT_GREEN, None);
            new_grid.rect = egui::Rect::from_min_size(
                egui::pos2(grid_rect.0 as f32, grid_rect.1 as f32),
                egui::vec2(grid_rect.2 as f32, grid_rect.3 as f32)
            );
            self.fill_grid = new_grid;
            self.grid_manager.grid = Some(&self.fill_grid as *const PositionGrid);

            // Send grid coordinates to Chrome launch thread if not launched yet
            if let Some(tx) = &self.chrome_launch_request_tx {
                if self.chrome_spawned.load(std::sync::atomic::Ordering::SeqCst) == false {
                    let (grid_x, grid_y, grid_w, grid_h) = grid_rect;
                    let _ = tx.send((grid_x, grid_y, grid_w, grid_h));
                    println!("[EGridDemo] Sent grid coordinates to Chrome launch thread: x={}, y={}, w={}, h={}", grid_x, grid_y, grid_w, grid_h);
                    self.chrome_spawned.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }

            ui.label(format!(
                "Grid: cells={} | grid_rect(screen)=({}, {}, {}, {}) | host_rect(screen)=({}, {}, {}, {}) | grid_offset=({}, {}) | host_hwnd={:?}",
                self.fill_grid.cell_count(),
                grid_rect.0, grid_rect.1, grid_rect.2, grid_rect.3,
                host_rect.0, host_rect.1, host_rect.2, host_rect.3,
                self.grid_offset_x, self.grid_offset_y,
                self.grid_manager.host_hwnd
            ));
            // --- GRID DRAW ---
            self.fill_grid.rect = egui::Rect::from_min_size(
                egui::pos2(grid_rect.0 as f32, grid_rect.1 as f32),
                egui::vec2(grid_rect.2 as f32, grid_rect.3 as f32)
            );
            self.fill_grid.draw(ui);

            // --- MOVE PINNED HWND TO GRID ---
            if let Some(hwnd_val) = CHROME_WINDOW_INFO_MAP.get("chrome").and_then(|win| win.hwnd) {
                let hwnd_win = hwnd_val as HWND;
                if PositionGridManager::is_window(hwnd_win) {
                    // Check if host window is minimized before moving pinned window
                    let host_hwnd = self.grid_manager.host_hwnd.map(|v| v as HWND);
                    let mut host_minimized = false;
                    #[cfg(target_os = "windows")]
                    if let Some(host_hwnd) = host_hwnd {
                        use winapi::um::winuser::IsIconic;
                        unsafe {
                            host_minimized = IsIconic(host_hwnd) != 0;
                        }
                    }
                    if host_minimized {
                        println!("[EGridDemo] Host window is minimized, skipping pinned HWND move.");
                    } else {
                        self.grid_manager.begin_programmatic_move();
                        let start_row = 0;
                        let start_col = 0;
                        let end_row = self.fill_grid.rows().saturating_sub(1);
                        let end_col = self.fill_grid.cols().saturating_sub(1);
                        if let (Some(start_rect), Some(end_rect)) = (
                            self.fill_grid.cell_rect_screen(start_row, start_col),
                            self.fill_grid.cell_rect_screen(end_row, end_col)
                        ) {
                            let left = start_rect.left().min(end_rect.left()) as i32;
                            let top = start_rect.top().min(end_rect.top()) as i32;
                            let right = start_rect.right().max(end_rect.right()) as i32;
                            let bottom = start_rect.bottom().max(end_rect.bottom()) as i32;
                            let width = right - left;
                            let height = bottom - top;
                            let current_rect = PositionGridManager::get_window_rect(hwnd_win);
                            if current_rect.0 != left || current_rect.1 != top || (current_rect.2-current_rect.0) != width || (current_rect.3-current_rect.1) != height {
                                let moved = PositionGridManager::move_window(
                                    hwnd_win,
                                    left,
                                    top,
                                    width,
                                    height
                                );
                                println!(
                                    "[EGridDemo] Moved pinned HWND=0x{:X} to cell_rect_screen=({}, {}, {}, {}), result={}",
                                    hwnd_val,
                                    left, top, width, height, moved
                                );
                            }
                        }
                        self.grid_manager.end_programmatic_move();
                        // --- ASYNC Z-ORDER LOGIC ---
                        let use_checkbox = ctx.memory(|mem| mem.data.get_temp::<bool>("pinned_above_host".into()).unwrap_or(true));
                        if use_checkbox {
                            // Use checkbox value directly
                            self.zorder_state = true;
                        } else {
                            // Use async channel for Z-order alternation
                            if !self.zorder_thread_running {
                                if let Some(tx) = &self.zorder_toggle_tx {
                                    let tx_clone = tx.clone();
                                    let delay_atomic = self.zorder_delay_ms.clone();
                                    std::thread::spawn(move || {
                                        loop {
                                            let delay = delay_atomic.load(std::sync::atomic::Ordering::Relaxed);
                                            std::thread::sleep(std::time::Duration::from_millis(delay));
                                            let _ = tx_clone.send(true); // signal toggle
                                        }
                                    });
                                    self.zorder_thread_running = true;
                                }
                            }
                            if let Some(rx) = &self.zorder_toggle_rx {
                                if let Ok(_msg) = rx.try_recv() {
                                    self.zorder_state = !self.zorder_state;
                                }
                            }
                        }
                        // Actually set Z-order: true = pinned above host, false = host above pinned
                        if let Some(host_hwnd) = self.grid_manager.host_hwnd {
                            use winapi::um::winuser::SetForegroundWindow;
                            static mut LAST_ZORDER_PRINTED: Option<bool> = None;
                            let zorder_state = self.zorder_state;
                            let should_print = unsafe {
                                if LAST_ZORDER_PRINTED != Some(zorder_state) {
                                    LAST_ZORDER_PRINTED = Some(zorder_state);
                                    true
                                } else {
                                    false
                                }
                            };
                            if zorder_state {
                                let _ = PositionGridManager::set_zorder_above(hwnd_win, host_hwnd as HWND);
                                if should_print {
                                    println!("[EGridDemo] Set pinned HWND=0x{:X} above host HWND=0x{:X}", hwnd_val, host_hwnd);
                                }
                                unsafe { SetForegroundWindow(hwnd_win); }
                            } else {
                                let _ = PositionGridManager::set_zorder_above(host_hwnd as HWND, hwnd_win);
                                if should_print {
                                    println!("[EGridDemo] Set host HWND=0x{:X} above pinned HWND=0x{:X}", host_hwnd, hwnd_val);
                                }
                                unsafe { SetForegroundWindow(host_hwnd as HWND); }
                            }
                        }
                    }
                }
            }

            // --- BOTTOM DIAGNOSTICS ---
            {
                let hwnd_val_opt = CHROME_WINDOW_INFO_MAP.get("chrome").and_then(|win| win.hwnd);
                if let Some(hwnd_val) = hwnd_val_opt {
                    ui.label(format!("Pinned HWND: 0x{:X}", hwnd_val));
                    // Check if pinned HWND is still valid
                    let hwnd_win = hwnd_val as HWND;
                    let is_valid = PositionGridManager::is_window(hwnd_win);
                    ui.label(format!("Pinned HWND valid: {}", is_valid));
                    if !is_valid {
                        println!("[EGridDemo] Pinned HWND is closed. Exiting app.");
                        std::process::exit(0);
                    }
                } else {
                    ui.label("Waiting for Chrome HWND...");
                }
                if let Some(pid) = self.chrome_pid {
                    ui.label(format!("Chrome PID: {}", pid));
                }
                // Show pinning status from channel
                if let Some(rx) = &self.pin_request_rx {
                    use winapi::um::winuser::{GetWindowRect, IsZoomed, SW_RESTORE, ShowWindow, GetWindowLongW, GWL_STYLE, GWL_EXSTYLE};
                    for (hwnd, rect) in rx.try_iter() {
                        ui.label(format!("Pin request: HWND=0x{:X}, rect={:?}", hwnd, rect));
                        let mut info = CHROME_WINDOW_INFO_MAP.get_mut("chrome").map(|mut_ref| mut_ref.clone()).unwrap_or_default();
                        info.hwnd = Some(hwnd);
                        CHROME_WINDOW_INFO_MAP.insert("chrome", info);
                        self.last_pinned_rect = Some(rect);
                        let hwnd_win = hwnd as HWND;
                        if PositionGridManager::is_window(hwnd_win) {
                            // Print window rect before move
                            let mut before_rect = unsafe { std::mem::zeroed() };
                            let got_rect = unsafe { GetWindowRect(hwnd_win, &mut before_rect) };
                            println!("[DIAG] Before move: HWND=0x{:X}, got_rect={}, rect=({}, {}, {}, {})", hwnd, got_rect, before_rect.left, before_rect.top, before_rect.right, before_rect.bottom);

                            // Print window styles
                            let style = unsafe { GetWindowLongW(hwnd_win, GWL_STYLE) };
                            let exstyle = unsafe { GetWindowLongW(hwnd_win, GWL_EXSTYLE) };
                            println!("[DIAG] Window styles: style=0x{:X}, exstyle=0x{:X}", style, exstyle);

                            // Print grid rect and host rect
                            let grid_rect = self.grid_manager.get_grid_screen_rect();
                            let host_rect = self.grid_manager.get_host_screen_rect();
                            println!("[DIAG] grid_rect={:?}, host_rect={:?}", grid_rect, host_rect);

                            // Print DPI
                            let dpi = PositionGridManager::get_dpi_for_window(hwnd_win);
                            println!("[DIAG] DPI for HWND=0x{:X}: {}", hwnd, dpi);

                            // Check if window is maximized
                            let is_max = unsafe { IsZoomed(hwnd_win) } != 0;
                            println!("[DIAG] IsZoomed (maximized) for HWND=0x{:X}: {}", hwnd, is_max);
                            if is_max {
                                let restore_res = unsafe { ShowWindow(hwnd_win, SW_RESTORE) };
                                println!("[DIAG] Restored window from maximized state, ShowWindow result={}", restore_res);
                            }

                            // Move Chrome window to grid position
                            let moved = self.grid_manager.move_and_resize(hwnd_win);
                            println!("[DIAG] grid_manager.move_and_resize result for HWND=0x{:X}: {}", hwnd, moved);
                            if !moved {
                                println!("[DIAG] Failed to move/resize pinned HWND=0x{:X}. Is window valid? {}", hwnd, PositionGridManager::is_window(hwnd_win));
                            }

                            // Print window rect after move
                            let mut after_rect = unsafe { std::mem::zeroed() };
                            let got_rect2 = unsafe { GetWindowRect(hwnd_win, &mut after_rect) };
                            println!("[DIAG] After move: HWND=0x{:X}, got_rect={}, rect=({}, {}, {}, {})", hwnd, got_rect2, after_rect.left, after_rect.top, after_rect.right, after_rect.bottom);

                            // Set topmost
                            let topmost_res = PositionGridManager::set_topmost(hwnd_win, true);
                            println!("[DIAG] grid_manager.set_topmost result for HWND=0x{:X}: {}", hwnd, topmost_res);

                            // Set Z-order above host
                            if let Some(host_hwnd) = self.grid_manager.host_hwnd {
                                let zorder_res = PositionGridManager::set_zorder_above(hwnd_win, host_hwnd as HWND);
                                println!("[DIAG] grid_manager.set_zorder_above result for HWND=0x{:X} above host HWND=0x{:X}: {}", hwnd, host_hwnd, zorder_res);
                            }
                        } else {
                            println!("[EGridDemo] Target HWND is invalid or closed. Exiting app.");
                            std::process::exit(0);
                        }
                    }
                }

                // Move Chrome window to grid position only if HWND is valid
                // ...moved move_and_resize to pin_request_rx handler above...
                // Mouse click requests
                if let Some(rx) = &self.click_request_rx {
                    for (cell_x, cell_y) in rx.try_iter() {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        let result = self.fill_grid.send_mouse_click_to_cell(cell_x, cell_y);
                        println!(
                            "[EGridDemo] Mouse click requested at cell ({}, {}) for HWND=0x{:?}, result={:?}",
                            cell_x, cell_y, self.eframe_hwnd, result
                        );
                        unsafe {
                            use winapi::shared::windef::POINT;
                            use winapi::um::winuser::GetCursorPos;
                            let mut pt: POINT = std::mem::zeroed();
                            if GetCursorPos(&mut pt) != 0 {
                                println!("[EGridDemo] Mouse pointer is now at: x={}, y={}", pt.x, pt.y);
                            } else {
                                println!("[EGridDemo] Failed to get mouse pointer position");
                            }
                        }
                    }
                }
                // Host move requests (main thread)
                if let Some(rx) = &mut self.host_move_rx {
                    for target_rect in rx.try_iter() {
                        // Suppress feedback loop if move was programmatic
                        if self.grid_manager.is_programmatic_move() {
                            println!("[SUPPRESS] Programmatic move detected, ignoring host_move_rx event: {:?}", target_rect);
                            self.grid_manager.end_programmatic_move();
                            continue;
                        }
                        if target_rect == (-1, -1, -1, -1) {
                            // Explicit grid alignment signal
                            println!("[MAIN] Received alignment signal, aligning host and pinned windows to grid rect");
                            self.grid_manager.begin_programmatic_move();
                            // Only move pinned window to grid anchor, do NOT move host above pinned
                            if let Some(pinned_hwnd) = PINNED_HWND_MAP.get("pinned") {
                                let pinned_hwnd = *pinned_hwnd;
                                let hwnd_win = pinned_hwnd as HWND;
                                // Get pinned window's current size
                                let pinned_rect = PositionGridManager::get_window_rect(hwnd_win);
                                let width = pinned_rect.2 - pinned_rect.0;
                                let height = pinned_rect.3 - pinned_rect.1;
                                // Get the grid cell's top-left in screen coordinates (anchor)
                                let grid_rect = self.grid_manager.get_grid_screen_rect().unwrap_or((0, 0, 0, 0));
                                let grid_anchor_x = grid_rect.0;
                                let grid_anchor_y = grid_rect.1;
                                // Move pinned window to grid anchor
                                let moved = PositionGridManager::move_window(hwnd_win, grid_anchor_x, grid_anchor_y, width, height);
                                println!("[MAIN] Anchored pinned HWND=0x{:X} to grid cell top-left=({}, {}), size=({}, {}), result={}", pinned_hwnd, grid_anchor_x, grid_anchor_y, width, height, moved);
                            }
                            self.grid_manager.end_programmatic_move();
                        } else {
                            // Disabled: Do NOT move host to match pinned window's rect
                            println!("[MAIN] Received pinned window move: {:?}, ignoring host move (focus on pinned alignment)", target_rect);
                            // ...do nothing, just log...
                        }
                    }
                }

                // Host focus requests (main thread)
                if let Some(rx) = &mut self.focus_request_rx {
                    for _ in rx.try_iter() {
                        let host_rect = self.grid_manager.get_host_screen_rect();
                        let grid_rect = self.grid_manager.get_grid_screen_rect();
                        println!("[MAIN] Received host focus event. host_rect={:?}, grid_rect={:?}", host_rect, grid_rect);
                        if let Some(pinned_hwnd) = PINNED_HWND_MAP.get("pinned") {
                            let pinned_val = *pinned_hwnd;
                            println!("[MAIN] Setting pinned HWND=0x{:X} topmost due to host focus", pinned_val);
                            let _ = PositionGridManager::set_topmost(pinned_val as HWND, true);
                            if let Some(host_hwnd) = self.grid_manager.host_hwnd {
                                 let _ = PositionGridManager::set_zorder_above(pinned_val as HWND, host_hwnd as HWND);
                            }
                        }
                    }
                }
            }
        });
        ctx.request_repaint(); // keep UI responsive
    }
}

// ...existing code for Default and eframe::App implementation...

#[cfg(target_os = "windows")]
impl Drop for EGridDemoApp {
    fn drop(&mut self) {
        // Send WM_CLOSE to all pinned HWNs in PINNED_HWND_MAP
        unsafe {
            use winapi::um::winuser::{PostMessageW, WM_CLOSE};
            for entry in PINNED_HWND_MAP.iter() {
                let hwnd_val = *entry.value();
                let hwnd = hwnd_val as winapi::shared::windef::HWND;
                let res = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                println!("[EGridDemo] Sent WM_CLOSE to pinned HWND key='{}' HWND=0x{:X}, result={}", entry.key(), hwnd_val, res);
            }
        }
        // Stop MIDI playback
        let _ = self.midi_sender.send(e_midi::MidiCommand::Stop);
        println!("[EGridDemo] App exiting, cleaned up Chrome and MIDI.");
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("Sorry, e_window_e_grid is only supported on Windows targets at this time.");
    std::process::exit(1);
}

// In update(), add diagnostics and MIDI status, and ensure Chrome HWND is registered with grid_client for event monitoring.
// On Drop, stop MIDI and clean up Chrome.
#[cfg(target_os = "windows")]
fn main() {

    println!("WARNING: This example is being released with known bugs!");
    println!("Do NOT run unless you are OK with losing all your window positions, or having windows close unexpectedly.");
    println!("Windows may be moved or closed without warning. Use at your own risk.");
    println!("If your windows disappear, use Alt+Space, M, then arrow keys to reposition them back into view.");
    println!("This example is intended for testing and debugging purposes only.");
    println!("good luck or wait for the next release.");
    println!();
    println!("Press Ctrl-C if you do NOT want to run this example.");
    print!("Press Enter to continue...");
    io::Write::flush(&mut io::stdout()).unwrap();
    let _ = io::stdin().read_line(&mut String::new());

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::sync::{Arc, Mutex};
        let window_info = Arc::new(Mutex::new(ChromeWindowInfo::default()));
        let window_info_ctrlc = Arc::clone(&window_info);
        ctrlc::set_handler(move || {
            // Send WM_CLOSE to all pinned HWNDs before exit
            unsafe {
                use winapi::um::winuser::{PostMessageW, WM_CLOSE};
                for entry in PINNED_HWND_MAP.iter() {
                    let hwnd_val = *entry.value();
                    let hwnd = hwnd_val as winapi::shared::windef::HWND;
                    let res = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    println!("[EGridDemo] Ctrl-C: Sent WM_CLOSE to pinned HWND key='{}' HWND=0x{:X}, result={}", entry.key(), hwnd_val, res);
                }
            }
            let win = window_info_ctrlc.lock().unwrap();
            if let Some(pid) = win.pid {
                println!("[EGridDemo] Ctrl-C pressed, killing Chrome PID: {}", pid);
                let _ = Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .status();
            }
            std::process::exit(0);
        }).expect("Error setting Ctrl-C handler");
        let options = eframe::NativeOptions::default();
        let _ = eframe::run_native(
            "e_window_e_grid",
            options,
            Box::new(|cc| {
                let mut hwnd_opt = None;
                #[cfg(target_os = "windows")]
                {
                    use winit::raw_window_handle::RawWindowHandle;
                    use winit::raw_window_handle::HasWindowHandle;


                    let raw = cc.window_handle().unwrap().as_raw();
                    if let RawWindowHandle::Win32(handle) = raw {
                        let hwnd = handle_to_hwnd(handle);
                        hwnd_opt = Some(hwnd.0 as u32);
                    }
                }
                Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                    EGridDemoApp::with_hwnd(hwnd_opt),
                ))
            }),
        );
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("Sorry, e_window_e_grid is only supported on Windows targets at this time.");
    }
}

// Helper to convert Win32WindowHandle to HWND
#[cfg(target_os = "windows")]
fn handle_to_hwnd(
    handle: winit::raw_window_handle::Win32WindowHandle,
) -> windows::Win32::Foundation::HWND {
    let hwnd_isize: isize = handle.hwnd.into();
    let hwnd = hwnd_isize as *mut core::ffi::c_void;
    windows::Win32::Foundation::HWND(hwnd)
}


