// build.rs

use std::env;
use std::path::Path;

// Pull in our custom build modules.
mod build_addendum_utils;
mod build_docs;

fn main() {
    // For example, when inlining, have your build script print:
    println!("cargo:rustc-cfg=inlined");

    // --- Documentation Copying ---
    // Call our documentation helper to copy media files.
    build_docs::copy_doc_media();

    // --- Addendum Code Inlining ---
    // Determine the addendum source directory.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let src_dir = Path::new(&manifest_dir)
        .join("addendum")
        .join("e_crate_version_checker")
        .join("src");

    // Write the generated code to a file in OUT_DIR.
    // Generate module declarations to inline all .rs files in the addendum directory.
    let out_dir_str = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_dir = Path::new(&out_dir_str);
    let generated_code = build_addendum_utils::generate_module_includes(&src_dir, &out_dir)
        .expect("Failed to generate module includes");
    let dest_path = Path::new(&out_dir).join("generated_e_crate_version_checker.rs");
    println!("cargo:warning=Writing generated file to {:?}", dest_path);
    let generated = std::fs::read_to_string(&dest_path).expect("Failed to read the generated file");
    println!("cargo:warning=Generated file content:\n{}", generated);
    if generated_code.is_empty() {
        println!("cargo:warning=No addendum files found in {:?}", src_dir);
        std::process::exit(1);
    }
    build_addendum_utils::write_generated_file(&dest_path, &generated_code)
        .expect("Failed to write generated file");

    // Re-run build if any addendum source file changes.
    println!("cargo:rerun-if-changed=addendum/e_crate_version_checker/src/");
}
