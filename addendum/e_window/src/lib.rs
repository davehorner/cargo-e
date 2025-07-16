//! Library interface for launching the e_window app with custom arguments.
pub mod app;
pub mod control;
pub mod parser;
pub mod pool_manager;
pub mod position_grid;
pub mod position_grid_manager;

use getargs::{Arg, Options};
use std::env::current_exe;
use std::fs;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Run the e_window app with the given arguments (excluding program name).
pub fn run_window<I, S>(args: I) -> eframe::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect::<Vec<_>>();
    let mut opts = Options::new(args.iter().map(String::as_str));

    // Defaults
    let mut title = "E Window".to_string();
    let mut appname = String::new();
    let mut width = 800u32;
    let mut height = 600u32;
    let mut x = 100i32;
    let mut y = 100i32;
    let mut input_file: Option<String> = None;
    let mut follow_hwnd: Option<usize> = None;
    let mut positional_args = Vec::new();

    // New pool options
    let mut w_pool_cnt: Option<usize> = None;
    let mut w_pool_ndx: Option<usize> = None;
    let mut w_pool_rate: Option<u64> = None;

    // Parent PID for child windows
    let mut parent_pid: Option<u32> = None;

    // Decode debug flag
    let mut decode_debug = false;
    let mut has_been_specified = false;
    while let Some(arg) = opts.next_arg().expect("argument parsing error") {
        match arg {
            Arg::Long("title") => {
                if let Ok(val) = opts.value() {
                    title = val.to_string();
                    has_been_specified = true;
                }
            }
            Arg::Long("width") => {
                if let Ok(val) = opts.value() {
                    width = val.parse().unwrap_or(width);
                    has_been_specified = true;
                }
            }
            Arg::Long("height") => {
                if let Ok(val) = opts.value() {
                    height = val.parse().unwrap_or(height);
                    has_been_specified = true;
                }
            }
            Arg::Long("x") => {
                if let Ok(val) = opts.value() {
                    x = val.parse().unwrap_or(x);
                    has_been_specified = true;
                }
            }
            Arg::Long("y") => {
                if let Ok(val) = opts.value() {
                    y = val.parse().unwrap_or(y);
                    has_been_specified = true;
                }
            }
            Arg::Long("appname") => {
                if let Ok(val) = opts.value() {
                    appname = val.to_string();
                    has_been_specified = true;
                }
            }
            Arg::Short('i') | Arg::Long("input-file") => {
                if let Ok(val) = opts.value() {
                    input_file = Some(val.to_string());
                    has_been_specified = true;
                }
            }
            Arg::Long("follow-hwnd") => {
                if let Ok(val) = opts.value() {
                    // Accept both decimal and hex (with 0x prefix)
                    follow_hwnd = if let Some(stripped) = val.strip_prefix("0x") {
                        usize::from_str_radix(stripped, 16).ok()
                    } else {
                        val.parse().ok()
                    };
                    has_been_specified = true;
                }
            }
            Arg::Long("w-pool-cnt") => {
                if let Ok(val) = opts.value() {
                    w_pool_cnt = val.parse().ok();
                    has_been_specified = true;
                }
            }
            Arg::Long("w-pool-ndx") => {
                if let Ok(val) = opts.value() {
                    w_pool_ndx = val.parse().ok();
                    has_been_specified = true;
                }
            }
            Arg::Long("w-pool-rate") => {
                if let Ok(val) = opts.value() {
                    w_pool_rate = val.parse().ok();
                    has_been_specified = true;
                }
            }
            Arg::Long("parent-pid") => {
                if let Ok(val) = opts.value() {
                    parent_pid = val.parse().ok();
                    has_been_specified = true;
                }
            }
            Arg::Long("decode-debug") => {
                decode_debug = true;
                has_been_specified = true;
            }
            Arg::Short('h') | Arg::Long("help") => {
                eprintln!(
                    r#"Usage: e_window [OPTIONS] [FILES...]
    --appname <NAME>     Set app name (default: executable name)
    --title <TITLE>      Set window title (default: "E Window")
    --width <WIDTH>      Set window width (default: 800)
    --height <HEIGHT>    Set window height (default: 600)
    --x <X>              Set window X position (default: 100)
    --y <Y>              Set window Y position (default: 100)
    -i, --input-file <FILE>  Read input data from file
    --follow-hwnd <HWND> Follow HWND (default: None)
    --w-pool-cnt <N>     Keep at least N windows open at all times
    --w-pool-ndx <N>     (internal) Index of this window instance
    --w-pool-rate <MS>   Minimum milliseconds between opening new windows (default: 1000)
    --decode-debug       Enable debug decoding mode
    -h, --help           Show this help and exit
Any other positional arguments are collected as files or piped input."#
                );
                return Ok(());
            }
            Arg::Positional(val) => {
                positional_args.push(val.to_string());
            }
            Arg::Short(_) | Arg::Long(_) => {
                // Ignore unknown flags for now
            }
        }
    }

    // Default appname to executable name (without extension) if not set
    if appname.is_empty() {
        appname = current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "e_window".to_string());
    }

    // Set up control channel once, share between stdin thread and app
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();

    // Read input data: from file if specified, else from positional args, else empty
    let (input_data, mut editor_mode) = if let Some(file) = input_file {
        (
            fs::read_to_string(file).unwrap_or_else(|_| "".to_string()),
            false,
        )
    } else if !positional_args.is_empty() {
        // If the first positional argument looks like a file and exists, use it as a file
        let first = &positional_args[0];
        if fs::metadata(first).is_ok() {
            (
                fs::read_to_string(first).unwrap_or_else(|_| "".to_string()),
                false,
            )
        } else {
            // Otherwise, treat it as the card content
            (first.clone(), false)
        }
    } else {
        // Buffer initial lines from stdin asynchronously and use them as input_data
        use std::sync::{Arc, Mutex};
        let initial_buffer = Arc::new(Mutex::new(Vec::new()));
        let initial_buffer_clone = initial_buffer.clone();
        // Start the stdin listener, but also buffer the first lines
        control::start_stdin_listener_with_buffer(tx.clone(), initial_buffer_clone);
        // Wait briefly for initial input (event-driven, but with a short timeout)
        let mut waited = 0;
        let max_wait = 200; // ms
        while waited < max_wait {
            if !initial_buffer.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            waited += 10;
        }
        let input_data = initial_buffer.lock().unwrap().join("\n");
        (input_data, false)
    };

    // If input_data is empty, use your DEFAULT_CARD
    let input_data = if input_data.trim().is_empty() {
        println!("Warning: No input data provided, using default card template.");
        let hwnd = {
            #[cfg(target_os = "windows")]
            {
                unsafe { winapi::um::winuser::GetForegroundWindow() as usize }
            }
            #[cfg(not(target_os = "windows"))]
            {
                0
            }
        };
        editor_mode = true; // Set editor mode if using default card
        app::default_card_with_hwnd(hwnd)
    } else {
        input_data
    };

    // Parse first line for CLI args, and use the rest as input_data
    let mut input_lines = input_data.lines();
    let mut actual_input = String::new();
    if let Some(first_line) = input_lines.next() {
        let input_args = shell_words::split(first_line).unwrap_or_default();
        if !input_args.is_empty() {
            let mut opts = Options::new(input_args.iter().map(String::as_str));
            while let Some(arg) = opts.next_arg().expect("argument parsing error") {
                match arg {
                    Arg::Long("follow-hwnd") => {
                        if let Ok(val) = opts.value() {
                            // Accept both decimal and hex (with 0x prefix)
                            follow_hwnd = if let Some(stripped) = val.strip_prefix("0x") {
                                usize::from_str_radix(stripped, 16).ok()
                            } else {
                                val.parse().ok()
                            };
                        }
                    }
                    Arg::Long("title") => {
                        if let Ok(val) = opts.value() {
                            title = val.to_string();
                        }
                    }
                    Arg::Long("width") => {
                        if let Ok(val) = opts.value() {
                            width = val.parse().unwrap_or(width);
                        }
                    }
                    Arg::Long("height") => {
                        if let Ok(val) = opts.value() {
                            height = val.parse().unwrap_or(height);
                        }
                    }
                    Arg::Long("x") => {
                        if let Ok(val) = opts.value() {
                            x = val.parse().unwrap_or(x);
                        }
                    }
                    Arg::Long("y") => {
                        if let Ok(val) = opts.value() {
                            y = val.parse().unwrap_or(y);
                        }
                    }
                    Arg::Long("appname") => {
                        if let Ok(val) = opts.value() {
                            appname = val.to_string();
                        }
                    }
                    Arg::Long("decode-debug") => {
                        println!(
                            "Warning: --decode-debug is deprecated, use --decode-debug instead."
                        );
                        decode_debug = true;
                        has_been_specified = true;
                    }
                    _ => {
                        // Ignore other flags for now
                        println!("Warning: Unknown argument: {:?}", arg);
                    }
                }
            }
        }
        // Use the rest of the lines as the actual input
        actual_input = input_lines.collect::<Vec<_>>().join("\n");
    }

    // --- Window pool logic ---
    if let Some(pool_size) = w_pool_cnt {
        // Monitor parent PID if this is a pool child
        if let (Some(pid), Some(_ndx)) = (parent_pid, w_pool_ndx) {
            std::thread::spawn(move || {
                use sysinfo::System;
                let mut sys = System::new_all();
                loop {
                    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                    if sys.process(sysinfo::Pid::from_u32(pid)).is_none() {
                        std::process::exit(0);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            });
        }

        // Only spawn the pool manager if this is NOT a child window and NOT already the pool manager
        if w_pool_ndx.is_none() && !args.iter().any(|a| a == "--w-pool-manager") {
            // Remove --w-pool-cnt and its value from args for child windows
            let mut child_args = args.clone();
            if let Some(idx) = child_args.iter().position(|a| a == "--w-pool-cnt") {
                child_args.drain(idx..=idx + 1);
            }
            // Remove any --w-pool-ndx from args (we'll add it per child)
            while let Some(idx) = child_args.iter().position(|a| a == "--w-pool-ndx") {
                child_args.drain(idx..=idx + 1);
            }
            // Remove --w-pool-rate and its value from args for child windows
            if let Some(idx) = child_args.iter().position(|a| a == "--w-pool-rate") {
                child_args.drain(idx..=idx + 1);
            }
            let exe = std::env::current_exe().expect("Failed to get current exe");
            let rate_ms = w_pool_rate.unwrap_or(1000);

            // Spawn the pool manager as a detached process and exit this process
            let mut cmd = std::process::Command::new(&exe);
            cmd.arg("--w-pool-manager")
                .arg("--parent-pid")
                .arg(std::process::id().to_string())
                .arg(format!("--w-pool-cnt={}", pool_size))
                .arg(format!("--w-pool-rate={}", rate_ms))
                .args(&child_args);

            let mut manager_process = cmd.spawn().expect("Failed to spawn pool manager");
            println!("e_window: Pool manager started...");

            // Wait for the pool manager to exit
            let status = manager_process
                .wait()
                .expect("Failed to wait on pool manager");
            println!("e_window: Pool manager exited with status: {}", status);

            return Ok(()); // Exit the original process
        }
    }

    // Pool manager logic (runs in a separate process)
    if args.iter().any(|a| a == "--w-pool-manager") {
        let pool_size = w_pool_cnt.unwrap_or(1);
        let rate_ms = w_pool_rate.unwrap_or(1000);

        // Spawn GUI for the pool manager
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 200.0])
                .with_title("e_window Pool Manager")
                .with_always_on_top(),
            ..Default::default()
        };

        // Spawn windows in a background thread as before
        let exe = std::env::current_exe().expect("Failed to get current exe");
        let mut child_args = args.clone();
        // Remove pool manager args as before...
        if let Some(idx) = child_args.iter().position(|a| a == "--w-pool-manager") {
            child_args.remove(idx);
        }
        if let Some(idx) = child_args.iter().position(|a| a == "--w-pool-cnt") {
            child_args.drain(idx..=idx + 1);
        }
        while let Some(idx) = child_args.iter().position(|a| a == "--w-pool-ndx") {
            child_args.drain(idx..=idx + 1);
        }
        if let Some(idx) = child_args.iter().position(|a| a == "--w-pool-rate") {
            child_args.drain(idx..=idx + 1);
        }

        let pool_manager = pool_manager::PoolManagerApp::new(pool_size, rate_ms);
        let pool_manager_thread = Arc::new(pool_manager);

        // Clone for thread
        let pool_manager_thread_clone = Arc::clone(&pool_manager_thread);

        std::thread::spawn(move || {
            let mut next_index = 1;
            loop {
                if pool_manager_thread_clone
                    .shutdown
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    break;
                }
                let count = count_running_windows(&exe);
                if count < pool_size {
                    let to_spawn = pool_size - count;
                    for _ in 0..to_spawn {
                        let mut args_with_index = child_args.clone();
                        args_with_index.push("--w-pool-ndx".to_string());
                        args_with_index.push(next_index.to_string());
                        args_with_index.push("--parent-pid".to_string());
                        args_with_index.push(std::process::id().to_string());
                        println!("Spawning: {:?} {:?}", exe, args_with_index);
                        // When you spawn a child:
                        if let Ok(child) = Command::new(&exe).args(&args_with_index).spawn() {
                            *pool_manager_thread_clone.spawned.lock().unwrap() += 1;
                            *pool_manager_thread_clone.last_spawn.lock().unwrap() = Instant::now();
                            pool_manager_thread_clone
                                .children
                                .lock()
                                .unwrap()
                                .push(child);
                        }
                        next_index += 1;
                        std::thread::sleep(Duration::from_millis(rate_ms));
                    }
                }
                std::thread::sleep(Duration::from_millis(rate_ms));
            }
        });

        // Run the pool manager GUI
        return eframe::run_native(
            "e_window Pool Manager",
            options,
            Box::new(move |_cc| {
                Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                    (*pool_manager_thread).clone(),
                ))
            }),
        );
    }

    // If you want to use the index in your window title:
    if let Some(ndx) = w_pool_ndx {
        title = format!("{} (Window #{})", title, ndx);
    }

    // Launch the GUI immediately, passing the shared receiver
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width as f32, height as f32])
            .with_position([x as f32, y as f32])
            .with_title(&title),
        ..Default::default()
    };
    // If actual_input is empty, use your DEFAULT_CARD
    let actual_input = if actual_input.trim().is_empty() {
        println!("Warning: No input data provided, using default card template.");
        let hwnd = {
            #[cfg(target_os = "windows")]
            {
                unsafe { winapi::um::winuser::GetForegroundWindow() as usize }
            }
            #[cfg(not(target_os = "windows"))]
            {
                0
            }
        };
        editor_mode = true; // Set editor mode if using default card
        app::default_card_with_hwnd(hwnd)
    } else {
        actual_input
    };
    eframe::run_native(
        &appname,
        options,
        Box::new(move |cc| {
            let app = app::App::with_initial_window(
                width as f32,
                height as f32,
                x as f32,
                y as f32,
                title.clone(),
                cc.storage,
                follow_hwnd,
                decode_debug,
            )
            .with_input_data_and_mode(actual_input, editor_mode);
            // Pass the receiver to the app
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(
                app.with_control_receiver(rx),
            ))
        }),
    )
}

// Helper: count running windows (processes) with our exe name
#[cfg(target_os = "windows")]
fn count_running_windows(_exe: &std::path::Path) -> usize {
    use std::ffi::OsString;

    use std::os::windows::ffi::OsStringExt;
    use sysinfo::System;
    use winapi::um::winuser::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    // Data struct to pass to callback
    struct EnumData<'a> {
        our_pids: &'a [u32],
        count: usize,
    }

    unsafe extern "system" fn enum_windows_proc(
        hwnd: winapi::shared::windef::HWND,
        lparam: winapi::shared::minwindef::LPARAM,
    ) -> i32 {
        let data = &mut *(lparam as *mut EnumData);
        let mut pid = 0u32;
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        GetWindowThreadProcessId(hwnd, &mut pid);
        if !data.our_pids.contains(&pid) {
            return 1;
        }
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            let title = OsString::from_wide(&buf[..len as usize])
                .to_string_lossy()
                .to_string();
            if title.contains("Window #") {
                data.count += 1;
            }
        }
        1
    }

    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    // Collect all process IDs for our exe
    let mut our_pids = Vec::new();
    for (pid, process) in sys.processes() {
        let name = process.name().to_ascii_lowercase();
        if name == "e_window.exe" || name == "e_window" {
            our_pids.push(pid.as_u32());
        }
    }

    let mut data = EnumData {
        our_pids: &our_pids,
        count: 0,
    };

    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut data as *mut _ as winapi::shared::minwindef::LPARAM,
        );
    }
    data.count
}

#[cfg(not(target_os = "windows"))]
fn count_running_windows(_exe: &std::path::Path) -> usize {
    #[cfg(not(target_os = "windows"))]
    use sysinfo::System;
    #[cfg(target_os = "windows")]
    use sysinfo::{ProcessExt, System, SystemExt};
    let mut sys = System::new_all();
    #[cfg(target_os = "windows")]
    sys.refresh_processes();
    #[cfg(not(target_os = "windows"))]
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut our_pids = Vec::new();
    // Collect all process IDs for our exe that are pool children
    for (pid, process) in sys.processes() {
        // Match exe name (case-insensitive) and check for --w-pool-ndx in cmdline
        let is_pool_child = process.cmd().iter().any(|arg| arg == "--w-pool-ndx");
        let exe_name = process.name().to_ascii_lowercase();
        if is_pool_child && (exe_name == "e_window.exe" || exe_name == "e_window") {
            our_pids.push(pid.as_u32());
        }
    }

    our_pids.len()
}
