/// Cross-platform screen size detection for e_window
/// Returns (width, height, method_used)
pub fn get_cross_platform_screen_size() -> (u32, u32, &'static str) {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
        (width, height, "winapi")
    }
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::{CGDisplayBounds, CGMainDisplayID};
        let display_id = unsafe { CGMainDisplayID() };
        let bounds = unsafe { CGDisplayBounds(display_id) };
        let width = bounds.size.width as u32;
        let height = bounds.size.height as u32;
        (width, height, "core-graphics")
    }
    #[cfg(target_os = "linux")]
    {
        use std::ptr;
        use x11::xlib::*;
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if !display.is_null() {
                let screen = XDefaultScreen(display);
                let width = XDisplayWidth(display, screen) as u32;
                let height = XDisplayHeight(display, screen) as u32;
                XCloseDisplay(display);
                (width, height, "x11")
            } else {
                (1920, 1080, "fallback-x11")
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        (1920, 1080, "fallback")
    }
}

/// Legacy API: Returns (width, height) only
pub fn get_screen_size() -> (u32, u32) {
    let (w, h, _) = get_cross_platform_screen_size();
    (w, h)
}

pub fn get_cross_platform_screen_size_f64() -> (f64, f64, &'static str) {
    #[cfg(target_os = "windows")]
    {
        use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        (width.into(), height.into(), "winapi")
    }
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::{CGDisplayBounds, CGMainDisplayID};
        let display_id = unsafe { CGMainDisplayID() };
        let bounds = unsafe { CGDisplayBounds(display_id) };
        let width = bounds.size.width;
        let height = bounds.size.height;
        (width, height, "core-graphics")
    }
    #[cfg(target_os = "linux")]
    {
        use std::ptr;
        use x11::xlib::*;
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if !display.is_null() {
                let screen = XDefaultScreen(display);
                let width = XDisplayWidth(display, screen);
                let height = XDisplayHeight(display, screen);
                XCloseDisplay(display);
                (width.into(), height.into(), "x11")
            } else {
                (1920.0, 1080.0, "fallback-x11")
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        (1920.0, 1080.0, "fallback")
    }
}
