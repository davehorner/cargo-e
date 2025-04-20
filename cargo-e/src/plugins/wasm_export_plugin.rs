use crate::plugins::plugin_api::{Plugin, Target};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use wasmparser::{ExternalKind, Parser, Payload};
use which::which;

/// A generic WASM export plugin: lists all exported functions in a .wasm file or a native dynamic library
pub struct WasmExportPlugin {
    path: PathBuf,
    wasmtime_path: PathBuf,
}

impl WasmExportPlugin {
    /// Load the generic export plugin if Wasmtime is available
    pub fn load(path: &Path) -> Result<Option<Self>> {
        let wasmtime_path =
            which("wasmtime").map_err(|e| anyhow::anyhow!("wasmtime not found in PATH: {}", e))?;
        Ok(Some(Self {
            path: path.to_path_buf(),
            wasmtime_path,
        }))
    }
}

impl Plugin for WasmExportPlugin {
    fn name(&self) -> &str {
        match self.path.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "dll" => "dll-export",
            _ => "wasm-export",
        }
    }

    fn matches(&self, _dir: &Path) -> bool {
        true
    }

    fn collect_targets(&self, _dir: &Path) -> Result<Vec<Target>> {
        if self.path.extension().and_then(|s| s.to_str()) == Some("dll") {
            let name = self
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("dll_export")
                .to_string();
            return Ok(vec![Target {
                name,
                metadata: None,
                cargo_target: None,
            }]);
        }
        let data = fs::read(&self.path)?;
        let mut targets = Vec::new();
        for payload in Parser::new(0).parse_all(&data) {
            let payload = payload?;
            if let Payload::ExportSection(section) = payload {
                for export in section {
                    let export = export?;
                    if export.kind == ExternalKind::Func {
                        targets.push(Target {
                            name: export.name.to_string(),
                            metadata: None,
                            cargo_target: None,
                        });
                    }
                }
            }
        }
        Ok(targets)
    }

    fn build_command(&self, _dir: &Path, target: &Target) -> Result<Command> {
        let mut cmd = Command::new(&self.wasmtime_path);
        cmd.arg(&self.path).arg("--invoke").arg(&target.name);
        Ok(cmd)
    }

    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
}
