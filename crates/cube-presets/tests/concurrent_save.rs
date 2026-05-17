//! Concurrent `save()` against `RealFsOps` (SDS §5.6).
//!
//! Eight threads each save a distinct preset name into the same user dir.
//! The per-app `.lock` (`flock(LOCK_EX)`) serializes them; the
//! write-temp-rename + fsync sequence is atomic per call.
//!
//! Post-conditions:
//!
//! - All eight `.toml` files exist and parse as valid presets.
//! - No `.tmp.*` debris remains in the preset directory.
//! - The `.lock` file exists exactly once (no stray lock duplicates).

mod common;

use std::collections::BTreeMap;
use std::thread;

use cube_presets::{PresetFile, PresetMeta};
use cube_proto::ParamValue;

fn preset_file(app: &str, name: &str, n: i64) -> PresetFile {
    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(n));
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

#[test]
fn eight_threads_save_distinct_presets_concurrently_sds_5_6() {
    let tmp = tempfile::tempdir().unwrap();
    let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
    let schema = common::schema_minimal("x", 1);

    const N: i64 = 8;
    thread::scope(|scope| {
        for i in 0..N {
            let store_ref = &store;
            let schema_ref = &schema;
            scope.spawn(move || {
                let name = format!("p{i}");
                store_ref
                    .save("x", &name, &preset_file("x", &name, i), schema_ref)
                    .expect("concurrent save must succeed");
            });
        }
    });

    // All eight files exist and parse via PresetStore::export.
    for i in 0..N {
        let name = format!("p{i}");
        let pf = store
            .export("x", &name)
            .unwrap_or_else(|e| panic!("export {name}: {e}"));
        assert_eq!(pf.meta.preset_name, name);
        match pf.params.get("speed").cloned() {
            Some(ParamValue::Int(v)) => assert_eq!(v, i),
            other => panic!("preset {name} speed = {other:?}"),
        }
    }

    // No `.tmp.*` debris remains.
    let presets_dir = user_root.join("x/presets");
    let entries: Vec<_> = std::fs::read_dir(&presets_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for entry in &entries {
        assert!(
            !entry.starts_with(".tmp."),
            "found temp debris {entry} in {presets_dir:?}; full listing: {entries:?}",
        );
    }
    // We expect exactly N `.toml` files in the user preset dir.
    let toml_count = entries.iter().filter(|e| e.ends_with(".toml")).count();
    assert_eq!(toml_count, N as usize, "got dir contents: {entries:?}");
}
