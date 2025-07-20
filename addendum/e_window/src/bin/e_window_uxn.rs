//! e_window_uxn.rs - Uxn GUI runner for e_window, with ROM selection and download

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use e_window::uxn::{UxnApp, UxnModule};
use eframe::egui;
use eframe::NativeOptions;
use reqwest::blocking::Client;
use reqwest::Url;
use std::sync::mpsc;

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
    let vm = vm.lock().unwrap();
    let mut dev = uxn_mod.varvara.take().expect("Varvara device missing");
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
            // Create UxnApp using the encapsulated logic
            let ctx = &cc.egui_ctx;
            let vm = Arc::clone(&uxn_mod.uxn);
            let mut vm = vm.lock().unwrap();
            // Create a new RAM buffer for the replacement Uxn instance
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
            // Patch: If auto_rom_select, update window title on ROM change
            if app_auto_rom_select {
                let ctx = cc.egui_ctx.clone();
                app.set_on_rom_change(move |rom_name| {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                        "e_window_uxn - {}",
                        rom_name
                    )));
                });
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

// ...removed UxnEguiApp, now using UxnApp from uxn.rs...
