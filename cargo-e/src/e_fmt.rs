use std::time::Duration;

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    } else if minutes > 0 {
        format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
    } else {
        format!("{:02}.{:03}", seconds, millis)
    }
}
/// Helper: Format a Duration in a humanedable way.
pub fn format_duration_secs(d: Duration) -> String {
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    format!("{}.{:03} secs", secs, millis)
}

/// Helper: Format a byte count in humanedable form.
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
