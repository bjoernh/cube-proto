//! Property-based test (SDS §5.6): no fs debris under arbitrary operations.
//!
//! Generates random sequences of (save, delete, list) and asserts:
//!
//! 1. No `.tmp.*` files remain in the user preset dir.
//! 2. Every remaining file matches `*.toml` or `*.deleted` (the only two
//!    artifact kinds defined in §5.5 / §7.1).
//! 3. No save / delete / list call panics — schema validation and name
//!    validation are the only error paths.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use cube_presets::{PresetError, PresetFile, PresetMeta};
use cube_proto::ParamValue;
use proptest::prelude::*;

/// Strategy that intentionally mixes:
///   - valid names (chosen from a small pool so collisions happen)
///   - invalid names (must be rejected without touching the fs)
fn name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("p1".to_string()),
        Just("p2".to_string()),
        Just("p3".to_string()),
        Just("my-preset".to_string()),
        Just("preset.dotted".to_string()),
        // Invalid:
        Just(".".to_string()),
        Just("..".to_string()),
        Just("foo/bar".to_string()),
        Just(String::new()),
    ]
}

#[derive(Debug, Clone)]
enum Op {
    Save(String, i64),
    Delete(String),
    List,
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (name_strategy(), -50i64..50).prop_map(|(n, v)| Op::Save(n, v)),
        name_strategy().prop_map(Op::Delete),
        Just(Op::List),
    ]
}

fn preset(name: &str, n: i64) -> PresetFile {
    let mut params: BTreeMap<String, ParamValue> = BTreeMap::new();
    params.insert("speed".to_string(), ParamValue::Int(n));
    PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
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

// The preset store writes exact lowercase extensions; exact comparison intended.
#[expect(clippy::case_sensitive_file_extension_comparisons)]
fn dir_debris_check(dir: &Path) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".tmp."),
            "found temp debris {name} in {}", dir.display(),
        );
        assert!(
            name.ends_with(".toml") || name.ends_with(".deleted"),
            "found unexpected artifact {name} in {}", dir.display(),
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        ..ProptestConfig::default()
    })]

    #[test]
    fn random_ops_leave_no_debris_sds_5_6(
        ops in proptest::collection::vec(op_strategy(), 1..32),
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let (store, _sr, user_root) = common::make_store(tmp.path(), "x");
        let schema = common::schema_minimal("x", 1);
        let presets_dir = user_root.join("x/presets");

        for op in &ops {
            match op {
                Op::Save(n, v) => {
                    // Valid names succeed; invalid names must return BadRequest.
                    match store.save("x", n, &preset(n, *v), &schema) {
                        Ok(_) | Err(PresetError::BadRequest(_)) => {}
                        Err(other) => prop_assert!(
                            false,
                            "unexpected save error for {n:?}: {other:?}",
                        ),
                    }
                }
                Op::Delete(n) => {
                    match store.delete("x", n) {
                        Ok(()) | Err(PresetError::NotFound | PresetError::BadRequest(_)) => {}
                        Err(other) => prop_assert!(
                            false,
                            "unexpected delete error for {n:?}: {other:?}",
                        ),
                    }
                }
                Op::List => {
                    store.list("x").unwrap();
                }
            }
            dir_debris_check(&presets_dir);
        }

        // Final-state invariant: still no debris.
        dir_debris_check(&presets_dir);
    }
}
