//! Import validation produces structured warnings (SDS §5.5).
//!
//! SDS §5.5 defines four warning codes:
//!
//! - `UNKNOWN_DROPPED` — key not declared in the schema.
//! - `CLAMPED`         — value clamped into the schema range (with `from`/`to`).
//! - `READONLY_DROPPED` — key is `readonly = true`.
//! - `NON_SHAREABLE_DROPPED` — key is `shareable = false`.
//!
//! The import report exposes the warnings AND the post-import `final_params`
//! map so callers can show both the action and the result.

mod common;

use cube_presets::WarningCode;
use cube_proto::ParamValue;

const FIXTURE: &str = include_str!("fixtures/import_warnings.toml");

#[test]
fn import_emits_all_four_warning_codes_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let schema = common::schema_for_warnings("x", 1);

    let report = store
        .import("x", "warnings", FIXTURE, &schema)
        .expect("import must succeed (warnings are non-fatal)");

    // Each code must appear at least once, keyed to the right param.
    let by_code: Vec<(WarningCode, &str)> = report
        .warnings
        .iter()
        .map(|w| (w.code, w.key.as_str()))
        .collect();
    assert!(by_code.contains(&(WarningCode::Clamped, "speed")), "{by_code:?}");
    assert!(by_code.contains(&(WarningCode::UnknownDropped, "old_param")), "{by_code:?}");
    assert!(by_code.contains(&(WarningCode::ReadonlyDropped, "gamma")), "{by_code:?}");
    assert!(
        by_code.contains(&(WarningCode::NonShareableDropped, "device_id")),
        "{by_code:?}",
    );
    assert_eq!(report.warnings.len(), 4, "expected exactly 4 warnings, got {by_code:?}");
}

#[test]
fn import_clamp_warning_carries_from_to_detail_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let schema = common::schema_for_warnings("x", 1);

    let report = store
        .import("x", "warnings", FIXTURE, &schema)
        .expect("import must succeed");

    let clamp = report
        .warnings
        .iter()
        .find(|w| w.code == WarningCode::Clamped)
        .expect("CLAMPED entry must exist");
    let from = clamp.detail.get("from").expect("CLAMPED.detail.from required").as_i64().unwrap();
    let to = clamp.detail.get("to").expect("CLAMPED.detail.to required").as_i64().unwrap();
    assert_eq!(from, 999, "from must equal raw preset value");
    assert_eq!(to, 10, "to must equal clamped schema-max");
}

#[test]
fn import_final_params_reflects_clamps_and_drops_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let schema = common::schema_for_warnings("x", 1);

    let report = store
        .import("x", "warnings", FIXTURE, &schema)
        .expect("import must succeed");

    // Kept and clamped:
    assert_eq!(report.final_params.get("brightness"), Some(&ParamValue::Int(5)));
    assert_eq!(report.final_params.get("speed"), Some(&ParamValue::Int(10)));

    // Dropped:
    assert!(!report.final_params.contains_key("gamma"), "gamma must be dropped (readonly)");
    assert!(
        !report.final_params.contains_key("device_id"),
        "device_id must be dropped (non-shareable)",
    );
    assert!(
        !report.final_params.contains_key("old_param"),
        "old_param must be dropped (unknown)",
    );
}

#[test]
fn warning_code_serializes_as_screaming_snake_sds_5_5() {
    // SDS §5.5 shows the wire codes are SCREAMING_SNAKE_CASE strings.
    assert_eq!(serde_json::to_string(&WarningCode::UnknownDropped).unwrap(), "\"UNKNOWN_DROPPED\"");
    assert_eq!(serde_json::to_string(&WarningCode::Clamped).unwrap(), "\"CLAMPED\"");
    assert_eq!(
        serde_json::to_string(&WarningCode::ReadonlyDropped).unwrap(),
        "\"READONLY_DROPPED\"",
    );
    assert_eq!(
        serde_json::to_string(&WarningCode::NonShareableDropped).unwrap(),
        "\"NON_SHAREABLE_DROPPED\"",
    );
}

#[test]
fn import_type_mismatch_returns_etype_sds_5_5() {
    // `speed` is int in the schema but supplied as a string here.
    const BODY: &str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "etype"
created = "2026-05-17T00:00:00Z"
version = "1.0"

[params]
speed = { type = "string", value = "fast" }
"#;
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let schema = common::schema_for_warnings("x", 1);

    let err = store
        .import("x", "etype", BODY, &schema)
        .expect_err("type mismatch must error");
    assert!(matches!(err, cube_presets::PresetError::TypeMismatch(_)), "{err:?}");
}
