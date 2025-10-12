// Test all message box types using the new --type and --body approach
use std::process::Command;

fn test_message_box_type(msg_type: &str, body: &str) -> std::io::Result<()> {
    println!("Testing {} message box...", msg_type);

    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "e_window", "--", "--type", msg_type, "--body", body,
        ])
        .current_dir(r"C:\w\cargo-e.clean")
        .output()?;

    if output.status.success() {
        println!("✅ {} message box completed successfully", msg_type);
    } else {
        println!(
            "❌ {} message box failed: {}",
            msg_type,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("=== Testing All Message Box Types ===\n");

    // Test all standard message box types
    test_message_box_type("Ok", "Simple OK message box")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("OkCancel", "OK/Cancel confirmation dialog")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("YesNo", "Yes/No question dialog")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("YesNoDefNo", "Yes/No with default No")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("YesNoCancel", "Yes/No/Cancel with three options")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("YesNoCancelDefNo", "Yes/No/Cancel with default No")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("RetryCancel", "Retry/Cancel error dialog")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("TextInput", "Please enter your name:")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    test_message_box_type("FileSelection", "Choose a file to open:")?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    println!("\n=== All Message Box Tests Completed ===");

    Ok(())
}
