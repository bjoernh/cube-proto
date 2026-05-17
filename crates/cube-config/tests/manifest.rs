//! Tests for `cube_config::manifest::Manifest` parsing (SDS §7.2).
//!
//! SDS §7.2 defines the `manifest.toml` shape:
//!   [app]      name / display_name / version / category / icon
//!   [requires] libcube version req / inputs / sensors
//!
//! App `name` reuses the same regex as preset names (SDS §5.5):
//! `^[a-zA-Z0-9._-]+$` and explicitly rejects `.` and `..`.

use cube_config::{ConfigError, ManifestCategory, load_manifest};

fn write(toml: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    // keep tempdir alive
    let dir = Box::leak(Box::new(dir));
    let p = dir.path().join("manifest.toml");
    std::fs::write(&p, toml).unwrap();
    p
}

const SNAKE: &str = "\
[app]
name = \"snake\"
display_name = \"Snake\"
version = \"2.0.0\"
category = \"game\"
icon = \"icon.png\"

[requires]
libcube = \">= 2.0\"
inputs = [\"joystick\"]
sensors = []
";

#[test]
fn manifest_parses_full_example_sds_7_2() {
    let p = write(SNAKE);
    let m = load_manifest(&p).expect("happy-path manifest must parse");

    assert_eq!(m.app.name, "snake");
    assert_eq!(m.app.display_name, "Snake");
    assert_eq!(m.app.version, semver::Version::new(2, 0, 0));
    assert_eq!(m.app.category, ManifestCategory::Game);
    assert_eq!(m.app.icon.as_deref(), Some("icon.png"));

    let requires = m.requires.expect("requires table present");
    assert_eq!(
        requires.libcube,
        Some(semver::VersionReq::parse(">= 2.0").unwrap())
    );
    assert_eq!(requires.inputs, vec!["joystick".to_owned()]);
    assert!(requires.sensors.is_empty());
}

#[test]
fn manifest_requires_table_is_optional_sds_7_2() {
    let body = "\
[app]
name = \"picture\"
display_name = \"Picture\"
version = \"0.1.0\"
category = \"demo\"
";
    let p = write(body);
    let m = load_manifest(&p).expect("manifest without [requires] must parse");
    assert!(m.requires.is_none(), "[requires] is optional per SDS §7.2");
    assert!(m.app.icon.is_none(), "icon is optional per SDS §7.2");
}

#[test]
fn manifest_missing_app_table_errors_sds_7_2() {
    let body = "\
[requires]
libcube = \">= 2.0\"
inputs = []
sensors = []
";
    let p = write(body);
    let err = load_manifest(&p).expect_err("missing [app] must error");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

#[test]
fn manifest_name_rejects_dot_sds_7_2() {
    let body = SNAKE.replace("name = \"snake\"", "name = \".\"");
    let p = write(&body);
    let err = load_manifest(&p).expect_err("name == '.' must be rejected");
    // Per SDS §5.5: "must not be equal to '.' or '..'". Surfacing as
    // either Parse or Validation is acceptable; the message must name the
    // offending key.
    let msg = format!("{err}");
    assert!(
        msg.contains("name") || msg.to_lowercase().contains("invalid"),
        "error must mention `name` validation, got {msg:?}"
    );
}

#[test]
fn manifest_name_rejects_dotdot_sds_7_2() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"..\"");
    let p = write(&body);
    let err = load_manifest(&p).expect_err("name == '..' must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("name") || msg.to_lowercase().contains("invalid"),
        "error must mention `name` validation, got {msg:?}"
    );
}

#[test]
fn manifest_name_rejects_slash_sds_7_2() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"foo/bar\"");
    let p = write(&body);
    load_manifest(&p).expect_err("name with '/' must be rejected (SDS §5.5)");
}

#[test]
fn manifest_name_rejects_space_sds_7_2() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"foo bar\"");
    let p = write(&body);
    load_manifest(&p).expect_err("name with space must be rejected (SDS §5.5)");
}

#[test]
fn manifest_name_rejects_empty_sds_7_2() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"\"");
    let p = write(&body);
    load_manifest(&p).expect_err("empty name must be rejected (SDS §5.5)");
}

#[test]
fn manifest_name_accepts_dot_underscore_dash_sds_7_2() {
    for ok_name in ["a.b", "a-b", "a_b", "a.b-c_d", "snake_2"] {
        let body = SNAKE.replace("name = \"snake\"", &format!("name = \"{ok_name}\""));
        let p = write(&body);
        let m = load_manifest(&p)
            .unwrap_or_else(|e| panic!("name {ok_name:?} must parse, got {e}"));
        assert_eq!(m.app.name, ok_name);
    }
}

#[test]
fn manifest_version_requires_semver_sds_7_2() {
    let body = SNAKE.replace("version = \"2.0.0\"", "version = \"not-a-semver\"");
    let p = write(&body);
    let err = load_manifest(&p).expect_err("non-semver version must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("version") || msg.to_lowercase().contains("semver"),
        "error should mention version/semver, got {msg:?}"
    );
}

#[test]
fn manifest_category_is_closed_enum_sds_7_2() {
    let body = SNAKE.replace("category = \"game\"", "category = \"toaster\"");
    let p = write(&body);
    load_manifest(&p).expect_err("unknown category must be rejected (closed enum)");
}

#[test]
fn manifest_accepts_all_known_categories_sds_7_2() {
    let cases = [
        ("game", ManifestCategory::Game),
        ("visualizer", ManifestCategory::Visualizer),
        ("utility", ManifestCategory::Utility),
        ("demo", ManifestCategory::Demo),
        ("system", ManifestCategory::System),
    ];
    for (s, expected) in cases {
        let body = SNAKE.replace("category = \"game\"", &format!("category = \"{s}\""));
        let p = write(&body);
        let m = load_manifest(&p)
            .unwrap_or_else(|e| panic!("category {s:?} must parse: {e}"));
        assert_eq!(m.app.category, expected, "category {s:?}");
    }
}
