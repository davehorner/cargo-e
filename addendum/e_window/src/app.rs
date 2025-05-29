// This file contains the App struct which implements the eframe::App trait.
// It manages the state of the application, including the text input and display logic.
// It has methods for handling user input and rendering the parsed text.

use crate::parser::{parse_text, ParsedText};
use eframe::egui;
use eframe::Frame;
use eframe::Storage;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::ffi::OsString;
use std::io::Write;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetForegroundWindow;
#[cfg(target_os = "windows")]
use winapi::um::winuser::MessageBoxW;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{IsWindow, MessageBeep};

// Helper function to convert ParsedText to Vec<String>
fn parsed_to_vec(parsed: &ParsedText) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(title) = &parsed.title {
        lines.push(format!("Title: {}", title));
    }
    if let Some(header) = &parsed.header {
        lines.push(format!("Header: {}", header));
    }
    if let Some(caption) = &parsed.caption {
        lines.push(format!("Caption: {}", caption));
    }
    for (k, v, t) in &parsed.triples {
        lines.push(format!("{}: {} ({})", k, v, t));
    }
    if let Some(body) = &parsed.body {
        lines.push(format!("Body: {}", body));
    }
    lines
}

#[derive(Serialize, Deserialize)]
pub struct App {
    input_text: String,
    parsed_data: Vec<String>,
    #[serde(skip)]
    first_frame: bool,
    #[serde(skip)]
    initial_window: Option<(f32, f32, f32, f32, String)>, // (width, height, x, y, title)
    #[serde(skip)]
    pub editor_mode: bool,
    #[serde(skip)]
    start_time: Option<Instant>,
    #[serde(skip)]
    start_datetime: String,
    #[serde(skip)]
    pub follow_hwnd: Option<usize>,
    #[serde(skip)]
    pub follow_triggered: bool,
}

impl Default for App {
    fn default() -> Self {
        let hwnd = {
            #[cfg(target_os = "windows")]
            {
                unsafe { GetForegroundWindow() as usize }
            }
            #[cfg(not(target_os = "windows"))]
            {
                0
            }
        };
        let input_text = default_card_with_hwnd(hwnd);
        let parsed_data = parsed_to_vec(&parse_text(&input_text));
        let now = chrono::Local::now();
        Self {
            input_text,
            parsed_data,
            first_frame: true,
            initial_window: None,
            editor_mode: false,
            start_time: Some(Instant::now()),
            start_datetime: now.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            follow_triggered: false,
            follow_hwnd: None,
        }
    }
}

impl App {
    pub fn with_initial_window(
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        title: String,
        storage: Option<&dyn Storage>,
        follow_hwnd: Option<usize>,
    ) -> Self {
        if let Some(storage) = storage {
            if let Some(restored) = eframe::get_value::<App>(storage, "app") {
                return App {
                    first_frame: true,
                    initial_window: Some((width, height, x, y, title)),
                    start_time: Some(Instant::now()),
                    start_datetime: chrono::Local::now()
                        .format("%Y-%m-%d %H:%M:%S%.3f")
                        .to_string(),
                    follow_hwnd: follow_hwnd,
                    ..restored
                };
            }
        }
        let mut app = App::default();
        app.initial_window = Some((width, height, x, y, title));
        app
    }

    pub fn with_input_data(mut self, input: String) -> Self {
        if input.trim().is_empty() {
            let hwnd = {
                #[cfg(target_os = "windows")]
                {
                    unsafe { GetForegroundWindow() as usize }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    0
                }
            };
            self.input_text = default_card_with_hwnd(hwnd);
        } else {
            self.input_text = input.clone();
        }
        self.parsed_data = parsed_to_vec(&parse_text(&self.input_text));
        self
    }

    pub fn with_input_data_and_mode(mut self, input: String, editor_mode: bool) -> Self {
        self.input_text = input.clone();
        self.parsed_data = parsed_to_vec(&parse_text(&self.input_text));
        self.editor_mode = editor_mode;
        self
    }
    #[cfg(not(target_os = "windows"))]
    pub fn start_following_hwnd(&mut self, _hwnd: usize) {
        // No-op on non-Windows platforms
    }
    #[cfg(target_os = "windows")]
    pub fn start_following_hwnd(&mut self, hwnd: usize) {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                unsafe {
                    if hwnd == 0
                        || IsWindow(hwnd as _) == 0
                        || !winapi::um::winuser::IsWindowVisible(hwnd as _) != 0
                    {
                        eprintln!("Window 0x{:X} is gone or invalid! Beeping...", hwnd);
                        MessageBeep(0xFFFFFFFF);
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
                unsafe {
                    let msg = format!("Following HWND: 0x{:X}", hwnd);
                    let wide: Vec<u16> = OsString::from(msg)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    let caption: Vec<u16> = OsString::from("e_window Follow")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    MessageBoxW(
                        std::ptr::null_mut(),
                        wide.as_ptr(),
                        caption.as_ptr(),
                        winapi::um::winuser::MB_OK,
                    );
                }
            }
        });
        // Store running in self if you want to stop it later
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        if self.first_frame {
            println!("Initial window setup: {:?}", self.initial_window);
            if let Some(storage) = frame.storage_mut() {
                eframe::set_value(storage, "app", self);
            }
            if let Some((w, h, x, y, ref title)) = self.initial_window {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
                eprintln!(
                    "Setting window position to: ({}, {}) and size to: ({}, {})",
                    x, y, w, h
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
                eprintln!("Initial window title: {}", title);
                eprintln!(
                    "Initial window dimensions: width={}, height={}, x={}, y={}",
                    w, h, x, y
                );

                // Get HWND and update title
                #[cfg(target_os = "windows")]
                unsafe {
                    let hwnd = GetForegroundWindow();
                    if !hwnd.is_null() {
                        let hwnd_val = hwnd as usize;
                        let mut new_title = format!("{title} | HWND: 0x{:X}", hwnd_val);
                        if let Some(hwnd) = self.follow_hwnd {
                            new_title = format!("{new_title} | FOLLOW 0x{:X}", hwnd);
                        } else {
                            new_title = format!("{new_title} | NO FOLLOW");
                        }

                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(new_title));
                    }
                }
            }
            if let Some(hwnd) = self.follow_hwnd {
                if !self.follow_triggered {
                    #[cfg(target_os = "windows")]
                    unsafe {
                        winapi::um::winuser::MessageBeep(0xFFFFFFFF);
                    }
                    eprintln!("Starting to follow HWND: 0x{:X}", hwnd);

                    self.start_following_hwnd(hwnd);
                    self.follow_triggered = true;
                }
            }
            self.first_frame = false;
        }

        // Parse the input text into a ParsedText struct
        let parsed = parse_text(&self.input_text);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            // Card-like background
            egui::Frame::group(ui.style())
                .fill(ui.visuals().panel_fill)
                // .rounding(egui::Rounding::same(12))
                .stroke(egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Title
                        if let Some(title) = &parsed.title {
                            ui.heading(title);
                        }
                        // Header
                        if let Some(header) = &parsed.header {
                            ui.label(
                                egui::RichText::new(header).strong().size(
                                    ui.style().text_styles[&egui::TextStyle::Heading].size * 0.8,
                                ), // egui::RichText::new(header).strong()
                            );
                        }
                        // Caption
                        if let Some(caption) = &parsed.caption {
                            ui.label(egui::RichText::new(caption).italics());
                        }
                        ui.add_space(8.0);

                        // Triples Table
                        if !parsed.triples.is_empty() {
                            ui.label(egui::RichText::new("Fields:").underline());
                            egui::Grid::new("triples_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Key").strong());
                                    ui.label(egui::RichText::new("Value").strong());
                                    ui.label(egui::RichText::new("Type").strong());
                                    ui.end_row();
                                    // Start time and timer as first fields
                                    ui.label("Started");
                                    ui.label(&self.start_datetime);
                                    ui.label("datetime");
                                    ui.end_row();

                                    // Compute elapsed_str here for use in editor_mode
                                    let elapsed = self
                                        .start_time
                                        .as_ref()
                                        .map(|t| t.elapsed())
                                        .unwrap_or_default();
                                    let elapsed_str = format!(
                                        "{:02}:{:02}:{:02}.{:03}",
                                        elapsed.as_secs() / 3600,
                                        (elapsed.as_secs() / 60) % 60,
                                        elapsed.as_secs() % 60,
                                        elapsed.subsec_millis()
                                    );

                                    ui.label("Timer");
                                    ui.label(&elapsed_str);
                                    ui.label("duration");
                                    ui.end_row();

                                    // Then the rest of the triples
                                    for (k, v, t) in &parsed.triples {
                                        ui.label(k);
                                        ui.label(v);
                                        ui.label(egui::RichText::new(t).monospace());
                                        ui.end_row();
                                    }
                                });
                        }

                        // Body
                        if let Some(body) = &parsed.body {
                            ui.separator();
                            ui.label(body);
                        }
                    });
                });

            // Editing and parsing area
            if self.editor_mode {
                if ui.button("Parse").clicked() {
                    self.parsed_data = parsed_to_vec(&parse_text(&self.input_text));
                }
                if ui.button("Run in new window").clicked() {
                    #[cfg(target_os = "windows")]
                    {
                        use std::ffi::OsStr;
                        
                        use std::os::windows::ffi::OsStrExt;
                        
                        
                        use winapi::um::winuser::{MessageBoxW, MB_OK};
                        let _ = std::process::Command::new("e_window")
                            //.creation_flags(0x00000008) // CREATE_NO_WINDOW
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .and_then(|mut child| {
                                if let Some(stdin) = child.stdin.as_mut() {
                                    fn to_wide(s: &str) -> Vec<u16> {
                                        OsStr::new(s)
                                            .encode_wide()
                                            .chain(std::iter::once(0))
                                            .collect()
                                    }

                                    let wide_text = to_wide(&self.input_text);
                                    let wide_caption = to_wide("e_window Input");
                                    unsafe {
                                        MessageBoxW(
                                            std::ptr::null_mut(),
                                            wide_text.as_ptr(),
                                            wide_caption.as_ptr(),
                                            MB_OK,
                                        );
                                    }
                                    stdin.write_all(self.input_text.as_bytes())?;
                                }
                                Ok(())
                            });
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let mut child = std::process::Command::new("e_window")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .expect("Failed to start e_window");
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(self.input_text.as_bytes());
                        }
                    }
                }
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.heading("📇 e_window default editor");
                });

                // Begin scroll area for everything below the editor heading
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        // Show the first line (CLI args) as a code block
                        if let Some(first_line) = self.input_text.lines().next() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Parsed CLI Arguments:")
                                    .underline()
                                    .small(),
                            );
                            ui.code(first_line);
                        }
                        ui.heading("Edit or Paste Input Below:");
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut self.input_text)
                                    .desired_rows(6)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed()
                        {
                            self.parsed_data = parsed_to_vec(&parse_text(&self.input_text));
                            if let Some(new_title) = extract_title_from_first_line(&self.input_text)
                            {
                                #[cfg(target_os = "windows")]
                                unsafe {
                                    let hwnd = GetForegroundWindow();
                                    if !hwnd.is_null() {
                                        let hwnd_val = hwnd as usize;
                                        let mut new_title =
                                            format!("{new_title} | SELF: 0x{:X}", hwnd_val);
                                        if let Some(hwnd) = self.follow_hwnd {
                                            new_title =
                                                format!("{new_title} | FOLLOW 0x{:X}", hwnd);
                                        }
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                                            new_title,
                                        ));
                                    }
                                }
                                #[cfg(not(target_os = "windows"))]
                                ctx.send_viewport_cmd(egui::ViewportCommand::Title(new_title));
                            }
                        }
                        ui.label("Parsed Output:");
                        for line in &self.parsed_data {
                            ui.label(line);
                        }
                    });
            } else {
                // Show timer and start info
                let elapsed = self
                    .start_time
                    .as_ref()
                    .map(|t| t.elapsed())
                    .unwrap_or_default();
                let elapsed_str = format!(
                    "{:02}:{:02}:{:02}.{:03}",
                    elapsed.as_secs() / 3600,
                    (elapsed.as_secs() / 60) % 60,
                    elapsed.as_secs() % 60,
                    elapsed.subsec_millis()
                );

                ui.label(format!("Started: {}", self.start_datetime));
                ui.label(format!("Timer:   {}", elapsed_str));
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if ui.button("OK").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            }
        });
    }
}

fn extract_title_from_first_line(input_text: &str) -> Option<String> {
    let first_line = input_text.lines().next().unwrap_or("");
    let args = shell_words::split(first_line).ok()?;
    let mut opts = getargs::Options::new(args.iter().map(String::as_str));
    while let Some(arg) = opts.next_arg().ok()? {
        if let getargs::Arg::Long("title") = arg {
            if let Ok(val) = opts.value() {
                return Some(val.to_string());
            }
        }
    }
    None
}

pub const DEFAULT_CARD_TEMPLATE: &str = r#"--title "Demo: e_window" --width 1024 --height 768 --x 200 --y 200 --follow-hwnd {PARENT_HWND}
name | e_window | string
version | 1.0 | string
author | GitHub Copilot | string

Welcome to e_window!
This demo shows how you can use this tool to display and edit "index cards" with structured data.

How to use:
- The **first line** can contain command-line options (e.g. --title, --width, --height, --x, --y, --follow-hwnd) to control the window.
- The `--follow-hwnd` option will make this window beep when the parent window (with the given HWND) closes.
- The lines before the first blank line are parsed as `key | value | type` triples and shown in the Fields table.
- After the first blank line:
    - The next line is the **Title**
    - The next line is the **Header**
    - The next line is the **Caption**
    - The rest is the **Body** (supports multiple lines)

Try editing the text below, or click "Run in new window" to open another instance with your changes!
"#;

pub fn default_card_with_hwnd(hwnd: usize) -> String {
    DEFAULT_CARD_TEMPLATE.replace("{PARENT_HWND}", &format!("0x{:X}", hwnd))
}
