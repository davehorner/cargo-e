//! Example: position_grid_demo.rs
// Demonstrates usage of the PositionGrid utility in an egui app.
extern crate e_window;
use eframe::egui;
use e_window::position_grid::PositionGrid;

#[cfg(target_os = "windows")]
use winapi::um::winuser::GetForegroundWindow;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowThreadProcessId;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use sysinfo::{System, Process};

pub struct GridDemoApp {
    #[cfg(target_os = "windows")]
    pinned_hwnd: Option<HWND>,
    #[cfg(target_os = "windows")]
    original_pinned_rect: Option<(i32, i32, i32, i32)>,
    #[cfg(target_os = "windows")]
    original_pinned_z: Option<isize>,
    #[cfg(target_os = "windows")]
    last_pinned_rect: Option<(i32, i32, i32, i32)>,
    #[cfg(target_os = "windows")]
    eframe_hwnd: Option<HWND>,
}

impl Default for GridDemoApp {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            pinned_hwnd: None,
            #[cfg(target_os = "windows")]
            original_pinned_rect: None,
            #[cfg(target_os = "windows")]
            original_pinned_z: None,
            #[cfg(target_os = "windows")]
            last_pinned_rect: None,
            #[cfg(target_os = "windows")]
            eframe_hwnd: None,
        }
    }
}

impl eframe::App for GridDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PositionGrid 2x2 Demo");
            ui.label("A 2x2 grid sized to the current font's uppercase letter is shown below:");
            // Add font size adjustment controls above the 2x2 grid
            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    ctx.set_pixels_per_point((ctx.pixels_per_point() - 0.1).max(0.5));
                }
                ui.label("Font Size");
                if ui.button("+").clicked() {
                    ctx.set_pixels_per_point((ctx.pixels_per_point() + 0.1).min(4.0));
                }
            });
            // 2x2 grid sized to font
            let (grid, char_size) = PositionGrid::from_text_style(None,ui, egui::TextStyle::Heading, egui::Color32::LIGHT_BLUE, Some((2, 2)));
            // Add vertical space before grid to ensure visibility
            ui.add_space(8.0);
            let (rect, _response) = ui.allocate_exact_size(grid.grid_size, egui::Sense::hover());
            let grid = PositionGrid::new_with_rect(None,rect, char_size, grid.grid_size, egui::Color32::LIGHT_BLUE);
            grid.draw(ui);
            // Add vertical space after grid to prevent overlap
            ui.add_space(16.0);

            ui.separator();
            ui.label("A grid that fills the window with as many font-sized cells as possible:");
            let style = ctx.style();
            let font_id = style.text_styles.get(&egui::TextStyle::Heading).unwrap();
            let label_height = ui.fonts(|fonts| fonts.row_height(font_id));


            let available = ui.available_size();
            let available_for_grid = egui::vec2(available.x, available.y - label_height); // 1px less vertically
            // Pass None to auto-calculate grid size
            let (mut fill_grid, _) = PositionGrid::from_text_style(None,ui, egui::TextStyle::Heading, egui::Color32::LIGHT_GREEN, None);
            let fill_grid_size = egui::vec2(fill_grid.grid_size.x, fill_grid.grid_size.y - 1.0); // 1px less vertically
            let (fill_rect, _response) = ui.allocate_exact_size(fill_grid_size, egui::Sense::hover());
            fill_grid.rect = fill_rect;
            #[cfg(target_os = "windows")]
            {

                use winapi::um::winuser::IsWindow;
                ctx.request_repaint();  // done for quick exit when you close the pinned window
                if let Some(hwnd) = self.pinned_hwnd {
                    // If pinned_hwnd is not a valid window, exit the app
                    if unsafe { IsWindow(hwnd) } == 0 {
                        println!("Pinned HWND is no longer valid. Exiting app.");
                        std::process::exit(0);
                    }
                }

                if ui.button("Pin first Chrome HWND").clicked() {
                    use sysinfo::ProcessesToUpdate;

                    println!("Searching for Chrome process...");
                    let mut sys = System::new_all();
                    sys.refresh_processes(ProcessesToUpdate::All, true);
                    let mut found = false;
                    let procs = sys.processes();
                    println!("{}",format!("Total processes: {}", procs.len()));
                    let mut hwnd_found = None;
                    let mut pinned_rect = None;
                    for p in procs.values() {
                        let pname = p.name().to_string_lossy().to_lowercase();
                        println!("Process: {} (pid: {:?})", pname, p.pid());
                        if pname.contains("chrome") {
                            let pid = p.pid().as_u32();
                            #[repr(C)]
                            struct EnumWindowsState {
                                target_pid: u32,
                                hwnd_result: Option<HWND>,
                            }
                            unsafe {
                                use winapi::um::winuser::{EnumWindows, IsWindowVisible, GetWindowRect, GetWindowTextW, GetWindowTextLengthW, GetWindowLongW, GWL_STYLE};
                                use winapi::shared::windef::RECT;
                                extern "system" fn enum_windows_proc(hwnd: HWND, lparam: winapi::shared::minwindef::LPARAM) -> i32 {
                                    unsafe {
                                        let state = &mut *(lparam as *mut EnumWindowsState);
                                        let mut pid: u32 = 0;
                                        GetWindowThreadProcessId(hwnd, &mut pid);
                                        if pid == state.target_pid && IsWindowVisible(hwnd) != 0 {
                                            let len = GetWindowTextLengthW(hwnd);
                                            let mut title = String::new();
                                            if len > 0 {
                                                let mut buf = vec![0u16; (len + 1) as usize];
                                                let read = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
                                                if read > 0 {
                                                    title = String::from_utf16_lossy(&buf[..read as usize]);
                                                }
                                            }
                                            let style = GetWindowLongW(hwnd, GWL_STYLE);
                                            println!("  Candidate HWND: 0x{:X}, title: '{}', style: 0x{:X}", hwnd as usize, title, style);
                                            // Only select windows with a non-empty title (browser windows)
                                            if !title.is_empty() {
                                                state.hwnd_result = Some(hwnd);
                                                return 0; // Stop enumeration
                                            }
                                        }
                                    }
                                    1 // Continue enumeration
                                }
                                let mut state = EnumWindowsState { target_pid: pid, hwnd_result: None };
                                let lparam = &mut state as *mut _ as winapi::shared::minwindef::LPARAM;
                                EnumWindows(Some(enum_windows_proc), lparam);
                                if let Some(hwnd) = state.hwnd_result {
                                    println!("Pinned HWND: 0x{:X}", hwnd as usize);
                                    hwnd_found = Some(hwnd);
                                    let mut rect: RECT = std::mem::zeroed();
                                    if GetWindowRect(hwnd, &mut rect) != 0 {
                                        pinned_rect = Some((rect.left, rect.top, rect.right, rect.bottom));
                                    }
                                    break;
                                } else {
                                    println!("No visible browser window found for Chrome process pid {}!", pid);
                                }
                            }
                        }
                    }
                    self.pinned_hwnd = hwnd_found;
                    self.original_pinned_rect = pinned_rect;
                    self.original_pinned_z = None; // Could be set with GetWindowLongPtr if needed
                    if hwnd_found.is_none() {
                        ui.label("No window found for any Chrome process!");
                    }
                }
                if let Some(hwnd) = self.pinned_hwnd {
                    ui.label(format!("Pinned HWND: 0x{:X}", hwnd as usize));
                    // Move and resize pinned window to overlay the fill grid only when focused
                    unsafe {
                        use winapi::um::winuser::{SetWindowPos, GetWindowRect, SWP_SHOWWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, HWND_TOPMOST, HWND_TOP, HWND_NOTOPMOST, ShowWindow, SW_RESTORE};
                        use winapi::shared::windef::RECT;
                        // Cache eframe window HWND once
                        let eframe_hwnd = if let Some(hwnd) = self.eframe_hwnd {
                            hwnd
                        } else {
                            let hwnd = winapi::um::winuser::GetForegroundWindow();
                            self.eframe_hwnd = Some(hwnd);
                            hwnd
                        };
                        let mut rect: RECT = std::mem::zeroed();
                        if winapi::um::winuser::GetWindowRect(eframe_hwnd, &mut rect) != 0 {
                            let dpi = ctx.pixels_per_point();
                            let window_x = rect.left as f32;
                            let window_y = rect.top as f32;
                            let pos = fill_grid.rect.min;
                            let size = fill_grid.rect.size();
                            // Dynamically get the bottom of the label above the grid
                            let title_bar_height = label_height + 14.0; // 5px padding for better visibility
                            let border_width = 5.0 * dpi;
                            let mut x = (window_x + pos.x * dpi) as i32 + border_width as i32;
                            let mut y = (window_y + pos.y * dpi) as i32 + title_bar_height as i32;
                            let mut w = (size.x * dpi) as i32;
                            let mut h = (size.y * dpi) as i32;
                            if x < rect.left { x = rect.left; }
                            if y < rect.top { y = rect.top; }
                            if x + w > rect.right { w = rect.right - x; }
                            if y + h > rect.bottom { h = rect.bottom - y; }


                            // Compare process IDs instead of HWNDs for more reliable focus detection
                            let mut fg_pid: u32 = 0;
                            let fg_hwnd = winapi::um::winuser::GetForegroundWindow();
                            winapi::um::winuser::GetWindowThreadProcessId(fg_hwnd, &mut fg_pid);
                            let mut eframe_pid: u32 = 0;
                            winapi::um::winuser::GetWindowThreadProcessId(eframe_hwnd, &mut eframe_pid);
                            let is_focused = fg_pid == eframe_pid;
                            // Set eframe window to normal z-order before making Chrome topmost
                            SetWindowPos(
                                eframe_hwnd,
                                HWND_NOTOPMOST,
                                0, 0, 0, 0,
                                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                            );
                            // Restore Chrome if minimized
                            ShowWindow(hwnd, SW_RESTORE);

                            // If focused and window is not marked topmost, make it topmost
                            let style = winapi::um::winuser::GetWindowLongW(hwnd, winapi::um::winuser::GWL_EXSTYLE);
                            let is_topmost = (style & winapi::um::winuser::WS_EX_TOPMOST as i32) != 0;
                            if is_focused && !is_topmost {
                                let result = SetWindowPos(
                                    hwnd,
                                    HWND_TOPMOST,
                                    0, 0, 0, 0,
                                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                                );
                                if result == 0 {
                                    ui.label("Failed to set pinned window topmost!");
                                }
                            }

                            if is_focused {
                                // Only move/resize if eframe is focused AND window is strictly within eframe rect
                                if x < rect.left || y < rect.top || (x + w) > rect.right || (y + h) > rect.bottom {
                                    println!("Warning: Target window position/size is outside the eframe window bounds! Skipping move/resize.");
                                } else {
                                    let new_rect = (x, y, w, h);
                                    if self.last_pinned_rect != Some(new_rect) {
                                        println!(
                                            "Moving/Resizing pinned window to: x={}, y={}, w={}, h={} (eframe_hwnd=0x{:X}, pinned_hwnd=0x{:X}), eframe rect: left={}, top={}, right={}, bottom={}",
                                            x, y, w, h, eframe_hwnd as usize, hwnd as usize,
                                            rect.left, rect.top, rect.right, rect.bottom
                                        );
                                        let result = SetWindowPos(
                                            hwnd,
                                            HWND_TOPMOST,
                                            x,
                                            y,
                                            w,
                                            h,
                                            SWP_SHOWWINDOW | SWP_NOACTIVATE,
                                        );
                                        if result == 0 {
                                            ui.label("Failed to move/resize pinned window!");
                                        }
                                        self.last_pinned_rect = Some(new_rect);
                                    }
                                }
                            } else {
                                // On focus loss, set Chrome to not topmost (HWND_NOTOPMOST), do not move/resize
                                let result = SetWindowPos(
                                    hwnd,
                                    HWND_NOTOPMOST,
                                    0, 0, 0, 0,
                                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                                );
                                if result == 0 {
                                    ui.label("Failed to set pinned window z-order!");
                                }
                            }
                            // Only restore original position on app exit (not handled here)
                        } else {
                            ui.label("Failed to get eframe window position!");
                        }
                    }
                }
            }
            fill_grid.draw(ui);
            ui.label(format!("Grid: {} cols x {} rows (font size: {:.1}x{:.1})", fill_grid.grid_dims.0, fill_grid.grid_dims.1, char_size.x, char_size.y));
        });
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PositionGrid Demo",
        options,
        Box::new(|_cc| Ok::<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>(Box::new(GridDemoApp::default()))),
    );
}
