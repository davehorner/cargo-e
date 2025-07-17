
//! PositionGridManager handles the positioning and alignment of a grid overlay with a host window.


 

use crate::position_grid::PositionGrid;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::shared::windef::RECT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowRect;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[cfg(target_os = "windows")]
pub struct PositionGridManager {
    pub host_hwnd: Option<u32>,
    pub grid: Option<*const PositionGrid>,
    /// Suppresses feedback loop when moving windows programmatically
    pub suppress_move_event: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
impl PositionGridManager {
    /// Returns true if the window is topmost (WS_EX_TOPMOST set).
    #[cfg(target_os = "windows")]
    pub fn is_topmost(hwnd: HWND) -> bool {
        use winapi::um::winuser::GetWindowLongW;
        use winapi::um::winuser::GWL_EXSTYLE;
        use winapi::um::winuser::WS_EX_TOPMOST;
        unsafe {
            let exstyle = GetWindowLongW(hwnd, GWL_EXSTYLE);
            (exstyle & WS_EX_TOPMOST as i32) != 0
        }
    }

       /// Get the window rect (left, top, right, bottom) for a HWND
    #[cfg(target_os = "windows")]
    pub fn get_window_rect(hwnd: winapi::shared::windef::HWND) -> (i32, i32, i32, i32) {
        unsafe {
            use winapi::um::winuser::GetWindowRect;
            let mut rect = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) != 0 {
                (rect.left, rect.top, rect.right, rect.bottom)
            } else {
                (0, 0, 0, 0)
            }
        }
    }

    /// Move and resize a HWND to (x, y, w, h)
    #[cfg(target_os = "windows")]
    pub fn move_window(hwnd: winapi::shared::windef::HWND, x: i32, y: i32, w: i32, h: i32) -> bool {
        unsafe {
            use winapi::um::winuser::MoveWindow;
            MoveWindow(hwnd, x, y, w, h, 1) != 0
        }
    }

    /// Move the host window so that the grid remains aligned with the target window's rect
    /// This keeps the grid in the same position relative to the target after the target moves
    pub fn move_host_to_maintain_grid_alignment(&self, target_rect: (i32, i32, i32, i32)) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        if let Some(host_hwnd) = self.host_hwnd {
            if let Some((grid_x, grid_y, grid_w, grid_h)) = self.get_grid_screen_rect() {
                let (target_x, target_y, target_w, target_h) = target_rect;
                // Calculate offset between target and grid
                let offset_x = target_x - grid_x;
                let offset_y = target_y - grid_y;
                // Move host window by the offset
                let new_host_x = (self.get_host_screen_rect().map(|(x, _, _, _)| x).unwrap_or(0)) + offset_x;
                let new_host_y = (self.get_host_screen_rect().map(|(_, y, _, _)| y).unwrap_or(0)) + offset_y;
                let new_host_w = (self.get_host_screen_rect().map(|(_, _, w, _)| w).unwrap_or(target_w));
                let new_host_h = (self.get_host_screen_rect().map(|(_, _, _, h)| h).unwrap_or(target_h));
                let result = unsafe {
                    SetWindowPos(
                        host_hwnd as HWND,
                        HWND_TOPMOST,
                        new_host_x,
                        new_host_y,
                        new_host_w,
                        new_host_h,
                        SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    )
                };
                return result != 0;
            }
        }
        false
    }
    /// Calculates the grid's screen rect that would align with the given host rect
    /// Use this in the UI to position the grid overlay correctly when the host moves
    pub fn calculate_grid_rect_for_host(&self, host_rect: (i32, i32, i32, i32)) -> Option<(i32, i32, i32, i32)> {
        if let Some(grid_ptr) = self.grid {
            let grid = unsafe { &*grid_ptr };
            // Use host_rect and grid layout to compute where the grid should be
            // For now, assume grid stays at the top-left of host window
            let (host_x, host_y, _host_w, _host_h) = host_rect;
            let dpi = Self::get_dpi_for_window(self.host_hwnd.unwrap_or(0) as HWND);
            let scale = dpi as f32 / 96.0;
            let grid_x = (grid.rect.min.x * scale).round() as i32 + host_x;
            let grid_y = (grid.rect.min.y * scale).round() as i32 + host_y;
            let grid_w = (grid.rect.size().x * scale).round() as i32;
            let grid_h = (grid.rect.size().y * scale).round() as i32;
            Some((grid_x, grid_y, grid_w, grid_h))
        } else {
            None
        }
    }
    /// Move and resize the host window to match the given target rect (x, y, w, h)
    pub fn move_host_to_target_rect(&self, target_rect: (i32, i32, i32, i32)) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        if let Some(host_hwnd) = self.host_hwnd {
            let (x, y, w, h) = target_rect;
            let result = unsafe {
                SetWindowPos(
                    host_hwnd as HWND,
                    HWND_TOPMOST,
                    x,
                    y,
                    w,
                    h,
                    SWP_SHOWWINDOW | SWP_NOACTIVATE,
                )
            };
            return result != 0;
        }
        false
    }
  
    /// Places the target window just above the host window in Z order and removes topmost
    pub fn place_above_host(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        if let Some(host_hwnd) = self.host_hwnd {
            println!("Placing target window above host window: {:?}", host_hwnd);
            unsafe {
                let _ = SetWindowPos(
                    target_hwnd,
                    HWND_NOTOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                );
                let result = SetWindowPos(
                    target_hwnd,
                    host_hwnd as HWND,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                );
                let _ = SetWindowPos(
                    target_hwnd,
                    HWND_NOTOPMOST,
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                );
                result != 0
            }
        } else {
            println!("No host window set, cannot place target window above.");
            false
        }
    }
    /// Places the target window just below the foreground window in Z order and removes topmost
    pub fn place_below_foreground(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, GetForegroundWindow, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        unsafe {
            let fg_hwnd = GetForegroundWindow();
            let result = SetWindowPos(
                target_hwnd,
                fg_hwnd,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
            let notopmost = SetWindowPos(
                target_hwnd,
                HWND_NOTOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
            result != 0 && notopmost != 0
        }
    }

    /// Sets the target window's Z order directly above the specified HWND
    pub fn set_zorder_above(target_hwnd: HWND, above_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SWP_NOACTIVATE};

        let result = unsafe {
            SetWindowPos(
                target_hwnd,
                above_hwnd,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        result != 0
    }



    /// Sets the target window as topmost or not topmost
    pub fn set_topmost(target_hwnd: HWND, topmost: bool) -> bool {
        println!("Setting target window topmost: {}", topmost);
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        let flag_hwnd = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
        let result = unsafe {
            SetWindowPos(
                target_hwnd,
                flag_hwnd,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        result != 0
    }
    /// Returns true if the HWND is a valid window
    pub fn is_window(hwnd: HWND) -> bool {
        use winapi::um::winuser::IsWindow;
        unsafe { IsWindow(hwnd) != 0 }
    }

    pub fn get_dpi_for_window(hwnd: HWND) -> u32 {
        #[cfg(target_os = "windows")]
        unsafe {
            // Try GetDpiForWindow (Windows 10+)
            #[allow(non_snake_case)]
            extern "system" {
                fn GetDpiForWindow(hwnd: HWND) -> u32;
            }
            let dpi = GetDpiForWindow(hwnd);
            if dpi == 0 {
                // Fallback: GetDeviceCaps
                use winapi::um::wingdi::{GetDeviceCaps, LOGPIXELSX};
                use winapi::um::winuser::GetDC;
                let hdc = GetDC(hwnd);
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

    pub fn new() -> Self {
        Self {
            host_hwnd: None,
            grid: None,
            suppress_move_event: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Call before a programmatic move to suppress event feedback
    pub fn begin_programmatic_move(&self) {
        self.suppress_move_event.store(true, Ordering::SeqCst);
    }

    /// Call in your event handler to check if move was programmatic
    pub fn is_programmatic_move(&self) -> bool {
        self.suppress_move_event.load(Ordering::SeqCst)
    }

    /// Call after handling a move event to clear suppression
    pub fn end_programmatic_move(&self) {
        self.suppress_move_event.store(false, Ordering::SeqCst);
    }

    /// Returns the host window's screen rect (x, y, w, h)
    pub fn get_host_screen_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let hwnd = self.host_hwnd?;
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        let got_rect = unsafe { GetWindowRect(hwnd as HWND, &mut rect) };
        if got_rect == 0 {
            return None;
        }
        let dpi = Self::get_dpi_for_window(hwnd as HWND);
        let scale = dpi as f32 / 96.0;
        let x = (rect.left as f32 * scale).round() as i32;
        let y = (rect.top as f32 * scale).round() as i32;
        let w = ((rect.right - rect.left) as f32 * scale).round() as i32;
        let h = ((rect.bottom - rect.top) as f32 * scale).round() as i32;
        Some((x, y, w, h))
    }

    /// Returns the grid's screen rect (x, y, w, h) relative to the host window
    pub fn get_grid_screen_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let hwnd = self.host_hwnd?;
        let grid_ptr = self.grid?;
        let grid = unsafe { &*grid_ptr };
        let mut client_rect: RECT = unsafe { std::mem::zeroed() };
        let got_client = unsafe { winapi::um::winuser::GetClientRect(hwnd as HWND, &mut client_rect) };
        if got_client == 0 {
            return None;
        }
        // Get client area's top-left in screen coordinates
        let mut pt = winapi::shared::windef::POINT { x: client_rect.left, y: client_rect.top };
        let got_screen = unsafe { winapi::um::winuser::ClientToScreen(hwnd as HWND, &mut pt) };
        if got_screen == 0 {
            return None;
        }
        let dpi = Self::get_dpi_for_window(hwnd as HWND);
        let scale = dpi as f32 / 96.0;
        let client_left = pt.x;
        let client_top = pt.y;
        let grid_x = (grid.rect.min.x * scale).round() as i32;
        let grid_y = (grid.rect.min.y * scale).round() as i32;
        let grid_w = (grid.rect.size().x * scale).round() as i32;
        let grid_h = (grid.rect.size().y * scale).round() as i32;
        let x = (client_left as f32 * scale).round() as i32 + grid_x;
        let y = (client_top as f32 * scale).round() as i32 + grid_y;
        Some((x, y, grid_w, grid_h))
    }

    /// Move pinned HWND to grid overlay, using host client area top-left for correct placement
    pub fn move_pinned_hwnd_to_grid(&self, pinned_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOACTIVATE, GetClientRect, ClientToScreen, GetWindowRect};
        use winapi::um::winuser::{MonitorFromWindow, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO};
        use winapi::shared::windef::RECT as WinRect;
        if let Some(grid_ptr) = self.grid {
            let grid = unsafe { &*grid_ptr };
            if let Some(host_hwnd) = self.host_hwnd {
                // Get host client area top-left in screen coordinates
                let mut client_rect: WinRect = unsafe { std::mem::zeroed() };
                let got_client = unsafe { GetClientRect(host_hwnd as HWND, &mut client_rect) };
                if got_client == 0 { return false; }
                let mut pt = winapi::shared::windef::POINT { x: client_rect.left, y: client_rect.top };
                let got_screen = unsafe { ClientToScreen(host_hwnd as HWND, &mut pt) };
                if got_screen == 0 { return false; }
                let dpi = Self::get_dpi_for_window(host_hwnd as HWND);
                let scale = dpi as f32 / 96.0;
                // grid.rect.min is relative to UI, so add to client area top-left
                let mut grid_x = pt.x + (grid.rect.min.x * scale).round() as i32;
                let mut grid_y = pt.y + (grid.rect.min.y * scale).round() as i32;
                let mut grid_w = (grid.rect.size().x * scale).round() as i32;
                let mut grid_h = (grid.rect.size().y * scale).round() as i32;

                // Clamp grid rect to host client area
                let host_left = pt.x;
                let host_top = pt.y;
                let host_right = pt.x + ((client_rect.right - client_rect.left) as f32 * scale).round() as i32;
                let host_bottom = pt.y + ((client_rect.bottom - client_rect.top) as f32 * scale).round() as i32;
                // Clamp left/top
                if grid_x < host_left { grid_x = host_left; }
                if grid_y < host_top { grid_y = host_top; }
                // Clamp right/bottom
                if grid_x + grid_w > host_right { grid_w = host_right - grid_x; }
                if grid_y + grid_h > host_bottom { grid_h = host_bottom - grid_y; }

                // Clamp bottom to monitor bottom
                let monitor = unsafe { MonitorFromWindow(host_hwnd as HWND, MONITOR_DEFAULTTONEAREST) };
                let mut monitor_info: MONITORINFO = unsafe { std::mem::zeroed() };
                monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                let got_monitor = unsafe { GetMonitorInfoW(monitor, &mut monitor_info) };
                if got_monitor != 0 {
                    let mon_bottom = monitor_info.rcMonitor.bottom;
                    if grid_y + grid_h > mon_bottom {
                        grid_h = mon_bottom - grid_y;
                        if grid_h < 1 { grid_h = 1; }
                    }
                }

                // Get current window rect
                let mut win_rect: WinRect = unsafe { std::mem::zeroed() };
                let got_win_rect = unsafe { GetWindowRect(pinned_hwnd, &mut win_rect) };
                if got_win_rect != 0 {
                    let cur_x = win_rect.left;
                    let cur_y = win_rect.top;
                    let cur_w = win_rect.right - win_rect.left;
                    let cur_h = win_rect.bottom - win_rect.top;
                    // Only move if position or size changed
                    if cur_x == grid_x && cur_y == grid_y && cur_w == grid_w && cur_h == grid_h {
                        println!("[move_pinned_hwnd_to_grid] Skipping move: already at target position/size (x={}, y={}, w={}, h={})", cur_x, cur_y, cur_w, cur_h);
                        return true;
                    }
                }

                let result = unsafe {
                    SetWindowPos(
                        pinned_hwnd,
                        HWND_TOPMOST,
                        grid_x,
                        grid_y,
                        grid_w,
                        grid_h,
                        SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    )
                };
                return result != 0;
            }
        }
        false
    }

    /// Move and resize a target window to the grid position relative to the host window
    pub fn move_and_resize(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOACTIVATE};

        // Use grid rect relative to screen, not host client area
        if let Some(grid_ptr) = self.grid {
            let grid = unsafe { &*grid_ptr };
            // Get host window's screen rect
            if let Some((host_x, host_y, _, _)) = self.get_host_screen_rect() {
                // Grid rect is relative to host client area, so add host_x/host_y offset
                let dpi = Self::get_dpi_for_window(self.host_hwnd.unwrap_or(0) as HWND);
                let scale = dpi as f32 / 96.0;
                let grid_x = (grid.rect.min.x * scale).round() as i32 + host_x;
                let grid_y = (grid.rect.min.y * scale).round() as i32 + host_y;
                let grid_w = (grid.rect.size().x * scale).round() as i32;
                let grid_h = (grid.rect.size().y * scale).round() as i32;
                let result = unsafe {
                    SetWindowPos(
                        target_hwnd,
                        HWND_TOPMOST,
                        grid_x,
                        grid_y,
                        grid_w,
                        grid_h,
                        SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    )
                };
                return result != 0;
            }
        }
        false
    }
    /// Returns true if the host window is currently focused (has foreground)
    /// Returns true if the given HWND is currently focused (has foreground)
    pub fn is_window_focused(hwnd: HWND) -> bool {
        use winapi::um::winuser::GetForegroundWindow;

        let fg_hwnd = unsafe { GetForegroundWindow() };
        fg_hwnd == hwnd
    }
    /// Returns the HWND of the host window if available
    pub fn get_host_hwnd(&self) -> Option<u32> {
        self.host_hwnd
    }

    /// Sets the pinned HWND Z order above or below the host HWND
    /// If above is true, pinned HWND is placed directly above host HWND.
    /// If above is false, pinned HWND is placed directly below host HWND.
    pub fn set_pinned_zorder(&self, pinned_hwnd: HWND, above: bool) -> bool {
        if let Some(host_hwnd) = self.host_hwnd {
            if above {
                // Place pinned HWND directly above host HWND
                return Self::set_zorder_above(pinned_hwnd, host_hwnd as HWND);
            } else {
                // Place pinned HWND directly below host HWND
                use winapi::um::winuser::{SetWindowPos, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SWP_NOACTIVATE};
                let result = unsafe {
                    SetWindowPos(
                        pinned_hwnd,
                        host_hwnd as HWND,
                        0, 0, 0, 0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    )
                };
                return result != 0;
            }
        }
        false
    }
}

// For non-windows, stub
#[cfg(not(target_os = "windows"))]
pub struct PositionGridManager;
#[cfg(not(target_os = "windows"))]
impl PositionGridManager {
    pub fn new(_hwnd: ()) -> Self { Self }
    pub fn get_screen_rect(&self, _grid: &PositionGrid) -> Option<(i32, i32, i32, i32)> { None }
}
