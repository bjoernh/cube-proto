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
