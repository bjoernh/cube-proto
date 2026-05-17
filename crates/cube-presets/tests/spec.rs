//! Cross-reference test binary for the cube-presets crate.
//!
//! Cargo treats every `tests/*.rs` as a separate integration test binary but
//! does NOT auto-include nested files. We `#[path]`-include each spec file
//! here so the Wave 3 SDS cross-references run as part of `cargo test
//! -p cube-presets`.

#[path = "spec/sds_5_5_imports.rs"]
mod sds_5_5_imports;

#[path = "spec/sds_5_6_persistence.rs"]
mod sds_5_6_persistence;
