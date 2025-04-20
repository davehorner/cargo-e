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
use rhai::{Array, Engine, Scope, AST};
// Import cargo-e library for target resolution
// Reference the internal crate modules rather than the external crate name
// Import target collection from the main library crate
use crate::e_collect::collect_all_targets_silent;
use crate::e_processmanager::ProcessManager;
use crate::e_target::CargoTarget;
use crate::plugins::plugin_api::{CommandSpec, Plugin, Target};
use crate::Cli;
use serde_json;
use std::sync::Arc;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// A Rhai-based plugin implementation for the `Plugin` trait.
pub struct RhaiPlugin {
    name: String,
    engine: Engine,
    ast: AST,
    path: PathBuf,
    /// CLI context for running real example targets
    cli: crate::Cli,
    /// Process manager for example execution
    manager: Arc<ProcessManager>,
}

impl RhaiPlugin {
    /// Load the Rhai script plugin from the given path.
    /// Load the Rhai script plugin from the given path, with full CLI and ProcessManager context.
    pub fn load(path: &Path, cli: &Cli, manager: Arc<ProcessManager>) -> Result<Self> {
        log::trace!("RhaiPlugin::load: reading script from {:?}", path);
        let code = fs::read_to_string(path)?;
        log::trace!("RhaiPlugin::load: script length {} bytes", code.len());
        // Create a Rhai engine and register built-in cargo-e functions
        let mut engine = Engine::new();
        // Built-in to collect Cargo targets via cargo-e library, returning JSON of Targets
        fn cargo_e_collect_json() -> String {
            // Determine parallelism for target collection
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            // Collect all Cargo targets silently (workspace enabled)
            let targets: Vec<CargoTarget> =
                collect_all_targets_silent(true, threads).unwrap_or_default();
            // Convert to plugin_api::Target and serialize to JSON
            let plugin_targets: Vec<Target> = targets
                .into_iter()
                .map(|t| Target {
                    name: t.display_name,
                    metadata: Some(t.manifest_path.to_string_lossy().to_string()),
                    cargo_target: None,
                })
                .collect();
            serde_json::to_string(&plugin_targets).unwrap_or_default()
        }
        engine.register_fn("cargo_e_collect", cargo_e_collect_json);
        // Expose `run_example(target_name)` to Rhai scripts: returns exit code (i64)
        {
            let mgr = manager.clone();
            let cli_clone = cli.clone();
            engine.register_fn("run_example", move |target_name: String| -> i64 {
                // Collect Cargo targets (silent)
                let threads = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                let targets =
                    crate::e_collect::collect_all_targets_silent(cli_clone.workspace, threads)
                        .unwrap_or_default();
                if let Some(ct) = targets.into_iter().find(|t| t.name == target_name) {
                    match crate::e_runner::run_example(mgr.clone(), &cli_clone, &ct) {
                        Ok(Some(status)) => status.code().unwrap_or(-1).into(),
                        Ok(None) => 0,
                        Err(_) => -1,
                    }
                } else {
                    -1
                }
            });
        }
        log::trace!("RhaiPlugin::load: compiling AST");
        let ast = engine.compile(&code)?;
        log::trace!("RhaiPlugin::load: compiled AST successfully");
        // Retrieve the plugin name by calling the `name()` function in the script
        let mut scope = Scope::new();
        log::trace!("RhaiPlugin::load: invoking name() in script");
        let name: String = engine
            .call_fn(&mut scope, &ast, "name", ())
            .map_err(|e| anyhow!("Rhai error calling name: {:?}", e))?;
        log::trace!("RhaiPlugin::load: plugin reports name = {}", name);
        Ok(RhaiPlugin {
            name,
            engine,
            ast,
            path: path.to_path_buf(),
            cli: cli.clone(),
            manager,
        })
    }
}

impl Plugin for RhaiPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        let dir_str = dir.to_string_lossy().to_string();
        log::trace!(
            "RhaiPlugin '{}' checking matches on dir {:?}",
            self.name,
            dir
        );
        let mut scope = Scope::new();
        let result = self
            .engine
            .call_fn::<bool>(&mut scope, &self.ast, "matches", (dir_str,))
            .unwrap_or(false);
        log::trace!("RhaiPlugin '{}' matches returned {}", self.name, result);
        result
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        let dir_str = dir.to_string_lossy().to_string();
        log::trace!(
            "RhaiPlugin '{}' collecting targets on dir {:?}",
            self.name,
            dir
        );
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(&mut scope, &self.ast, "collect_targets", (dir_str,))
            .map_err(|e| anyhow!("Rhai error calling collect_targets: {:?}", e))?;
        let targets: Vec<Target> = serde_json::from_str(&json)?;
        log::trace!(
            "RhaiPlugin '{}' collect_targets returned {} targets",
            self.name,
            targets.len()
        );
        Ok(targets)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        let dir_str = dir.to_string_lossy().to_string();
        let target_str = target.name.clone();
        log::trace!(
            "RhaiPlugin '{}' building command for target {}",
            self.name,
            target.name
        );
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(
                &mut scope,
                &self.ast,
                "build_command",
                (dir_str, target_str),
            )
            .map_err(|e| anyhow!("Rhai error calling build_command: {:?}", e))?;
        let spec: CommandSpec = serde_json::from_str(&json)
            .map_err(|e| anyhow!("Invalid JSON from Rhai: {:?}\nOriginal: {}", e, json))?;
        log::trace!(
            "RhaiPlugin '{}' build_command JSON spec: {}",
            self.name,
            json
        );
        Ok(spec.into_command(dir))
    }

    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
    /// Override in-process plugin run: call script-defined `run`, or fallback to Cargo-e runner.
    fn run(&self, dir: &Path, target: &Target) -> Result<Vec<String>> {
        // 1. Try a per-target function matching the target name in the script
        let mut scope = Scope::new();
        let dir_str = dir.to_string_lossy().to_string();
        let tgt_str = target.name.clone();
        if let Ok(arr) = self.engine.call_fn::<Array>(
            &mut scope,
            &self.ast,
            &target.name,
            (dir_str.clone(), tgt_str.clone()),
        ) {
            let mut result = Vec::new();
            if !arr.is_empty() {
                result.push(arr[0].to_string());
                for v in arr.iter().skip(1) {
                    result.push(v.to_string());
                }
            }
            return Ok(result);
        }
        // 2. Try a generic `run(dir, target)` function if defined in the script
        if let Ok(arr) = self.engine.call_fn::<Array>(
            &mut scope,
            &self.ast,
            "run",
            (dir_str.clone(), tgt_str.clone()),
        ) {
            let mut result = Vec::new();
            if !arr.is_empty() {
                result.push(arr[0].to_string());
                for v in arr.iter().skip(1) {
                    result.push(v.to_string());
                }
            }
            return Ok(result);
        }
        // 3. Fallback: run the external command as specified by build_command
        let spec_json = self
            .engine
            .call_fn::<String>(
                &mut scope,
                &self.ast,
                "build_command",
                (dir_str.clone(), tgt_str.clone()),
            )
            .map_err(|e| anyhow!("Rhai error calling build_command: {:?}", e))?;
        let spec: CommandSpec = serde_json::from_str(&spec_json).map_err(|e| {
            anyhow!(
                "Invalid JSON from Rhai build_command: {:?}\nJSON: {}",
                e,
                spec_json
            )
        })?;
        let mut cmd = spec.into_command(dir);
        // Execute and capture output
        let output = cmd.output()?;
        let mut result = Vec::new();
        let code = output.status.code().unwrap_or(0);
        result.push(code.to_string());
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            result.push(line.to_string());
        }
        Ok(result)
    }
}
