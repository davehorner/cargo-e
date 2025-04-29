#[cfg(target_os = "windows")]
pub fn auto_sense_llvm() {
    use which::which;
    use std::process::Command;

    // Check if choco is installed
    if which("choco").is_err() {
        eprintln!("Chocolatey (choco) is not installed.");
        println!("Please install Chocolatey from https://chocolatey.org/install to proceed with LLVM installation.");
        return;
    }

    println!("libclang is missing. You can install LLVM using Chocolatey (choco).");
    println!("Suggestion: choco install llvm");

    match crate::e_prompts::yesno(
        "Do you want to install LLVM using choco?",
        Some(true), // Default to yes
    ) {
        Ok(Some(true)) => {
            println!("Installing LLVM...");
            match Command::new("choco")
                .args(["install", "llvm", "-y"])
                .spawn()
            {
                Ok(mut child) => {
                    child.wait().ok(); // Wait for installation to complete
                    println!("LLVM installation completed.");
                }
                Err(e) => {
                    eprintln!("Error installing LLVM via choco: {}", e);
                }
            }
        }
        Ok(Some(false)) => {
            println!("LLVM installation skipped.");
        }
        Ok(None) => {
            println!("Installation cancelled (timeout or invalid input).");
        }
        Err(e) => {
            eprintln!("Error during prompt: {}", e);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn auto_sense_llvm() {
    println!("auto_sense_llvm is only supported on Windows with Chocolatey.");
}
