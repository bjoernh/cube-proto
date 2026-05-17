//! Preset TOML I/O for cubed.
//!
//! See SDS §5.5 (preset semantics, import warnings, schema_version) and
//! §5.6 (persistence: flock, write-temp-rename, fsync ordering).
//!
//! Wave 3 implementation will land in this crate; this stub exists so the
//! red tests can reference the module by name.

#![allow(dead_code)]
