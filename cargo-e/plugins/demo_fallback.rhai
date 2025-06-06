// demo_fallback.rhai: Demonstrate in-process run, per-target hook, and external fallback
// ============================================================================

// Unique plugin identifier; appears in the target list
fn name() {
    "rhai-demo"
}

// Always match any directory for demo purposes
fn matches(dir) {
    true
}

// Provide three targets: 'inproc', 'external', and 'fail'
fn collect_targets(dir) {
    // 'inproc'  -> handled by in-process run
    // 'external'-> falls back to external build_command
    // 'fail'    -> in-process failure for non-zero exit
    "[{\"name\":\"inproc\",\"metadata\":null},{\"name\":\"external\",\"metadata\":null},{\"name\":\"fail\",\"metadata\":null}]"
}

// Generic in-process handler for 'inproc'
fn run(dir, target) {
    if target == "inproc" {
        let msg = "✅  in-process handler for " + target;
        ["0", msg]
    } else if target == "fail" {
        let msg = "❌  in-process failure for " + target;
        ["5", msg]
    } else {
        // Throw to trigger external fallback for other targets
        throw "no in-process support"
    }
}

// Heuristic: if the script filename is "C:\…" it’s Windows
fn is_windows_script() {
    !script_path().starts_with("/")
}

// External fallback: invoked via build_command when run() throws or is absent
fn build_command(dir, target) {
    let msg = script_path()+" 🛑  external fallback for " + target;
    if is_windows_script() {
        // Windows: use cmd.exe /C echo
        "{\"prog\":\"cmd\",\"args\":[\"/C\",\"echo\",\"" + msg + "\"],\"cwd\":null}"
    } else {
        // macOS/Linux: use /bin/echo
        "{\"prog\":\"/bin/echo\",\"args\":[\"" + msg + "\"],\"cwd\":null}"
    }
}

// Optional per-target override for 'external'; uncomment to override the fallback
// fn external(dir, target) {
//     ["0", "🚀  per-target external() invoked"]
// }

