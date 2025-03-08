
use crate::prelude::*;
use crate::{ Example, TargetKind};

/// Runs `cargo run --bin` with the given manifest path and without specifying a binary name,
/// so that Cargo prints an error with a list of available binary targets.
/// Then parses that list to return a vector of Example instances, using the provided prefix.
pub fn collect_binaries(
    prefix: &str,
    manifest_path: &PathBuf,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    // Run `cargo run --bin --manifest-path <manifest_path>`.
    // Note: Cargo will return a non-zero exit code, but we only care about its stderr.
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let bin_names = crate::parse_available(&stderr_str, "binaries");

    // Map each binary name into an Example instance.
    let binaries = bin_names
        .into_iter()
        .map(|name| {
            let display_name = if prefix.starts_with('$') {
                format!("{} > binary > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin binary: {}", name)
            } else {
                name.clone()
            };

            Example {
                name: name.clone(),
                display_name: display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: TargetKind::Binary,
                extended: extended,
            }
        })
        .collect();

    Ok(binaries)
}

/// Runs `cargo run --example --manifest-path <manifest_path>` to trigger Cargo to
/// list available examples. Then it parses the stderr output using our generic parser.
pub fn collect_examples(
    prefix: &str,
    manifest_path: &PathBuf,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--example")
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    eprintln!("DEBUG: stderr (examples) = {:?}", stderr_str);

    let names = crate::parse_available(&stderr_str, "examples");
    eprintln!("DEBUG: example names = {:?}", names);

    let examples = names
        .into_iter()
        .map(|name| {
            // If the prefix starts with '$', we assume this came from a workspace member.
            let display_name = if prefix.starts_with('$') {
                format!("{} > example > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin example: {}", name)
            } else {
                name.clone()
            };

            Example {
                name: name.clone(),
                display_name: display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: TargetKind::Example,
                extended: extended,
            }
        })
        .collect();

    Ok(examples)
}


// --- Concurrent or sequential collection ---
pub fn collect_samples(
    manifest_infos: Vec<(String, PathBuf, bool)>,
    __max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let start_total = Instant::now();
    let mut all_samples = Vec::new();

    // "Before" message: starting collection
    println!("Timing: Starting sample collection...");

    #[cfg(feature = "concurrent")]
    {
        use threadpool::ThreadPool;
        let pool = ThreadPool::new(__max_concurrency);
        let (tx, rx) = mpsc::channel();

        let start_concurrent = Instant::now();
        for (prefix, manifest_path, extended) in manifest_infos {
            let tx = tx.clone();
            let prefix_clone = prefix.clone();
            let manifest_clone = manifest_path.clone();
            pool.execute(move || {
                let mut results = Vec::new();
                if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut ex);
                }
                if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut bins);
                }
                tx.send(results).expect("Failed to send results");
            });
        }
        drop(tx);
        pool.join(); // Wait for all tasks to finish.
        let duration_concurrent = start_concurrent.elapsed();
        println!(
            "Timing: Concurrent processing took {:?}",
            duration_concurrent
        );

        for samples in rx {
            all_samples.extend(samples);
        }
    }

    #[cfg(not(feature = "concurrent"))]
    {
        // Sequential fallback: process one manifest at a time.
        let start_seq = Instant::now();
        for (prefix, manifest_path,extended) in manifest_infos {
            if let Ok(mut ex) = collect_examples(&prefix, &manifest_path, extended) {
                all_samples.append(&mut ex);
            }
            if let Ok(mut bins) = collect_binaries(&prefix, &manifest_path, extended) {
                all_samples.append(&mut bins);
            }
        }
        let duration_seq = start_seq.elapsed();
        println!("Timing: Sequential processing took {:?}", duration_seq);
    }

    let total_duration = start_total.elapsed();
    println!("Timing: Total collection time: {:?}", total_duration);
    Ok(all_samples)
}
/// This function collects sample targets (examples and binaries) from both the current directory
/// and, if the --workspace flag is used, from each workspace member. The built–in samples (from
/// the current directory) are tagged with a "builtin" prefix, while workspace member samples are
/// tagged with "$member" so that the display name becomes "$member > example > sample_name" or
/// "$member > binary > sample_name".
pub fn collect_all_samples(
    use_workspace: bool,
    max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let mut manifest_infos: Vec<(String, PathBuf, bool)> = Vec::new();
    let cwd = env::current_dir()?;
    // Built-in samples: if there is a Cargo.toml in cwd, add it.
    let built_in_manifest = cwd.join("Cargo.toml");
    if built_in_manifest.exists() {
        println!(
            "Found built-in Cargo.toml in current directory: {}",
            cwd.display()
        );
        // For built-in samples, we use a fixed prefix.
        manifest_infos.push(("builtin".to_string(), built_in_manifest, false));
    } else {
        eprintln!("No Cargo.toml found in current directory for built-in samples.");
    }

    // If workspace flag is used, locate the workspace root and then collect all member manifests.
    if use_workspace {
        let ws_manifest = crate::locate_manifest(true)?;
        println!("Workspace root manifest: {}", ws_manifest);
        let ws_members = crate::collect_workspace_members(&ws_manifest)?;
        for (member_name, manifest_path) in ws_members {
            // The prefix for workspace samples is formatted as "$member_name"
            manifest_infos.push((format!("${}", member_name), manifest_path, false));
        }
    }

    // Also, extended samples: assume they live in an "examples" folder relative to cwd.
    let extended_root = cwd.join("examples");
    if extended_root.exists() {
        for entry in fs::read_dir(&extended_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("Cargo.toml").exists() {
                // Use the directory name as the prefix.
                let prefix = path.file_name().unwrap().to_string_lossy().to_string();
                let manifest_path = path.join("Cargo.toml");
                manifest_infos.push((prefix, manifest_path, true));
            }
        }
    } else {
        eprintln!(
            "Extended samples directory {:?} does not exist.",
            extended_root
        );
    }

    eprintln!("DEBUG: manifest infos: {:?}", manifest_infos);

    // Now, use either concurrent or sequential collection.
    // Here we assume a function similar to our earlier collect_samples_concurrently.
    // We reuse our previously defined collect_samples function, which now accepts a Vec<(String, PathBuf, bool)>.
    let samples = collect_samples(manifest_infos, max_concurrency)?;
    Ok(samples)
}
