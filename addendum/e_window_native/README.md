# e_window_native

Provides cross-platform screen size detection for e_window and related crates.

## Usage

Add to your Cargo.toml:

```toml
e_window_native = { path = "../e_window_native" }
```

Import and use:

```rust
use e_window_native::{get_cross_platform_screen_size, get_screen_size};
```
