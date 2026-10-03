use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use cube_presets::{PresetFile, PresetMeta};
use cube_proto::ParamValue;

/// Serialize a `PresetFile` and verify the `created` field is unquoted.
#[test]
fn serializes_created_as_native_toml_datetime() {
    let dt = DateTime::<Utc>::from_timestamp(1_716_000_000, 0).unwrap();
    let pf = PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "test".to_string(),
            author: None,
            description: None,
            created: dt,
            version: "1.0".to_string(),
        },
        params: BTreeMap::new(),
    };

    let toml_str = toml::to_string(&pf).unwrap();
    assert!(
        toml_str.contains("created = 2024-05-18"),
        "expected native TOML datetime (unquoted), got:\n{toml_str}"
    );
    assert!(
        !toml_str.contains("created = \""),
        "created must NOT be quoted:\n{toml_str}"
    );
}

/// Deserialize a TOML string with a quoted datetime (legacy format).
#[test]
fn deserializes_quoted_datetime_for_backward_compat() {
    let toml_str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "test"
created = "2026-05-17T00:00:00Z"
version = "1.0"

[params]
"#;

    let pf: PresetFile = toml::from_str(toml_str).unwrap();
    assert_eq!(pf.meta.created.to_rfc3339(), "2026-05-17T00:00:00+00:00");
}

/// Deserialize a TOML string with a native datetime (new format).
#[test]
fn deserializes_native_datetime() {
    let toml_str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "test"
created = 2026-05-17T00:00:00Z
version = "1.0"

[params]
"#;

    let pf: PresetFile = toml::from_str(toml_str).unwrap();
    assert_eq!(pf.meta.created.to_rfc3339(), "2026-05-17T00:00:00+00:00");
}

/// Full round-trip: serialize then deserialize preserves the timestamp.
#[test]
fn roundtrip_preserves_timestamp() {
    let original = PresetFile {
        meta: PresetMeta {
            app: "x".to_string(),
            schema_version: 1,
            preset_name: "roundtrip".to_string(),
            author: Some("agent".to_string()),
            description: Some("roundtrip test".to_string()),
            created: DateTime::<Utc>::from_timestamp(1_716_000_000, 123_000_000).unwrap(),
            version: "1.0".to_string(),
        },
        params: {
            let mut m = BTreeMap::new();
            m.insert("speed".to_string(), ParamValue::Int(7));
            m
        },
    };

    let toml_str = toml::to_string(&original).unwrap();
    let loaded: PresetFile = toml::from_str(&toml_str).unwrap();
    assert_eq!(original.meta.created, loaded.meta.created);
}

/// A native datetime carrying a non-Z offset must map to the correct UTC
/// instant, not be reinterpreted as wall-clock UTC.
#[test]
fn deserializes_offset_datetime_to_correct_instant() {
    let toml_str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "test"
created = 2026-05-17T02:00:00+02:00
version = "1.0"

[params]
"#;

    let pf: PresetFile = toml::from_str(toml_str).unwrap();
    // 02:00 at +02:00 is 00:00Z.
    assert_eq!(pf.meta.created.to_rfc3339(), "2026-05-17T00:00:00+00:00");
}

/// A client-supplied preset whose `created` is a date-only (or time-only)
/// TOML datetime must surface a serde error, never panic the deserializer.
#[test]
fn date_only_created_errors_not_panics() {
    let toml_str = r#"
[meta]
app = "x"
schema_version = 1
preset_name = "test"
created = 2026-05-17
version = "1.0"

[params]
"#;

    let err = toml::from_str::<PresetFile>(toml_str).unwrap_err();
    assert!(
        err.to_string().contains("missing a time"),
        "expected a missing-time error, got: {err}"
    );
}
