/// Parses the stderr output to extract available items (e.g. binaries or examples)
/// by looking for a marker of the form "Available {item}:".
///
/// # Example
/// ```
/// use cargo_e::e_parser::parse_available;
///
/// let stderr = "Available examples:\n  example1\n  example2\n";
/// let result = parse_available(stderr, "examples");
/// assert_eq!(result, vec!["example1", "example2"]);
/// ```
pub fn parse_available(stderr: &str, item: &str) -> Vec<String> {
    let marker = format!("Available {}:", item);
    let mut available = Vec::new();
    let mut collecting = false;

    for line in stderr.lines() {
        if collecting {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                available.push(trimmed.to_string());
            }
        }
        if line.contains(&marker) {
            collecting = true;
        }
    }
    available
}

/// Reads run history from a file and returns a HashMap mapping target names to run counts.
pub fn read_run_history(
    history_path: &std::path::Path,
) -> std::collections::HashMap<String, usize> {
    let mut history = std::collections::HashMap::new();
    if let Ok(contents) = std::fs::read_to_string(history_path) {
        for line in contents.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Increment the count for this target.
                *history.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }
    }
    history
}
