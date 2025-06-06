// Example Rhai Plugin for cargo-e-ext
//
// This script demonstrates how to implement a Rhai-based project-local plugin.
// It defines the following entrypoints:
//   fn name() -> String
//   fn matches(dir: String) -> bool
//   fn collect_targets(dir: String) -> String
//   fn build_command(dir: String, target: String) -> String
// Optional in-process execution functions:
//   fn <target>(dir: String, target: String) -> Array
//   fn run(dir: String, target: String) -> Array
//
// Execution logic in the host:
// 1. Call fn <target>(dir, target) if present.
// 2. Else call fn run(dir, target) if present.
// 3. Else spawn the external command described by build_command.
// Return values from in-process functions must be an Array where:
// - The first element is the exit code (as String).
// - Remaining elements are output lines (as String).
// ================================================================================
fn name() {
    // Plugin name: this value is returned by name() and identifies the plugin.
    "cargo_e_collect"
}

// Determine whether this plugin applies to the given directory.
// `dir` is the current working directory as a String.
// Return `true` to match any directory.
fn matches(dir) {
    true
}

        fn collect_targets(dir) {
          let json = cargo_e_collect();
          print(json);
          json
        }

fn build_command(dir, target) {
// Build a JSON CommandSpec as a fallback when no in-process run is defined.
// CommandSpec fields:
//   prog: program to execute (e.g., "echo")
//   args: array of command-line arguments
//   cwd: optional working directory (null for default)
    let msg = "Hello from Rhai: " + target;
    "{\"prog\":\"echo\",\"args\":[\"" + msg + "\"],\"cwd\":null}"
}
// In-process execution entrypoint: called by the Rust host if defined.
// Should return an Array with the first element being the exit code (as String or Int),
// followed by any output lines (as String).
// Here, we delegate to the real cargo-e runner via the built-in `run_example` function.
fn run(dir, target) {
    // Execute the actual example by name and return its exit code.
    let code = run_example(target);
    // Return a single-element array with the exit code.
    [code]
}