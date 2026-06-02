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
fn status_report_roundtrips_every_field_sds_11_1() {
    let mut per_app = BTreeMap::new();
    per_app.insert(
        "picture".to_owned(),
        PerAppStatus {
            pid: Some(4242),
            systemd_unit: "cube-app@picture.service".to_owned(),
            connection_state: "running".to_owned(),
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
        },
    );

    let report = StatusReport {
        cubed: CubedStatus {
            version: "0.1.0".to_owned(),
            build_date: "2026-06-01T00:00:00+00:00".to_owned(),
            uptime_seconds: 600,
            protocol_version: "1.0".to_owned(),
            active_app: Some("picture".to_owned()),
            focused_app: Some("picture".to_owned()),
            launcher_state: "idle".to_owned(),
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
        },
        per_app,
    };

    let v = serde_json::to_value(&report).unwrap();

    // Every SDS §11.1 cubed-field present.
    assert!(v["cubed"]["version"].is_string());
    assert!(v["cubed"]["uptime_seconds"].is_number());
    assert!(v["cubed"]["protocol_version"].is_string());
    assert!(v["cubed"]["active_app"].is_string());
    assert!(v["cubed"]["focused_app"].is_string());
    assert!(v["cubed"]["launcher_state"].is_string());
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
    assert!(app["present_to_displayed_latency_mean_ms"].is_number());
    assert!(app["present_to_displayed_latency_p95_ms"].is_number());
    assert!(app["remote_sender_drops"].is_number());

    // Round-trip.
    let back: StatusReport = serde_json::from_value(v).unwrap();
    assert_eq!(back, report);
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
