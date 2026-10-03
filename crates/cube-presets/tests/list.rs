//! `PresetStore::list` merge semantics.
//!
//! - User presets shadow same-named built-ins.
//! - `<name>.deleted` tombstones in the user dir hide the built-in.
//! - Only `.toml` and `.deleted` files in the user dir contribute; everything
//!   else is ignored.

mod common;

use std::path::Path;

use cube_presets::PresetOrigin;

const BUILTIN_A: &str = include_str!("fixtures/builtin_a.toml");
const BUILTIN_B: &str = include_str!("fixtures/builtin_b.toml");
const USER_B: &str = include_str!("fixtures/user_b.toml");
const USER_C: &str = include_str!("fixtures/user_c.toml");

fn seed_layout(root: &Path) {
    // /system/apps/x/presets/{a.toml,b.toml}
    // /user/apps/x/presets/{b.toml,c.toml,a.deleted}
    let (system_root, user_root) = common::make_layout(root, "x");
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();
    std::fs::write(system_root.join("x/presets/b.toml"), BUILTIN_B).unwrap();
    std::fs::write(user_root.join("x/presets/b.toml"), USER_B).unwrap();
    std::fs::write(user_root.join("x/presets/c.toml"), USER_C).unwrap();
    std::fs::write(user_root.join("x/presets/a.deleted"), b"").unwrap();
}

#[test]
fn list_tombstone_hides_builtin_and_user_shadows() {
    let tmp = tempfile::tempdir().unwrap();
    seed_layout(tmp.path());

    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let mut listed = store.list("x").expect("list must succeed");
    listed.sort_by(|a, b| a.0.cmp(&b.0));

    // Expected: ("b", User), ("c", User).
    // - "a" is hidden by the `a.deleted` tombstone.
    // - "b" is the user version shadowing the built-in.
    // - "c" is user-only.
    assert_eq!(listed.len(), 2, "got {listed:?}");
    assert_eq!(listed[0].0, "b");
    assert!(matches!(listed[0].1, PresetOrigin::User));
    assert_eq!(listed[1].0, "c");
    assert!(matches!(listed[1].1, PresetOrigin::User));
}

#[test]
fn list_builtin_only_when_no_user_dir_present() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();
    std::fs::write(system_root.join("x/presets/b.toml"), BUILTIN_B).unwrap();
    // No user preset files at all (dir exists, but is empty).
    let store = cube_presets::PresetStore::new(system_root, user_root, cube_presets::RealFsOps);

    let mut listed = store.list("x").expect("list must succeed");
    listed.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].0, "a");
    assert!(matches!(listed[0].1, PresetOrigin::BuiltIn));
    assert_eq!(listed[1].0, "b");
    assert!(matches!(listed[1].1, PresetOrigin::BuiltIn));
}

#[test]
fn list_user_only_when_no_builtin_dir_present() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    std::fs::write(user_root.join("x/presets/c.toml"), USER_C).unwrap();
    // No built-in preset files.
    let store = cube_presets::PresetStore::new(system_root, user_root, cube_presets::RealFsOps);

    let listed = store.list("x").expect("list must succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, "c");
    assert!(matches!(listed[0].1, PresetOrigin::User));
}

#[test]
fn list_ignores_non_toml_non_tombstone_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (system_root, user_root) = common::make_layout(tmp.path(), "x");
    std::fs::write(user_root.join("x/presets/c.toml"), USER_C).unwrap();
    // Junk files (e.g. editor backups, partial temp files) must be ignored
    // so the list doesn't leak debris.
    std::fs::write(user_root.join("x/presets/c.toml~"), b"editor backup").unwrap();
    std::fs::write(user_root.join("x/presets/.tmp.XYZ"), b"in-flight temp").unwrap();
    std::fs::write(user_root.join("x/presets/README"), b"hi").unwrap();

    let store = cube_presets::PresetStore::new(system_root, user_root, cube_presets::RealFsOps);

    let listed = store.list("x").expect("list must succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, "c");
}

#[test]
fn export_user_shadows_builtin() {
    let tmp = tempfile::tempdir().unwrap();
    seed_layout(tmp.path());

    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let pf = store.export("x", "b").expect("export b must succeed");
    // The user fixture has speed=7; built-in is speed=2.
    let speed = pf
        .params
        .get("speed")
        .cloned()
        .expect("preset b must have key speed");
    match speed {
        cube_proto::ParamValue::Int(v) => assert_eq!(v, 7, "user fixture must shadow built-in"),
        other => panic!("expected int, got {other:?}"),
    }
}

#[test]
fn export_unknown_returns_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    seed_layout(tmp.path());

    let (store, _sr, _ur) = common::make_store(tmp.path(), "x");
    let err = store
        .export("x", "no-such-preset")
        .expect_err("missing preset must error");
    assert!(
        matches!(err, cube_presets::PresetError::NotFound),
        "{err:?}"
    );
}
