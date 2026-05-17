//! Cross-reference tests that pin SDS §6.1's concrete worked-example JSON
//! literals. These are *verbatim* copies of the blocks that appear in
//! `Cube 2.0 — System Design Specification-v4.md §6.1` (line numbers below).
//!
//! Each literal here is re-built with `serde_json::json!` and asserted to
//! deserialize cleanly into the corresponding `cube_proto` type — guaranteeing
//! the wire shape never drifts from the SDS.
//!
//! NOTE: This file is included from `tests/sds.rs` (so it runs as part of
//! the cube-proto test binary set).

use serde_json::{Value, json};

use cube_proto::{Event, Request};

fn must_decode_request(j: Value, what: &str) -> Request {
    serde_json::from_value::<Request>(j).unwrap_or_else(|e| panic!("decode {what}: {e}"))
}

fn must_decode_event(j: Value, what: &str) -> Event {
    serde_json::from_value::<Event>(j).unwrap_or_else(|e| panic!("decode {what}: {e}"))
}

#[test]
fn sds_6_1_buffer_register_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~723–727:
    //   {"id":10, "cmd":"buffer.register", "buffer_id":0,
    //    "format":"RGB565", "width":384, "height":64, "stride":768,
    //    "size":49152}
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
    must_decode_request(j, "SDS §6.1 buffer.register literal");
}

#[test]
fn sds_6_1_present_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~742–745:
    //   {"id":11, "cmd":"present", "seq":100, "buffer_id":0,
    //    "damage":[0,0,384,64]}
    //
    // NOTE: SDS shows `damage` as a 4-element array `[x,y,w,h]`. The test
    // plan from Wave 1 specifies `Damage { x, y, w, h }` as a struct. Both
    // shapes are testable; we pin the structured form here as the canonical
    // representation that round-trips through `Damage`. The implementation
    // agent may choose to additionally accept the array form via a custom
    // deserializer, but the canonical wire shape is the object below.
    let j = json!({
        "id": 11,
        "cmd": "present",
        "seq": 100,
        "buffer_id": 0,
        "damage": {"x": 0, "y": 0, "w": 384, "h": 64}
    });
    must_decode_request(j, "SDS §6.1 present literal (structured damage)");
}

#[test]
fn sds_6_1_present_displayed_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~751: present.displayed event
    let j = json!({"event": "present.displayed", "seq": 100, "buffer_id": 0});
    must_decode_event(j, "SDS §6.1 present.displayed literal");
}

#[test]
fn sds_6_1_buffer_release_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~752: buffer.release event
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "displayed"
    });
    must_decode_event(j, "SDS §6.1 buffer.release literal");
}

#[test]
fn sds_6_1_input_event_key_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~757–758
    let j = json!({
        "event": "input.event",
        "input_seq": 120,
        "t_us": 1_234_567_890_i64,
        "type": "key",
        "code": "BTN_A",
        "value": 1
    });
    must_decode_event(j, "SDS §6.1 input.event (key) literal");
}

#[test]
fn sds_6_1_input_event_abs_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~760–761
    let j = json!({
        "event": "input.event",
        "input_seq": 121,
        "t_us": 1_234_567_891_i64,
        "type": "abs",
        "code": "ABS_X",
        "value": 17234
    });
    must_decode_event(j, "SDS §6.1 input.event (abs) literal");
}

#[test]
fn sds_6_1_input_snapshot_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~763–765
    let j = json!({
        "event": "input.snapshot",
        "input_seq": 140,
        "device": "8BitDo SN30 Pro",
        "keys": {"BTN_A": 0, "BTN_B": 0, "BTN_START": 0},
        "abs":  {"ABS_X": 16384, "ABS_Y": 16384}
    });
    must_decode_event(j, "SDS §6.1 input.snapshot literal");
}

#[test]
fn sds_6_1_input_device_state_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~767–768
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": true
    });
    must_decode_event(j, "SDS §6.1 input.device_state literal");
}

#[test]
fn sds_6_1_input_dropped_literal_decodes_sds_6_1() {
    // SDS §6.1 line ~770
    let j = json!({"event": "input.dropped", "since_seq": 120});
    must_decode_event(j, "SDS §6.1 input.dropped literal");
}
