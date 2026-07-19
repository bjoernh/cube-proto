//! Community preset origin (App-Store M6): `presets/community/` as a third
//! namespace with resolution precedence user > community > built-in.
//!
//! - `import(..., Community)` lands in `presets/community/` and can never
//!   overwrite a same-named user preset.
//! - `list` reports `Community` origin; user files shadow community files;
//!   community files shadow built-ins; tombstones hide built-ins only.
//! - `export` (and thus `load`) resolves through the same chain.
//! - `delete` removes the visible entry along the chain.

mod common;

use cube_presets::{PresetError, PresetOrigin};

const BUILTIN_A: &str = include_str!("fixtures/builtin_a.toml");
const USER_C: &str = include_str!("fixtures/user_c.toml");

/// A community-shareable preset body for app `x`, param `speed`.
fn community_toml(speed: i64) -> String {
    format!(
        r#"[meta]
app = "x"
schema_version = 1
preset_name = "Fireplace Glow"
author = "alex"
created = "2026-07-18T00:00:00Z"
version = "1.0"

[params]
speed = {{ type = "int", value = {speed} }}
"#
    )
}

#[test]
fn community_import_lands_in_community_dir_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    store
        .import("x", "fireplace-glow", &community_toml(3), &schema, PresetOrigin::Community)
        .expect("community import must succeed");

    assert!(
        user_root.join("x/presets/community/fireplace-glow.toml").exists(),
        "community import must land in presets/community/"
    );
    assert!(
        !user_root.join("x/presets/fireplace-glow.toml").exists(),
        "community import must not write into the user preset dir"
    );

    let list = store.list("x").unwrap();
    assert_eq!(list, vec![("fireplace-glow".to_owned(), PresetOrigin::Community)]);
}

#[test]
fn community_import_cannot_overwrite_user_preset_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);
    std::fs::write(user_root.join("x/presets/c.toml"), USER_C).unwrap();

    store
        .import("x", "c", &community_toml(9), &schema, PresetOrigin::Community)
        .expect("import under a taken name still succeeds — into its own namespace");

    // The user's own file is untouched; the visible `c` is still the user one.
    assert_eq!(std::fs::read_to_string(user_root.join("x/presets/c.toml")).unwrap(), USER_C);
    let list = store.list("x").unwrap();
    assert_eq!(list, vec![("c".to_owned(), PresetOrigin::User)], "user file shadows community");

    let pf = store.export("x", "c").expect("export resolves the user file");
    assert_eq!(pf.meta.preset_name, "c", "user file wins the precedence chain");
}

#[test]
fn community_shadows_builtin_and_ignores_tombstone_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();

    store
        .import("x", "a", &community_toml(5), &schema, PresetOrigin::Community)
        .expect("community import must succeed");

    let list = store.list("x").unwrap();
    assert_eq!(
        list,
        vec![("a".to_owned(), PresetOrigin::Community)],
        "community must shadow the same-named built-in"
    );

    // A tombstone hides built-ins only — the community preset stays visible
    // and export still resolves it.
    std::fs::write(user_root.join("x/presets/a.deleted"), "").unwrap();
    let list = store.list("x").unwrap();
    assert_eq!(list, vec![("a".to_owned(), PresetOrigin::Community)]);
    let pf = store.export("x", "a").expect("tombstone must not hide a community preset");
    assert_eq!(pf.meta.preset_name, "Fireplace Glow");
}

#[test]
fn delete_removes_community_file_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    store
        .import("x", "fireplace-glow", &community_toml(3), &schema, PresetOrigin::Community)
        .unwrap();
    store.delete("x", "fireplace-glow").expect("delete must remove the community preset");

    assert!(!user_root.join("x/presets/community/fireplace-glow.toml").exists());
    assert!(store.list("x").unwrap().is_empty());
    assert!(matches!(store.export("x", "fireplace-glow"), Err(PresetError::NotFound)));
}

#[test]
fn delete_community_over_builtin_writes_tombstone_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();

    store.import("x", "a", &community_toml(5), &schema, PresetOrigin::Community).unwrap();
    store.delete("x", "a").expect("delete must succeed");

    // Deleting the visible community preset must not reveal the built-in —
    // "delete" means the name goes away.
    assert!(user_root.join("x/presets/a.deleted").exists(), "tombstone keeps the built-in hidden");
    assert!(store.list("x").unwrap().is_empty(), "{:?}", store.list("x").unwrap());
}

#[test]
fn community_import_does_not_resurrect_tombstoned_builtin_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, system_root, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);
    std::fs::write(system_root.join("x/presets/a.toml"), BUILTIN_A).unwrap();
    store.delete("x", "a").unwrap(); // tombstone the built-in

    // Import a community preset under a *different* name: the built-in's
    // tombstone must survive (only a user import un-tombstones).
    store.import("x", "other", &community_toml(2), &schema, PresetOrigin::Community).unwrap();
    assert!(user_root.join("x/presets/a.deleted").exists());
    let list = store.list("x").unwrap();
    assert_eq!(list, vec![("other".to_owned(), PresetOrigin::Community)]);
}

#[test]
fn builtin_origin_import_is_rejected_m6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _system_root, _user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    let err = store
        .import("x", "a", &community_toml(1), &schema, PresetOrigin::BuiltIn)
        .expect_err("BuiltIn is not an importable origin");
    assert!(matches!(err, PresetError::BadRequest(_)), "{err:?}");
}
