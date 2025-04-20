use crate::e_processmanager::ProcessManager;
use crate::plugins::plugin_api::CommandSpec;
use crate::plugins::plugin_api::{Plugin, Target};
use crate::Cli;
use anyhow::Result;
use mlua::{Function, Lua, Table};
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use std::{path::Path, process::Command};

/// A Lua-based plugin implementation for the `Plugin` trait.
#[allow(dead_code)]
pub struct LuaPlugin {
    name: String,
    lua: &'static Lua,
    tbl: Table,
    path: PathBuf,
}

impl LuaPlugin {
    /// Load the Lua script plugin from the given path, with full CLI and ProcessManager context.
    pub fn load(path: &Path, _cli: &Cli, _manager: Arc<ProcessManager>) -> Result<Self> {
        let code = std::fs::read_to_string(path)?;

        // Create a Lua context and leak it for 'static lifetime
        let lua: &'static Lua = Box::leak(Box::new(Lua::new()));

        // Evaluate the Lua code, expecting it to return a table
        let plugin_tbl: Table = lua
            .load(&code)
            .eval()
            .map_err(|e| anyhow::anyhow!("Lua eval error in plugin {}: {:?}", path.display(), e))?;

        // Debug: print keys returned in the plugin table
        for pair in plugin_tbl.clone().pairs::<String, mlua::Value>() {
            if let Ok((k, _)) = pair {
                println!("[debug] Lua key: {}", k);
            }
        }

        // Extract the plugin name from the table
        let name_val: mlua::Value = plugin_tbl
            .get("name")
            .map_err(|e| anyhow::anyhow!("Lua error getting 'name': {:?}", e))?;
        let name = match name_val {
            mlua::Value::String(s) => s
                .to_str()
                .map_err(|e| anyhow::anyhow!("Lua error converting name to str: {:?}", e))?
                .to_owned(),
            mlua::Value::Function(f) => f
                .call::<String>(())
                .map_err(|e| anyhow::anyhow!("Lua error calling name function: {:?}", e))?,
            _ => anyhow::bail!("Expected 'name' to be string or function"),
        };

        let tbl: Table = plugin_tbl;

        Ok(LuaPlugin {
            name,
            lua,
            tbl,
            path: path.to_path_buf(),
        })
    }
}

impl Plugin for LuaPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        // Call the Lua `matches(dir)` function
        let f: Function = match self.tbl.get("matches") {
            Ok(func) => func,
            Err(_) => return false,
        };
        f.call(dir.to_string_lossy().as_ref()).unwrap_or(false)
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        // Call the Lua `collect_targets(dir)` function, which returns JSON
        let f: Function = self.tbl.get("collect_targets").map_err(|e| {
            anyhow::anyhow!("Lua error getting 'collect_targets' function: {:?}", e)
        })?;
        let json: String = f
            .call(dir.to_string_lossy().as_ref())
            .map_err(|e| anyhow::anyhow!("Lua error calling collect_targets: {:?}", e))?;
        let v: Vec<Target> = serde_json::from_str(&json)?;
        Ok(v)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        // Call the Lua `build_command(dir, target_name)` function, which returns JSON
        let f: Function = self
            .tbl
            .get("build_command")
            .map_err(|e| anyhow::anyhow!("Lua error getting 'build_command' function: {:?}", e))?;
        let b = dir.to_string_lossy();
        let dir_str = b.as_ref();
        let target_str = target.name.as_str();
        // let json: String = f.call((dir_str, target_str))?;
        // let spec: CommandSpec = serde_json::from_str(&json)?;
        let json: String = f
            .call((dir_str, target_str))
            .map_err(|e| anyhow::anyhow!("Lua error calling build_command: {:?}", e))?;

        let spec: CommandSpec = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Invalid JSON from Lua: {:?}\nOriginal: {}", e, json))?;
        Ok(spec.into_command(dir))
    }
    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }

    /// Override in-process plugin run: call script-defined `run`, or fallback to external command.
    fn run(&self, dir: &Path, target: &Target) -> Result<Vec<String>> {
        let dir_str = dir.to_string_lossy().to_string();
        let tgt_str = target.name.clone();
        // 1. Try a per-target function matching the target name in the script
        if let Some(func) = self
            .tbl
            .get::<Option<Function>>(target.name.as_str())
            .map_err(|e| anyhow::anyhow!("Lua error getting '{}' function: {:?}", target.name, e))?
        {
            let table: Table = func.call((dir_str.clone(), tgt_str.clone())).map_err(|e| {
                anyhow::anyhow!(
                    "Lua error calling target '{}' function: {:?}",
                    target.name,
                    e
                )
            })?;
            let mut result = Vec::new();
            for entry in table.sequence_values::<mlua::Value>() {
                let val = entry.map_err(|e| anyhow::anyhow!("Lua error parsing table: {:?}", e))?;
                let s = match val {
                    mlua::Value::String(s) => s
                        .to_str()
                        .map_err(|e| anyhow::anyhow!("Lua error converting string entry: {:?}", e))?
                        .to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    other => format!("{:?}", other),
                };
                result.push(s);
            }
            return Ok(result);
        }
        // 2. Try a generic `run(dir, target)` function if defined in the script
        if let Some(func) = self
            .tbl
            .get::<Option<Function>>("run")
            .map_err(|e| anyhow::anyhow!("Lua error getting 'run' function: {:?}", e))?
        {
            let table: Table = func
                .call((dir_str.clone(), tgt_str.clone()))
                .map_err(|e| anyhow::anyhow!("Lua error calling 'run' function: {:?}", e))?;
            let mut result = Vec::new();
            for entry in table.sequence_values::<mlua::Value>() {
                let val = entry.map_err(|e| anyhow::anyhow!("Lua error parsing table: {:?}", e))?;
                let s = match val {
                    mlua::Value::String(s) => s
                        .to_str()
                        .map_err(|e| anyhow::anyhow!("Lua error converting string entry: {:?}", e))?
                        .to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    other => format!("{:?}", other),
                };
                result.push(s);
            }
            return Ok(result);
        }
        // 3. Fallback: run the external command as specified by build_command
        let mut cmd = self.build_command(dir, target)?;
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
