//! Tests for `cube_config::schema::Schema`.
//!
//! Schema is fully declarative — every per-param field listed in has to
//! round-trip. Tests cover: happy path, missing `schema_version`, missing
//! `default`, `min > max`, enum without `values`, dimensionality mismatch on
//! `vec2`, and insertion-order preservation via `IndexMap`.

use cube_config::{ParamType, load_schema};
use cube_proto::ParamValue;

fn write(toml: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let dir = Box::leak(Box::new(dir));
    let p = dir.path().join("schema.toml");
    std::fs::write(&p, toml).unwrap();
    p
}

#[test]
fn schema_parses_happy_path() {
    let body = r##"
schema_version = 1

[params.speed]
key = "speed"
type = "int"
default = 5
min = 1
max = 10
step = 1
unit = "level"
required = true
readonly = false
shareable = true

[params.speed.ui]
group = "gameplay"
label = "Speed"
description = "Snake step rate"

[params.color]
key = "color"
type = "color"
default = "#FF8800"

[params.color.ui]
group = "appearance"

[params.mode]
key = "mode"
type = "enum"
default = "classic"
values = ["classic", "neon", "retro"]

[params.mode.ui]
"##;
    let p = write(body);
    let s = load_schema(&p).expect("happy-path schema must parse");

    assert_eq!(s.schema_version, 1);
    assert!(s.params.contains_key("speed"));
    assert!(s.params.contains_key("color"));
    assert!(s.params.contains_key("mode"));

    let speed = &s.params["speed"];
    assert_eq!(speed.key, "speed");
    assert!(matches!(speed.ty, ParamType::Int));
    assert_eq!(speed.default, ParamValue::Int(5));
    assert_eq!(speed.min, Some(ParamValue::Int(1)));
    assert_eq!(speed.max, Some(ParamValue::Int(10)));
    assert_eq!(speed.step, Some(ParamValue::Int(1)));
    assert_eq!(speed.unit.as_deref(), Some("level"));
    assert!(speed.required);
    assert!(!speed.readonly);
    assert!(speed.shareable);
    assert_eq!(speed.ui.group.as_deref(), Some("gameplay"));
    assert_eq!(speed.ui.label.as_deref(), Some("Speed"));
    assert_eq!(speed.ui.description.as_deref(), Some("Snake step rate"));

    let mode = &s.params["mode"];
    assert!(matches!(mode.ty, ParamType::Enum));
    assert_eq!(
        mode.enum_values.as_deref(),
        Some(&["classic".to_owned(), "neon".to_owned(), "retro".to_owned()][..]),
    );
}

#[test]
fn schema_defaults_required_true_readonly_false_shareable_true() {
    let body = r#"
schema_version = 1

[params.foo]
key = "foo"
type = "bool"
default = true

[params.foo.ui]
"#;
    let p = write(body);
    let s = load_schema(&p).expect("schema must parse with defaults");
    let foo = &s.params["foo"];
    assert!(foo.required, "required defaults to true");
    assert!(!foo.readonly, "readonly defaults to false");
    assert!(foo.shareable, "shareable defaults to true");
}

#[test]
fn schema_missing_schema_version_errors() {
    let body = r#"
[params.foo]
key = "foo"
type = "int"
default = 0
"#;
    let p = write(body);
    load_schema(&p).expect_err("missing schema_version must error");
}

#[test]
fn schema_param_without_default_errors() {
    let body = r#"
schema_version = 1

[params.foo]
key = "foo"
type = "int"
min = 0
max = 10

[params.foo.ui]
"#;
    let p = write(body);
    load_schema(&p).expect_err("param without `default` must error");
}

#[test]
fn schema_min_greater_than_max_errors() {
    let body = r#"
schema_version = 1

[params.foo]
key = "foo"
type = "int"
default = 5
min = 10
max = 1

[params.foo.ui]
"#;
    let p = write(body);
    load_schema(&p).expect_err("min > max must error");
}

#[test]
fn schema_enum_without_values_errors() {
    let body = r#"
schema_version = 1

[params.mode]
key = "mode"
type = "enum"
default = "classic"

[params.mode.ui]
"#;
    let p = write(body);
    load_schema(&p).expect_err("enum without values[] must error");
}

#[test]
fn schema_vec2_default_with_three_components_errors() {
    // A Vec2 default given as a 3-tuple is a dimensionality mismatch.
    let body = r#"
schema_version = 1

[params.offset]
key = "offset"
type = "vec2"
default = { x = 1.0, y = 2.0, z = 3.0 }

[params.offset.ui]
"#;
    let p = write(body);
    load_schema(&p).expect_err("vec2 default with 3 components must error");
}

#[test]
fn schema_indexmap_preserves_insertion_order() {
    let body_ab = r#"
schema_version = 1

[params.alpha]
key = "alpha"
type = "int"
default = 0
[params.alpha.ui]

[params.beta]
key = "beta"
type = "int"
default = 0
[params.beta.ui]
"#;
    let body_ba = r#"
schema_version = 1

[params.beta]
key = "beta"
type = "int"
default = 0
[params.beta.ui]

[params.alpha]
key = "alpha"
type = "int"
default = 0
[params.alpha.ui]
"#;
    let p_ab = write(body_ab);
    let p_ba = write(body_ba);
    let s_ab = load_schema(&p_ab).unwrap();
    let s_ba = load_schema(&p_ba).unwrap();

    let keys_ab: Vec<&String> = s_ab.params.keys().collect();
    let keys_ba: Vec<&String> = s_ba.params.keys().collect();
    assert_eq!(
        keys_ab,
        vec![&"alpha".to_owned(), &"beta".to_owned()],
        "alpha-before-beta source must yield [alpha, beta]"
    );
    assert_eq!(
        keys_ba,
        vec![&"beta".to_owned(), &"alpha".to_owned()],
        "beta-before-alpha source must yield [beta, alpha]"
    );
    assert_ne!(
        keys_ab, keys_ba,
        "IndexMap must preserve insertion order from the TOML source"
    );
}
