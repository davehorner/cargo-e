// extended/e_crate_version_checker/build.rs

use std::io::Write;

fn main() {
    // For example, tell Cargo to rerun this build script if Cargo.toml changes,
    // or if any file in the "src" directory changes.
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/");
    // If "changelog" feature is enabled, set default changelog path if not overridden
    if std::env::var("CARGO_FEATURE_CHANGELOG").is_ok() && std::env::var("E_CRATE_CHANGELOG_PATH").is_err() {
            println!("cargo:rustc-env=E_CRATE_CHANGELOG_PATH=../cargo-e.CHANGELOG.md");
    }

    // If "fortune" feature is enabled, select external file or generate default fortunes
    if std::env::var("CARGO_FEATURE_FORTUNE").is_ok() {
        // Try external override
        let ext = std::env::var("E_CRATE_FORTUNE_PATH").ok();
        let valid_ext = ext.as_ref().and_then(|p| {
            let path = std::path::Path::new(p);
            if path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                Some(p.clone())
            } else {
                None
            }
        });
        // Determine final path: external if valid, else write defaults to OUT_DIR
        let final_path = if let Some(path) = valid_ext {
            path
        } else {
            let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
            let out_path = std::path::Path::new(&out_dir).join("fortunes.txt");
            let mut file = std::fs::File::create(&out_path)
                .expect("Failed to create default fortunes file in OUT_DIR");
            for line in default_fortunes() {
                writeln!(file, "{}", line).expect("Failed to write default fortune");
            }
            out_path.to_string_lossy().into_owned()
        };
        println!("cargo:rustc-env=E_CRATE_FORTUNE_PATH={}", final_path);
    }
}

/// Default fortunes used when genai integration is unavailable or for fallback
fn default_fortunes() -> Vec<String> {
    vec![
        String::from("Why do programmers prefer dark mode? Because light attracts bugs."),
        String::from("A SQL query walks into a bar, walks up to two tables and asks, 'Can I join you?'."),
        String::from("Why do Java developers wear glasses? Because they don't see sharp."),
        String::from("There are only 10 types of people in the world: those who understand binary and those who don't."),
        String::from("Debugging: Being the detective in a crime movie where you are also the murderer."),
        String::from("I would tell you a UDP joke, but you might not get it."),
        String::from("https://github.com/davehorner/cargo-e consider a star to show your support"),
        String::from("In a world without fences and walls, who needs Gates and Windows?"),
        String::from("'Knock, knock.' 'Who's there?' very long pause… 'Java.'"),
        String::from("Please consider giving this project a star on https://github.com/davehorner/cargo-e"),
    ]
}
