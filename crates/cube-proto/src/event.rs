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
    },

    #[serde(rename = "params.changed")]
    ParamsChanged {
        app: String,
        seq: u64,
        values: BTreeMap<String, ParamValue>,
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
}
