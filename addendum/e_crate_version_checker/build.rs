// extended/e_crate_version_checker/build.rs

fn main() {
    // For example, tell Cargo to rerun this build script if Cargo.toml changes,
    // or if any file in the "src" directory changes.
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/");
}
