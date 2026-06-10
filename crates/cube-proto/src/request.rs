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

/// `result` body of the `hello` OK response (SDS §5.3; SDS v6 delta §7).
///
/// Today `cubed` replies to a compatible `hello` with an empty
/// `{"id":..,"ok":true}` (no `result`) — see `cubed`'s
/// `control_plane::handshake::do_handshake_sync`, which calls
/// `send_ok_sync(fd, id)`. This type is the v6 wire shape for a richer OK
/// response that lets a client (e.g. `cubekit`) read back `cubed`'s
/// advertised protocol version and detect the v6 surface, per delta §7 /
/// cubekit spec §3.5. Wiring `cubed`'s handshake to actually send this body
/// is a daemon-wave change (see the `// TODO(sds-v6 2.1)` left in
/// `handshake.rs`); this crate defines the type + round-trips it now so both
/// sides share one source of truth ([`crate::PROTOCOL_VERSION`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloResult {
    /// `cubed`'s advertised protocol version, `"<major>.<minor>"`
    /// (see [`crate::PROTOCOL_VERSION`]).
    pub protocol_version: String,
    /// Optional surface/feature marker for forward-compat (e.g.
    /// `"v6"`). Absent on older daemons; clients that only check
    /// `protocol_version` minor can ignore this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
}

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

    #[serde(rename = "brightness.get")]
    BrightnessGet { id: u64 },

    #[serde(rename = "doctor.report")]
    DoctorReport { id: u64 },

    // ── Power (admin) ─────────────────────────────────────────────────────────
    /// Admin-only (SDS v6 §5.12): immediately blank the display, idempotent.
    /// `EBADREQ` on app connections — enforced by `cubed`, not this crate.
    #[serde(rename = "power.blank")]
    PowerBlank { id: u64 },

    /// Admin-only (SDS v6 §5.12): immediately wake the display, idempotent.
    /// `EBADREQ` on app connections — enforced by `cubed`, not this crate.
    #[serde(rename = "power.wake")]
    PowerWake { id: u64 },
}
