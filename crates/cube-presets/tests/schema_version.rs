//! `meta.schema_version` enforcement.
//!
//! > Reject on `meta.schema_version` mismatch with `EVERSION`. v1 deliberately
//! > has no migration mechanism: if the preset was written against an older
//! > schema, the user re-saves from current app defaults. The error message
//! > names both versions so the user knows what happened.

mod common;

use cube_presets::{PresetError, PresetOrigin};

const PRESET_V1: &str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "old"
created = "2026-05-17T00:00:00Z"
version = "1.0"

[params]
speed = { type = "int", value = 5 }
"#;

#[test]
fn import_v1_preset_against_v2_schema_returns_eversion() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 2);

    let err = store
        .import("x", "old", PRESET_V1, &schema, PresetOrigin::User)
        .expect_err("schema_version mismatch must error");

    match err {
        PresetError::SchemaVersionMismatch { expected, got } => {
            assert_eq!(expected, 2);
            assert_eq!(got, 1);
        }
        other => panic!("expected SchemaVersionMismatch, got {other:?}"),
    }

    // Critical: no file written.
    let target = user_root.join("x/presets/old.toml");
    assert!(
        !target.exists(),
        "rejected import must not create the target file"
    );
    // No temp debris either.
    let presets_dir = user_root.join("x/presets");
    let entries: Vec<_> = std::fs::read_dir(&presets_dir).unwrap().collect();
    assert!(
        entries.is_empty(),
        "rejected import must not leave debris: {entries:?}"
    );
}

#[test]
fn eversion_error_message_names_both_versions() {
    let err = PresetError::SchemaVersionMismatch {
        expected: 2,
        got: 1,
    };
    let msg = err.to_string();
    assert!(
        msg.contains('2'),
        "error message must mention expected version, got {msg:?}"
    );
    assert!(
        msg.contains('1'),
        "error message must mention got version, got {msg:?}"
    );
}

#[test]
fn import_app_mismatch_returns_enoapp() {
    const PRESET_FOR_Y: &str = r#"
[meta]
app = "y"
schema_version = 1
preset_name = "z"
created = "2026-05-17T00:00:00Z"
version = "1.0"

[params]
speed = { type = "int", value = 5 }
"#;
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    let err = store
        .import("x", "z", PRESET_FOR_Y, &schema, PresetOrigin::User)
        .expect_err("meta.app != app must error");
    assert!(matches!(err, PresetError::AppMismatch), "{err:?}");

    let target = user_root.join("x/presets/z.toml");
    assert!(!target.exists(), "rejected import must not write");
}
