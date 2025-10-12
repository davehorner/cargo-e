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
use rfd;
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
    result_sent: bool, // Flag to prevent duplicate result sending
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
    #[serde(skip)]
    pub should_auto_center: bool, // Flag for auto-centering after first frame
    pub box_type: String, // Add a field to store the box_type
    #[serde(skip)]
    pub text_input: String, // Store text input for TextInput message boxes
    #[serde(skip)]
    pub file_input: String, // Store file path input for FileSelection message boxes
    #[serde(skip)]
    pub input_focus_requested: bool, // Flag to request focus on input fields
    #[serde(skip)]
    pub file_dialog_opened: bool, // Flag to track if file dialog was opened
    #[serde(skip)]
    pub auto_close_on_file_select: bool, // Flag to auto-close when file is selected
    #[serde(skip)]
    pub file_dialog_cancelled: bool, // Flag to track if file dialog was cancelled
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
            control_rx: None, // Revert to default initialization as None
            result_sent: false,
            current_title: "e_window".to_string(),
            pending_delay: None,
            rect_animation: None,
            pending_exit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            in_document: false,
            document_args_line: None,
            current_rect: (0, 0, 0, 0), // Will be set by initial window or content
            should_auto_center: false, // Default to no auto-centering
            box_type: "Ok".to_string(), // Default box_type
            text_input: String::new(), // Initialize empty text input
            file_input: String::new(), // Initialize empty file input
            input_focus_requested: false, // No initial focus request
            file_dialog_opened: false, // No dialog opened initially
            auto_close_on_file_select: false, // No auto-close by default
            file_dialog_cancelled: false, // No dialog cancelled initially
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

                // Auto-center the window after content processing if flag is set
                if let Some(ctx) = ctx {
                    eprintln!("[App] EndDocument: should_auto_center={}", self.should_auto_center);
                    // Only auto-center if the flag is explicitly set
                    if self.should_auto_center {
                        // Schedule a repaint to trigger auto-centering in the next frame
                        ctx.request_repaint();
                        eprintln!("[App] Scheduled auto-centering after content processing");
                    } else {
                        eprintln!("[App] Skipping auto-centering: should_auto_center=false");
                    }
                }
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
        box_type: String, // Added box_type parameter
    ) -> Self {
        eprintln!("[DEBUG] with_initial_window called with: width={}, height={}, x={}, y={}, title='{}', box_type='{}'", width, height, x, y, title, box_type);
        
        // Try to restore from storage first
        if let Some(storage) = storage {
            if let Some(restored) = eframe::get_value::<App>(storage, "app") {
                let _restored_rect = restored.current_rect;

                let should_auto_center = x == -1.0 && y == -1.0;
                eprintln!("[DEBUG] Storage restore: auto-center check: x={}, y={}, should_auto_center={}", 
                         x, y, should_auto_center);

                let final_rect = if should_auto_center {
                    (0, 0, width as u32, height as u32)
                } else {
                    (x as i32, y as i32, width as u32, height as u32)
                };

                return App {
                    input_text: String::new(),
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
                    control_rx: None, // Revert to default initialization as None
                    result_sent: restored.result_sent,
                    current_title: title.clone(),
                    pending_delay: restored.pending_delay.clone(),
                    rect_animation: restored.rect_animation.clone(),
                    pending_exit: restored.pending_exit.clone(),
                    in_document: restored.in_document,
                    document_args_line: restored.document_args_line.clone(),
                    current_rect: final_rect,
                    should_auto_center,
                    box_type: box_type.clone(),
                    text_input: String::new(), // Initialize empty text input
                    file_input: String::new(), // Initialize empty file input
                    input_focus_requested: false, // No initial focus request
                    file_dialog_opened: false, // No dialog opened initially
                    auto_close_on_file_select: false, // No auto-close by default
                    file_dialog_cancelled: false, // No dialog cancelled initially
                };
            }
        }

        // Default initialization if no storage is available
        let mut app = App::default();
        app.box_type = box_type.clone();
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
        // Get the current thread name for better debugging
        let thread_name = std::thread::current().name().map(|s| s.to_string()).unwrap_or_else(|| "unnamed".to_string());
        println!("App is being dropped. (thread: {})", thread_name);
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
            
            // Calculate optimal sizing for message boxes using actual font metrics
            if !self.box_type.is_empty() {
                // Get message text from parsed data for sizing calculations
                let message_text = if let Some(title) = &self.parsed_data.title {
                    title.as_str()
                } else if let Some(first_triple) = self.parsed_data.triples.first() {
                    &first_triple.2  // The third element is the value
                } else {
                    ""
                };
                
                // Count the number of fields (triples) that will be displayed in the table
                let fields_count = self.parsed_data.triples.len();
                
                if !message_text.is_empty() || fields_count > 0 {
                    // Calculate dimensions based on actual font metrics
                    let font_size = ctx.style().text_styles[&egui::TextStyle::Body].size;
                    
                    // Measure text dimensions more accurately with wrapping
                    let font_id = &ctx.style().text_styles[&egui::TextStyle::Body];
                    
                    // For long text, calculate with reasonable line wrapping (max 500px width)
                    let max_text_width = 500.0;
                    
                    // Create a temporary painter to get access to layout methods
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    
                    let galley = if !message_text.is_empty() {
                        if message_text.len() > 60 { // If text is long, use wrapping
                            painter.layout(
                                message_text.to_string(),
                                font_id.clone(),
                                egui::Color32::WHITE,
                                max_text_width,
                            )
                        } else {
                            painter.layout_no_wrap(
                                message_text.to_string(),
                                font_id.clone(),
                                egui::Color32::WHITE,
                            )
                        }
                    } else {
                        // If no message text, create a minimal galley for sizing calculation
                        painter.layout_no_wrap(
                            "".to_string(),
                            font_id.clone(),
                            egui::Color32::WHITE,
                        )
                    };
                    
                    // Calculate required window size based on text and fields
                    let text_width = galley.size().x + 80.0; // More padding for content margins
                    
                    // Calculate comprehensive height including all UI elements
                    let title_bar_height = 30.0; // Window title bar
                    let content_top_margin = 15.0; // Space at top of content area
                    let content_bottom_margin = 15.0; // Space at bottom of content area
                    
                    // Fields table height (if any fields exist)
                    let fields_table_height = if fields_count > 0 {
                        // Each field takes about 20px + table header (25px) + spacing (20px)
                        (fields_count as f32 * 20.0) + 25.0 + 20.0
                    } else {
                        0.0
                    };
                    
                    // Spacing between fields and content (if both exist)
                    let fields_content_spacing = if fields_count > 0 && !message_text.is_empty() {
                        0.0
                    } else {
                        0.0
                    };
                    
                    // Content sections spacing (title, header, caption, body each need vertical space)
                    let content_sections_height = if !message_text.is_empty() {
                        galley.size().y + 10.0 // Text height + spacing between sections
                    } else {
                        0.0
                    };
                    
                    // Input field height for interactive types
                    let input_field_height = match self.box_type.as_str() {
                        "TextInput" => 40.0, // Text input field + labels
                        "FileSelection" => 60.0, // File path field + browse button + labels
                        _ => 0.0,
                    };
                    
                    // Calculate button area height based on message box type
                    let button_area_height = match self.box_type.as_str() {
                        "Ok" => 40.0,
                        "OkCancel" | "YesNo" | "YesNoDefNo" | "RetryCancel" => 50.0,
                        "YesNoCancel" | "YesNoCancelDefNo" => 60.0, // More height for 3 buttons
                        "TextInput" | "FileSelection" => 150.0, // Extra space for input fields, separators, and labels
                        _ => 50.0,
                    };
                    
                    // Calculate total height including all components
                    let total_height = title_bar_height +
                                     content_top_margin +
                                     fields_table_height +
                                     fields_content_spacing +
                                     content_sections_height +
                                     input_field_height +
                                     button_area_height +
                                     40.0 + // padding and spacing
                                     content_bottom_margin;
                    
                    // Apply reasonable bounds - much more generous sizing
                    let final_width = if self.box_type == "TextInput" || self.box_type == "FileSelection" {
                        text_width.clamp(500.0, 800.0) // Much wider for input fields
                    } else {
                        text_width.clamp(400.0, 700.0) // Much wider for all message boxes
                    };
                    let final_height = if self.box_type == "TextInput" || self.box_type == "FileSelection" {
                        total_height.clamp(350.0, 600.0) // Much taller for input fields
                    } else {
                        total_height.clamp(250.0, 500.0) // Much taller for message boxes
                    };
                    
                    eprintln!("[DEBUG] Sizing with min dimensions: font_size={}, fields_count={}, fields_height={}, content_height={}, input_height={}, button_height={}, total_height={}, text='{}', final={}x{}", 
                        font_size, fields_count, fields_table_height, content_sections_height, input_field_height, button_area_height, total_height, message_text, final_width, final_height);
                    
                    // Update window size with calculated dimensions
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(final_width, final_height)));
                    
                    // Center the window after sizing
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(f32::INFINITY, f32::INFINITY)));
                }
            }
            
            if let Some((w, h, x, y, ref title)) = self.initial_window {
                eprintln!("[DEBUG] Sending viewport commands for size and title");
                // Only set explicit size if both width and height are specified (> 0)
                // If w=0 or h=0, let the font-based sizing take precedence
                if w > 0.0 && h > 0.0 {
                    eprintln!("Setting explicit window size to: ({}, {})", w, h);
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
                } else {
                    eprintln!("[DEBUG] Skipping size setting (w={}, h={}) - using font-based sizing", w, h);
                }
                // Only set position if not auto-centering (x != -1, y != -1)
                if x != -1.0 && y != -1.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
                    eprintln!("Setting window position to: ({}, {})", x, y);
                } else {
                    eprintln!("[DEBUG] Skipping position setting for auto-centering");
                }
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

            // Debug: Show actual window dimensions after viewport commands
            let actual_size = ctx.input(|i| i.content_rect().size());
            eprintln!("[DEBUG] Actual window size after viewport commands: {}x{}", actual_size.x, actual_size.y);
            eprintln!("[DEBUG] should_auto_center flag: {}", self.should_auto_center);

            // Auto-center the window if requested and content size is now available
            if self.should_auto_center {
                eprintln!("[DEBUG] Applying auto-centering with content size: {}x{}", actual_size.x, actual_size.y);

                // Cross-platform screen size detection
                let (screen_width, screen_height, method_used) = e_window_native::get_cross_platform_screen_size();

                // Check if the window should be resized to fit content instead of using default size
                // This handles the case where egui applies a default size (like 1024x768) instead of content size
                let content_size = ctx.available_rect().size();
                eprintln!("[DEBUG] Available content area: {}x{}", content_size.x, content_size.y);

                // Use the actual content size for centering if it's smaller than the window
                let (final_width, final_height) = if content_size.x < actual_size.x && content_size.y < actual_size.y {
                    eprintln!("[DEBUG] Window is larger than content, resizing to fit content");
                    // Add some padding around the content
                    let padding = 40.0;
                    let target_width = content_size.x + padding;
                    let target_height = content_size.y + padding;

                    // Resize window to fit content
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(target_width, target_height)));
                    (target_width as u32, target_height as u32)
                } else {
                    (actual_size.x as u32, actual_size.y as u32)
                };

                // Calculate center position based on final window size
                if screen_width > final_width && screen_height > final_height {
                    let center_x = (screen_width as f32 - final_width as f32) / 2.0;
                    let center_y = (screen_height as f32 - final_height as f32) / 2.0;

                    eprintln!("[DEBUG] Auto-centering ({}): screen={}x{}, window={}x{}, center=({}, {})", 
                             method_used, screen_width, screen_height, final_width, final_height, center_x, center_y);

                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(center_x, center_y)));
                    self.current_rect = (center_x as i32, center_y as i32, final_width as u32, final_height as u32);
                } else {
                    eprintln!("[DEBUG] Could not determine valid screen size (got {}x{}), skipping auto-center", 
                             screen_width, screen_height);
                }

                self.should_auto_center = false; // Only auto-center once
                eprintln!("[DEBUG] Auto-centering applied successfully, setting should_auto_center=false");
            }

            // Output ready signal for API timing
            eprintln!("[e_window] READY");
        }

        // Report actual window size after any content changes
        let current_size = ctx.input(|i| i.content_rect().size());
        let current_pos = ctx.input(|i| i.content_rect().min);

        // Check if size/position changed and report it
        let current_rect = (current_pos.x as i32, current_pos.y as i32, current_size.x as u32, current_size.y as u32);
        if current_rect != self.current_rect {
            let old_size = (self.current_rect.2, self.current_rect.3);
            let new_size = (current_size.x as u32, current_size.y as u32);

            self.current_rect = current_rect;
            eprintln!("[e_window] WINDOW_INFO size={}x{} pos={}x{}", 
                     current_size.x, current_size.y, current_pos.x, current_pos.y);

            // Auto-center if the window size changed due to content processing
            // and we're still at origin position AND auto-centering is enabled
            if self.should_auto_center && old_size != new_size && current_pos.x as i32 == 0 && current_pos.y as i32 == 0 {
                // Use cross-platform screen detection
                let (screen_width, screen_height, method_used) = e_window_native::get_cross_platform_screen_size();
                let new_center_x = ((screen_width as f32 - new_size.0 as f32) / 2.0).round() as i32;
                let new_center_y = ((screen_height as f32 - new_size.1 as f32) / 2.0).round() as i32;

                eprintln!("[App] Auto-centering ({}): old_size={}x{}, new_size={}x{}, screen={}x{}, new_center=({}, {})", 
                         method_used, old_size.0, old_size.1, new_size.0, new_size.1, screen_width, screen_height, new_center_x, new_center_y);

                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    new_center_x as f32, new_center_y as f32,
                )));
                self.current_rect = (new_center_x, new_center_y, new_size.0, new_size.1);
                ctx.request_repaint();
            }
        }

        // Pause control command processing if a delay or animation is active
        let mut _skip_control = false;
        // Check for pending delay
        if let Some(delay_until) = self.pending_delay {
            if std::time::Instant::now() < delay_until {
                _skip_control = true;
            } else {
                self.pending_delay = None;
            }
        }
        // Check for active animation
        if self.rect_animation.is_some() {
            _skip_control = true;
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
            // Use the box_type field instead of parsing from triples, with fallback for compatibility
            let msg_box_type = if !self.box_type.is_empty() && self.box_type != "Ok" {
                self.box_type.as_str()
            } else {
                // Fallback to parsing from triples for backward compatibility
                self.parsed_data.triples.iter()
                    .find(|(k, _, t)| k == "type" && t == "string")
                    .map(|(_, v, _)| v.as_str())
                    .unwrap_or("Ok")
            };

            let default_button = self.parsed_data.triples.iter()
                .find(|(k, _, t)| k == "default_button" && t == "string")
                .map(|(_, v, _)| v.as_str());

            let mut close_requested = false;
            let mut keyboard_result = None;

            // Check for window close request (X button)
            ctx.input(|i| {
                if i.viewport().close_requested() {
                    close_requested = true;
                    // Set appropriate result for window close based on message box type
                    keyboard_result = match msg_box_type {
                        "RetryCancel" => Some("Cancel"),
                        "OkCancel" => Some("Cancel"), 
                        "YesNoCancel" => Some("Cancel"),
                        "YesNoCancelDefNo" => Some("Cancel"),
                        "YesNo" => Some("No"),
                        "YesNoDefNo" => Some("No"),
                        "TextInput" => Some("Cancel"),
                        "FileSelection" => Some("Cancel"),
                        _ => Some("Ok"),
                    };
                }
            });

            // Handle keyboard input based on message box type
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    // Special handling for FileSelection when input is empty
                    if msg_box_type == "FileSelection" && self.file_input.trim().is_empty() {
                        // Open file dialog instead of closing
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Select File")
                            .pick_file() {
                            self.file_input = path.display().to_string();
                            close_requested = true;
                            keyboard_result = Some("FileSelected");
                        }
                        // If cancelled, don't close - continue with the message box
                    } else {
                        close_requested = true;
                        keyboard_result = match msg_box_type {
                            "YesNo" => Some(if default_button == Some("No") { "No" } else { "Yes" }),
                            "YesNoDefNo" => Some("No"), // Always default to No
                            "YesNoCancel" => Some(default_button.unwrap_or("Yes")),
                            "YesNoCancelDefNo" => Some("No"), // Always default to No
                            "OkCancel" => Some("Ok"),
                            "RetryCancel" => Some("Retry"),
                            "TextInput" => Some("TextInput"),
                            "FileSelection" => Some("FileSelected"),
                            _ => Some("Ok"),
                        };
                    }
                } else if i.key_pressed(egui::Key::Escape) {
                    close_requested = true;
                    keyboard_result = match msg_box_type {
                        "YesNo" => Some("No"),
                        "YesNoDefNo" => Some("No"),
                        "YesNoCancel" => Some("Cancel"),
                        "YesNoCancelDefNo" => Some("Cancel"),
                        "OkCancel" => Some("Cancel"),
                        "RetryCancel" => Some("Cancel"),
                        "TextInput" => Some("Cancel"),
                        "FileSelection" => Some("Cancel"),
                        _ => Some("Ok"),
                    };
                } else if i.key_pressed(egui::Key::Y) {
                    if matches!(msg_box_type, "YesNo" | "YesNoDefNo" | "YesNoCancel" | "YesNoCancelDefNo") {
                        close_requested = true;
                        keyboard_result = Some("Yes");
                    }
                } else if i.key_pressed(egui::Key::N) {
                    if matches!(msg_box_type, "YesNo" | "YesNoDefNo" | "YesNoCancel" | "YesNoCancelDefNo") {
                        close_requested = true;
                        keyboard_result = Some("No");
                    }
                } else if i.key_pressed(egui::Key::R) {
                    if msg_box_type == "RetryCancel" {
                        close_requested = true;
                        keyboard_result = Some("Retry");
                    }
                }
            });

            if let Some(result) = keyboard_result {
                if !self.result_sent {
                    self.result_sent = true;
                    
                    // Format result with input data if applicable
                    let final_result = match result {
                        "TextInput" => {
                            if self.text_input.is_empty() {
                                "TextInput:".to_string()
                            } else {
                                format!("TextInput:{}", self.text_input)
                            }
                        },
                        "FileSelected" => {
                            if self.file_input.is_empty() {
                                "FileSelected:".to_string()
                            } else {
                                format!("FileSelected:{}", self.file_input)
                            }
                        },
                        _ => result.to_string()
                    };
                    
                    println!("[App] Keyboard result: {}", final_result);
                    // Send result to stdout for API caller to read
                    println!("RESULT:{}", final_result);
                    std::io::stdout().flush().unwrap_or_default();

                    // Add a small delay to ensure the result gets flushed before closing
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    // Exit the application after sending result
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
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
                        // Set much larger minimum dimensions to force proper sizing
                        ui.set_min_width(400.0);
                        ui.set_min_height(250.0);
                        
                        // Add top padding to ensure proper window sizing
                        // ui.add_space(15.0);
                        
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

                        // ui.add_space(8.0);

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

                                            // Special handling for expiration fields
                                            if t == "expiration" {
                                                if let Ok(duration_secs) = v.parse::<u64>() {
                                                    let elapsed = self
                                                        .start_time
                                                        .as_ref()
                                                        .map(|t| t.elapsed())
                                                        .unwrap_or_default();

                                                    let remaining = duration_secs.saturating_sub(elapsed.as_secs());
                                                    
                                                    if remaining > 0 {
                                                        // Calculate remaining time with milliseconds for higher resolution
                                                        let total_remaining_millis = (duration_secs * 1000) as i64 - elapsed.as_millis() as i64;
                                                        let remaining_secs = (total_remaining_millis / 1000).max(0) as u64;
                                                        let remaining_millis = (total_remaining_millis % 1000).max(0) as u64;
                                                        
                                                        let remaining_str = format!(
                                                            "{:02}:{:02}:{:02}.{:03}",
                                                            remaining_secs / 3600,
                                                            (remaining_secs / 60) % 60,
                                                            remaining_secs % 60,
                                                            remaining_millis
                                                        );
                                                        ui.label(egui::RichText::new(&remaining_str).color(
                                                            if remaining <= 10 { egui::Color32::RED } 
                                                            else if remaining <= 30 { egui::Color32::from_rgb(255, 165, 0) } // Orange color for better visibility
                                                            else { egui::Color32::GREEN }
                                                        ));
                                                    } else {
                                                        ui.label(egui::RichText::new("EXPIRED").color(egui::Color32::RED));
                                                        
                                                        // Send timeout result to stdout and close window
                                                        if !close_requested {
                                                            println!("RESULT:TIMEOUT");
                                                            std::io::stdout().flush().unwrap_or_default();
                                                            
                                                            // Add a small delay to ensure the result gets flushed before closing
                                                            std::thread::sleep(std::time::Duration::from_millis(50));
                                                            
                                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                        }
                                                    }
                                                } else {
                                                    ui.label(v);
                                                }
                                            } else {
                                                ui.label(v);
                                            }
                                            
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

                        // ui.add_space(8.0);
                        
                        // Use the box_type field instead of parsing from triples, with fallback for compatibility
                        let msg_box_type = if !self.box_type.is_empty() && self.box_type != "Ok" {
                            self.box_type.as_str()
                        } else {
                            // Fallback to parsing from triples for backward compatibility
                            self.parsed_data.triples.iter()
                                .find(|(k, _, t)| k == "type" && t == "string")
                                .map(|(_, v, _)| v.as_str())
                                .unwrap_or("Ok")
                        };
                        
                        let default_button = self.parsed_data.triples.iter()
                            .find(|(k, _, t)| k == "default_button" && t == "string")
                            .map(|(_, v, _)| v.as_str());
                        
                        let input_type = self.parsed_data.triples.iter()
                            .find(|(k, _, t)| k == "input_type" && t == "string")
                            .map(|(_, v, _)| v.as_str());
                        
                        // Handle input fields for TextInput and FileSelection types
                        if let Some(input_type) = input_type {
                            ui.separator();
                            // ui.add_space(8.0);
                            
                            match input_type {
                                "text" => {
                                    ui.label("Enter text:");
                                    ui.add_space(4.0);
                                    let text_edit = egui::TextEdit::singleline(&mut self.text_input)
                                        .desired_width(ui.available_width() - 40.0)
                                        .hint_text("Type here...");
                                    
                                    let response = ui.add(text_edit);
                                    
                                    // Request focus on the first frame for text input
                                    if !self.input_focus_requested && self.box_type == "TextInput" {
                                        response.request_focus();
                                        self.input_focus_requested = true;
                                    }
                                },
                                "file" => {
                                    ui.label("File path:");
                                    ui.add_space(4.0);
                                    
                                    ui.horizontal(|ui| {
                                        let text_edit = egui::TextEdit::singleline(&mut self.file_input)
                                            .desired_width(ui.available_width() - 120.0)
                                            .hint_text("Enter file path or press Enter to browse...");
                                        
                                        let response = ui.add(text_edit);
                                        
                                        // Request focus on the first frame for file input
                                        if !self.input_focus_requested && self.box_type == "FileSelection" {
                                            response.request_focus();
                                            self.input_focus_requested = true;
                                        }
                                        
                                        // Check if Enter was pressed on the text input
                                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            // If input is empty and dialog hasn't been cancelled yet, open file dialog
                                            if self.file_input.trim().is_empty() && !self.file_dialog_cancelled {
                                                if let Some(path) = rfd::FileDialog::new()
                                                    .set_title("Select File")
                                                    .pick_file() {
                                                    self.file_input = path.display().to_string();
                                                    
                                                    // Auto-close after file selection
                                                    if !self.result_sent {
                                                        self.result_sent = true;
                                                        let final_result = format!("FileSelected:{}", self.file_input);
                                                        println!("[App] File selected via Enter: {}", final_result);
                                                        println!("RESULT:{}", final_result);
                                                        std::io::stdout().flush().unwrap_or_default();
                                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                        return; // Exit early
                                                    }
                                                } else {
                                                    // User cancelled the dialog, set flag to prevent reopening
                                                    self.file_dialog_cancelled = true;
                                                }
                                                // If cancelled, continue with the message box (don't auto-close)
                                            } else {
                                                // Enter pressed with text - treat as OK with current path
                                                if !self.result_sent {
                                                    self.result_sent = true;
                                                    let final_result = format!("FileSelected:{}", self.file_input);
                                                    println!("[App] File path entered: {}", final_result);
                                                    println!("RESULT:{}", final_result);
                                                    std::io::stdout().flush().unwrap_or_default();
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                    return; // Exit early
                                                }
                                            }
                                        }
                                        
                                        if ui.button("📂 Browse...").clicked() {
                                            // Open file dialog
                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_title("Select File")
                                                .pick_file() {
                                                self.file_input = path.display().to_string();
                                                
                                                // Auto-close after file selection via Browse button
                                                if !self.result_sent {
                                                    self.result_sent = true;
                                                    let final_result = format!("FileSelected:{}", self.file_input);
                                                    println!("[App] File selected via Browse: {}", final_result);
                                                    println!("RESULT:{}", final_result);
                                                    std::io::stdout().flush().unwrap_or_default();
                                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                                    return; // Exit early
                                                }
                                            }
                                            // If cancelled, continue with the message box (don't auto-close)
                                        }
                                    });
                                },
                                _ => {}
                            }
                            // ui.add_space(8.0);
                        }
                        
                        // Add bottom spacer to ensure proper window sizing
                        // ui.add_space(20.0);
                        
                        // Render buttons based on message box type
                        ui.vertical_centered(|ui| {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                // Add flexible space to center the buttons
                                ui.allocate_space(egui::Vec2::new(ui.available_width() * 0.3, 0.0));
                                
                                let mut should_close = close_requested;
                                let mut result_type = "Cancel";
                                
                                match msg_box_type {
                                    "Ok" => {
                                        if ui.button("✅ OK").clicked() {
                                            println!("[DEBUG] OK button clicked - setting should_close = true");
                                            should_close = true;
                                            result_type = "Ok";
                                        }
                                    },
                                    "OkCancel" => {
                                        if ui.button("✅ OK").clicked() {
                                            should_close = true;
                                            result_type = "Ok";
                                        }
                                        if ui.button("❌ Cancel").clicked() {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    "YesNo" => {
                                        if ui.button("✅ Yes").clicked() {
                                            should_close = true;
                                            result_type = "Yes";
                                        }
                                        if ui.button("❌ No").clicked() {
                                            should_close = true;
                                            result_type = "No";
                                        }
                                    },
                                    "YesNoDefNo" => {
                                        // No is the default, so it gets the default styling
                                        if ui.button("✅ Yes").clicked() {
                                            should_close = true;
                                            result_type = "Yes";
                                        }
                                        if ui.button("❌ No (Default)").clicked() || close_requested {
                                            should_close = true;
                                            result_type = "No";
                                        }
                                    },
                                    "YesNoCancel" => {
                                        let default_btn = default_button.unwrap_or("Yes");
                                        
                                        let yes_label = if default_btn == "Yes" { "✅ Yes (Default)" } else { "✅ Yes" };
                                        let no_label = if default_btn == "No" { "❌ No (Default)" } else { "❌ No" };
                                        let cancel_label = if default_btn == "Cancel" { "⚪ Cancel (Default)" } else { "⚪ Cancel" };
                                        
                                        if ui.button(yes_label).clicked() || (close_requested && default_btn == "Yes") {
                                            should_close = true;
                                            result_type = "Yes";
                                        }
                                        if ui.button(no_label).clicked() || (close_requested && default_btn == "No") {
                                            should_close = true;
                                            result_type = "No";
                                        }
                                        if ui.button(cancel_label).clicked() || (close_requested && default_btn == "Cancel") {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    "YesNoCancelDefNo" => {
                                        // No is the default for this variant
                                        if ui.button("✅ Yes").clicked() {
                                            should_close = true;
                                            result_type = "Yes";
                                        }
                                        if ui.button("❌ No (Default)").clicked() || close_requested {
                                            should_close = true;
                                            result_type = "No";
                                        }
                                        if ui.button("⚪ Cancel").clicked() {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    "RetryCancel" => {
                                        if ui.button("🔄 Retry").clicked() {
                                            should_close = true;
                                            result_type = "Retry";
                                        }
                                        if ui.button("❌ Cancel").clicked() {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    "TextInput" => {
                                        if ui.button("📝 OK").clicked() {
                                            should_close = true;
                                            result_type = "TextInput";
                                        }
                                        if ui.button("❌ Cancel").clicked() {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    "FileSelection" => {
                                        if ui.button("📁 Select").clicked() {
                                            should_close = true;
                                            result_type = "FileSelected";
                                        }
                                        if ui.button("❌ Cancel").clicked() {
                                            should_close = true;
                                            result_type = "Cancel";
                                        }
                                    },
                                    _ => {
                                        // Default fallback
                                        if ui.button("✅ OK").clicked() || close_requested {
                                            should_close = true;
                                            result_type = "Ok";
                                        }
                                    }
                                }
                                
                                // Add flexible space to balance the centering
                                // ui.allocate_space(egui::Vec2::new(ui.available_width() * 0.3, 0.0));
                                
                                if should_close {
                                    println!("[DEBUG] Button should_close triggered, result_sent: {}", self.result_sent);
                                    eprintln!("[STDERR DEBUG] Button should_close triggered, result_sent: {}", self.result_sent);
                                    if !self.result_sent {
                                        self.result_sent = true;
                                        
                                        // Format result with input data if applicable
                                        let final_result = match result_type {
                                            "TextInput" => {
                                                if self.text_input.is_empty() {
                                                    "TextInput:".to_string()
                                                } else {
                                                    format!("TextInput:{}", self.text_input)
                                                }
                                            },
                                            "FileSelected" => {
                                                if self.file_input.is_empty() {
                                                    "FileSelected:".to_string()
                                                } else {
                                                    format!("FileSelected:{}", self.file_input)
                                                }
                                            },
                                            _ => result_type.to_string()
                                        };
                                        
                                        println!("[App] MessageBox result: {}", final_result);
                                        eprintln!("[STDERR DEBUG] About to send RESULT:{}", final_result);
                                        // Send result to stdout for API caller to read
                                        println!("RESULT:{}", final_result);
                                        std::io::stdout().flush().unwrap_or_default();
                                        eprintln!("[STDERR DEBUG] RESULT sent and flushed");
                                        
                                        // Add a small delay to ensure the result gets flushed before closing
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        
                                        // Exit the application after sending result
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    } else {
                                        println!("[DEBUG] Result already sent, skipping duplicate");
                                    }
                                }
                            });
                        });
                    });
                });
                
                // Add much larger final bottom padding to ensure proper window sizing
                // ui.add_space(25.0);

            // Editing and parsing area below the card
            if self.editor_mode {
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.heading("📇 e_window default editor");
                });

                if ui.button("🔍 Parse").clicked() {
                    self.parsed_data = parse_text(&self.input_text, self.decode_debug);
                }
                if ui.button("🚀 Run in new window").clicked() {
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

pub const DEFAULT_CARD_TEMPLATE: &str = r#"--title "Demo: e_window" --follow-hwnd {PARENT_HWND}
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
