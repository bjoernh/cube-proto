//! Test binary that owns the `tests/spec/` cross-reference suite.
//!
//! Cargo treats every `tests/*.rs` as a separate integration test binary but
//! does NOT auto-include nested files. We `#[path]`-include each spec file
//! here so the spec cross-references run as part of `cargo test
//! -p cube-config`.

#[path = "spec/system_toml_spec.rs"]
mod system_toml_spec;
