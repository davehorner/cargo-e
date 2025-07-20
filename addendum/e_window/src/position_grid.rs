//! position_grid.rs
// Provides a utility for drawing a 2x2 grid sized to the current font's uppercase letter dimensions in egui.

use egui::{pos2, vec2, Color32, CornerRadius, Rect, Response, Stroke, TextStyle, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    SingleClick,
    FollowSingle,
    ClickAndDrag,
    FollowClickAndDrag,
    ClickAndClick,
    FollowClickAndClick,
}

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
    // Removed: pub selected: Option<(usize, usize)>,
    pub selection_mode: SelectionMode,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub hover_cell: Option<(usize, usize)>,
    pub highlight_anchor: Option<(usize, usize)>,
    pub highlight_end: Option<(usize, usize)>,
    #[cfg(target_os = "windows")]
    pub host_hwnd: Option<u32>,
}

impl PositionGrid {
    /// Set the number of columns in the grid. Preserves selection if possible.
    pub fn set_cols(&mut self, cols: usize) {
        let cols = cols.max(1);
        self.grid_dims.0 = cols;
        self.grid_size.x = cols as f32 * self.char_size.x;
        // Clamp selection to new grid size
        if let Some((c, r)) = self.selection_anchor {
            if c >= cols {
                self.selection_anchor = Some((cols - 1, r));
            }
        }
        if let Some((c, r)) = self.selection_end {
            if c >= cols {
                self.selection_end = Some((cols - 1, r));
            }
        }
    }

    /// Set the number of rows in the grid. Preserves selection if possible.
    pub fn set_rows(&mut self, rows: usize) {
        let rows = rows.max(1);
        self.grid_dims.1 = rows;
        self.grid_size.y = rows as f32 * self.char_size.y;
        // Clamp selection to new grid size
        if let Some((c, r)) = self.selection_anchor {
            if r >= rows {
                self.selection_anchor = Some((c, rows - 1));
            }
        }
        if let Some((c, r)) = self.selection_end {
            if r >= rows {
                self.selection_end = Some((c, rows - 1));
            }
        }
    }

    /// Set both columns and rows at once. Preserves selection if possible.
    pub fn set_dims(&mut self, cols: usize, rows: usize) {
        self.set_cols(cols);
        self.set_rows(rows);
    }

    /// Set the selection anchor and end (single cell or rectangular selection).
    pub fn set_selection(&mut self, anchor: Option<(usize, usize)>, end: Option<(usize, usize)>) {
        self.selection_anchor = anchor;
        self.selection_end = end;
    }

    /// Set the selection to a single cell.
    pub fn set_selected_cell(&mut self, cell: Option<(usize, usize)>) {
        self.selection_anchor = cell;
        self.selection_end = cell;
    }

    /// Set the grid's pixel size and update rect accordingly.
    pub fn set_grid_size(&mut self, grid_size: egui::Vec2) {
        self.grid_size = grid_size;
        self.grid_dims.0 = (grid_size.x / self.char_size.x).floor().max(1.0) as usize;
        self.grid_dims.1 = (grid_size.y / self.char_size.y).floor().max(1.0) as usize;
    }

    /// Set the grid's bounding rect (absolute position in UI coordinates).
    pub fn set_rect(&mut self, rect: egui::Rect) {
        self.rect = rect;
    }

    /// Set the character size (cell size in pixels).
    pub fn set_char_size(&mut self, char_size: egui::Vec2) {
        self.char_size = char_size;
        self.grid_size.x = self.grid_dims.0 as f32 * char_size.x;
        self.grid_size.y = self.grid_dims.1 as f32 * char_size.y;
    }
    /// Returns the number of cells in the X direction.
    pub fn rows(&self) -> usize {
        self.grid_dims.0
    }

    /// Returns the number of cells in the Y direction.
    pub fn cols(&self) -> usize {
        self.grid_dims.1
    }

    /// Returns the rectangle for the cell at (col, row).
    pub fn cell_rect(&self, col: usize, row: usize) -> Option<Rect> {
        if col >= self.cell_count_x() || row >= self.cell_count_y() {
            return None;
        }
        let cell_w = self.char_size.x;
        let cell_h = self.char_size.y;
        let x = self.rect.min.x + col as f32 * cell_w;
        let y = self.rect.min.y + row as f32 * cell_h;
        Some(Rect::from_min_size(pos2(x, y), vec2(cell_w, cell_h)))
    }
    /// Returns the screen coordinates (monitor space) for the cell at (col, row).
    #[cfg(target_os = "windows")]
    pub fn cell_rect_screen(&self, col: usize, row: usize) -> Option<Rect> {
        use winapi::shared::windef::RECT;
        use winapi::um::winuser::GetWindowRect;
        let cell_rect = self.cell_rect(col, row)?;
        let hwnd = self.host_hwnd?;
        let dpi = Self::get_dpi_for_window(hwnd);
        let scale = dpi as f32 / 96.0;

        // Get host window position in screen coordinates
        let (host_x, host_y) = unsafe {
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd as HWND, &mut rect) != 0 {
                (rect.left as f32, rect.top as f32)
            } else {
                (0.0, 0.0)
            }
        };

        // Convert cell_rect to screen coordinates
        let min_screen = pos2(
            host_x + cell_rect.min.x / scale,
            host_y + cell_rect.min.y / scale,
        );
        let max_screen = pos2(
            host_x + cell_rect.max.x / scale,
            host_y + cell_rect.max.y / scale,
        );
        Some(Rect::from_min_max(min_screen, max_screen))
    }

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
            println!(
                "[PositionGrid] cell_count_x: {}, cell_count_y: {}",
                cell_count_x, cell_count_y
            );
            println!(
                "[PositionGrid] Requested cell_x: {}, cell_y: {}",
                cell_x, cell_y
            );
            if cell_x >= cell_count_x || cell_y >= cell_count_y {
                println!("[PositionGrid] Cell index out of bounds");
                return false;
            }
            let cell_w = self.rect.size().x as f64 / cell_count_x as f64;
            let cell_h = self.rect.size().y as f64 / cell_count_y as f64;
            println!("[PositionGrid] cell_w: {}, cell_h: {}", cell_w, cell_h);
            let center_x = self.rect.min.x as f64 + (cell_x as f64 + 0.5) * cell_w;
            let center_y = self.rect.min.y as f64 + (cell_y as f64 + 0.5) * cell_h;
            println!(
                "[PositionGrid] rect.min.x: {}, rect.min.y: {}",
                self.rect.min.x, self.rect.min.y
            );
            println!(
                "[PositionGrid] center_x: {}, center_y: {}",
                center_x, center_y
            );
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
                println!(
                    "[PositionGrid] Using SetCursorPos: x={}, y={}",
                    x as i32, y as i32
                );
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
                        println!(
                            "[PositionGrid] Mouse pointer is now at: x={}, y={}",
                            pt.x, pt.y
                        );
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
        let galley = ui
            .fonts(|fonts| fonts.layout_no_wrap("M".to_string(), font_id.clone(), Color32::WHITE));
        let char_size = galley.size();
        let grid_size = vec2(char_size.x * 2.0, char_size.y * 2.0);
        let (rect, response) = ui.allocate_exact_size(grid_size, egui::Sense::hover());
        (
            Self::new_with_rect(None, rect, char_size, grid_size, color),
            response,
        )
    }

    /// Create a new PositionGrid with all fields settable.
    pub fn new_with_rect(
        host_hwnd: Option<u32>,
        rect: Rect,
        char_size: egui::Vec2,
        grid_size: egui::Vec2,
        color: Color32,
    ) -> Self {
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
            selection_mode: SelectionMode::SingleClick,
            selection_anchor: None,
            selection_end: None,
            hover_cell: None,
            highlight_end: None,
            highlight_anchor: None,
        }
    }

    /// Create a PositionGrid from a TextStyle and requested grid size (cols, rows).
    pub fn from_text_style(
        host_hwnd: Option<u32>,
        ui: &Ui,
        style: egui::TextStyle,
        color: Color32,
        grid_dims: Option<(usize, usize)>,
    ) -> (Self, egui::Vec2) {
        let font_id = style.resolve(ui.style());
        let galley = ui
            .fonts(|fonts| fonts.layout_no_wrap("M".to_string(), font_id.clone(), Color32::WHITE));
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
                selection_mode: SelectionMode::SingleClick,
                selection_anchor: None,
                selection_end: None,
                hover_cell: None,
                highlight_end: None,
                highlight_anchor: None,
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
            selection_mode: SelectionMode::SingleClick,
            selection_anchor: None,
            selection_end: None,
            hover_cell: None,
            highlight_end: None,
            #[cfg(target_os = "windows")]
            host_hwnd: None,
            highlight_anchor: None,
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
            painter.line_segment(
                [pos2(x, top_left.y), pos2(x, bottom_right.y)],
                (1.0, self.color),
            );
        }
        // Draw all internal horizontal lines
        for row in 1..self.grid_dims.1 {
            let y = top_left.y + self.char_size.y * row as f32;
            painter.line_segment(
                [pos2(top_left.x, y), pos2(bottom_right.x, y)],
                (1.0, self.color),
            );
        }
        // Explicitly draw border lines
        painter.line_segment(
            [top_left, pos2(bottom_right.x, top_left.y)],
            (1.0, self.color),
        ); // Top
        painter.line_segment(
            [pos2(bottom_right.x, top_left.y), bottom_right],
            (1.0, self.color),
        ); // Right
        painter.line_segment(
            [bottom_right, pos2(top_left.x, bottom_right.y)],
            (1.0, self.color),
        ); // Bottom
        painter.line_segment(
            [pos2(top_left.x, bottom_right.y), top_left],
            (1.0, self.color),
        ); // Left
           // Optionally fill background
        let fill = self.fill.unwrap_or(Color32::TRANSPARENT);
        painter.rect(
            self.rect,
            self.corner_rounding,
            fill,
            self.stroke,
            self.stroke_kind,
        );
    }

    /// Draws the grid as an interactive widget. Returns Some((col, row)) if a cell was clicked.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<(usize, usize)> {
        let mut clicked_cell = None;
        // Always track hover_cell
        self.hover_cell = None;
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        for col in 0..self.grid_dims.0 {
            for row in 0..self.grid_dims.1 {
                if let Some(cell_rect) = self.cell_rect(col, row) {
                    if let Some(mouse_pos) = pointer_pos {
                        if cell_rect.contains(mouse_pos) {
                            self.hover_cell = Some((col, row));
                        }
                    }
                }
            }
        }
        // Show grid info label (fix rows/cols order and add debug info)
        let cols = self.grid_dims.0;
        let rows = self.grid_dims.1;
        let total = cols * rows;
        let anchor = self
            .selection_anchor
            .map(|(c, r)| format!("({}, {})", c, r))
            .unwrap_or("None".to_string());
        let end = self
            .selection_end
            .map(|(c, r)| format!("({}, {})", c, r))
            .unwrap_or("None".to_string());
        let hover = self
            .hover_cell
            .map(|(c, r)| format!("({}, {})", c, r))
            .unwrap_or("None".to_string());
        ui.label(format!(
            "Grid: cols = {cols}, rows = {rows}, total = {total} | anchor = {anchor} | end = {end} | pointer: {:?} | hover: {hover}",
            pointer_pos
        ));
        let painter = ui.painter_at(self.rect);
        let stroke_offset = self.stroke.width / 2.0;
        let rect = self.rect.shrink(stroke_offset);
        let top_left = rect.left_top();
        let bottom_right = rect.right_bottom();
        // Draw all internal vertical lines
        for col in 1..self.grid_dims.0 {
            let x = top_left.x + self.char_size.x * col as f32;
            painter.line_segment(
                [egui::pos2(x, top_left.y), egui::pos2(x, bottom_right.y)],
                (1.0, self.color),
            );
        }
        // Draw all internal horizontal lines
        for row in 1..self.grid_dims.1 {
            let y = top_left.y + self.char_size.y * row as f32;
            painter.line_segment(
                [egui::pos2(top_left.x, y), egui::pos2(bottom_right.x, y)],
                (1.0, self.color),
            );
        }
        // Draw border
        painter.line_segment(
            [top_left, egui::pos2(bottom_right.x, top_left.y)],
            (1.0, self.color),
        );
        painter.line_segment(
            [egui::pos2(bottom_right.x, top_left.y), bottom_right],
            (1.0, self.color),
        );
        painter.line_segment(
            [bottom_right, egui::pos2(top_left.x, bottom_right.y)],
            (1.0, self.color),
        );
        painter.line_segment(
            [egui::pos2(top_left.x, bottom_right.y), top_left],
            (1.0, self.color),
        );
        // Optionally fill background
        let fill = self.fill.unwrap_or(egui::Color32::TRANSPARENT);
        painter.rect(
            self.rect,
            self.corner_rounding,
            fill,
            self.stroke,
            self.stroke_kind,
        );

        // Handle selection logic
        let is_dragging = false;
        let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
        // match self.selection_mode {
        //     SelectionMode::FollowSingle | SelectionMode::FollowClickAndDrag | SelectionMode::FollowClickAndClick => {
        //         // Track hover cell
        //         let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        //         for col in 0..self.grid_dims.0 {
        //             for row in 0..self.grid_dims.1 {
        //                 if let Some(cell_rect) = self.cell_rect(col, row) {
        //                     if let Some(mouse_pos) = pointer_pos {
        //                         if cell_rect.contains(mouse_pos) {
        //                             self.hover_cell = Some((col, row));
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     _ => {}
        // }

        match self.selection_mode {
            SelectionMode::SingleClick => {
                // Only select on click, no preview, selection is persistent until new click or Escape
                if escape_pressed {
                    self.selection_anchor = None;
                    self.selection_end = None;
                } else {
                    for col in 0..self.grid_dims.0 {
                        for row in 0..self.grid_dims.1 {
                            if let Some(cell_rect) = self.cell_rect(col, row) {
                                let cell_response = ui.interact(
                                    cell_rect,
                                    ui.id().with((col, row, "rect")),
                                    egui::Sense::click(),
                                );
                                if cell_response.clicked() {
                                    self.selection_anchor = Some((col, row));
                                    self.selection_end = Some((col, row));
                                    clicked_cell = Some((col, row));
                                }
                            }
                        }
                    }
                }
            }
            SelectionMode::FollowSingle => {
                // Hover highlights, click selects, selection is persistent until new click or Escape
                if escape_pressed {
                    self.selection_anchor = None;
                    self.selection_end = None;
                } else {
                    for col in 0..self.grid_dims.0 {
                        for row in 0..self.grid_dims.1 {
                            if let Some(cell_rect) = self.cell_rect(col, row) {
                                let cell_response = ui.interact(
                                    cell_rect,
                                    ui.id().with((col, row, "rect")),
                                    egui::Sense::click(),
                                );
                                if let Some((hcol, hrow)) = self.hover_cell {
                                    if hcol == col && hrow == row {
                                        // hover highlight
                                        ui.painter().rect_filled(
                                            cell_rect,
                                            2.0,
                                            egui::Color32::from_rgba_unmultiplied(
                                                200, 200, 100, 80,
                                            ),
                                        );
                                    }
                                }
                                if cell_response.clicked() {
                                    self.selection_anchor = Some((col, row));
                                    self.selection_end = Some((col, row));
                                    clicked_cell = Some((col, row));
                                }
                            }
                        }
                    }
                }
            }
            SelectionMode::ClickAndDrag => {
                // Drag to preview, release to select
                let mut dragging = false;
                for col in 0..self.grid_dims.0 {
                    for row in 0..self.grid_dims.1 {
                        if let Some(cell_rect) = self.cell_rect(col, row) {
                            let cell_response = ui.interact(
                                cell_rect,
                                ui.id().with((col, row, "rect")),
                                egui::Sense::click_and_drag(),
                            );
                            if cell_response.drag_started() {
                                self.highlight_anchor = Some((col, row));
                                self.highlight_end = Some((col, row));
                                dragging = true;
                            }
                            if cell_response.dragged() {
                                if self.highlight_anchor.is_some() {
                                    self.highlight_end = Some((col, row));
                                    dragging = true;
                                }
                            }
                            if cell_response.drag_stopped() {
                                if let (Some(anchor), Some(end)) =
                                    (self.highlight_anchor, self.highlight_end)
                                {
                                    self.selection_anchor = Some(anchor);
                                    self.selection_end = Some(end);
                                }
                                self.highlight_anchor = None;
                                self.highlight_end = None;
                                dragging = false;
                            }
                        }
                    }
                }
            }
            SelectionMode::FollowClickAndDrag => {
                // Drag to preview, hover highlights, release to select
                let mut dragging = false;
                for col in 0..self.grid_dims.0 {
                    for row in 0..self.grid_dims.1 {
                        if let Some(cell_rect) = self.cell_rect(col, row) {
                            let cell_response = ui.interact(
                                cell_rect,
                                ui.id().with((col, row, "rect")),
                                egui::Sense::click_and_drag(),
                            );
                            if let Some((hcol, hrow)) = self.hover_cell {
                                if hcol == col && hrow == row {
                                    ui.painter().rect_filled(
                                        cell_rect,
                                        2.0,
                                        egui::Color32::from_rgba_unmultiplied(200, 200, 100, 80),
                                    );
                                }
                            }
                            if cell_response.drag_started() {
                                self.highlight_anchor = Some((col, row));
                                self.highlight_end = Some((col, row));
                                dragging = true;
                            }
                            if cell_response.dragged() {
                                if self.highlight_anchor.is_some() {
                                    self.highlight_end = Some((col, row));
                                    dragging = true;
                                }
                            }
                            if cell_response.drag_stopped() {
                                if let (Some(anchor), Some(end)) =
                                    (self.highlight_anchor, self.highlight_end)
                                {
                                    self.selection_anchor = Some(anchor);
                                    self.selection_end = Some(end);
                                }
                                self.highlight_anchor = None;
                                self.highlight_end = None;
                                dragging = false;
                            }
                        }
                    }
                }
            }
            SelectionMode::ClickAndClick => {
                // First click sets anchor, second click sets end and finalizes selection
                for col in 0..self.grid_dims.0 {
                    for row in 0..self.grid_dims.1 {
                        if let Some(cell_rect) = self.cell_rect(col, row) {
                            let cell_response = ui.interact(
                                cell_rect,
                                ui.id().with((col, row, "rect")),
                                egui::Sense::click(),
                            );
                            if cell_response.clicked() {
                                if self.highlight_anchor.is_none() {
                                    self.highlight_anchor = Some((col, row));
                                    self.highlight_end = None;
                                } else if self.highlight_anchor.is_some()
                                    && self.highlight_end.is_none()
                                {
                                    self.highlight_end = Some((col, row));
                                    // Finalize selection
                                    self.selection_anchor = self.highlight_anchor;
                                    self.selection_end = self.highlight_end;
                                    self.highlight_anchor = None;
                                    self.highlight_end = None;
                                }
                            }
                        }
                    }
                }
            }
            SelectionMode::FollowClickAndClick => {
                // First click sets anchor, hover previews, second click sets end and finalizes selection
                for col in 0..self.grid_dims.0 {
                    for row in 0..self.grid_dims.1 {
                        if let Some(cell_rect) = self.cell_rect(col, row) {
                            let cell_response = ui.interact(
                                cell_rect,
                                ui.id().with((col, row, "rect")),
                                egui::Sense::click(),
                            );
                            if let Some((hcol, hrow)) = self.hover_cell {
                                if self.highlight_anchor.is_some()
                                    && self.highlight_end.is_none()
                                    && hcol == col
                                    && hrow == row
                                {
                                    ui.painter().rect_filled(
                                        cell_rect,
                                        2.0,
                                        egui::Color32::from_rgba_unmultiplied(200, 200, 100, 80),
                                    );
                                }
                            }
                            if cell_response.clicked() {
                                if self.highlight_anchor.is_none() {
                                    self.highlight_anchor = Some((col, row));
                                    self.highlight_end = None;
                                } else if self.highlight_anchor.is_some()
                                    && self.highlight_end.is_none()
                                {
                                    self.highlight_end = Some((col, row));
                                    // Finalize selection
                                    self.selection_anchor = self.highlight_anchor;
                                    self.selection_end = self.highlight_end;
                                    self.highlight_anchor = None;
                                    self.highlight_end = None;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Draw and handle each cell
        for col in 0..self.grid_dims.0 {
            // Draw highlights for selection and preview
            for col in 0..self.grid_dims.0 {
                for row in 0..self.grid_dims.1 {
                    if let Some(cell_rect) = self.cell_rect(col, row) {
                        // Draw preview highlight (yellow) for highlight_anchor/highlight_end
                        if let (Some((ax, ay)), Some((ex, ey))) =
                            (self.highlight_anchor, self.highlight_end)
                        {
                            let min_col = ax.min(ex);
                            let max_col = ax.max(ex);
                            let min_row = ay.min(ey);
                            let max_row = ay.max(ey);
                            if col >= min_col && col <= max_col && row >= min_row && row <= max_row
                            {
                                painter.rect_filled(
                                    cell_rect,
                                    2.0,
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 100, 80),
                                );
                            }
                        }
                        // Always draw selection highlight for selected cell in SingleClick/FollowSingle
                        match self.selection_mode {
                            SelectionMode::SingleClick | SelectionMode::FollowSingle => {
                                if let (Some((ax, ay)), Some((ex, ey))) =
                                    (self.selection_anchor, self.selection_end)
                                {
                                    if ax == ex && ay == ey && col == ax && row == ay {
                                        painter.rect_filled(
                                            cell_rect,
                                            2.0,
                                            egui::Color32::from_rgba_unmultiplied(
                                                100, 200, 255, 80,
                                            ),
                                        );
                                    }
                                }
                            }
                            _ => {
                                // Draw selection highlight (blue) for selection_anchor/selection_end (rectangular)
                                if let (Some((ax, ay)), Some((ex, ey))) =
                                    (self.selection_anchor, self.selection_end)
                                {
                                    let min_col = ax.min(ex);
                                    let max_col = ax.max(ex);
                                    let min_row = ay.min(ey);
                                    let max_row = ay.max(ey);
                                    if col >= min_col
                                        && col <= max_col
                                        && row >= min_row
                                        && row <= max_row
                                    {
                                        painter.rect_filled(
                                            cell_rect,
                                            2.0,
                                            egui::Color32::from_rgba_unmultiplied(
                                                100, 200, 255, 80,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        match self.selection_mode {
            SelectionMode::SingleClick | SelectionMode::FollowSingle => {
                if let (Some(anchor), Some(end)) = (self.selection_anchor, self.selection_end) {
                    if anchor == end {
                        Some(anchor)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            SelectionMode::ClickAndDrag
            | SelectionMode::FollowClickAndDrag
            | SelectionMode::ClickAndClick
            | SelectionMode::FollowClickAndClick => {
                if self.selection_anchor.is_some() && self.selection_end.is_some() {
                    self.selection_anchor
                } else {
                    None
                }
            }
        }
    }
}
