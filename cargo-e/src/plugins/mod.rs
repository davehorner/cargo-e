//! Aggregates all plugin implementations, gated by feature flags
// Core plugin API (needed if any plugin feature is enabled)
#[cfg(feature = "uses_plugins")]
pub mod plugin_api;

// Lua-based plugin
#[cfg(all(feature = "uses_plugins", feature = "uses_lua"))]
pub mod lua_plugin;

// Rhai-based plugin
#[cfg(all(feature = "uses_plugins", feature = "uses_rhai"))]
pub mod rhai_plugin;

// WASM Plugin
#[cfg(all(feature = "uses_plugins", feature = "uses_wasm"))]
pub mod wasm_plugin;

// Fallback export-based plugin
#[cfg(feature = "uses_wasm")]
pub mod wasm_export_plugin;
