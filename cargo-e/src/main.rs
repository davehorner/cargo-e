#![allow(unused_variables)]
//! # cargo-e
//!
//! `cargo-e` is a command-line tool to run and explore examples and binaries from Rust projects.
//! Unlike `cargo run --example`, it will run the example directly if only one exists.
//!
//! ## Features
//! - Runs single examples automatically
//! - Supports examples in different locations (bins, workspaces, etc.)
//! - Provides better navigation for Rust projects
//!
//! ## Quick Start
//! ```sh
//! cargo install cargo-e
//! cargo e
//! ```
//!
//! See the [GitHub repository](https://github.com/davehorner/cargo-e) for more details.

use cargo_e::e_cli::custom_cli;
use cargo_e::e_cli::RunAll;
use cargo_e::e_processmanager::ProcessManager;
use cargo_e::e_runner;
use cargo_e::e_runner::is_active_rust_script;
use cargo_e::e_target::CargoTarget;
use cargo_e::e_target::TargetKind;
use cargo_e::prelude::*;
use cargo_e::Cli;
use clap::Parser;
#[cfg(feature = "tui")]
use crossterm::terminal::size;
#[cfg(feature = "check-version-program-start")]
use e_crate_version_checker::prelude::*;
use futures::executor::block_on;
use once_cell::sync::Lazy;
#[cfg(feature = "uses_serde")]
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
#[cfg(not(target_os = "windows"))]
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::Mutex;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{AllocConsole, SetConsoleTitleW};
#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

// Plugin API
// Imports for plugin system
#[cfg(feature = "uses_plugins")]
use cargo_e::e_target::TargetOrigin;
#[cfg(feature = "uses_plugins")]
use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
use std::io::Read;
#[cfg(not(target_os = "windows"))]
use std::os::unix::process;
#[cfg(target_os = "windows")]
use std::os::windows::process;
#[cfg(feature = "uses_plugins")]
use std::path::PathBuf;
// Plugin loader modules
// Plugin modules (only when plugin system is enabled)
// Plugin modules moved to the library crate under `cargo_e::plugins`
//#[cfg(feature = "uses_plugins")]
//mod plugins;

static EXPLICIT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
static EXTRA_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

lazy_static::lazy_static! {
    static ref LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
}

fn enable_ansi_support() {
    #[cfg(windows)]
    {
        use std::io::stdout;
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Console::{
            GetConsoleMode, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        };

        unsafe {
            let handle = HANDLE(stdout().as_raw_handle() as *mut _);
            use windows::Win32::System::Console::CONSOLE_MODE;
            let mut mode: CONSOLE_MODE = std::mem::zeroed();
            if GetConsoleMode(handle, &mut mode as *mut CONSOLE_MODE).is_ok() {
                let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

fn setup_logging(log_path: Option<std::path::PathBuf>) -> io::Result<()> {
    if let Some(path) = log_path {
        println!("Logging output to: {}", path.display());
        let file = File::create(&path)?;
        *LOG_FILE.lock().unwrap() = Some(file);

        // On Windows, allocate a new console window to display the log file path.
        #[cfg(target_os = "windows")]
        {
            if unsafe { AllocConsole() }.is_ok() {
                // let title = format!("cargo-e log: {}", path.display());
                // let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                // unsafe { SetConsoleTitleW(windows::core::PWSTR(wide.as_ptr() as _)); }
                println!("Log file created at: {}", path.display());
                let hwnd = unsafe { windows::Win32::System::Console::GetConsoleWindow() };
                let pid = std::process::id();
                let hwnd_val = hwnd.0 as usize;
                let version = env!("CARGO_PKG_VERSION");
                let title = format!(
                    "cargo-e v{} | HWND: {:#x} | PID: {}",
                    version, hwnd_val, pid
                );
                let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                println!("Setting console title to: {}", title);
                let result = unsafe { SetConsoleTitleW(windows::core::PWSTR(wide.as_ptr() as _)) };
                println!("SetConsoleTitleW result: {:?}", result);
            }
        }
        // On other platforms, just print the log file path to stdout.
        #[cfg(not(target_os = "windows"))]
        {
            println!("Log file created at: {}", path.display());
        }
        let file = LOG_FILE.lock().unwrap();
        if let Some(ref file) = *file {
            #[cfg(unix)]
            {
                let fd = file.as_raw_fd();
                unsafe {
                    libc::dup2(fd, libc::STDOUT_FILENO);
                    libc::dup2(fd, libc::STDERR_FILENO);
                }
            }
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                let handle = HANDLE(file.as_raw_handle() as *mut _);
                unsafe {
                    let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
                    let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
                }
                // No need to call set_output_capture for file logging.
            }
        }
    }
    Ok(())
}

pub fn main() -> anyhow::Result<()> {
    enable_ansi_support();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).init();
    log::trace!(
        "cargo-e starting with args: {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    let mut args: Vec<String> = env::args().collect();

    let (run_at_a_time, filtered_args) = custom_cli(&mut args);

    let mut cli = Cli::parse_from(filtered_args);
    let log_path = cli.log.clone();
    setup_logging(log_path)?;
    if let Some(n) = run_at_a_time {
        cli.run_at_a_time = n;
    }
    if cli.version {
        cargo_e::e_cli::print_version_and_features();
        exit(0);
    }
    cargo_e::GLOBAL_CLI
        .set(cli.clone())
        .expect("Failed to set global CLI");

    let subcommand_provided_explicitly =
        args.iter().any(|arg| arg == "-s" || arg == "--subcommand");

    #[cfg(feature = "equivalent")]
    run_equivalent_example(&cli).ok(); // this std::process::exit()s

    let is_install_command = matches!(cli.subcommand.as_str(), "install" | "i");
    // Handle install command with diagnostic processing (without modifying cli)
    if is_install_command && std::env::var("CARGO_E_INSTALL_CHILD").is_err() {
        let install_path = if let Some(ref explicit) = cli.explicit_example {
            explicit.as_str()
        } else {
            "."
        };
        // Re-invoke this binary with an extra env var to indicate child mode.
        let mut args: Vec<String> = std::env::args().collect();
        let bin = args.remove(0);

        // On Windows, use PowerShell to detach and redirect output.
        #[cfg(target_os = "windows")]
        {
            let cargo = which::which("cargo").unwrap();
            let mut cmd = Command::new("powershell");
            cmd.arg("-NoProfile")
                .arg("-Command")
                .arg(format!(
                    "Start-Process -WindowStyle Hidden -FilePath '{}' -ArgumentList 'install','--path','{}' -Wait -RedirectStandardOutput 1install_out.txt -RedirectStandardError 1install_err.txt; \
                    $version = & '{}' --version; Write-Host `n$version; \
                    & '{}' --stdout 1install_out.txt --stderr 1install_err.txt; \
                    Write-Host \"Install finished at: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')\"\
                    ",// pause",
                    cargo.display(),
                    install_path,
                    bin,
                    bin
                ))
                .env("CARGO_E_INSTALL_CHILD", "1");
            let _ = cmd.spawn()?; // Detach and exit
            println!("spawned install process 1install_out.txt | 1install_err.txt; wait for output in this terminal.");
            std::process::exit(0);
        }
        // On Unix, use setsid to detach.
        #[cfg(not(target_os = "windows"))]
        {
            let args_str = args
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(" ");

            let setsid_prefix = if cfg!(target_os = "linux") {
                "setsid "
            } else {
                ""
            };
            let shell_cmd = format!(
                "sleep 1; {}sh -c 'env CARGO_E_INSTALL_CHILD=1 cargo install --path \"{}\" >1install_out.txt 2>1install_err.txt; \
                version=$({} --version); echo \"\\n$version\"; \
                {} --stdout 1install_out.txt --stderr 1install_err.txt; \
                echo \"Install finished at: $(date +\"%Y-%m-%d %H:%M:%S\")\"' < /dev/null &",
                setsid_prefix,
                install_path,
                bin,
                bin
            );

            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(&shell_cmd)
                .env("CARGO_E_INSTALL_CHILD", "1");

            // Ensure the process is fully detached by clearing the parent process group
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(|| {
                    libc::setsid(); // Create a new session
                    Ok(())
                });
            }

            let _ = cmd.spawn()?; // Fully detached
            println!("spawned install process 1install_out.txt | 1install_err.txt; wait for output in this terminal.");
            std::process::exit(0);
        }
    }

    // This block runs in the child process (CARGO_E_INSTALL_CHILD=1)
    if is_install_command {
        let install_path = if let Some(ref explicit) = cli.explicit_example {
            explicit.as_str()
        } else {
            "."
        };

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("install")
            .arg("--path")
            .arg(install_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        let output = child.wait_with_output()?; // Wait for process and capture output

        // Write stdout and stderr to files
        std::fs::write("1install_out.txt", &output.stdout)?;
        std::fs::write("1install_err.txt", &output.stderr)?;

        // Print version after install
        let bin = std::env::current_exe()?;
        let version_output = std::process::Command::new(&bin).arg("--version").output()?;
        print!("{}", String::from_utf8_lossy(&version_output.stdout));

        // Now call bin again with --stdout and --stderr to process diagnostics
        let status = std::process::Command::new(&bin)
            .arg("--stdout")
            .arg("1install_out.txt")
            .arg("--stderr")
            .arg("1install_err.txt")
            .status()?;

        std::process::exit(output.status.code().unwrap_or(1));
    }

    // Handle --stdout and --stderr flags for diagnostic processing
    let mut args_iter = std::env::args().peekable();
    let mut stdout_path = None;
    let mut stderr_path = None;
    while let Some(arg) = args_iter.next() {
        if arg == "--stdout" {
            stdout_path = args_iter.next();
        } else if arg == "--stderr" {
            stderr_path = args_iter.next();
        }
    }
    if let (Some(stdout_path), Some(stderr_path)) = (stdout_path, stderr_path) {
        use cargo_e::e_cargocommand_ext::CargoDiagnostic;
        use cargo_e::e_cargocommand_ext::CargoStats;
        use cargo_e::e_diagnostics_dispatchers::{
            create_stderr_dispatcher, create_stdout_dispatcher,
        };
        use std::io::{BufRead, BufReader};
        use std::sync::{Arc, Mutex};

        let diagnostics = Arc::new(Mutex::new(Vec::<CargoDiagnostic>::new()));
        let manifest_path = std::env::current_dir()?
            .join("Cargo.toml")
            .to_string_lossy()
            .to_string();
        let cargo_stats = Arc::new(Mutex::new(CargoStats::default()));
        let _stdout_dispatcher = create_stdout_dispatcher();
        let _stderr_dispatcher =
            create_stderr_dispatcher(diagnostics.clone(), manifest_path.clone());

        let mut any_output = false;

        // Process stdout file
        if let Ok(file) = std::fs::File::open(&stdout_path) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if _stdout_dispatcher
                    .dispatch(&line, cargo_stats.clone())
                    .iter()
                    .any(|r| r.is_some())
                {
                    any_output = true;
                }
            }
        }

        // Process stderr file
        if let Ok(file) = std::fs::File::open(&stderr_path) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if _stderr_dispatcher
                    .dispatch(&line, cargo_stats.clone())
                    .iter()
                    .any(|r| r.is_some())
                {
                    any_output = true;
                }
            }
        }

        // Print debug info for each diagnostic, printing errors last
        let diags = diagnostics.lock().unwrap();
        let (errors, others): (
            Vec<&cargo_e::e_cargocommand_ext::CargoDiagnostic>,
            Vec<&cargo_e::e_cargocommand_ext::CargoDiagnostic>,
        ) = diags
            .iter()
            .partition(|d: &&cargo_e::e_cargocommand_ext::CargoDiagnostic| d.level.eq("error"));
        for diag in &others {
            println!("{:?}", diag);
        }
        for diag in &errors {
            println!("{:?}", diag);
        }

        // Fallback: print raw output if nothing was shown
        if !any_output {
            if let Ok(out) = std::fs::read_to_string(&stdout_path) {
                print!("{}", out);
            }
            if let Ok(err) = std::fs::read_to_string(&stderr_path) {
                print!("{}", err);
            }
        }

        std::process::exit(0);
    }
    // // Here we run "cargo run --example funny_example" so that the build phase and runtime output are distinct.
    // println!("=== Running: cargo run --example funny_example ===");
    // let mut command = Command::new("cargo");
    // command.args(&[
    //     "run",
    //     "--example",
    //     "funny_example",
    //     "--color", "always",
    //     "--message-format=json-render-diagnostics",
    // ]);

    // // First run without an estimated output size.
    // let cargo_handle = command.spawn_cargo_capture(
    //     Some(stdout_dispatcher.clone()),
    //     Some(stderr_dispatcher.clone()),
    //     Some(progress_dispatcher.clone()),
    //     Some(stage_dispatcher.clone()),
    //     None, // no estimate provided
    // );
    // let result = cargo_handle.wait().expect("Failed during run");

    // let _ = cargo_e::e_runner::register_ctrlc_handler();
    #[cfg(feature = "check-version-program-start")]
    {
        if !cli.json_all_targets {
            e_crate_version_checker::register_user_crate!();
            // Attempt to retrieve the version from `cargo-e -v`
            let version = local_crate_version_via_executable("cargo-e")
                .map(|(_, version)| version)
                .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

            // Use the version from `lookup_cargo_e_version` if valid,
            // otherwise fallback to the compile-time version.
            let _ = interactive_crate_upgrade(env!("CARGO_PKG_NAME"), &version, cli.wait);
        }
    }
    let manager = ProcessManager::new(&cli);
    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // Collect built-in Cargo targets
    #[allow(unused_mut)]
    let mut examples = cargo_e::e_collect::collect_all_targets(
        cli.manifest_path.clone(),
        cli.workspace,
        num_threads,
        cli.json_all_targets,
        false,
    )
    .unwrap_or_default();

    if let Some(scan_dir) = &cli.scan_dir {
        let scanned_targets = cargo_e::e_discovery::scan_directory_for_targets(
            scan_dir,
            cli.quiet || cli.json_all_targets,
        );
        if scanned_targets.is_empty() {
            eprintln!("No targets found in scanned directories.");
        } else {
            examples.extend(scanned_targets);
            if !(cli.quiet || cli.json_all_targets) {
                println!("Scanned targets from directory: {}", scan_dir.display());
            }
        }
    }

    if cli.parse_available {
        use cargo_e::e_collect::collect_stdin_available;
        let manifest_path = PathBuf::from(
            cargo_e::e_manifest::locate_manifest(cli.workspace)
                .expect("Failed to locate Cargo.toml"),
        );
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        let stdin_examples = collect_stdin_available("examples", &manifest_path, &buf, false);
        let stdin_binaries = collect_stdin_available("binaries", &manifest_path, &buf, false);

        examples.extend(stdin_examples);
        examples.extend(stdin_binaries);

        println!("Parsed examples and binaries from stdin.");
    }

    // Collect plugin-provided targets and merge
    #[cfg(feature = "uses_plugins")]
    {
        #[allow(unused_imports)]
        use cargo_e::e_target::{CargoTarget, TargetKind, TargetOrigin};
        use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
        use std::path::PathBuf;
        let cwd = std::env::current_dir()?;
        log::trace!("Invoking load_plugins() to discover plugins");
        for plugin in load_plugins(&cli, manager.clone())? {
            if plugin.matches(&cwd) {
                let plugin_path = plugin.source().map(PathBuf::from).unwrap_or(cwd.clone());
                for mut pt in plugin.collect_targets(&cwd)? {
                    // If plugin provided a full CargoTarget, use it directly.
                    if let Some(ct) = pt.cargo_target.take() {
                        examples.push(ct);
                    } else {
                        let reported = pt
                            .metadata
                            .as_ref()
                            .map(PathBuf::from)
                            .unwrap_or_else(|| cwd.clone());
                        examples.push(CargoTarget {
                            name: pt.name.clone(),
                            display_name: pt.name.clone(),
                            manifest_path: cwd.clone(),
                            kind: TargetKind::Plugin,
                            extended: false,
                            toml_specified: false,
                            origin: Some(TargetOrigin::Plugin {
                                plugin_path: plugin_path.clone(),
                                reported,
                            }),
                        });
                    }
                }
            }
        }
    }
    use std::collections::HashSet;

    // After collecting all samples, deduplicate them.
    let mut seen = HashSet::new();
    let unique_examples: Vec<CargoTarget> = examples
        .clone()
        .into_iter()
        .filter(|e| {
            let key = (e.name.clone(), e.kind); //, e.extended //, e.toml_specified
            seen.insert(key)
        })
        .collect();

    let builtin_examples: Vec<&CargoTarget> = examples
        .iter()
        .filter(|e| e.toml_specified && matches!(e.kind, TargetKind::Example))
        .collect();

    let builtin_binaries: Vec<&CargoTarget> = examples
        .iter()
        .filter(|e| e.toml_specified && e.kind == TargetKind::Binary)
        .collect();
    // If --target is specified and explicit_example is None, set explicit_example to the value of target
    if cli.explicit_example.is_none() {
        if let Some(ref target) = cli.target {
            cli.explicit_example = Some(target.clone());
        }
    }
    // Handle --json-targets: print all discovered targets as JSON and exit
    #[cfg(feature = "uses_serde")]
    if cli.json_all_targets {
        let json_targets = unique_examples
            .iter()
            .map(|t| {
                let command = {
                    use cargo_e::e_command_builder::CargoCommandBuilder;
                    let manifest_path = t
                        .manifest_path
                        .canonicalize()
                        .unwrap_or_else(|_| t.manifest_path.clone());
                    let builder = CargoCommandBuilder::new(
                        &t.name,
                        &manifest_path,
                        &cli.subcommand,
                        cli.filter,
                        cli.cached,
                        cli.default_binary_is_runner,
                        cli.quiet || cli.json_all_targets,
                        cli.detached,
                    )
                    .with_target(&t)
                    .with_cli(&cli)
                    .with_extra_args(&cli.extra);
                    builder.injected_args()
                };
                json!({
                    "name": t.name,
                    "display_name": t.display_name,
                    "manifest_path": t.manifest_path.display().to_string(),
                    "kind": format!("{:?}", t.kind),
                    "extended": t.extended,
                    "toml_specified": t.toml_specified,
                    "origin": t.origin.as_ref().map(|o| format!("{:?}", o)),
                    "program": command.0,
                    "args": command.1,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&json_targets).unwrap());
        std::process::exit(0);
    }
    if let Some(explicit) = cli.explicit_example.clone() {
        if let Some(explicit_example) = cli.explicit_example.clone() {
            let mut explicit_lock = EXPLICIT.lock().unwrap();
            *explicit_lock = explicit_example;
        }
        {
            let mut extra_lock = EXTRA_ARGS.lock().unwrap();
            *extra_lock = cli.extra.clone();
        }
        println!("Explicit: {:?}", explicit);
        // Now call run_rust_script_with_ctrlc_handling
        e_runner::run_scriptisto_with_ctrlc_handling(explicit.clone(), cli.extra.clone());
        e_runner::run_rust_script_with_ctrlc_handling(explicit.clone(), cli.extra.clone());
        // Search the discovered targets for one with the matching name.
        // Try examples first.
        if let Some(target) = examples.iter().find(|t| t.name == explicit) {
            // Plugin target in explicit mode
            if target.kind == TargetKind::Plugin {
                #[cfg(feature = "uses_plugins")]
                {
                    use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
                    // Find corresponding plugin and run in-process
                    let cwd = std::env::current_dir()?;
                    if let Some(origin) = &target.origin {
                        if let TargetOrigin::Plugin {
                            plugin_path,
                            reported,
                        } = origin
                        {
                            let pt = PluginTarget {
                                name: target.name.clone(),
                                metadata: Some(reported.to_string_lossy().to_string()),
                                cargo_target: None,
                            };
                            for plugin in load_plugins(&cli, manager.clone())? {
                                if plugin.source().map(|s| PathBuf::from(s))
                                    == Some(plugin_path.clone())
                                {
                                    // Delegate execution to run_with_manager
                                    let result =
                                        plugin.run_with_manager(manager.clone(), &cli, target)?;
                                    if let Some(status) = result {
                                        println!("Plugin exited with code: {:?}", status.code());
                                    }
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            } else {
                #[cfg(feature = "tui")]
                if cli.tui {
                    do_tui_and_exit(manager, &cli, &unique_examples);
                }
                let ret = cargo_e::e_runner::run_example(manager.clone(), &cli, target)?;
                manager.clone().generate_report(cli.gist);
                manager.clone().cleanup();
                std::process::exit(ret.map(|status| status.code().unwrap_or(1)).unwrap_or(1));
            }
        }
        // If not found among examples, search for a binary with that name.
        else if let Some(target) = examples
            .iter()
            .find(|t| t.kind == TargetKind::Binary && t.name == explicit)
        {
            #[cfg(feature = "tui")]
            if cli.tui {
                do_tui_and_exit(manager, &cli, &unique_examples);
            }
            let ret = cargo_e::e_runner::run_example(manager.clone(), &cli, target)?;
            manager.clone().generate_report(cli.gist);
            manager.clone().cleanup();
            std::process::exit(ret.map(|status| status.code().unwrap_or(1)).unwrap_or(1));
        } else {
            eprintln!(
                "error: 0 named '{}' found in examples or binaries.",
                explicit
            );

            // no exact match found: perform a partial search over the unique examples.
            let query = explicit.to_lowercase();
            let fuzzy_matches: Vec<CargoTarget> = unique_examples
                .iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
            if fuzzy_matches.is_empty() {
                std::process::exit(1);
            } else {
                println!(
                    "partial search results {} for '{}':",
                    fuzzy_matches.len(),
                    explicit
                );
                if fuzzy_matches.len() == 1 && subcommand_provided_explicitly {
                    println!(
                        "Subcommand provided explicitly with 1 target.\nRunning {}...",
                        fuzzy_matches[0].name
                    );
                    cargo_e::e_runner::run_example(manager.clone(), &cli, &fuzzy_matches[0])?;
                    return Ok(());
                }

                if cli.run_all != RunAll::NotSpecified {
                    //PROMPT cargo_e::e_prompts::prompt(&"", 2).ok();
                    // Pass in your default packages, which are now generic.
                    cargo_e::e_runall::run_all_examples(manager.clone(), &cli, &fuzzy_matches)?;
                    manager.generate_report(cli.gist);
                    Arc::clone(&manager).cleanup();
                    return Ok(());
                }

                #[cfg(feature = "tui")]
                if cli.tui {
                    do_tui_and_exit(manager, &cli, &fuzzy_matches);
                }
                cli_loop(manager.clone(), &cli, &fuzzy_matches, &[], &[]);
            }
            manager.clone().generate_report(cli.gist);
            manager.clone().cleanup();
            std::process::exit(1);
        }
    }

    if cli.run_all != RunAll::NotSpecified {
        cargo_e::e_runall::run_all_examples(manager.clone(), &cli, &unique_examples)?;
        manager.generate_report(cli.gist);
        manager.cleanup();
        return Ok(());
    }

    if builtin_examples.len() == 1
        || (builtin_examples.is_empty() && builtin_binaries.len() == 1)
        || unique_examples.len() == 1
    {
        #[cfg(feature = "tui")]
        if cli.tui {
            do_tui_and_exit(manager, &cli, &unique_examples);
        }

        let target = if builtin_examples.len() == 1 {
            builtin_examples[0]
        } else if builtin_binaries.len() == 1 {
            builtin_binaries[0]
        } else {
            &unique_examples[0]
        };

        handle_single_target(
            manager.clone(),
            &cli,
            target,
            &unique_examples,
            &builtin_examples,
            &builtin_binaries,
            subcommand_provided_explicitly,
        )?;
    } else {
        if !subcommand_provided_explicitly {
            provide_notice_of_no_examples(manager.clone(), &cli, &unique_examples).ok();
        }

        #[cfg(feature = "tui")]
        if cli.tui {
            do_tui_and_exit(manager, &cli, &unique_examples);
        }
        cli_loop(
            manager.clone(),
            &cli,
            &unique_examples,
            &builtin_examples,
            &builtin_binaries,
        );
        // if builtin_examples.len() + builtin_binaries.len() > 1 {
        //     //select_and_run_target(&cli, &examples, &builtin_examples, &builtin_binaries)?;
        // } else {
        //     eprintln!("Available: {:#?}", examples);
        //     std::process::exit(1);
        // }
    }
    println!("Exiting.");
    manager.kill_all();
    println!("generate_report.");
    manager.generate_report(cli.gist);
    println!("Done.");
    Ok(())
}

#[allow(dead_code)]
fn do_tui_and_exit(manager: Arc<ProcessManager>, cli: &Cli, unique_examples: &[CargoTarget]) -> ! {
    #[cfg(feature = "tui")]
    {
        let ret = cargo_e::e_tui::tui_interactive::launch_tui(manager, cli, unique_examples);
        if let Err(e) = ret {
            eprintln!("TUI Error: {:?}", e);
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "tui"))]
    {
        // If TUI is not enabled, just print a message and exit.
        eprintln!("TUI is not supported in this build. Exiting.");
        cli_loop(manager, &cli, &unique_examples, &[], &[]);
    }
    std::process::exit(0);
}

fn provide_notice_of_no_examples(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    examples: &[CargoTarget],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ex_count = examples
        .iter()
        .filter(|e| matches!(e.kind, TargetKind::Example))
        .count();
    let bin_count = examples
        .iter()
        .filter(|e| e.kind == TargetKind::Binary)
        .count();

    println!(
        "({} targets: {} examples, {} binaries)",
        examples.len(),
        ex_count,
        bin_count
    );

    if cli.wait != 0 {
        println!(
            "== press q, t, wait for {} seconds, or other key to continue.",
            cli.wait
        );
    }
    if let Some(line) =
        cargo_e::e_prompts::prompt_line_with_poll_opts(cli.wait, &[' ', 'c', 't', 'q', '\n'], None)?
    {
        let trimmed = line.trim();
        // Continue if input is empty or "c"; otherwise, quit.
        #[cfg(feature = "tui")]
        {
            if trimmed.eq_ignore_ascii_case("t") {
                do_tui_and_exit(manager, cli, examples);
            }
        }

        if trimmed.eq_ignore_ascii_case("q") {
            println!("quit.");
            std::process::exit(0);
        }
    }
    Ok(())
}

#[cfg(feature = "equivalent")]
fn run_equivalent_example(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example");
    if let Some(explicit) = &cli.explicit_example {
        cmd.arg(explicit);
    }
    if !cli.extra.is_empty() {
        cmd.arg("--").args(cli.extra.clone());
    }
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

/// The result returned by the selection loop.
enum LoopResult {
    Quit,
    Continue,
    Run(std::process::ExitStatus, usize), // second value is the current offset/page index
}

/// The selection function: displays targets, waits for input, and returns a LoopResult.
fn select_and_run_target_loop(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    unique_targets: &[CargoTarget],
    builtin_examples: &[&CargoTarget],
    builtin_binaries: &[&CargoTarget],
    start_offset: usize, // new parameter for starting page offset
) -> Result<LoopResult, Box<dyn Error + Send + Sync>> {
    let current_offset = start_offset;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let prompt_loop = format!(
        "== # to run, tui, e<#> edit, i<#> info, 'q' to quit (waiting {} seconds) ",
        cli.wait
    );

    // Determine if there are multiple manifest paths
    let manifest_paths: Vec<_> = unique_targets
        .iter()
        .map(|t| t.manifest_path.clone())
        .collect();
    let has_multiple_manifests = manifest_paths
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
        > 1;

    // Build a combined list: examples first, then binaries.
    let mut combined: Vec<(String, &CargoTarget)> = Vec::new();
    for target in unique_targets {
        let label = target.display_label();
        let manifest_relative = if has_multiple_manifests {
            target
                .manifest_path
                .strip_prefix(&cwd)
                .unwrap_or(&target.manifest_path)
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| String::from("<unknown>"))
        } else {
            String::new()
        };
        let manifest_relative = manifest_relative
            .trim_start_matches("./")
            .trim_start_matches(".\\");
        let manifest_relative =
            manifest_relative.replace(['/', '\\'], &std::path::MAIN_SEPARATOR.to_string());
        let display_label = if has_multiple_manifests && !manifest_relative.is_empty() {
            format!("{} {}", manifest_relative, label)
        } else {
            label
        };
        combined.push((display_label, target));
    }

    combined.sort_by(|(a, ex_a), (b, ex_b)| {
        if a == b {
            ex_a.name.cmp(&ex_b.name)
        } else {
            a.cmp(b)
        }
    });
    // Determine the required padding width based on the number of targets
    let pad_width = combined.len().to_string().len();
    // Load run history from file.
    let manifest_dir = cargo_e::e_manifest::find_manifest_dir()
        .unwrap_or_else(|_| std::env::current_dir().unwrap());
    let history_path = manifest_dir.join("run_history.txt");
    let run_history = cargo_e::e_parser::read_run_history(&history_path);

    // Print the list.
    if cli.paging {
        //println!("Available:");
        let total = combined.len();
        let mut current_index = current_offset;
        while current_index < total {
            // Reserve two lines for the prompt/status.
            let rows = {
                #[cfg(feature = "tui")]
                {
                    let (_, term_rows) = size()?; // get terminal size
                    std::cmp::min(term_rows, 9) // choose the lesser of terminal rows and 9
                }
                #[cfg(not(feature = "tui"))]
                {
                    9
                }
            };
            let page_lines = {
                #[cfg(feature = "tui")]
                {
                    let (_, term_rows) = size()?;
                    // If the terminal has fewer than 9 rows, subtract 2 for the prompt/input,
                    // but ensure that we display at least one line.
                    if term_rows < 9 {
                        std::cmp::max((term_rows.saturating_sub(2)) as usize, 1)
                    } else {
                        9
                    }
                }
                #[cfg(not(feature = "tui"))]
                {
                    9
                }
            };
            let end_index = usize::min(current_index + page_lines as usize, total);
            for (i, (target_type, target)) in combined[current_index..end_index].iter().enumerate()
            {
                // let base_line = format!(
                //     "  {:>2}: [{}] {}",
                //     current_index + i + 1,
                //     target_type,
                //     target.name
                // );
                // Use relative numbering if enabled (reset per page), otherwise absolute numbering.
                let line_number = if cli.relative_numbers {
                    i + 1
                } else {
                    current_index + i + 1
                };
                let base_line = format!(
                    "  {:>width$}: [{}] {} ", //{:?} {:?}",
                    line_number,
                    target_type,
                    target.display_name,
                    //target.origin.as_ref().unwrap(),
                    //target.name,
                    width = pad_width
                );
                let styled_line = if let Some(count) = run_history.get(&target.name) {
                    // If the target was run before, highlight in blue and append run count.
                    let line_with_count = format!(
                        "{} ({} run{})",
                        base_line,
                        count,
                        if *count == 1 { "" } else { "s" }
                    );
                    #[cfg(feature = "tui")]
                    crossterm::style::Stylize::bold(crossterm::style::Stylize::with(
                        line_with_count,
                        crossterm::style::Color::Blue,
                    ))
                } else {
                    // Otherwise, print in default white.
                    #[cfg(feature = "tui")]
                    crossterm::style::Stylize::with(base_line, crossterm::style::Color::White)
                };
                #[cfg(not(feature = "tui"))]
                let styled_line = base_line;
                println!("{}", styled_line);

                //                println!("  {:>2}: [{}] {}", current_index + i + 1, target_type, target.name);
            }
            if end_index < total {
                println!("{}", &prompt_loop);
                io::Write::flush(&mut io::stdout())?;
                let mut quick_exit_keys = vec!['q', 't', ' '];
                if cli.relative_numbers {
                    // If relative numbering is enabled, add all digits as additional quick exit keys.
                    quick_exit_keys.extend('0'..='9');
                }
                let mut allowed_chars: Vec<char> = ('0'..='9').collect();
                allowed_chars.push('e');
                allowed_chars.push('E');
                allowed_chars.push('i');
                allowed_chars.push('I');
                if let Some(line) = cargo_e::e_prompts::prompt_line_with_poll_opts(
                    cli.wait.max(3),
                    &quick_exit_keys,
                    Some(&allowed_chars),
                )? {
                    if !line.trim().is_empty() {
                        // Early selection.
                        let selection = line;
                        // current_index = total; // break out of paging loop.
                        // println!("{} currindex", current_index);
                        return process_input(manager, &selection, &combined, cli, current_index);
                    }
                }
                current_index = end_index;
            } else {
                break;
            }
        }
    } else {
        // Non-paging mode: print all targets.
        //println!("Available:");
        for (i, (target_type, target)) in combined.iter().enumerate() {
            let base_line = format!(
                "  {:>width$}: [{}] {}",
                i + 1,
                target_type,
                target.name,
                width = pad_width
            );

            // let base_line = format!("  {:>2}: [{}] {}", i + 1, target_type, target.name);
            let styled_line = if let Some(count) = run_history.get(&target.name) {
                // If the target was run before, highlight in blue and append run count.
                let line_with_count = format!(
                    "{} ({} run{})",
                    base_line,
                    count,
                    if *count == 1 { "" } else { "s" }
                );
                #[cfg(feature = "tui")]
                crossterm::style::Stylize::bold(crossterm::style::Stylize::with(
                    line_with_count,
                    crossterm::style::Color::Blue,
                ))
            } else {
                // Otherwise, print in default white.
                #[cfg(feature = "tui")]
                crossterm::style::Stylize::with(base_line, crossterm::style::Color::White)
            };
            #[cfg(not(feature = "tui"))]
            let styled_line = base_line;
            println!("{}", styled_line);

            //println!("  {:>2}: [{}] {}", i + 1, target_type, target.name);
        }
    }

    // Final prompt.
    println!("* {}", &prompt_loop);
    io::Write::flush(&mut io::stdout())?;
    let final_input = if combined.len() > 9 {
        let quick_exit_keys = ['q', 't', ' '];
        let allowed_digits: Vec<char> = ('0'..='9').collect();
        cargo_e::e_prompts::prompt_line_with_poll_opts(
            cli.wait.max(3),
            &quick_exit_keys,
            Some(&allowed_digits),
        )?
    } else {
        // For fewer targets, use a simple single-character prompt.
        let mut allowed: Vec<char> = ('0'..='9').collect();
        allowed.extend(['e']);
        allowed.extend(['i']);
        allowed.extend(['I']);
        allowed.extend(['E']);
        let mut quick_exit_keys: Vec<char> = vec!['q', 't', ' '];
        quick_exit_keys.extend('0'..='9');
        cargo_e::e_prompts::prompt_line_with_poll_opts(
            cli.wait.max(3),
            &quick_exit_keys,
            Some(&allowed),
        )?
    }
    .unwrap_or_default();
    println!("{}", &final_input);
    process_input(manager, &final_input, &combined, cli, 0)
}
pub fn append_run_history(target_name: &str) -> io::Result<()> {
    let manifest_dir = cargo_e::e_manifest::find_manifest_dir()
        .unwrap_or_else(|_| std::env::current_dir().expect("Failed to get current directory"));
    let history_path = manifest_dir.join("run_history.txt");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;
    writeln!(file, "{}", target_name)?;
    Ok(())
}
/// Processes the final input string and returns a LoopResult.
fn process_input(
    manager: Arc<ProcessManager>,
    input: &str,
    combined: &[(String, &CargoTarget)],
    cli: &Cli,
    offset: usize, // offset for the current page
) -> Result<LoopResult, Box<dyn Error + Send + Sync>> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("q") {
        Ok(LoopResult::Quit)
    } else if trimmed.eq_ignore_ascii_case("t") {
        #[cfg(feature = "tui")]
        {
            let tui_examples: Vec<CargoTarget> =
                combined.iter().map(|&(_, ex)| ex.clone()).collect();
            do_tui_and_exit(manager, cli, &tui_examples);
        }
        #[cfg(not(feature = "tui"))]
        {
            Ok(LoopResult::Quit)
        }
    } else if trimmed.to_lowercase().starts_with("i") {
        // Handle the "edit" command: e<num>
        let num_str = trimmed[1..].trim();
        if let Ok(rel_index) = num_str.parse::<usize>() {
            let abs_index = if cli.relative_numbers {
                offset + rel_index - 1
            } else {
                rel_index - 1
            };
            if abs_index > combined.len() {
                eprintln!("error: Invalid target number for edit: {}", trimmed);
                Ok(LoopResult::Quit)
            } else {
                let (target_type, target) = &combined[abs_index];
                println!("info {} \"{}\"...", target_type, target.name);
                futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(target));
                cargo_e::e_prompts::prompt_line("", 120).ok();
                // After editing, you might want to pause briefly or simply return to the menu.
                Ok(LoopResult::Run(
                    <std::process::ExitStatus as process::ExitStatusExt>::from_raw(0),
                    offset,
                ))
            }
        } else {
            eprintln!("error: Invalid edit command: {}", trimmed);
            Ok(LoopResult::Quit)
        }
    } else if trimmed.to_lowercase().starts_with("e") {
        // Handle the "edit" command: e<num>
        let num_str = trimmed[1..].trim();
        if let Ok(rel_index) = num_str.parse::<usize>() {
            let abs_index = if cli.relative_numbers {
                offset + rel_index - 1
            } else {
                rel_index - 1
            };
            if abs_index > combined.len() {
                eprintln!("error: Invalid target number for edit: {}", trimmed);
                Ok(LoopResult::Quit)
            } else {
                let (target_type, target) = &combined[abs_index];
                println!("editing {} \"{}\"...", target_type, target.name);
                // Call the appropriate function to open the target for editing.
                // For example, if using VSCode:
                block_on(cargo_e::e_findmain::open_vscode_for_sample(target));
                // After editing, you might want to pause briefly or simply return to the menu.
                Ok(LoopResult::Run(
                    <std::process::ExitStatus as process::ExitStatusExt>::from_raw(0),
                    offset,
                ))
            }
        } else {
            eprintln!("error: Invalid edit command: {}", trimmed);
            Ok(LoopResult::Continue)
        }
    } else if let Ok(rel_index) = trimmed.parse::<usize>() {
        let abs_index = if cli.relative_numbers {
            offset + rel_index - 1
        } else {
            rel_index - 1
        };
        if abs_index > combined.len() {
            eprintln!("invalid number: {}", trimmed);
            Ok(LoopResult::Continue)
        } else {
            let (target_type, target) = &combined[abs_index];
            if cli.print_program_name {
                println!("running {} \"{}\"...", target_type, target.name);
            }
            let status = e_runner::run_example(manager, &cli, target)?;
            let _ = append_run_history(&target.name.clone());
            let message = if cli.print_exit_code {
                format!(
                    "Exitcode {:?}. Press any key to continue...",
                    status.unwrap().code()
                )
            } else {
                "".to_string()
            };
            //PROMPT let _ = cargo_e::e_prompts::prompt(&message, cli.wait)?;

            Ok(LoopResult::Run(
                status.unwrap_or(<std::process::ExitStatus as process::ExitStatusExt>::from_raw(1)),
                offset,
            ))
        }
    } else {
        Ok(LoopResult::Quit)
    }
}

/// The outer CLI loop. It repeatedly calls the selection loop.
/// If a target exits with an "interrupted" code (e.g. 130), it re‑displays the menu.
/// If the user quits (input "q"), it exits.
fn cli_loop(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    unique_examples: &[CargoTarget],
    builtin_examples: &[&CargoTarget],
    builtin_binaries: &[&CargoTarget],
) {
    let mut current_offset = 0;
    let manager_clone = manager.clone();

    loop {
        match select_and_run_target_loop(
            manager_clone.clone(),
            cli,
            unique_examples,
            builtin_examples,
            builtin_binaries,
            current_offset,
        ) {
            Ok(LoopResult::Quit) => {
                manager_clone.generate_report(cli.gist);
                println!("quitting.");
                break;
            }
            Ok(LoopResult::Run(status, new_offset)) => {
                current_offset = new_offset;
                if status.code() == Some(130) {
                    println!("interrupted (Ctrl+C). returning to menu.");
                    continue;
                }
                if cli.run_at_a_time > 0 {
                    let timeout = std::time::Duration::from_secs(cli.run_at_a_time as u64);
                    let handle = std::thread::spawn(move || status);

                    match handle.join_timeout(timeout) {
                        Ok(_) => {
                            println!("Process finished successfully.");
                            let hold = cli.detached_hold.unwrap_or(0);
                            if cli.detached_hold.is_some() && hold > 0 {
                                println!("holding for the duration (detached_hold enabled). Sleeping for {} seconds...", hold);
                                std::thread::sleep(std::time::Duration::from_secs(hold as u64));
                            }
                        }
                        Err(_) => {
                            eprintln!("Timeout reached. Killing all processes.");
                            let hold = cli.detached_hold.unwrap_or(0);
                            if cli.detached_hold.is_some() && hold > 0 {
                                println!("holding for the duration (detached_hold enabled). Sleeping for {} seconds...", hold);
                                std::thread::sleep(std::time::Duration::from_secs(hold as u64));
                            }
                            manager_clone.kill_all();
                            break;
                        }
                    }
                } else {
                    println!("Process finished with status: {:?}", status.code());
                    let hold = cli.detached_hold.unwrap_or(0);
                    if cli.detached_hold.is_some() && hold > 0 {
                        println!("holding for the duration (detached_hold enabled). Sleeping for {} seconds...", hold);
                        std::thread::sleep(std::time::Duration::from_secs(hold as u64));
                    }
                }
            }
            Ok(LoopResult::Continue) => continue,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                let hold = cli.detached_hold.unwrap_or(0);
                if cli.detached_hold.is_some() && hold > 0 {
                    println!("holding for the duration (detached_hold enabled). Sleeping for {} seconds...", hold);
                    std::thread::sleep(std::time::Duration::from_secs(hold as u64));
                }
                std::process::exit(1);
            }
        }
    }
}

trait JoinTimeout {
    fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()>;
}

impl<T> JoinTimeout for std::thread::JoinHandle<T> {
    fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()> {
        let result = std::thread::sleep(timeout);
        match self.join() {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub fn run_rust_script_with_ctrlc_handling() {
    let explicit = {
        let lock = EXPLICIT.lock().unwrap_or_else(|e| {
            eprintln!("Failed to acquire lock: {}", e);
            std::process::exit(1); // Exit the program if the lock cannot be obtained
        });
        lock.clone() // Clone the data to move it into the thread
    };

    let explicit_path = Path::new(&explicit); // Construct Path outside the lock

    if explicit_path.exists() {
        match is_active_rust_script(&explicit_path) {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                // Handle the error locally without propagating it
                eprintln!("Failed to check if the file is a rust-script");
                std::process::exit(1); // Exit with an error code
            }
        }

        let extra_args = EXTRA_ARGS.lock().unwrap(); // Locking the Mutex to access the data
        let extra_str_slice: Vec<String> = extra_args.iter().cloned().collect();

        // Run the child process in a separate thread to allow Ctrl+C handling
        let handle = std::thread::spawn(move || {
            let extra_str_slice_cloned = extra_str_slice.clone();
            let child = e_runner::run_rust_script(
                &explicit,
                &extra_str_slice_cloned
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|| {
                eprintln!("Failed to run rust-script: {:?}", &explicit);
                std::process::exit(1); // Exit with an error code
            });
        });

        // Wait for the thread to complete, but with a timeout
        let timeout = std::time::Duration::from_secs(10);
        match handle.join_timeout(timeout) {
            Ok(_) => {
                println!("Child process finished successfully.");
            }
            Err(_) => {
                eprintln!("Child process took too long to finish. Exiting...");
                std::process::exit(1); // Exit if the process takes too long
            }
        }
    }
}

fn handle_single_target(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    target: &CargoTarget,
    unique_examples: &[CargoTarget],
    builtin_examples: &[&CargoTarget],
    builtin_binaries: &[&CargoTarget],
    subcommand_provided_explicitly: bool,
) -> anyhow::Result<()> {
    if subcommand_provided_explicitly {
        println!("Subcommand provided explicitly. Running {}...", target.name);
        cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
        return Ok(());
    } else if !cli.extra.is_empty() {
        // If there are extra arguments provided, run the target directly
        cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
        return Ok(());
    }

    let message = format!(
        "{} found. run? (Yes / no / edit / tui / info)     waiting {} seconds.",
        target.name, cli.wait
    );

    match cargo_e::e_prompts::prompt(&message, cli.wait.max(3))? {
        Some('y') | Some(' ') | Some('\n') => {
            println!("running {}...", target.name);
            cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
        }
        Some('n') => {
            cli_loop(
                manager,
                cli,
                unique_examples,
                builtin_examples,
                builtin_binaries,
            );
            std::process::exit(0);
        }
        Some('e') => {
            block_on(cargo_e::e_findmain::open_vscode_for_sample(target));
        }
        Some('i') => {
            futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(target));
            cargo_e::e_prompts::prompt_line("", 120).ok();
            cli_loop(
                manager.clone(),
                cli,
                unique_examples,
                builtin_examples,
                builtin_binaries,
            );
        }
        Some('t') => {
            #[cfg(feature = "tui")]
            do_tui_and_exit(manager, cli, unique_examples);
            #[cfg(not(feature = "tui"))]
            {
                println!("tui not enabled.");
                cli_loop(
                    manager,
                    cli,
                    unique_examples,
                    builtin_examples,
                    builtin_binaries,
                );
                std::process::exit(0);
            }
        }
        Some(other) => {
            println!("{}. exiting.", other);
            std::process::exit(1);
        }
        None => {
            cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
        }
    }

    Ok(())
}

// fn cli_loop(
//     manager: Arc<ProcessManager>,
//     cli: &Cli,
//     unique_examples: &[CargoTarget],
//     builtin_examples: &[&CargoTarget],
//     builtin_binaries: &[&CargoTarget],
// ) {
//     let mut current_offset = 0;
//     let manager_clone = manager.clone();

//     loop {
//         match select_and_run_target_loop(
//             manager_clone.clone(),
//             cli,
//             unique_examples,
//             builtin_examples,
//             builtin_binaries,
//             current_offset,
//         ) {
//             Ok(LoopResult::Quit) => {
//                 manager_clone.generate_report(cli.gist);
//                 println!("quitting.");
//                 break;
//             }
//             Ok(LoopResult::Run(status, new_offset)) => {
//                 current_offset = new_offset;

//                 if cli.run_at_a_time > 0 {
//                     let timeout = Duration::from_secs(cli.run_at_a_time as u64);
//                     let handle = std::thread::spawn(move || status);

//                     match handle.join_timeout(timeout) {
//                         Ok(_) => println!("Process finished successfully."),
//                         Err(_) => {
//                             eprintln!("Timeout reached. Killing all processes.");
//                             manager_clone.kill_all();
//                             break;
//                         }
//                     }
//                 } else {
//                     println!("Process finished with status: {:?}", status.code());
//                 }
//             }
//             Ok(LoopResult::Continue) => continue,
//             Err(err) => {
//                 eprintln!("Error: {:?}", err);
//                 std::process::exit(1);
//             }
//         }
//     }
// }

// trait JoinTimeout {
//     fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()>;
// }

// impl<T> JoinTimeout for std::thread::JoinHandle<T> {
//     fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()> {
//         let result = std::thread::sleep(timeout);
//         match self.join() {
//             Ok(_) => Ok(()),
//             Err(_) => Err(()),
//         }
//     }
// }

// pub fn run_rust_script_with_ctrlc_handling() {
//     let explicit = {
//         let lock = EXPLICIT.lock().unwrap_or_else(|e| {
//             eprintln!("Failed to acquire lock: {}", e);
//             std::process::exit(1); // Exit the program if the lock cannot be obtained
//         });
//         lock.clone() // Clone the data to move it into the thread
//     };

//     let explicit_path = Path::new(&explicit); // Construct Path outside the lock

//     if explicit_path.exists() {
//         match is_active_rust_script(&explicit_path) {
//             Ok(true) => {}
//             Ok(false) | Err(_) => {
//                 // Handle the error locally without propagating it
//                 eprintln!("Failed to check if the file is a rust-script");
//                 std::process::exit(1); // Exit with an error code
//             }
//         }

//         let extra_args = EXTRA_ARGS.lock().unwrap(); // Locking the Mutex to access the data
//         let extra_str_slice: Vec<String> = extra_args.iter().cloned().collect();

//         // Run the child process in a separate thread to allow Ctrl+C handling
//         let handle = std::thread::spawn(move || {
//             let extra_str_slice_cloned = extra_str_slice.clone();
//             let child = e_runner::run_rust_script(
//                 &explicit,
//                 &extra_str_slice_cloned
//                     .iter()
//                     .map(String::as_str)
//                     .collect::<Vec<_>>(),
//             )
//             .unwrap_or_else(|| {
//                 eprintln!("Failed to run rust-script: {:?}", &explicit);
//                 std::process::exit(1); // Exit with an error code
//             });
//         });

//         // Wait for the thread to complete, but with a timeout
//         let timeout = std::time::Duration::from_secs(10);
//         match handle.join_timeout(timeout) {
//             Ok(_) => {
//                 println!("Child process finished successfully.");
//             }
//             Err(_) => {
//                 eprintln!("Child process took too long to finish. Exiting...");
//                 std::process::exit(1); // Exit if the process takes too long
//             }
//         }
//     }
// }

// fn handle_single_target(
//     manager: Arc<ProcessManager>,
//     cli: &Cli,
//     target: &CargoTarget,
//     unique_examples: &[CargoTarget],
//     builtin_examples: &[&CargoTarget],
//     builtin_binaries: &[&CargoTarget],
//     subcommand_provided_explicitly: bool,
// ) -> anyhow::Result<()> {
//     if subcommand_provided_explicitly {
//         println!("Subcommand provided explicitly. Running {}...", target.name);
//         cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
//         return Ok(());
//     }

//     if cli.run_at_a_time > 0 {
//         let timeout = std::time::Duration::from_secs(cli.run_at_a_time as u64);
//         let handle = std::thread::spawn(move || {
//             cargo_e::e_runner::run_example(manager.clone(), cli, target)
//         });

//         match handle.join_timeout(timeout) {
//             Ok(result) => result?,
//             Err(_) => {
//                 eprintln!("Timeout reached for target: {}", target.name);
//                 manager.kill_all();
//                 return Err(anyhow::anyhow!("Timeout reached for target: {}", target.name));
//             }
//         }
//     } else {
//         cargo_e::e_runner::run_example(manager.clone(), cli, target)?;
//     }

//     Ok(())
// }

// fn cli_loop(
//     manager: Arc<ProcessManager>,
//     cli: &Cli,
//     unique_examples: &[CargoTarget],
//     builtin_examples: &[&CargoTarget],
//     builtin_binaries: &[&CargoTarget],
// ) {
//     let mut current_offset = 0;
//     let manager_clone = manager.clone();

//     loop {
//         match select_and_run_target_loop(
//             manager_clone.clone(),
//             cli,
//             unique_examples,
//             builtin_examples,
//             builtin_binaries,
//             current_offset,
//         ) {
//             Ok(LoopResult::Quit) => {
//                 manager_clone.generate_report(cli.gist);
//                 println!("quitting.");
//                 break;
//             }
//             Ok(LoopResult::Run(status, new_offset)) => {
//                 current_offset = new_offset;

//                 if cli.run_at_a_time > 0 {
//                     let timeout = Duration::from_secs(cli.run_at_a_time as u64);
//                     let handle = std::thread::spawn(move || status);

//                     match handle.join_timeout(timeout) {
//                         Ok(_) => println!("Process finished successfully."),
//                         Err(_) => {
//                             eprintln!("Timeout reached. Killing all processes.");
//                             manager_clone.kill_all();
//                             break;
//                         }
//                     }
//                 } else {
//                     println!("Process finished with status: {:?}", status.code());
//                 }
//             }
//             Ok(LoopResult::Continue) => continue,
//             Err(err) => {
//                 eprintln!("Error: {:?}", err);
//                 std::process::exit(1);
//             }
//         }
//     }
// }
