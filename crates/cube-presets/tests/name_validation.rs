//! Preset / app name validation.
//!
//! > Preset names must match `^[a-zA-Z0-9._-]+$` and must not be equal to
//! > `.` or `..`. Any name containing `/`, control characters, or equal to
//! > the single-component path references `.` or `..` is rejected with
//! > `EBADREQ` **before any file is touched**. App names are validated
//! > identically.
//!
//! These tests assert both the error code *and* the "before any file is
//! touched" promise — we snapshot the user dir contents around each
//! rejected call and require they are byte-identical.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use cube_presets::{PresetError, PresetFile, PresetMeta, PresetOrigin};
use cube_proto::ParamValue;

fn snapshot(dir: &Path) -> Vec<String> {
    let mut entries: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

fn preset_file(app: &str, name: &str) -> PresetFile {
    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(5));
    PresetFile {
        meta: PresetMeta {
            app: app.to_string(),
            schema_version: 1,
            preset_name: name.to_string(),
            author: None,
            description: None,
            created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            version: "1.0".to_string(),
        },
        params,
    }
}

const BAD_NAMES: &[&str] = &[".", "..", "foo/bar", "foo\u{0001}evil", ""];

#[test]
fn save_rejects_invalid_names_before_fs_touch() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let presets_dir = user_root.join("x/presets");
    let schema = common::schema_minimal("x", 1);

    let before = snapshot(&presets_dir);
    for bad in BAD_NAMES {
        let err = store
            .save("x", bad, &preset_file("x", bad), &schema)
            .expect_err(&format!("save({bad:?}) must be rejected"));
        assert!(
            matches!(err, PresetError::BadRequest(_)),
            "name={bad:?}, err={err:?}"
        );
    }
    let after = snapshot(&presets_dir);
    assert_eq!(
        before, after,
        "rejected saves must not touch the user preset dir"
    );
}

#[test]
fn import_rejects_invalid_names_before_fs_touch() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let presets_dir = user_root.join("x/presets");
    let schema = common::schema_minimal("x", 1);

    let body = include_str!("fixtures/minimal_preset.toml");
    let before = snapshot(&presets_dir);
    for bad in BAD_NAMES {
        let err = store
            .import("x", bad, body, &schema, PresetOrigin::User)
            .expect_err(&format!("import({bad:?}) must be rejected"));
        assert!(
            matches!(err, PresetError::BadRequest(_)),
            "name={bad:?}, err={err:?}"
        );
    }
    let after = snapshot(&presets_dir);
    assert_eq!(before, after);
}

#[test]
fn delete_rejects_invalid_names_before_fs_touch() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let presets_dir = user_root.join("x/presets");

    // Pre-populate one valid file so we can detect any accidental tombstone.
    std::fs::write(
        presets_dir.join("kept.toml"),
        include_str!("fixtures/user_c.toml"),
    )
    .unwrap();
    let before = snapshot(&presets_dir);

    for bad in BAD_NAMES {
        let err = store
            .delete("x", bad)
            .expect_err(&format!("delete({bad:?}) must be rejected"));
        assert!(
            matches!(err, PresetError::BadRequest(_)),
            "name={bad:?}, err={err:?}"
        );
    }
    let after = snapshot(&presets_dir);
    assert_eq!(before, after);
}

#[test]
fn export_rejects_invalid_names() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    for bad in BAD_NAMES {
        let err = store
            .export("x", bad)
            .expect_err(&format!("export({bad:?}) must be rejected"));
        assert!(
            matches!(err, PresetError::BadRequest(_)),
            "name={bad:?}, err={err:?}"
        );
    }
}

#[test]
fn list_rejects_invalid_app_names() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    // App-name regex is the same as preset-name regex.
    let bad_apps: &[&str] = &[".", "..", "foo/bar", "foo\u{0001}evil", ""];
    for bad in bad_apps {
        let err = store
            .list(bad)
            .expect_err(&format!("list({bad:?}) must be rejected"));
        assert!(
            matches!(err, PresetError::BadRequest(_)),
            "app={bad:?}, err={err:?}"
        );
    }
}

#[test]
fn save_accepts_valid_names() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);
    // spec regex: ^[a-zA-Z0-9._-]+$
    for good in [
        "a",
        "p1",
        "my-preset",
        "my_preset",
        "my.preset",
        "PRESET_2026-05-17",
    ] {
        store
            .save("x", good, &preset_file("x", good), &schema)
            .unwrap_or_else(|e| panic!("save({good:?}) must succeed: {e}"));
    }
}
