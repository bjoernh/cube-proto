//! Unsolicited event types (SDS §5.1, §5.3, §5.4, §5.7.1, §5.12, §6.1).
//!
//! `Event` is internally tagged on `"event"`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::value::ParamValue;

// ─────────────────────────────────────────────────────────────────────────────
// Nested enums
// ─────────────────────────────────────────────────────────────────────────────

/// Reason a buffer was released (SDS §5.1, §5.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseReason {
    Displayed,
    Dropped,
    Replaced,
    FocusLost,
    Blanked,
}

/// Display power state (SDS §5.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    Active,
    Blanked,
}

/// Reason a presented frame was dropped (SDS v5 §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentDroppedReason {
    TooLate,
    Fragmented,
    FocusLost,
    Blanked,
}

/// Who initiated a parameter change. Stamped onto every `param.changed`
/// and `params.changed` event so subscribers can route differently.
/// Defaults to [`ChangeSource::Unknown`] for forward-compat — older
/// daemons that have no `source` field will deserialize as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSource {
    Cubectl,
    Midi,
    Preset,
    App,
    #[default]
    Unknown,
}

impl ChangeSource {
    /// `true` when the source is the default placeholder. Used as
    /// `skip_serializing_if` on `ParamChanged`/`ParamsChanged` so the
    /// wire stays clean when the daemon has no specific attribution.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event enum
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level event. Internally tagged on `"event"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum Event {
    // ── Lifecycle ────────────────────────────────────────────────────────────
    #[serde(rename = "app.started")]
    AppStarted { app: String },

    #[serde(rename = "app.stopped")]
    AppStopped { app: String },

    // ── Frame / buffer ───────────────────────────────────────────────────────
    #[serde(rename = "present.displayed")]
    PresentDisplayed { seq: u64, buffer_id: u32 },

    #[serde(rename = "buffer.release")]
    BufferRelease {
        buffer_id: u32,
        seq: u64,
        reason: ReleaseReason,
    },

    // ── Parameter events ─────────────────────────────────────────────────────
    #[serde(rename = "param.changed")]
    ParamChanged {
        app: String,
        seq: u64,
        key: String,
        value: ParamValue,
        #[serde(default, skip_serializing_if = "ChangeSource::is_unknown")]
        source: ChangeSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preset: Option<String>,
    },

    #[serde(rename = "params.changed")]
    ParamsChanged {
        app: String,
        seq: u64,
        values: BTreeMap<String, ParamValue>,
        #[serde(default, skip_serializing_if = "ChangeSource::is_unknown")]
        source: ChangeSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preset: Option<String>,
    },

    #[serde(rename = "events.dropped")]
    EventsDropped { since_seq: u64 },

    // ── Input ────────────────────────────────────────────────────────────────
    #[serde(rename = "input.event")]
    InputEvent {
        input_seq: u64,
        t_us: i64,
        #[serde(rename = "type")]
        kind: String,
        code: String,
        value: i32,
    },

    #[serde(rename = "input.snapshot")]
    InputSnapshot {
        input_seq: u64,
        device: String,
        keys: BTreeMap<String, i32>,
        abs: BTreeMap<String, i32>,
    },

    #[serde(rename = "input.device_state")]
    InputDeviceState { device: String, connected: bool },

    #[serde(rename = "input.dropped")]
    InputDropped { since_seq: u64 },

    // ── Power / system ───────────────────────────────────────────────────────
    #[serde(rename = "power.state")]
    PowerState { state: PowerState },

    #[serde(rename = "config.reloaded")]
    ConfigReloaded,

    // ── Remote rendering ─────────────────────────────────────────────────────
    #[serde(rename = "frame_stream.bound")]
    FrameStreamBound {
        max_inflight: u32,
        policy: String,
        expected_payload_bytes: u32,
        mtu_hint: String,
    },

    #[serde(rename = "present.dropped")]
    PresentDropped { seq: u64, reason: PresentDroppedReason },

    #[serde(rename = "remote.frames_dropped")]
    RemoteFramesDropped { client: String, dropped: u64, since_seq: u64 },

    #[serde(rename = "client.stats")]
    ClientStats {
        client: String,
        remote_sender_dropped: u64,
        last_seq: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        video_latency_us: Option<u64>,
    },
}
