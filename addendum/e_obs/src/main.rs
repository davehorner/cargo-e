#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! tokio = { version = "1", features = ["full"] }
//! tokio-tungstenite = "0.20"
//! futures-util = "0.3"
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! tokio-native-tls = "0.3"
//! uuid = { version = "1", features = ["v4"] }
//! sha2 = "0.10"
//! base64 = "0.21"
//! url = "2"
//! chrono = "*"
//! wallpaper = "*"
//! windows = { version="*", features = ["Win32_System_Registry","Win32_UI","Win32_UI_WindowsAndMessaging"] }
//! clap = { version = "4", features = ["derive"] }
//! which = "*"
//! once_cell = "1.10"
//! ```
// use windows::Win32::Security::Authentication::Identity::SAM_CREDENTIAL_UPDATE_REGISTER_ROUTINE;
use base64::Engine;
use chrono::Local;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

// use std::ptr::null;
// use std::ffi::OsStr;
// use std::os::windows::ffi::OsStrExt;
use once_cell::sync::OnceCell;
use rand::Rng;
use std::env;
use which;

static SCREENSHOT_COUNTER: Lazy<std::sync::atomic::AtomicUsize> =
    Lazy::new(|| std::sync::atomic::AtomicUsize::new(1));
static PROJECT_NAME: OnceCell<String> = OnceCell::new();
static RECORD_DIRECTORY: OnceCell<String> = OnceCell::new();
// Store the cmd_screenshot_dir in a OnceCell for later access if needed
static CMD_GIF_DIRS: Lazy<std::sync::Mutex<Vec<PathBuf>>> = Lazy::new(Default::default);
// Use an atomic counter for the current command index
static CMD_INDEX: Lazy<std::sync::atomic::AtomicUsize> =
    Lazy::new(|| std::sync::atomic::AtomicUsize::new(0));

// use windows::{
//     Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE},
//     Win32::Foundation::{LPARAM, WPARAM},
// };
/// OBS Control Script
#[derive(Debug, Clone, Parser)]
struct Args {
    // /// Monitor index (e.g. 0, 1, 2)
    // #[arg(short, long, default_value = "0")]
    // monitor: u8,

    // /// OBS Display Capture source name
    // #[arg(short, long, default_value = "Display Capture")]
    // source: String,
    /// OBS WebSocket password (if any)
    #[arg(short, long)]
    password: Option<String>,

    /// Set text fields: --set-text "name" "value"
    #[arg(long = "set-text", value_names = ["NAME", "VALUE"], num_args = 2, action = clap::ArgAction::Append)]
    set_text: Vec<String>,

    /// Commands to run sequentially
    #[arg(long = "cmd", value_names = ["CMD"], num_args = 1, action = clap::ArgAction::Append)]
    cmd: Vec<String>,

    /// Disable screenshots during recording
    #[arg(long)]
    disable_screenshots: bool,
}

// Global variable for OBS recording directory
static DIR: Lazy<PathBuf> = Lazy::new(|| {
    let base_dir = dirs::video_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Videos")))
        .unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\Videos"))
        .join("e_obs");

    let project_dir = base_dir.join(format!(
        "{}_{}",
        PROJECT_NAME.get().unwrap_or(&"unknown".to_string()),
        Local::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    project_dir
});

#[tokio::main]
async fn main() {
    // Handle --version: print version and exit if requested
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        // You can use env! macro to embed version info at compile time
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let args = Args::parse();

    if check_obs_running().await {
        println!("OBS is already running, not starting a new instance.");
    } else {
        start_obs().expect("Could not start OBS");
        tokio::time::sleep(Duration::from_secs(3)).await;
        if !check_obs_running().await {
            println!("OBS is not starting.");
            return;
        }
    }

    let (mut ws, _) = connect_async("ws://localhost:4455")
        .await
        .expect("Could not connect to OBS");

    // IDENTIFY
    let identify = json!({
        "op": 1,
        "d": {
            "rpcVersion": 1
        }
    });
    ws.send(Message::Text(identify.to_string().into()))
        .await
        .unwrap();

    // HANDLE HELLO / AUTH
    let hello = ws.next().await.unwrap().unwrap();
    let value: serde_json::Value = serde_json::from_str(hello.to_text().unwrap()).unwrap();
    println!("DEBUG: Received hello/auth: {value:?}");

    if value["op"] == 2 && value["d"]["authentication"].is_string() {
        println!("OBS requires authentication.");

        let salt = value["d"]["authenticationSalt"].as_str().unwrap();
        let challenge = value["d"]["authenticationChallenge"].as_str().unwrap();
        let password = args.password.expect("Password required but not provided");

        let secret = sha256_b64(&format!("{password}{salt}"));
        let auth = sha256_b64(&format!("{secret}{challenge}"));

        let auth_msg = json!({
            "op": 3,
            "d": {
                "authentication": auth
            }
        });

        ws.send(Message::Text(auth_msg.to_string().into()))
            .await
            .unwrap();

        // Wait for authentication response
        let auth_resp = ws.next().await.unwrap().unwrap();
        let auth_value: serde_json::Value =
            serde_json::from_str(auth_resp.to_text().unwrap()).unwrap();
        println!("DEBUG: Auth response: {auth_value:?}");
    }
    // Ensure IDENTIFY response is awaited before proceeding
    let identify_resp = ws.next().await.unwrap().unwrap();
    let identify_value: serde_json::Value =
        serde_json::from_str(identify_resp.to_text().unwrap()).unwrap();
    if identify_value["op"] != 2 {
        panic!(
            "Failed to identify with OBS WebSocket. Response: {:?}",
            identify_value
        );
    }
    println!("DEBUG: Identify response: {identify_value:?}");
    // SET MONITOR
    // set_display_capture_monitor(&mut ws, &args.source, args.monitor)
    //     .await
    //     .expect("Failed to set display capture source");

    // // Wait for response to SetInputSettings
    // if let Some(resp) = ws.next().await {
    //     match resp {
    //         Ok(msg) => {
    //             println!("DEBUG: SetInputSettings response: {:?}", msg);
    //         }
    //         Err(e) => {
    //             println!("DEBUG: Error receiving SetInputSettings response: {e}");
    //         }
    //     }
    // }

    // Get the current recording directory using Get-OBSRecordDirectory
    let req_id = Uuid::new_v4().to_string();
    let get_dir = json!({
        "op": 6,
        "d": {
            "requestType": "GetRecordDirectory",
            "requestId": req_id
        }
    });
    ws.send(Message::Text(get_dir.to_string().into()))
        .await
        .unwrap();

    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: GetRecordDirectory response: {text}");
                let value: serde_json::Value = serde_json::from_str(text).unwrap();
                if let Some(dir) = value["d"]["responseData"]["recordDirectory"].as_str() {
                    println!("OBS recording directory: {dir}");
                    // Store recordDirectory in a OnceCell for global access

                    RECORD_DIRECTORY.set(dir.to_string()).ok();

                    // Check if "project-name" is already set in set_text
                    let has_project_name = args
                        .set_text
                        .chunks(2)
                        .any(|chunk| matches!(chunk, [name, _] if name == "project-name"));

                    let project_name = if has_project_name {
                        // Find the value for "project-name" in set_text
                        args.set_text
                            .chunks(2)
                            .find_map(|chunk| {
                                if let [name, value] = chunk {
                                    if name == "project-name" {
                                        Some(value.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| "unknown".to_string())
                    } else {
                        // Try to get project name from Cargo.toml in cwd, else use folder name
                        let name = (|| {
                            let cwd = std::env::current_dir().ok()?;
                            let cargo_toml = cwd.join("Cargo.toml");
                            if cargo_toml.exists() {
                                let contents = std::fs::read_to_string(&cargo_toml).ok()?;
                                for line in contents.lines() {
                                    if let Some(rest) = line.strip_prefix("name") {
                                        // Accept lines like: name = "foo"
                                        let eq = rest.find('=')?;
                                        let val = rest[eq + 1..].trim();
                                        if let Some(stripped) =
                                            val.strip_prefix('"').and_then(|v| v.strip_suffix('"'))
                                        {
                                            return Some(stripped.to_string());
                                        }
                                        // fallback: just return trimmed value
                                        return Some(val.trim_matches('"').to_string());
                                    }
                                }
                            }
                            // fallback: use folder name
                            cwd.file_name()
                                .and_then(|os| os.to_str())
                                .map(|s| s.to_string())
                        })()
                        .unwrap_or_else(|| "unknown".to_string());

                        set_text(&mut ws, "project-name", &name)
                            .await
                            .unwrap_or_else(|e| eprintln!("Failed to set project-name: {e}"));
                        name
                    };

                    // Store project_name in a OnceCell for global access
                    PROJECT_NAME.set(project_name.clone()).ok();
                    let project_dir = &*DIR;

                    set_text(&mut ws, "Text (GDI+)", project_name.as_str())
                        .await
                        .expect("Failed to sequence startt version info");

                    // Create the directory if it doesn't exist
                    fs::create_dir_all(&project_dir)
                        .expect("Failed to create project directory for screenshots/recordings");
                    set_record_directory(&mut ws, &project_dir)
                        .await
                        .expect("Failed to set OBS recording directory");

                    let _ = wallpaper::set_from_path("");

                    // sequence_words(
                    //     &mut ws,
                    //     "Text (GDI+)",
                    //     vec!["Hello World", "OBS WebSocket", "Rust"],
                    //     2
                    // )
                    // .await
                    // .expect("Failed to sequence words");

                    for chunk in args.set_text.chunks(2) {
                        if let [name, value] = chunk {
                            set_text(&mut ws, name, value).await.unwrap_or_else(|e| {
                                eprintln!("Failed to set_text for {name}: {e}")
                            });
                            // Create a subdirectory named with the current date and time
                            // let now = Local::now();
                            // let filename = format!("screenshot_{}.png", now.format("%Y-%m-%d_%H-%M-%S"));
                            // let screenshot_dir = &*DIR;
                            // let screenshot_dir = screenshot_dir.join("e_obs");
                            // let screenshot_path = screenshot_dir.join(&filename);
                            // let screenshot_path_str = screenshot_path.to_str().unwrap();
                        }
                        let count =
                            SCREENSHOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let filename = format!("screenshot_{}.png", format!("{:04}", count));
                        let screenshot_dir = &*DIR;
                        let screenshot_path = screenshot_dir.join(&filename);
                        let screenshot_path_str = &screenshot_path.to_string_lossy().to_string();
                        save_source_screenshot(&mut ws, "Display Capture", screenshot_path_str)
                            .await
                            .expect("Failed to save screenshot");
                    }

                    // Find the path to cargo-e using the which crate, run `cargo-e --version`, and sequence the first line
                    let cargo_e_path = which::which("cargo-e").expect("cargo-e not found in PATH");
                    let output = Command::new(cargo_e_path)
                        .arg("--version")
                        .output()
                        .expect("Failed to run cargo-e --version");
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    let version_line = version_output
                        .lines()
                        .next()
                        .unwrap_or("cargo-e --version failed");
                    set_text(&mut ws, "cargo-e-version-info", version_line)
                        .await
                        .expect("Failed to sequence cargo-e version info");

                    let e_obs_path = which::which("e_obs").expect("e_obs not found in PATH");
                    let output = Command::new(e_obs_path)
                        .arg("--version")
                        .output()
                        .expect("Failed to run e_obs --version");
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    let version_line = version_output
                        .lines()
                        .next()
                        .unwrap_or("e_obs --version failed");
                    set_text(&mut ws, "e_obs-version-info", version_line)
                        .await
                        .expect("Failed to sequence e_obs version info");

                    // Find the path to startt using the which crate, run `startt --version`, and sequence the first line
                    let startt_path = which::which("startt").expect("startt not found in PATH");
                    let output = Command::new(startt_path)
                        .arg("--version")
                        .output()
                        .expect("Failed to run startt --version");
                    let version_output = String::from_utf8_lossy(&output.stdout);
                    let version_line = version_output
                        .lines()
                        .next()
                        .unwrap_or("startt --version failed");
                    set_text(&mut ws, "startt-version-info", version_line)
                        .await
                        .expect("Failed to sequence startt version info");

                    // Set datetime-stamp to the current date and time
                    let current_datetime = Local::now().format("%Y-%m-%d").to_string(); //"%Y-%m-%d %H:%M:%S%.3f"
                    set_text(&mut ws, "datetime-stamp", &current_datetime)
                        .await
                        .expect("Failed to set datetime-stamp");

                    let cmdline = std::env::args_os()
                        .map(|arg| {
                            let s = arg.to_string_lossy();
                            if s.contains(' ') || s.contains('"') {
                                format!("\"{}\"", s.replace('"', "\\\""))
                            } else {
                                s.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    set_text(&mut ws, "cli-args", &cmdline)
                        .await
                        .expect("Failed to set cli-args");

                    if let Ok(git_path) = which::which("git") {
                        let output = Command::new(git_path)
                            .arg("rev-parse")
                            .arg("--short")
                            .arg("HEAD")
                            .output();

                        let sha1 = output
                            .as_ref()
                            .ok()
                            .and_then(|o| String::from_utf8(o.stdout.clone()).ok())
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let sha1_short = if sha1.len() >= 4 {
                            sha1[sha1.len() - 4..].to_string()
                        } else {
                            sha1.clone()
                        };
                        let git_remote_url = Command::new("git")
                            .args(["remote", "get-url", "origin"])
                            .output()
                            .ok()
                            .and_then(|o| String::from_utf8(o.stdout).ok())
                            .map(|s| {
                                let url = s.trim();
                                if url.starts_with("git@") {
                                    url.trim_start_matches("git@").to_string()
                                } else {
                                    url.to_string()
                                }
                            })
                            .unwrap_or_else(|| "unknown".to_string());
                        let git_branch = Command::new("git")
                            .args(["rev-parse", "--abbrev-ref", "HEAD"])
                            .output()
                            .ok()
                            .and_then(|o| String::from_utf8(o.stdout).ok())
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let sha1 = format!("{}@{}#{}", git_remote_url, git_branch, sha1_short);
                        set_text(&mut ws, "project-sha1", &sha1)
                            .await
                            .unwrap_or_else(|e| eprintln!("Failed to set project-sha1: {e}"));
                    }
                    let screenshot_dir = &*DIR;
                    let screenshot_path =
                        screenshot_dir.join(&format!("1{}-wallpaper.png", project_name));
                    let screenshot_path_str = &screenshot_path.to_string_lossy().to_string();

                    save_source_screenshot(&mut ws, "Scene", screenshot_path_str)
                        .await
                        .expect("Failed to save screenshot");
                    println!("{:?}", wallpaper::get());

                    wallpaper::set_from_path(screenshot_path_str).expect("Failed to set wallpaper");
                    set_text(&mut ws, "Text (GDI+)", "")
                        .await
                        .expect("Failed to sequence startt version info");
                } else {
                    println!("Could not get OBS recording directory.");
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving GetRecordDirectory response: {e}");
            }
        }
    }

    // Wait for response to StartRecord
    // Check if already recording
    let req_id = Uuid::new_v4().to_string();
    let get_status = json!({
        "op": 6,
        "d": {
            "requestType": "GetRecordStatus",
            "requestId": req_id
        }
    });
    ws.send(Message::Text(get_status.to_string().into()))
        .await
        .unwrap();

    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: GetRecordStatus response: {text}");
                let value: serde_json::Value = serde_json::from_str(text).unwrap();
                let recording = value["d"]["responseData"]["outputActive"]
                    .as_bool()
                    .unwrap_or(false);
                if recording {
                    // Stop recording
                    let stop_req_id = Uuid::new_v4().to_string();
                    let stop = json!({
                        "op": 6,
                        "d": {
                            "requestType": "StopRecord",
                            "requestId": stop_req_id
                        }
                    });
                    ws.send(Message::Text(stop.to_string().into()))
                        .await
                        .unwrap();

                    // Wait for StopRecord response
                    if let Some(stop_resp) = ws.next().await {
                        if let Ok(stop_msg) = stop_resp {
                            let stop_text = stop_msg.to_text().unwrap();
                            println!("DEBUG: StopRecord response: {stop_text}");
                            let stop_value: serde_json::Value =
                                serde_json::from_str(stop_text).unwrap();
                            if let Some(path) =
                                stop_value["d"]["responseData"]["outputPath"].as_str()
                            {
                                println!("Recording stopped. File saved at: {path}");
                            } else {
                                println!("Recording stopped, but output path not found.");
                            }
                        }
                    }
                    return;
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving GetRecordStatus response: {e}");
            }
        }
    }

    // Not recording, so start recording
    let req_id = Uuid::new_v4().to_string();
    let start = json!({
        "op": 6,
        "d": {
            "requestType": "StartRecord",
            "requestId": req_id
        }
    });
    ws.send(Message::Text(start.to_string().into()))
        .await
        .unwrap();
    println!("Starting OBS recording...");
    // Wait for response to StartRecord
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                //                 set_text(
                //     &mut ws,
                //     "cli-args",
                //     "",
                // )
                // .await
                // .expect("Failed to set cli-args");

                println!("DEBUG: StartRecord response: {:?}", msg);
                let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
                let mut screenshot_handle = None;
                if !args.disable_screenshots {
                    // Spawn a background task to take screenshots in a loop
                    let mut ws = ws; // take ownership for the task
                    let curr_index = 0;
                    // Create a channel to signal the screenshot task to stop
                    screenshot_handle = Some(tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                                    _ = &mut stop_rx => {
                                                        println!("Screenshot task received stop signal.");
                                                                            // Stop recording
                                            let stop_req_id = Uuid::new_v4().to_string();
                                            let stop = json!({
                                                "op": 6,
                                                "d": {
                                                    "requestType": "StopRecord",
                                                    "requestId": stop_req_id
                                                }
                                            });
                                            ws.send(Message::Text(stop.to_string().into())).await.unwrap();
                                                                // Wait for StopRecord response
                                            if let Some(stop_resp) = ws.next().await {
                                                if let Ok(stop_msg) = stop_resp {
                                                    let stop_text = stop_msg.to_text().unwrap();
                                                    println!("DEBUG: StopRecord response: {stop_text}");
                                                    let stop_value: serde_json::Value = serde_json::from_str(stop_text).unwrap();
                                                    if let Some(path) = stop_value["d"]["responseData"]["outputPath"].as_str() {
                                                        println!("Recording stopped. File saved at: {path}");
                                                    } else {
                                                        println!("Recording stopped, but output path not found.");
                                                    }

                                                    // Use project_dir for all subsequent screenshots and video outputs
                                                    if let Some(dir_str) = RECORD_DIRECTORY.get() {
                                                        let dir_path = PathBuf::from(dir_str);
                                                        set_record_directory(&mut ws, &dir_path).await.expect("Failed to set OBS recording directory");
                                                    } else {
                                                        eprintln!("RECORD_DIRECTORY is not set");
                                                    }

                                                }
                                            }
                                                        break;
                                                    }
                                                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                                                                let project_dir = &*DIR;
                            let cmd_idx = CMD_INDEX.load(std::sync::atomic::Ordering::SeqCst);
                            if cmd_idx>0 && cmd_idx!=curr_index {
                                    let prev_dir = project_dir.join(format!("cmd_{}", cmd_idx-1));
                                    if let Some(last_screenshot) = get_last_screenshot(&prev_dir) {
                                        let wallpaper_path = project_dir.join(format!("1wallpaper_cmd{}.png", cmd_idx-1));
                                        if !wallpaper_path.exists() {
                                            fs::copy(&last_screenshot, &wallpaper_path).expect("Failed to save wallpaper");
                                            wallpaper::set_from_path(wallpaper_path.to_str().unwrap()).expect("Failed to set wallpaper");
                                            println!("Wallpaper set from: {}", wallpaper_path.display());
                                        }
                                    }
                            }
                            let cmd_screenshot_dir = project_dir.join(format!("cmd_{}", cmd_idx));
                            if !cmd_screenshot_dir.exists() {
                                fs::create_dir_all(&cmd_screenshot_dir).expect("Failed to create command screenshot directory");
                                SCREENSHOT_COUNTER.store(0, std::sync::atomic::Ordering::SeqCst);
                                CMD_GIF_DIRS.lock().unwrap().push(cmd_screenshot_dir.clone());
                            }


                            // The snapshot thread should use CMD_INDEX.load(Ordering::SeqCst) to determine the directory:
                                                        let count = SCREENSHOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                                        let filename = format!(
                                                            "screenshot_{}.png",
                                                            format!("{:04}", count)
                                                        );
                                                        // let screenshot_dir = CMD_SCREENSHOT_DIR.get().expect("CMD_SCREENSHOT_DIR not set");
                                                        let screenshot_path = cmd_screenshot_dir.join(&filename);
                                                        let screenshot_path_str = screenshot_path.to_string_lossy().to_string();
                                                        if let Err(e) = save_source_screenshot(&mut ws, "Scene", &screenshot_path_str).await {
                                                            eprintln!("Failed to save screenshot: {e}");
                                                        } else {
                                                             println!("Screenshot saved to: {}", screenshot_path_str);
                                                        }
                                                    }
                                                }
                        }
                    }));
                }
                for (i, cmd) in args.cmd.iter().enumerate() {
                    // Set the current command index atomically for the snapshot thread to read
                    CMD_INDEX.store(i, std::sync::atomic::Ordering::SeqCst);

                    println!("Executing command: {cmd}");
                    let mut child = Command::new("cmd")
                        .arg("/C")
                        .arg(cmd)
                        .spawn()
                        .expect("Failed to execute command");

                    // Wait for the command to finish
                    let status = child.wait().expect("Failed to wait on child process");
                    println!("Command exited with status: {status}");
                }
                // Signal the screenshot task to stop and wait for it to finish
                let _ = stop_tx.send(());
                if !args.disable_screenshots {
                    if let Some(screenshot_handle) = screenshot_handle {
                        println!("Waiting for screenshot task to finish...");
                        let _ = screenshot_handle.await;
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving StartRecord response: {e}");
            }
        }
    }
    // After taking screenshots, create a GIF from them
    // Create a GIF for each directory in CMD_GIF_DIRS
    let gif_dirs = CMD_GIF_DIRS.lock().unwrap().clone();
    for (i, dir) in gif_dirs.iter().enumerate() {
        let output_gif = dir.join(format!("../cmd_{}_animation.gif", i + 1));
        match create_gif_with_ffmpeg(
            dir,
            &output_gif,
            10, // frame rate
        ) {
            Ok(_) => println!("GIF created at: {}", output_gif.display()),
            Err(e) => eprintln!("Failed to create GIF for {}: {e}", dir.display()),
        }
    }
    println!("OBS recording started.");
}

async fn check_obs_running() -> bool {
    tokio_tungstenite::connect_async("ws://localhost:4455")
        .await
        .is_ok()
}

fn start_obs() -> std::io::Result<Child> {
    let obs_path = r"C:\Program Files\obs-studio\bin\64bit";

    // Spawn a new thread to launch OBS
    let handle = std::thread::spawn(move || {
        // Add OBS directory to PATH
        if let Ok(path) = env::var("PATH") {
            let new_path = format!("{obs_path};{path}");
            env::set_var("PATH", new_path);
        }
        // Change current directory to obs_path
        std::env::set_current_dir(obs_path).ok();

        let mut cmd = Command::new("obs64");
        cmd.arg("--minimize-to-tray");

        cmd.spawn()
    });

    // Wait for the thread to return the Child process
    handle.join().unwrap()
}

fn sha256_b64(input: &str) -> String {
    let hash = Sha256::digest(input.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hash)
}

async fn _set_display_capture_monitor<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    source_name: &str,
    monitor_index: u8,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req_id = Uuid::new_v4().to_string();
    let msg = json!({
        "op": 6,
        "d": {
            "requestType": "SetInputSettings",
            "requestId": req_id,
            "requestData": {
                "inputName": source_name,
                "inputSettings": {
                    "monitor": monitor_index
                }
            }
        }
    });

    ws.send(Message::Text(msg.to_string().into())).await?;
    Ok(())
}

async fn save_source_screenshot<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    source_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req_id = Uuid::new_v4().to_string();
    let msg = json!({
        "op": 6,
        "d": {
            "requestType": "SaveSourceScreenshot",
            "requestId": req_id,
            "requestData": {
                "sourceName": source_name,
                "imageFormat": "png",
                "imageFilePath": file_path
            }
        }
    });

    ws.send(Message::Text(msg.to_string().into())).await?;

    // Wait for the response
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                // println!("DEBUG: SaveSourceScreenshot response: {text}");
                let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
                // Check if requestId matches
                if value["d"]["requestId"] == req_id {
                    if value["d"]["requestStatus"]["result"]
                        .as_bool()
                        .unwrap_or(false)
                    {
                        println!("Screenshot saved successfully to: {file_path}");
                    } else {
                        println!("Failed to save screenshot.");
                    }
                } else {
                    //println!("Received response with unexpected requestId.\nExpected: {req_id}\nReceived: {}", value["d"]["requestId"]);
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving SaveSourceScreenshot response: {e}");
            }
        }
    }

    Ok(())
}

async fn set_text<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    source_name: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req_id = Uuid::new_v4().to_string();

    // Step 1: Get current settings
    let get_settings_msg = json!({
        "op": 6,
        "d": {
            "requestType": "GetInputSettings",
            "requestId": req_id,
            "requestData": {
                "inputName": source_name
            }
        }
    });

    ws.send(Message::Text(get_settings_msg.to_string().into()))
        .await?;

    let mut current_settings = serde_json::Value::Null;
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                let value: serde_json::Value = serde_json::from_str(text).unwrap();
                if value["d"]["requestId"] == req_id {
                    current_settings = value["d"]["responseData"]["inputSettings"].clone();
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving GetInputSettings response: {e}");
            }
        }
    }

    // Step 2: Merge settings
    if current_settings.is_object() {
        current_settings["text"] = serde_json::Value::String(text.to_string());
    } else {
        current_settings = json!({ "text": text });
    }

    // Step 3: Set updated settings
    let set_settings_msg = json!({
        "op": 6,
        "d": {
            "requestType": "SetInputSettings",
            "requestId": req_id,
            "requestData": {
                "inputName": source_name,
                "inputSettings": current_settings,
                "overlay": false
            }
        }
    });

    ws.send(Message::Text(set_settings_msg.to_string().into()))
        .await?;

    // Wait for the response and consume it
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: SetInputSettings response: {text}");
            }
            Err(e) => {
                println!("DEBUG: Error receiving SetInputSettings response: {e}");
            }
        }
    }
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: SetInputSettings response: {text}");
            }
            Err(e) => {
                println!("DEBUG: Error receiving SetInputSettings response: {e}");
            }
        }
    }
    Ok(())
}
async fn _sequence_words<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    source_name: &str,
    words: Vec<&str>,
    interval: u64,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    for word in words {
        let req_id = Uuid::new_v4().to_string();

        // Step 1: Get current settings
        let get_settings_msg = json!({
            "op": 6,
            "d": {
                "requestType": "GetInputSettings",
                "requestId": req_id,
                "requestData": {
                    "inputName": source_name
                }
            }
        });

        ws.send(Message::Text(get_settings_msg.to_string().into()))
            .await?;

        let mut current_settings = serde_json::Value::Null;
        if let Some(resp) = ws.next().await {
            match resp {
                Ok(msg) => {
                    let text = msg.to_text().unwrap();
                    let value: serde_json::Value = serde_json::from_str(text).unwrap();
                    if value["d"]["requestId"] == req_id {
                        println!("DEBUG: GetInputSettings response: {value:?}");
                        current_settings = value["d"]["responseData"]["inputSettings"].clone();
                    }
                }
                Err(e) => {
                    println!("DEBUG: Error receiving GetInputSettings response: {e}");
                }
            }
        }

        // Step 2: Merge settings
        if current_settings.is_object() {
            current_settings["text"] = serde_json::Value::String(word.to_string());
            // Ensure bk_opacity is preserved or set to a default value
            if !current_settings
                .as_object()
                .unwrap()
                .contains_key("bkOpacity")
            {
                current_settings["bkOpacity"] =
                    serde_json::Value::Number(serde_json::Number::from(100)); // Default to 100
            }
        } else {
            current_settings = json!({
                "text": word,
                "bkOpacity": 100 // Default to 100
            });
        }

        // Step 3: Set updated settings
        let set_settings_msg = json!({
            "op": 6,
            "d": {
                "requestType": "SetInputSettings",
                "requestId": req_id,
                "requestData": {
                    "inputName": source_name,
                    "inputSettings": current_settings,
                    "overlay": false
                }
            }
        });

        ws.send(Message::Text(set_settings_msg.to_string().into()))
            .await?;

        // Wait for the response and consume it
        if let Some(resp) = ws.next().await {
            match resp {
                Ok(msg) => {
                    let text = msg.to_text().unwrap();
                    println!("DEBUG: SetInputSettings response: {text}");
                }
                Err(e) => {
                    println!("DEBUG: Error receiving SetInputSettings response: {e}");
                }
            }
        }

        // Wait for the specified interval before updating the next word
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }

    Ok(())
}

async fn _set_text_with_scene_item_properties<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    scene_name: &str,
    item_name: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req_id = Uuid::new_v4().to_string();

    // Step 1: Get current scene item properties
    let get_properties_msg = json!({
        "op": 6,
        "d": {
            "requestType": "GetSceneItemProperties",
            "requestId": req_id,
            "requestData": {
                "sceneName": scene_name,
                "itemName": item_name
            }
        }
    });

    ws.send(Message::Text(get_properties_msg.to_string().into()))
        .await?;

    let mut current_properties = serde_json::Value::Null;
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                let value: serde_json::Value = serde_json::from_str(text).unwrap();
                if value["d"]["requestId"] == req_id {
                    println!("DEBUG: GetSceneItemProperties response: {value:?}");
                    current_properties = value["d"]["responseData"].clone();
                }
            }
            Err(e) => {
                println!("DEBUG: Error receiving GetSceneItemProperties response: {e}");
            }
        }
    }

    // Step 2: Update the text field
    if current_properties.is_object() {
        current_properties["text"] = serde_json::Value::String(text.to_string());
    } else {
        current_properties = json!({ "text": text });
    }

    // Step 3: Set updated scene item properties
    let set_properties_msg = json!({
        "op": 6,
        "d": {
            "requestType": "SetSceneItemProperties",
            "requestId": req_id,
            "requestData": {
                "sceneName": scene_name,
                "itemName": item_name,
                "sceneItemProperties": current_properties
            }
        }
    });

    ws.send(Message::Text(set_properties_msg.to_string().into()))
        .await?;

    // Wait for the response and consume it
    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: SetSceneItemProperties response: {text}");
            }
            Err(e) => {
                println!("DEBUG: Error receiving SetSceneItemProperties response: {e}");
            }
        }
    }

    Ok(())
}

// use std::io;
// use windows::Win32::System::Registry::{
//     RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, KEY_READ,
// };
// use windows::core::PCWSTR;
//                          match is_windows_spotlight_enabled() {
//         Ok(true) => println!("Windows Spotlight is enabled."),
//         Ok(false) => println!("Windows Spotlight is disabled."),
//         Err(e) => eprintln!("Failed to check Spotlight status: {e}"),
//     }
//                enable_windows_spotlight();
// fn is_windows_spotlight_enabled() -> io::Result<bool> {
//     fn read_dword_value(subkey: &str, value_name: &str) -> io::Result<u32> {
//         let subkey_w: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
//         let value_name_w: Vec<u16> = value_name.encode_utf16().chain(Some(0)).collect();
//         let mut data: u32 = 0;
//         let mut data_size = std::mem::size_of::<u32>() as u32;

//         let result = unsafe {
//             RegGetValueW(
//                 HKEY_CURRENT_USER,
//                 PCWSTR(subkey_w.as_ptr()),
//                 PCWSTR(value_name_w.as_ptr()),
//                 RRF_RT_REG_DWORD,
//                 None,
//                 Some(&mut data as *mut _ as *mut _),
//                 Some(&mut data_size),
//             )
//         };

//         if result.0 == 0 {
//             Ok(data)
//         } else {
//             Err(io::Error::new(
//                 io::ErrorKind::NotFound,
//                 format!("Registry key or value not found: {}", value_name),
//             ))
//         }
//     }

//     // Check the DisableWindowsSpotlightFeatures value
//     let spotlight_disabled = read_dword_value(
//         "Software\\Policies\\Microsoft\\Windows\\CloudContent",
//         "DisableWindowsSpotlightFeatures",
//     )
//     .unwrap_or(0); // Default to 0 if the key is missing

//     Ok(spotlight_disabled == 0) // Spotlight is enabled if the value is 0
// }

// /// Re-enables Windows Spotlight for the current user by updating the registry.
// /// Note: User may need to manually refresh their desktop background settings for it to take effect.
// pub fn enable_windows_spotlight() -> std::io::Result<()> {
//   let ret = set_windows_spotlight(true);
//   broadcast_setting_change();
//       Command::new("RUNDLL32.EXE")
//         .arg("USER32.DLL,UpdatePerUserSystemParameters")
//         .spawn()
//         .expect("Failed to refresh desktop settings");
//   ret
// }

// use windows::Win32::System::Registry::{RegSetValueExW, RegOpenKeyExW, KEY_WRITE};

// fn set_windows_spotlight(enabled: bool) -> std::io::Result<()> {
//     let spotlight_subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\DesktopSpotlight\\Settings";
//     let wallpapers_subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Wallpapers";
//     let spotlight_value_name = "EnabledState";
//     let wallpapers_value_name = "BackgroundType";

//     let spotlight_value_data: u32 = if enabled { 1 } else { 0 }; // 1 to enable, 0 to disable
//     let wallpapers_value_data: u32 = if enabled { 3 } else { 0 }; // 3 for Spotlight, 0 for default

//     let spotlight_subkey_w: Vec<u16> = spotlight_subkey.encode_utf16().chain(Some(0)).collect();
//     let wallpapers_subkey_w: Vec<u16> = wallpapers_subkey.encode_utf16().chain(Some(0)).collect();
//     let spotlight_value_name_w: Vec<u16> = spotlight_value_name.encode_utf16().chain(Some(0)).collect();
//     let wallpapers_value_name_w: Vec<u16> = wallpapers_value_name.encode_utf16().chain(Some(0)).collect();

//     unsafe {
//         let mut hkey_spotlight = std::ptr::null_mut();
//         let result_spotlight = RegOpenKeyExW(
//             HKEY_CURRENT_USER,
//             PCWSTR(spotlight_subkey_w.as_ptr()),
//             Some(0), // Reserved; must be 0
//             KEY_WRITE, // Desired access rights
//             &mut windows::Win32::System::Registry::HKEY(hkey_spotlight), // Pointer to receive the handle
//         );

//         if result_spotlight.0 != 0 {
//             return Err(std::io::Error::new(
//                 std::io::ErrorKind::PermissionDenied,
//                 "Failed to open DesktopSpotlight registry key",
//             ));
//         }

//         let spotlight_value_data_bytes = &spotlight_value_data.to_le_bytes();
//         let result_spotlight_set = RegSetValueExW(
//             windows::Win32::System::Registry::HKEY(hkey_spotlight),
//             PCWSTR(spotlight_value_name_w.as_ptr()),
//             Some(0),
//             windows::Win32::System::Registry::REG_DWORD,
//             Some(spotlight_value_data_bytes),
//         );

//         if result_spotlight_set.0 != 0 {
//             return Err(std::io::Error::new(
//                 std::io::ErrorKind::PermissionDenied,
//                 "Failed to set DesktopSpotlight registry value",
//             ));
//         }

//         windows::Win32::System::Registry::RegCloseKey(windows::Win32::System::Registry::HKEY(hkey_spotlight));

//         let mut hkey_wallpapers = std::ptr::null_mut();
//         let result_wallpapers = RegOpenKeyExW(
//             HKEY_CURRENT_USER,
//             PCWSTR(wallpapers_subkey_w.as_ptr()),
//             Some(0), // Reserved; must be 0
//             KEY_WRITE, // Desired access rights
//             &mut windows::Win32::System::Registry::HKEY(hkey_wallpapers), // Pointer to receive the handle
//         );

//         if result_wallpapers.0 != 0 {
//             return Err(std::io::Error::new(
//                 std::io::ErrorKind::PermissionDenied,
//                 "Failed to open Wallpapers registry key",
//             ));
//         }

//         let wallpapers_value_data_bytes = &wallpapers_value_data.to_le_bytes();
//         let result_wallpapers_set = RegSetValueExW(
//             windows::Win32::System::Registry::HKEY(hkey_wallpapers),
//             PCWSTR(wallpapers_value_name_w.as_ptr()),
//             Some(0),
//             windows::Win32::System::Registry::REG_DWORD,
//             Some(wallpapers_value_data_bytes),
//         );

//         if result_wallpapers_set.0 != 0 {
//             return Err(std::io::Error::new(
//                 std::io::ErrorKind::PermissionDenied,
//                 "Failed to set Wallpapers registry value",
//             ));
//         }

//         windows::Win32::System::Registry::RegCloseKey(windows::Win32::System::Registry::HKEY(hkey_wallpapers));
//     }

//     Ok(())
// }

// fn broadcast_setting_change() {
//     let param = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\ContentDeliveryManager");
//     unsafe {
//         let _ = SendMessageTimeoutW(
//             HWND_BROADCAST,
//             WM_SETTINGCHANGE,
//             WPARAM(0),
//             LPARAM(param.as_ptr() as isize),
//             SMTO_ABORTIFHUNG,
//             5000,
//             None,
//         );
//     }
// }

// // Helper to convert &str to null-terminated UTF-16 Vec
// fn wide_null(s: &str) -> Vec<u16> {
//     let mut v: Vec<u16> = OsStr::new(s).encode_wide().collect();
//     v.push(0);
//     v
// }

fn _get_obs_profile_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let appdata = std::env::var("APPDATA")?;
    let obs_profiles_path = Path::new(&appdata).join("obs-studio/basic/profiles");

    if !obs_profiles_path.exists() {
        return Err("OBS profiles directory does not exist".into());
    }

    for entry in std::fs::read_dir(&obs_profiles_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let ini_path = path.join("basic.ini");
            if ini_path.exists() {
                let contents = std::fs::read_to_string(&ini_path)?;
                let mut in_simple_output_section = false;
                for line in contents.lines() {
                    if line.trim() == "[SimpleOutput]" {
                        in_simple_output_section = true;
                    } else if line.starts_with('[') {
                        // Exit section
                        in_simple_output_section = false;
                    }

                    if in_simple_output_section {
                        if let Some(filepath) = line.strip_prefix("FilePath=") {
                            return Ok(PathBuf::from(filepath));
                        }
                    }
                }
            }
        }
    }

    Err("FilePath not found in the [SimpleOutput] section of any basic.ini file".into())
}

/// Creates an animated GIF from a series of image files using ffmpeg.
///
/// # Arguments
/// * `input_pattern` - A glob pattern for the input image files (e.g., "screenshot_*.png").
/// * `output_path` - The path to the output GIF file.
/// * `frame_rate` - The frame rate for the GIF (e.g., 10 for 10 frames per second).
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if successful, or an error if the command fails.
// fn create_gif_with_ffmpeg(
//     input_pattern: &str,
//     output_path: &str,
//     frame_rate: u32,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     use std::process::Command;

//     let status = Command::new("ffmpeg")
//         .arg("-y") // Overwrite output files without asking
//         .arg("-framerate")
//         .arg(frame_rate.to_string())
//         .arg("-f")
//         .arg("concat")
//         .arg("-safe")
//         .arg("0")
//         .arg("-i")
//         .arg(input_pattern)
//         .arg("-vf")
//         .arg("scale=640:-1:flags=lanczos,fps=10") // Scale and set frame rate
//         .arg(output_path)
//         .status()?;

//     if status.success() {
//         println!("GIF created successfully at: {}", output_path);
//         Ok(())
//     } else {
//         Err(format!("ffmpeg command failed with status: {:?}", status).into())
//     }
// }

/// Creates an animated GIF from a series of screenshots using ffmpeg.
///
/// # Arguments
/// * `input_dir` - The directory containing the screenshot images.
/// * `output_gif` - The path to the output GIF file.
/// * `frame_rate` - The frame rate for the GIF.
///
/// # Returns
/// * `Result<(), String>` - Returns `Ok(())` if successful, or an error message if the operation fails.
fn create_gif_with_ffmpeg(
    input_dir: &Path,
    output_gif: &Path,
    frame_rate: u32,
) -> Result<(), String> {
    // Collect matching files
    let mut matching_files: Vec<_> = fs::read_dir(input_dir)
        .map_err(|e| format!("Failed to read input directory: {e}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().to_string_lossy().contains("screenshot_"))
        .map(|entry| entry.path())
        .collect();

    // Sort files by name to ensure correct order
    matching_files.sort();

    if matching_files.is_empty() {
        return Err("No matching files found for screenshots".to_string());
    }

    // Create a temporary file listing all input files
    let mut file_list_path = input_dir.to_path_buf();
    file_list_path.push("file_list.txt");
    let file_list = fs::File::create(&file_list_path)
        .map_err(|e| format!("Failed to create file list: {e}"))?;
    let mut writer = BufWriter::new(file_list);

    for entry in &matching_files {
        writeln!(writer, "file '{}'", entry.display())
            .map_err(|e| format!("Failed to write to file list: {e}"))?;
    }

    // Flush the writer to ensure all data is written
    writer
        .flush()
        .map_err(|e| format!("Failed to flush file list: {e}"))?;

    // Log the file list for debugging
    println!("Generated file list for ffmpeg:");
    for entry in &matching_files {
        println!("- {}", entry.display());
    }

    // Construct and execute the ffmpeg command
    let output_gif_str = output_gif.to_string_lossy().to_string();
    let file_list_str = file_list_path.to_string_lossy().to_string();

    println!("Executing ffmpeg command:");
    println!(
        "ffmpeg -y -fps_mode vfr -f concat -safe 0 -i {} -vf fps={},scale=640:-1:flags=lanczos {}",
        file_list_str, frame_rate, output_gif_str
    );

    let input_pattern = input_dir
        .join("screenshot_%04d.png")
        .to_string_lossy()
        .to_string();

    let status = Command::new("ffmpeg")
        .arg("-y") // Overwrite output files without asking
        .arg("-framerate")
        .arg("10")
        .arg("-i")
        .arg(&input_pattern) // Use frame pattern instead of file list
        .arg("-vf")
        .arg(format!("fps={},scale=640:-1:flags=lanczos", frame_rate))
        .arg("-loop")
        .arg("0") // Loop the GIF
        .arg(&output_gif_str)
        .status();

    // Clean up the temporary file
    let _ = fs::remove_file(file_list_path);

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("ffmpeg exited with status code: {}", status)),
        Err(e) => Err(format!("Failed to execute ffmpeg: {e}")),
    }
}

async fn set_record_directory<S>(
    ws: &mut tokio_tungstenite::WebSocketStream<S>,
    output_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let req_id = Uuid::new_v4().to_string();
    let set_dir_msg = json!({
        "op": 6,
        "d": {
            "requestType": "SetRecordDirectory",
            "requestId": req_id,
            "requestData": {
                "recordDirectory": output_dir
            }
        }
    });

    ws.send(Message::Text(set_dir_msg.to_string().into()))
        .await?;

    if let Some(resp) = ws.next().await {
        match resp {
            Ok(msg) => {
                let text = msg.to_text().unwrap();
                println!("DEBUG: SetRecordDirectory response: {text}");
            }
            Err(e) => {
                println!("DEBUG: Error receiving SetRecordDirectory response: {e}");
            }
        }
    }

    Ok(())
}

/// Returns the last screenshot file (by name sort) in the given directory, or None if not found.
fn get_last_screenshot(dir: &Path) -> Option<PathBuf> {
    let mut screenshots: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("screenshot_") && n.ends_with(".png"))
                .unwrap_or(false)
        })
        .collect();
    screenshots.sort();
    let len = screenshots.len();
    if len == 0 {
        return None;
    }
    let start = len - len / 3;
    let back_third = &screenshots[start..];
    let mut rng = rand::rng();
    let random_index = rng.random_range(0..back_third.len());
    back_third.get(random_index).cloned()
}
