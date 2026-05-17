//! Test binary that owns the `tests/spec/` cross-reference suite.
//!
//! Cargo treats every `tests/*.rs` as a separate integration test binary but
//! does NOT auto-include nested files. We `#[path]`-include each spec file
//! here so the Wave 2 SDS cross-references run as part of `cargo test
//! -p cube-config`.

#[path = "spec/sds_6_4_system_toml.rs"]
mod sds_6_4_system_toml;
