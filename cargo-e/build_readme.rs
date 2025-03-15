use regex::Regex;
use std::fs;
use std::path::Path;

pub fn update_readme() {
    // Read Cargo.toml and extract the version string.
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let version_line = cargo_toml
        .lines()
        .find(|line| line.trim_start().starts_with("version"))
        .expect("Version not found in Cargo.toml");
    let version = version_line
        .split('=')
        .nth(1)
        .expect("Malformed version line")
        .trim()
        .trim_matches('"');
    println!("Found version: {}", version);

    // Read the local README.md.
    let readme = fs::read_to_string("README.md").expect("Failed to read README.md");

    // Regex matches a semantic version number only if it is surrounded by '>' and '<'
    let re = Regex::new(r">(?P<version>\d+\.\d+\.\d+)<").expect("Invalid regex");
    let new_readme = re.replace_all(&readme, |_: &regex::Captures| format!(">{}<", version));

    // Write the updated README.md back to the current directory.
    fs::write("README.md", new_readme.as_ref()).expect("Failed to write updated README.md locally");
    println!("Updated local README.md with version {}", version);

    // Also write the updated README.md to the parent directory.
    let parent_readme = Path::new("..").join("README.md");
    fs::write(&parent_readme, new_readme.as_ref())
        .expect("Failed to write updated README.md to parent directory");
    println!("Copied updated README.md to {:?}", parent_readme);
}
