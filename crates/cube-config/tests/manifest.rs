//! Tests for `cube_config::manifest::Manifest` parsing.
//!
//! defines the `manifest.toml` shape:
//!   [app]      name / `display_name` / version / category / icon
//!   [requires] libcube version req / inputs / sensors
//!
//! App `name` reuses the same regex as preset names:
//! `^[a-zA-Z0-9._-]+$` and explicitly rejects `.` and `..`.

use cube_config::{Accent, ConfigError, ManifestCategory, Players, load_manifest};

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
fn manifest_parses_full_example() {
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
fn manifest_requires_table_is_optional() {
    let body = "\
[app]
name = \"picture\"
display_name = \"Picture\"
version = \"0.1.0\"
category = \"demo\"
";
    let p = write(body);
    let m = load_manifest(&p).expect("manifest without [requires] must parse");
    assert!(m.requires.is_none(), "[requires] is optional");
    assert!(m.app.icon.is_none(), "icon is optional");
}

#[test]
fn manifest_missing_app_table_errors() {
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
fn manifest_name_rejects_dot() {
    let body = SNAKE.replace("name = \"snake\"", "name = \".\"");
    let p = write(&body);
    let err = load_manifest(&p).expect_err("name == '.' must be rejected");
    // Per: "must not be equal to '.' or '..'". Surfacing as
    // either Parse or Validation is acceptable; the message must name the
    // offending key.
    let msg = format!("{err}");
    assert!(
        msg.contains("name") || msg.to_lowercase().contains("invalid"),
        "error must mention `name` validation, got {msg:?}"
    );
}

#[test]
fn manifest_name_rejects_dotdot() {
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
fn manifest_name_rejects_slash() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"foo/bar\"");
    let p = write(&body);
    load_manifest(&p).expect_err("name with '/' must be rejected");
}

#[test]
fn manifest_name_rejects_space() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"foo bar\"");
    let p = write(&body);
    load_manifest(&p).expect_err("name with space must be rejected");
}

#[test]
fn manifest_name_rejects_empty() {
    let body = SNAKE.replace("name = \"snake\"", "name = \"\"");
    let p = write(&body);
    load_manifest(&p).expect_err("empty name must be rejected");
}

#[test]
fn manifest_name_accepts_dot_underscore_dash() {
    for ok_name in ["a.b", "a-b", "a_b", "a.b-c_d", "snake_2"] {
        let body = SNAKE.replace("name = \"snake\"", &format!("name = \"{ok_name}\""));
        let p = write(&body);
        let m =
            load_manifest(&p).unwrap_or_else(|e| panic!("name {ok_name:?} must parse, got {e}"));
        assert_eq!(m.app.name, ok_name);
    }
}

#[test]
fn manifest_version_requires_semver() {
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
fn manifest_category_is_closed_enum() {
    let body = SNAKE.replace("category = \"game\"", "category = \"toaster\"");
    let p = write(&body);
    load_manifest(&p).expect_err("unknown category must be rejected (closed enum)");
}

#[test]
fn manifest_accepts_all_known_categories() {
    let cases = [
        ("game", ManifestCategory::Game),
        ("visualizer", ManifestCategory::Visualizer),
        ("utility", ManifestCategory::Utility),
        ("demo", ManifestCategory::Demo),
        ("system", ManifestCategory::System),
        ("wellness", ManifestCategory::Wellness),
    ];
    for (s, expected) in cases {
        let body = SNAKE.replace("category = \"game\"", &format!("category = \"{s}\""));
        let p = write(&body);
        let m = load_manifest(&p).unwrap_or_else(|e| panic!("category {s:?} must parse: {e}"));
        assert_eq!(m.app.category, expected, "category {s:?}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// `[requires] network`, `libcube`/`cubekit` XOR, `[power]` table
// ─────────────────────────────────────────────────────────────────────────────

/// A missing `network` key in `[requires]` defaults to `false`.
#[test]
fn manifest_requires_network_defaults_false() {
    let p = write(SNAKE);
    let m = load_manifest(&p).expect("manifest must parse");
    let requires = m.requires.expect("requires table present");
    assert!(
        !requires.network,
        "missing [requires] network must default to false"
    );
}

/// `[requires] network = true` is honoured.
#[test]
fn manifest_requires_network_true() {
    let body = SNAKE.replace(
        "[requires]\nlibcube = \">= 2.0\"",
        "[requires]\nlibcube = \">= 2.0\"\nnetwork = true",
    );
    let p = write(&body);
    let m = load_manifest(&p).expect("manifest must parse");
    let requires = m.requires.expect("requires table present");
    assert!(
        requires.network,
        "[requires] network = true must round-trip"
    );
}

/// `[requires] cubekit = "..."` is a valid alternative to
/// `libcube` — exactly one SDK-compatibility field is required.
#[test]
fn manifest_requires_cubekit_only_is_ok() {
    let body = "\
[app]
name = \"sand\"
display_name = \"Sand\"
version = \"1.0.0\"
category = \"visualizer\"

[requires]
cubekit = \">= 1.0\"
inputs = []
sensors = []
";
    let p = write(body);
    let m = load_manifest(&p).expect("manifest with only cubekit requirement must parse");
    let requires = m.requires.expect("requires table present");
    assert_eq!(
        requires.cubekit,
        Some(semver::VersionReq::parse(">= 1.0").unwrap())
    );
    assert!(requires.libcube.is_none());
}

/// `[requires] cubego = "..."` is the third valid alternative
/// (the Go SDK) — exactly one SDK-compatibility field is required.
#[test]
fn manifest_requires_cubego_only_is_ok() {
    let body = "\
[app]
name = \"gosnake\"
display_name = \"Go Snake\"
version = \"1.0.0\"
category = \"game\"

[requires]
cubego = \">= 0.3\"
inputs = []
sensors = []
";
    let p = write(body);
    let m = load_manifest(&p).expect("manifest with only cubego requirement must parse");
    let requires = m.requires.expect("requires table present");
    assert_eq!(
        requires.cubego,
        Some(semver::VersionReq::parse(">= 0.3").unwrap())
    );
    assert!(requires.libcube.is_none());
    assert!(requires.cubekit.is_none());
}

/// `cubego` is mutually exclusive with the other two SDK-compat
/// fields — declaring it alongside `cubekit` is rejected.
#[test]
fn manifest_requires_both_cubekit_and_cubego_is_error() {
    let body = SNAKE.replace(
        "[requires]\nlibcube = \">= 2.0\"",
        "[requires]\ncubekit = \">= 1.0\"\ncubego = \">= 0.3\"",
    );
    let p = write(&body);
    let err = load_manifest(&p).expect_err("declaring both cubekit and cubego must be rejected");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

/// Declaring both `libcube` and `cubekit` is an error — exactly
/// one SDK-compatibility field must be present when `[requires]` exists.
#[test]
fn manifest_requires_both_libcube_and_cubekit_is_error() {
    let body = SNAKE.replace(
        "[requires]\nlibcube = \">= 2.0\"",
        "[requires]\nlibcube = \">= 2.0\"\ncubekit = \">= 1.0\"",
    );
    let p = write(&body);
    let err = load_manifest(&p).expect_err("declaring both libcube and cubekit must be rejected");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

/// `[requires]` present but neither `libcube` nor `cubekit`
/// declared is an error.
#[test]
fn manifest_requires_neither_libcube_nor_cubekit_is_error() {
    let body = "\
[app]
name = \"snake\"
display_name = \"Snake\"
version = \"2.0.0\"
category = \"game\"

[requires]
inputs = [\"joystick\"]
sensors = []
";
    let p = write(body);
    let err =
        load_manifest(&p).expect_err("[requires] without libcube or cubekit must be rejected");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

/// A manifest without any `[requires]` table at all remains
/// valid (v5 compat) — the libcube/cubekit XOR is enforced only when
/// `[requires]` is present.
#[test]
fn manifest_no_requires_table_remains_valid() {
    let body = "\
[app]
name = \"picture\"
display_name = \"Picture\"
version = \"0.1.0\"
category = \"demo\"
";
    let p = write(body);
    let m = load_manifest(&p).expect("manifest without [requires] must remain valid for v5 compat");
    assert!(m.requires.is_none());
}

/// Optional `[power] idle_blank = true`.
#[test]
fn manifest_power_table_idle_blank_true() {
    let body = format!("{SNAKE}\n[power]\nidle_blank = true\n");
    let p = write(&body);
    let m = load_manifest(&p).expect("manifest with [power] table must parse");
    let power = m.power.expect("[power] table present");
    assert!(power.idle_blank);
}

/// Optional `[power] idle_blank = false`.
#[test]
fn manifest_power_table_idle_blank_false() {
    let body = format!("{SNAKE}\n[power]\nidle_blank = false\n");
    let p = write(&body);
    let m = load_manifest(&p).expect("manifest with [power] table must parse");
    let power = m.power.expect("[power] table present");
    assert!(!power.idle_blank);
}

/// A missing `[power]` table is treated as `idle_blank = false`.
#[test]
fn manifest_power_table_absent_defaults_false() {
    let p = write(SNAKE);
    let m = load_manifest(&p).expect("manifest must parse");
    match m.power {
        None => {} // absent table — caller treats as idle_blank = false
        Some(power) => assert!(
            !power.idle_blank,
            "absent [power] table must imply idle_blank = false"
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// [app] metadata fields accent / description / preview / players
// ─────────────────────────────────────────────────────────────────────────────

/// A `[app]` manifest carrying an accent, a full `[app]` header with the four
/// new metadata fields. `players` is an inline `{ min, max }`.
const SNAKE_ENRICHED: &str = "\
[app]
name = \"snake\"
display_name = \"Snake\"
version = \"2.0.0\"
category = \"game\"
icon = \"icon.png\"
accent = \"green\"
description = \"Classic snake, six faces\"
preview = \"preview.gif\"
players = { min = 1, max = 2 }

[requires]
libcube = \">= 2.0\"
inputs = [\"joystick\"]
sensors = []
";

/// The four metadata fields parse into `AppSection`.
#[test]
fn manifest_parses_a1_metadata_fields() {
    let p = write(SNAKE_ENRICHED);
    let m = load_manifest(&p).expect("enriched manifest must parse");
    assert_eq!(m.app.accent, Some(Accent::Green));
    assert_eq!(
        m.app.description.as_deref(),
        Some("Classic snake, six faces")
    );
    assert_eq!(m.app.preview.as_deref(), Some("preview.gif"));
    assert_eq!(m.app.players, Some(Players { min: 1, max: 2 }));
}

/// All four metadata fields are optional (absent ⇒ None).
#[test]
fn manifest_a1_metadata_fields_are_optional() {
    let p = write(SNAKE);
    let m = load_manifest(&p).expect("manifest must parse");
    assert!(m.app.accent.is_none());
    assert!(m.app.description.is_none());
    assert!(m.app.preview.is_none());
    assert!(m.app.players.is_none());
}

/// `accent` accepts every token of the closed canonical palette.
#[test]
fn manifest_accepts_all_known_accents() {
    let cases = [
        ("red", Accent::Red),
        ("amber", Accent::Amber),
        ("yellow", Accent::Yellow),
        ("green", Accent::Green),
        ("cyan", Accent::Cyan),
        ("blue", Accent::Blue),
        ("violet", Accent::Violet),
        ("magenta", Accent::Magenta),
    ];
    for (tok, expected) in cases {
        let body = SNAKE.replace(
            "icon = \"icon.png\"",
            &format!("icon = \"icon.png\"\naccent = \"{tok}\""),
        );
        let p = write(&body);
        let m = load_manifest(&p).unwrap_or_else(|e| panic!("accent {tok:?} must parse: {e}"));
        assert_eq!(m.app.accent, Some(expected), "accent {tok:?}");
    }
}

/// An unknown `accent` token is rejected via the closed enum,
/// surfaced as `ConfigError::Parse` (never a panic) so `cubectl doctor` sees it.
#[test]
fn manifest_rejects_unknown_accent_token() {
    let body = SNAKE.replace(
        "icon = \"icon.png\"",
        "icon = \"icon.png\"\naccent = \"chartreuse\"",
    );
    let p = write(&body);
    let err = load_manifest(&p).expect_err("unknown accent token must be rejected");
    assert!(
        matches!(err, ConfigError::Parse { .. }),
        "unknown accent must surface as ConfigError::Parse, got {err:?}"
    );
}

/// `players` with `min > max` is rejected by the loader.
#[test]
fn manifest_rejects_players_min_gt_max() {
    let body = SNAKE.replace(
        "icon = \"icon.png\"",
        "icon = \"icon.png\"\nplayers = { min = 3, max = 2 }",
    );
    let p = write(&body);
    let err = load_manifest(&p).expect_err("players min > max must be rejected");
    assert!(
        matches!(err, ConfigError::Parse { .. }),
        "players min > max must be ConfigError::Parse, got {err:?}"
    );
    assert!(
        format!("{err}").contains("players"),
        "message names players"
    );
}

/// `players` with `min < 1` is rejected by the loader.
#[test]
fn manifest_rejects_players_min_below_one() {
    let body = SNAKE.replace(
        "icon = \"icon.png\"",
        "icon = \"icon.png\"\nplayers = { min = 0, max = 2 }",
    );
    let p = write(&body);
    let err = load_manifest(&p).expect_err("players min < 1 must be rejected");
    assert!(
        matches!(err, ConfigError::Parse { .. }),
        "players min < 1 must be ConfigError::Parse, got {err:?}"
    );
    assert!(
        format!("{err}").contains("players"),
        "message names players"
    );
}

/// `players` with `min == max` (a fixed count) is accepted.
#[test]
fn manifest_accepts_players_min_eq_max() {
    let body = SNAKE.replace(
        "icon = \"icon.png\"",
        "icon = \"icon.png\"\nplayers = { min = 1, max = 1 }",
    );
    let p = write(&body);
    let m = load_manifest(&p).expect("players min == max must parse");
    assert_eq!(m.app.players, Some(Players { min: 1, max: 1 }));
}
