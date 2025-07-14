use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum ControlCommand {
    Exit,
    SetRect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },
    SetRectEased {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        duration_ms: u32,
        easing: String,
    },
    SetTitle(String),
    BeginDocument,
    EndDocument,
    Content(String),
    Delay(u32), // milliseconds
}

pub fn parse_control(line: &str) -> Option<ControlCommand> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("!control:") {
        let rest = rest.trim();
        if rest.starts_with("exit") {
            return Some(ControlCommand::Exit);
        } else if rest.starts_with("set_rect_eased") {
            // Syntax: set_rect_eased x y w h duration_ms easing
            let parts: Vec<_> = rest["set_rect_eased".len()..]
                .trim()
                .split_whitespace()
                .collect();
            if parts.len() == 6 {
                if let (Ok(x), Ok(y), Ok(w), Ok(h), Ok(duration_ms)) = (
                    parts[0].parse(),
                    parts[1].parse(),
                    parts[2].parse(),
                    parts[3].parse(),
                    parts[4].parse(),
                ) {
                    let easing = parts[5].to_string();
                    return Some(ControlCommand::SetRectEased {
                        x,
                        y,
                        w,
                        h,
                        duration_ms,
                        easing,
                    });
                }
            }
        } else if rest.starts_with("set_rect") {
            let parts: Vec<_> = rest["set_rect".len()..].trim().split_whitespace().collect();
            if parts.len() == 4 {
                if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    parts[0].parse(),
                    parts[1].parse(),
                    parts[2].parse(),
                    parts[3].parse(),
                ) {
                    return Some(ControlCommand::SetRect { x, y, w, h });
                }
            }
        } else if rest.starts_with("set_title") {
            let title = rest["set_title".len()..].trim().to_string();
            return Some(ControlCommand::SetTitle(title));
        } else if rest == "begin_document" {
            return Some(ControlCommand::BeginDocument);
        } else if rest == "end_document" {
            return Some(ControlCommand::EndDocument);
        } else if rest.starts_with("delay") {
            let parts: Vec<_> = rest["delay".len()..].trim().split_whitespace().collect();
            if parts.len() == 1 {
                if let Ok(ms) = parts[0].parse() {
                    return Some(ControlCommand::Delay(ms));
                }
            }
        }
    }
    None
}

pub fn start_stdin_listener(tx: Sender<ControlCommand>) {
    std::thread::spawn(move || {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if let Some(cmd) = parse_control(&line) {
                tx.send(cmd).ok();
            } else {
                tx.send(ControlCommand::Content(line)).ok();
            }
        }
    });
}

pub fn start_stdin_listener_with_buffer(
    tx: Sender<ControlCommand>,
    buffer: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) {
    std::thread::spawn(move || {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            // Buffer the line
            buffer.lock().unwrap().push(line.clone());
            if let Some(cmd) = parse_control(&line) {
                tx.send(cmd).ok();
            } else {
                tx.send(ControlCommand::Content(line)).ok();
            }
        }
    });
}
