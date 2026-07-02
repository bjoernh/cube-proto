//! Unsolicited event types (SDS §5.1, §5.3, §5.4, §5.7.1, §5.12, §6.1).
//!
//! `Event` is internally tagged on `"event"`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::input::BindingScope;
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

/// Why a client overlay was dismissed (SDS v7 §5.13, §6.1). Delivered as the
/// `reason` of an [`Event::OverlayDismissed`] event to the overlay's owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayDismissReason {
    /// The owner released it (`overlay.release`) or its connection closed.
    Released,
    /// Its base app lost focus, taking its over-self overlay with it.
    FocusLost,
    /// The display blanked.
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

    /// A previously-presented buffer is free for the client to reuse
    /// (SDS §5.1, §5.12). `layer` disambiguates *which* layer of a multi-layer
    /// connection the buffer belonged to (SDS v7 §5.13, D7): absent (or `0`) is
    /// the connection's **base** layer — the pre-compositor shape — while an
    /// overlay-buffer release carries the `layer` id returned by
    /// `overlay.acquire`. Additive and `skip_serializing_if`, so a base release
    /// keeps the exact `{buffer_id, seq, reason}` wire shape and old clients that
    /// never present overlays simply never see (and can safely ignore) the field.
    #[serde(rename = "buffer.release")]
    BufferRelease {
        buffer_id: u32,
        seq: u64,
        reason: ReleaseReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layer: Option<u32>,
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

    /// A pad was assigned to player `player` (cube-gamepad "Multiplayer";
    /// "Wire-format changes"). `name` is the controller's human-readable name;
    /// `vid_pid` is its USB identity as a lower-case `"vvvv:pppp"` hex string —
    /// the same identity used for slot persistence and tier-2 bindings — and is
    /// omitted for pads with no VID:PID (some BT pads). `hw_id` is the stable
    /// per-device id (the BT HW address / evdev `uniq`, falling back to
    /// `ID_PATH`/`phys`) that differs between two identical-model pads, so a
    /// client can map each physical controller to its slot unambiguously
    /// (LEDCube/cube#28); omitted when the device surfaced no stable id. Backs
    /// the companion's controller roster and per-player "Player N joined" UI; it
    /// is also the wire source for `cubekit`'s `RawEvent::Connected(true)`.
    #[serde(rename = "input.player_connected")]
    InputPlayerConnected {
        player: u8,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vid_pid: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hw_id: Option<String>,
    },

    /// A pad left player `player` (cube-gamepad "Multiplayer"; "Wire-format
    /// changes"). The wire source for `cubekit`'s `RawEvent::Connected(false)`
    /// and the companion's "Player N left" UI.
    #[serde(rename = "input.player_disconnected")]
    InputPlayerDisconnected { player: u8 },

    /// A tier-2 binding edit landed (cube-gamepad "Wire-format changes"). Lets
    /// the companion (and any client) reflect edits live. `seq` is the
    /// echo-suppression stamp — sourced from the same hub-publish path as
    /// `param.changed`, so the editing client ignores the echo of its own write.
    ///
    /// For a single `input.bindings.set`, `physical`/`action` carry the changed
    /// binding (canonical button config-names; `action` may be `"unbound"`). For
    /// an `input.bindings.reset`, both are omitted — the whole `(vid_pid, scope)`
    /// reverted to the profile 1:1 default.
    #[serde(rename = "input.binding_changed")]
    InputBindingChanged {
        vid_pid: String,
        scope: BindingScope,
        seq: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        physical: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<String>,
    },

    /// A tier-2 tuning edit landed (cube-gamepad "Wire-format changes"). Mirrors
    /// the `input.tuning.set` verb's optional fields: an omitted field was not
    /// part of this edit. `seq` is the echo-suppression stamp (as
    /// `param.changed`).
    #[serde(rename = "input.tuning_changed")]
    InputTuningChanged {
        vid_pid: String,
        scope: BindingScope,
        seq: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dead_zone: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stick_dpad_threshold: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        invert: Option<bool>,
    },

    /// One sample from the opt-in `input.capture` tap (cube-gamepad "Wire-format
    /// changes"): the live input behind the companion's press-to-bind and
    /// button-highlight. `button` is the canonical button config-name the user
    /// pressed (pre-remap). A read-only observation tap — events still route to
    /// the focused app; subscribers merely also receive them. High-volume, so
    /// the carrying class is lossy/coalescible (telemetry-style).
    ///
    /// `raw_type`/`raw_code`/`raw_value` are the *originating* evdev triple that
    /// produced this canonical button — the symbolic kind (`key`/`btn`/`abs`/…),
    /// the symbolic code (e.g. `BTN_SOUTH`, `ABS_HAT0Y`), and the raw evdev
    /// value. They expose the pre-normalization wire value so an input
    /// visualiser (`cubectl input watch`, LEDCube/cube#7) shows both the raw and
    /// decoded view side by side, making axis-encoding bugs (unsigned `0..255`
    /// vs zero-centred, LEDCube/cube#5) obvious at a glance. Additive: omitted
    /// by daemons that don't populate the tap, and they default to `None`.
    #[serde(rename = "input.capture")]
    InputSample {
        player: u8,
        button: String,
        pressed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_value: Option<i32>,
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

    /// A client overlay is no longer composed (SDS v7 §5.13, §6.1): the owner
    /// must stop presenting into `layer`. `reason` is why it was dismissed.
    #[serde(rename = "overlay.dismissed")]
    OverlayDismissed {
        layer: u32,
        reason: OverlayDismissReason,
    },

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
    PresentDropped {
        seq: u64,
        reason: PresentDroppedReason,
    },

    #[serde(rename = "remote.frames_dropped")]
    RemoteFramesDropped {
        client: String,
        dropped: u64,
        since_seq: u64,
    },

    #[serde(rename = "client.stats")]
    ClientStats {
        client: String,
        remote_sender_dropped: u64,
        last_seq: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        video_latency_us: Option<u64>,
    },
}
