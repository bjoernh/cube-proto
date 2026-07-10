//! Persistence call-order contract (SDS §5.6).
//!
//! SDS §5.6 mandates:
//!
//! > Explicit user actions (`preset.save`, `preset.import`, `preset.delete`)
//! > call `fsync` on the file and on the parent directory.
//!
//! and earlier in the same section that writes are *write-temp-rename*. The
//! resulting call ordering is:
//!
//! 1. open temp + write contents
//! 2. `fsync(temp_file)`
//! 3. `rename(temp -> target)`
//! 4. `fsync(parent_dir)`
//!
//! These tests use `RecordingFsOps` to assert the sequence is observed by
//! `save()` and `delete()`.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use cube_presets::{FsCall, PresetFile, PresetMeta, RecordingFsOps};
use cube_proto::ParamValue;

fn meta(app: &str, name: &str) -> PresetMeta {
    PresetMeta {
        app: app.to_string(),
        schema_version: 1,
        preset_name: name.to_string(),
        author: None,
        description: None,
        created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
        version: "1.0".to_string(),
    }
}

fn preset_file(app: &str, name: &str) -> PresetFile {
    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(5));
    PresetFile { meta: meta(app, name), params }
}

/// Find positions of the expected ordered call kinds in the recorded log.
/// Each predicate must match exactly one call; the matched indices must be
/// strictly increasing.
fn assert_strict_order(calls: &[FsCall], stages: &[&dyn Fn(&FsCall) -> bool], labels: &[&str]) {
    assert_eq!(stages.len(), labels.len());
    let mut last_idx: Option<usize> = None;
    for (stage, label) in stages.iter().zip(labels.iter()) {
        let position = calls.iter().position(stage).unwrap_or_else(|| {
            panic!("missing call stage `{label}` in recorded calls: {calls:?}")
        });
        if let Some(prev) = last_idx {
            assert!(
                position > prev,
                "stage `{label}` (at {position}) must come after the previous stage (at {prev}) — full log: {calls:?}",
            );
        }
        last_idx = Some(position);
    }
}

#[test]
fn save_fsync_call_order_sds_5_6() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    let fs = RecordingFsOps::new(tmp.path());
    let calls_handle = fs.calls.clone();
    let store = cube_presets::PresetStore::new(system_root, user_root.clone(), fs);

    let schema = common::schema_minimal("x", 1);
    let report = store
        .save("x", "p", &preset_file("x", "p"), &schema)
        .expect("save must succeed");
    assert!(report.warnings.is_empty(), "no warnings expected");

    let calls = calls_handle.lock().unwrap().clone();
    let target = user_root.join("x/presets/p.toml");

    assert_strict_order(
        &calls,
        &[
            &|c| matches!(c, FsCall::Write(p, _) if p.parent() == target.parent()),
            &|c| matches!(c, FsCall::FsyncFile(p) if p.parent() == target.parent()),
            &|c| matches!(c, FsCall::Rename { to, .. } if to == &target),
            &|c| matches!(c, FsCall::FsyncParent(p) if p == &target.parent().unwrap().to_path_buf()),
        ],
        &["Write(temp)", "FsyncFile(temp)", "Rename(temp -> target)", "FsyncParent(presets/)"],
    );
}

#[test]
fn delete_user_preset_unlink_then_fsync_parent_sds_5_6() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    // Place a user preset that delete() will remove.
    let target = user_root.join("x/presets/p.toml");
    std::fs::write(&target, include_str!("fixtures/user_c.toml")).unwrap();

    let fs = RecordingFsOps::new(tmp.path());
    let calls_handle = fs.calls.clone();
    let store = cube_presets::PresetStore::new(system_root, user_root.clone(), fs);

    store.delete("x", "p").expect("delete must succeed");

    let calls = calls_handle.lock().unwrap().clone();
    let removed_target: PathBuf = target.clone();
    assert_strict_order(
        &calls,
        &[
            &|c| matches!(c, FsCall::RemoveFile(p) if p == &removed_target),
            &|c| matches!(c, FsCall::FsyncParent(p) if p == &removed_target.parent().unwrap().to_path_buf()),
        ],
        &["RemoveFile(user/p.toml)", "FsyncParent(presets/)"],
    );
}

#[test]
fn delete_builtin_writes_tombstone_then_fsync_sds_5_6() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    // Built-in preset only — no user file.
    std::fs::write(system_root.join("x/presets/a.toml"), include_str!("fixtures/builtin_a.toml"))
        .unwrap();

    let fs = RecordingFsOps::new(tmp.path());
    let calls_handle = fs.calls.clone();
    let store = cube_presets::PresetStore::new(system_root, user_root.clone(), fs);

    store.delete("x", "a").expect("delete (tombstone) must succeed");

    let calls = calls_handle.lock().unwrap().clone();
    let tombstone = user_root.join("x/presets/a.deleted");

    assert_strict_order(
        &calls,
        &[
            &|c| matches!(c, FsCall::CreateTombstone(p) if p == &tombstone),
            &|c| matches!(c, FsCall::FsyncFile(p) if p == &tombstone),
            &|c| matches!(c, FsCall::FsyncParent(p) if p == &tombstone.parent().unwrap().to_path_buf()),
        ],
        &["CreateTombstone", "FsyncFile(tombstone)", "FsyncParent(presets/)"],
    );
}
