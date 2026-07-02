//! Tier-2 gamepad wire vocabulary (cube-gamepad "Tier 2 — user bindings & the
//! companion control-plane surface").
//!
//! These are the shared wire types behind the tier-2 admin verbs
//! ([`crate::Request::InputControllers`] / `InputBindings*` / `InputTuningSet`),
//! the tier-2 events ([`crate::Event::InputBindingChanged`] /
//! `InputTuningChanged` / `InputSample`), and the subscription snapshot sections
//! ([`crate::InputBindingsSnapshot`]). Keeping them in one module lets the verb,
//! event, and snapshot surfaces speak the same vocabulary.
//!
//! This crate is the pure wire contract: buttons travel as their canonical
//! config-name strings (`"A"`, `"ShoulderLeft"`, `"Guide"`, …, plus the
//! `"unbound"` action sentinel) — `cube-input` owns the parse/print of those
//! names against its canonical `Button` enum.

use serde::{Deserialize, Serialize};

/// The scope a tier-2 binding / tuning override applies in (cube-gamepad
/// "Data model": *global* or *per-game*).
///
/// Wire form (externally tagged): `BindingScope::Global` serializes to the bare
/// string `"global"`; `BindingScope::Game(app)` serializes to `{"game": <app>}`.
/// This is the value of the `scope` field on the `input.bindings.*` /
/// `input.tuning.set` verbs and the `input.binding_changed` /
/// `input.tuning_changed` events.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingScope {
    /// Applies to this controller in every app.
    Global,
    /// Applies only while the named app is the focused app.
    Game(String),
}

/// Whether a connected controller matched a recognized device profile or fell
/// back to the generic profile (cube-gamepad: `input.controllers` roster).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerProfile {
    /// A VID:PID- or name-matched device profile was applied.
    Recognized,
    /// No profile matched; the generic-fallback profile is in effect.
    Generic,
}

/// One connected controller in the roster.
///
/// The element type of the `input.controllers` result array
/// (`[{ player, name, vid_pid, hw_id, profile, connected }]`) and of the
/// [`InputBindingsSnapshot`](crate::InputBindingsSnapshot) `controllers` list.
/// `vid_pid` is the lower-case `"vvvv:pppp"` USB identity (the same key used for
/// slot persistence and tier-2 bindings) and is omitted for pads with no VID:PID
/// (some BT pads). `hw_id` is the stable per-device id (the BT HW address /
/// evdev `uniq`, falling back to `ID_PATH`/`phys`) that distinguishes two pads
/// of the same model (LEDCube/cube#28).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// Slot index the pad is seated in.
    pub player: u8,
    /// Human-readable controller name.
    pub name: String,
    /// Lower-case `"vvvv:pppp"` USB identity; omitted for pads with no VID:PID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vid_pid: Option<String>,
    /// Stable per-device id (LEDCube/cube#28): the evdev `uniq` / udev `UNIQ`
    /// (for Bluetooth pads the hardware address, e.g. `"E4:17:D8:25:FB:42"`),
    /// falling back to `ID_PATH`/`phys` for wired pads. Unlike `vid_pid`, this
    /// differs between two controllers of the *same* model, letting a client map
    /// each physical pad to its player slot unambiguously. Omitted when the
    /// device surfaced no stable id. Additive/optional for forward-compat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hw_id: Option<String>,
    /// Whether a recognized profile matched, or the generic fallback is in use.
    pub profile: ControllerProfile,
    /// Whether the pad is currently connected (a remembered-but-absent slot is
    /// `false`).
    pub connected: bool,
}
