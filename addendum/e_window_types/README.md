# e_window_types

A lightweight shared types crate for the e_window ecosystem, designed to optimize build dependencies and eliminate unnecessary compilation of heavy UI frameworks when only type definitions are needed.

## Purpose

This crate was created to solve a critical build dependency optimization issue: `e_window_api` was previously required to depend on the full `e_window` crate, which includes heavy dependencies like `egui`, `eframe`, and platform-specific UI libraries. This meant that any project using the high-level API had to compile the entire UI framework stack, even if they only needed basic type definitions for message box functionality.

## Architecture

The `e_window_types` crate provides shared type definitions that can be used across the entire e_window ecosystem:

- **e_window_types** (this crate) - Lightweight shared types with minimal dependencies
- **e_window_api** - High-level API, depends ONLY on e_window_types (no heavy UI dependencies!)
- **e_window** - Full UI implementation, imports types from e_window_types

The API communicates with e_window via process spawning rather than library dependencies, ensuring complete separation.

## Shared Types

### Message Box Types

- `MessageBoxType` - Defines dialog types (Ok, OkCancel, YesNo, etc.)
- `MessageBoxIcon` - Defines icon types (Information, Warning, Error, etc.)  
- `MessageBoxResult` - Defines user response results (Ok, Cancel, Yes, No, etc.)

All types support optional `serde` serialization when the `serde` feature is enabled.

## Features

- `serde` - Enables serde serialization support for all types (optional)

## Benefits

1. **Dramatically faster builds** - e_window_api has ZERO dependency on heavy UI frameworks (egui, eframe, winapi, etc.)
2. **Complete separation** - API layer communicates via process spawning, not library dependencies
3. **Minimal footprint** - Only essential dependencies for high-level API usage
4. **Shared consistency** - All crates in the ecosystem use the same type definitions
5. **Optional serialization** - Serde support can be enabled when needed without forcing it on all users

## Version History

- v0.1.0 - Initial release with core message box types and optional serde support