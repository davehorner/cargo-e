use crate::e_prompts::yesno;
use anyhow::{bail, Context, Result};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use which::which;

/// Ensure `npm` is on PATH.  
/// Returns the full path to the `npm` executable, or an error.
pub fn ensure_npm() -> Result<PathBuf> {
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
/// If it’s missing, will use `npm` (via `ensure_npm`) to install `pnpm` globally.
/// Returns the full path to the `pnpm` executable.
pub fn ensure_pnpm() -> Result<PathBuf> {
    // 1) If pnpm is already installed, we’re done
    if let Ok(path) = which("pnpm") {
        return Ok(path);
    }

    // 2) Otherwise, prompt the user to install it via npm
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

    // 3) Re‐try locating `pnpm`
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
pub fn check_npm_and_install(workspace_parent: &Path) -> Result<(), Box<dyn Error>> {
    if workspace_parent.join("pnpm-workspace.yaml").exists() {
        // If this is a pnpm workspace, skip npm checks
        println!("Skipping npm checks for pnpm workspace.");
        return Ok(());
    }
    // Check if package.json exists at the workspace parent level
    println!(
        "Checking for package.json in: {}",
        workspace_parent.display()
    );
    if workspace_parent.join("package.json").exists() {
        println!("package.json found in: {}", workspace_parent.display());
        // Get the path to npm using `which`.
        match which("npm") {
            Ok(npm_path) => {
                println!("Found npm at: {}", npm_path.display());

                // Run `npm ls --depth=1` in the specified directory
                let output = Command::new(npm_path.clone())
                    .arg("ls")
                    .arg("--depth=1")
                    .current_dir(workspace_parent)
                    .output()
                    .map_err(|e| eprintln!("Failed to execute npm ls: {}", e))
                    .ok();

                if let Some(output) = output {
                    println!("npm ls output: {}", String::from_utf8_lossy(&output.stdout));
                    if !output.status.success() {
                        // Print the npm error output for debugging.
                        eprintln!(
                            "npm ls failed for directory: {}",
                            workspace_parent.display()
                        );
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));

                        // Run `npm install` to fix the missing dependencies
                        println!(
                            "Running npm install in directory: {}",
                            workspace_parent.display()
                        );
                        let install_output = Command::new(npm_path)
                            .arg("install")
                            .current_dir(workspace_parent)
                            .output()
                            .map_err(|e| eprintln!("Failed to execute npm install: {}", e))
                            .ok();

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
                                eprintln!("{}", String::from_utf8_lossy(&install_output.stderr));
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
pub fn check_pnpm_and_install(workspace_parent: &Path) -> Result<PathBuf> {
    // if this is a pnpm workspace, install deps
    let workspace_yaml = workspace_parent.join("pnpm-workspace.yaml");
    if workspace_yaml.exists() {
        // ensure pnpm is available (and install it if necessary)
        let pnpm = ensure_pnpm()?;
        println!(
            "Found pnpm-workspace.yaml in: {}",
            workspace_parent.display()
        );
        println!("Running `pnpm install`…");

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

        println!("✅ pnpm install succeeded");
        return Ok(pnpm);
    } else {
        println!(
            "No pnpm-workspace.yaml found in {}, skipping `pnpm install`.",
            workspace_parent.display()
        );
    }
    Ok(PathBuf::new())
}
