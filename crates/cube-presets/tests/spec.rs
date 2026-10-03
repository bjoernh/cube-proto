//! Cross-reference test binary for the cube-presets crate.
//!
//! Cargo treats every `tests/*.rs` as a separate integration test binary but
//! does NOT auto-include nested files. We `#[path]`-include each spec file
//! here so the spec cross-references run as part of `cargo test
//! -p cube-presets`.

#[path = "spec/imports_spec.rs"]
mod imports_spec;

#[path = "spec/persistence_spec.rs"]
mod persistence_spec;
