//! Error types for the Cube protocol.
//!
//! `CubeErrno` is the closed set of error codes defined in SDS §5.3.
//! `CubeError` is the error body carried in a `Response` when `ok: false`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Closed set of error codes (SDS §5.3). Serializes to / from its uppercase
/// variant name (e.g. `EBADREQ`).  Unknown strings must fail to deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CubeErrno {
    EBADREQ,
    EUNKNOWN,
    ENOAPP,
    ENOKEY,
    ETYPE,
    ERANGE,
    EENUM,
    EREADONLY,
    EEXIST,
    ENOENT,
    EVERSION,
    ETIMEOUT,
    EBUSY,
    ENOTRUNNING,
}

/// Error body carried in `{"ok": false, "error": { … }}` responses (SDS §5.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct CubeError {
    pub code: CubeErrno,
    pub message: String,
    #[serde(default)]
    pub context: BTreeMap<String, serde_json::Value>,
}
