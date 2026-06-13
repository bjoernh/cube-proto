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
