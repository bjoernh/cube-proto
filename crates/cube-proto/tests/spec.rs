//! Test binary that owns the `tests/spec/` cross-reference suite.
//!
//! Cargo treats every `tests/*.rs` as a separate integration test binary but
//! does NOT auto-include nested files. We `#[path]`-include each spec file
//! here so the wave-1 SDS cross-references run as part of `cargo test
//! -p cube-proto`.

#[path = "spec/sds_6_1_wire_envelopes.rs"]
mod sds_6_1_wire_envelopes;
