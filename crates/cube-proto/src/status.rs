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
    pub active_app: Option<String>,
    pub focused_app: Option<String>,
    pub launcher_state: String,
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
}
