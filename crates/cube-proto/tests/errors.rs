//! Coverage for the 14 `CubeErrno` variants and for the `CubeError`
//! body that carries them in `Response.error`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use cube_proto::{CubeErrno, CubeError};

fn assert_errno_roundtrip(variant: CubeErrno, wire: &str) {
    let v = serde_json::to_value(variant).expect("encode errno");
    assert_eq!(v, Value::String(wire.to_string()), "encoded form");
    let back: CubeErrno = serde_json::from_value(v).expect("decode errno");
    assert_eq!(back, variant, "roundtrip mismatch");
}

#[test]
fn errno_ebadreq_round_trips() {
    assert_errno_roundtrip(CubeErrno::EBADREQ, "EBADREQ");
}

#[test]
fn errno_eunknown_round_trips() {
    assert_errno_roundtrip(CubeErrno::EUNKNOWN, "EUNKNOWN");
}

#[test]
fn errno_enoapp_round_trips() {
    assert_errno_roundtrip(CubeErrno::ENOAPP, "ENOAPP");
}

#[test]
fn errno_enokey_round_trips() {
    assert_errno_roundtrip(CubeErrno::ENOKEY, "ENOKEY");
}

#[test]
fn errno_etype_round_trips() {
    assert_errno_roundtrip(CubeErrno::ETYPE, "ETYPE");
}

#[test]
fn errno_erange_round_trips() {
    assert_errno_roundtrip(CubeErrno::ERANGE, "ERANGE");
}

#[test]
fn errno_eenum_round_trips() {
    assert_errno_roundtrip(CubeErrno::EENUM, "EENUM");
}

#[test]
fn errno_ereadonly_round_trips() {
    assert_errno_roundtrip(CubeErrno::EREADONLY, "EREADONLY");
}

#[test]
fn errno_eexist_round_trips() {
    assert_errno_roundtrip(CubeErrno::EEXIST, "EEXIST");
}

#[test]
fn errno_enoent_round_trips() {
    assert_errno_roundtrip(CubeErrno::ENOENT, "ENOENT");
}

#[test]
fn errno_eversion_round_trips() {
    assert_errno_roundtrip(CubeErrno::EVERSION, "EVERSION");
}

#[test]
fn errno_etimeout_round_trips() {
    assert_errno_roundtrip(CubeErrno::ETIMEOUT, "ETIMEOUT");
}

#[test]
fn errno_ebusy_round_trips() {
    assert_errno_roundtrip(CubeErrno::EBUSY, "EBUSY");
}

#[test]
fn errno_enotrunning_round_trips() {
    assert_errno_roundtrip(CubeErrno::ENOTRUNNING, "ENOTRUNNING");
}

#[test]
fn unknown_errno_string_is_rejected() {
    let j = json!("ENOTANERRNO");
    let r: Result<CubeErrno, _> = serde_json::from_value(j);
    assert!(r.is_err(), "unknown errno must not deserialize");
}

#[test]
fn cube_error_with_context_round_trips() {
    let mut ctx = BTreeMap::new();
    ctx.insert("app".to_string(), json!("snake"));
    ctx.insert("key".to_string(), json!("speed"));
    ctx.insert("min".to_string(), json!(0));
    ctx.insert("max".to_string(), json!(10));
    let err = CubeError {
        code: CubeErrno::ERANGE,
        message: "value out of range".to_string(),
        context: ctx,
    };

    let v = serde_json::to_value(&err).expect("encode CubeError");
    assert_eq!(
        v,
        json!({
            "code": "ERANGE",
            "message": "value out of range",
            "context": {"app": "snake", "key": "speed", "min": 0, "max": 10}
        })
    );

    let back: CubeError = serde_json::from_value(v).expect("decode CubeError");
    assert_eq!(back, err);
}

#[test]
fn cube_error_empty_context_round_trips() {
    let err = CubeError {
        code: CubeErrno::EBADREQ,
        message: "bad request".to_string(),
        context: BTreeMap::new(),
    };
    let v = serde_json::to_value(&err).unwrap();
    let back: CubeError = serde_json::from_value(v).unwrap();
    assert_eq!(back, err);
}

#[test]
fn cube_error_implements_std_error() {
    // Compile-time check: CubeError must be a `std::error::Error`.
    fn assert_error<E: std::error::Error>() {}
    assert_error::<CubeError>();
}
