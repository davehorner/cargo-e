use crate::e_prompts::yesno;
use anyhow::{bail, Context, Result};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::which;

// https://github.com/ahaoboy/is-admin
#[cfg(windows)]
pub fn is_admin() -> bool {
    let shell = "[bool]([System.Security.Principal.WindowsPrincipal][System.Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)";
    let output = std::process::Command::new("powershell")
        .args(["-c", shell])
        .output()
        .expect("Failed to execute PowerShell command");
    String::from_utf8(output.stdout).unwrap_or_default().trim() == "True"
}

// https://github.com/ahaoboy/is-admin
#[cfg(unix)]
pub fn is_admin() -> bool {
    use libc::{geteuid, getuid};
    unsafe { getuid() == 0 || geteuid() == 0 }
}

/// Check if the program is running as an administrator.
/// Returns an error if the program is not running with administrative privileges.
pub fn ensure_admin_privileges() -> Result<()> {
    if !is_admin() {
        return Err(anyhow::anyhow!(
            "This program must be run as an administrator. Please restart it with administrative privileges."
        ));
    }
    Ok(())
}
/// Ensure `npm` is on PATH.  
/// Ensures Node.js is installed first.  
/// Returns the full path to the `npm` executable, or an error.
pub fn ensure_npm() -> Result<PathBuf> {
    // Ensure Node.js is installed
    ensure_node()?;
    which("npm").context("`npm` not found in PATH. Please install Node.js and npm.")
}

/// Ensure the `napi` CLI is on PATH (provided by `@napi-rs/cli`).  
/// If missing, prompts the user and installs it globally via `npm install -g @napi-rs/cli`.  
/// Returns the full path to the `napi` executable.
pub fn ensure_napi_cli() -> Result<PathBuf, Box<dyn Error>> {
    // 1) Already installed?
    if let Ok(path) = which("napi") {
        return Ok(path);
    }

    // 2) Prompt the user to install it via npm
    println!("`napi` CLI not found. Install it globally now?");
    match yesno(
        "Do you want to install `@napi-rs/cli` globally via npm?",
        Some(true),
    ) {
        Ok(Some(true)) => {
            let npm = ensure_npm()?;
            println!("Installing `@napi-rs/cli` via `npm install -g @napi-rs/cli`…");
            let mut child = Command::new(npm)
                .args(&["install", "-g", "@napi-rs/cli"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| format!("Failed to spawn install command: {}", e))?;

            child
                .wait()
                .map_err(|e| format!("Error while waiting for installation: {}", e))?;
        }
        Ok(Some(false)) => return Err("User skipped installing `@napi-rs/cli`".into()),
        Ok(None) => return Err("Installation of `@napi-rs/cli` cancelled (timeout)".into()),
        Err(e) => return Err(format!("Error during prompt: {}", e).into()),
    }

    // 3) Retry locating `napi`
    which("napi").map_err(|_| "`napi` still not found after installation".into())
}

/// Ensure `cross-env` is on PATH.  
/// If it’s missing, prompts the user and installs it globally via `npm install -g cross-env`.  
/// Returns the full path to the `cross-env` executable.
pub fn ensure_cross_env() -> Result<PathBuf, Box<dyn Error>> {
    // 1) Already installed?
    if let Ok(path) = which("cross-env") {
        return Ok(path);
    }

    // 2) Prompt the user to install it via npm
    println!("`cross-env` is not installed. Install it globally now?");
    match yesno(
        "Do you want to install `cross-env` globally via npm?",
        Some(true),
    ) {
        Ok(Some(true)) => {
            // Make sure npm is available
            let npm = ensure_npm()?;
            println!("Installing `cross-env` via `npm install -g cross-env`…");
            let mut child = Command::new(npm)
                .args(&["install", "-g", "cross-env"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| format!("Failed to spawn install command: {}", e))?;

            // Wait for the installation to finish
            child
                .wait()
                .map_err(|e| format!("Error while waiting for installation: {}", e))?;
        }
        Ok(Some(false)) => return Err("User skipped installing `cross-env`".into()),
        Ok(None) => return Err("Installation of `cross-env` cancelled (timeout)".into()),
        Err(e) => return Err(format!("Error during prompt: {}", e).into()),
    }

    // 3) Retry locating `cross-env`
    which("cross-env").map_err(|_| "`cross-env` still not found after installation".into())
}
/// Ensure `pnpm` is on PATH.  
/// Ensures Node.js is installed first.  
/// If it’s missing, will use `npm` (via `ensure_npm`) to install `pnpm` globally.  
/// Returns the full path to the `pnpm` executable.
pub fn ensure_pnpm() -> Result<PathBuf> {
    // Ensure Node.js is installed
    ensure_node()?;

    // Check if `pnpm` is already installed
    if let Ok(path) = which("pnpm") {
        return Ok(path);
    }

    // Otherwise, prompt the user to install it via npm
    println!("`pnpm` is not installed. Install it now?");
    match yesno(
        "Do you want to install `pnpm` globally via npm?",
        Some(true),
    ) {
        Ok(Some(true)) => {
            // Make sure we have npm
            let npm_path = ensure_npm()?;
            println!("Installing `pnpm` via `npm install -g pnpm`…");

            let mut child = Command::new(npm_path)
                .args(&["install", "-g", "pnpm"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("failed to spawn `npm install -g pnpm`")?;

            child
                .wait()
                .context("error while waiting for `npm install -g pnpm` to finish")?;
        }
        Ok(Some(false)) => bail!("user skipped installing `pnpm`"),
        Ok(None) => bail!("installation of `pnpm` cancelled (timeout)"),
        Err(e) => bail!("error during prompt: {}", e),
    }

    // Retry locating `pnpm`
    which("pnpm").context("`pnpm` still not found in PATH after installation")
}

/// Ensure the `dx` CLI (the Dioxus helper) is on PATH.
/// If missing, prompts the user to install the Dioxus CLI via `cargo install dioxus-cli`.
/// Returns the full path to the `dx` executable.
pub fn ensure_dx() -> Result<PathBuf> {
    // 1) Check if `dx` is already on PATH
    if let Ok(path) = which("dx") {
        return Ok(path);
    }

    // 2) Prompt the user to install it
    println!("`dx` CLI not found. Install the Dioxus CLI now?");
    match yesno(
        "Do you want to install the Dioxus CLI via `cargo install dioxus-cli`?",
        Some(true),
    ) {
        Ok(Some(true)) => {
            println!("Installing `dioxus-cli` via `cargo install dioxus-cli`…");
            let mut child = Command::new("cargo")
                .args(&["install", "dioxus-cli"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("failed to spawn `cargo install dioxus-cli`")?;

            child
                .wait()
                .context("error while waiting for `cargo install dioxus-cli` to finish")?;
        }
        Ok(Some(false)) => bail!("user skipped installing the Dioxus CLI"),
        Ok(None) => bail!("installation of the Dioxus CLI cancelled (timeout)"),
        Err(e) => bail!("error during prompt: {}", e),
    }

    // 3) Retry locating `dx`
    which("dx").context("`dx` still not found in PATH after installation")
}

/// Ensure `trunk` is on PATH.  
/// Returns the full path to the `trunk` executable, or an error.
pub fn ensure_trunk() -> Result<PathBuf> {
    // 1) First try to locate `trunk`
    if let Ok(path) = which("trunk") {
        return Ok(path);
    }

    // 2) Prompt the user to install it
    println!("`trunk` is not installed. Install it now?");
    match yesno("Do you want to install `trunk`?", Some(true)) {
        Ok(Some(true)) => {
            println!("Installing `trunk` via `cargo install trunk`…");
            let mut child = Command::new("cargo")
                .args(&["install", "trunk"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("failed to spawn `cargo install trunk`")?;

            child
                .wait()
                .context("failed while waiting for `cargo install trunk` to finish")?;
        }
        Ok(Some(false)) => {
            anyhow::bail!("user skipped installing `trunk`");
        }
        Ok(None) => {
            anyhow::bail!("installation of `trunk` cancelled (timeout)");
        }
        Err(e) => {
            anyhow::bail!("error during prompt: {}", e);
        }
    }

    // 3) Re‐try locating `trunk`
    which("trunk").context("`trunk` still not found in PATH after installation")
}

/// Ensure `rust-script` is on PATH.  
/// Returns the full path to the `rust-script` executable, or an error.
pub fn ensure_rust_script() -> Result<PathBuf> {
    // 1) First try to locate `trunk`
    if let Ok(path) = which("rust-script") {
        return Ok(path);
    }

    // 2) Prompt the user to install it
    println!("`rust-script` is not installed. Install it now?");
    match yesno("Do you want to install `rust-script`?", Some(true)) {
        Ok(Some(true)) => {
            println!("Installing `rust-script` via `cargo install rust-script`…");
            let mut child = Command::new("cargo")
                .args(&["install", "rust-script"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("failed to spawn `cargo install rust-script`")?;

            child
                .wait()
                .context("failed while waiting for `cargo install rust-script` to finish")?;
        }
        Ok(Some(false)) => {
            anyhow::bail!("user skipped installing `rust-script`");
        }
        Ok(None) => {
            anyhow::bail!("installation of `rust-script` cancelled (timeout)");
        }
        Err(e) => {
            anyhow::bail!("error during prompt: {}", e);
        }
    }
    which("rust-script").context("`rust-script` still not found in PATH after installation")
}
// Helper function to check for package.json and run npm install if needed
pub fn check_npm_and_install(
    workspace_parent: &Path,
    be_silent: bool,
) -> Result<(), Box<dyn Error>> {
    if workspace_parent.join("pnpm-workspace.yaml").exists() {
        // If this is a pnpm workspace, skip npm checks
        println!("Skipping npm checks for pnpm workspace.");
        return Ok(());
    }
    // Check if package.json exists at the workspace parent level
    if !be_silent {
        println!(
            "Checking for package.json in: {}",
            workspace_parent.display()
        );
    }
    if workspace_parent.join("package.json").exists() {
        if !be_silent {
            println!("package.json found in: {}", workspace_parent.display());
        }
        // Get the path to npm using `which`.
        match which("npm") {
            Ok(npm_path) => {
                if !be_silent {
                    println!("Found npm at: {}", npm_path.display());
                }

                // Run `npm ls --depth=1` in the specified directory
                let output = Command::new(npm_path.clone())
                    .arg("ls")
                    .arg("--depth=1")
                    .current_dir(workspace_parent)
                    .output()
                    .map_err(|e| eprintln!("Failed to execute npm ls: {}", e))
                    .ok();

                if let Some(output) = output {
                    if !be_silent {
                        println!("npm ls output: {}", String::from_utf8_lossy(&output.stdout));
                    }
                    if !output.status.success() {
                        // Print the npm error output for debugging.
                        eprintln!(
                            "npm ls failed for directory: {}",
                            workspace_parent.display()
                        );
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));

                        // Run `npm install` to fix the missing dependencies
                        if !be_silent {
                            println!(
                                "Running npm install in directory: {}",
                                workspace_parent.display()
                            );
                        }
                        let install_output = Command::new(npm_path)
                            .arg("install")
                            .current_dir(workspace_parent)
                            .output()
                            .map_err(|e| eprintln!("Failed to execute npm install: {}", e))
                            .ok();

                        if !be_silent {
                            if let Some(install_output) = install_output {
                                println!(
                                    "npm install output: {}",
                                    String::from_utf8_lossy(&install_output.stdout)
                                );
                                if install_output.status.success() {
                                    println!(
                                        "npm install completed successfully in: {}",
                                        workspace_parent.display()
                                    );
                                } else {
                                    eprintln!(
                                        "npm install failed in directory: {}",
                                        workspace_parent.display()
                                    );
                                    eprintln!(
                                        "{}",
                                        String::from_utf8_lossy(&install_output.stderr)
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("npm is not installed or not in the system PATH.");
                return Err("npm not found".into());
            }
        }
    }
    Ok(())
}

/// Check for a pnpm workspace and, if found, run `pnpm install`.  
/// Returns the full path to the `pnpm` executable.
pub fn check_pnpm_and_install(workspace_parent: &Path, be_silent: bool) -> Result<PathBuf> {
    // if this is a pnpm workspace, install deps
    let workspace_yaml = workspace_parent.join("pnpm-workspace.yaml");
    if workspace_yaml.exists() {
        // ensure pnpm is available (and install it if necessary)
        let pnpm = ensure_pnpm()?;
        if !be_silent {
            println!(
                "Found pnpm-workspace.yaml in: {}",
                workspace_parent.display()
            );
            println!("Running `pnpm install`…");
        }

        let status = Command::new(&pnpm)
            .arg("install")
            .current_dir(workspace_parent)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("failed to execute `pnpm install`")?;

        if !status.success() {
            bail!("`pnpm install` failed with exit code {}", status);
        }
        //         if cfg!( target_os = "windows" ) {
        //             #[cfg(windows)]
        // use std::os::windows::process::CommandExt;
        //             // WinAPI flag for “create a new console window”
        // #[cfg(windows)]
        // const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        //             println!("Running `pnpm run build:debug windows");
        //                 // Build the command
        //     let mut cmd = Command::new("cmd");
        //     cmd.args(&["/C", "pnpm run build:debug"])
        //        .current_dir(workspace_parent);
        //     //    .stdin(Stdio::null())
        //     //    .stdout(Stdio::inherit())
        //     //    .stderr(Stdio::inherit());

        //     // On Windows, ask for a new console window
        //     #[cfg(windows)]
        //     {
        //         cmd.creation_flags(CREATE_NEW_CONSOLE);
        //     }
        //                 let status =
        //     cmd.status()?;
        // if !status.success() {
        //     anyhow::bail!("`pnpm run build:debug` failed with {}", status);
        // }
        //         } else {
        // ensure_napi_cli().ok();
        // ensure_cross_env().ok();
        Command::new(&pnpm)
            .args(&["run", "build:debug"])
            .current_dir(workspace_parent)
            .env("CARGO", "cargo")
            .status()?;
        // };

        if !be_silent {
            println!("pnpm install succeeded");
        }
        return Ok(pnpm);
    } else {
        if !be_silent {
            println!(
                "No pnpm-workspace.yaml found in {}, skipping `pnpm install`.",
                workspace_parent.display()
            );
        }
    }
    Ok(PathBuf::new())
}

/// Ensure `node` is on PATH.  
/// If missing, attempts to install Node.js using `nvm` (automated for Windows, manual prompt otherwise).  
/// Returns the full path to the `node` executable.
pub fn ensure_node() -> Result<PathBuf> {
    // Check if `node` is already installed
    if let Ok(path) = which("node") {
        return Ok(path);
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use Chocolatey to install NVM and set Node.js to LTS
        println!("`node` is not installed.");
        match yesno(
            "Do you want to install Node.js using NVM (via Chocolatey)?",
            Some(true),
        ) {
            Ok(Some(true)) => {
                println!("Installing NVM via Chocolatey...");
                let choco = ensure_choco()?;
                let mut child = Command::new(choco)
                    .args(&["install", "nvm"]) //, "-y"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .context("Failed to spawn `choco install nvm`")?;

                child
                    .wait()
                    .context("Error while waiting for `choco install nvm` to finish")?;

                // Use NVM to install and use the latest LTS version of Node.js
                let nvm = which("nvm").context("`nvm` not found in PATH after installation.")?;
                let mut child = Command::new(&nvm)
                    .args(&["install", "lts"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .context("Failed to spawn `nvm install lts`")?;

                child
                    .wait()
                    .context("Error while waiting for `nvm install lts` to finish")?;

                let mut child = Command::new(&nvm)
                    .args(&["use", "lts"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .context("Failed to spawn `nvm use lts`")?;

                child
                    .wait()
                    .context("Error while waiting for `nvm use lts` to finish")?;
            }
            Ok(Some(false)) => {
                anyhow::bail!("User declined to install Node.js.");
            }
            Ok(None) => {
                anyhow::bail!("Installation of Node.js cancelled (timeout).");
            }
            Err(e) => {
                anyhow::bail!("Error during prompt: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows systems, prompt the user to install Node.js manually
        println!("`node` is not installed. Please install Node.js manually.");
        anyhow::bail!("Node.js installation is not automated for this platform.");
    }

    // Retry locating `node`
    which("node").context("`node` still not found after installation")
}

/// Ensure the GitHub CLI (`gh`) is on PATH.  
/// If missing, installs it using Chocolatey on Windows.  
/// Returns the full path to the `gh` executable.
pub fn ensure_github_gh() -> Result<PathBuf> {
    // Check if `gh` is already installed
    if let Ok(path) = which("gh") {
        return Ok(path);
    }
    // Check if `gh.exe` exists in the default installation path
    let default_path = Path::new("C:\\Program Files\\GitHub CLI\\gh.exe");
    if default_path.exists() {
        return Ok(default_path.to_path_buf());
    }
    #[cfg(target_os = "windows")]
    {
        // Ensure Chocolatey is installed
        let choco = ensure_choco()?;

        // Install GitHub CLI using Chocolatey
        println!("Installing GitHub CLI (`gh`) via Chocolatey...");
        if let Err(e) = ensure_admin_privileges() {
            eprintln!("Error: {}", e);
            return Err(e);
        }
        let mut child = Command::new(choco)
            .args(&["install", "gh", "y"])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn `choco install gh`")?;

        child
            .wait()
            .context("Error while waiting for `choco install gh` to finish")?;
        if default_path.exists() {
            return Ok(default_path.to_path_buf());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("GitHub CLI installation is only automated on Windows.");
    }

    // Retry locating `gh`
    which("gh").context("`gh` still not found after installation")
}

/// Ensure `choco` (Chocolatey) is on PATH.  
/// If missing, prompts the user to install Chocolatey manually.  
/// Returns the full path to the `choco` executable.
pub fn ensure_choco() -> Result<PathBuf> {
    // Check if `choco` is already installed
    if let Ok(path) = which("choco") {
        return Ok(path);
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, prompt the user to install Chocolatey manually
        println!("`choco` (Chocolatey) is not installed.");
        println!("It is required to proceed. Do you want to install it manually?");
        match yesno(
            "Do you want to install Chocolatey manually by following the instructions?",
            Some(true),
        ) {
            Ok(Some(true)) => {
                println!("Please run the following command in PowerShell to install Chocolatey:");
                println!("Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))");
                anyhow::bail!(
                    "Chocolatey installation is not automated. Please install it manually."
                );
            }
            Ok(Some(false)) => {
                anyhow::bail!("User declined to install Chocolatey.");
            }
            Ok(None) => {
                anyhow::bail!("Installation of Chocolatey cancelled (timeout).");
            }
            Err(e) => {
                anyhow::bail!("Error during prompt: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Chocolatey is only supported on Windows.");
    }
}

/// Ensure the `cargo-leptos` CLI is on PATH.  
/// If missing, prompts the user to install it via `cargo install cargo-leptos`.  
/// Returns the full path to the `cargo-leptos` executable.
pub fn ensure_leptos() -> Result<PathBuf> {
    // 1) Check if `cargo-leptos` is already on PATH
    if let Ok(path) = which("cargo-leptos") {
        return Ok(path);
    }

    // 2) Prompt the user to install it
    println!("`cargo-leptos` CLI not found. Install it now?");
    match yesno(
        "Do you want to install the `cargo-leptos` CLI via `cargo install cargo-leptos`?",
        Some(true),
    ) {
        Ok(Some(true)) => {
            // Check if `perl` is available
            if which("perl").is_err() {
                println!("`perl` is not installed or not found in PATH.");
                println!("OpenSSL requires `perl` for installation unless OpenSSL is already configured in your environment.");
                println!("It is recommended to have a working `perl` distribution installed for openssl.");
                ensure_perl();
            }

            println!("Installing `cargo-leptos` via `cargo install cargo-leptos`…");
            let mut child = Command::new("cargo")
                .args(&["install", "cargo-leptos"])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .context("Failed to spawn `cargo install cargo-leptos`")?;

            child
                .wait()
                .context("Error while waiting for `cargo install cargo-leptos` to finish")?;
        }
        Ok(Some(false)) => bail!("User skipped installing `cargo-leptos`."),
        Ok(None) => bail!("Installation of `cargo-leptos` cancelled (timeout)."),
        Err(e) => bail!("Error during prompt: {}", e),
    }

    // 3) Retry locating `cargo-leptos`
    which("cargo-leptos").context("`cargo-leptos` still not found after installation")
}

#[cfg(target_os = "windows")]
pub fn ensure_perl() {
    use std::process::Command;
    use which::which;

    // Check if choco is installed
    if which("choco").is_err() {
        eprintln!("Chocolatey (choco) is not installed.");
        println!("Please install Chocolatey from https://chocolatey.org/install to proceed with Perl installation.");
        return;
    }

    println!("Perl is missing. You can install Strawberry Perl using Chocolatey (choco).");
    println!("Suggestion: choco install strawberryperl");

    match crate::e_prompts::yesno(
        "Do you want to install Strawberry Perl using choco?",
        Some(true), // Default to yes
    ) {
        Ok(Some(true)) => {
            println!("Installing Strawberry Perl...");
            match Command::new("choco")
                .args(["install", "strawberryperl", "-y"])
                .spawn()
            {
                Ok(mut child) => {
                    child.wait().ok(); // Wait for installation to complete
                    println!("Strawberry Perl installation completed.");
                }
                Err(e) => {
                    eprintln!("Error installing Strawberry Perl via choco: {}", e);
                }
            }
        }
        Ok(Some(false)) => {
            println!("Strawberry Perl installation skipped.");
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
pub fn ensure_perl() {
    println!("auto_sense_perl is only supported on Windows with Chocolatey.");
}
