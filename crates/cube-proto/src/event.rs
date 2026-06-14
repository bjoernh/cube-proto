//! Unsolicited event types (SDS §5.1, §5.3, §5.4, §5.7.1, §5.12, §6.1).
//!
//! `Event` is internally tagged on `"event"`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::status::PauseCauses;
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

/// Why a bounded subscription was terminated by the daemon (SDS v6 §5.13
/// "Bounded subscriptions"). Delivered as the `reason` of the final
/// [`Event::SubscriptionEnded`] event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionEndReason {
    /// The `max_events` budget was exhausted.
    MaxEvents,
    /// The `timeout_ms` deadline elapsed.
    Timeout,
    /// The per-connection outbox overflowed (back-pressure shed, §5.13).
    Overflow,
}

/// Reason a presented frame was dropped (SDS v5 §6.2; SDS v6 §6.2).
///
/// v6 §6.2 pins the remote `present.dropped` vocabulary to exactly
/// `too_late | fragmented | focus_lost`. `Blanked` is retained here because
/// the local `buffer.release` event (SDS §5.1, §5.12) still carries
/// `reason:"blanked"` and shares this enum's wire representation; v6 does
/// not remove `blanked` from `buffer.release`, only from the `present.dropped`
/// remote vocabulary. No `present.dropped {reason:"blanked"}` literal is
/// constructed by `cubed` (blanking does not affect remote-drop accounting),
/// so keeping the variant here is additive and harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentDroppedReason {
    TooLate,
    Fragmented,
    FocusLost,
    Blanked,
}

/// Reason `focus.lost` was emitted (SDS v6 §5.2, §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusLostReason {
    /// Another app was launched or focused, displacing this one.
    AppSwitch,
    /// The Home key returned focus to the launcher.
    Home,
    /// Emitted on the stop path before the stop sequence proceeds
    /// (pause-before-stop, SDS v6 §5.2).
    Stopping,
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
    /// `event_seq` is the per-connection reliable-event sequence stamp added in
    /// SDS v6 §5.13. Omitted on the wire (and `None`) for v5 daemons, so the
    /// `{"event":"app.started","app":"snake"}` shape is byte-preserved.
    #[serde(rename = "app.started")]
    AppStarted {
        app: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_seq: Option<u64>,
    },

    /// `reason` is a free-form wire vocabulary string (SDS v6 §6.1, §6.2).
    /// Known values include `"normal"`, `"failed"`, `"register_timeout"`,
    /// `"replaced"`, `"control_lost"`, `"frame_handshake_timeout"`,
    /// `"frame_stream_lost"`, `"frame_stream_idle"`, and `"evicted"`
    /// (LRU eviction, SDS v6 §5.13). Older daemons that omit `reason`
    /// deserialize as `None`; the field is omitted on the wire when absent so
    /// v5 clients see the unchanged `{"event":"app.stopped","app":"snake"}`
    /// shape. `event_seq` is the SDS v6 §5.13 reliable-event stamp (omitted
    /// when `None`).
    #[serde(rename = "app.stopped")]
    AppStopped {
        app: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_seq: Option<u64>,
    },

    /// Resident-session state transition (SDS v6 §5.13 "Event payloads —
    /// lifecycle"). `from`/`to` are the v6 lifecycle state strings
    /// (`starting | focused | paused | stopping`). `cause` is present only on
    /// focus/blank pause edges and omitted otherwise. `event_seq` is the
    /// reliable-event stamp.
    #[serde(rename = "app.state")]
    AppState {
        app: String,
        from: String,
        to: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cause: Option<PauseCauses>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_seq: Option<u64>,
    },

    /// Focus moved between resident apps (SDS v6 §5.13 "Event payloads —
    /// lifecycle"). `focused_app` is absent when focus drops to nothing;
    /// `previous` is absent for the first focus after boot. `event_seq` is the
    /// reliable-event stamp.
    #[serde(rename = "focus.changed")]
    FocusChanged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        focused_app: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_seq: Option<u64>,
    },

    /// Reliable-tier (SDS v6 §5.3 classification): a non-reading app trips
    /// the existing fail-loud disconnect policy. Emitted on the app's own
    /// connection on focus loss (SDS v6 §5.2, §6.1).
    #[serde(rename = "focus.lost")]
    FocusLost { reason: FocusLostReason },

    /// Reliable-tier (SDS v6 §5.3 classification), see [`Event::FocusLost`].
    /// Always followed by `input.snapshot` before any `input.event`
    /// (SDS v6 §5.2, §6.1).
    #[serde(rename = "focus.gained")]
    FocusGained,

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
    /// `player` is the slot index of the originating pad (cube-gamepad
    /// "Multiplayer"; "Wire-format changes"). Additive: legacy daemons that
    /// omit it deserialize as player 0, so the `{input_seq, t_us, type, code,
    /// value}` shape stays backward-compatible.
    #[serde(rename = "input.event")]
    InputEvent {
        input_seq: u64,
        t_us: i64,
        #[serde(rename = "type")]
        kind: String,
        code: String,
        value: i32,
        #[serde(default)]
        player: u8,
    },

    /// `player` is the slot index whose held-key/axis state this snapshot
    /// describes (cube-gamepad "Wire-format changes": snapshot/dropped become
    /// per-player). Additive — omitted defaults to player 0.
    #[serde(rename = "input.snapshot")]
    InputSnapshot {
        input_seq: u64,
        device: String,
        keys: BTreeMap<String, i32>,
        abs: BTreeMap<String, i32>,
        #[serde(default)]
        player: u8,
    },

    #[serde(rename = "input.device_state")]
    InputDeviceState { device: String, connected: bool },

    /// `player` is the slot index whose stream dropped (cube-gamepad
    /// "Wire-format changes"). Additive — omitted defaults to player 0.
    #[serde(rename = "input.dropped")]
    InputDropped {
        since_seq: u64,
        #[serde(default)]
        player: u8,
    },

    // ── Power / system ───────────────────────────────────────────────────────
    /// `event_seq` is the SDS v6 §5.13 reliable-event stamp (omitted when
    /// `None`, preserving the v5 `{"event":"power.state","state":..}` shape).
    #[serde(rename = "power.state")]
    PowerState {
        state: PowerState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_seq: Option<u64>,
    },

    #[serde(rename = "config.reloaded")]
    ConfigReloaded,

    // ── Telemetry / brightness / subscription control (SDS v6 §5.13) ──────────
    /// Per-app frame telemetry sample (SDS v6 §5.13 "Event payloads —
    /// telemetry"). Telemetry is a coalescible class: it carries no
    /// `event_seq`. `drops_delta` is the drop count since the previous sample;
    /// `frame_seq` is the monotonic present sequence the sample was taken at.
    #[serde(rename = "app.stats")]
    AppStats {
        app: String,
        fps: f32,
        drops: u64,
        drops_delta: u64,
        frame_seq: u64,
    },

    /// Global brightness changed (SDS v6 §5.13 "Event payloads — brightness").
    /// Coalescible class: carries no `event_seq`, ever.
    #[serde(rename = "brightness.changed")]
    BrightnessChanged { value: u8 },

    /// A bounded subscription was terminated by the daemon (SDS v6 §5.13
    /// "Bounded subscriptions"): the final event delivered on that `sub_id`.
    #[serde(rename = "subscription.ended")]
    SubscriptionEnded {
        sub_id: String,
        reason: SubscriptionEndReason,
    },

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
