//! Public types for cube-presets.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use cube_proto::ParamValue;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// PresetOrigin
// ─────────────────────────────────────────────────────────────────────────────

/// Whether a preset comes from the system (built-in) or the user directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetOrigin {
    BuiltIn,
    User,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetMeta
// ─────────────────────────────────────────────────────────────────────────────

/// `[meta]` section of a preset TOML file (SDS §5.5 format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetMeta {
    pub app: String,
    pub schema_version: u32,
    pub preset_name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub version: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetFile
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed preset TOML file with `[meta]` and `[params]` sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetFile {
    pub meta: PresetMeta,
    pub params: BTreeMap<String, ParamValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WarningCode
// ─────────────────────────────────────────────────────────────────────────────

/// Import warning codes (SDS §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    UnknownDropped,
    Clamped,
    ReadonlyDropped,
    NonShareableDropped,
}

// ─────────────────────────────────────────────────────────────────────────────
// ImportWarning
// ─────────────────────────────────────────────────────────────────────────────

/// A single warning emitted during import validation.
#[derive(Debug, Clone)]
pub struct ImportWarning {
    pub code: WarningCode,
    pub key: String,
    pub detail: BTreeMap<String, serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ImportReport
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a successful `import` operation.
#[derive(Debug, Clone)]
pub struct ImportReport {
    pub warnings: Vec<ImportWarning>,
    /// The final parameter map after all drops and clamps have been applied.
    pub final_params: BTreeMap<String, ParamValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SaveReport
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a successful `save` operation.
#[derive(Debug, Clone, Default)]
pub struct SaveReport {
    pub warnings: Vec<ImportWarning>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetError
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by `PresetStore` operations.
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("preset not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("schema version mismatch: expected {expected}, got {got}")]
    SchemaVersionMismatch { expected: u32, got: u32 },

    #[error("meta.app does not match the requested app")]
    AppMismatch,

    #[error("type mismatch: {0}")]
    TypeMismatch(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
