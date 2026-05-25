//! Command request envelope (SDS §5.3, §5.4, §5.5, §5.7.1, §5.10, §6.1).
//!
//! `Request` is an internally-tagged enum where the `"cmd"` field selects the
//! variant. Each variant carries its fields inline (struct variant).  The
//! `deny_unknown_fields` attribute on each variant struct (via the flattened
//! intermediate) enforces protocol strictness: extra fields are rejected.
//!
//! Note: serde internally-tagged enums with struct variants do propagate
//! `deny_unknown_fields` from the outer enum only in some situations. We place
//! it directly on the enum and rely on the fact that the struct-variant form
//! already handles this in serde's internally-tagged dispatch.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::value::{Damage, Format, ParamValue};

/// Top-level request envelope. Internally tagged on `"cmd"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", deny_unknown_fields)]
pub enum Request {
    // ── Session ───────────────────────────────────────────────────────────────
    #[serde(rename = "hello")]
    Hello { id: u64, protocol_version: String },

    #[serde(rename = "register")]
    Register {
        id: u64,
        name: String,
        mode: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_token: Option<String>,
    },

    // ── Lifecycle ─────────────────────────────────────────────────────────────
    #[serde(rename = "list")]
    List { id: u64 },

    #[serde(rename = "status")]
    Status {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },

    #[serde(rename = "launch")]
    Launch { id: u64, app: String },

    #[serde(rename = "stop")]
    Stop {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },

    #[serde(rename = "focus")]
    Focus { id: u64, app: String },

    #[serde(rename = "restart")]
    Restart { id: u64, app: String },

    #[serde(rename = "journal")]
    Journal {
        id: u64,
        app: String,
        follow: bool,
        tail: u32,
    },

    // ── Buffer / frame ────────────────────────────────────────────────────────
    #[serde(rename = "buffer.register")]
    BufferRegister {
        id: u64,
        buffer_id: u32,
        format: Format,
        width: u32,
        height: u32,
        stride: u32,
        size: u32,
    },

    #[serde(rename = "present")]
    Present {
        id: u64,
        seq: u64,
        buffer_id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        damage: Option<Damage>,
    },

    // ── Parameters ────────────────────────────────────────────────────────────
    #[serde(rename = "get")]
    Get { id: u64, app: String, key: String },

    #[serde(rename = "get-all")]
    GetAll { id: u64, app: String },

    #[serde(rename = "set")]
    Set {
        id: u64,
        app: String,
        key: String,
        value: ParamValue,
    },

    #[serde(rename = "set-many")]
    SetMany {
        id: u64,
        app: String,
        values: BTreeMap<String, ParamValue>,
    },

    #[serde(rename = "describe")]
    Describe {
        id: u64,
        app: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },

    #[serde(rename = "subscribe")]
    Subscribe { id: u64, app: String },

    #[serde(rename = "unsubscribe")]
    Unsubscribe { id: u64, app: String },

    // ── Presets ───────────────────────────────────────────────────────────────
    #[serde(rename = "preset.list")]
    PresetList { id: u64, app: String },

    #[serde(rename = "preset.load")]
    PresetLoad { id: u64, app: String, name: String },

    #[serde(rename = "preset.save")]
    PresetSave { id: u64, app: String, name: String },

    #[serde(rename = "preset.delete")]
    PresetDelete { id: u64, app: String, name: String },

    #[serde(rename = "preset.current")]
    PresetCurrent { id: u64, app: String },

    #[serde(rename = "preset.export")]
    PresetExport { id: u64, app: String, name: String },

    #[serde(rename = "preset.import")]
    PresetImport { id: u64, app: String, toml: String },

    // ── Brightness / diagnostics ──────────────────────────────────────────────
    #[serde(rename = "brightness.set")]
    BrightnessSet { id: u64, value: u8 },

    #[serde(rename = "doctor.report")]
    DoctorReport { id: u64 },
}
