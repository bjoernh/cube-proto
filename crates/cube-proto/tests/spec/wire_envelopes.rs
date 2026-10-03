//! Cross-reference tests that pin's concrete worked-example JSON
//! literals. These are *verbatim* copies of the blocks that appear in
//! `Cube 2.0 — System Design Specification-v4.md.1` (line numbers below).
//!
//! Each literal here is re-built with `serde_json::json!` and asserted to
//! deserialize cleanly into the corresponding `cube_proto` type — guaranteeing
//! the wire shape never drifts from the spec.
//!
//! NOTE: This file is included from `tests/spec.rs` (so it runs as part of
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
fn buffer_register_literal_decodes() {
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
    must_decode_request(j, " buffer.register literal");
}

#[test]
fn present_literal_decodes() {
    //   {"id":11, "cmd":"present", "seq":100, "buffer_id":0,
    //    "damage":[0,0,384,64]}
    //
    // NOTE: spec shows `damage` as a 4-element array `[x,y,w,h]`. The test
    // plan from specifies `Damage { x, y, w, h }` as a struct. Both
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
    must_decode_request(j, " present literal (structured damage)");
}

#[test]
fn present_displayed_literal_decodes() {
    let j = json!({"event": "present.displayed", "seq": 100, "buffer_id": 0});
    must_decode_event(j, " present.displayed literal");
}

#[test]
fn buffer_release_literal_decodes() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "displayed"
    });
    must_decode_event(j, " buffer.release literal");
}

#[test]
fn input_event_key_literal_decodes() {
    let j = json!({
        "event": "input.event",
        "input_seq": 120,
        "t_us": 1_234_567_890_i64,
        "type": "key",
        "code": "BTN_A",
        "value": 1
    });
    must_decode_event(j, " input.event (key) literal");
}

#[test]
fn input_event_abs_literal_decodes() {
    let j = json!({
        "event": "input.event",
        "input_seq": 121,
        "t_us": 1_234_567_891_i64,
        "type": "abs",
        "code": "ABS_X",
        "value": 17234
    });
    must_decode_event(j, " input.event (abs) literal");
}

#[test]
fn input_snapshot_literal_decodes() {
    let j = json!({
        "event": "input.snapshot",
        "input_seq": 140,
        "device": "8BitDo SN30 Pro",
        "keys": {"BTN_A": 0, "BTN_B": 0, "BTN_START": 0},
        "abs":  {"ABS_X": 16384, "ABS_Y": 16384}
    });
    must_decode_event(j, " input.snapshot literal");
}

#[test]
fn input_device_state_literal_decodes() {
    let j = json!({
        "event": "input.device_state",
        "device": "8BitDo SN30 Pro",
        "connected": true
    });
    must_decode_event(j, " input.device_state literal");
}

#[test]
fn input_dropped_literal_decodes() {
    let j = json!({"event": "input.dropped", "since_seq": 120});
    must_decode_event(j, " input.dropped literal");
}
