// src/e_target.rs
use std::{
    ffi::OsString,
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub enum TargetOrigin {
    SingleFile(PathBuf),
    MultiFile(PathBuf),
    SubProject(PathBuf),
    Named(OsString),
}

#[derive(Debug, Clone,PartialEq)]
pub enum TargetKind {
    Example,
    Binary,
    Test,
    Manifest, // For browsing the entire Cargo.toml or package-level targets.
}

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub name: String,
    pub display_name: String,
    pub manifest_path: String,
    pub kind: TargetKind,
    pub extended: bool,
    pub origin: Option<TargetOrigin>,
}

