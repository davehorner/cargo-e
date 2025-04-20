#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! cargo-e = { path = ".." }
//! eframe = "0.31.1"
//! egui = "0.31.1"
//! clap = { version = "4.5", features = ["derive"] }
//! ```
//!
//! cargo-e ExtContext Run GUI Demo
//!
//! Shows all targets (built-in + plugin) using `ExtContext`,
//! with a Run button for each, using the new `run_target` interface.

use clap::Parser;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

use cargo_e::ext::ExtContext;
use cargo_e::e_processmanager::ProcessManager;
use cargo_e::e_target::{CargoTarget, TargetKind};
use cargo_e::Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI and initialize process manager and context
    // Parse CLI and initialize ProcessManager
    let cli = Cli::parse();
    let manager = ProcessManager::new(&cli);
    // Collect all targets via an ExtContext
    let ctx = ExtContext::new(cli.clone(), manager.clone())?;
    let targets = ctx.collect_targets()?;

    // Launch the GUI application (holding cli and manager for run)
    let app = TargetRunGuiApp::new(cli, manager, targets);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "cargo-e ExtContext Run GUI",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )?;
    Ok(())
}

struct TargetRunGuiApp {
    cli: Cli,
    manager: Arc<ProcessManager>,
    targets: Vec<CargoTarget>,
    run_result: Arc<Mutex<Option<String>>>,
}

impl TargetRunGuiApp {
    fn new(cli: Cli, manager: Arc<ProcessManager>, targets: Vec<CargoTarget>) -> Self {
        Self {
            cli,
            manager,
            targets,
            run_result: Arc::new(Mutex::new(None)),
        }
    }
}

impl eframe::App for TargetRunGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🚀 cargo-e Run Target GUI Demo");
            ui.separator();

            // Scrollable list of targets
            egui::ScrollArea::vertical().show(ui, |ui| {
                for target in &self.targets {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("[{:?}] {}", target.kind, target.display_name));
                            // Run button for all target kinds
                            if ui.button("▶ Run").clicked() {
                                    // Clone parameters for thread
                                    let cli = self.cli.clone();
                                    let manager = self.manager.clone();
                                    let target = target.clone();
                                    let run_result = self.run_result.clone();
                                    // Indicate running
                                    {
                                        let mut lock = run_result.lock().unwrap();
                                        *lock = Some(format!("Running {}...", target.display_name));
                                    }
                                    // Spawn thread to run target with a fresh ExtContext
                                    thread::spawn(move || {
                                        let ctx = ExtContext::new(cli, manager).unwrap();
                                        let res = ctx.run_target(&target);
                                        let mut lock = run_result.lock().unwrap();
                                        *lock = Some(match res {
                                            Ok(Some(status)) => format!("Exited with: {}", status),
                                            Ok(None) => format!("Run completed for {}", target.display_name),
                                            Err(e) => format!("Error running {}: {}", target.name, e),
                                        });
                                    });
                                }
                        });
                        ui.label(format!("📁 {}", target.manifest_path.display()));
                    });
                    ui.add_space(4.0);
                }
            });

            // Display run result status
            if let Some(msg) = &*self.run_result.lock().unwrap() {
                ui.separator();
                ui.label(msg);
            }
        });
    }
}