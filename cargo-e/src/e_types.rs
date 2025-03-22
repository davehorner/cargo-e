/// Represents the kind of target that can be run by `cargo-e`.
///
/// This differentiates between Rust examples and binaries.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum TargetKind {
    Example,
    Binary,
    ExtendedExample,
    ExtendedBinary,
}

/// Represents an example or binary that can be executed using `cargo-e`.
///
/// This struct holds metadata about a runnable target within a Rust project.
///
/// # Fields
/// - `name`: The actual name of the target (e.g., `"hello_world"`).
/// - `display_name`: A formatted name used for display purposes.
/// - `manifest_path`: The path to the `Cargo.toml` file defining the target.
/// - `kind`: Specifies whether the target is an example or a binary.
/// - `extended`: Indicates whether the example is located in an extended directory.
///
/// # Example
/// ```
/// use cargo_e::{Example, TargetKind};
///
/// let example = Example {
///     name: "demo".to_string(),
///     display_name: "Demo Example".to_string(),
///     manifest_path: "examples/demo/Cargo.toml".to_string(),
///     kind: TargetKind::Example,
///     extended: true,
/// };
///
/// assert_eq!(example.name, "demo");
/// assert!(example.extended);
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Example {
    pub name: String,
    pub display_name: String,
    pub manifest_path: String,
    pub kind: TargetKind,
    pub extended: bool,
}
