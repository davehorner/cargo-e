//! position_grid.rs
// Provides a utility for drawing a 2x2 grid sized to the current font's uppercase letter dimensions in egui.

use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Stroke, TextStyle, Ui};

#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;

/// Struct representing a 2x2 position grid sized to the current font's uppercase letter.
pub struct PositionGrid {
    pub rect: Rect,
    pub char_size: egui::Vec2,
    pub grid_size: egui::Vec2,
    pub grid_dims: (usize, usize),
    pub color: Color32,
    pub corner_rounding: CornerRadius,
    pub stroke: Stroke,
    pub stroke_kind: egui::StrokeKind,
    pub fill: Option<Color32>,
    #[cfg(target_os = "windows")]
    pub host_hwnd: Option<u32>,
}

impl PositionGrid {
        pub fn get_dpi_for_window(hwnd: u32) -> u32 {
        #[cfg(target_os = "windows")]
        unsafe {
            // Try GetDpiForWindow (Windows 10+)
            #[allow(non_snake_case)]
            extern "system" {
                fn GetDpiForWindow(hwnd: HWND) -> u32;
            }
            let dpi = GetDpiForWindow(hwnd as HWND);
            if dpi == 0 {
                // Fallback: GetDeviceCaps
                use winapi::um::wingdi::{GetDeviceCaps, LOGPIXELSX};
                use winapi::um::winuser::GetDC;
                let hdc = GetDC(hwnd as HWND);
                let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
                dpi as u32
            } else {
                dpi
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            96
        }
    }

    /// Sends a mouse click to the center of the specified cell (cell_x, cell_y) using the provided host HWND and grid geometry.
    #[cfg(target_os = "windows")]
    pub fn send_mouse_click_to_cell(&self, cell_x: usize, cell_y: usize) -> bool {
        #[cfg(target_os = "windows")]
        {
            let cell_count_x = self.cell_count_x();
            let cell_count_y = self.cell_count_y();
            println!("[PositionGrid] cell_count_x: {}, cell_count_y: {}", cell_count_x, cell_count_y);
            println!("[PositionGrid] Requested cell_x: {}, cell_y: {}", cell_x, cell_y);
            if cell_x >= cell_count_x || cell_y >= cell_count_y {
                println!("[PositionGrid] Cell index out of bounds");
                return false;
            }
            let cell_w = self.rect.size().x as f64 / cell_count_x as f64;
            let cell_h = self.rect.size().y as f64 / cell_count_y as f64;
            println!("[PositionGrid] cell_w: {}, cell_h: {}", cell_w, cell_h);
            let center_x = self.rect.min.x as f64 + (cell_x as f64 + 0.5) * cell_w;
            let center_y = self.rect.min.y as f64 + (cell_y as f64 + 0.5) * cell_h;
            println!("[PositionGrid] rect.min.x: {}, rect.min.y: {}", self.rect.min.x, self.rect.min.y);
            println!("[PositionGrid] center_x: {}, center_y: {}", center_x, center_y);
            if let Some(hwnd) = self.get_grid_hwnd() {
                let dpi = Self::get_dpi_for_window(hwnd as u32);
                println!("[PositionGrid] DPI for window: {}", dpi);
                let scale = dpi as f64 / 96.0;
                println!("[PositionGrid] DPI scale: {}", scale);
                // Offset by host window position using WinAPI directly
                let (host_x, host_y) = unsafe {
                    use winapi::shared::windef::RECT;
                    use winapi::um::winuser::GetWindowRect;
                    if let Some(hwnd) = self.host_hwnd {
                        let mut rect: RECT = std::mem::zeroed();
                        if GetWindowRect(hwnd as HWND, &mut rect) != 0 {
                            println!("[PositionGrid] Host window rect: left={}, top={}, right={}, bottom={}", rect.left, rect.top, rect.right, rect.bottom);
                            (rect.left as f64, rect.top as f64)
                        } else {
                            println!("[PositionGrid] Failed to get host window rect");
                            (0.0, 0.0)
                        }
                    } else {
                        println!("[PositionGrid] host_hwnd is None");
                        (0.0, 0.0)
                    }
                };
                let x = host_x + center_x / scale;
                let y = host_y + center_y / scale;
                println!("[PositionGrid] Sending mouse to cell ({},{}): x={}, y={}, dpi_scale={}, host_offset=({}, {})", cell_x, cell_y, x, y, scale, host_x, host_y);
                // Use WinAPI SetCursorPos for mouse movement
                println!("[PositionGrid] Using SetCursorPos: x={}, y={}", x as i32, y as i32);
                unsafe {
                    use winapi::um::winuser::SetCursorPos;
                    SetCursorPos(x as i32, y as i32);
                }
                // If you want to simulate mouse click, you can use SendInput or mouse_event here
                // rdev simulate calls are commented out
                    // Print the current mouse pointer location using WinAPI
                    unsafe {
                        use winapi::shared::windef::POINT;
                        use winapi::um::winuser::GetCursorPos;
                        let mut pt: POINT = std::mem::zeroed();
                        if GetCursorPos(&mut pt) != 0 {
                            println!("[PositionGrid] Mouse pointer is now at: x={}, y={}", pt.x, pt.y);
                        } else {
                            println!("[PositionGrid] Failed to get mouse pointer position");
                        }
                    }
                return true;
            }
            false
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
    /// Returns the HWND associated with the grid (Windows only).
    #[cfg(target_os = "windows")]
    pub fn get_grid_hwnd(&self) -> Option<u32> {
        self.host_hwnd
    }
    /// Create and allocate a new PositionGrid at the current cursor position.
    pub fn allocate(ui: &mut Ui, color: Color32) -> (Self, Response) {
        let font_id = TextStyle::Heading.resolve(ui.style());
        let galley = ui.fonts(|fonts| fonts.layout_no_wrap("M".to_string(), font_id.clone(), Color32::WHITE));
        let char_size = galley.size();
        let grid_size = vec2(char_size.x * 2.0, char_size.y * 2.0);
        let (rect, response) = ui.allocate_exact_size(grid_size, egui::Sense::hover());
        (
            Self::new_with_rect(None,rect, char_size, grid_size, color),
            response,
        )
    }

    /// Create a new PositionGrid with all fields settable.
    pub fn new_with_rect(host_hwnd: Option<u32>, rect: Rect, char_size: egui::Vec2, grid_size: egui::Vec2, color: Color32) -> Self {
        let grid_dims = (
            (grid_size.x / char_size.x).floor() as usize,
            (grid_size.y / char_size.y).floor() as usize,
        );
        Self {
            rect,
            char_size,
            grid_size,
            grid_dims,
            color,
            corner_rounding: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, color),
            stroke_kind: egui::StrokeKind::Middle,
            fill: None,
            #[cfg(target_os = "windows")]
            host_hwnd,
        }
    }

    /// Create a PositionGrid from a TextStyle and requested grid size (cols, rows).
    pub fn from_text_style(host_hwnd: Option<u32>, ui: &Ui, style: egui::TextStyle, color: Color32, grid_dims: Option<(usize, usize)>) -> (Self, egui::Vec2) {
        let font_id = style.resolve(ui.style());
        let galley = ui.fonts(|fonts| fonts.layout_no_wrap("M".to_string(), font_id.clone(), Color32::WHITE));
        let char_size = galley.size();
        let available = ui.available_size();
        let label_height = 32.0;
        let available_for_grid_y = (available.y - label_height).max(char_size.y);
        let (cols, rows) = match grid_dims {
            Some((c, r)) => (c, r),
            None => {
                let c = (available.x / char_size.x).floor().max(1.0) as usize;
                let r = (available_for_grid_y / char_size.y).floor().max(1.0) as usize;
                (c, r)
            }
        };
        // Clamp to at least 1x1 grid
        let cols = cols.max(1);
        let rows = rows.max(1);
        let grid_size = egui::vec2(cols as f32 * char_size.x, rows as f32 * char_size.y);
        let rect = ui.available_rect_before_wrap();
        (
            Self {
                rect,
                char_size,
                grid_size,
                grid_dims: (cols, rows),
                color,
                corner_rounding: CornerRadius::ZERO,
                stroke: Stroke::new(1.0, color),
                stroke_kind: egui::StrokeKind::Middle,
                fill: None,
                #[cfg(target_os = "windows")]
                host_hwnd: host_hwnd.map(|hwnd| hwnd as u32),
            },
            char_size,
        )
    }


    /// Returns the total number of cells in the grid (columns * rows).
    pub fn cell_count(&self) -> usize {
        self.grid_dims.0 * self.grid_dims.1
    }
    
    /// Returns the number of cells in the X (columns) direction.
    pub fn cell_count_x(&self) -> usize {
        self.grid_dims.0
    }

    /// Returns the number of cells in the Y (rows) direction.
    pub fn cell_count_y(&self) -> usize {
        self.grid_dims.1
    }
}

impl Default for PositionGrid {
    fn default() -> Self {
        Self {
            rect: Rect::NOTHING,
            char_size: egui::Vec2::ZERO,
            grid_size: egui::Vec2::ZERO,
            grid_dims: (2, 2),
            color: Color32::LIGHT_GRAY,
            corner_rounding: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, Color32::LIGHT_GRAY),
            stroke_kind: egui::StrokeKind::Middle,
            fill: None,
            #[cfg(target_os = "windows")]
            host_hwnd: None,
        }
    }
}

impl PositionGrid {
    /// Draw the grid using the provided painter and current struct settings.
    pub fn draw(&self, ui: &Ui) {
        let painter = ui.painter_at(self.rect);
        let stroke_offset = self.stroke.width / 2.0;
        let rect = self.rect.shrink(stroke_offset);
        let top_left = rect.left_top();
        let bottom_right = rect.right_bottom();
        // Draw all internal vertical lines
        for col in 1..self.grid_dims.0 {
            let x = top_left.x + self.char_size.x * col as f32;
            painter.line_segment([
                pos2(x, top_left.y),
                pos2(x, bottom_right.y)
            ], (1.0, self.color));
        }
        // Draw all internal horizontal lines
        for row in 1..self.grid_dims.1 {
            let y = top_left.y + self.char_size.y * row as f32;
            painter.line_segment([
                pos2(top_left.x, y),
                pos2(bottom_right.x, y)
            ], (1.0, self.color));
        }
        // Explicitly draw border lines
        painter.line_segment([top_left, pos2(bottom_right.x, top_left.y)], (1.0, self.color)); // Top
        painter.line_segment([pos2(bottom_right.x, top_left.y), bottom_right], (1.0, self.color)); // Right
        painter.line_segment([bottom_right, pos2(top_left.x, bottom_right.y)], (1.0, self.color)); // Bottom
        painter.line_segment([pos2(top_left.x, bottom_right.y), top_left], (1.0, self.color)); // Left
        // Optionally fill background
        let fill = self.fill.unwrap_or(Color32::TRANSPARENT);
        painter.rect(self.rect, self.corner_rounding, fill, self.stroke, self.stroke_kind);
    }
 }
