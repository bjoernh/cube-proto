//! Round-trip tests for every wire type in scope for local Unix `SOCK_SEQPACKET`
//! transport. Each test pins one concrete JSON shape (`serde_json::json!`),
//! deserializes it into the appropriate `cube_proto` type, re-serializes, and
//! compares the resulting `serde_json::Value` to the original.
//!
//! Naming convention: `<thing>_<form>_roundtrips`.
//!
//! Deviations from the issue prompt:
//! - For `Request`/`Event` we wrap the spec-literal JSON in the envelope keys
//!   they actually carry on the wire (`id`, `cmd`/`event`, etc.). The spec shows
//!   most worked examples already complete; we keep them verbatim.
//! - Some commands ("focus", "stop", "restart", "journal", "preset.*",
//!   "brightness.set", "doctor.report", etc.) do not have inline spec literals,
//!   so we synthesise minimal but normative-shape literals from the command
//!   surface. Those literal shapes are the contract this test file pins.

use serde_json::{Value, json};

use cube_proto::{Damage, Event, Format, HelloResult, ParamValue, Request, Response, Vec2, Vec3};

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

fn roundtrip_request(j: &Value) -> Value {
    let req: Request = serde_json::from_value(j.clone()).expect("Request decode");
    let back = serde_json::to_value(&req).expect("Request encode");
    assert_eq!(&back, j, "Request did not round-trip");
    // bytes round-trip too
    let bytes = serde_json::to_vec(&req).expect("Request to_vec");
    let req2: Request = serde_json::from_slice(&bytes).expect("Request from_slice");
    let back2 = serde_json::to_value(&req2).expect("Request encode #2");
    assert_eq!(&back2, j, "Request bytes round-trip mismatch");
    back
}

fn roundtrip_event(j: &Value) -> Value {
    let ev: Event = serde_json::from_value(j.clone()).expect("Event decode");
    let back = serde_json::to_value(&ev).expect("Event encode");
    assert_eq!(&back, j, "Event did not round-trip");
    let bytes = serde_json::to_vec(&ev).expect("Event to_vec");
    let ev2: Event = serde_json::from_slice(&bytes).expect("Event from_slice");
    let back2 = serde_json::to_value(&ev2).expect("Event encode #2");
    assert_eq!(&back2, j, "Event bytes round-trip mismatch");
    back
}

fn roundtrip_response(j: &Value) -> Value {
    let r: Response = serde_json::from_value(j.clone()).expect("Response decode");
    let back = serde_json::to_value(&r).expect("Response encode");
    assert_eq!(&back, j, "Response did not round-trip");
    let bytes = serde_json::to_vec(&r).expect("Response to_vec");
    let r2: Response = serde_json::from_slice(&bytes).expect("Response from_slice");
    let back2 = serde_json::to_value(&r2).expect("Response encode #2");
    assert_eq!(&back2, j, "Response bytes round-trip mismatch");
    back
}

// ─────────────────────────────────────────────────────────────────────────────
// Session: hello, register
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hello_request_roundtrips() {
    let j = json!({
        "id": 1,
        "cmd": "hello",
        "protocol_version": "1.0.0"
    });
    roundtrip_request(&j);
}

#[test]
fn hello_request_v6_minor_roundtrips() {
    // v6 is a minor bump within major 1; a `hello` quoting
    // the full v6 wire version (cube_proto::PROTOCOL_VERSION) round-trips
    // identically to any other protocol_version string.
    let j = json!({
        "id": 1,
        "cmd": "hello",
        "protocol_version": cube_proto::PROTOCOL_VERSION
    });
    roundtrip_request(&j);
}

#[test]
fn hello_result_roundtrips() {
    // the hello OK response carries
    // cubed's advertised protocol version + surface marker.
    let j = json!({
        "protocol_version": "1.1",
        "surface": "v6"
    });
    let r: HelloResult = serde_json::from_value(j.clone()).expect("HelloResult decode");
    let back = serde_json::to_value(&r).expect("HelloResult encode");
    assert_eq!(back, j);
    assert_eq!(r.protocol_version, "1.1");
    assert_eq!(r.surface.as_deref(), Some("v6"));
}

#[test]
fn hello_result_without_surface_omits_field() {
    // `surface` is optional / `skip_serializing_if` so older clients that
    // only read `protocol_version` see a minimal body.
    let r = HelloResult {
        protocol_version: "1.1".to_string(),
        surface: None,
        capabilities: None,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v, json!({"protocol_version": "1.1"}));
    let r2: HelloResult = serde_json::from_value(v).unwrap();
    assert_eq!(r, r2);
}

#[test]
fn response_ok_with_hello_result_roundtrips() {
    let j = json!({
        "id": 1,
        "ok": true,
        "result": {"protocol_version": "1.1", "surface": "v6"}
    });
    roundtrip_response(&j);
}

#[test]
fn register_request_roundtrips() {
    let j = json!({
        "id": 2,
        "cmd": "register",
        "name": "snake",
        "mode": "supervised"
    });
    roundtrip_request(&j);
}

#[test]
fn register_request_dev_mode_roundtrips() {
    let j = json!({
        "id": 3,
        "cmd": "register",
        "name": "snake",
        "mode": "dev"
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle: list, status, launch, stop, focus, restart, journal
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_request_roundtrips() {
    let j = json!({"id": 4, "cmd": "list"});
    roundtrip_request(&j);
}

#[test]
fn status_request_roundtrips() {
    let j = json!({"id": 5, "cmd": "status", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn status_request_no_app_roundtrips() {
    let j = json!({"id": 6, "cmd": "status"});
    roundtrip_request(&j);
}

#[test]
fn launch_request_roundtrips() {
    let j = json!({"id": 7, "cmd": "launch", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn stop_request_roundtrips() {
    let j = json!({"id": 8, "cmd": "stop", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn stop_request_no_app_roundtrips() {
    let j = json!({"id": 9, "cmd": "stop"});
    roundtrip_request(&j);
}

#[test]
fn focus_request_roundtrips() {
    let j = json!({"id": 10, "cmd": "focus", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn restart_request_roundtrips() {
    let j = json!({"id": 11, "cmd": "restart", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn journal_request_roundtrips_arch_8_4() {
    let j = json!({"id": 12, "cmd": "journal", "app": "snake", "follow": false, "tail": 100});
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Buffers: buffer.register
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn buffer_register_request_roundtrips() {
    let j = json!({
        "id": 10,
        "cmd": "buffer.register",
        "buffer_id": 0,
        "format": "RGB565",
        "width": 384,
        "height": 64,
        "stride": 768,
        "size": 49152
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Frames: present
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_request_with_damage_roundtrips() {
    let j = json!({
        "id": 11,
        "cmd": "present",
        "seq": 100,
        "buffer_id": 0,
        "damage": {"x": 0, "y": 0, "w": 384, "h": 64}
    });
    roundtrip_request(&j);
}

#[test]
fn present_request_no_damage_roundtrips() {
    let j = json!({
        "id": 12,
        "cmd": "present",
        "seq": 101,
        "buffer_id": 0
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Parameters: get, get-all, set, set-many, describe, subscribe, unsubscribe
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_request_roundtrips() {
    let j = json!({"id": 20, "cmd": "get", "app": "snake", "key": "speed"});
    roundtrip_request(&j);
}

#[test]
fn get_all_request_roundtrips() {
    let j = json!({"id": 21, "cmd": "get-all", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn set_request_int_roundtrips() {
    let j = json!({
        "id": 22,
        "cmd": "set",
        "app": "snake",
        "key": "speed",
        "value": {"type": "int", "value": 5}
    });
    roundtrip_request(&j);
}

#[test]
fn set_request_color_roundtrips() {
    let j = json!({
        "id": 23,
        "cmd": "set",
        "app": "snake",
        "key": "tint",
        "value": {"type": "color", "value": "#FF8800"}
    });
    roundtrip_request(&j);
}

#[test]
fn set_many_request_roundtrips() {
    let j = json!({
        "id": 24,
        "cmd": "set-many",
        "app": "snake",
        "values": {
            "speed": {"type": "int", "value": 5},
            "wrap":  {"type": "bool", "value": true}
        }
    });
    roundtrip_request(&j);
}

#[test]
fn describe_request_one_key_roundtrips() {
    let j = json!({"id": 25, "cmd": "describe", "app": "snake", "key": "speed"});
    roundtrip_request(&j);
}

#[test]
fn describe_request_all_keys_roundtrips() {
    let j = json!({"id": 26, "cmd": "describe", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn subscribe_request_roundtrips() {
    let j = json!({"id": 27, "cmd": "subscribe", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn unsubscribe_request_roundtrips() {
    let j = json!({"id": 28, "cmd": "unsubscribe", "app": "snake"});
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Presets: preset.list, preset.load, preset.save, preset.delete, preset.current,
//          preset.export, preset.import
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn preset_list_request_roundtrips() {
    let j = json!({"id": 30, "cmd": "preset.list", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn preset_load_request_roundtrips() {
    let j = json!({"id": 31, "cmd": "preset.load", "app": "snake", "name": "default"});
    roundtrip_request(&j);
}

#[test]
fn preset_save_request_roundtrips() {
    let j = json!({"id": 32, "cmd": "preset.save", "app": "snake", "name": "fast"});
    roundtrip_request(&j);
}

#[test]
fn preset_delete_request_roundtrips() {
    let j = json!({"id": 33, "cmd": "preset.delete", "app": "snake", "name": "fast"});
    roundtrip_request(&j);
}

#[test]
fn preset_current_request_roundtrips() {
    let j = json!({"id": 34, "cmd": "preset.current", "app": "snake"});
    roundtrip_request(&j);
}

#[test]
fn preset_export_request_roundtrips() {
    let j = json!({"id": 35, "cmd": "preset.export", "app": "snake", "name": "default"});
    roundtrip_request(&j);
}

#[test]
fn preset_import_request_roundtrips() {
    let j = json!({
        "id": 36,
        "cmd": "preset.import",
        "app": "snake",
        "toml": "[meta]\napp=\"snake\"\nschema_version=1\npreset_name=\"x\"\n[params]\nspeed=5\n"
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Brightness and Diagnostics doctor.report
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn brightness_set_request_roundtrips() {
    let j = json!({"id": 40, "cmd": "brightness.set", "value": 128});
    roundtrip_request(&j);
}

#[test]
fn brightness_get_request_roundtrips() {
    let j = json!({"id": 41, "cmd": "brightness.get"});
    roundtrip_request(&j);
}

#[test]
fn doctor_report_request_roundtrips() {
    let j = json!({"id": 41, "cmd": "doctor.report"});
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Power (admin): power.blank, power.wake
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn power_blank_request_roundtrips() {
    let j = json!({"id": 50, "cmd": "power.blank"});
    roundtrip_request(&j);
}

#[test]
fn power_wake_request_roundtrips() {
    let j = json!({"id": 51, "cmd": "power.wake"});
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// App-Store (M1): apt.install, apt.upgrade, preview.asset (admin verbs)
//
// New admin-socket verbs. Wire cmd tags follow the codebase's dotted
// convention (like power.blank / overlay.text). `version` (install/upgrade)
// and `timeout_secs` (preview) are OPTIONAL — omitted round-trips byte-for-byte
// (skip_serializing_if). Per D2, preview.asset carries a path STRING, never
// bytes.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn apt_install_request_roundtrips_appstore() {
    // Pinned version.
    let j = json!({"id": 70, "cmd": "apt.install", "package": "cube-app-voxel-sand", "version": "1.2.3"});
    roundtrip_request(&j);
}

#[test]
fn apt_install_request_without_version_roundtrips_appstore() {
    // No version pin → install latest; `version` must stay off the wire.
    let j = json!({"id": 71, "cmd": "apt.install", "package": "cube-app-voxel-sand"});
    roundtrip_request(&j);
}

#[test]
fn apt_upgrade_request_roundtrips_appstore() {
    let j =
        json!({"id": 72, "cmd": "apt.upgrade", "package": "cube-app-tron3d", "version": "0.4.1"});
    roundtrip_request(&j);
}

#[test]
fn apt_upgrade_request_without_version_roundtrips_appstore() {
    let j = json!({"id": 73, "cmd": "apt.upgrade", "package": "cube-app-tron3d"});
    roundtrip_request(&j);
}

#[test]
fn preview_asset_request_roundtrips_appstore() {
    // D2: path handoff — the verb carries only a filesystem path plus an
    // optional per-request timeout override.
    let j = json!({
        "id": 74,
        "cmd": "preview.asset",
        "path": "/run/cube/store-previews/voxel-sand.gif",
        "timeout_secs": 20
    });
    roundtrip_request(&j);
}

#[test]
fn preview_asset_request_without_timeout_roundtrips_appstore() {
    // Omitted `timeout_secs` → cubed falls back to [apps] preview_timeout_secs;
    // must stay off the wire.
    let j = json!({
        "id": 75,
        "cmd": "preview.asset",
        "path": "/run/cube/store-previews/tron3d.gif"
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Transitions on launch / focus
//
// `launch` and `focus` take an OPTIONAL `transition: { kind: "cut"|"crossfade", duration_ms?: u32 }`
// field. These pin the exact wire shapes:
//
//   {"id":30,"cmd":"focus", "app":"snake",     "transition":{"kind":"crossfade","duration_ms":250}}
//   {"id":31,"cmd":"launch","app":"pixelflow", "transition":{"kind":"cut"}}
//
// The field is skipped when absent (skip_serializing_if = none), so the existing
// `launch_request_roundtrips` / `focus_request_roundtrips` — which omit
// `transition` — keep round-tripping byte-for-byte.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn transition_field_roundtrips_on_launch() {
    // `cut` is the hard-cut opt-out; `duration_ms` is omitted (optional).
    let j = json!({
        "id": 31,
        "cmd": "launch",
        "app": "pixelflow",
        "transition": {"kind": "cut"}
    });
    roundtrip_request(&j);
}

#[test]
fn transition_field_roundtrips_on_focus() {
    // `crossfade` with an explicit duration.
    let j = json!({
        "id": 30,
        "cmd": "focus",
        "app": "snake",
        "transition": {"kind": "crossfade", "duration_ms": 250}
    });
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Four additional transition kinds. The wire `TransitionKind` reserves room for
// these ("`wipe` / `dissolve` / `push` are reserved … addable without a protocol
// change"). Tokens: `dissolve`, `dip_to_black`, `particle_dissolve`, `push`.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn transition_dissolve_roundtrips() {
    let j = json!({"id": 60, "cmd": "focus", "app": "snake", "transition": {"kind": "dissolve"}});
    roundtrip_request(&j);
}

#[test]
fn transition_dip_to_black_roundtrips() {
    let j = json!({
        "id": 61,
        "cmd": "launch",
        "app": "pixelflow",
        "transition": {"kind": "dip_to_black", "duration_ms": 400}
    });
    roundtrip_request(&j);
}

#[test]
fn transition_particle_dissolve_roundtrips() {
    let j = json!({
        "id": 62,
        "cmd": "focus",
        "app": "snake",
        "transition": {"kind": "particle_dissolve", "duration_ms": 600}
    });
    roundtrip_request(&j);
}

#[test]
fn transition_push_roundtrips() {
    let j = json!({"id": 63, "cmd": "focus", "app": "snake", "transition": {"kind": "push"}});
    roundtrip_request(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// — ARGB8888 overlay format + admin text overlay
//
// The overlay pixel format and the two admin-only system-text-overlay commands:
//
//   - `Format::Argb8888` with the wire token `"ARGB8888"` (alongside `RGB565`).
//   - `Request::OverlayText { id, text, duration_ms?, z?, color? }` (cmd
//     `"overlay.text"`) — system-rendered text overlay (no buffer/SCM_RIGHTS).
//   - `Request::OverlayClear { id, z? }` (cmd `"overlay.clear"`).
//
// All three pin the exact JSON.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn format_argb8888_roundtrips() {
    //: overlay buffers use ARGB8888 (real alpha for `over`
    // compositing). The wire token is "ARGB8888". Decode-through `Format` so the
    // test compiles today and fails at runtime until the variant exists.
    let v = json!("ARGB8888");
    let f: Format =
        serde_json::from_value(v.clone()).expect("Format must decode the \"ARGB8888\" token");
    let back = serde_json::to_value(f).expect("encode");
    assert_eq!(back, v, "ARGB8888 round-trips to the same wire token");
    // RGB565 still decodes (the new variant is additive, not a replacement).
    let r: Format = serde_json::from_value(json!("RGB565")).expect("RGB565 still decodes");
    assert_ne!(
        serde_json::to_value(r).unwrap(),
        back,
        "ARGB8888 and RGB565 are distinct variants"
    );
}

#[test]
fn overlay_text_command_roundtrips() {
    // worked example — admin-only system text overlay (all fields):
    //   {"id":50,"cmd":"overlay.text","text":"Hello TEST",
    //    "duration_ms":3000,"z":1,"color":"#FFFFFF"}
    let j = json!({
        "id": 50,
        "cmd": "overlay.text",
        "text": "Hello TEST",
        "duration_ms": 3000,
        "z": 1,
        "color": "#FFFFFF"
    });
    roundtrip_request(&j);

    // `duration_ms` / `z` / `color` are all optional ("z (default 1) and
    // color (default white) are optional"; duration_ms `0` = sticky). The
    // minimal form omits them and must round-trip byte-for-byte (the omitted
    // fields stay off the wire via skip_serializing_if).
    let minimal = json!({
        "id": 51,
        "cmd": "overlay.text",
        "text": "x"
    });
    roundtrip_request(&minimal);
}

#[test]
fn overlay_clear_command_roundtrips() {
    //: {"id":51,"cmd":"overlay.clear","z":1}.
    let j = json!({"id": 52, "cmd": "overlay.clear", "z": 1});
    roundtrip_request(&j);

    // `z` is optional — "overlay.clear with no z clears all admin text
    // overlays". The clear-all form omits z and round-trips.
    let clear_all = json!({"id": 53, "cmd": "overlay.clear"});
    roundtrip_request(&clear_all);
}

// ─────────────────────────────────────────────────────────────────────────────
// Response envelope
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn response_ok_with_result_roundtrips() {
    let j = json!({"id": 7, "ok": true, "result": {"any": "value"}});
    roundtrip_response(&j);
}

#[test]
fn response_ok_empty_result_roundtrips() {
    // `buffer.register` returns just `{"id":10,"ok":true}`
    let j = json!({"id": 10, "ok": true});
    roundtrip_response(&j);
}

#[test]
fn response_error_roundtrips() {
    let j = json!({
        "id": 99,
        "ok": false,
        "error": {
            "code": "ENOAPP",
            "message": "no such app",
            "context": {"app": "ghost"}
        }
    });
    roundtrip_response(&j);
}

#[test]
fn response_preset_import_warnings_roundtrips() {
    // explicit worked example.
    let j = json!({
        "id": 42,
        "ok": true,
        "result": {
            "warnings": [
                {"key": "speed",     "code": "CLAMPED",        "from": 999, "to": 10},
                {"key": "old_param", "code": "UNKNOWN_DROPPED"}
            ]
        }
    });
    roundtrip_response(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: app.started, app.stopped
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn app_started_event_roundtrips() {
    let j = json!({"event": "app.started", "app": "snake"});
    roundtrip_event(&j);
}

#[test]
fn app_stopped_event_roundtrips() {
    // v5 wire shape: no `reason` field. `reason` is `#[serde(default,
    // skip_serializing_if = "Option::is_none")]` so this v5-shaped literal
    // still round-trips byte-for-byte (backward compat).
    let j = json!({"event": "app.stopped", "app": "snake"});
    roundtrip_event(&j);
}

#[test]
fn app_stopped_event_with_reason_roundtrips() {
    let j = json!({"event": "app.stopped", "app": "snake", "reason": "control_lost"});
    roundtrip_event(&j);
}

#[test]
fn app_stopped_event_with_frame_stream_idle_reason_roundtrips() {
    // / item 2.7: the `frame_stream_idle` teardown reason added
    // to the `app.stopped` wire vocabulary.
    let j = json!({"event": "app.stopped", "app": "snake", "reason": "frame_stream_idle"});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: install.progress, install.complete (App-Store M1)
//
// Streamed while an `apt.install` / `apt.upgrade` job runs. `app` is the deb
// package name. `install.progress.line` is one line of apt output; `.ok` on
// install.complete reports success/failure. `event_seq` follows the reliable-
// event convention: `Option<u64>`, omitted on the wire when absent.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn install_progress_event_roundtrips_appstore() {
    let j = json!({
        "event": "install.progress",
        "app": "cube-app-voxel-sand",
        "line": "Unpacking cube-app-voxel-sand (1.2.3) ..."
    });
    roundtrip_event(&j);
}

#[test]
fn install_progress_event_with_event_seq_roundtrips_appstore() {
    let j = json!({
        "event": "install.progress",
        "app": "cube-app-voxel-sand",
        "line": "Setting up cube-app-voxel-sand (1.2.3) ...",
        "event_seq": 42
    });
    roundtrip_event(&j);
}

#[test]
fn install_complete_event_ok_roundtrips_appstore() {
    let j = json!({"event": "install.complete", "app": "cube-app-voxel-sand", "ok": true});
    roundtrip_event(&j);
}

#[test]
fn install_complete_event_failure_roundtrips_appstore() {
    let j = json!({"event": "install.complete", "app": "cube-app-voxel-sand", "ok": false});
    roundtrip_event(&j);
}

#[test]
fn install_complete_event_with_event_seq_roundtrips_appstore() {
    let j = json!({
        "event": "install.complete",
        "app": "cube-app-voxel-sand",
        "ok": true,
        "event_seq": 43
    });
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: focus.lost, focus.gained (reliable-tier)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn focus_lost_event_app_switch_roundtrips() {
    let j = json!({"event": "focus.lost", "reason": "app_switch"});
    roundtrip_event(&j);
}

#[test]
fn focus_lost_event_home_roundtrips() {
    let j = json!({"event": "focus.lost", "reason": "home"});
    roundtrip_event(&j);
}

#[test]
fn focus_lost_event_stopping_roundtrips() {
    let j = json!({"event": "focus.lost", "reason": "stopping"});
    roundtrip_event(&j);
}

#[test]
fn focus_gained_event_roundtrips() {
    let j = json!({"event": "focus.gained"});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: present.displayed, buffer.release × 5 reasons
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_displayed_event_roundtrips() {
    let j = json!({
        "event": "present.displayed",
        "seq": 100,
        "buffer_id": 0
    });
    roundtrip_event(&j);
}

#[test]
fn buffer_release_displayed_reason_roundtrips() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "displayed"
    });
    roundtrip_event(&j);
}

#[test]
fn buffer_release_dropped_reason_roundtrips() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "dropped"
    });
    roundtrip_event(&j);
}

#[test]
fn buffer_release_replaced_reason_roundtrips() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "replaced"
    });
    roundtrip_event(&j);
}

#[test]
fn buffer_release_focus_lost_reason_roundtrips() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "focus_lost"
    });
    roundtrip_event(&j);
}

#[test]
fn buffer_release_blanked_reason_roundtrips() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "blanked"
    });
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: param.changed, params.changed, events.dropped
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn param_changed_event_roundtrips() {
    let j = json!({
        "event": "param.changed",
        "app": "snake",
        "seq": 17,
        "key": "speed",
        "value": {"type": "int", "value": 5}
    });
    roundtrip_event(&j);
}

#[test]
fn params_changed_event_roundtrips() {
    let j = json!({
        "event": "params.changed",
        "app": "snake",
        "seq": 18,
        "values": {
            "speed": {"type": "int", "value": 5},
            "wrap":  {"type": "bool", "value": true}
        }
    });
    roundtrip_event(&j);
}

#[test]
fn events_dropped_event_roundtrips() {
    let j = json!({"event": "events.dropped", "since_seq": 1234});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: input.event (key + abs), input.snapshot, input.device_state,
//         input.dropped
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_event_key_roundtrips() {
    let j = json!({
        "event": "input.event",
        "input_seq": 120,
        "t_us": 1_234_567_890_i64,
        "type": "key",
        "code": "BTN_A",
        "value": 1,
        "player": 0
    });
    roundtrip_event(&j);
}

#[test]
fn input_event_abs_roundtrips() {
    let j = json!({
        "event": "input.event",
        "input_seq": 121,
        "t_us": 1_234_567_891_i64,
        "type": "abs",
        "code": "ABS_X",
        "value": 17234,
        "player": 0
    });
    roundtrip_event(&j);
}

#[test]
fn input_event_without_player_defaults_to_zero() {
    // Wire back-compat (cube-gamepad "Wire-format changes"): a legacy daemon
    // omits `player`; it must deserialize as player 0, not fail.
    let j = json!({
        "event": "input.event",
        "input_seq": 120,
        "t_us": 1_234_567_890_i64,
        "type": "key",
        "code": "BTN_A",
        "value": 1
    });
    let ev: Event = serde_json::from_value(j).expect("legacy input.event (no player) must decode");
    match ev {
        Event::InputEvent { player, .. } => assert_eq!(player, 0, "absent player defaults to 0"),
        other => panic!("expected InputEvent, got {other:?}"),
    }
}

#[test]
fn input_snapshot_event_roundtrips() {
    let j = json!({
        "event": "input.snapshot",
        "input_seq": 140,
        "device": "8BitDo SN30 Pro",
        "keys": {"BTN_A": 0, "BTN_B": 0, "BTN_START": 0},
        "abs":  {"ABS_X": 16384, "ABS_Y": 16384},
        "player": 0
    });
    roundtrip_event(&j);
}

#[test]
fn input_device_state_event_connected_roundtrips() {
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": true
    });
    roundtrip_event(&j);
}

#[test]
fn input_device_state_event_disconnected_roundtrips() {
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": false
    });
    roundtrip_event(&j);
}

#[test]
fn input_dropped_event_roundtrips() {
    let j = json!({"event": "input.dropped", "since_seq": 120, "player": 0});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: input.player_connected / input.player_disconnected
// (cube-gamepad "Wire-format changes")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_player_connected_roundtrips() {
    let j = json!({
        "event": "input.player_connected",
        "player": 1,
        "name": "8BitDo SN30 Pro",
        "vid_pid": "2dc8:9018",
        "hw_id": "E4:17:D8:25:FB:42"
    });
    let back = roundtrip_event(&j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        // The stable per-device id round-trips and lets a
        // client map this physical pad to its slot even when a second
        // identical-model pad is present.
        Event::InputPlayerConnected { hw_id, .. } => {
            assert_eq!(hw_id.as_deref(), Some("E4:17:D8:25:FB:42"));
        }
        other => panic!("expected InputPlayerConnected, got {other:?}"),
    }
}

#[test]
fn input_player_connected_without_vid_pid_roundtrips() {
    // A pad with no USB VID:PID (some BT pads) omits the field on the wire; the
    // omitted form must round-trip unchanged (skip_serializing_if on `None`).
    let j = json!({
        "event": "input.player_connected",
        "player": 2,
        "name": "Generic Gamepad"
    });
    let back = roundtrip_event(&j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputPlayerConnected {
            player,
            vid_pid,
            hw_id,
            ..
        } => {
            assert_eq!(player, 2);
            assert_eq!(vid_pid, None, "absent vid_pid decodes to None");
            assert_eq!(hw_id, None, "absent hw_id decodes to None");
        }
        other => panic!("expected InputPlayerConnected, got {other:?}"),
    }
}

#[test]
fn input_player_disconnected_roundtrips() {
    let j = json!({"event": "input.player_disconnected", "player": 1});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Requests: tier-2 input bindings / tuning (cube-gamepad "Tier 2 — user
//   bindings & the companion control-plane surface")
//   input.controllers / input.bindings.get|set|reset / input.tuning.set
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_controllers_request_roundtrips() {
    let j = json!({"id": 70, "cmd": "input.controllers"});
    roundtrip_request(&j);
}

#[test]
fn input_bindings_get_request_global_roundtrips() {
    let j = json!({
        "id": 71, "cmd": "input.bindings.get",
        "vid_pid": "2dc8:9018", "scope": "global"
    });
    roundtrip_request(&j);
}

#[test]
fn input_bindings_get_request_game_scope_roundtrips() {
    // Per-game scope serializes as the externally-tagged `{"game": <app>}`.
    let j = json!({
        "id": 72, "cmd": "input.bindings.get",
        "vid_pid": "2dc8:9018", "scope": {"game": "cubeboy"}
    });
    roundtrip_request(&j);
}

#[test]
fn input_bindings_set_request_roundtrips() {
    // physical/action are canonical button config-names ("A", "ShoulderLeft", …).
    let j = json!({
        "id": 73, "cmd": "input.bindings.set",
        "vid_pid": "2dc8:9018", "physical": "A", "action": "B", "scope": "global"
    });
    roundtrip_request(&j);
}

#[test]
fn input_bindings_set_request_unbound_action_roundtrips() {
    // The action may be the "unbound" sentinel (drop the event).
    let j = json!({
        "id": 74, "cmd": "input.bindings.set",
        "vid_pid": "2dc8:9018", "physical": "Y", "action": "unbound",
        "scope": {"game": "cubeboy"}
    });
    roundtrip_request(&j);
}

#[test]
fn input_bindings_reset_request_roundtrips() {
    let j = json!({
        "id": 75, "cmd": "input.bindings.reset",
        "vid_pid": "2dc8:9018", "scope": "global"
    });
    roundtrip_request(&j);
}

#[test]
fn input_tuning_set_request_full_roundtrips() {
    // Exactly-representable f32 values so the f32→f64→JSON round-trip is stable.
    let j = json!({
        "id": 76, "cmd": "input.tuning.set",
        "vid_pid": "2dc8:9018",
        "dead_zone": 0.25, "stick_dpad_threshold": 0.5, "invert": false,
        "scope": "global"
    });
    roundtrip_request(&j);
}

#[test]
fn input_tuning_set_request_partial_omits_unset() {
    // All of dead_zone/stick_dpad_threshold/invert/scope are optional; an
    // unset field is omitted from the wire (the daemon defaults scope→Global).
    let j = json!({
        "id": 77, "cmd": "input.tuning.set",
        "vid_pid": "2dc8:9018", "dead_zone": 0.25
    });
    let back = roundtrip_request(&j);
    match serde_json::from_value::<Request>(back).expect("decode") {
        Request::InputTuningSet {
            dead_zone,
            stick_dpad_threshold,
            invert,
            scope,
            ..
        } => {
            assert_eq!(dead_zone, Some(0.25));
            assert_eq!(stick_dpad_threshold, None);
            assert_eq!(invert, None);
            assert_eq!(scope, None);
        }
        other => panic!("expected InputTuningSet, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: tier-2 input.binding_changed / input.tuning_changed / input.capture
// (cube-gamepad "Wire-format changes")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_binding_changed_event_roundtrips() {
    let j = json!({
        "event": "input.binding_changed",
        "vid_pid": "2dc8:9018",
        "scope": "global",
        "seq": 42,
        "physical": "A",
        "action": "B"
    });
    roundtrip_event(&j);
}

#[test]
fn input_binding_changed_reset_omits_physical_action() {
    // A scope reset emits binding_changed with no single physical/action.
    let j = json!({
        "event": "input.binding_changed",
        "vid_pid": "2dc8:9018",
        "scope": {"game": "cubeboy"},
        "seq": 43
    });
    let back = roundtrip_event(&j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputBindingChanged {
            physical,
            action,
            seq,
            ..
        } => {
            assert_eq!(physical, None);
            assert_eq!(action, None);
            assert_eq!(seq, 43);
        }
        other => panic!("expected InputBindingChanged, got {other:?}"),
    }
}

#[test]
fn binding_changed_carries_seq() {
    // Echo-suppression: the editing client matches the seq of its own write,
    // exactly like param.changed.
    let j = json!({
        "event": "input.binding_changed",
        "vid_pid": "2dc8:9018", "scope": "global", "seq": 7,
        "physical": "X", "action": "Y"
    });
    match serde_json::from_value::<Event>(j).expect("decode") {
        Event::InputBindingChanged { seq, vid_pid, .. } => {
            assert_eq!(seq, 7);
            assert_eq!(vid_pid, "2dc8:9018");
        }
        other => panic!("expected InputBindingChanged, got {other:?}"),
    }
}

#[test]
fn input_tuning_changed_event_roundtrips() {
    let j = json!({
        "event": "input.tuning_changed",
        "vid_pid": "2dc8:9018",
        "scope": "global",
        "seq": 12,
        "dead_zone": 0.25,
        "invert": true
    });
    roundtrip_event(&j);
}

#[test]
fn tuning_changed_carries_seq() {
    let j = json!({
        "event": "input.tuning_changed",
        "vid_pid": "2dc8:9018", "scope": "global", "seq": 99
    });
    match serde_json::from_value::<Event>(j).expect("decode") {
        Event::InputTuningChanged { seq, .. } => assert_eq!(seq, 99),
        other => panic!("expected InputTuningChanged, got {other:?}"),
    }
}

#[test]
fn input_sample_shape() {
    // The capture-tap event: {player, button, pressed}. button is a canonical
    // button config-name (the physical key the user pressed, pre-remap).
    let j = json!({"event": "input.capture", "player": 1, "button": "A", "pressed": true});
    roundtrip_event(&j);
}

#[test]
fn input_sample_with_raw_triple_roundtrips() {
    // The enriched capture-tap event carries the originating evdev triple so an
    // input visualiser shows raw + decoded together: here a dpad-via-
    // axis press arrives as ABS_HAT0Y = 255 yet decodes to the canonical DPadDown
    // — the exact shape that makes axis-encoding bugs obvious.
    let j = json!({
        "event": "input.capture", "player": 0, "button": "DPadDown", "pressed": true,
        "raw_type": "abs", "raw_code": "abs_hat0y", "raw_value": 255
    });
    let back = roundtrip_event(&j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputSample {
            button,
            pressed,
            raw_type,
            raw_code,
            raw_value,
            ..
        } => {
            assert_eq!(button, "DPadDown");
            assert!(pressed);
            assert_eq!(raw_type.as_deref(), Some("abs"));
            assert_eq!(raw_code.as_deref(), Some("abs_hat0y"));
            assert_eq!(raw_value, Some(255));
        }
        other => panic!("expected InputSample, got {other:?}"),
    }
}

#[test]
fn input_sample_released_roundtrips() {
    let j = json!({
        "event": "input.capture", "player": 0, "button": "ShoulderLeft", "pressed": false
    });
    let back = roundtrip_event(&j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputSample {
            player,
            button,
            pressed,
            raw_type,
            raw_code,
            raw_value,
        } => {
            assert_eq!(player, 0);
            assert_eq!(button, "ShoulderLeft");
            assert!(!pressed);
            // Legacy daemons omit the raw evdev triple; it defaults to None.
            assert_eq!(raw_type, None);
            assert_eq!(raw_code, None);
            assert_eq!(raw_value, None);
        }
        other => panic!("expected InputSample, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// input.controllers roster entry (ControllerInfo / ControllerProfile)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_controllers_roster_entry_recognized_roundtrips() {
    use cube_proto::{ControllerInfo, ControllerProfile};
    let j = json!({
        "player": 0,
        "name": "8BitDo SN30 Pro",
        "vid_pid": "2dc8:9018",
        "hw_id": "E4:17:D8:25:FB:42",
        "profile": "recognized",
        "connected": true
    });
    let info: ControllerInfo = serde_json::from_value(j.clone()).expect("ControllerInfo decode");
    assert_eq!(info.player, 0);
    assert_eq!(info.profile, ControllerProfile::Recognized);
    assert_eq!(info.vid_pid.as_deref(), Some("2dc8:9018"));
    // Stable per-device id: distinguishes two identical pads.
    assert_eq!(info.hw_id.as_deref(), Some("E4:17:D8:25:FB:42"));
    assert!(info.connected);
    assert_eq!(serde_json::to_value(&info).expect("encode"), j);
}

#[test]
fn input_controllers_roster_two_identical_pads_differ_by_hw_id() {
    use cube_proto::ControllerInfo;
    // Two controllers of the SAME model (identical name + vid_pid) are
    // distinguishable only by their stable per-device id.
    let a: ControllerInfo = serde_json::from_value(json!({
        "player": 0, "name": "8BitDo SN30 Pro", "vid_pid": "2dc8:9018",
        "hw_id": "E4:17:D8:25:FB:42", "profile": "recognized", "connected": true
    }))
    .expect("decode a");
    let b: ControllerInfo = serde_json::from_value(json!({
        "player": 1, "name": "8BitDo SN30 Pro", "vid_pid": "2dc8:9018",
        "hw_id": "E4:17:D8:25:FB:99", "profile": "recognized", "connected": true
    }))
    .expect("decode b");
    assert_eq!(a.name, b.name);
    assert_eq!(a.vid_pid, b.vid_pid);
    assert_ne!(
        a.hw_id, b.hw_id,
        "same model → the hw_id is the only discriminator"
    );
}

#[test]
fn input_controllers_roster_entry_generic_no_vid_pid_roundtrips() {
    use cube_proto::{ControllerInfo, ControllerProfile};
    // A pad with no USB VID:PID omits the field; profile is "generic".
    let j = json!({
        "player": 2,
        "name": "Generic Gamepad",
        "profile": "generic",
        "connected": false
    });
    let info: ControllerInfo = serde_json::from_value(j.clone()).expect("decode");
    assert_eq!(info.vid_pid, None);
    assert_eq!(info.profile, ControllerProfile::Generic);
    assert_eq!(serde_json::to_value(&info).expect("encode"), j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: power.state, config.reloaded
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn power_state_active_event_roundtrips() {
    let j = json!({"event": "power.state", "state": "active"});
    roundtrip_event(&j);
}

#[test]
fn power_state_blanked_event_roundtrips() {
    let j = json!({"event": "power.state", "state": "blanked"});
    roundtrip_event(&j);
}

#[test]
fn config_reloaded_event_roundtrips_arch_7_0() {
    let j = json!({"event": "config.reloaded"});
    roundtrip_event(&j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared value types — direct round-trip
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn damage_roundtrips() {
    let d = Damage {
        x: 0,
        y: 1,
        w: 384,
        h: 64,
    };
    let v = serde_json::to_value(d).unwrap();
    assert_eq!(v, json!({"x": 0, "y": 1, "w": 384, "h": 64}));
    let d2: Damage = serde_json::from_value(v).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn format_rgb565_roundtrips() {
    let f = Format::Rgb565;
    let v = serde_json::to_value(f).unwrap();
    assert_eq!(v, json!("RGB565"));
    let f2: Format = serde_json::from_value(v).unwrap();
    assert_eq!(f, f2);
}

#[test]
fn paramvalue_bool_roundtrips() {
    let p = ParamValue::Bool(true);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "bool", "value": true}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_int_roundtrips() {
    let p = ParamValue::Int(-42);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "int", "value": -42}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_float_roundtrips() {
    let p = ParamValue::Float(0.5);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "float", "value": 0.5}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_string_roundtrips() {
    let p = ParamValue::String("hello".into());
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "string", "value": "hello"}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_enum_roundtrips() {
    let p = ParamValue::Enum("classic".into());
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "enum", "value": "classic"}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_vec2_roundtrips() {
    let p = ParamValue::Vec2(Vec2 { x: 0.25, y: -0.75 });
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "vec2", "value": {"x": 0.25, "y": -0.75}}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_vec3_roundtrips() {
    let p = ParamValue::Vec3(Vec3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    });
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(
        v,
        json!({"type": "vec3", "value": {"x": 1.0, "y": 2.0, "z": 3.0}})
    );
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

// ─────────────────────────────────────────────────────────────────────────────
// — client overlays (acquire/release/dismissed) + `layer` on present +
//          the EPERM error code
//
// The CLIENT-provided overlay surface (a privileged client acquires an
// overlay layer and presents its own ARGB8888 frames into it) and the modal
// input grab:
//
//   - `Request::OverlayAcquire { id, z?, input? }` (cmd `"overlay.acquire"`).
//       `z` (optional, default 1) and `input` (optional, default `"none"`;
//       one of `"none" | "modal"`) — worked example.
//   - `Request::OverlayRelease { id, layer }` (cmd `"overlay.release"`).
//   - `Event::OverlayDismissed { layer, reason }` (event `"overlay.dismissed"`;
//       `reason ∈ "released" | "focus_lost" | "blanked"`).
//   - an optional `layer` field on `Request::Present` (absent / `0` ⇒ base
//       layer; an id returned by `overlay.acquire` ⇒ that overlay layer).
//   - `CubeErrno::EPERM` — the capability-denied error for an unauthorized
//       `overlay.acquire`.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_acquire_release_dismissed_roundtrip() {
    // worked example — acquire a modal overlay at z = 1:
    //   {"id":40,"cmd":"overlay.acquire","z":1,"input":"modal"}
    //      → {"id":40,"ok":true,"result":{"layer":7}}
    //   {"id":42,"cmd":"overlay.release","layer":7}
    //   {"event":"overlay.dismissed","layer":7,"reason":"released"}
    let acquire = json!({"id": 40, "cmd": "overlay.acquire", "z": 1, "input": "modal"});
    roundtrip_request(&acquire);

    // `z` (default 1) and `input` (default "none") are optional — the minimal
    // acquire omits both and must round-trip byte-for-byte (omitted fields stay
    // off the wire via skip_serializing_if).
    let acquire_min = json!({"id": 41, "cmd": "overlay.acquire"});
    roundtrip_request(&acquire_min);

    // `input:"none"` is the visual-only (no grab) variant.
    let acquire_none = json!({"id": 41, "cmd": "overlay.acquire", "input": "none"});
    roundtrip_request(&acquire_none);

    // The acquire OK response carries the assigned `{layer}` in `result`. The
    // result body is opaque JSON, so this already round-trips through `Response`
    // — it documents the wire shape the daemon returns.
    let acquire_ok = json!({"id": 40, "ok": true, "result": {"layer": 7}});
    roundtrip_response(&acquire_ok);

    // overlay.release names the layer to drop.
    let release = json!({"id": 42, "cmd": "overlay.release", "layer": 7});
    roundtrip_request(&release);

    // overlay.dismissed EVENT — all three reasons.
    for reason in ["released", "focus_lost", "blanked"] {
        let dismissed = json!({"event": "overlay.dismissed", "layer": 7, "reason": reason});
        roundtrip_event(&dismissed);
    }
}

#[test]
fn overlay_dismiss_reason_unknown_is_forward_compatible() {
    use cube_proto::{Event, OverlayDismissReason};

    // A reason token this build does not recognize (a future `cubed` reason, or
    // an SDK drift) must NOT hard-fail strict decode — the `#[serde(other)]`
    // catch-all maps it to `Unknown`. This is the shared
    // vocabulary both SDKs consume, so an unknown reason is handled identically
    // to any known one: stop presenting into the layer.
    let ev: Event = serde_json::from_value(
        json!({"event": "overlay.dismissed", "layer": 7, "reason": "evicted"}),
    )
    .expect("an unknown dismissal reason must decode, not error");
    match ev {
        Event::OverlayDismissed { layer, reason } => {
            assert_eq!(layer, 7);
            assert_eq!(reason, OverlayDismissReason::Unknown);
        }
        other => panic!("expected OverlayDismissed, got {other:?}"),
    }

    // `Unknown` is a first-class wire citizen: it serializes to `"unknown"` and
    // round-trips through that token byte-for-byte.
    roundtrip_event(&json!({"event": "overlay.dismissed", "layer": 7, "reason": "unknown"}));
    let back: Event = serde_json::from_value(
        json!({"event": "overlay.dismissed", "layer": 1, "reason": "unknown"}),
    )
    .expect("the `unknown` token itself decodes");
    assert!(matches!(
        back,
        Event::OverlayDismissed {
            reason: OverlayDismissReason::Unknown,
            ..
        }
    ));

    // The three known reasons are unaffected — still decode to their variants.
    for (token, want) in [
        ("released", OverlayDismissReason::Released),
        ("focus_lost", OverlayDismissReason::FocusLost),
        ("blanked", OverlayDismissReason::Blanked),
    ] {
        let ev: Event = serde_json::from_value(
            json!({"event": "overlay.dismissed", "layer": 3, "reason": token}),
        )
        .unwrap_or_else(|e| panic!("{token} decode: {e}"));
        match ev {
            Event::OverlayDismissed { reason, .. } => assert_eq!(reason, want),
            other => panic!("expected OverlayDismissed, got {other:?}"),
        }
    }
}

#[test]
fn present_with_layer_field_roundtrips() {
    //: `present` gains an optional `layer` selecting which compositor
    // layer the buffer updates. `layer:7` targets an acquired overlay layer:
    //   {"id":41,"cmd":"present","seq":1,"buffer_id":2,"layer":7}
    let to_overlay = json!({
        "id": 41,
        "cmd": "present",
        "seq": 1,
        "buffer_id": 2,
        "layer": 7
    });
    roundtrip_request(&to_overlay);

    // `layer:0` is the base layer (the v6 behaviour), and round-trips alongside
    // `damage`.
    let to_base = json!({
        "id": 11,
        "cmd": "present",
        "seq": 100,
        "buffer_id": 0,
        "damage": {"x": 0, "y": 0, "w": 384, "h": 64},
        "layer": 0
    });
    roundtrip_request(&to_base);
}

#[test]
fn eperm_error_roundtrips() {
    //: `overlay.acquire` from a client without overlay
    // capability returns EPERM. The closed `CubeErrno` set must gain
    // the `EPERM` variant; it serializes to/from its uppercase name.
    let code: cube_proto::CubeErrno =
        serde_json::from_value(json!("EPERM")).expect("CubeErrno must decode the \"EPERM\" token");
    assert_eq!(serde_json::to_value(code).unwrap(), json!("EPERM"));

    // …and round-trips inside an `{ok:false,error:{…}}` response envelope.
    let j = json!({
        "id": 40,
        "ok": false,
        "error": {
            "code": "EPERM",
            "message": "overlay capability denied",
            "context": {}
        }
    });
    roundtrip_response(&j);
}
