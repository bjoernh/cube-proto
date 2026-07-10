//! Tombstone semantics (SDS §5.5).
//!
//! - Deleting a built-in preset writes `<name>.deleted` and that hides the
//!   built-in from `list`.
//! - Deleting a user preset unlinks the file (no tombstone needed).
//! - Deleting an unknown preset returns `ENOENT`.

mod common;

use cube_presets::PresetError;

const BUILTIN_A: &str = include_str!("fixtures/builtin_a.toml");
const USER_C: &str = include_str!("fixtures/user_c.toml");

#[test]
fn delete_builtin_writes_tombstone_and_hides_it_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();
    let store = cube_presets::PresetStore::new(
        system_root,
        user_root.clone(),
        cube_presets::RealFsOps,
    );

    // Before: built-in `a` shows up in the list.
    let pre = store.list("x").unwrap();
    assert_eq!(pre.len(), 1, "{pre:?}");
    assert_eq!(pre[0].0, "a");
    assert!(matches!(pre[0].1, cube_presets::PresetOrigin::BuiltIn));

    store.delete("x", "a").expect("delete (tombstone) must succeed");

    // Tombstone file exists.
    let tombstone = user_root.join("x/presets/a.deleted");
    assert!(tombstone.exists(), "tombstone must be written to user dir");

    // After: list is empty because the built-in is hidden by the tombstone.
    let post = store.list("x").unwrap();
    assert!(post.is_empty(), "tombstone must hide the built-in, got {post:?}");
}

#[test]
fn delete_user_preset_removes_file_no_tombstone_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    let target = user_root.join("x/presets/c.toml");
    std::fs::write(&target, USER_C).unwrap();

    let store = cube_presets::PresetStore::new(
        system_root,
        user_root.clone(),
        cube_presets::RealFsOps,
    );

    store.delete("x", "c").expect("delete must succeed");
    assert!(!target.exists(), "user preset file must be removed");
    let tombstone = user_root.join("x/presets/c.deleted");
    assert!(!tombstone.exists(), "no tombstone needed when no built-in exists");
}

#[test]
fn delete_unknown_returns_enoent_sds_5_5() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");

    let err = store
        .delete("x", "nope")
        .expect_err("delete of unknown preset must error");
    assert!(matches!(err, PresetError::NotFound), "{err:?}");
}

#[test]
fn save_after_tombstone_removes_tombstone_and_restores_preset_sds_5_5() {
    // If the user re-saves a preset whose name was tombstoned, the tombstone
    // is implicitly cleared by the presence of the user file (user shadows
    // built-in, tombstone shadows built-in). This is the natural read order
    // but `save` should also clean the tombstone so `list` sees the new user
    // entry rather than treating it as deleted.
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();
    std::fs::write(user_root.join("x/presets/a.deleted"), b"").unwrap();

    let store = cube_presets::PresetStore::new(
        system_root,
        user_root.clone(),
        cube_presets::RealFsOps,
    );
    let schema = common::schema_minimal("x", 1);

    // Empty before re-save.
    assert!(store.list("x").unwrap().is_empty());

    let mut params = std::collections::BTreeMap::<String, cube_proto::ParamValue>::new();
    params.insert("speed".to_string(), cube_proto::ParamValue::Int(8));
    let pf = cube_presets::PresetFile {
        meta: cube_presets::PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "a".to_string(),
            author: None,
            description: None,
            created: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap(),
            version: "1.0".to_string(),
        },
        params,
    };
    store.save("x", "a", &pf, &schema).expect("re-save must succeed");

    let post = store.list("x").unwrap();
    assert_eq!(post.len(), 1, "{post:?}");
    assert_eq!(post[0].0, "a");
    assert!(matches!(post[0].1, cube_presets::PresetOrigin::User));
}
