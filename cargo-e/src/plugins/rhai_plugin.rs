// RhaiPlugin: Load and execute Rhai script files as plugins.
//
// Required script functions:
//   fn name() -> String
//   fn matches(dir: String) -> bool
//   fn collect_targets(dir: String) -> String  // JSON-encoded Targets array
//   fn build_command(dir: String, target: String) -> String  // JSON CommandSpec
// Optional in-process execution:
//   fn <target>(dir: String, target: String) -> Array  // [exit_code, output...]
//   fn run(dir: String, target: String) -> Array  // fallback if no per-target fn
//
// Execution priority:
// 1. per-target function `fn <target>(dir, target)` if defined.
// 2. generic `fn run(dir, target)` if defined.
// 3. fall back to external spawning of the JSON command spec.
use anyhow::{anyhow, Result};
use rhai::{Engine, AST, Scope, Array};
// Import cargo-e library for target resolution
// Reference the internal crate modules rather than the external crate name
// Import target collection from the main library crate
use crate::e_collect::collect_all_targets_silent;
use crate::e_target::CargoTarget;
use serde_json;
use std::{fs, path::{Path, PathBuf}, process::Command};
use crate::plugins::plugin_api::{Plugin, Target, CommandSpec};

/// A Rhai-based plugin implementation for the `Plugin` trait.
pub struct RhaiPlugin {
    name: String,
    engine: Engine,
    ast: AST,
    path: PathBuf,
}

impl RhaiPlugin {
    /// Load the Rhai script plugin from the given path.
    pub fn load(path: &Path) -> Result<Self> {
        let code = fs::read_to_string(path)?;
        // Create a Rhai engine and register built-in cargo-e functions
        let mut engine = Engine::new();
        // Built-in to collect Cargo targets via cargo-e library, returning JSON of Targets
        fn cargo_e_collect_json() -> String {
            // Determine parallelism for target collection
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            // Collect all Cargo targets silently (workspace enabled)
            let targets: Vec<CargoTarget> = collect_all_targets_silent(true, threads)
                .unwrap_or_default();
            // Convert to plugin_api::Target and serialize to JSON
            let plugin_targets: Vec<Target> = targets
                .into_iter()
                .map(|t| 
                    Target {
                    name: t.display_name,
                    metadata: Some(t.manifest_path.to_string_lossy().to_string()),
                })
                .collect();
            serde_json::to_string(&plugin_targets).unwrap_or_default()
        }
        engine.register_fn("cargo_e_collect", cargo_e_collect_json);
        let ast = engine.compile(&code)?;
        // Retrieve the plugin name by calling the `name()` function in the script
        let mut scope = Scope::new();
        let name: String = engine
            .call_fn(&mut scope, &ast, "name", ())
            .map_err(|e| anyhow!("Rhai error calling name: {:?}", e))?;
        Ok(Self { name, engine, ast, path: path.to_path_buf() })
    }
}

impl Plugin for RhaiPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        let dir_str = dir.to_string_lossy().to_string();
        let mut scope = Scope::new();
        self.engine
            .call_fn::<bool>(&mut scope, &self.ast, "matches", (dir_str,))
            .unwrap_or(false)
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        let dir_str = dir.to_string_lossy().to_string();
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(&mut scope, &self.ast, "collect_targets", (dir_str,))
            .map_err(|e| anyhow!("Rhai error calling collect_targets: {:?}", e))?;
        let targets: Vec<Target> = serde_json::from_str(&json)?;
        Ok(targets)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        let dir_str = dir.to_string_lossy().to_string();
        let target_str = target.name.clone();
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(&mut scope, &self.ast, "build_command", (dir_str, target_str))
            .map_err(|e| anyhow!("Rhai error calling build_command: {:?}", e))?;
        let spec: CommandSpec = serde_json::from_str(&json)
            .map_err(|e| anyhow!("Invalid JSON from Rhai: {:?}\nOriginal: {}", e, json))?;
        Ok(spec.into_command(dir))
    }

    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
    /// Run a Rhai plugin target in-process via the embedded engine.
    /// Looks for a function named after the target; falls back to `run(dir, target)`.
    fn run(&self, dir: &std::path::Path, target: &Target) -> Result<Vec<String>> {
        let dir_str = dir.to_string_lossy().to_string();
        let mut scope = Scope::new();
        // Attempt in-process execution via Rhai: try per-target fn, then generic `run`, else fallback to external.
        let arr = match self.engine.call_fn::<Array>(
            &mut scope,
            &self.ast,
            &target.name,
            (dir_str.clone(), target.name.clone()),
        ) {
            Ok(v) => v,
            Err(_) => {
                // Try generic `run(dir, target)`
                match self.engine.call_fn::<Array>(
                    &mut scope,
                    &self.ast,
                    "run",
                    (dir_str.clone(), target.name.clone()),
                ) {
                    Ok(v2) => v2,
                    Err(_) => {
                        // No in-process entrypoint: fallback to external command
                        let mut cmd = self.build_command(dir, target)?;
                        let output = cmd.output()?;
                        let mut result = Vec::new();
                        let code = output.status.code().unwrap_or(0);
                        // push exit code as string
                        result.push(code.to_string());
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        for line in stdout.lines() {
                            // push each line as-is
                            result.push(line.to_string());
                        }
                        return Ok(result);
                    }
                }
            }
        };
        // Convert to Vec<String> preserving raw values (no quotes)
        let mut result = Vec::new();
        for value in arr {
            let s = if let Some(i) = value.clone().try_cast::<i64>() {
                // integer
                i.to_string()
            } else if let Some(f) = value.clone().try_cast::<f64>() {
                // float
                f.to_string()
            } else if let Some(st) = value.clone().try_cast::<String>() {
                // string: push as-is without quotes
                st
            } else {
                // fallback to generic string conversion
                value.to_string()
            };
            result.push(s);
        }
        Ok(result)
    }
}