/// Build an InjectEvent queue for orca file injection with rectangle and efficient movement
fn build_orca_inject_queue(file_path: &str) -> std::collections::VecDeque<e_window::uxn::InjectEvent> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::collections::VecDeque;
    use raven_varvara::Key;
    let mut queue = VecDeque::new();
    const CTRL_H: Key = Key::Ctrl;
    const RIGHT: Key = Key::Right;
    const LEFT: Key = Key::Left;
    const UP: Key = Key::Up;
    const DOWN: Key = Key::Down;
    // Read file into lines
    let mut lines: Vec<Vec<char>> = Vec::new();
    let mut max_len = 0;
    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let chars: Vec<char> = line.chars().collect();
            max_len = max_len.max(chars.len());
            lines.push(chars);
        }
    }
    let rows = lines.len();
    let cols = max_len;
    // Build rectangle with '/' border
    let mut grid = vec![vec![' '; cols + 2]; rows + 2];
    // Fill top and bottom borders
    for c in 0..cols + 2 {
        grid[0][c] = '/';
        grid[rows + 1][c] = '/';
    }
    // Fill left and right borders with '/' (actual border logic handled in event queue below)
    // Fill file contents
    for (i, line) in lines.iter().enumerate() {
        for (j, &ch) in line.iter().enumerate() {
            grid[i + 1][j + 1] = ch;
        }
    }
    // Start at (1,1)
    let mut cur_row = 1;
    let mut cur_col = 1;
    queue.push_back(e_window::uxn::InjectEvent::KeyPress(CTRL_H));
    queue.push_back(e_window::uxn::InjectEvent::KeyRelease(CTRL_H));
    // Visit all non '.' cells efficiently
    let mut visited = vec![vec![false; cols + 2]; rows + 2];
    for r in 0..rows + 2 {
        for c in 0..cols + 2 {
            if grid[r][c] != '.' && !visited[r][c] {
                // Move to (r,c)
                let dr = r as isize - cur_row as isize;
                let dc = c as isize - cur_col as isize;
                for _ in 0..dr.abs() {
                    queue.push_back(if dr > 0 {
                        e_window::uxn::InjectEvent::KeyPress(DOWN)
                    } else {
                        e_window::uxn::InjectEvent::KeyPress(UP)
                    });
                    queue.push_back(if dr > 0 {
                        e_window::uxn::InjectEvent::KeyRelease(DOWN)
                    } else {
                        e_window::uxn::InjectEvent::KeyRelease(UP)
                    });
                }
                for _ in 0..dc.abs() {
                    queue.push_back(if dc > 0 {
                        e_window::uxn::InjectEvent::KeyPress(RIGHT)
                    } else {
                        e_window::uxn::InjectEvent::KeyPress(LEFT)
                    });
                    queue.push_back(if dc > 0 {
                        e_window::uxn::InjectEvent::KeyRelease(RIGHT)
                    } else {
                        e_window::uxn::InjectEvent::KeyRelease(LEFT)
                    });
                }
                cur_row = r;
                cur_col = c;
                // Print char
                if r == 0 || r == rows + 1 {
                    // Top or bottom border: just '/'
                    if grid[r][c] == '/' {
                        queue.push_back(e_window::uxn::InjectEvent::Char('/' as u8));
                    } else {
                        queue.push_back(e_window::uxn::InjectEvent::Char(grid[r][c] as u8));
                    }
                } else if c == 0 {
                    // Left border: '/' then row and col as two hex digits each
                    queue.push_back(e_window::uxn::InjectEvent::Char('/' as u8));
                                            queue.push_back(e_window::uxn::InjectEvent::KeyPress(RIGHT));
                    let hex = format!("{:01X}{:01X}", r, c);
                    for b in hex.bytes() {
                        queue.push_back(e_window::uxn::InjectEvent::Char(b));

                        queue.push_back(e_window::uxn::InjectEvent::KeyRelease(RIGHT));
                    }
                } else if c == cols + 1 {
                    // Right border: '/' then row and col as two hex digits each
                    queue.push_back(e_window::uxn::InjectEvent::Char('/' as u8));
                                            queue.push_back(e_window::uxn::InjectEvent::KeyPress(RIGHT));
                    let hex = format!("{:01X}{:01X}", r, c);
                    for b in hex.bytes() {
                        queue.push_back(e_window::uxn::InjectEvent::Char(b));
                        queue.push_back(e_window::uxn::InjectEvent::KeyRelease(RIGHT));
                    }
                    // After right border, return to start of next row
                    for _ in 0..(cols + 1) {
                        queue.push_back(e_window::uxn::InjectEvent::KeyPress(LEFT));
                        queue.push_back(e_window::uxn::InjectEvent::KeyRelease(LEFT));
                    }
                    queue.push_back(e_window::uxn::InjectEvent::KeyPress(DOWN));
                } else {
                    // File contents
                    queue.push_back(e_window::uxn::InjectEvent::Char(grid[r][c] as u8));
                }
                visited[r][c] = true;
            }
        }
    }
    queue
}
// e_window_uxn.rs - Uxn GUI runner for e_window, with ROM selection and download

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use e_window::uxn::InjectEvent;
use raven_varvara::Key;
use e_window::uxn::{UxnApp, UxnModule};
use eframe::egui;
use eframe::NativeOptions;
use reqwest::blocking::Client;
use reqwest::Url;
use std::sync::mpsc;
use rand::prelude::IndexedRandom;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut rom_path: Option<PathBuf> = None;
    let mut scale: f32 = 2.0;
    let mut window_size = (640, 480);
    let mut title = "e_window Uxn".to_string();
    let mut window_mode = "free".to_string(); // static, free, proportional

    // Parse CLI args (simple version, extend as needed)
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rom" => {
                if i + 1 < args.len() {
                    rom_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--scale" => {
                if i + 1 < args.len() {
                    scale = args[i + 1].parse().unwrap_or(scale);
                    i += 1;
                }
            }
            "--width" => {
                if i + 1 < args.len() {
                    window_size.0 = args[i + 1].parse().unwrap_or(window_size.0);
                    i += 1;
                }
            }
            "--height" => {
                if i + 1 < args.len() {
                    window_size.1 = args[i + 1].parse().unwrap_or(window_size.1);
                    i += 1;
                }
            }
            "--title" => {
                if i + 1 < args.len() {
                    title = args[i + 1].clone();
                    i += 1;
                }
            }
            "--window-mode" => {
                if i + 1 < args.len() {
                    window_mode = args[i + 1].to_lowercase();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // --- Static ROM URLs ---
    let static_rom_urls = vec![
        (
            "orca-toy/orca.rom",
            "https://rabbits.srht.site/orca-toy/orca.rom",
        ),
        ("potato.rom", "https://rabbits.srht.site/potato/potato.rom"),
        //("uxn.rom", "https://rabbits.srht.site/uxn/uxn.rom"),
        ("oekaki.rom", "https://rabbits.srht.site/oekaki/oekaki.rom"),
        ("flick.rom", "https://rabbits.srht.site/flick/flick.rom"),
        //("adelie.rom", "https://rabbits.srht.site/adelie/adelie.rom"),
        //("nasu.rom", "https://hundredrabbits.itch.io/nasu"),
        // ("noodle.rom", "https://hundredrabbits.itch.io/noodle"),
        // ("left.rom", "http://hundredrabbits.itch.io/Left")
        // Add more static ROMs here: ("label", "url")
    ];

    // Download static ROMs and collect their names and paths
    let mut static_rom_names = Vec::new();
    let mut static_rom_paths = Vec::new();
    for (label, url) in &static_rom_urls {
        let path = download_static_rom(label, url)?;
        static_rom_names.push(label.to_string());
        static_rom_paths.push(path);
    }

    // If no ROM is selected, fetch ROM list and prompt user
    let mut auto_rom_select = false;
    let mut selected_rom_label: Option<String> = None;
    if rom_path.is_none() {
        let mut roms = static_rom_names.clone();
        let github_roms = fetch_rom_list()?;
        roms.extend(github_roms.iter().cloned());
        let selected = prompt_rom_selection(&roms)?;
        if selected == "__AUTO__" {
            // User hit return: enable auto ROM cycling
            auto_rom_select = true;
        } else {
            let idx = roms.iter().position(|r| r == &selected).unwrap_or(0);
            selected_rom_label = Some(roms[idx].clone());
            if idx < static_rom_paths.len() {
                rom_path = Some(static_rom_paths[idx].clone());
                title = format!("e_window_uxn - {}", roms[idx]);
            } else {
                let github_idx = idx - static_rom_paths.len();
                let rom_file = download_rom(&github_roms[github_idx])?;
                title = format!("e_window_uxn - {}", github_roms[github_idx]);
                rom_path = Some(rom_file);
            }
        }
    } else if let Some(path) = &rom_path {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            selected_rom_label = Some(name.to_string());
            title = format!("e_window_uxn - {}", name);
        }
    }

    // Build all_roms and all_labels if needed (for auto ROM cycling)
    let mut all_roms: Vec<Vec<u8>> = Vec::new();
    let mut all_labels: Vec<String> = Vec::new();
    if auto_rom_select {
        // Add static ROMs first
        for (i, path) in static_rom_paths.iter().enumerate() {
            let bytes = std::fs::read(path)?;
            all_roms.push(bytes);
            all_labels.push(static_rom_names[i].clone());
        }
        // Then add GitHub ROMs
        let github_roms = fetch_rom_list()?;
        for name in &github_roms {
            let rom_file = download_rom(name)?;
            let bytes = std::fs::read(&rom_file)?;
            all_roms.push(bytes);
            all_labels.push(name.clone());
        }
    }

    // --- Helper to download static ROMs by URL ---
    fn download_static_rom(label: &str, url: &str) -> Result<std::path::PathBuf, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("e_window_uxn")
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(url).send().map_err(|e| e.to_string())?;
        let bytes = resp.bytes().map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        let path = file.into_temp_path().keep().map_err(|e| e.to_string())?;
        Ok(path)
    }

    // Create UxnModule
    let mut uxn_mod = UxnModule::new(rom_path.as_ref().map(|p| p.as_path()))?;

    // Set up event channel for UxnApp
    let (event_tx, event_rx) = mpsc::channel();

    // Prepare VM and Varvara for UxnApp

let vm = Arc::clone(&uxn_mod.uxn);
let mut vm = vm.lock().unwrap();
let mut dev = uxn_mod.varvara.take().expect("Varvara device missing");

// Use selected_rom_label for matching, not temp file name
println!("[DEBUG] selected_rom_label: {:?}", selected_rom_label);
if let Some(label) = &selected_rom_label {
    if label.contains("orca.rom") {
        println!("[DEBUG] ROM matched orca.rom by label: {}", label);
        let dir_path = r"C:\w\music\Orca-c\examples\basics";
        let entries = fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory: {}", e))
            .and_then(|read_dir| {
            let files: Vec<_> = read_dir
                .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("orca") {
                    Some(path)
                    } else {
                    None
                    }
                })
                })
                .collect();
            if files.is_empty() {
                Err("No .orca files found".to_string())
            } else {
                Ok(files)
            }
            });

        match entries {
            Ok(files) => {
            let mut rng = rand::thread_rng();
            if let Some(random_file) = files.choose(&mut rng) {
                println!("[DEBUG] Detected orca.rom, sending {:?} to console...", random_file);
                match send_orca_file_to_console(&mut dev, &mut vm, random_file.to_str().unwrap()) {
                Ok(_) => println!("[DEBUG] {:?} sent to console successfully.", random_file),
                Err(e) => eprintln!("Failed to send file: {}", e),
                }
            }
            }
            Err(e) => {
            eprintln!("[DEBUG] Could not select random .orca file: {}", e);
            }
        }
    } else {
        println!("[DEBUG] ROM label did not match orca.rom: {}", label);
    }
} else {
    println!("[DEBUG] selected_rom_label is None");
}
#[cfg(windows)]
fn beep() {
    unsafe { winapi::um::winuser::MessageBeep(0xFFFFFFFF); }
}

#[cfg(not(windows))]
fn beep() {
    print!("\x07");
}
// Register listeners for console output
dev.console.register_stdout_listener(|byte| {
    beep();
    if byte == 0x07 {
        println!("[CONSOLE BEEP] BEL (0x07) received!");
    } else {
        match std::str::from_utf8(&[byte]) {
            Ok(s) => print!("{}", s),
            Err(_) => print!("?"), // Replacement character for invalid UTF-8
        }
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
});
dev.console.register_stderr_listener(|byte| {
    println!("Console stderr: {}", byte);
});

    let size = dev.output(&vm).size;
    drop(vm); // Release lock

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([
            (window_size.0 as f32 * scale) as f32,
            (window_size.1 as f32 * scale) as f32,
        ])
        .with_title(&title);

    match window_mode.as_str() {
        "static" => {
            viewport = viewport.with_resizable(false);
        }
        _ => {
            viewport = viewport.with_resizable(true);
        }
    }

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    // If auto_rom_select, pass all_roms and set auto_rom_select flag, else pass empty vec and false
    let app_all_roms = if auto_rom_select {
        all_roms.clone()
    } else {
        Vec::new()
    };
    let app_auto_rom_select = auto_rom_select;

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            let ctx = &cc.egui_ctx;
            let vm = Arc::clone(&uxn_mod.uxn);
            let mut vm = vm.lock().unwrap();
            static mut RAM: [u8; 65536] = [0; 65536];
            let ram: &'static mut [u8; 65536] = unsafe { &mut RAM };
            let new_uxn = raven_uxn::Uxn::new(ram, raven_uxn::Backend::Interpreter);
            let mut app = UxnApp::new_with_mode(
                std::mem::replace(&mut *vm, new_uxn),
                dev,
                size,
                scale,
                event_rx,
                ctx,
                window_mode.clone(),
                app_all_roms,
                if app_auto_rom_select {
                    all_labels.clone()
                } else {
                    Vec::new()
                },
                app_auto_rom_select,
            );
            if app_auto_rom_select {
                let ctx = cc.egui_ctx.clone();
                app.set_on_rom_change(move |rom_name| {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                        "e_window_uxn - {}",
                        rom_name
                    )));
                });
            }

            // --- Orca file injection after UI is ready ---
            let mut pending_orca_inject = false;
            let orca_dir = r"C:\w\music\Orca-c\examples\basics";
            let orca_path = format!("{}/k.orca", orca_dir);
            if let Some(label) = &selected_rom_label {
                if label.contains("orca.rom") {
                    pending_orca_inject = true;
                }
            }
            if pending_orca_inject {
                let orca_path = orca_path.clone();
                app.set_on_first_update(Box::new(move |app_ref: &mut UxnApp| {
                    let queue = build_orca_inject_queue(&orca_path);
                    app_ref.queue_input(queue);
                }));
            }

            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )?;
    Ok(())
}

/// Fetch the list of ROMs from the GitHub directory listing
fn fetch_rom_list() -> Result<Vec<String>, String> {
    let url = "https://api.github.com/repos/mkeeter/raven/contents/roms";
    let client = Client::builder()
        .user_agent("e_window_uxn")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .map_err(|e| e.to_string())?;
    let mut roms = Vec::new();
    if let Some(arr) = resp.as_array() {
        for entry in arr {
            if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                if name.ends_with(".rom") {
                    roms.push(name.to_string());
                }
            }
        }
    }
    Ok(roms)
}

/// Prompt the user to select a ROM (simple CLI prompt)
fn prompt_rom_selection(roms: &[String]) -> Result<String, String> {
    println!("Available ROMs: (hit return for auto ROM cycling)");
    for (i, rom) in roms.iter().enumerate() {
        println!("  [{}] {}", i + 1, rom);
    }
    println!("  [Return] Enable AUTO ROM CYCLING mode (cycle all ROMs every 10s)");
    print!("Select a ROM by number, or hit return for auto cycling: ");
    std::io::stdout().flush();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input);
    let trimmed = input.trim();
    if trimmed.is_empty() {
        println!(
            "[AUTO ROM CYCLING] You selected auto mode. All ROMs will cycle every 10 seconds."
        );
        return Ok("__AUTO__".to_string());
    }
    let idx: usize = trimmed.parse().unwrap_or(1);
    let idx = idx.saturating_sub(1).min(roms.len().saturating_sub(1));
    Ok(roms[idx].clone())
}

/// Download the selected ROM to a temp file and return its path
fn download_rom(rom_name: &str) -> Result<PathBuf, String> {
    let url = format!(
        "https://raw.githubusercontent.com/mkeeter/raven/main/roms/{}",
        rom_name
    );
    let client = Client::builder()
        .user_agent("e_window_uxn")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(Url::parse(&url).map_err(|e| e.to_string())?)
        .send()
        .map_err(|e| e.to_string())?;
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    let path = file.into_temp_path().keep().map_err(|e| e.to_string())?;
    Ok(path)
}

/// Send an orca file to the VM console, simulating character entry with right/left/down arrows
fn send_orca_file_to_console(dev: &mut raven_varvara::Varvara, vm: &mut raven_uxn::Uxn, file_path: &str) -> Result<(), String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    // Key codes (may need adjustment for your VM)
    // const CTRL_H: u8 = 0x08; // Ctrl+H (backspace, often used for home)
    const RIGHT_ARROW: u8 = 0x1B; // Example: ESC for right arrow (replace with actual code)
    const LEFT_ARROW: u8 = 0x1A; // Example: SUB for left arrow (replace with actual code)
    const DOWN_ARROW: u8 = 0x0A; // LF for down arrow (replace with actual code)

    // Send Ctrl+H to start
    // dev.console(vm, CTRL_H);
    // println!("[DEBUG] Sent Ctrl+H to console");

    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    for (line_idx, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| e.to_string())?;
        let mut arrow_count = 0;
        for (col_idx, ch) in line.chars().enumerate() {
            let byte = ch as u8;
            dev.console(vm, byte);
            // println!("[DEBUG] Line {}, Col {}: Sent char '{}' (0x{:02X})", line_idx, col_idx, ch, byte);
            dev.console(vm, RIGHT_ARROW);
            // println!("[DEBUG] Sent RIGHT_ARROW after char");
            arrow_count += 1;
        }
        // After line, send LEFT_ARROW 'arrow_count' times to return to column 0
        for i in 0..arrow_count {
            dev.console(vm, LEFT_ARROW);
            // println!("[DEBUG] Sent LEFT_ARROW to return to column 0 ({} of {})", i+1, arrow_count);
        }
        // Send DOWN_ARROW to move to next line
        dev.console(vm, DOWN_ARROW);
        // println!("[DEBUG] Sent DOWN_ARROW to move to next line");
    }
    Ok(())
}
// ...removed UxnEguiApp, now using UxnApp from uxn.rs...
