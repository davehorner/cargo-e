//! position_grid_manager.rs
//! Provides reusable grid-to-screen mapping for e_window demos.

use crate::position_grid::PositionGrid;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::shared::windef::RECT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowRect;
use std::sync::Arc;

#[cfg(target_os = "windows")]
pub struct PositionGridManager {
    pub host_hwnd: Option<HWND>,
    pub grid: Option<*const PositionGrid>,
}

#[cfg(target_os = "windows")]
impl PositionGridManager {
  
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
                    host_hwnd,
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
    /// Sets the target window as topmost or not topmost
    pub fn set_topmost(&self, target_hwnd: HWND, topmost: bool) -> bool {
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
        Self { host_hwnd: None, grid: None }
    }

    /// Returns the host window's screen rect (x, y, w, h)
    pub fn get_host_screen_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let hwnd = self.host_hwnd?;
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        let got_rect = unsafe { GetWindowRect(hwnd, &mut rect) };
        if got_rect == 0 {
            return None;
        }
        let dpi = Self::get_dpi_for_window(hwnd);
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
        let got_client = unsafe { winapi::um::winuser::GetClientRect(hwnd, &mut client_rect) };
        if got_client == 0 {
            return None;
        }
        // Get client area's top-left in screen coordinates
        let mut pt = winapi::shared::windef::POINT { x: client_rect.left, y: client_rect.top };
        let got_screen = unsafe { winapi::um::winuser::ClientToScreen(hwnd, &mut pt) };
        if got_screen == 0 {
            return None;
        }
        let dpi = Self::get_dpi_for_window(hwnd);
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

    /// Move and resize a target window to the grid position relative to the host window
    pub fn move_and_resize(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW, SWP_NOACTIVATE};
        if let Some((x, y, w, h)) = self.get_grid_screen_rect() {
            let result = unsafe {
                SetWindowPos(
                    target_hwnd,
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
    
    /// Returns the HWND of the host window if available
    pub fn get_host_hwnd(&self) -> Option<HWND> {
        self.host_hwnd
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
