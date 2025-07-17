//! window_easing.rs
//! Provides window movement easing/animation for HWNDs on Windows.

#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::shared::windef::RECT;
#[cfg(target_os = "windows")]
use winapi::um::winuser::GetWindowRect;
#[cfg(target_os = "windows")]
use winapi::um::winuser::SetWindowPos;

#[cfg(target_os = "windows")]
pub struct WindowEasing {
    pub hwnd: HWND,
    pub target_rect: (i32, i32, i32, i32),
    pub duration_ms: u32,
    pub start_time: std::time::Instant,
    pub easing_fn: fn(f32) -> f32,
    pub start_rect: (i32, i32, i32, i32),
}

#[cfg(target_os = "windows")]
impl WindowEasing {
    pub fn new(hwnd: HWND, target_rect: (i32, i32, i32, i32), duration_ms: u32, easing_fn: fn(f32) -> f32) -> Self {
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        let got_rect = unsafe { GetWindowRect(hwnd, &mut rect) };
        let start_rect = if got_rect != 0 {
            (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top)
        } else {
            target_rect
        };
        Self {
            hwnd,
            target_rect,
            duration_ms,
            start_time: std::time::Instant::now(),
            easing_fn,
            start_rect,
        }
    }

    /// Call this every frame to animate the window
    pub fn update(&mut self) -> bool {
        let elapsed = self.start_time.elapsed().as_millis() as u32;
        let t = (elapsed as f32 / self.duration_ms as f32).min(1.0);
        let ease = (self.easing_fn)(t);
        let (sx, sy, sw, sh) = self.start_rect;
        let (tx, ty, tw, th) = self.target_rect;
        let x = sx + ((tx - sx) as f32 * ease).round() as i32;
        let y = sy + ((ty - sy) as f32 * ease).round() as i32;
        let w = sw + ((tw - sw) as f32 * ease).round() as i32;
        let h = sh + ((th - sh) as f32 * ease).round() as i32;
        let result = unsafe {
            SetWindowPos(
                self.hwnd,
                winapi::um::winuser::HWND_TOPMOST,
                x,
                y,
                w,
                h,
                winapi::um::winuser::SWP_SHOWWINDOW | winapi::um::winuser::SWP_NOACTIVATE,
            )
        };
        t >= 1.0 || result == 0
    }
}

#[cfg(target_os = "windows")]
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

// Usage:
// let mut easing = WindowEasing::new(hwnd, (x, y, w, h), 500, ease_in_out_quad);
// loop { if easing.update() { break; } std::thread::sleep(Duration::from_millis(16)); }
