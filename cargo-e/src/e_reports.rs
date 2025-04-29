use crate::e_cargocommand_ext::CargoProcessResult;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use git2::{Error, Repository};
use std::fs::File;
use std::io::{self, Write};
use std::process::Command;

fn current_remote_and_short_sha() -> Result<(String, String, String), Error> {
    let repo = Repository::discover(".")?;

    // Get HEAD OID and shorten to 7 chars
    let oid = repo
        .head()?
        .target()
        .ok_or_else(|| Error::from_str("no HEAD target"))?;
    let short = repo
        .find_object(oid, None)?
        .short_id()?
        .as_str()
        .ok_or_else(|| Error::from_str("invalid short id"))?
        .to_string();

    // Look up the "origin" remote URL
    let remote = repo
        .find_remote("origin")
        .or_else(|_| {
            repo.remotes()?
                .get(0)
                .and_then(|name| repo.find_remote(name).ok())
                .ok_or_else(|| Error::from_str("no remotes configured"))
        })?
        .url()
        .unwrap_or_default()
        .to_string();

    // Extract the repository name from the remote URL
    let repo_name = remote
        .split('/')
        .last()
        .and_then(|name| name.strip_suffix(".git"))
        .unwrap_or("Unknown")
        .to_string();

    Ok((remote, short, repo_name))
}

pub fn generate_comfy_report(results: &[CargoProcessResult]) -> String {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let system_name = sysinfo::System::name().unwrap_or_else(|| "Unknown".to_string());
    let system_version = sysinfo::System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let system_long_version =
        sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string());

    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let (repo_remote, repo_sha, repo_name) = current_remote_and_short_sha().unwrap_or_else(|_| {
        (
            "Unknown".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
        )
    });

    let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Metadata Table
    let mut metadata_table = Table::new();
    metadata_table.set_content_arrangement(ContentArrangement::Dynamic);
    metadata_table.set_width(80);
    metadata_table.add_row(Row::from(vec![
        Cell::new("Run Report"),
        Cell::new(format!("{} ({} {})", repo_name, repo_remote, repo_sha)),
    ]));
    metadata_table.add_row(Row::from(vec![
        Cell::new("generated on"),
        Cell::new(current_time),
    ]));
    metadata_table.add_row(Row::from(vec![
        Cell::new("cargo-e version"),
        Cell::new(env!("CARGO_PKG_VERSION")),
    ]));
    metadata_table.add_row(Row::from(vec![
        Cell::new("rustc version"),
        Cell::new(rustc_version),
    ]));
    metadata_table.add_row(Row::from(vec![
        Cell::new("system info"),
        Cell::new(format!(
            "{} - {} - {}",
            system_name, system_version, system_long_version
        )),
    ]));

    let mut report = metadata_table.to_string();
    report.push_str("\n\n");

    // Results Table
    let mut cnt = 0;
    for result in results {
        cnt += 1;
        let mut result_table = Table::new();
        result_table.set_content_arrangement(ContentArrangement::Dynamic);
        result_table.set_width(100);

        let start_time = result
            .start_time
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        let end_time = result
            .end_time
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        let duration = result
            .elapsed_time
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "-".to_string());
        let exit_code = result.exit_status.map_or("-".to_string(), |s| {
            s.code().map_or("-".to_string(), |c| c.to_string())
        });
        let success = result
            .exit_status
            .map_or("No", |s| if s.success() { "Yes" } else { "No" });

        report.push_str(&format!("## {}. {}\n\n", cnt, result.target_name));
        report.push_str(&format!("{} {}\n", result.cmd, result.args.join(" ")));
        result_table.add_row(Row::from(vec![
            Cell::new(result.target_name.clone()),
            // Cell::new(format!("{} {}", result.cmd, result.args.join(" "))),
        ]));
        result_table.add_row(Row::from(vec![
            Cell::new("Start Time"),
            Cell::new(start_time),
        ]));
        result_table.add_row(Row::from(vec![Cell::new("End Time"), Cell::new(end_time)]));
        result_table.add_row(Row::from(vec![Cell::new("Duration"), Cell::new(duration)]));
        result_table.add_row(Row::from(vec![
            Cell::new("Exit Code"),
            Cell::new(exit_code),
        ]));
        result_table.add_row(Row::from(vec![Cell::new("Success"), Cell::new(success)]));
        report.push_str(&result_table.to_string());
        report.push_str("\n\n");

        // Diagnostics Table
        if !result.diagnostics.is_empty() {
            let mut diagnostics_table = Table::new();
            diagnostics_table.set_content_arrangement(ContentArrangement::Dynamic);
            diagnostics_table.set_width(100);

            diagnostics_table.add_row(Row::from(vec![
                Cell::new("Level"),
                Cell::new("Lineref"),
                Cell::new("Error Code"),
                Cell::new("Message"),
            ]));

            for diagnostic in &result.diagnostics {
                diagnostics_table.add_row(Row::from(vec![
                    Cell::new(format!(
                        "{}{}",
                        diagnostic
                            .level
                            .chars()
                            .next()
                            .unwrap_or_default()
                            .to_uppercase(),
                        diagnostic.diag_number.unwrap_or_default()
                    )),
                    Cell::new(&diagnostic.lineref),
                    Cell::new(
                        diagnostic
                            .error_code
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::new(&diagnostic.message),
                ]));
            }
            report.push_str(&diagnostics_table.to_string());
            report.push_str("\n");

            // Add a row with the full debug print of the diagnostic
            let mut detail_table = Table::new();
            detail_table.set_content_arrangement(ContentArrangement::Dynamic);
            detail_table.set_width(100);
            for diagnostic in &result.diagnostics {
                let mut nocolor = diagnostic.clone();
                nocolor.uses_color = false; // Disable color for detailed output
                detail_table.add_row(Row::from(vec![Cell::new(format!("{:?}", nocolor))]));
            }

            report.push_str(&detail_table.to_string());
            report.push_str("\n\n");
        }
    }

    report
}

pub fn generate_markdown_report(results: &[CargoProcessResult]) -> String {
    return generate_comfy_report(results);
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let system_name = sysinfo::System::name().unwrap_or_else(|| "Unknown".to_string());
    let system_version = sysinfo::System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let system_long_version =
        sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string());

    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let (repo_remote, repo_sha, repo_name) = current_remote_and_short_sha().unwrap_or_else(|_| {
        (
            "Unknown".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
        )
    });

    let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Create the main table
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_width(100);

    // Add metadata rows
    table.add_row(Row::from(vec![
        Cell::new("Run Report"),
        Cell::new(format!("{} ({}@{})", repo_name, repo_remote, repo_sha)),
    ]));
    table.add_row(Row::from(vec![
        Cell::new("Generated On"),
        Cell::new(current_time),
    ]));
    table.add_row(Row::from(vec![
        Cell::new("Cargo-e Version"),
        Cell::new(env!("CARGO_PKG_VERSION")),
    ]));
    table.add_row(Row::from(vec![
        Cell::new("Rustc Version"),
        Cell::new(rustc_version),
    ]));
    table.add_row(Row::from(vec![
        Cell::new("System Info"),
        Cell::new(format!(
            "{} - {} - {}",
            system_name, system_version, system_long_version
        )),
    ]));

    // Add a separator row
    table.add_row(Row::from(vec![Cell::new(""), Cell::new("")]));

    // Add results for each target
    for (index, result) in results.iter().enumerate() {
        let start_time = result
            .start_time
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        let end_time = result
            .end_time
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        let duration = result
            .elapsed_time
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "-".to_string());
        let exit_code = result.exit_status.map_or("-".to_string(), |s| {
            s.code().map_or("-".to_string(), |c| c.to_string())
        });
        let success = result
            .exit_status
            .map_or("No", |s| if s.success() { "Yes" } else { "No" });

        table.add_row(Row::from(vec![
            Cell::new(format!("{}. {}", index + 1, result.target_name)),
            Cell::new(""),
        ]));
        table.add_row(Row::from(vec![
            Cell::new("Start Time"),
            Cell::new(start_time),
        ]));
        table.add_row(Row::from(vec![Cell::new("End Time"), Cell::new(end_time)]));
        table.add_row(Row::from(vec![Cell::new("Duration"), Cell::new(duration)]));
        table.add_row(Row::from(vec![
            Cell::new("Exit Code"),
            Cell::new(exit_code),
        ]));
        table.add_row(Row::from(vec![Cell::new("Success"), Cell::new(success)]));

        // Add diagnostics if available
        if !result.diagnostics.is_empty() {
            table.add_row(Row::from(vec![Cell::new("Diagnostics"), Cell::new("")]));
            for diagnostic in &result.diagnostics {
                table.add_row(Row::from(vec![
                    Cell::new(format!("Level: {}", diagnostic.level)),
                    Cell::new(&diagnostic.message),
                ]));
            }
        }

        // Add a separator row
        table.add_row(Row::from(vec![Cell::new(""), Cell::new("")]));
    }

    // Convert the table to a string
    table.to_string()
}

// pub fn generate_markdown_report(results: &[CargoProcessResult]) -> String {
//         let mut system = sysinfo::System::new_all();
//     system.refresh_all();
//     let system_name = sysinfo::System::name().unwrap_or_else(|| "Unknown".to_string());
//     let system_long_version = sysinfo::System::long_os_version().unwrap_or_else(|| "Unknown".to_string());
//     let system_version = sysinfo::System::os_version().unwrap_or_else(|| "Unknown".to_string());

//         let rustc_version = Command::new("rustc")
//         .arg("--version")
//         .output()
//         .ok()
//         .and_then(|output| {
//             if output.status.success() {
//                 Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
//             } else {
//                 None
//             }
//         })
//         .unwrap_or_else(|| "Unknown".to_string());
//         let (repo_remote, repo_sha, repo_name) = current_remote_and_short_sha()
//         .unwrap_or_else(|_| ("-".to_string(), "-".to_string(), "-".to_string()));
//     let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
//         let mut report = String::from(format!(
//         "# Run Report for {} ({}@{})\n\nGenerated on: {}\n\n",
//         repo_name, repo_remote, repo_sha, current_time
//     ));
//     report.push_str(&format!("**repo info** {} @ {}\n", repo_remote, repo_sha));
//     report.push_str(&format!("**rustc version** {}\n", rustc_version));
//     report.push_str(&format!("**system name** {} - {} - {}\n", system_name, system_version, system_long_version));
//     report.push_str(&format!("**cargo-e version** {}\n", env!("CARGO_PKG_VERSION")));
//     report.push_str("\n");
//     let mut cnt = 0;
//     for result in results {
//         cnt=cnt + 1;
//     report.push_str(&format!("## {}. {}\n\n", cnt, result.target_name));
//     report.push_str("| Target Name | Start Time | End Time | Duration | Exit Code | Success |\n");
//     report.push_str("|-------------|------------|----------|----------|-----------|---------|\n");

//         let start_time = result
//             .start_time
//             .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%H:%M:%S").to_string())
//             .unwrap_or_else(|| "-".to_string());
//         let end_time = result
//             .end_time
//             .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%H:%M:%S").to_string())
//             .unwrap_or_else(|| "-".to_string());
//         let duration = result
//             .elapsed_time
//             .map(|d| format!("{:.2?}", d))
//             .unwrap_or_else(|| "-".to_string());
//         let exit_code = result.exit_status.map_or("-".to_string(), |s| s.code().map_or("-".to_string(), |c| c.to_string()));
//         let success = result.exit_status.map_or("No", |s| if s.success() { "Yes" } else { "No" });

//         report.push_str(&format!(
//             "| {} | {} | {} | {} | {} | {} |\n",
//             result.target_name, start_time, end_time, duration, exit_code, success
//         ));

//         if !result.diagnostics.is_empty() {
//             report.push_str("\n| Level | Message                            |\n");
//             report.push_str("|----------------------------------------------|\n");
//             for diagnostic in &result.diagnostics {
//                 report.push_str(&format!("| **{}** | {} |\n",diagnostic.level, diagnostic.message));
//             }
//             report.push_str("|----------------------------------------------------------------------|\n");
//         }
//         report.push_str("\n");
//     }

//     report
// }

pub fn save_report_to_file(report: &str, file_path: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    file.write_all(report.as_bytes())?;
    Ok(())
}

pub fn create_gist(content: &str, description: &str) -> io::Result<()> {
    let output = Command::new("gh")
        .args(&["gist", "create", "--public", "--desc", description])
        .stdin(std::process::Stdio::piped())
        .spawn()?
        .stdin
        .as_mut()
        .unwrap()
        .write_all(content.as_bytes());

    output.map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to create gist: {}", e),
        )
    })
}
