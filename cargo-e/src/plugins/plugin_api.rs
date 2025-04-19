use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};
use std::path::PathBuf;
use walkdir::WalkDir;
 #[cfg(feature = "uses_wasm")]
use crate::plugins::wasm_plugin::WasmPlugin;
use toml;
// Generic export plugin for Wasm/DLL exports (always available)
#[cfg(feature = "uses_wasm")]
use crate::plugins::wasm_export_plugin::WasmExportPlugin;
use std::fs;
use std::sync::Arc;
use crate::e_processmanager::ProcessManager;
use crate::Cli;
use crate::e_target::CargoTarget;
use std::process::ExitStatus;
 #[cfg(feature = "uses_rhai")]
use crate::plugins::rhai_plugin::RhaiPlugin;

/// Returns the directories to search for plugins in precedence order:
/// 1) development-time CARGO_MANIFEST_DIR/plugins
/// 2) project-local .cargo-e/plugins in the current working directory
fn plugin_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // 1. Development plugins from source tree (when running in the repo)
    let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins");
    if dev_dir.is_dir() {
        dirs.push(dev_dir);
    }
    // 2. Global user plugins in $HOME/.cargo-e/plugins
    #[cfg(unix)]
    if let Some(home) = std::env::var_os("HOME") {
        let global = PathBuf::from(home).join(".cargo-e").join("plugins");
        if global.is_dir() {
            dirs.push(global);
        }
    }
    #[cfg(windows)]
    if let Some(userprof) = std::env::var_os("USERPROFILE") {
        let global = PathBuf::from(userprof).join(".cargo-e").join("plugins");
        if global.is_dir() {
            dirs.push(global);
        }
    }
    // 3. Project-local hidden plugins in .cargo-e/plugins
    if let Ok(cwd) = std::env::current_dir() {
        let proj_dir = cwd.join(".cargo-e").join("plugins");
        if proj_dir.is_dir() {
            dirs.push(proj_dir);
        }
    }
    dirs
}

pub fn find_wasm_plugins() -> Vec<PathBuf> {
    let mut wasm_paths = Vec::new();
    // Search in each plugin directory
    for base in plugin_directories() {
        if !base.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                let path = e.path();
                // Only allow *.wasm files
                let is_wasm = path.extension().map_or(false, |ext| ext == "wasm");
                // also allow native dynamic libraries as plugins
                let is_dll = path.extension().map_or(false, |ext| ext == "dll");
                let is_wasm_or_dll = is_wasm || is_dll;
                // Skip anything inside a /deps/ directory
                let not_in_deps = !path
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy() == "deps");
                is_wasm_or_dll && not_in_deps
            })
        {
            wasm_paths.push(entry.into_path());
        }
    }
    wasm_paths
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub metadata: Option<String>,
}

pub trait Plugin {
    fn name(&self) -> &str;
    fn matches(&self, dir: &Path) -> bool;
    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>>;
    /// Build a system command to execute this target when no in-process entrypoint is provided.
    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command>;
    /// Build a system command to run this target, with interactive stdio inheritance.
    /// By default, this delegates to `build_command`.
    fn run_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        self.build_command(dir, target)
    }
    /// Indicates whether the plugin target should perform a build step before running.
    /// Defaults to true; plugin implementations may override to skip build.
    fn should_build(&self, _dir: &Path, _target: &Target) -> bool {
        true
    }
    /// Run the plugin target, either in-process or by spawning the external command.
    ///
    /// Returns a Vec of strings:
    /// - The first element is the exit code (as a debug-formatted string).
    /// - Subsequent elements are the debug-formatted output lines from stdout.
    fn run(&self, dir: &Path, target: &Target) -> Result<Vec<String>> {
        // Default: spawn the command returned by run_command and capture output.
        let mut cmd = self.run_command(dir, target)?;
        let output = cmd.output()?;
        let mut result = Vec::new();
        // Exit code, default to 0 if unavailable.
        let code = output.status.code().unwrap_or(0);
        // Push exit code as string (no quotes)
        result.push(code.to_string());
        // Capture stdout lines and push as-is
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            result.push(line.to_string());
        }
        Ok(result)
    }
    /// Optional human-readable source path of the plugin (e.g., .lua script, .wasm file, crate path)
    fn source(&self) -> Option<String> {
        None
    }
    /// Run the plugin target via the main ProcessManager and runner.
    /// Default implementation falls back to the standard example runner.
    fn run_with_manager(
        &self,
        manager: Arc<ProcessManager>,
        cli: &Cli,
        cargo_target: &CargoTarget,
    ) -> Result<Option<ExitStatus>> {
        crate::e_runner::run_example(manager, cli, cargo_target)
    }
}

pub fn load_plugins() -> Result<Vec<Box<dyn Plugin>>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();
    log::trace!("Initializing plugin loading; current dir = {:?}", std::env::current_dir()?);
    // current directory for matches
    let cwd = std::env::current_dir()?;

    // Load Lua and Rhai script plugins from project-local and built-in `plugins/` directories
    for base in plugin_directories() {
        log::trace!("Scanning plugin directory: {:?}", base);
        if !base.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&base)? {
            let path = entry?.path();
            log::trace!("Found plugin candidate: {:?}", path);
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                #[cfg(feature = "uses_lua")]
                if ext == "lua" {
                    log::trace!("Loading Lua plugin at {:?}", path);
                    let plugin = crate::plugins::lua_plugin::LuaPlugin::load(&path)?;
                    plugins.push(Box::new(plugin));
                }
                #[cfg(feature = "uses_rhai")]
                if ext == "rhai" {
                    log::trace!("Loading Rhai plugin at {:?}", path);
                    let plugin = crate::plugins::rhai_plugin::RhaiPlugin::load(&path)?;
                    plugins.push(Box::new(plugin));
                }
            }
        }
    }
    log::trace!("Loaded {} script plugins", plugins.len());

    // Recursively find all .wasm plugins in plugins/**/target/**/*.wasm
    #[cfg(feature = "uses_wasm")]
    for wasm_path in find_wasm_plugins() {
        println!("[debug] trying plugin: {}", wasm_path.display());
        // First, try the protocol-aware WasmPlugin
        if let Some(wp) = WasmPlugin::load(&wasm_path)? {
            if wp.matches(&cwd) {
                plugins.push(Box::new(wp));
                continue;
            }
        }
        // Fallback to generic export plugin for arbitrary exports
        if let Some(gp) = WasmExportPlugin::load(&wasm_path)? {
            plugins.push(Box::new(gp));
        }
    }
    Ok(plugins)
}


/// Internal structure matching the JSON command spec returned by plugins
#[derive(serde::Deserialize)]
pub struct CommandSpec {
    pub prog: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

impl CommandSpec {
    /// Convert the spec into a `std::process::Command`, defaulting to `default_dir` if `cwd` is None
    pub fn into_command(self, default_dir: &Path) -> Command {
        let mut cmd = Command::new(self.prog);
        for arg in self.args {
            cmd.arg(arg);
        }
        if let Some(cwd) = self.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(default_dir);
        }
        cmd
    }
}