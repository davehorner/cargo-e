// This file contains the App struct which implements the eframe::App trait.
// It manages the state of the application, including the text input and display logic.
// It has methods for handling user input and rendering the parsed text.

use crate::control::ControlCommand;
use crate::parser::{parse_text, ParsedText};
use eframe::egui;
use eframe::Storage;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetForegroundWindow;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{IsWindow, MessageBeep};
#[cfg(target_os = "windows")]
use winapi::um::winuser::{PostMessageW, WM_NULL};

#[derive(Debug, Clone)]
pub struct RectAnimation {
    pub start_rect: (i32, i32, u32, u32),
    pub end_rect: (i32, i32, u32, u32),
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
    pub easing: String,
}

#[derive(Serialize, Deserialize)]
pub struct App {
    #[serde(skip)]
    input_text: String,
    parsed_data: ParsedText,
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
    #[serde(skip)]
    pub decode_debug: bool,
    #[serde(skip)]
    pub follow_running: Option<Arc<AtomicBool>>, // Add this field to the App struct
    #[serde(skip)]
    pub doc_buffer: Vec<String>, // For document buffering
    #[serde(skip)]
    control_rx: Option<std::sync::mpsc::Receiver<crate::control::ControlCommand>>, // For control commands
    #[serde(skip)]
    pub current_title: String, // Live window title
    #[serde(skip)]
    pub pending_delay: Option<std::time::Instant>, // For delay command
    #[serde(skip)]
    pub rect_animation: Option<RectAnimation>, // For SetRectEased animation
    #[serde(skip)]
    pub pending_exit: std::sync::Arc<std::sync::atomic::AtomicBool>, // Signal to close window in update
    #[serde(skip)]
    pub in_document: bool, // Track if currently in a document block
    #[serde(skip)]
    pub document_args_line: Option<String>, // Track the argument line for document streaming
    #[serde(skip)]
    pub current_rect: (i32, i32, u32, u32), // Track current window position and size
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
        let input_text = String::new();
        let parsed_data = parse_text(&input_text, true);
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
            decode_debug: false,
            follow_running: None,
            doc_buffer: Vec::new(),
            control_rx: None,
            current_title: "e_window".to_string(),
            pending_delay: None,
            rect_animation: None,
            pending_exit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            in_document: false,
            document_args_line: None,
            current_rect: (200, 200, 1024, 768), // Default rect
        }
    }
}
impl App {
    // Send a synthetic event to the window to force it to process pending changes (Windows only)
    #[cfg(target_os = "windows")]
    fn send_synthetic_event() {
        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.is_null() {
                PostMessageW(hwnd, WM_NULL, 0, 0);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn send_synthetic_event() {}
    pub fn with_control_receiver(
        mut self,
        rx: std::sync::mpsc::Receiver<crate::control::ControlCommand>,
    ) -> Self {
        self.control_rx = Some(rx);
        self
    }
}

// Handle incoming control commands
impl App {
    pub fn handle_control(&mut self, cmd: ControlCommand, ctx: Option<&egui::Context>) {
        eprintln!("[App] Handling control command: {:?}", cmd);
        match cmd {
            ControlCommand::SetRect { x, y, w, h } => {
                eprintln!("[App] Received SetRect: x={}, y={}, w={}, h={}", x, y, w, h);
                self.current_rect = (x, y, w, h);
                if let Some(ctx) = ctx {
                    self.rect_animation = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        w as f32, h as f32,
                    )));
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        x as f32, y as f32,
                    )));
                    eprintln!("[App] Applied SetRect to viewport");
                    ctx.request_repaint();
                    ctx.request_repaint_after(std::time::Duration::from_millis(16));
                    Self::send_synthetic_event();
                }
            }
            ControlCommand::SetTitle(title) => {
                eprintln!("[App] Received SetTitle: {}", title);
                self.current_title = title.clone();
                if let Some(ctx) = ctx {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                    eprintln!("[App] Applied SetTitle to viewport");
                    ctx.request_repaint();
                }
            }
            ControlCommand::BeginDocument => {
                eprintln!("[App] Received BeginDocument");
                self.doc_buffer.clear();
                self.in_document = true;
                self.document_args_line = None;
            }
            ControlCommand::EndDocument => {
                eprintln!("[App] Received EndDocument");
                let new_input = match &self.document_args_line {
                    Some(args_line) => {
                        let mut lines = vec![args_line.clone()];
                        lines.extend(self.doc_buffer.iter().cloned());
                        lines.join("\n")
                    }
                    None => self.doc_buffer.join("\n"),
                };
                self.input_text = new_input.clone();
                self.parsed_data = parse_text(&new_input, self.decode_debug);
                self.in_document = false;
                self.doc_buffer.clear();
            }
            ControlCommand::Delay(ms) => {
                eprintln!("[App] Received Delay: {} ms", ms);
                self.pending_delay =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(ms as u64));
            }
            ControlCommand::SetRectEased {
                x,
                y,
                w,
                h,
                duration_ms,
                easing,
            } => {
                eprintln!("[App] Received SetRectEased: x={}, y={}, w={}, h={}, duration_ms={}, easing={}", x, y, w, h, duration_ms, easing);
                let now = std::time::Instant::now();
                let (start_x, start_y, start_w, start_h) = self.current_rect;
                self.rect_animation = Some(RectAnimation {
                    start_rect: (start_x, start_y, start_w, start_h),
                    end_rect: (x, y, w, h),
                    start_time: now,
                    duration: std::time::Duration::from_millis(duration_ms as u64),
                    easing,
                });
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            }
            ControlCommand::Exit => {
                eprintln!("[App] Received Exit command. Closing window.");
                if let Some(ctx) = ctx {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    eprintln!("[App] No context available to close viewport.");
                }
            }
            ControlCommand::Content(line) => {
                println!("[App] Received Content: {}", &line);
                for (i, line) in self.doc_buffer.iter().enumerate() {
                    println!("doc_buffer[{}]: {}", i, line);
                }
                if self.in_document {
                    // First line after BeginDocument is argument line, skip adding to doc_buffer
                    if self.document_args_line.is_none() {
                        self.document_args_line = Some(line.clone());
                    } else {
                        self.doc_buffer.push(line);
                    }
                } else {
                    // If this is the first content received, replace the default card
                    if self.doc_buffer.is_empty() {
                        self.input_text = line.clone();
                        self.parsed_data = parse_text(&self.input_text, self.decode_debug);
                        self.doc_buffer.push(line);
                    } else {
                        self.doc_buffer.push(line);
                        self.input_text = self.doc_buffer.join("\n");
                        self.parsed_data = parse_text(&self.input_text, self.decode_debug);
                    }
                }
            }
        }
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    }
}

impl App {
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub fn with_initial_window(
        width: f32,
        height: f32,
        x: f32,
        y: f32,
        title: String,
        storage: Option<&dyn Storage>,
        follow_hwnd: Option<usize>,
        decode_debug: bool,
    ) -> Self {
        // Only restore from storage if app is not already initialized
        let mut app = App::default();
        let should_restore = app.first_frame && app.initial_window.is_none();
        if should_restore {
            if let Some(storage) = storage {
                if let Some(restored) = eframe::get_value::<App>(storage, "app") {
                    // Use restored current_rect if available, else fallback to default
                    let restored_rect = restored.current_rect;
                    return App {
                        input_text: String::new(), // Do not restore input_text
                        parsed_data: restored.parsed_data.clone(),
                        first_frame: true,
                        initial_window: Some((width, height, x, y, title.clone())),
                        editor_mode: false,
                        start_time: Some(Instant::now()),
                        start_datetime: chrono::Local::now()
                            .format("%Y-%m-%d %H:%M:%S%.3f")
                            .to_string(),
                        follow_hwnd,
                        follow_triggered: restored.follow_triggered,
                        decode_debug,
                        follow_running: restored.follow_running.clone(),
                        doc_buffer: Vec::new(),
                        control_rx: None,
                        current_title: title.clone(),
                        pending_delay: None,
                        rect_animation: None,
                        pending_exit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                            false,
                        )),
                        in_document: false,
                        document_args_line: None,
                        current_rect: restored_rect,
                    };
                }
            }
        }
        app.initial_window = Some((width, height, x, y, title.clone()));
        app.doc_buffer = app.input_text.lines().map(|s| s.to_string()).collect();
        app.control_rx = None;
        app.current_title = title;
        app.pending_exit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        app.current_rect = (x as i32, y as i32, width as u32, height as u32);
        app
    }

    #[allow(dead_code)]
    pub fn with_input_data(mut self, input: String) -> Self {
        // if input.trim().is_empty() {
        //     let hwnd = {
        //         #[cfg(target_os = "windows")]
        //         {
        //             unsafe { GetForegroundWindow() as usize }
        //         }
        //         #[cfg(not(target_os = "windows"))]
        //         {
        //             0
        //         }
        //     };
        //     self.input_text = default_card_with_hwnd(hwnd);
        // } else {
        self.input_text = input.clone();
        // }
        self.parsed_data = parse_text(&self.input_text, self.decode_debug); // Changed to ParsedText
        self
    }

    #[allow(dead_code)]
    pub fn with_input_data_and_mode(mut self, input: String, editor_mode: bool) -> Self {
        self.input_text = input.clone();
        self.parsed_data = parse_text(&self.input_text, self.decode_debug); // Changed to ParsedText
        self.editor_mode = editor_mode;
        println!("Editor mode set to: {}", self.editor_mode);
        self
    }
    #[cfg(not(target_os = "windows"))]
    pub fn start_following_hwnd(&mut self, _hwnd: usize) {
        // No-op on non-Windows platforms
    }
    #[cfg(target_os = "windows")]
    pub fn start_following_hwnd(&mut self, hwnd: usize) {
        let running = Arc::new(AtomicBool::new(true));
        self.follow_running = Some(running.clone()); // Store the flag in the struct

        thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
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
            }
            println!("Follow thread for HWND 0x{:X} has exited.", hwnd);
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        println!("App is being dropped.");
        if let Some(running) = &self.follow_running {
            running.store(false, Ordering::Relaxed); // Signal the thread to stop
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(16)); // Reliable periodic refresh (60 FPS)
                                                                         // Check for pending exit signal
        if self.pending_exit.load(std::sync::atomic::Ordering::SeqCst) {
            println!("[App] update: Closing window due to pending_exit signal.");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.pending_exit
                .store(false, std::sync::atomic::Ordering::SeqCst);
            return;
        }
        if self.first_frame {
            println!("Initial window setup: {:?}", self.initial_window);
            // Only set storage on first frame to avoid repeated App drops
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
                        let pid = std::process::id();
                        new_title = format!("{new_title} | PID: {}", pid);
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

        // Pause control command processing if a delay or animation is active
        let mut skip_control = false;
        // Check for pending delay
        if let Some(delay_until) = self.pending_delay {
            if std::time::Instant::now() < delay_until {
                skip_control = true;
            } else {
                self.pending_delay = None;
            }
        }
        // Check for active animation
        if self.rect_animation.is_some() {
            skip_control = true;
        }

        // Queue control commands and only process one per frame when not skipping
        if let Some(rx) = &mut self.control_rx {
            // Maintain a queue of pending commands
            if self.doc_buffer.is_empty() {
                self.doc_buffer = Vec::new();
            }
            // Use a local queue for control commands
            if self.pending_delay.is_none() && self.rect_animation.is_none() {
                if let Ok(cmd) = rx.try_recv() {
                    eprintln!("[App] Received control command: {:?}", cmd);
                    self.handle_control(cmd, Some(ctx));
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut close_requested = false;
            if ctx.input(|i| i.key_pressed(egui::Key::Enter))
                || ctx.input(|i| i.key_pressed(egui::Key::Escape))
            {
                close_requested = true;
            }
            let total_height = ui.available_height();
            let (card_height, editor_height) = if self.editor_mode {
                (total_height * 0.5, total_height * 0.5)
            } else {
                (total_height, 0.0)
            };
            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.set_height(card_height);
                    ui.vertical(|ui| {
                        // Title
                        if let Some(title) = &self.parsed_data.title {
                            if !title.is_empty() {
                                ui.heading(title);
                            }
                        }
                        // Header
                        if let Some(header) = &self.parsed_data.header {
                            if !header.is_empty() {
                                ui.label(egui::RichText::new(header).strong().size(
                                    ui.style().text_styles[&egui::TextStyle::Heading].size * 0.8,
                                ));
                            }
                        }
                        // Caption
                        if let Some(caption) = &self.parsed_data.caption {
                            if !caption.is_empty() {
                                ui.label(egui::RichText::new(caption).italics());
                            }
                        }
                        // Display anchors with clickable labels
                        if !self.parsed_data.anchors.is_empty() {
                            for anchor in &self.parsed_data.anchors {
                                if ui.button(&anchor.text).clicked() {
                                    if anchor.href.starts_with("http://")
                                        || anchor.href.starts_with("https://")
                                    {
                                        if let Err(err) = open::that(&anchor.href) {
                                            eprintln!(
                                                "Failed to open URL {}: {}",
                                                anchor.href, err
                                            );
                                        }
                                    } else {
                                        let mut parts = shell_words::split(&anchor.href)
                                            .unwrap_or_else(|_| vec![]);
                                        if !parts.is_empty() {
                                            let program = parts.remove(0);
                                            let args: Vec<&str> =
                                                parts.iter().map(String::as_str).collect();

                                            if let Err(err) = std::process::Command::new(&program)
                                                .args(args)
                                                .spawn()
                                            {
                                                eprintln!(
                                                    "Failed to run command {}: {}",
                                                    anchor.href, err
                                                );
                                            }
                                        } else {
                                            eprintln!("Invalid command: {}", anchor.href);
                                        }
                                    }
                                }
                            }
                        }

                        ui.add_space(8.0);

                        // Triples Table
                        if !self.parsed_data.triples.is_empty() {
                            egui::Grid::new("triples_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Started");
                                    ui.label(&self.start_datetime);
                                    ui.label("datetime");
                                    ui.end_row();

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

                                    for (k, v, t) in &self.parsed_data.triples {
                                        if !k.is_empty() && !v.is_empty() && !t.is_empty() {
                                            ui.label(k);
                                            ui.label(v);
                                            ui.label(egui::RichText::new(t).monospace());
                                            ui.end_row();
                                        }
                                    }
                                });
                        }

                        // Body
                        if let Some(body) = &self.parsed_data.body {
                            ui.separator();
                            ui.label(body);
                        }

                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            if ui.button("OK").clicked() || close_requested {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    });
                });

            // Editing and parsing area below the card
            if self.editor_mode {
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.heading("📇 e_window default editor");
                });

                if ui.button("Parse").clicked() {
                    self.parsed_data = parse_text(&self.input_text, self.decode_debug);
                }
                if ui.button("Run in new window").clicked() {
                    #[cfg(target_os = "windows")]
                    {
                        use std::ffi::OsStr;
                        use std::os::windows::ffi::OsStrExt;
                        use winapi::um::winuser::{MessageBoxW, MB_OK};
                        // Show the message box with the editor content
                        let wide_text = OsStr::new(&self.input_text)
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect::<Vec<u16>>();
                        let wide_caption = OsStr::new("e_window Input")
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect::<Vec<u16>>();
                        unsafe {
                            MessageBoxW(
                                std::ptr::null_mut(),
                                wide_text.as_ptr(),
                                wide_caption.as_ptr(),
                                MB_OK,
                            );
                        }
                        // Pass the editor content as a positional argument and write to stdin
                        let mut child = std::process::Command::new("e_window")
                            .arg(&self.input_text)
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .expect("Failed to start e_window");
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(self.input_text.as_bytes());
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let mut child = std::process::Command::new("e_window")
                            .arg(&self.input_text)
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .expect("Failed to start e_window");
                        if let Some(stdin) = child.stdin.as_mut() {
                            let _ = stdin.write_all(self.input_text.as_bytes());
                        }
                    }
                }

                egui::ScrollArea::vertical()
                    .id_salt("editor_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_height(editor_height);
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
                            self.parsed_data = parse_text(&self.input_text, self.decode_debug);
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
                    });
            }
        });

        if let Some(anim) = &self.rect_animation {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(anim.start_time);
            let t = (elapsed.as_secs_f32() / anim.duration.as_secs_f32())
                .min(1.0)
                .max(0.0);
            let ease_t = match anim.easing.as_str() {
                "linear" => t,
                // Add more easing types here
                _ => t,
            };
            let (sx, sy, sw, sh) = anim.start_rect;
            let (ex, ey, ew, eh) = anim.end_rect;
            let nx = sx as f32 + (ex as f32 - sx as f32) * ease_t;
            let ny = sy as f32 + (ey as f32 - sy as f32) * ease_t;
            let nw = sw as f32 + (ew as f32 - sw as f32) * ease_t;
            let nh = sh as f32 + (eh as f32 - sh as f32) * ease_t;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(nw, nh)));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(nx, ny)));
            self.current_rect = (nx as i32, ny as i32, nw as u32, nh as u32);
            ctx.request_repaint();
            if t >= 1.0 {
                self.rect_animation = None;
                Self::send_synthetic_event();
            }
        }
    }
} // Added missing closing brace for impl eframe::App for App
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
anchor: Click me! | e_window --title "you clicked!" --width 800 --height 600 --x 100 --y 100
"#;

pub fn default_card_with_hwnd(hwnd: usize) -> String {
    DEFAULT_CARD_TEMPLATE.replace("{PARENT_HWND}", &format!("0x{:X}", hwnd))
}
