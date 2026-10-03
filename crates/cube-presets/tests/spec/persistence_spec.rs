//! Cross-reference tests that pin's persistence contract.
//!
//! NOTE: this file is included from `tests/spec.rs`.

#![allow(dead_code)]

#[path = "../common/mod.rs"]
mod common;

use std::collections::BTreeMap;

use cube_presets::{FsCall, PresetFile, PresetMeta, RecordingFsOps};
use cube_proto::ParamValue;

/// (line ≈ 510–514):
///
/// > Explicit preset save / import / delete →
/// > `/var/lib/cube/apps/<app>/presets/<name>.toml`,
/// > immediately on user request,
/// > write-temp-rename + `fsync` file + `fsync` parent dir.
#[test]
fn save_emits_write_fsync_rename_fsync_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    let fs = RecordingFsOps::new(tmp.path());
    let calls_handle = fs.calls.clone();
    let store = cube_presets::PresetStore::new(system_root, user_root.clone(), fs);
    let schema = common::schema_minimal("x", 1);

    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(5));
    let pf = PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "p".to_string(),
            author: None,
            description: None,
            created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            version: "1.0".to_string(),
        },
        params,
    };
    store.save("x", "p", &pf, &schema).unwrap();

    let calls = calls_handle.lock().unwrap().clone();
    let target = user_root.join("x/presets/p.toml");
    let parent = target.parent().unwrap().to_path_buf();

    // Find the four stages in order.
    let pos_write = calls
        .iter()
        .position(|c| matches!(c, FsCall::Write(_, _)))
        .unwrap();
    let pos_fsync_file = calls
        .iter()
        .position(|c| matches!(c, FsCall::FsyncFile(_)))
        .unwrap();
    let pos_rename = calls
        .iter()
        .position(|c| matches!(c, FsCall::Rename { to, .. } if to == &target))
        .unwrap();
    let pos_fsync_parent = calls
        .iter()
        .position(|c| matches!(c, FsCall::FsyncParent(p) if p == &parent))
        .unwrap();

    assert!(
        pos_write < pos_fsync_file,
        "Write before FsyncFile: {calls:?}"
    );
    assert!(
        pos_fsync_file < pos_rename,
        "FsyncFile before Rename: {calls:?}"
    );
    assert!(
        pos_rename < pos_fsync_parent,
        "Rename before FsyncParent: {calls:?}"
    );
}

///:
///
/// > Both parties acquire an exclusive `flock` on this file...
/// > The lock is released as soon as the atomic-rename completes.
/// > Holders must not retain the lock across IPC or sleeps.
///
/// We can't observe "no sleep across IPC" directly here, but we can pin
/// that the `.lock` file lives at `<user_root>/<app>/.lock`.
#[test]
fn lock_file_lives_at_user_root_dot_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(1));
    let pf = PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "p".to_string(),
            author: None,
            description: None,
            created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            version: "1.0".to_string(),
        },
        params,
    };
    store.save("x", "p", &pf, &schema).unwrap();
    assert!(
        user_root.join("x/.lock").exists(),
        "save() must create the per-app .lock file at <user_root>/<app>/.lock",
    );
}
