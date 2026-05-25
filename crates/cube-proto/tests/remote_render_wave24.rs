//! Wave 24 — cube-proto extensions for remote rendering (RED tests).
//!
//! These tests pin the wire shapes for the Phase 3 remote-render protocol
//! extensions before any of the corresponding types exist. They are expected
//! to fail to compile until Wave 24 implementation lands.
//!
//! Naming convention: `<thing>_roundtrips_sds_<sec>_<sub>`. SDS sections:
//! - `_sds_6_1_` — JSON control-message extensions (hello.frame, register).
//! - `_sds_6_2_` — Binary frame stream (FrameHeader, present.dropped,
//!   remote.frames_dropped, payload size, session token).
//! - `_sds_5_11_` — Client stats (`client.stats`).

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use cube_proto::frame_header::{
    FRAME_HEADER_BYTES, FRAME_HEADER_MAGIC, FrameHeader, FrameHeaderError, SeqGuard,
};
use cube_proto::session_token::{SessionTokenError, validate as validate_session_token};
use cube_proto::{Event, REMOTE_FRAME_PAYLOAD_BYTES, Request};

// ─────────────────────────────────────────────────────────────────────────────
// helpers (mirrors tests/wire_roundtrip.rs)
// ─────────────────────────────────────────────────────────────────────────────

fn roundtrip_request(j: Value) -> Value {
    let req: Request = serde_json::from_value(j.clone()).expect("Request decode");
    let back = serde_json::to_value(&req).expect("Request encode");
    assert_eq!(back, j, "Request did not round-trip");
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

// ─────────────────────────────────────────────────────────────────────────────
// 1. Event::PresentDropped  (SDS §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn present_dropped_too_late_roundtrips_sds_6_2() {
    let j = serde_json::json!({
        "event": "present.dropped",
        "seq": 99,
        "reason": "too_late"
    });
    let e: Event = serde_json::from_value(j.clone()).expect("decode");
    assert_eq!(serde_json::to_value(&e).unwrap(), j);
}

#[test]
fn present_dropped_fragmented_roundtrips_sds_6_2() {
    let j = serde_json::json!({
        "event": "present.dropped",
        "seq": 99,
        "reason": "fragmented"
    });
    let e: Event = serde_json::from_value(j.clone()).expect("decode");
    assert_eq!(serde_json::to_value(&e).unwrap(), j);
}

#[test]
fn present_dropped_focus_lost_roundtrips_sds_6_2_present() {
    let j = json!({
        "event": "present.dropped",
        "seq": 44,
        "reason": "focus_lost",
    });
    roundtrip_event(j);
}

#[test]
fn present_dropped_blanked_roundtrips_sds_6_2_present() {
    let j = json!({
        "event": "present.dropped",
        "seq": 45,
        "reason": "blanked",
    });
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Event::RemoteFramesDropped  (SDS §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn remote_frames_dropped_roundtrips_sds_6_2_counters() {
    let j = json!({
        "event": "remote.frames_dropped",
        "client": "snake",
        "dropped": 17,
        "since_seq": 100,
    });
    roundtrip_event(j);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Event::ClientStats  (SDS §5.11, §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn client_stats_with_latency_roundtrips_sds_5_11_stats() {
    let j = json!({
        "event": "client.stats",
        "client": "snake",
        "remote_sender_dropped": 4,
        "last_seq": 999,
        "video_latency_us": 18_500,
    });
    roundtrip_event(j);
}

#[test]
fn client_stats_without_latency_roundtrips_sds_5_11_stats() {
    let j = json!({
        "event": "client.stats",
        "client": "snake",
        "remote_sender_dropped": 0,
        "last_seq": 1,
    });
    // Round-trip and additionally assert the key is omitted when None.
    let back = roundtrip_event(j.clone());
    let obj = back.as_object().expect("event is a JSON object");
    assert!(
        !obj.contains_key("video_latency_us"),
        "video_latency_us must be omitted when None (skip_serializing_if), got: {back}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Request::Register backwards-compat session_token  (SDS §6.1)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn register_without_session_token_roundtrips_sds_6_1_register() {
    let j = json!({
        "id": 1,
        "cmd": "register",
        "name": "snake",
        "mode": "dev",
    });
    let back = roundtrip_request(j.clone());
    let obj = back.as_object().expect("request is a JSON object");
    assert!(
        !obj.contains_key("session_token"),
        "session_token must be omitted when None (skip_serializing_if), got: {back}"
    );
}

#[test]
fn register_with_session_token_roundtrips_sds_6_1_register() {
    let j = json!({
        "id": 2,
        "cmd": "register",
        "name": "snake",
        "mode": "dev",
        "session_token": "abcdefghijklmnopqrstuvwxyz012345",
    });
    let back = roundtrip_request(j.clone());
    assert_eq!(
        back.get("session_token").and_then(Value::as_str),
        Some("abcdefghijklmnopqrstuvwxyz012345"),
        "session_token must be preserved verbatim"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. cube_proto::frame_header  (SDS §6.2)
// ─────────────────────────────────────────────────────────────────────────────

fn good_header(seq: u32) -> FrameHeader {
    FrameHeader {
        magic: FRAME_HEADER_MAGIC,
        seq,
        reserved2: 0,
        reserved: 0,
    }
}

#[test]
fn frame_header_constants_sds_6_2_layout() {
    assert_eq!(FRAME_HEADER_MAGIC, 0x4355_4246, "magic = 'CUBF' big-endian");
    assert_eq!(FRAME_HEADER_BYTES, 16, "header is exactly 16 bytes");
}

#[test]
fn frame_header_encode_decode_roundtrips_sds_6_2_layout() {
    let h = good_header(1);
    let bytes = h.encode();
    assert_eq!(bytes.len(), FRAME_HEADER_BYTES);
    let h2 = FrameHeader::decode(&bytes).expect("decode good header");
    assert_eq!(h2.magic, h.magic);
    assert_eq!(h2.seq, h.seq);
    assert_eq!(h2.reserved2, h.reserved2);
    assert_eq!(h2.reserved, h.reserved);
}

#[test]
fn frame_header_encodes_little_endian_magic_sds_6_2_layout() {
    let h = good_header(1);
    let bytes = h.encode();
    // 0x43554246 little-endian = [0x46, 0x42, 0x55, 0x43]
    assert_eq!(
        &bytes[..4],
        &[0x46, 0x42, 0x55, 0x43],
        "magic must be little-endian on the wire"
    );
}

#[test]
fn frame_header_rejects_bad_magic_sds_6_2_layout() {
    let h = FrameHeader {
        magic: 0xDEAD_BEEF,
        seq: 1,
        reserved2: 0,
        reserved: 0,
    };
    let bytes = h.encode();
    let err = FrameHeader::decode(&bytes).expect_err("bad magic must error");
    assert!(matches!(err, FrameHeaderError::BadMagic), "got {err:?}");
}

#[test]
fn frame_header_rejects_nonzero_reserved_sds_6_2_layout() {
    // Non-zero at offset 8 (reserved2)
    let h = FrameHeader {
        magic: FRAME_HEADER_MAGIC,
        seq: 1,
        reserved2: 1,
        reserved: 0,
    };
    let bytes = h.encode();
    let err = FrameHeader::decode(&bytes).expect_err("nonzero reserved2 must error");
    assert!(
        matches!(err, FrameHeaderError::NonZeroReserved),
        "got {err:?}"
    );

    // Non-zero at offset 12 (reserved)
    let h2 = FrameHeader {
        magic: FRAME_HEADER_MAGIC,
        seq: 1,
        reserved2: 0,
        reserved: 1,
    };
    let bytes2 = h2.encode();
    let err2 = FrameHeader::decode(&bytes2).expect_err("nonzero reserved must error");
    assert!(
        matches!(err2, FrameHeaderError::NonZeroReserved),
        "got {err2:?}"
    );
}

#[test]
fn frame_header_rejects_short_buffer_sds_6_2_layout() {
    let bytes = [0u8; 15];
    let err = FrameHeader::decode(&bytes).expect_err("short buffer must error");
    assert!(matches!(err, FrameHeaderError::ShortBuffer), "got {err:?}");
}

#[test]
fn seq_guard_accepts_monotonic_sds_6_2_seq() {
    let mut g = SeqGuard::new(0);
    g.check(10).expect("first seq always ok");
    g.check(11).expect("monotonic seq ok");
}

#[test]
fn seq_guard_rejects_backwards_sds_6_2_seq() {
    let mut g = SeqGuard::new(0);
    g.check(10).expect("first seq always ok");
    let err = g.check(5).expect_err("backwards seq must error");
    assert!(
        matches!(err, FrameHeaderError::BackwardsSeq),
        "got {err:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. REMOTE_FRAME_PAYLOAD_BYTES  (SDS §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[allow(clippy::assertions_on_constants)]
fn remote_frame_payload_bytes_constant_sds_6_2_layout() {
    assert_eq!(cube_proto::REMOTE_FRAME_PAYLOAD_BYTES, 49_152);
    // sanity: REMOTE_FRAME_PAYLOAD_BYTES is a separate envelope from the JSON
    // line cap. We only assert they're different to catch accidental aliasing.
    assert!(
        cube_proto::REMOTE_FRAME_PAYLOAD_BYTES < cube_proto::MAX_MESSAGE_BYTES,
        "payload-bytes and max-message-bytes are separate envelopes"
    );
    // Reference the import so the namespaced and bare consts both resolve.
    let _ = REMOTE_FRAME_PAYLOAD_BYTES;
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. cube_proto::session_token  (SDS §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn session_token_accepts_16_bytes_sds_6_2_token() {
    let token = URL_SAFE_NO_PAD.encode([7u8; 16]);
    validate_session_token(&token).expect("16-byte token must validate");
}

#[test]
fn session_token_rejects_15_bytes_sds_6_2_token() {
    let token = URL_SAFE_NO_PAD.encode([7u8; 15]);
    let err = validate_session_token(&token).expect_err("15-byte token must error");
    assert!(matches!(err, SessionTokenError::TooShort), "got {err:?}");
}

#[test]
fn session_token_rejects_invalid_base64_sds_6_2_token() {
    let err = validate_session_token("!!!").expect_err("non-base64url must error");
    assert!(
        matches!(err, SessionTokenError::InvalidBase64),
        "got {err:?}"
    );
}

#[test]
fn session_token_rejects_empty_sds_6_2_token() {
    let err = validate_session_token("").expect_err("empty token must error");
    assert!(
        matches!(
            err,
            SessionTokenError::TooShort | SessionTokenError::InvalidBase64
        ),
        "got {err:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Event::FrameStreamBound  (SDS v5 §6.2)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn frame_stream_bound_event_roundtrips_sds_6_2() {
    let j = serde_json::json!({
        "event": "frame_stream.bound",
        "max_inflight": 1,
        "policy": "latest-frame-wins",
        "expected_payload_bytes": 49152,
        "mtu_hint": "jumbo_recommended"
    });
    let e: Event = serde_json::from_value(j.clone()).expect("decode");
    assert_eq!(serde_json::to_value(&e).unwrap(), j);
}
