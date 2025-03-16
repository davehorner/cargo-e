#[test]
fn test_readme_project() {
    version_sync::assert_markdown_deps_updated!("README.md");
}
#[test]
fn test_readme_toplevel() {
    version_sync::assert_markdown_deps_updated!("../README.md");
}

#[test]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
