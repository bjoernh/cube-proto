//! Wave 15: round-trip tests for `StatusReport` and `DoctorReportPayload`.
//!
//! These are the payloads carried inside `Response.result` for the
//! `status` and `doctor.report` commands (SDS §11.1).

use std::collections::BTreeMap;

use cube_proto::{
    CheckLevel, CheckResult, CubedStatus, DisplayStatus, DoctorReportPayload, FramesDropped,
    PerAppStatus, StatusReport,
};
use serde_json::json;

#[test]
#[allow(clippy::too_many_lines)]
fn status_report_roundtrips_every_field_sds_11_1() {
    let mut per_app = BTreeMap::new();
    per_app.insert(
        "picture".to_owned(),
        PerAppStatus {
            pid: Some(4242),
            systemd_unit: "cube-app@picture.service".to_owned(),
            connection_state: "running".to_owned(),
            state: "focused".to_owned(),
            last_focused: 7,
            last_present_seq: 1234,
            inflight_buffers: 1,
            fps_submitted: 60.0,
            fps_displayed: 59.5,
            dropped_frames: FramesDropped {
                displayed: 100,
                replaced: 1,
                dropped: 0,
                focus_lost: 2,
                blanked: 0,
            },
            parameter_seq: 7,
            input_events_forwarded: 99,
            present_to_displayed_latency_mean_ms: Some(12.4),
            present_to_displayed_latency_p95_ms: Some(18.7),
            remote_sender_drops: 0,
            latest_video_latency_us: None,
            total_remote_sender_dropped: 0,
            last_seq: 0,
            pause_causes: None,
        },
    );

    let report = StatusReport {
        cubed: CubedStatus {
            version: "0.1.0".to_owned(),
            build_date: "2026-06-01T00:00:00+00:00".to_owned(),
            uptime_seconds: 600,
            protocol_version: "1.0".to_owned(),
            focused_app: Some("picture".to_owned()),
            launcher_state: "idle".to_owned(),
            resident_apps: 1,
            max_resident: 1,
        },
        display: DisplayStatus {
            driver_name: "vkms".to_owned(),
            drm_device_path: "/dev/dri/card0".to_owned(),
            mode: "64x64@60".to_owned(),
            pixel_format: "XR24".to_owned(),
            bitstream_version: 7,
            last_commit_us: 123_456_789,
            frames_displayed: 1234,
            frames_dropped: FramesDropped {
                displayed: 1234,
                replaced: 1,
                dropped: 0,
                focus_lost: 2,
                blanked: 3,
            },
            spi_errors: 0,
            blank_source: "none".to_owned(),
        },
        per_app,
        compositor: cube_proto::CompositorStatus::default(),
    };

    let v = serde_json::to_value(&report).unwrap();

    // Every SDS §11.1 cubed-field present.
    assert!(v["cubed"]["version"].is_string());
    assert!(v["cubed"]["uptime_seconds"].is_number());
    assert!(v["cubed"]["protocol_version"].is_string());
    assert!(v["cubed"]["focused_app"].is_string());
    assert!(v["cubed"]["launcher_state"].is_string());
    // SDS v6 §1.2 / delta §6: residency cap vs current count.
    assert!(v["cubed"]["resident_apps"].is_number());
    assert!(v["cubed"]["max_resident"].is_number());
    // display section.
    assert!(v["display"]["driver_name"].is_string());
    assert!(v["display"]["drm_device_path"].is_string());
    assert!(v["display"]["mode"].is_string());
    assert!(v["display"]["pixel_format"].is_string());
    assert!(v["display"]["bitstream_version"].is_number());
    assert!(v["display"]["last_commit_us"].is_number());
    assert!(v["display"]["frames_displayed"].is_number());
    assert!(v["display"]["frames_dropped"]["displayed"].is_number());
    assert!(v["display"]["frames_dropped"]["replaced"].is_number());
    assert!(v["display"]["frames_dropped"]["dropped"].is_number());
    assert!(v["display"]["frames_dropped"]["focus_lost"].is_number());
    assert!(v["display"]["frames_dropped"]["blanked"].is_number());
    assert!(v["display"]["spi_errors"].is_number());
    // per app
    let app = &v["per_app"]["picture"];
    assert!(app["pid"].is_number());
    assert!(app["systemd_unit"].is_string());
    assert!(app["connection_state"].is_string());
    assert!(app["last_present_seq"].is_number());
    assert!(app["inflight_buffers"].is_number());
    assert!(app["fps_submitted"].is_number());
    assert!(app["fps_displayed"].is_number());
    assert!(app["dropped_frames"]["displayed"].is_number());
    assert!(app["parameter_seq"].is_number());
    assert!(app["input_events_forwarded"].is_number());
    // SDS v6 §1.1 / delta §6: per-session resident state + last_focused.
    assert!(app["state"].is_string());
    assert!(app["last_focused"].is_number());
    assert!(app["present_to_displayed_latency_mean_ms"].is_number());
    assert!(app["present_to_displayed_latency_p95_ms"].is_number());
    assert!(app["remote_sender_drops"].is_number());

    // Round-trip.
    let back: StatusReport = serde_json::from_value(v).unwrap();
    assert_eq!(back, report);
}

/// SDS v6 §1.1/§1.2 / delta §6: a v5 status payload (no `resident_apps` /
/// `max_resident` / per-app `state` / `last_focused`) still deserializes,
/// defaulting the new fields to v5-equivalent values.
#[test]
fn v5_status_payload_without_v6_fields_deserializes_with_defaults_sds_6() {
    let v5_json = serde_json::json!({
        "cubed": {
            "version": "0.1.0",
            "build_date": "2026-06-01T00:00:00+00:00",
            "uptime_seconds": 600,
            "protocol_version": "1.0",
            "active_app": "picture",
            "focused_app": "picture",
            "launcher_state": "idle"
        },
        "display": {
            "driver_name": "vkms",
            "drm_device_path": "/dev/dri/card0",
            "mode": "64x64@60",
            "pixel_format": "XR24",
            "bitstream_version": 7,
            "last_commit_us": 123_456_789,
            "frames_displayed": 1234,
            "frames_dropped": {
                "displayed": 1234, "replaced": 1, "dropped": 0, "focus_lost": 2, "blanked": 3
            },
            "spi_errors": 0
        },
        "per_app": {
            "picture": {
                "pid": 4242,
                "systemd_unit": "cube-app@picture.service",
                "connection_state": "running",
                "last_present_seq": 1234,
                "inflight_buffers": 1,
                "fps_submitted": 60.0,
                "fps_displayed": 59.5,
                "dropped_frames": {
                    "displayed": 100, "replaced": 1, "dropped": 0, "focus_lost": 2, "blanked": 0
                },
                "parameter_seq": 7,
                "input_events_forwarded": 99,
                "present_to_displayed_latency_mean_ms": 12.4,
                "present_to_displayed_latency_p95_ms": 18.7,
                "remote_sender_drops": 0
            }
        }
    });

    let report: StatusReport = serde_json::from_value(v5_json).expect("v5 payload deserializes");
    assert_eq!(report.cubed.resident_apps, 0);
    assert_eq!(report.cubed.max_resident, 1);
    assert_eq!(report.display.blank_source, "none");
    let app = &report.per_app["picture"];
    assert_eq!(app.state, "");
    assert_eq!(app.last_focused, 0);
}

#[test]
fn doctor_report_payload_roundtrips_sds_11_1() {
    let payload = DoctorReportPayload {
        checks: vec![
            CheckResult {
                name: "drm.device".to_owned(),
                level: CheckLevel::Ok,
                detail: "opened /dev/dri/card0".to_owned(),
            },
            CheckResult {
                name: "controller.grab".to_owned(),
                level: CheckLevel::Warn,
                detail: "controller not connected".to_owned(),
            },
            CheckResult {
                name: "polkit.rule".to_owned(),
                level: CheckLevel::Fail,
                detail: "polkit rule missing".to_owned(),
            },
        ],
    };
    let v = serde_json::to_value(&payload).unwrap();
    // CheckLevel serializes lowercase.
    assert_eq!(v["checks"][0]["level"], json!("ok"));
    assert_eq!(v["checks"][1]["level"], json!("warn"));
    assert_eq!(v["checks"][2]["level"], json!("fail"));
    let back: DoctorReportPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back, payload);
    assert!(payload.any_fail());
}

#[test]
fn doctor_check_level_serializes_lowercase_sds_11_1() {
    assert_eq!(serde_json::to_value(CheckLevel::Ok).unwrap(), json!("ok"));
    assert_eq!(serde_json::to_value(CheckLevel::Warn).unwrap(), json!("warn"));
    assert_eq!(serde_json::to_value(CheckLevel::Fail).unwrap(), json!("fail"));
}
