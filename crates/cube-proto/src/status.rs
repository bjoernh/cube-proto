//! `cubectl status` payload — SDS §11.1.
//!
//! The wire response carries this struct serialized into the `Response.result`
//! field. The fields mirror SDS §11.1's bullet structure:
//!
//! ```text
//! cubed:     version, uptime, protocol_version, active app, focused app, launcher state
//! display:   driver name, DRM device path, mode, pixel format, bitstream version,
//!            last commit time, frames displayed, frames dropped (per reason), SPI errors
//! per app:   pid, systemd unit, connection state, last present seq, inflight buffers,
//!            fps_submitted, fps_displayed, dropped_frames (per reason), parameter seq,
//!            input.events_forwarded, present_to_displayed latency (mean/p95), remote sender drops
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level payload of a `status` response (SDS §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusReport {
    pub cubed: CubedStatus,
    pub display: DisplayStatus,
    pub per_app: BTreeMap<String, PerAppStatus>,
    /// `compositor:` section (SDS v7 §11.1, §5.13). Defaults so a pre-compositor
    /// (v6) status payload without this key still deserializes.
    #[serde(default)]
    pub compositor: CompositorStatus,
}

/// `compositor:` section (SDS v7 §11.1, §5.13).
///
/// Snapshots the DRM-thread compositor's layer state: how many layers are
/// composed this frame, whether a focus transition is animating (and a short
/// human-readable description of it), the active overlays with provenance + z,
/// who currently holds the modal input grab, and the cumulative
/// frames-composited counter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompositorStatus {
    /// Number of layers composed into the output framebuffer this snapshot
    /// (visible base / transition blend + client + system overlays).
    pub composited_layers: u32,
    /// `true` while a focus-change transition is animating.
    pub transition_in_progress: bool,
    /// Short description of the in-flight transition (kind + progress), or
    /// `None` when idle. Omitted from the wire when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    /// The currently-composed overlays, each with its provenance and z.
    pub overlays: Vec<OverlayStatus>,
    /// Who currently owns the modal input grab — `base` when input routes to the
    /// focused app, or `overlay:<layer>` for a modal client overlay (SDS §5.7.1).
    pub input_grab: String,
    /// Cumulative count of frames composed since `cubed` started.
    pub frames_composited: u64,
}

/// One composited overlay in the [`CompositorStatus`] snapshot (SDS v7 §11.1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlayStatus {
    /// `"system"` (cubed-rendered: banner / `overlay.text`) or `"client"`
    /// (a privileged client's acquired layer).
    pub provenance: String,
    /// Stacking order; higher z composes on top (SDS §5.13).
    pub z: i32,
    /// The client `layer` id for a client overlay, or `None` for a system one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<u32>,
}

/// `cubed:` section (SDS §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubedStatus {
    pub version: String,
    /// ISO 8601 (RFC 3339, UTC) timestamp of when this `cubed` binary was
    /// compiled. Lets a debugging session confirm via `cubectl status` that
    /// the freshly-built binary is actually the one running.
    pub build_date: String,
    pub uptime_seconds: u64,
    pub protocol_version: String,
    pub focused_app: Option<String>,
    pub launcher_state: String,
    /// Current count of resident non-launcher app sessions (focused + paused,
    /// SDS v6 §1.2 / delta §6). Defaults to `0` for backward compatibility
    /// with v5 status payloads.
    #[serde(default)]
    pub resident_apps: u32,
    /// `[apps] max_resident` from `system.toml` (SDS v6 §1.2 / delta §6).
    /// Defaults to `1` (v5-equivalent) for backward compatibility.
    #[serde(default = "max_resident_default")]
    pub max_resident: u32,
}

fn max_resident_default() -> u32 {
    1
}

/// `display:` section (SDS §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayStatus {
    pub driver_name: String,
    pub drm_device_path: String,
    pub mode: String,
    pub pixel_format: String,
    pub bitstream_version: u32,
    pub last_commit_us: u64,
    pub frames_displayed: u64,
    pub frames_dropped: FramesDropped,
    pub spi_errors: u64,
    /// Cumulative count of DRM-thread output failures — a `device.commit()` or
    /// `frame_events` send that returned an error (SDS v7 §11.1; R3.5). `0` in
    /// steady state; a non-zero value flags a failing display pipeline. Defaults
    /// to `0` for backward compatibility with pre-R3.5 status payloads.
    #[serde(default)]
    pub commit_errors: u64,
    /// What caused the display to be blanked (SDS v6 §5.12, delta §6/§8):
    /// `none` (active), `idle` (idle timer), or `command` (`power.blank`).
    /// Defaults to `none` for backward compatibility with v5 status payloads.
    #[serde(default = "blank_source_default")]
    pub blank_source: String,
}

fn blank_source_default() -> String {
    "none".to_owned()
}

/// Why a resident app is paused (SDS v6 §5.13). Mirrors the SDK's
/// `PauseCauses`: an app may be paused because it lost focus, because the
/// display was blanked, or both. Carried inline on [`PerAppStatus`] and in the
/// lifecycle snapshot / `app.state` event `cause`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PauseCauses {
    pub focus_lost: bool,
    pub blanked: bool,
}

/// Per-reason dropped-frame counters (SDS §5.1 / §11.1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FramesDropped {
    pub displayed: u64,
    pub replaced: u64,
    pub dropped: u64,
    pub focus_lost: u64,
    pub blanked: u64,
}

/// `per app:` map entry (SDS §11.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerAppStatus {
    pub pid: Option<u32>,
    pub systemd_unit: String,
    pub connection_state: String,
    /// Resident-session state (SDS v6 §1.1 / delta §6):
    /// `starting | focused | paused | stopping`. Empty string for apps with
    /// no tracked session (v5 compatibility / launcher).
    #[serde(default)]
    pub state: String,
    /// Monotonic focus stamp; eviction order is by this value (SDS v6 §1.2 /
    /// delta §6). `0` if the session has never been focused or is untracked.
    #[serde(default)]
    pub last_focused: u64,
    pub last_present_seq: u64,
    pub inflight_buffers: u32,
    pub fps_submitted: f32,
    pub fps_displayed: f32,
    pub dropped_frames: FramesDropped,
    pub parameter_seq: u64,
    pub input_events_forwarded: u64,
    pub present_to_displayed_latency_mean_ms: Option<f32>,
    pub present_to_displayed_latency_p95_ms: Option<f32>,
    pub remote_sender_drops: u64,
    /// Last `video_latency_us` value reported via `client.stats`
    /// (SDS §6.1). `None` until the first stats event arrives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_video_latency_us: Option<u64>,
    /// Cumulative remote-sender dropped frames reported via
    /// `client.stats`. Updated to the latest value each event.
    #[serde(default)]
    pub total_remote_sender_dropped: u64,
    /// Last `last_seq` value reported via `client.stats`.
    #[serde(default)]
    pub last_seq: u64,
    /// Why this session is paused (SDS v6 §5.13). `None` for sessions that are
    /// not paused or predate the field; omitted from the wire when absent so
    /// the pre-§5.13 session shape round-trips unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_causes: Option<PauseCauses>,
}
