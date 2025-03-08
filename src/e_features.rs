
/// Returns a vector of feature flag strings.
/// Enabled features are listed as-is while disabled ones are prefixed with "!".
pub fn get_feature_flags() -> Vec<&'static str> {
    [
        if cfg!(feature = "tui") { "tui" } else { "!tui" },
        if cfg!(feature = "concurrent") { "concurrent" } else { "!concurrent" },
        if cfg!(feature = "windows") { "windows" } else { "!windows" },
        if cfg!(feature = "equivalent") { "equivalent" } else { "!equivalent" },
    ]
    .to_vec()
}

/// Returns a JSON string representation of the feature flags.
pub fn get_feature_flags_json() -> String {
    let flags = get_feature_flags();
    format!(
        "[{}]",
        flags
            .iter()
            .map(|flag| format!("\"{}\"", flag))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
