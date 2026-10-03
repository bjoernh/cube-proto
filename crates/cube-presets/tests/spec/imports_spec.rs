//! Cross-reference tests that pin's concrete worked examples.
//!
//! Each literal here is a verbatim copy of the example block in
//! `Cube 2.0 — System Design Specification-v4.md.5` (line numbers below).
//!
//! NOTE: this file is included from `tests/spec.rs`.

#![allow(dead_code)]

// Shared helpers are included per test target via #[path]; each target is its
// own crate, and within the `spec` target this is the only inclusion.
#[allow(clippy::duplicate_mod)]
#[path = "../common/mod.rs"]
mod common;

use cube_presets::WarningCode;
use serde_json::{Value, json};

/// (≈ line 485–498) defined-codes example:
///
/// ```json
/// {
///   "id": 42,
///   "ok": true,
///   "result": {
///     "warnings": [
///       {"key":"speed",     "code":"CLAMPED",        "from":999, "to":10},
///       {"key":"old_param", "code":"UNKNOWN_DROPPED"}
///     ]
///   }
/// }
/// ```
#[test]
fn warnings_literal_codes_are_screaming_snake() {
    let j: Value = json!({
        "id": 42,
        "ok": true,
        "result": {
            "warnings": [
                {"key": "speed", "code": "CLAMPED", "from": 999, "to": 10},
                {"key": "old_param", "code": "UNKNOWN_DROPPED"}
            ]
        }
    });
    let warnings = j["result"]["warnings"].as_array().unwrap();
    assert_eq!(warnings[0]["code"], "CLAMPED");
    assert_eq!(warnings[1]["code"], "UNKNOWN_DROPPED");

    // Round-trip the two strings through cube_presets::WarningCode.
    let clamped: WarningCode = serde_json::from_value(warnings[0]["code"].clone()).unwrap();
    let unknown: WarningCode = serde_json::from_value(warnings[1]["code"].clone()).unwrap();
    assert_eq!(clamped, WarningCode::Clamped);
    assert_eq!(unknown, WarningCode::UnknownDropped);
}

/// defines four codes total. Pin them all so a renamed variant in
/// `cube_presets` triggers a failure here, not silently in `cubectl` output.
#[test]
fn all_four_warning_codes_round_trip() {
    let cases = [
        (WarningCode::UnknownDropped, "\"UNKNOWN_DROPPED\""),
        (WarningCode::Clamped, "\"CLAMPED\""),
        (WarningCode::ReadonlyDropped, "\"READONLY_DROPPED\""),
        (
            WarningCode::NonShareableDropped,
            "\"NON_SHAREABLE_DROPPED\"",
        ),
    ];
    for (code, wire) in cases {
        let ser = serde_json::to_string(&code).unwrap();
        assert_eq!(ser, wire, "{code:?} serializes to {wire}");
        let de: WarningCode = serde_json::from_str(wire).unwrap();
        assert_eq!(de, code);
    }
}

/// mandates rejecting `meta.app` mismatch with `ENOAPP` and
/// `meta.schema_version` mismatch with `EVERSION`, with the version error
/// naming both the expected and observed version (so the user knows what
/// happened).
#[test]
fn eversion_error_message_mentions_both_versions() {
    let e = cube_presets::PresetError::SchemaVersionMismatch {
        expected: 7,
        got: 3,
    };
    let msg = e.to_string();
    assert!(
        msg.contains('7'),
        "expected msg to mention expected=7; got {msg:?}"
    );
    assert!(
        msg.contains('3'),
        "expected msg to mention got=3; got {msg:?}"
    );
}

///: "v1 deliberately has no migration mechanism". Pin the variant
/// name to flag any accidental introduction of a `Migrated` warning code.
#[test]
fn no_migration_warning_variant_exists() {
    // If someone adds a `Migrated` variant this function no longer compiles
    // because the match would no longer be exhaustive — that's the contract.
    fn assert_exhaustive(w: WarningCode) {
        match w {
            WarningCode::UnknownDropped
            | WarningCode::Clamped
            | WarningCode::ReadonlyDropped
            | WarningCode::NonShareableDropped => (),
        }
    }
    let _ = assert_exhaustive;
}

/// preset format: `[meta]` (app, `schema_version`, `preset_name`,
/// optional author/description/created/version) + `[params]` (key=value).
/// Round-trip a self-describing preset through `cube_presets::PresetFile`.
#[test]
fn preset_file_round_trips_through_toml() {
    let body = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "demo"
author = "claude"
description = "round-trip example"
created = "2026-05-17T00:00:00Z"
version = "1.0"

[params]
speed = { type = "int", value = 5 }
"#;
    let parsed: cube_presets::PresetFile =
        toml::from_str(body).expect("PresetFile must parse example shape");
    assert_eq!(parsed.meta.app, "x");
    assert_eq!(parsed.meta.schema_version, 1);
    assert_eq!(parsed.meta.preset_name, "demo");
    assert_eq!(parsed.meta.author.as_deref(), Some("claude"));
    assert_eq!(parsed.meta.version, "1.0");

    // Re-serialize and parse again.
    let again = toml::to_string(&parsed).expect("PresetFile must serialize");
    let reparsed: cube_presets::PresetFile = toml::from_str(&again).expect("round-trip parse");
    assert_eq!(reparsed.meta.preset_name, parsed.meta.preset_name);
    assert_eq!(reparsed.params, parsed.params);
}
