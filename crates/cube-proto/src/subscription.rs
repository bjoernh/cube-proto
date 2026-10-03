//! Subscribe response model: `SubscribeResult` + the baseline `Snapshot`.
//!
//! A `subscribe` OK response carries a [`SubscribeResult`] in `Response.result`:
//! the assigned `sub_id`, the `event_seq` baseline the stream starts after, and
//! — only when the request set `snapshot: true` — a [`Snapshot`] capturing the
//! current state of the subscribed classes.
//!
//! The snapshot schema is normative: only subscribed classes appear (each
//! section is omitted when its class was not subscribed), it is bounded by
//! `max_resident` (the lifecycle section never lists more sessions than the
//! daemon can hold resident), and it never carries parameter values — param
//! state is read via `get`/`get-all`, not the snapshot.

use serde::{Deserialize, Serialize};

use crate::event::PowerState;
use crate::input::ControllerInfo;
use crate::status::PauseCauses;

/// `result` body of a `subscribe` OK response.
///
/// Without `snapshot: true` the result carries `sub_id` + the `event_seq`
/// baseline only; the `snapshot` field is omitted from the wire when absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeResult {
    /// Daemon-assigned identifier for the new subscription.
    pub sub_id: String,
    /// The connection's reliable-event sequence at install time; every
    /// subsequent reliable event on this connection carries a higher value.
    pub event_seq: u64,
    /// Baseline state of the subscribed classes, present only when the request
    /// set `snapshot: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Snapshot>,
}

/// Baseline state captured at subscribe time. Only subscribed classes appear — each section is omitted when
/// its class was not part of the `events` filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifecycleSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<TelemetrySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brightness: Option<BrightnessSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerSnapshot>,
    /// `input_bindings` snapshot section (cube-gamepad "Tier 2"): the
    /// controller roster, present only when the `InputBindings` class was
    /// subscribed with `snapshot: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_bindings: Option<InputBindingsSnapshot>,
    /// `input_capture` snapshot section (cube-gamepad "Tier 2"): the buttons
    /// currently held at subscribe time, present only when the `InputCapture`
    /// class was subscribed with `snapshot: true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_capture: Option<InputCaptureSnapshot>,
}

/// `lifecycle` snapshot section. Bounded by `max_resident`:
/// `sessions` never exceeds the daemon's resident capacity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleSnapshot {
    pub max_resident: u32,
    pub resident_apps: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_app: Option<String>,
    pub sessions: Vec<SessionSnapshot>,
}

/// One resident session in a [`LifecycleSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub app: String,
    /// v6 lifecycle state string (`starting | focused | paused | stopping`).
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Monotonic focus stamp (the same value `status` reports).
    pub last_focused: u64,
    pub pause_causes: PauseCauses,
}

/// `telemetry` snapshot section: the current per-app frame
/// telemetry. Never carries parameter values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub app: String,
    pub fps: f32,
    pub drops: u64,
    pub frame_seq: u64,
}

/// `brightness` snapshot section: the current global brightness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrightnessSnapshot {
    pub value: u8,
}

/// `power` snapshot section: the current display power state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerSnapshot {
    pub state: PowerState,
}

/// `input_bindings` snapshot section (cube-gamepad "Tier 2"): the connected
/// controller roster — the same `[{ player, name, vid_pid, profile, connected }]`
/// shape the `input.controllers` verb returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputBindingsSnapshot {
    pub controllers: Vec<ControllerInfo>,
}

/// `input_capture` snapshot section (cube-gamepad "Tier 2"): the canonical
/// buttons held down at subscribe time, so a press-to-bind screen starts from
/// the right baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputCaptureSnapshot {
    pub held: Vec<HeldButton>,
}

/// One held canonical button in an [`InputCaptureSnapshot`]. `button` is the
/// canonical button config-name (`"A"`, `"DPadUp"`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldButton {
    pub player: u8,
    pub button: String,
}
