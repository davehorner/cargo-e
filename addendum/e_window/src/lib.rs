//! Library interface for launching the e_window app with custom arguments.

pub mod app;
pub mod parser;

use getargs::{Arg, Options};
use std::env::current_exe;
use std::fs;
use std::io::{self, Read};

/// Run the e_window app with the given arguments (excluding program name).
pub fn run_window<I, S>(args: I) -> eframe::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter().map(|s| s.as_ref().to_string()).collect::<Vec<_>>();
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

    while let Some(arg) = opts.next_arg().expect("argument parsing error") {
        match arg {
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
    -h, --help           Show this help and exit
Any other positional arguments are collected as files or piped input."#
                );
                return Ok(());
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
            Arg::Short('i') | Arg::Long("input-file") => {
                if let Ok(val) = opts.value() {
                    input_file = Some(val.to_string());
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
                }
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

    // Read input data: from file if specified, else from positional args or stdin
    let (input_data, editor_mode) = if let Some(file) = input_file {
        (fs::read_to_string(file).unwrap_or_else(|_| "".to_string()), true)
    } else if !positional_args.is_empty() {
        (positional_args.join("\n"), true)
    } else {
        // Try to read from stdin
        let mut buffer = String::new();
        use std::io::IsTerminal;
        if !io::stdin().is_terminal() && io::stdin().read_to_string(&mut buffer).unwrap_or(0) > 0 {
            (buffer, false)
        } else {
            (String::new(), true)
        }
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
                    _ => {}
                }
            }
        }
        // Use the rest of the lines as the actual input
        actual_input = input_lines.collect::<Vec<_>>().join("\n");
    }

    // If actual_input is empty, use your DEFAULT_CARD
    let actual_input = if actual_input.trim().is_empty() {
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
        app::default_card_with_hwnd(hwnd)
    } else {
        actual_input
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width as f32, height as f32])
            .with_position([x as f32, y as f32])
            .with_title(&title),
        ..Default::default()
    };
    eframe::run_native(
        &appname,
        options,
        Box::new(|cc| {
            Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(
                Box::new(app::App::with_initial_window(
                    width as f32,
                    height as f32,
                    x as f32,
                    y as f32,
                    title.clone(),
                    cc.storage,
                    follow_hwnd,
                ).with_input_data_and_mode(actual_input, editor_mode))
            )
        }),
    )
}