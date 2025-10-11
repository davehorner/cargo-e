// Simple test to check App drop behavior
use std::process::Command;

fn main() {
    println!("=== Testing App drop behavior ===");
    
    // Run a simple message box and capture output
    let output = Command::new("target/debug/e_window.exe")
        .args(&["--type", "Ok", "--body", "Test message"])
        .output()
        .expect("Failed to execute e_window");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Count "App is being dropped" messages
    let drop_count_stdout = stdout.matches("App is being dropped").count();
    let drop_count_stderr = stderr.matches("App is being dropped").count();
    let total_drops = drop_count_stdout + drop_count_stderr;
    
    println!("STDOUT drop messages: {}", drop_count_stdout);
    println!("STDERR drop messages: {}", drop_count_stderr);
    println!("Total drop messages: {}", total_drops);
    
    if total_drops > 1 {
        println!("❌ PROBLEM: Found {} drop messages (expected 1)", total_drops);
        println!("\nSTDOUT:");
        println!("{}", stdout);
        println!("\nSTDERR:");
        println!("{}", stderr);
    } else {
        println!("✅ SUCCESS: Found {} drop message (expected 1)", total_drops);
    }
}