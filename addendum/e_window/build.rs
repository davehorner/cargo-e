fn main() {
    // Get current date and time in a reliable way
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Convert to a readable format
    let datetime = chrono::DateTime::from_timestamp(now as i64, 0)
        .unwrap_or_else(|| chrono::Utc::now())
        .format("%Y-%m-%d %H:%M:%S UTC");
    
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", datetime);
    
    // Rerun if this build script changes
    println!("cargo:rerun-if-changed=build.rs");
}