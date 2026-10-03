//! Negative tests: malformed input must produce a structured error, never a
//! panic. Covers missing required fields, wrong-typed fields, unknown extra
//! fields (deny on requests per the test-plan default; allow on responses),
//! and the oversize-message helper from `lib.rs`.
//!
//! All assertions exercise the *public* serde-derived behaviour. The
//! implementation agent will satisfy these via `#[serde(deny_unknown_fields)]`
//! on request bodies and untagged-but-permissive responses (see lib.rs).

use serde_json::json;

use cube_proto::{
    CubeErrno, Event, MAX_MESSAGE_BYTES, ParamValue, Request, Response, enforce_max_size,
};

// ─────────────────────────────────────────────────────────────────────────────
// Missing required fields (request envelope)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hello_missing_protocol_version_is_err() {
    let j = json!({"id": 1, "cmd": "hello"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "hello without protocol_version must fail");
}

/// a `hello` whose major version
/// differs from [`cube_proto::PROTOCOL_MAJOR`] must be rejected with
/// `EVERSION`.
///
/// The actual rejection (parsing `protocol_version`, comparing majors, and
/// sending `EVERSION`) is `cubed`'s `control_plane::handshake` /
/// `is_compatible_version` — exercised end-to-end by
/// `cubed/tests/control_plane/handshake.rs::hello_version_mismatch_returns_eversion_and_closes`.
/// At the type level this crate pins: (1) an incompatible-major
/// `protocol_version` string still decodes as a structurally valid `Hello`
/// request (the version *value* is not type-checked — `EVERSION` is a
/// semantic, not syntactic, rejection); (2) `EVERSION` exists in
/// `CubeErrno` as the variant `cubed` returns for this case; and (3) major 1
/// (this crate's `PROTOCOL_MAJOR`) is the version family v6 belongs to —
/// `PROTOCOL_VERSION` starts with `"1."`.
#[test]
fn hello_incompatible_major_decodes_but_eversion_is_semantic() {
    let j = json!({"id": 1, "cmd": "hello", "protocol_version": "99.0"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(
        r.is_ok(),
        "an incompatible-major hello is structurally valid; EVERSION is cubed's semantic check"
    );
    let _ = CubeErrno::EVERSION;
    assert!(
        cube_proto::PROTOCOL_VERSION.starts_with(&format!("{}.", cube_proto::PROTOCOL_MAJOR)),
        "PROTOCOL_VERSION must be within the PROTOCOL_MAJOR family (v6 is a minor bump)"
    );
}

#[test]
fn register_missing_name_is_err() {
    let j = json!({"id": 2, "cmd": "register"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "register without name must fail");
}

#[test]
fn buffer_register_missing_size_is_err() {
    let j = json!({
        "id": 10,
        "cmd": "buffer.register",
        "buffer_id": 0,
        "format": "RGB565",
        "width": 384,
        "height": 64,
        "stride": 768
        // missing "size"
    });
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "buffer.register without size must fail");
}

#[test]
fn present_missing_seq_is_err() {
    let j = json!({"id": 11, "cmd": "present", "buffer_id": 0});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "present without seq must fail");
}

#[test]
fn set_missing_value_is_err() {
    let j = json!({"id": 22, "cmd": "set", "app": "snake", "key": "speed"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "set without value must fail");
}

#[test]
fn brightness_set_missing_value_is_err() {
    let j = json!({"id": 40, "cmd": "brightness.set"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "brightness.set without value must fail");
}

// ─────────────────────────────────────────────────────────────────────────────
// Wrong-typed fields
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_seq_wrong_type_is_err() {
    let j = json!({"id": 11, "cmd": "present", "seq": "100", "buffer_id": 0});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "present.seq must be integer");
}

#[test]
fn buffer_register_format_unknown_value_is_err() {
    let j = json!({
        "id": 10,
        "cmd": "buffer.register",
        "buffer_id": 0,
        "format": "RGBA9999",
        "width": 384,
        "height": 64,
        "stride": 768,
        "size": 49152
    });
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "unknown format must fail");
}

#[test]
fn paramvalue_int_with_string_payload_is_err() {
    let j = json!({"type": "int", "value": "not-an-int"});
    let r: Result<ParamValue, _> = serde_json::from_value(j);
    assert!(r.is_err(), "ParamValue::Int requires integer value");
}

#[test]
fn paramvalue_unknown_type_tag_is_err() {
    let j = json!({"type": "matrix", "value": [[0]]});
    let r: Result<ParamValue, _> = serde_json::from_value(j);
    assert!(r.is_err(), "unknown ParamValue type tag must fail");
}

#[test]
fn buffer_release_unknown_reason_is_err() {
    let j = json!({
        "event": "buffer.release",
        "buffer_id": 0,
        "seq": 100,
        "reason": "exploded"
    });
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "buffer.release with unknown reason must fail");
}

#[test]
fn power_state_unknown_state_is_err() {
    let j = json!({"event": "power.state", "state": "dimmed"});
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "power.state with unknown state must fail");
}

#[test]
fn focus_lost_unknown_reason_is_err() {
    let j = json!({"event": "focus.lost", "reason": "minimized"});
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "focus.lost with unknown reason must fail");
}

#[test]
fn focus_lost_missing_reason_is_err() {
    let j = json!({"event": "focus.lost"});
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "focus.lost without reason must fail");
}

#[test]
fn present_dropped_unknown_reason_is_err() {
    let j = json!({"event": "present.dropped", "seq": 1, "reason": "wrong_format"});
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "present.dropped with unknown reason must fail");
}

// ─────────────────────────────────────────────────────────────────────────────
// Unknown extra fields — requests deny, responses allow.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn request_with_unknown_field_is_err() {
    let j = json!({
        "id": 5,
        "cmd": "status",
        "app": "snake",
        "bogus_field": "x"
    });
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(
        r.is_err(),
        "requests use #[serde(deny_unknown_fields)] — extras must fail"
    );
}

#[test]
fn response_with_unknown_field_is_ok() {
    // Daemon must be free to add forward-compatible fields to responses.
    let j = json!({
        "id": 7,
        "ok": true,
        "result": {"any": "value"},
        "server_added_later": "future"
    });
    let r: Result<Response, _> = serde_json::from_value(j);
    assert!(
        r.is_ok(),
        "responses tolerate unknown fields for forward-compat"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Unknown command / event tag
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn unknown_request_cmd_is_err() {
    let j = json!({"id": 1, "cmd": "haxx"});
    let r: Result<Request, _> = serde_json::from_value(j);
    assert!(r.is_err(), "unknown cmd must fail");
}

#[test]
fn unknown_event_tag_is_err() {
    let j = json!({"event": "ghost"});
    let r: Result<Event, _> = serde_json::from_value(j);
    assert!(r.is_err(), "unknown event tag must fail");
}

// ─────────────────────────────────────────────────────────────────────────────
// MAX_MESSAGE_BYTES helper (64 KiB cap)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn max_message_bytes_is_64_kib() {
    assert_eq!(MAX_MESSAGE_BYTES, 65_536);
}

#[test]
fn enforce_max_size_accepts_small_buffer() {
    let bytes = b"{\"id\":1,\"cmd\":\"list\"}";
    assert!(enforce_max_size(bytes).is_ok());
}

#[test]
fn enforce_max_size_accepts_exact_limit() {
    let bytes = vec![b' '; MAX_MESSAGE_BYTES];
    assert!(enforce_max_size(&bytes).is_ok());
}

#[test]
fn enforce_max_size_rejects_oversize() {
    let bytes = vec![b' '; MAX_MESSAGE_BYTES + 1];
    let r = enforce_max_size(&bytes);
    assert!(r.is_err(), "oversize buffer must fail");
}

#[test]
fn realistic_wire_messages_fit_under_limit() {
    // Sanity: every routine wire object encodes well under 64 KiB.
    let samples = [
        json!({"id": 1, "cmd": "hello", "protocol_version": "1.0.0"}),
        json!({
            "id": 10, "cmd": "buffer.register", "buffer_id": 0,
            "format": "RGB565", "width": 384, "height": 64,
            "stride": 768, "size": 49152
        }),
        json!({"id": 11, "cmd": "present", "seq": 100, "buffer_id": 0,
               "damage": {"x": 0, "y": 0, "w": 384, "h": 64}}),
        json!({"event": "buffer.release", "buffer_id": 0, "seq": 1,
               "reason": "displayed"}),
    ];
    for s in &samples {
        let bytes = serde_json::to_vec(s).unwrap();
        assert!(bytes.len() < MAX_MESSAGE_BYTES, "sample exceeded cap");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sanity: every CubeErrno is reachable, including the request-policy code.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ebadreq_is_the_canonical_oversize_code() {
    // "Messages exceeding the limit return EBADREQ."
    // The implementation agent must make `enforce_max_size`'s error carry
    // or be representable as EBADREQ. This test pins the variant exists.
    let _ = CubeErrno::EBADREQ;
}
