//! — the `compositor:` block of the `cubectl status` payload.
//!
//! ## Contract
//!
//! - `pub struct CompositorStatus { composited_layers: u32,
//!   transition_in_progress: bool, transition: Option<String>,
//!   overlays: Vec<OverlayStatus>, input_grab: String, frames_composited: u64 }`
//!   with the usual derives (`Debug, Clone, PartialEq, Serialize, Deserialize`)
//!   and a `Default` impl.
//! - `pub struct OverlayStatus { provenance: String, z: i32, layer: Option<u32> }`
//!   (`provenance ∈ "system" | "client"`).
//! - `StatusReport` gains `#[serde(default)] pub compositor: CompositorStatus`
//!   so older (pre-compositor) payloads still deserialize.

use std::collections::BTreeMap;

use cube_proto::{
    CompositorStatus, CubedStatus, DisplayStatus, FramesDropped, OverlayStatus, StatusReport,
};

fn sample_report() -> StatusReport {
    StatusReport {
        cubed: CubedStatus {
            version: "0.1.0".to_owned(),
            build_date: "2026-06-25T00:00:00+00:00".to_owned(),
            uptime_seconds: 1,
            protocol_version: "1.0".to_owned(),
            focused_app: Some("snake".to_owned()),
            launcher_state: "idle".to_owned(),
            resident_apps: 1,
            max_resident: 2,
        },
        display: DisplayStatus {
            driver_name: "vkms".to_owned(),
            drm_device_path: "/dev/dri/card0".to_owned(),
            mode: "384x64@60".to_owned(),
            pixel_format: "XR24".to_owned(),
            bitstream_version: 7,
            last_commit_us: 0,
            frames_displayed: 0,
            frames_dropped: FramesDropped::default(),
            spi_errors: 0,
            commit_errors: 0,
            blank_source: "none".to_owned(),
        },
        per_app: BTreeMap::new(),
        compositor: CompositorStatus::default(),
    }
}

#[test]
fn status_report_carries_compositor_section() {
    // A populated compositor block round-trips field-for-field.
    let cs = CompositorStatus {
        composited_layers: 4,
        transition_in_progress: true,
        transition: Some("crossfade @ 50%".to_owned()),
        overlays: vec![
            OverlayStatus {
                provenance: "client".to_owned(),
                z: 2,
                layer: Some(7),
            },
            OverlayStatus {
                provenance: "system".to_owned(),
                z: 100,
                layer: None,
            },
        ],
        input_grab: "overlay:7".to_owned(),
        frames_composited: 1234,
    };
    let json = serde_json::to_string(&cs).expect("serialize CompositorStatus");
    let back: CompositorStatus = serde_json::from_str(&json).expect("deserialize CompositorStatus");
    assert_eq!(cs, back, "CompositorStatus round-trips");

    // Every compositor field is present on the wire.
    for key in [
        "composited_layers",
        "transition_in_progress",
        "overlays",
        "input_grab",
        "frames_composited",
    ] {
        assert!(
            json.contains(key),
            "compositor JSON missing `{key}`: {json}"
        );
    }
    assert!(
        json.contains("provenance"),
        "overlay provenance on the wire: {json}"
    );

    // The full StatusReport serializes a `compositor` section.
    let report = sample_report();
    let mut v = serde_json::to_value(&report).expect("serialize StatusReport");
    assert!(
        v.get("compositor").is_some(),
        "StatusReport serializes a compositor section"
    );

    // A pre-compositor payload (no `compositor` key) still deserializes, with
    // the field defaulting — backward compatibility.
    v.as_object_mut()
        .expect("status report is a JSON object")
        .remove("compositor");
    let back: StatusReport =
        serde_json::from_value(v).expect("a status payload without `compositor` still decodes");
    assert_eq!(
        back.compositor,
        CompositorStatus::default(),
        "absent compositor block defaults"
    );
}
