//! High-level API for embedding cargo-e: unified target collection (builtins + plugins)
//!
//! Provides `ExtContext` which holds the parsed CLI and process manager,
//! and a `collect_targets` method to enumerate both built-in and plugin targets.
// Core types always available
use crate::{
    e_collect::collect_all_targets, e_processmanager::ProcessManager, e_target::CargoTarget, Cli,
};
// Plugin API loading and target collection (only when plugins are enabled)
#[cfg(feature = "uses_plugins")]
use crate::plugins::plugin_api::load_plugins;
use anyhow::{anyhow, Result};
#[cfg(feature = "uses_plugins")]
use once_cell::sync::OnceCell;
use std::{collections::HashSet, path::PathBuf, sync::Arc};

/// Embedding context for cargo-e: CLI, process manager, and discovery directory.
pub struct ExtContext {
    /// Parsed CLI options (same flags as the `cargo-e` binary)
    pub cli: Cli,
    /// Shared process manager for plugin in-process execution
    pub manager: Arc<ProcessManager>,
    /// Working directory used for scanning plugins
    pub cwd: PathBuf,
    /// Lazily-loaded plugin instances
    #[cfg(feature = "uses_plugins")]
    plugins: OnceCell<Vec<Box<dyn crate::plugins::plugin_api::Plugin>>>,
}

impl ExtContext {
    /// Create a new embedding context with a CLI and ProcessManager.
    /// Does not perform any target collection yet.
    /// Create a new ExtContext; plugin loading is deferred until first use.
    pub fn new(cli: Cli, manager: Arc<ProcessManager>) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        Ok(ExtContext {
            cli,
            manager,
            cwd,
            #[cfg(feature = "uses_plugins")]
            plugins: OnceCell::new(),
        })
    }

    /// Collect all targets: built-in examples/binaries/tests/benches and plugin targets.
    /// Deduplicates by (name, kind, extended).
    pub fn collect_targets(&self) -> Result<Vec<CargoTarget>> {
        // 1. Built-in targets
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let mut all = collect_all_targets(
            self.cli.manifest_path.clone(),
            self.cli.workspace,
            threads,
            self.cli.json_all_targets,
        )
        .map_err(|e| anyhow!("collect_all_targets failed: {}", e))?;
        // 2. Plugin targets (load plugins only once)
        #[cfg(feature = "uses_plugins")]
        {
            use crate::e_target::TargetKind;
            use crate::e_target::TargetOrigin;
            let plugins = self
                .plugins
                .get_or_try_init(|| load_plugins(&self.cli, self.manager.clone()))?;
            for plugin in plugins.iter() {
                if plugin.matches(&self.cwd) {
                    let plugin_path = plugin
                        .source()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| self.cwd.clone());
                    for pt in plugin.collect_targets(&self.cwd)? {
                        let ct = if let Some(ct) = pt.cargo_target {
                            ct
                        } else {
                            let reported = pt
                                .metadata
                                .as_ref()
                                .map(PathBuf::from)
                                .unwrap_or_else(|| self.cwd.clone());
                            CargoTarget {
                                name: pt.name.clone(),
                                display_name: pt.name.clone(),
                                manifest_path: self.cwd.clone(),
                                kind: TargetKind::Plugin,
                                extended: false,
                                toml_specified: false,
                                origin: Some(TargetOrigin::Plugin {
                                    plugin_path: plugin_path.clone(),
                                    reported,
                                }),
                            }
                        };
                        all.push(ct);
                    }
                }
            }
        }
        // 3. Deduplicate
        let mut seen = HashSet::new();
        all.retain(|t| seen.insert((t.name.clone(), t.kind, t.extended)));
        Ok(all)
    }
    /// Run a target (example, binary, or plugin-provided) using the shared ProcessManager.
    /// Returns Ok(Some(status)) for built-in runs, or Ok(None) if no action was taken.
    pub fn run_target(&self, target: &CargoTarget) -> Result<Option<std::process::ExitStatus>> {
        // Plugin targets
        #[cfg(feature = "uses_plugins")]
        {
            #[allow(unused_imports)]
            use crate::e_target::{TargetKind, TargetOrigin};
            if target.kind == TargetKind::Plugin {
                // Find matching plugin (cached)
                let plugins = self
                    .plugins
                    .get_or_try_init(|| load_plugins(&self.cli, self.manager.clone()))?;
                for plugin in plugins.iter() {
                    if let Some(crate::e_target::TargetOrigin::Plugin { plugin_path, .. }) =
                        &target.origin
                    {
                        if plugin.source().map(PathBuf::from) == Some(plugin_path.clone()) {
                            // Convert to plugin_api::Target
                            let _plugin_target =
                                crate::plugins::plugin_api::Target::from(target.clone());
                            // Run in-process via plugin
                            plugin.run_with_manager(self.manager.clone(), &self.cli, target)?;
                            return Ok(None);
                        }
                    }
                }
                return Ok(None);
            }
        }
        // Built-in via e_runner
        crate::e_runner::run_example(self.manager.clone(), &self.cli, target)
    }
}
