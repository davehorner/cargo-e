<!--
  README-PLUGINS.md
  A companion document describing cargo-e's plugin discovery and precedence.
-->
# Cargo-e Plugin System Overview

This document explains how **cargo-e** discovers, loads, and overrides plugins.

> **Note:** This file is a supplemental README. The main usage guide remains in `README.md`.

## Plugin Locations and Precedence

cargo-e supports loading plugins from multiple locations, in this precedence order:

1. **Development plugins**
   - Directory: `plugins` folder in the cargo-e source tree (CARGO_MANIFEST_DIR/plugins).
   - Used when running from the repository checkout during development.
2. **Global user plugins**
   - Directory: `$HOME/.cargo-e/plugins` (or `%USERPROFILE%\.cargo-e\plugins` on Windows).
   - Install plugins you want available in all projects on a given machine.
3. **Project-local hidden plugins**
   - Directory: `.cargo-e/plugins` in the current working directory.
   - Store project-specific plugins without cluttering the repository root.

Entries are searched in this order; an earlier-loaded plugin overrides any later one with the same name.

## Plugin Types and Formats

cargo-e can load several plugin formats:
  - **Script plugins**: `*.lua` or `*.rhai` files, implementing the plugin API in their language.
  - **WASM plugins**: `*.wasm` modules, following the WASM export protocol.
  - **Rust crate plugins**: Directories with a `Cargo.toml`, built into Wasm or native dynamic libraries.

## Embedding and Distribution

> Installing:
```sh
cargo install cargo-e
```

On installation, only the `cargo-e` executable is installed. To use plugins:

1. **Global plugins:** place files in `$HOME/.cargo-e/plugins` (or `%USERPROFILE%\.cargo-e\plugins` on Windows).
2. **Project plugins:** create a `.cargo-e/plugins` folder in your project root and add plugins there.

Run `cargo e` as usual—cargo-e will scan in this precedence (dev → global → project) and load available plugins.

By following this scheme, you get:
- **Development-time** plugins via the source-tree `plugins/` folder (when working on cargo-e itself).
- **Global** shared plugins across all projects.
- **Per-project** overrides in the local `.cargo-e/plugins` directory.