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

use crate::input::BindingScope;
use crate::value::{Color, Damage, Format, ParamValue};

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

/// Transition **kind** requested on a focus change (SDS v7 §5.13, §6.1).
///
/// `cut` | `crossfade` plus the crossfade-family effects `dissolve` |
/// `dip_to_black` | `particle_dissolve` | `push` — all the same machinery with a
/// different per-pixel sample arm (added without a protocol change). `cut` (or
/// `duration_ms = 0`) is the hard-cut opt-out that reproduces v6's abrupt swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    /// Hard cut — zero intermediate frames (the v6 behaviour).
    Cut,
    /// Per-channel crossfade between the outgoing and incoming images.
    Crossfade,
    /// Whole-pixel random fizzle reveal of the incoming image (`dissolve`).
    Dissolve,
    /// Fade the outgoing image to black, then up to the incoming image
    /// (`dip_to_black`).
    DipToBlack,
    /// Vertical particle curtain — the incoming image rains in from the top
    /// (`particle_dissolve`).
    ParticleDissolve,
    /// Vertical slide — the incoming image pushes in from the top, displacing
    /// the outgoing image out the bottom (`push`).
    Push,
}

/// Animated transition requested by the client that initiates a focus change,
/// carried on `launch` / `focus` (SDS v7 §5.13, §6.1). Optional on both verbs;
/// absent ⇒ `cubed` applies its configured default. `duration_ms` is clamped to
/// a sane range by `cubed` and omitted on the wire when unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionSpec {
    /// `cut` | `crossfade` (§6.1).
    pub kind: TransitionKind,
    /// Transition window in milliseconds. Absent ⇒ `cubed`'s configured
    /// default; `0` is equivalent to `cut`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
}

/// Input mode requested at `overlay.acquire` (SDS v7 §5.13, §6.1, §5.7.1).
///
/// `none` is a purely visual overlay — input continues to the base app; `modal`
/// pushes an input grab so non-reserved controller events route to the overlay
/// owner while it is shown (§5.7.1). Serializes to/from `"none"` | `"modal"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayInputMode {
    /// Purely visual — input continues to the focused base app.
    None,
    /// Input is routed to the overlay owner while the overlay is shown.
    Modal,
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
    Launch {
        id: u64,
        app: String,
        /// Optional transition for the focus change this launch causes
        /// (SDS v7 §5.13, §6.1). Absent ⇒ `cubed`'s configured default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transition: Option<TransitionSpec>,
    },

    #[serde(rename = "stop")]
    Stop {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },

    #[serde(rename = "focus")]
    Focus {
        id: u64,
        app: String,
        /// Optional transition for this focus change (SDS v7 §5.13, §6.1).
        /// Absent ⇒ `cubed`'s configured default; ignored if the visible base
        /// does not actually change.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        transition: Option<TransitionSpec>,
    },

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
        /// Which compositor layer this buffer updates (SDS v7 §5.13, §6.1).
        /// Absent or `0` is the connection's **base** layer (the v6 behaviour);
        /// a `layer` id returned by `overlay.acquire` targets that overlay layer.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layer: Option<u32>,
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

    /// Subscribe to an event stream (SDS v6 §5.13 "The commands").
    ///
    /// The legacy v5 param-watch shape `{id, app}` round-trips byte-identically:
    /// `app` defaults to `"*"` on parse and is always re-serialized, while every
    /// §5.13 field is omitted from the wire when unset. New fields:
    /// `events` (class filter), `interval_ms` (telemetry coalescing cadence),
    /// `snapshot` (request a baseline [`crate::SubscribeResult`] snapshot),
    /// `max_events` / `timeout_ms` (bounded subscriptions).
    #[serde(rename = "subscribe")]
    Subscribe {
        id: u64,
        #[serde(default = "subscribe_app_default")]
        app: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        events: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interval_ms: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_events: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },

    /// Cancel a subscription (SDS v6 §5.13 "The commands").
    ///
    /// Targets one subscription by `sub_id`, all of a connection's
    /// subscriptions with `all: true`, or — for the legacy v5 param-watch shape
    /// `{id, app}` — a single app's param stream. `all` is omitted on the wire
    /// when `false`; `app`/`sub_id` are omitted when absent.
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sub_id: Option<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        all: bool,
    },

    /// List this connection's active subscriptions (SDS v6 §5.13
    /// "The commands"): an introspection verb for debugging the subscription
    /// surface.
    #[serde(rename = "subscriptions")]
    Subscriptions { id: u64 },

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

    // ── System text overlay (admin peers only — SDS v7 §5.13, §6.1) ───────────
    /// Render a line of text into a system overlay layer (SDS v7 §6.1). No
    /// buffer / `SCM_RIGHTS` — `cubed` renders the text itself through its glyph
    /// renderer, so it works on `/run/cube/admin` (STREAM). Admin-only:
    /// `EBADREQ` on app connections (like `power.blank`) — enforced by `cubed`.
    ///
    /// `duration_ms` auto-hides after the interval (`0` = sticky until
    /// `overlay.clear` or the next `overlay.text` on the same `z`); `z`
    /// (default 1) and `color` (default white) are optional.
    #[serde(rename = "overlay.text")]
    OverlayText {
        id: u64,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        z: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<Color>,
    },

    /// Remove a system text overlay (SDS v7 §6.1). `z` selects the layer to
    /// clear; omitted clears all admin text overlays. Admin-only — `EBADREQ` on
    /// app connections.
    #[serde(rename = "overlay.clear")]
    OverlayClear {
        id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        z: Option<i32>,
    },

    // ── Client overlays (apps / launcher on /run/cube/ctl — SDS v7 §5.13, §6.1) ─
    /// Acquire a client overlay layer (SDS v7 §5.13, §6.1). The OK `result`
    /// carries the assigned `{layer}` id, used on a subsequent `present {layer}`
    /// and on `overlay.release`. `z` (default 1) is the stacking order among
    /// overlays; `input` (default `none`) is the input mode — `modal` grabs
    /// controller input while the overlay is shown (§5.7.1). A client without
    /// overlay capability is rejected with `EPERM`.
    #[serde(rename = "overlay.acquire")]
    OverlayAcquire {
        id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        z: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<OverlayInputMode>,
    },

    /// Release a client overlay layer acquired with `overlay.acquire`
    /// (SDS v7 §5.13, §6.1). `layer` is the id returned by `overlay.acquire`.
    #[serde(rename = "overlay.release")]
    OverlayRelease { id: u64, layer: u32 },

    // ── Tier-2 gamepad bindings / tuning (cube-gamepad "Tier 2") ──────────────
    /// List the connected-controller roster (cube-gamepad: `input.controllers`).
    /// The OK `result` is a `[`[`ControllerInfo`](crate::ControllerInfo)`]`
    /// array `[{ player, name, vid_pid, profile, connected }]`.
    #[serde(rename = "input.controllers")]
    InputControllers { id: u64 },

    /// Read a controller's bindings + tuning for one scope (cube-gamepad
    /// "Admin verbs"). `vid_pid` is the lower-case `"vvvv:pppp"` USB identity.
    #[serde(rename = "input.bindings.get")]
    InputBindingsGet {
        id: u64,
        vid_pid: String,
        scope: BindingScope,
    },

    /// Set one binding for `(vid_pid, scope)`: remap canonical button `physical`
    /// to `action` (cube-gamepad "Admin verbs"). `physical`/`action` are
    /// canonical button config-names (`"A"`, `"ShoulderLeft"`, …); `action` may
    /// be the `"unbound"` sentinel to drop the event. Reserved keys are never
    /// bindable (rejected by `cubed`, not this crate).
    #[serde(rename = "input.bindings.set")]
    InputBindingsSet {
        id: u64,
        vid_pid: String,
        physical: String,
        action: String,
        scope: BindingScope,
    },

    /// Reset `(vid_pid, scope)` to the profile's 1:1 default (cube-gamepad
    /// "Admin verbs"): clears that scope's bindings.
    #[serde(rename = "input.bindings.reset")]
    InputBindingsReset {
        id: u64,
        vid_pid: String,
        scope: BindingScope,
    },

    /// Set a tuning override for a controller (cube-gamepad "Admin verbs"). Each
    /// of `dead_zone` / `stick_dpad_threshold` / `invert` is an optional partial
    /// set — an omitted field leaves the prior value untouched. `scope` is
    /// optional and defaults to [`BindingScope::Global`] at the daemon.
    #[serde(rename = "input.tuning.set")]
    InputTuningSet {
        id: u64,
        vid_pid: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dead_zone: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stick_dpad_threshold: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        invert: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<BindingScope>,
    },
}

/// Default `app` selector for [`Request::Subscribe`] (SDS v6 §5.13): a client
/// that omits `app` subscribes to global event classes across all apps.
fn subscribe_app_default() -> String {
    "*".to_owned()
}

/// `skip_serializing_if` helper: keep `false` booleans off the wire so the
/// legacy `{id, app}` unsubscribe shape round-trips byte-identically.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}
