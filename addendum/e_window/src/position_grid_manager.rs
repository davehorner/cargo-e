//! PositionGridManager handles the positioning and alignment of a grid overlay with a host window.

/// Alignment modes for moving a window relative to grid or host
#[derive(Debug, Clone, Copy)]
pub enum AlignmentMode {
    /// Align the client area to the grid rect (default for grid pinning)
    Grid,
    /// Align the outer window rect to the host window's outer rect
    HostExact,
    /// Align to an offset relative to the grid or host
    Offset {
        base: AlignmentBase,
        dx: i32,
        dy: i32,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum AlignmentBase {
    Grid,
    Host,
}

use crate::position_grid::PositionGrid;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::shared::windef::RECT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowRect;

#[cfg(target_os = "windows")]
pub struct PositionGridManager {
    pub host_hwnd: Option<u32>,
    pub grid: Option<*const PositionGrid>,
    /// Suppresses feedback loop when moving windows programmatically
    pub suppress_move_event: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
impl PositionGridManager {
    /// Computes the target window's rect (x, y, w, h) for the given alignment mode.
    pub unsafe fn get_target_rect_for_alignment(
        &self,
        hwnd_win: HWND,
        mode: AlignmentMode,
    ) -> Option<(i32, i32, i32, i32)> {
        use winapi::shared::windef::RECT as WinRect;
        use winapi::um::winuser::{ClientToScreen, GetClientRect, GetWindowRect};

        match mode {
            AlignmentMode::Grid => {
                if let Some(grid_ptr) = self.grid {
                    let grid = unsafe { &*grid_ptr };
                    if let Some(host_hwnd) = self.host_hwnd {
                        let mut client_rect: WinRect = unsafe { std::mem::zeroed() };
                        let got_client =
                            unsafe { GetClientRect(host_hwnd as HWND, &mut client_rect) };
                        if got_client == 0 {
                            return None;
                        }
                        let mut pt = winapi::shared::windef::POINT {
                            x: client_rect.left,
                            y: client_rect.top,
                        };
                        let got_screen = unsafe { ClientToScreen(host_hwnd as HWND, &mut pt) };
                        if got_screen == 0 {
                            return None;
                        }
                        let dpi = Self::get_dpi_for_window(host_hwnd as HWND);
                        let scale = dpi as f32 / 96.0;
                        let grid_x = pt.x + (grid.rect.min.x * scale).round() as i32;
                        let grid_y = pt.y + (grid.rect.min.y * scale).round() as i32;
                        let grid_w = (grid.rect.size().x * scale).round() as i32;
                        let grid_h = (grid.rect.size().y * scale).round() as i32;
                        // Adjust for non-client area of target window
                        let mut win_rect: WinRect = unsafe { std::mem::zeroed() };
                        let mut client_rect2: WinRect = unsafe { std::mem::zeroed() };
                        let got_win = unsafe { GetWindowRect(hwnd_win, &mut win_rect) };
                        let got_client2 = unsafe { GetClientRect(hwnd_win, &mut client_rect2) };
                        let mut client_pt = winapi::shared::windef::POINT {
                            x: client_rect2.left,
                            y: client_rect2.top,
                        };
                        let got_client_screen = unsafe { ClientToScreen(hwnd_win, &mut client_pt) };
                        if got_win != 0 && got_client2 != 0 && got_client_screen != 0 {
                            let offset_x = client_pt.x - win_rect.left;
                            let offset_y = client_pt.y - win_rect.top;
                            let outer_x = grid_x - offset_x;
                            let outer_y = grid_y - offset_y;
                            let outer_w = grid_w
                                + (win_rect.right
                                    - win_rect.left
                                    - (client_rect2.right - client_rect2.left));
                            let outer_h = grid_h
                                + (win_rect.bottom
                                    - win_rect.top
                                    - (client_rect2.bottom - client_rect2.top));
                            return Some((outer_x, outer_y, outer_w, outer_h));
                        }
                    }
                }
                None
            }
            AlignmentMode::HostExact => {
                if let Some(host_hwnd) = self.host_hwnd {
                    let mut host_rect: WinRect = unsafe { std::mem::zeroed() };
                    let got_host = unsafe { GetWindowRect(host_hwnd as HWND, &mut host_rect) };
                    if got_host == 0 {
                        return None;
                    }
                    let width = host_rect.right - host_rect.left;
                    let height = host_rect.bottom - host_rect.top;
                    return Some((host_rect.left, host_rect.top, width, height));
                }
                None
            }
            AlignmentMode::Offset { base, dx, dy } => {
                let (base_x, base_y, base_w, base_h) = match base {
                    AlignmentBase::Grid => {
                        if let Some(grid_ptr) = self.grid {
                            let grid = unsafe { &*grid_ptr };
                            if let Some(host_hwnd) = self.host_hwnd {
                                let mut client_rect: WinRect = unsafe { std::mem::zeroed() };
                                let got_client =
                                    unsafe { GetClientRect(host_hwnd as HWND, &mut client_rect) };
                                if got_client == 0 {
                                    return None;
                                }
                                let mut pt = winapi::shared::windef::POINT {
                                    x: client_rect.left,
                                    y: client_rect.top,
                                };
                                let got_screen =
                                    unsafe { ClientToScreen(host_hwnd as HWND, &mut pt) };
                                if got_screen == 0 {
                                    return None;
                                }
                                let dpi = Self::get_dpi_for_window(host_hwnd as HWND);
                                let scale = dpi as f32 / 96.0;
                                let grid_x = pt.x + (grid.rect.min.x * scale).round() as i32;
                                let grid_y = pt.y + (grid.rect.min.y * scale).round() as i32;
                                let grid_w = (grid.rect.size().x * scale).round() as i32;
                                let grid_h = (grid.rect.size().y * scale).round() as i32;
                                (grid_x, grid_y, grid_w, grid_h)
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    AlignmentBase::Host => {
                        if let Some(host_hwnd) = self.host_hwnd {
                            let mut host_rect: WinRect = unsafe { std::mem::zeroed() };
                            let got_host =
                                unsafe { GetWindowRect(host_hwnd as HWND, &mut host_rect) };
                            if got_host == 0 {
                                return None;
                            }
                            let width = host_rect.right - host_rect.left;
                            let height = host_rect.bottom - host_rect.top;
                            (host_rect.left, host_rect.top, width, height)
                        } else {
                            return None;
                        }
                    }
                };
                Some((base_x + dx, base_y + dy, base_w, base_h))
            }
        }
    }

    /// Move the target window according to the specified alignment mode
    pub unsafe fn move_window_to_alignment(&self, target_hwnd: HWND, mode: AlignmentMode) -> bool {
        match mode {
            AlignmentMode::Grid => {
                // Align client area to grid rect
                // Get grid rect in screen coordinates
                if let Some(grid_ptr) = self.grid {
                    let grid = unsafe { &*grid_ptr };
                    // Get host client area top-left in screen coordinates
                    if let Some(host_hwnd) = self.host_hwnd {
                        use winapi::shared::windef::RECT as WinRect;
                        use winapi::um::winuser::{
                            ClientToScreen, GetClientRect, GetWindowRect, SetWindowPos,
                            HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                        };
                        let mut client_rect: WinRect = unsafe { std::mem::zeroed() };
                        let got_client =
                            unsafe { GetClientRect(host_hwnd as HWND, &mut client_rect) };
                        if got_client == 0 {
                            return false;
                        }
                        let mut pt = winapi::shared::windef::POINT {
                            x: client_rect.left,
                            y: client_rect.top,
                        };
                        let got_screen = unsafe { ClientToScreen(host_hwnd as HWND, &mut pt) };
                        if got_screen == 0 {
                            return false;
                        }
                        let dpi = Self::get_dpi_for_window(host_hwnd as HWND);
                        let scale = dpi as f32 / 96.0;
                        let grid_x = pt.x + (grid.rect.min.x * scale).round() as i32;
                        let grid_y = pt.y + (grid.rect.min.y * scale).round() as i32;
                        let grid_w = (grid.rect.size().x * scale).round() as i32;
                        let grid_h = (grid.rect.size().y * scale).round() as i32;
                        // Adjust for non-client area of target window
                        let mut win_rect: WinRect = unsafe { std::mem::zeroed() };
                        let mut client_rect2: WinRect = unsafe { std::mem::zeroed() };
                        let got_win = unsafe { GetWindowRect(target_hwnd, &mut win_rect) };
                        let got_client2 = unsafe { GetClientRect(target_hwnd, &mut client_rect2) };
                        let mut client_pt = winapi::shared::windef::POINT {
                            x: client_rect2.left,
                            y: client_rect2.top,
                        };
                        let got_client_screen =
                            unsafe { ClientToScreen(target_hwnd, &mut client_pt) };
                        if got_win != 0 && got_client2 != 0 && got_client_screen != 0 {
                            let offset_x = client_pt.x - win_rect.left;
                            let offset_y = client_pt.y - win_rect.top;
                            let outer_x = grid_x - offset_x;
                            let outer_y = grid_y - offset_y;
                            let outer_w = grid_w
                                + (win_rect.right
                                    - win_rect.left
                                    - (client_rect2.right - client_rect2.left));
                            let outer_h = grid_h
                                + (win_rect.bottom
                                    - win_rect.top
                                    - (client_rect2.bottom - client_rect2.top));
                            let result = unsafe {
                                SetWindowPos(
                                    target_hwnd,
                                    HWND_TOPMOST,
                                    outer_x,
                                    outer_y,
                                    outer_w,
                                    outer_h,
                                    SWP_SHOWWINDOW | SWP_NOACTIVATE,
                                )
                            };
                            return result != 0;
                        }
                    }
                }
                false
            }
            AlignmentMode::HostExact => {
                // Align outer rect to host window's outer rect
                if let Some(host_hwnd) = self.host_hwnd {
                    use winapi::shared::windef::RECT as WinRect;
                    use winapi::um::winuser::{
                        GetWindowRect, SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                    };
                    let mut host_rect: WinRect = unsafe { std::mem::zeroed() };
                    let got_host = unsafe { GetWindowRect(host_hwnd as HWND, &mut host_rect) };
                    if got_host == 0 {
                        return false;
                    }
                    let width = host_rect.right - host_rect.left;
                    let height = host_rect.bottom - host_rect.top;
                    let result = unsafe {
                        SetWindowPos(
                            target_hwnd,
                            HWND_TOPMOST,
                            host_rect.left,
                            host_rect.top,
                            width,
                            height,
                            SWP_SHOWWINDOW | SWP_NOACTIVATE,
                        )
                    };
                    return result != 0;
                }
                false
            }
            AlignmentMode::Offset { base, dx, dy } => {
                // Align to an offset relative to grid or host
                let (base_x, base_y, base_w, base_h) = match base {
                    AlignmentBase::Grid => {
                        if let Some(grid_ptr) = self.grid {
                            let grid = unsafe { &*grid_ptr };
                            if let Some(host_hwnd) = self.host_hwnd {
                                use winapi::shared::windef::RECT as WinRect;
                                use winapi::um::winuser::{ClientToScreen, GetClientRect};
                                let mut client_rect: WinRect = unsafe { std::mem::zeroed() };
                                let got_client =
                                    unsafe { GetClientRect(host_hwnd as HWND, &mut client_rect) };
                                if got_client == 0 {
                                    return false;
                                }
                                let mut pt = winapi::shared::windef::POINT {
                                    x: client_rect.left,
                                    y: client_rect.top,
                                };
                                let got_screen =
                                    unsafe { ClientToScreen(host_hwnd as HWND, &mut pt) };
                                if got_screen == 0 {
                                    return false;
                                }
                                let dpi = Self::get_dpi_for_window(host_hwnd as HWND);
                                let scale = dpi as f32 / 96.0;
                                let grid_x = pt.x + (grid.rect.min.x * scale).round() as i32;
                                let grid_y = pt.y + (grid.rect.min.y * scale).round() as i32;
                                let grid_w = (grid.rect.size().x * scale).round() as i32;
                                let grid_h = (grid.rect.size().y * scale).round() as i32;
                                (grid_x, grid_y, grid_w, grid_h)
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    AlignmentBase::Host => {
                        if let Some(host_hwnd) = self.host_hwnd {
                            use winapi::shared::windef::RECT as WinRect;
                            use winapi::um::winuser::GetWindowRect;
                            let mut host_rect: WinRect = unsafe { std::mem::zeroed() };
                            let got_host =
                                unsafe { GetWindowRect(host_hwnd as HWND, &mut host_rect) };
                            if got_host == 0 {
                                return false;
                            }
                            let width = host_rect.right - host_rect.left;
                            let height = host_rect.bottom - host_rect.top;
                            (host_rect.left, host_rect.top, width, height)
                        } else {
                            return false;
                        }
                    }
                };
                use winapi::um::winuser::{
                    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                };
                let result = unsafe {
                    SetWindowPos(
                        target_hwnd,
                        HWND_TOPMOST,
                        base_x + dx,
                        base_y + dy,
                        base_w,
                        base_h,
                        SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    )
                };
                result != 0
            }
        }
    }
    /// Returns true if the window is topmost (WS_EX_TOPMOST set).
    #[cfg(target_os = "windows")]
    pub unsafe fn is_topmost(hwnd: HWND) -> bool {
        use winapi::um::winuser::GetWindowLongW;
        use winapi::um::winuser::GWL_EXSTYLE;
        use winapi::um::winuser::WS_EX_TOPMOST;
        let exstyle = GetWindowLongW(hwnd, GWL_EXSTYLE);
        (exstyle & WS_EX_TOPMOST as i32) != 0
    }

    /// Get the window rect (left, top, right, bottom) for a HWND
    #[cfg(target_os = "windows")]
    pub unsafe fn get_window_rect(hwnd: winapi::shared::windef::HWND) -> (i32, i32, i32, i32) {
        use winapi::um::winuser::GetWindowRect;
        let mut rect = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) != 0 {
            (rect.left, rect.top, rect.right, rect.bottom)
        } else {
            (0, 0, 0, 0)
        }
    }

    /// Move and resize a HWND to (x, y, w, h)
    #[cfg(target_os = "windows")]
    pub unsafe fn move_window(
        hwnd: winapi::shared::windef::HWND,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> bool {
        use winapi::um::winuser::MoveWindow;
        MoveWindow(hwnd, x, y, w, h, 1) != 0
    }

    /// Move the host window so that the grid remains aligned with the target window's rect
    /// This keeps the grid in the same position relative to the target after the target moves
    pub unsafe fn move_host_to_maintain_grid_alignment(
        &self,
        target_rect: (i32, i32, i32, i32),
    ) -> bool {
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};
        if let Some(host_hwnd) = self.host_hwnd {
            if let Some((grid_x, grid_y, grid_w, grid_h)) = self.get_grid_screen_rect() {
                let (target_x, target_y, target_w, target_h) = target_rect;
                // Calculate offset between target and grid
                let offset_x = target_x - grid_x;
                let offset_y = target_y - grid_y;
                // Move host window by the offset
                let new_host_x = (self
                    .get_host_screen_rect()
                    .map(|(x, _, _, _)| x)
                    .unwrap_or(0))
                    + offset_x;
                let new_host_y = (self
                    .get_host_screen_rect()
                    .map(|(_, y, _, _)| y)
                    .unwrap_or(0))
                    + offset_y;
                let new_host_w = self
                    .get_host_screen_rect()
                    .map(|(_, _, w, _)| w)
                    .unwrap_or(target_w);
                let new_host_h = self
                    .get_host_screen_rect()
                    .map(|(_, _, _, h)| h)
                    .unwrap_or(target_h);
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
    pub unsafe fn calculate_grid_rect_for_host(
        &self,
        host_rect: (i32, i32, i32, i32),
    ) -> Option<(i32, i32, i32, i32)> {
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
        use winapi::um::winuser::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW};
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
    pub unsafe fn place_above_host(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{
            SetWindowPos, HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        };
        if let Some(host_hwnd) = self.host_hwnd {
            println!("Placing target window above host window: {:?}", host_hwnd);
            let _ = unsafe {
                SetWindowPos(
                    target_hwnd,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                )
            };
            let result = unsafe {
                SetWindowPos(
                    target_hwnd,
                    host_hwnd as HWND,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                )
            };
            let _ = unsafe {
                SetWindowPos(
                    target_hwnd,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                )
            };
            result != 0
        } else {
            println!("No host window set, cannot place target window above.");
            false
        }
    }
    /// Places the target window just below the foreground window in Z order and removes topmost
    pub unsafe fn place_below_foreground(&self, target_hwnd: HWND) -> bool {
        use winapi::um::winuser::{
            GetForegroundWindow, SetWindowPos, HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, SWP_SHOWWINDOW,
        };
        let fg_hwnd = unsafe { GetForegroundWindow() };
        let result = unsafe {
            SetWindowPos(
                target_hwnd,
                fg_hwnd,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        let notopmost = unsafe {
            SetWindowPos(
                target_hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        result != 0 && notopmost != 0
    }

    /// Sets the target window's Z order directly above the specified HWND
    pub unsafe fn set_zorder_above(target_hwnd: HWND, above_hwnd: HWND) -> bool {
        use winapi::um::winuser::{
            SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
        };

        let result = unsafe {
            SetWindowPos(
                target_hwnd,
                above_hwnd,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        result != 0
    }

    /// Sets the target window as topmost or not topmost
    pub unsafe fn set_topmost(target_hwnd: HWND, topmost: bool) -> bool {
        println!("Setting target window topmost: {}", topmost);
        use winapi::um::winuser::{
            SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_SHOWWINDOW,
        };
        let flag_hwnd = if topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        let result = unsafe {
            SetWindowPos(
                target_hwnd,
                flag_hwnd,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        result != 0
    }
    /// Returns true if the HWND is a valid window
    pub unsafe fn is_window(hwnd: HWND) -> bool {
        use winapi::um::winuser::IsWindow;
        unsafe { IsWindow(hwnd) != 0 }
    }

    pub unsafe fn get_dpi_for_window(hwnd: HWND) -> u32 {
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
    pub unsafe fn get_host_screen_rect(&self) -> Option<(i32, i32, i32, i32)> {
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
    pub unsafe fn get_grid_screen_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let hwnd = self.host_hwnd?;
        let grid_ptr = self.grid?;
        let grid = unsafe { &*grid_ptr };
        let mut client_rect: RECT = unsafe { std::mem::zeroed() };
        let got_client =
            unsafe { winapi::um::winuser::GetClientRect(hwnd as HWND, &mut client_rect) };
        if got_client == 0 {
            return None;
        }
        // Get client area's top-left in screen coordinates
        let mut pt = winapi::shared::windef::POINT {
            x: client_rect.left,
            y: client_rect.top,
        };
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
    pub unsafe fn move_pinned_hwnd_to_grid(&self, pinned_hwnd: HWND) -> bool {
        /// Uses AlignmentMode::Grid. For custom alignment, call move_window_to_alignment directly.
        self.move_window_to_alignment(pinned_hwnd, AlignmentMode::Grid)
    }

    /// Move and resize a target window to the grid position relative to the host window
    pub unsafe fn move_and_resize(&self, target_hwnd: HWND) -> bool {
        /// Uses AlignmentMode::Grid. For custom alignment, call move_window_to_alignment directly.
        self.move_window_to_alignment(target_hwnd, AlignmentMode::Grid)
    }
    /// Returns true if the host window is currently focused (has foreground)
    /// Returns true if the given HWND is currently focused (has foreground)
    pub unsafe fn is_window_focused(hwnd: HWND) -> bool {
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
    pub unsafe fn set_pinned_zorder(&self, pinned_hwnd: HWND, above: bool) -> bool {
        if let Some(host_hwnd) = self.host_hwnd {
            if above {
                // Place pinned HWND directly above host HWND
                return Self::set_zorder_above(pinned_hwnd, host_hwnd as HWND);
            } else {
                // Place pinned HWND directly below host HWND
                use winapi::um::winuser::{
                    SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
                };
                let result = unsafe {
                    SetWindowPos(
                        pinned_hwnd,
                        host_hwnd as HWND,
                        0,
                        0,
                        0,
                        0,
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
    pub fn new(_hwnd: ()) -> Self {
        Self
    }
    pub fn get_screen_rect(&self, _grid: &PositionGrid) -> Option<(i32, i32, i32, i32)> {
        None
    }
}
