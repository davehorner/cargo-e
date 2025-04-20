#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! cargo-e = { path = ".." }
//! eframe = "0.31.1"
//! egui = "0.31.1"
//! clap = { version = "4.5", features = ["derive"] }
//! ```
//!
//! cargo-e ExtContext GUI Demo
//!
//! Shows all targets (built-in + plugin) using `ExtContext`.
//!
//! Plugin targets are prefixed by their plugin file name.

use cargo_e::ext::ExtContext;
use cargo_e::e_processmanager::ProcessManager;
use cargo_e::e_target::{TargetKind, TargetOrigin};
use cargo_e::Cli;
use clap::Parser;
use eframe::egui;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI and set up context
    let cli = Cli::parse();
    // Initialize ProcessManager (returns Arc<ProcessManager>)
    let manager = ProcessManager::new(&cli);
    let ctx = ExtContext::new(cli, manager)?;
    let mut targets = ctx.collect_targets()?;

    // Prefix plugin targets with their plugin file name
    for t in &mut targets {
        if t.kind == TargetKind::Plugin {
            if let Some(TargetOrigin::Plugin { plugin_path, .. }) = &t.origin {
                if let Some(fname) = plugin_path.file_name().and_then(|s| s.to_str()) {
                    t.display_name = format!("{}: {}", fname, t.display_name);
                }
            }
        }
    }

    // Launch the GUI
    let app = TargetGuiApp::new(targets);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("cargo-e ExtContext GUI", native_options, Box::new(|_cc| Ok(Box::new(app))))?;
    Ok(())
}

struct TargetGuiApp {
    targets: Vec<cargo_e::e_target::CargoTarget>,
}

impl TargetGuiApp {
    fn new(targets: Vec<cargo_e::e_target::CargoTarget>) -> Self {
        Self { targets }
    }
}

impl eframe::App for TargetGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("cargo-e ExtContext GUI Demo");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for t in &self.targets {
                    let kind_label = match t.kind {
                        TargetKind::Example => "Example",
                        TargetKind::Binary => "Binary",
                        TargetKind::Test => "Test",
                        TargetKind::Bench => "Bench",
                        TargetKind::Plugin => "Plugin",
                        _ => "Other",
                    };
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}] {}", kind_label, t.display_name));
                    });
                    ui.label(format!("📁 {}", t.manifest_path.display()));
                    ui.separator();
                }
            });
        });
    }
}