use std::{path::PathBuf, sync::Arc};
use anyhow::Result;
use cargo_e::plugins::rhai_plugin::RhaiPlugin;
use cargo_e::plugins::plugin_api::Plugin;
use cargo_e::Cli;
use clap::Parser;
use cargo_e::e_processmanager::ProcessManager;

#[test]
fn test_example_rhai_plugin() -> Result<()> {
    // Path to the example.rhai plugin script in the development plugins directory.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let plugin_path = PathBuf::from(manifest_dir).join("plugins").join("example.rhai");

    // Current working directory for plugin operations.
    let cwd = std::env::current_dir()?;

    // Initialize CLI context and process manager.
    let cli = Cli::parse_from(&["cargo-e"]);
    // Create the process manager; ProcessManager::new already returns an Arc.
    let manager = ProcessManager::new(&cli);

    // Load the Rhai plugin.
    // Load the Rhai plugin, passing the Arc-managed ProcessManager.
    let plugin = RhaiPlugin::load(&plugin_path, &cli, manager)?;

    // Verify plugin identification and applicability.
    assert_eq!(plugin.name(), "cargo_e_collect");
    assert!(plugin.matches(&cwd));

    // Collect targets and locate the `funny_example`.
    let targets = plugin.collect_targets(&cwd)?;
    let target = targets
        .iter()
        .find(|t| t.name == "funny_example")
        .expect("`funny_example` target not found by plugin");

    // Execute the plugin for the target, which should call the script's `run` and echo a message.
    let output = plugin.run(&cwd, target)?;
    // Expect only the exit code "0"; the real example output is printed directly by the runner.
    assert_eq!(output, vec!["0".to_string()]);

    Ok(())
}