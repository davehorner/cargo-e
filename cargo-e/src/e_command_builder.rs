use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Sender};
use std::time::SystemTime;
use which::which;

use anyhow::Context;
 use std::sync::{Arc, Mutex};
use crate::e_runner::GLOBAL_CHILDREN;
use crate::e_target::{CargoTarget, TargetKind, TargetOrigin};
use crate::e_cargocommand_ext::{CargoCommandExt, CargoProcessHandle};
use crate::e_eventdispatcher::{CallbackResponse, EventDispatcher};
use crate::e_cargocommand_ext::CargoProcessResult;

#[derive(Debug,Clone,PartialEq,Copy)]
pub enum TerminalError {
    NotConnected,
    NoTerminal,
    NoError,
}

impl Default for TerminalError {
    fn default() -> Self {
        TerminalError::NoError
    }
}

/// A builder that constructs a Cargo command for a given target.
#[derive(Clone)]
pub struct CargoCommandBuilder {
    pub args: Vec<String>,
    pub pid: Option<u32>,
    pub alternate_cmd: Option<String>,
    pub execution_dir: Option<PathBuf>,
    pub suppressed_flags: HashSet<String>,
    pub stdout_dispatcher: Option<Arc<EventDispatcher>>,
    pub stderr_dispatcher: Option<Arc<EventDispatcher>>,
    pub progress_dispatcher: Option<Arc<EventDispatcher>>,
    pub stage_dispatcher: Option<Arc<EventDispatcher>>,
    pub terminal_error_flag: Arc<Mutex<bool>>,
    pub sender: Option<Arc<Mutex<Sender<TerminalError>>>>, 
}
impl Default for CargoCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl CargoCommandBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        let (sender, receiver) = channel::<TerminalError>();
        let sender = Arc::new(Mutex::new(sender));
        let mut builder =CargoCommandBuilder {
            args: Vec::new(),
            pid: None,
            alternate_cmd: None,
            execution_dir: None,
            suppressed_flags: HashSet::new(),
                        stdout_dispatcher: None,
            stderr_dispatcher: None,
            progress_dispatcher: None,
            stage_dispatcher: None,
            terminal_error_flag: Arc::new(Mutex::new(false)),
            sender: Some(sender),
        };
        builder.set_default_dispatchers();

        builder
    }

        /// Lazily creates a default stdout dispatcher if not already set.
    fn get_stdout_dispatcher(&mut self) -> &mut Arc<EventDispatcher> {
        if self.stdout_dispatcher.is_none() {
            self.stdout_dispatcher = Some(Arc::new(EventDispatcher::new()));
        }
        self.stdout_dispatcher.as_mut().unwrap()
    }

    /// Lazily creates a default stderr dispatcher if not already set.
    fn get_stderr_dispatcher(&mut self) -> &mut Arc<EventDispatcher> {
        if self.stderr_dispatcher.is_none() {
            self.stderr_dispatcher = Some(Arc::new(EventDispatcher::new()));
        }
        self.stderr_dispatcher.as_mut().unwrap()
    }

    /// Lazily creates a default progress dispatcher if not already set.
    fn get_progress_dispatcher(&mut self) -> &mut Arc<EventDispatcher> {
        if self.progress_dispatcher.is_none() {
            self.progress_dispatcher = Some(Arc::new(EventDispatcher::new()));
        }
        self.progress_dispatcher.as_mut().unwrap()
    }

    /// Lazily creates a default stage dispatcher if not already set.
    fn get_stage_dispatcher(&mut self) -> &mut Arc<EventDispatcher> {
        if self.stage_dispatcher.is_none() {
            self.stage_dispatcher = Some(Arc::new(EventDispatcher::new()));
        }
        self.stage_dispatcher.as_mut().unwrap()
    }

    // /// Configures the command based on the provided CargoTarget.
    // pub fn with_target(mut self, target: &CargoTarget) -> Self {
    //     match target.kind {
    //         CargoTargetKind::Example => {
    //             self.args.push("run".into());
    //             self.args.push("--example".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Binary => {
    //             self.args.push("run".into());
    //             self.args.push("--bin".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Test => {
    //             self.args.push("test".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Manifest => {
    //             // For a manifest target, you might simply want to open or browse it.
    //             // Adjust the behavior as needed.
    //             self.args.push("manifest".into());
    //         }
    //     }

    //     // If the target is "extended", add a --manifest-path flag
    //     if target.extended {
    //         self.args.push("--manifest-path".into());
    //         self.args.push(target.manifest_path.clone());
    //     }

    //     // Optionally use the origin information if available.

    //     if let Some(TargetOrigin::SubProject(ref path)) = target.origin {
    //         self.args.push("--manifest-path".into());
    //         self.args.push(path.display().to_string());
    //     }

    //     self
    // }

        // Switch to passthrough mode when the terminal error is detected
    fn switch_to_passthrough_mode(&self) {
        println!("Switching to passthrough mode...");

        let mut command = self.build_command();

        // Now, spawn the cargo process in passthrough mode
        let cargo_process_handle = command.spawn_cargo_passthrough();

        let pid = cargo_process_handle.pid;

        println!("Passthrough mode activated for PID {}", pid);
    }

    // Set up the default dispatchers, which includes error detection
    fn set_default_dispatchers(&mut self) {
        let sender = self.sender.clone().unwrap();

        let mut stdout_dispatcher = EventDispatcher::new();
        stdout_dispatcher.add_callback(r"listening on", Box::new(|line, captures| {
            println!("(STDOUT) Dispatcher caught: {}", line);
            None
        })); 
        stdout_dispatcher.add_callback(r"BuildFinished", Box::new(|line, captures| {
            println!("(STDOUT) Dispatcher caught: {}", line);
            None
        }));
        self.stdout_dispatcher = Some(Arc::new(stdout_dispatcher));

        let mut stderr_dispatcher = EventDispatcher::new();
        stderr_dispatcher.add_callback(r"IO\(Custom \{ kind: NotConnected", Box::new(move |line, captures| {
            println!("(STDERR) Terminal error detected: {}", &line);
            let result = if line.contains("NotConnected") {
                TerminalError::NoTerminal
            } else {
                TerminalError::NoError
            };
            let sender = sender.lock().unwrap();
            sender.send(result).ok();
            Some(CallbackResponse{ number: 255, message: Some(line.to_string()) })
        }));
        self.stderr_dispatcher = Some(Arc::new(stderr_dispatcher));

        let mut progress_dispatcher = EventDispatcher::new();
        progress_dispatcher.add_callback(r"Progress", Box::new(|line, captures| {
            println!("(Progress) {}", line);
            None
        }));
        self.progress_dispatcher = Some(Arc::new(progress_dispatcher));

        let mut stage_dispatcher = EventDispatcher::new();
        stage_dispatcher.add_callback(r"Stage:", Box::new(|line, captures| {
            println!("(Stage) {}", line);
            None
        }));
        self.stage_dispatcher = Some(Arc::new(stage_dispatcher));
    }




       pub fn run<F>(self: Arc<Self>, on_spawn: F) -> anyhow::Result<u32>
where
    F: FnOnce(u32, CargoProcessHandle)

    {
        let mut command = self.build_command();

        let cargo_process_handle = command.spawn_cargo_capture(
            self.stdout_dispatcher.clone(),
            self.stderr_dispatcher.clone(),
            self.progress_dispatcher.clone(),
            self.stage_dispatcher.clone(),
            None,
        );

        let pid = cargo_process_handle.pid;

        // Notify observer
        on_spawn(pid,cargo_process_handle);

        Ok(pid)
    }

// pub fn run(self: Arc<Self>) -> anyhow::Result<u32> {
//     // Build the command using the builder's configuration
//     let mut command = self.build_command();

//     // Spawn the cargo process handle
//     let cargo_process_handle = command.spawn_cargo_capture(
//         self.stdout_dispatcher.clone(),
//         self.stderr_dispatcher.clone(),
//         self.progress_dispatcher.clone(),
//         self.stage_dispatcher.clone(),
//         None,
//     );
// let pid = cargo_process_handle.pid;
// let mut global = GLOBAL_CHILDREN.lock().unwrap();
// global.insert(pid, Arc::new(Mutex::new(cargo_process_handle)));
//     Ok(pid)
// }

pub fn wait(self: Arc<Self>, pid: Option<u32>) -> anyhow::Result<CargoProcessResult> {
    let mut global = GLOBAL_CHILDREN.lock().unwrap();
    if let Some(pid) = pid {

    // Lock the global list of processes and attempt to find the cargo process handle directly by pid
    if let Some(cargo_process_handle) = global.get_mut(&pid) {
        let mut cargo_process_handle = cargo_process_handle.lock().unwrap();
        
        // Wait for the process to finish and retrieve the result
        // println!("Waiting for process with PID: {}", pid);
        // let result = cargo_process_handle.wait();
        // println!("Process with PID {} finished", pid);
             loop {
                println!("Waiting for process with PID: {}", pid);
                
                // Attempt to wait for the process, but don't block indefinitely
                let status = cargo_process_handle.child.try_wait()?;

                // If the status is `Some(status)`, the process has finished
                if let Some(status) = status {

// Check the terminal error flag and update the result if there is an error
if *cargo_process_handle.terminal_error_flag.lock().unwrap() != TerminalError::NoError {
    let terminal_error = *cargo_process_handle.terminal_error_flag.lock().unwrap();
    cargo_process_handle.result.terminal_error = Some(terminal_error);
}

                    cargo_process_handle.result.exit_status = Some(status);
                    cargo_process_handle.result.end_time = Some( SystemTime::now() );
                    println!("Process with PID {} finished", pid);
                    return Ok(cargo_process_handle.result.clone());
                    // return Ok(CargoProcessResult { exit_status: status, ..Default::default() });
                }

                // Sleep briefly to yield control back to the system and avoid blocking
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

        // Return the result
        // match result {
        //     Ok(res) => Ok(res),
        //     Err(e) => Err(anyhow::anyhow!("Failed to wait for cargo process: {}", e).into()),
        // }
    } else {
        Err(anyhow::anyhow!("Process handle with PID {} not found in GLOBAL_CHILDREN", pid).into())
    }
    } else {
        Err(anyhow::anyhow!("No PID provided for waiting on cargo process").into())
    }
}

// pub fn run_wait(self: Arc<Self>) -> anyhow::Result<CargoProcessResult> {
//     // Run the cargo command and get the process handle (non-blocking)
//     let pid = self.clone().run()?; // adds to global list of processes
//     let result = self.wait(Some(pid)); // Wait for the process to finish
//     // Remove the completed process from GLOBAL_CHILDREN
//     let mut global = GLOBAL_CHILDREN.lock().unwrap();
//     global.remove(&pid);

//     result
// }

       /// Runs the cargo command using the builder's configuration.
    // pub fn run(&self) -> anyhow::Result<CargoProcessResult> {
    //     // Build the command using the builder's configuration
    //     let mut command = self.build_command();

    //     // Now use the `spawn_cargo_capture` extension to run the command
    //     let mut cargo_process_handle = command.spawn_cargo_capture(
    //         self.stdout_dispatcher.clone(),
    //         self.stderr_dispatcher.clone(),
    //         self.progress_dispatcher.clone(),
    //         self.stage_dispatcher.clone(),
    //         None,
    //     );

    //     // Wait for the process to finish and retrieve the results
    //     cargo_process_handle.wait().context("Failed to execute cargo process")
    // }

    /// Configure the command based on the target kind.
    pub fn with_target(mut self, target: &CargoTarget) -> Self {
        if let Some(origin) = target.origin.clone() {
            println!("Target origin: {:?}", origin);
        } else {
            println!("Target origin is not set");
        }
        match target.kind {
            TargetKind::Unknown => {
                return self;
            }
            TargetKind::Bench => {
                // To run benchmarks, use the "bench" command.
                self.alternate_cmd = Some("bench".to_string());
                self.args.push(target.name.clone());
            }
            TargetKind::Test => {
                self.args.push("test".into());
                // Pass the target's name as a filter to run specific tests.
                self.args.push(target.name.clone());
            }
            TargetKind::Example | TargetKind::ExtendedExample => {
                self.args.push("run".into());
                //self.args.push("--message-format=json".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::Binary | TargetKind::ExtendedBinary => {
                self.args.push("run".into());
                self.args.push("--bin".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::Manifest => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push("run".into());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ManifestTauriExample => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push("run".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ManifestTauri => {
                self.suppressed_flags.insert("quiet".to_string());
                // Helper closure to check for tauri.conf.json in a directory.
                let has_tauri_conf = |dir: &Path| -> bool { dir.join("tauri.conf.json").exists() };

                // Try candidate's parent (if origin is SingleFile or DefaultBinary).
                let candidate_dir_opt = match &target.origin {
                    Some(TargetOrigin::SingleFile(path))
                    | Some(TargetOrigin::DefaultBinary(path)) => path.parent(),
                    _ => None,
                };

                if let Some(candidate_dir) = candidate_dir_opt {
                    if has_tauri_conf(candidate_dir) {
                        println!("Using candidate directory: {}", candidate_dir.display());
                        self.execution_dir = Some(candidate_dir.to_path_buf());
                    } else if let Some(manifest_parent) = target.manifest_path.parent() {
                        if has_tauri_conf(manifest_parent) {
                            println!("Using manifest parent: {}", manifest_parent.display());
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        } else if let Some(grandparent) = manifest_parent.parent() {
                            if has_tauri_conf(grandparent) {
                                println!("Using manifest grandparent: {}", grandparent.display());
                                self.execution_dir = Some(grandparent.to_path_buf());
                            } else {
                                println!("No tauri.conf.json found in candidate, manifest parent, or grandparent; defaulting to manifest parent: {}", manifest_parent.display());
                                self.execution_dir = Some(manifest_parent.to_path_buf());
                            }
                        } else {
                            println!("No grandparent for manifest; defaulting to candidate directory: {}", candidate_dir.display());
                            self.execution_dir = Some(candidate_dir.to_path_buf());
                        }
                    } else {
                        println!(
                            "No manifest parent found for: {}",
                            target.manifest_path.display()
                        );
                    }
                } else if let Some(manifest_parent) = target.manifest_path.parent() {
                    if has_tauri_conf(manifest_parent) {
                        println!("Using manifest parent: {}", manifest_parent.display());
                        self.execution_dir = Some(manifest_parent.to_path_buf());
                    } else if let Some(grandparent) = manifest_parent.parent() {
                        if has_tauri_conf(grandparent) {
                            println!("Using manifest grandparent: {}", grandparent.display());
                            self.execution_dir = Some(grandparent.to_path_buf());
                        } else {
                            println!(
                                "No tauri.conf.json found; defaulting to manifest parent: {}",
                                manifest_parent.display()
                            );
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        }
                    }
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.args.push("tauri".into());
                self.args.push("dev".into());
            }
            TargetKind::ManifestLeptos => {
                let readme_path = target
                    .manifest_path
                    .parent()
                    .map(|p| p.join("README.md"))
                    .filter(|p| p.exists())
                    .or_else(|| {
                        target
                            .manifest_path
                            .parent()
                            .map(|p| p.join("readme.md"))
                            .filter(|p| p.exists())
                    });

                if let Some(readme) = readme_path {
                    if let Ok(mut file) = std::fs::File::open(&readme) {
                        let mut contents = String::new();
                        if file.read_to_string(&mut contents).is_ok()
                            && contents.contains("cargo leptos watch")
                        {
                            // Use cargo leptos watch
                            println!("Detected 'cargo leptos watch' in {}", readme.display());
                            self.execution_dir =
                                target.manifest_path.parent().map(|p| p.to_path_buf());
                            self.alternate_cmd = Some("cargo".to_string());
                            self.args.push("leptos".into());
                            self.args.push("watch".into());
                            self = self.with_required_features(&target.manifest_path, target);
                            return self;
                        }
                    }
                }

                // fallback to trunk
                let exe_path = match which("trunk") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'trunk' not found in PATH: {}", err);
                        return self;
                    }
                };

                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }

                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self.args.push("--open".into());
                self = self.with_required_features(&target.manifest_path, target);
            }
            TargetKind::ManifestDioxus => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self = self.with_required_features(&target.manifest_path, target);
            }
            TargetKind::ManifestDioxusExample => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self = self.with_required_features(&target.manifest_path, target);
            }
        }
        self
    }

    /// Configure the command using CLI options.
    pub fn with_cli(mut self, cli: &crate::Cli) -> Self {
        if cli.quiet && !self.suppressed_flags.contains("quiet") {
            // Insert --quiet right after "run" if present.
            if let Some(pos) = self.args.iter().position(|arg| arg == "run") {
                self.args.insert(pos + 1, "--quiet".into());
            } else {
                self.args.push("--quiet".into());
            }
        }
        if cli.release {
            // Insert --release right after the initial "run" command if applicable.
            // For example, if the command already contains "run", insert "--release" after it.
            if let Some(pos) = self.args.iter().position(|arg| arg == "run") {
                self.args.insert(pos + 1, "--release".into());
            } else {
                // If not running a "run" command (like in the Tauri case), simply push it.
                self.args.push("--release".into());
            }
        }
        // Append extra arguments (if any) after a "--" separator.
        if !cli.extra.is_empty() {
            self.args.push("--".into());
            self.args.extend(cli.extra.iter().cloned());
        }
        self
    }
    /// Append required features based on the manifest, target kind, and name.
    /// This method queries your manifest helper function and, if features are found,
    /// appends "--features" and the feature list.
    pub fn with_required_features(mut self, manifest: &PathBuf, target: &CargoTarget) -> Self {
        if let Some(features) = crate::e_manifest::get_required_features_from_manifest(
            manifest,
            &target.kind,
            &target.name,
        ) {
            self.args.push("--features".to_string());
            self.args.push(features);
        }
        self
    }

    /// Appends extra arguments to the command.
    pub fn with_extra_args(mut self, extra: &[String]) -> Self {
        if !extra.is_empty() {
            // Use "--" to separate Cargo arguments from target-specific arguments.
            self.args.push("--".into());
            self.args.extend(extra.iter().cloned());
        }
        self
    }

    /// Builds the final vector of command-line arguments.
    pub fn build(self) -> Vec<String> {
        self.args
    }

    /// Optionally, builds a std::process::Command.
    pub fn build_command(&self) -> Command {
        let mut is_cargo = false;
        let mut new_args = self.args.clone();
        let supported_subcommands = ["run", "build", "test", "bench", "clean", "doc", "publish", "update"];

        let mut cmd = if let Some(alternate) = &self.alternate_cmd {
            Command::new(alternate)
        } else {
            is_cargo = true;
            Command::new("cargo")
        };
        if is_cargo {
                if let Some(pos) = new_args.iter().position(|arg| supported_subcommands.contains(&arg.as_str())) {

        // If the command is "cargo run", insert the JSON output format and color options.
        new_args.insert(pos + 1, "--message-format=json".into());
        new_args.insert(pos + 2, "--color".into());
        new_args.insert(pos + 3, "always".into());
    }
        }
        cmd.args(new_args);
        if let Some(dir) = &self.execution_dir {
            cmd.current_dir(dir);
        }
        cmd
    }
}

// --- Example usage ---
#[cfg(test)]
mod tests {
    use crate::e_target::TargetOrigin;

    use super::*;

    #[test]
    fn test_command_builder_example() {
        let target = CargoTarget {
            name: "my_example".to_string(),
            display_name: "My Example".to_string(),
            manifest_path: "Cargo.toml".into(),
            kind: TargetKind::Example,
            extended: true,
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(PathBuf::from(
                "examples/my_example.rs",
            ))),
        };

        let extra_args = vec!["--flag".to_string(), "value".to_string()];

        let args = CargoCommandBuilder::new()
            .with_target(&target)
            .with_extra_args(&extra_args)
            .build();

        // For an example target, we expect something like:
        // cargo run --example my_example --manifest-path Cargo.toml -- --flag value
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--example".to_string()));
        assert!(args.contains(&"my_example".to_string()));
        assert!(args.contains(&"--manifest-path".to_string()));
        assert!(args.contains(&"Cargo.toml".to_string()));
        assert!(args.contains(&"--".to_string()));
        assert!(args.contains(&"--flag".to_string()));
        assert!(args.contains(&"value".to_string()));
    }
}
