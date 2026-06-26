//! Round-trip tests for every wire type in scope for local Unix SOCK_SEQPACKET
//! transport. Each test pins one concrete JSON shape (`serde_json::json!`),
//! deserializes it into the appropriate `cube_proto` type, re-serializes, and
//! compares the resulting `serde_json::Value` to the original.
//!
//! Naming convention: `<thing>_<form>_roundtrips_sds_<sec>_<sub>` /
//! `_arch_<sec>_<sub>` per the test plan.
//!
//! Deviations from the issue prompt:
//! - For `Request`/`Event` we wrap the SDS-literal JSON in the envelope keys
//!   they actually carry on the wire (`id`, `cmd`/`event`, etc.). The SDS shows
//!   most worked examples already complete; we keep them verbatim.
//! - Some commands ("focus", "stop", "restart", "journal", "preset.*",
//!   "brightness.set", "doctor.report", etc.) do not have inline SDS literals,
//!   so we synthesise minimal but normative-shape literals from §5.3 and
//!   ARCH §8.4 (command surface). Those literal shapes are the contract this
//!   test file pins for the implementation agent.

use serde_json::{Value, json};

use cube_proto::{Damage, Event, Format, HelloResult, ParamValue, Request, Response, Vec2, Vec3};

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

fn roundtrip_request(j: Value) -> Value {
    let req: Request = serde_json::from_value(j.clone()).expect("Request decode");
    let back = serde_json::to_value(&req).expect("Request encode");
    assert_eq!(back, j, "Request did not round-trip");
    // bytes round-trip too
    let bytes = serde_json::to_vec(&req).expect("Request to_vec");
    let req2: Request = serde_json::from_slice(&bytes).expect("Request from_slice");
    let back2 = serde_json::to_value(&req2).expect("Request encode #2");
    assert_eq!(back2, j, "Request bytes round-trip mismatch");
    back
}

fn roundtrip_event(j: Value) -> Value {
    let ev: Event = serde_json::from_value(j.clone()).expect("Event decode");
    let back = serde_json::to_value(&ev).expect("Event encode");
    assert_eq!(back, j, "Event did not round-trip");
    let bytes = serde_json::to_vec(&ev).expect("Event to_vec");
    let ev2: Event = serde_json::from_slice(&bytes).expect("Event from_slice");
    let back2 = serde_json::to_value(&ev2).expect("Event encode #2");
    assert_eq!(back2, j, "Event bytes round-trip mismatch");
    back
}

fn roundtrip_response(j: Value) -> Value {
    let r: Response = serde_json::from_value(j.clone()).expect("Response decode");
    let back = serde_json::to_value(&r).expect("Response encode");
    assert_eq!(back, j, "Response did not round-trip");
    let bytes = serde_json::to_vec(&r).expect("Response to_vec");
    let r2: Response = serde_json::from_slice(&bytes).expect("Response from_slice");
    let back2 = serde_json::to_value(&r2).expect("Response encode #2");
    assert_eq!(back2, j, "Response bytes round-trip mismatch");
    back
}

// ─────────────────────────────────────────────────────────────────────────────
// Session: hello, register   (SDS §5.3)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hello_request_roundtrips_sds_5_3() {
    let j = json!({
        "id": 1,
        "cmd": "hello",
        "protocol_version": "1.0.0"
    });
    roundtrip_request(j);
}

#[test]
fn hello_request_v6_minor_roundtrips_sds_5_3() {
    // SDS v6 delta §7: v6 is a minor bump within major 1; a `hello` quoting
    // the full v6 wire version (cube_proto::PROTOCOL_VERSION) round-trips
    // identically to any other protocol_version string.
    let j = json!({
        "id": 1,
        "cmd": "hello",
        "protocol_version": cube_proto::PROTOCOL_VERSION
    });
    roundtrip_request(j);
}

#[test]
fn hello_result_roundtrips_sds_5_3_v6_delta_7() {
    // SDS v6 delta §7 / cubekit spec §3.5: the hello OK response carries
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
fn hello_result_without_surface_omits_field_sds_5_3_v6_delta_7() {
    // `surface` is optional / `skip_serializing_if` so older clients that
    // only read `protocol_version` see a minimal body.
    let r = HelloResult {
        protocol_version: "1.1".to_string(),
        surface: None,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v, json!({"protocol_version": "1.1"}));
    let r2: HelloResult = serde_json::from_value(v).unwrap();
    assert_eq!(r, r2);
}

#[test]
fn response_ok_with_hello_result_roundtrips_sds_5_3_v6_delta_7() {
    let j = json!({
        "id": 1,
        "ok": true,
        "result": {"protocol_version": "1.1", "surface": "v6"}
    });
    roundtrip_response(j);
}

#[test]
fn register_request_roundtrips_sds_5_3() {
    let j = json!({
        "id": 2,
        "cmd": "register",
        "name": "snake",
        "mode": "supervised"
    });
    roundtrip_request(j);
}

#[test]
fn register_request_dev_mode_roundtrips_sds_5_3() {
    let j = json!({
        "id": 3,
        "cmd": "register",
        "name": "snake",
        "mode": "dev"
    });
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle: list, status, launch, stop, focus, restart, journal  (SDS §5.3, ARCH §8.4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_request_roundtrips_sds_5_3() {
    let j = json!({"id": 4, "cmd": "list"});
    roundtrip_request(j);
}

#[test]
fn status_request_roundtrips_sds_5_3() {
    let j = json!({"id": 5, "cmd": "status", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn status_request_no_app_roundtrips_sds_5_3() {
    let j = json!({"id": 6, "cmd": "status"});
    roundtrip_request(j);
}

#[test]
fn launch_request_roundtrips_sds_5_3() {
    let j = json!({"id": 7, "cmd": "launch", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn stop_request_roundtrips_sds_5_3() {
    let j = json!({"id": 8, "cmd": "stop", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn stop_request_no_app_roundtrips_sds_5_3() {
    let j = json!({"id": 9, "cmd": "stop"});
    roundtrip_request(j);
}

#[test]
fn focus_request_roundtrips_sds_5_3() {
    let j = json!({"id": 10, "cmd": "focus", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn restart_request_roundtrips_sds_5_3() {
    let j = json!({"id": 11, "cmd": "restart", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn journal_request_roundtrips_arch_8_4() {
    let j = json!({"id": 12, "cmd": "journal", "app": "snake", "follow": false, "tail": 100});
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Buffers: buffer.register      (SDS §6.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn buffer_register_request_roundtrips_sds_6_1() {
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
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Frames: present  (SDS §6.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_request_with_damage_roundtrips_sds_6_1() {
    let j = json!({
        "id": 11,
        "cmd": "present",
        "seq": 100,
        "buffer_id": 0,
        "damage": {"x": 0, "y": 0, "w": 384, "h": 64}
    });
    roundtrip_request(j);
}

#[test]
fn present_request_no_damage_roundtrips_sds_6_1() {
    let j = json!({
        "id": 12,
        "cmd": "present",
        "seq": 101,
        "buffer_id": 0
    });
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Parameters: get, get-all, set, set-many, describe, subscribe, unsubscribe (SDS §5.4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_request_roundtrips_sds_5_4() {
    let j = json!({"id": 20, "cmd": "get", "app": "snake", "key": "speed"});
    roundtrip_request(j);
}

#[test]
fn get_all_request_roundtrips_sds_5_4() {
    let j = json!({"id": 21, "cmd": "get-all", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn set_request_int_roundtrips_sds_5_4() {
    let j = json!({
        "id": 22,
        "cmd": "set",
        "app": "snake",
        "key": "speed",
        "value": {"type": "int", "value": 5}
    });
    roundtrip_request(j);
}

#[test]
fn set_request_color_roundtrips_sds_5_4() {
    let j = json!({
        "id": 23,
        "cmd": "set",
        "app": "snake",
        "key": "tint",
        "value": {"type": "color", "value": "#FF8800"}
    });
    roundtrip_request(j);
}

#[test]
fn set_many_request_roundtrips_sds_5_4() {
    let j = json!({
        "id": 24,
        "cmd": "set-many",
        "app": "snake",
        "values": {
            "speed": {"type": "int", "value": 5},
            "wrap":  {"type": "bool", "value": true}
        }
    });
    roundtrip_request(j);
}

#[test]
fn describe_request_one_key_roundtrips_sds_5_4() {
    let j = json!({"id": 25, "cmd": "describe", "app": "snake", "key": "speed"});
    roundtrip_request(j);
}

#[test]
fn describe_request_all_keys_roundtrips_sds_5_4() {
    let j = json!({"id": 26, "cmd": "describe", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn subscribe_request_roundtrips_sds_5_4() {
    let j = json!({"id": 27, "cmd": "subscribe", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn unsubscribe_request_roundtrips_sds_5_4() {
    let j = json!({"id": 28, "cmd": "unsubscribe", "app": "snake"});
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Presets: preset.list, preset.load, preset.save, preset.delete, preset.current,
//          preset.export, preset.import     (SDS §5.5)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn preset_list_request_roundtrips_sds_5_5() {
    let j = json!({"id": 30, "cmd": "preset.list", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn preset_load_request_roundtrips_sds_5_5() {
    let j = json!({"id": 31, "cmd": "preset.load", "app": "snake", "name": "default"});
    roundtrip_request(j);
}

#[test]
fn preset_save_request_roundtrips_sds_5_5() {
    let j = json!({"id": 32, "cmd": "preset.save", "app": "snake", "name": "fast"});
    roundtrip_request(j);
}

#[test]
fn preset_delete_request_roundtrips_sds_5_5() {
    let j = json!({"id": 33, "cmd": "preset.delete", "app": "snake", "name": "fast"});
    roundtrip_request(j);
}

#[test]
fn preset_current_request_roundtrips_sds_5_5() {
    let j = json!({"id": 34, "cmd": "preset.current", "app": "snake"});
    roundtrip_request(j);
}

#[test]
fn preset_export_request_roundtrips_sds_5_5() {
    let j = json!({"id": 35, "cmd": "preset.export", "app": "snake", "name": "default"});
    roundtrip_request(j);
}

#[test]
fn preset_import_request_roundtrips_sds_5_5() {
    let j = json!({
        "id": 36,
        "cmd": "preset.import",
        "app": "snake",
        "toml": "[meta]\napp=\"snake\"\nschema_version=1\npreset_name=\"x\"\n[params]\nspeed=5\n"
    });
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Brightness (SDS §5.10) and Diagnostics doctor.report (SDS §5.3, §11.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn brightness_set_request_roundtrips_sds_5_10() {
    let j = json!({"id": 40, "cmd": "brightness.set", "value": 128});
    roundtrip_request(j);
}

#[test]
fn brightness_get_request_roundtrips_sds_5_10() {
    let j = json!({"id": 41, "cmd": "brightness.get"});
    roundtrip_request(j);
}

#[test]
fn doctor_report_request_roundtrips_sds_11_1() {
    let j = json!({"id": 41, "cmd": "doctor.report"});
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Power (admin): power.blank, power.wake  (SDS v6 §5.12)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn power_blank_request_roundtrips_sds_5_12() {
    let j = json!({"id": 50, "cmd": "power.blank"});
    roundtrip_request(j);
}

#[test]
fn power_wake_request_roundtrips_sds_5_12() {
    let j = json!({"id": 51, "cmd": "power.wake"});
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Transitions on launch / focus  (SDS v7 §5.13, §6.1)
//
// W1 adds an OPTIONAL `transition: { kind: "cut"|"crossfade", duration_ms?: u32 }`
// field to `launch` and `focus`. These pin the exact wire shapes from SDS §6.1:
//
//   {"id":30,"cmd":"focus", "app":"snake",     "transition":{"kind":"crossfade","duration_ms":250}}
//   {"id":31,"cmd":"launch","app":"pixelflow", "transition":{"kind":"cut"}}
//
// RED expectation: `Request::{Launch,Focus}` carry no `transition` field yet, and
// the enum is `deny_unknown_fields`, so decoding either literal fails today. GREEN
// adds the optional field (skip_serializing_if = none, so the existing
// `launch_request_roundtrips_sds_5_3` / `focus_request_roundtrips_sds_5_3` — which
// omit `transition` — keep round-tripping byte-for-byte).
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn transition_field_roundtrips_on_launch_sds_6_1() {
    // `cut` is the hard-cut opt-out; `duration_ms` is omitted (optional).
    let j = json!({
        "id": 31,
        "cmd": "launch",
        "app": "pixelflow",
        "transition": {"kind": "cut"}
    });
    roundtrip_request(j);
}

#[test]
fn transition_field_roundtrips_on_focus_sds_6_1() {
    // `crossfade` with an explicit duration.
    let j = json!({
        "id": 30,
        "cmd": "focus",
        "app": "snake",
        "transition": {"kind": "crossfade", "duration_ms": 250}
    });
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// W5 — four NEW transition kinds (SDS v7 §5.13; plan "Add four compositor
// transition effects"). The wire `TransitionKind` reserves room for these
// ("`wipe` / `dissolve` / `push` are reserved … addable without a protocol
// change", request.rs:46). RED: until GREEN adds the variants the snake_case
// tokens fail to decode AT RUNTIME (unknown enum variant), which is the RED
// signal for this crate. Tokens: `dissolve`, `dip_to_black`, `particle_dissolve`,
// `push`.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn transition_dissolve_roundtrips_sds_5_13() {
    let j = json!({"id": 60, "cmd": "focus", "app": "snake", "transition": {"kind": "dissolve"}});
    roundtrip_request(j);
}

#[test]
fn transition_dip_to_black_roundtrips_sds_5_13() {
    let j = json!({
        "id": 61,
        "cmd": "launch",
        "app": "pixelflow",
        "transition": {"kind": "dip_to_black", "duration_ms": 400}
    });
    roundtrip_request(j);
}

#[test]
fn transition_particle_dissolve_roundtrips_sds_5_13() {
    let j = json!({
        "id": 62,
        "cmd": "focus",
        "app": "snake",
        "transition": {"kind": "particle_dissolve", "duration_ms": 600}
    });
    roundtrip_request(j);
}

#[test]
fn transition_push_roundtrips_sds_5_13() {
    let j = json!({"id": 63, "cmd": "focus", "app": "snake", "transition": {"kind": "push"}});
    roundtrip_request(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Wave 2 — ARGB8888 overlay format + admin text overlay  (SDS v7 §5.13, §6.1)
//
// W2 adds the overlay pixel format and the two admin-only system-text-overlay
// commands. RED contract (what GREEN must add to `cube-proto`):
//
//   - `Format::Argb8888` with the wire token `"ARGB8888"` (alongside `RGB565`).
//   - `Request::OverlayText { id, text, duration_ms?, z?, color? }`  (cmd
//     `"overlay.text"`) — system-rendered text overlay (no buffer/SCM_RIGHTS).
//   - `Request::OverlayClear { id, z? }`  (cmd `"overlay.clear"`).
//
// All three pin the exact JSON from SDS §6.1. They reference the new shapes only
// through `serde_json` so the binary keeps compiling: until GREEN lands the
// variants the decode FAILS AT RUNTIME (unknown `Format` variant / unknown
// `cmd`), which is the RED state.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn format_argb8888_roundtrips_sds_5_1() {
    // SDS v7 §5.13 / §6.1: overlay buffers use ARGB8888 (real alpha for `over`
    // compositing). The wire token is "ARGB8888". Decode-through `Format` so the
    // test compiles today and fails at runtime until the variant exists.
    let v = json!("ARGB8888");
    let f: Format =
        serde_json::from_value(v.clone()).expect("Format must decode the \"ARGB8888\" token");
    let back = serde_json::to_value(&f).expect("encode");
    assert_eq!(back, v, "ARGB8888 round-trips to the same wire token");
    // RGB565 still decodes (the new variant is additive, not a replacement).
    let r: Format = serde_json::from_value(json!("RGB565")).expect("RGB565 still decodes");
    assert_ne!(
        serde_json::to_value(&r).unwrap(),
        back,
        "ARGB8888 and RGB565 are distinct variants"
    );
}

#[test]
fn overlay_text_command_roundtrips_sds_5_13() {
    // SDS §6.1 worked example — admin-only system text overlay (all fields):
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
    roundtrip_request(j);

    // `duration_ms` / `z` / `color` are all optional (§6.1: "z (default 1) and
    // color (default white) are optional"; duration_ms `0` = sticky). The
    // minimal form omits them and must round-trip byte-for-byte (the omitted
    // fields stay off the wire via skip_serializing_if).
    let minimal = json!({
        "id": 51,
        "cmd": "overlay.text",
        "text": "x"
    });
    roundtrip_request(minimal);
}

#[test]
fn overlay_clear_command_roundtrips_sds_5_13() {
    // SDS §6.1: {"id":51,"cmd":"overlay.clear","z":1}.
    let j = json!({"id": 52, "cmd": "overlay.clear", "z": 1});
    roundtrip_request(j);

    // `z` is optional — "overlay.clear with no z clears all admin text
    // overlays" (§6.1). The clear-all form omits z and round-trips.
    let clear_all = json!({"id": 53, "cmd": "overlay.clear"});
    roundtrip_request(clear_all);
}

// ─────────────────────────────────────────────────────────────────────────────
// Response envelope  (SDS §5.3)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn response_ok_with_result_roundtrips_sds_5_3() {
    let j = json!({"id": 7, "ok": true, "result": {"any": "value"}});
    roundtrip_response(j);
}

#[test]
fn response_ok_empty_result_roundtrips_sds_5_3() {
    // `buffer.register` returns just `{"id":10,"ok":true}` per SDS §6.1
    let j = json!({"id": 10, "ok": true});
    roundtrip_response(j);
}

#[test]
fn response_error_roundtrips_sds_5_3() {
    let j = json!({
        "id": 99,
        "ok": false,
        "error": {
            "code": "ENOAPP",
            "message": "no such app",
            "context": {"app": "ghost"}
        }
    });
    roundtrip_response(j);
}

#[test]
fn response_preset_import_warnings_roundtrips_sds_5_5() {
    // SDS §5.5 explicit worked example.
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
    roundtrip_response(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: app.started, app.stopped  (SDS §5.3)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn app_started_event_roundtrips_sds_5_3() {
    let j = json!({"event": "app.started", "app": "snake"});
    roundtrip_event(j);
}

#[test]
fn app_stopped_event_roundtrips_sds_5_3() {
    // v5 wire shape: no `reason` field. `reason` is `#[serde(default,
    // skip_serializing_if = "Option::is_none")]` so this v5-shaped literal
    // still round-trips byte-for-byte (backward compat, SDS v6 delta §7).
    let j = json!({"event": "app.stopped", "app": "snake"});
    roundtrip_event(j);
}

#[test]
fn app_stopped_event_with_reason_roundtrips_sds_6_2() {
    let j = json!({"event": "app.stopped", "app": "snake", "reason": "control_lost"});
    roundtrip_event(j);
}

#[test]
fn app_stopped_event_with_frame_stream_idle_reason_roundtrips_sds_6_2() {
    // SDS v6 §6.2 / item 2.7: the `frame_stream_idle` teardown reason added
    // to the `app.stopped` wire vocabulary.
    let j = json!({"event": "app.stopped", "app": "snake", "reason": "frame_stream_idle"});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: focus.lost, focus.gained  (SDS v6 §5.2, §6.1 — reliable-tier)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn focus_lost_event_app_switch_roundtrips_sds_6_1() {
    let j = json!({"event": "focus.lost", "reason": "app_switch"});
    roundtrip_event(j);
}

#[test]
fn focus_lost_event_home_roundtrips_sds_6_1() {
    let j = json!({"event": "focus.lost", "reason": "home"});
    roundtrip_event(j);
}

#[test]
fn focus_lost_event_stopping_roundtrips_sds_6_1() {
    let j = json!({"event": "focus.lost", "reason": "stopping"});
    roundtrip_event(j);
}

#[test]
fn focus_gained_event_roundtrips_sds_6_1() {
    let j = json!({"event": "focus.gained"});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: present.displayed, buffer.release × 5 reasons  (SDS §5.1, §6.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_displayed_event_roundtrips_sds_6_1() {
    let j = json!({
        "event": "present.displayed",
        "seq": 100,
        "buffer_id": 0
    });
    roundtrip_event(j);
}

#[test]
fn buffer_release_displayed_reason_roundtrips_sds_5_1() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "displayed"
    });
    roundtrip_event(j);
}

#[test]
fn buffer_release_dropped_reason_roundtrips_sds_5_1() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "dropped"
    });
    roundtrip_event(j);
}

#[test]
fn buffer_release_replaced_reason_roundtrips_sds_5_1() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "replaced"
    });
    roundtrip_event(j);
}

#[test]
fn buffer_release_focus_lost_reason_roundtrips_sds_5_1() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "focus_lost"
    });
    roundtrip_event(j);
}

#[test]
fn buffer_release_blanked_reason_roundtrips_sds_5_12() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "blanked"
    });
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: param.changed, params.changed, events.dropped   (SDS §5.4, §5.3)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn param_changed_event_roundtrips_sds_5_4() {
    let j = json!({
        "event": "param.changed",
        "app": "snake",
        "seq": 17,
        "key": "speed",
        "value": {"type": "int", "value": 5}
    });
    roundtrip_event(j);
}

#[test]
fn params_changed_event_roundtrips_sds_5_4() {
    let j = json!({
        "event": "params.changed",
        "app": "snake",
        "seq": 18,
        "values": {
            "speed": {"type": "int", "value": 5},
            "wrap":  {"type": "bool", "value": true}
        }
    });
    roundtrip_event(j);
}

#[test]
fn events_dropped_event_roundtrips_sds_5_3() {
    let j = json!({"event": "events.dropped", "since_seq": 1234});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: input.event (key + abs), input.snapshot, input.device_state,
//         input.dropped   (SDS §5.7.1, §6.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_event_key_roundtrips_sds_6_1() {
    let j = json!({
        "event": "input.event",
        "input_seq": 120,
        "t_us": 1_234_567_890_i64,
        "type": "key",
        "code": "BTN_A",
        "value": 1,
        "player": 0
    });
    roundtrip_event(j);
}

#[test]
fn input_event_abs_roundtrips_sds_6_1() {
    let j = json!({
        "event": "input.event",
        "input_seq": 121,
        "t_us": 1_234_567_891_i64,
        "type": "abs",
        "code": "ABS_X",
        "value": 17234,
        "player": 0
    });
    roundtrip_event(j);
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
fn input_snapshot_event_roundtrips_sds_6_1() {
    let j = json!({
        "event": "input.snapshot",
        "input_seq": 140,
        "device": "8BitDo SN30 Pro",
        "keys": {"BTN_A": 0, "BTN_B": 0, "BTN_START": 0},
        "abs":  {"ABS_X": 16384, "ABS_Y": 16384},
        "player": 0
    });
    roundtrip_event(j);
}

#[test]
fn input_device_state_event_connected_roundtrips_sds_6_1() {
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": true
    });
    roundtrip_event(j);
}

#[test]
fn input_device_state_event_disconnected_roundtrips_sds_6_1() {
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": false
    });
    roundtrip_event(j);
}

#[test]
fn input_dropped_event_roundtrips_sds_6_1() {
    let j = json!({"event": "input.dropped", "since_seq": 120, "player": 0});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Events: input.player_connected / input.player_disconnected
//         (cube-gamepad "Wire-format changes")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_player_connected_roundtrips() {
    let j = json!({
        "event": "input.player_connected",
        "player": 1,
        "name": "8BitDo SN30 Pro",
        "vid_pid": "2dc8:9018"
    });
    roundtrip_event(j);
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
    let back = roundtrip_event(j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputPlayerConnected { player, vid_pid, .. } => {
            assert_eq!(player, 2);
            assert_eq!(vid_pid, None, "absent vid_pid decodes to None");
        }
        other => panic!("expected InputPlayerConnected, got {other:?}"),
    }
}

#[test]
fn input_player_disconnected_roundtrips() {
    let j = json!({"event": "input.player_disconnected", "player": 1});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Requests: tier-2 input bindings / tuning (cube-gamepad "Tier 2 — user
//   bindings & the companion control-plane surface")
//   input.controllers / input.bindings.get|set|reset / input.tuning.set
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_controllers_request_roundtrips() {
    let j = json!({"id": 70, "cmd": "input.controllers"});
    roundtrip_request(j);
}

#[test]
fn input_bindings_get_request_global_roundtrips() {
    let j = json!({
        "id": 71, "cmd": "input.bindings.get",
        "vid_pid": "2dc8:9018", "scope": "global"
    });
    roundtrip_request(j);
}

#[test]
fn input_bindings_get_request_game_scope_roundtrips() {
    // Per-game scope serializes as the externally-tagged `{"game": <app>}`.
    let j = json!({
        "id": 72, "cmd": "input.bindings.get",
        "vid_pid": "2dc8:9018", "scope": {"game": "cubeboy"}
    });
    roundtrip_request(j);
}

#[test]
fn input_bindings_set_request_roundtrips() {
    // physical/action are canonical button config-names ("A", "ShoulderLeft", …).
    let j = json!({
        "id": 73, "cmd": "input.bindings.set",
        "vid_pid": "2dc8:9018", "physical": "A", "action": "B", "scope": "global"
    });
    roundtrip_request(j);
}

#[test]
fn input_bindings_set_request_unbound_action_roundtrips() {
    // The action may be the "unbound" sentinel (drop the event).
    let j = json!({
        "id": 74, "cmd": "input.bindings.set",
        "vid_pid": "2dc8:9018", "physical": "Y", "action": "unbound",
        "scope": {"game": "cubeboy"}
    });
    roundtrip_request(j);
}

#[test]
fn input_bindings_reset_request_roundtrips() {
    let j = json!({
        "id": 75, "cmd": "input.bindings.reset",
        "vid_pid": "2dc8:9018", "scope": "global"
    });
    roundtrip_request(j);
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
    roundtrip_request(j);
}

#[test]
fn input_tuning_set_request_partial_omits_unset() {
    // All of dead_zone/stick_dpad_threshold/invert/scope are optional; an
    // unset field is omitted from the wire (the daemon defaults scope→Global).
    let j = json!({
        "id": 77, "cmd": "input.tuning.set",
        "vid_pid": "2dc8:9018", "dead_zone": 0.25
    });
    let back = roundtrip_request(j);
    match serde_json::from_value::<Request>(back).expect("decode") {
        Request::InputTuningSet {
            dead_zone, stick_dpad_threshold, invert, scope, ..
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
//   (cube-gamepad "Wire-format changes")
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
    roundtrip_event(j);
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
    let back = roundtrip_event(j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputBindingChanged { physical, action, seq, .. } => {
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
    roundtrip_event(j);
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
    roundtrip_event(j);
}

#[test]
fn input_sample_with_raw_triple_roundtrips() {
    // The enriched capture-tap event carries the originating evdev triple so an
    // input visualiser shows raw + decoded together (cube#7): here a dpad-via-
    // axis press arrives as ABS_HAT0Y = 255 yet decodes to the canonical DPadDown
    // — the exact shape that makes axis-encoding bugs (cube#5) obvious.
    let j = json!({
        "event": "input.capture", "player": 0, "button": "DPadDown", "pressed": true,
        "raw_type": "abs", "raw_code": "abs_hat0y", "raw_value": 255
    });
    let back = roundtrip_event(j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputSample { button, pressed, raw_type, raw_code, raw_value, .. } => {
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
    let back = roundtrip_event(j);
    match serde_json::from_value::<Event>(back).expect("decode") {
        Event::InputSample { player, button, pressed, raw_type, raw_code, raw_value } => {
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
// input.controllers roster entry  (ControllerInfo / ControllerProfile)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn input_controllers_roster_entry_recognized_roundtrips() {
    use cube_proto::{ControllerInfo, ControllerProfile};
    let j = json!({
        "player": 0,
        "name": "8BitDo SN30 Pro",
        "vid_pid": "2dc8:9018",
        "profile": "recognized",
        "connected": true
    });
    let info: ControllerInfo = serde_json::from_value(j.clone()).expect("ControllerInfo decode");
    assert_eq!(info.player, 0);
    assert_eq!(info.profile, ControllerProfile::Recognized);
    assert_eq!(info.vid_pid.as_deref(), Some("2dc8:9018"));
    assert!(info.connected);
    assert_eq!(serde_json::to_value(&info).expect("encode"), j);
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
// Events: power.state, config.reloaded   (SDS §5.12, ARCH §7)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn power_state_active_event_roundtrips_sds_5_12() {
    let j = json!({"event": "power.state", "state": "active"});
    roundtrip_event(j);
}

#[test]
fn power_state_blanked_event_roundtrips_sds_5_12() {
    let j = json!({"event": "power.state", "state": "blanked"});
    roundtrip_event(j);
}

#[test]
fn config_reloaded_event_roundtrips_arch_7_0() {
    let j = json!({"event": "config.reloaded"});
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared value types — direct round-trip   (SDS §6.1, §5.4)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn damage_roundtrips_sds_6_1() {
    let d = Damage {
        x: 0,
        y: 1,
        w: 384,
        h: 64,
    };
    let v = serde_json::to_value(&d).unwrap();
    assert_eq!(v, json!({"x": 0, "y": 1, "w": 384, "h": 64}));
    let d2: Damage = serde_json::from_value(v).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn format_rgb565_roundtrips_sds_6_1() {
    let f = Format::Rgb565;
    let v = serde_json::to_value(&f).unwrap();
    assert_eq!(v, json!("RGB565"));
    let f2: Format = serde_json::from_value(v).unwrap();
    assert_eq!(f, f2);
}

#[test]
fn paramvalue_bool_roundtrips_sds_5_4() {
    let p = ParamValue::Bool(true);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "bool", "value": true}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_int_roundtrips_sds_5_4() {
    let p = ParamValue::Int(-42);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "int", "value": -42}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_float_roundtrips_sds_5_4() {
    let p = ParamValue::Float(0.5);
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "float", "value": 0.5}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_string_roundtrips_sds_5_4() {
    let p = ParamValue::String("hello".into());
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "string", "value": "hello"}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_enum_roundtrips_sds_5_4() {
    let p = ParamValue::Enum("classic".into());
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "enum", "value": "classic"}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_vec2_roundtrips_sds_5_4() {
    let p = ParamValue::Vec2(Vec2 { x: 0.25, y: -0.75 });
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v, json!({"type": "vec2", "value": {"x": 0.25, "y": -0.75}}));
    let p2: ParamValue = serde_json::from_value(v).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn paramvalue_vec3_roundtrips_sds_5_4() {
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
// Wave 3 — client overlays (acquire/release/dismissed) + `layer` on present +
//          the EPERM error code  (SDS v7 §5.13, §6.1)
//
// W3 adds the CLIENT-provided overlay surface (a privileged client acquires an
// overlay layer and presents its own ARGB8888 frames into it) and the modal
// input grab. RED contract — what GREEN must add to `cube-proto`:
//
//   - `Request::OverlayAcquire { id, z?, input? }`  (cmd `"overlay.acquire"`).
//       `z` (optional, default 1) and `input` (optional, default `"none"`;
//       one of `"none" | "modal"`) — SDS §6.1 worked example.
//   - `Request::OverlayRelease { id, layer }`       (cmd `"overlay.release"`).
//   - `Event::OverlayDismissed { layer, reason }`   (event `"overlay.dismissed"`;
//       `reason ∈ "released" | "focus_lost" | "blanked"`).
//   - an optional `layer` field on `Request::Present` (absent / `0` ⇒ base
//       layer; an id returned by `overlay.acquire` ⇒ that overlay layer).
//   - `CubeErrno::EPERM` — the capability-denied error for an unauthorized
//       `overlay.acquire` (SDS §5.13 / §6.1).
//
// Each test references the new shapes only through the existing
// `Request`/`Event`/`Response`/`CubeErrno` types, so the binary keeps compiling;
// each one FAILS AT RUNTIME today (unknown `cmd`/`event`, the `deny_unknown_fields`
// `layer` rejection, or the unknown `EPERM` token), which is the RED state.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overlay_acquire_release_dismissed_roundtrip_sds_5_13() {
    // SDS §6.1 worked example — acquire a modal overlay at z = 1:
    //   {"id":40,"cmd":"overlay.acquire","z":1,"input":"modal"}
    //      → {"id":40,"ok":true,"result":{"layer":7}}
    //   {"id":42,"cmd":"overlay.release","layer":7}
    //   {"event":"overlay.dismissed","layer":7,"reason":"released"}
    let acquire = json!({"id": 40, "cmd": "overlay.acquire", "z": 1, "input": "modal"});
    roundtrip_request(acquire);

    // `z` (default 1) and `input` (default "none") are optional — the minimal
    // acquire omits both and must round-trip byte-for-byte (omitted fields stay
    // off the wire via skip_serializing_if).
    let acquire_min = json!({"id": 41, "cmd": "overlay.acquire"});
    roundtrip_request(acquire_min);

    // `input:"none"` is the visual-only (no grab) variant.
    let acquire_none = json!({"id": 41, "cmd": "overlay.acquire", "input": "none"});
    roundtrip_request(acquire_none);

    // The acquire OK response carries the assigned `{layer}` in `result`. The
    // result body is opaque JSON, so this already round-trips through `Response`
    // — it documents the wire shape GREEN's handler returns.
    let acquire_ok = json!({"id": 40, "ok": true, "result": {"layer": 7}});
    roundtrip_response(acquire_ok);

    // overlay.release names the layer to drop.
    let release = json!({"id": 42, "cmd": "overlay.release", "layer": 7});
    roundtrip_request(release);

    // overlay.dismissed EVENT — all three SDS §6.1 reasons.
    for reason in ["released", "focus_lost", "blanked"] {
        let dismissed = json!({"event": "overlay.dismissed", "layer": 7, "reason": reason});
        roundtrip_event(dismissed);
    }
}

#[test]
fn present_with_layer_field_roundtrips_sds_5_1() {
    // SDS §6.1: `present` gains an optional `layer` selecting which compositor
    // layer the buffer updates. `layer:7` targets an acquired overlay layer:
    //   {"id":41,"cmd":"present","seq":1,"buffer_id":2,"layer":7}
    let to_overlay = json!({
        "id": 41,
        "cmd": "present",
        "seq": 1,
        "buffer_id": 2,
        "layer": 7
    });
    roundtrip_request(to_overlay);

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
    roundtrip_request(to_base);
}

#[test]
fn eperm_error_roundtrips_sds_5_13() {
    // SDS §5.13 / §6.1: `overlay.acquire` from a client without overlay
    // capability returns EPERM. The closed `CubeErrno` set (SDS §5.3) must gain
    // the `EPERM` variant; it serializes to/from its uppercase name.
    let code: cube_proto::CubeErrno =
        serde_json::from_value(json!("EPERM")).expect("CubeErrno must decode the \"EPERM\" token");
    assert_eq!(serde_json::to_value(&code).unwrap(), json!("EPERM"));

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
    roundtrip_response(j);
}
