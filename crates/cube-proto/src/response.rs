//! Response envelope (SDS §5.3).
//!
//! `{"id": u64, "ok": bool, …}` where the remainder depends on `ok`:
//!   - `ok: true`  → optional `"result"` field (any JSON value)
//!   - `ok: false` → `"error"` field carrying a `CubeError`
//!
//! Responses are intentionally permissive (no `deny_unknown_fields`) so the
//! daemon can add forward-compatible fields.

use serde::{Deserialize, Serialize};

use crate::error::CubeError;

/// Full response message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    #[serde(flatten)]
    pub body: ResponseBody,
}

/// Body discriminated by which optional keys are present.
///
/// `Error` must be listed first in the untagged enum so that serde tries it
/// before `Result`. This ensures a JSON object with an `"error"` key decodes
/// as `Error` rather than `Result { result: None }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseBody {
    /// `ok: false` — carries an `"error"` object.
    Error { error: CubeError },
    /// `ok: true` — carries an optional `"result"` value.
    Result {
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
    },
}
