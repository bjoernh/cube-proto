//! Tests for `cube_config::system::SystemConfig` parsing.
//!
//! These tests pin the parsing surface against the verbatim
//! example fixture stored at `tests/fixtures/system_full.toml`.

use std::path::PathBuf;

use cube_config::{AppsConfig, ConfigError, load_system};

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}

#[test]
fn system_toml_parses_full_example() {
    let path = fixture("system_full.toml");
    let (cfg, report) = load_system(&path).expect(" fixture must parse");

    // Display
    assert_eq!(cfg.display.drm_driver, "cube-fpga-rpispi");
    assert_eq!(cfg.display.connector, "Cube-1");
    assert_eq!(cfg.display.mode, "384x64@60");
    assert_eq!(cfg.display.refresh_hz, 60);
    assert_eq!(cfg.display.spi_clock_hz, 35_000_000);

    // Remote render
    assert!(cfg.remote_render.enabled);
    assert_eq!(cfg.remote_render.bind, "127.0.0.1");
    assert_eq!(cfg.remote_render.port, 2017);
    assert_eq!(cfg.remote_render.mtu_hint, "jumbo_recommended");

    // Input — the example uses the (now deprecated) legacy name pattern;
    // it still parses, and loading folds it into `device_allow`
    // and surfaces an `INPUT_DEPRECATED_NAME_PATTERN` warning (asserted below).
    assert_eq!(
        cfg.input.system_controller_name_pattern.as_deref(),
        Some("8BitDo*")
    );
    assert!(
        cfg.input.device_allow.iter().any(|e| e == "8BitDo*"),
        "deprecated pattern must seed device_allow, got {:?}",
        cfg.input.device_allow,
    );
    assert_eq!(cfg.input.key_back, "Select");
    assert_eq!(cfg.input.key_home, "Start");
    assert_eq!(cfg.input.key_power, "Guide");

    // IMU
    assert!((cfg.imu.xy_rotation_deg - 0.0).abs() < f64::EPSILON);
    assert!((cfg.imu.xz_rotation_deg - 45.0).abs() < f64::EPSILON);
    assert!((cfg.imu.yz_rotation_deg - 0.0).abs() < f64::EPSILON);

    // Transitions
    assert_eq!(cfg.transitions.mode, "hard_cut");

    // Power
    assert_eq!(cfg.power.idle_blank_after_sec, 0);

    // Apps: the fixture has no [apps] table, so
    // max_resident must default to 1 (v5-equivalent behaviour).
    assert_eq!(cfg.apps.max_resident, 1);

    // Default-bind fixture must not produce a public-bind warning (the loopback
    // bind is safe). The fixture's legacy `system_controller_name_pattern` does
    // trigger the one-time `INPUT_DEPRECATED_NAME_PATTERN` deprecation warning.
    assert!(
        report
            .warnings
            .iter()
            .all(|w| w.code != "NETWORK_PUBLIC_BIND"),
        "loopback bind must not warn about public exposure, got {:?}",
        report.warnings,
    );
    assert_eq!(
        report
            .warnings
            .iter()
            .filter(|w| w.code == "INPUT_DEPRECATED_NAME_PATTERN")
            .count(),
        1,
        "deprecated name pattern must warn exactly once, got {:?}",
        report.warnings,
    );
    assert!(report.errors.is_empty());
}

#[test]
fn system_toml_debug_section_is_optional_defaults_to_none() {
    // A config with no [debug] section must parse, with capture disabled.
    let body = "\
[display]
drm_driver = \"vkms\"
connector = \"Writeback-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[remote_render]
enabled = false
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"

[input]
system_controller_name_pattern = \"8BitDo*\"
key_back = \"BTN_SELECT\"
key_home = \"BTN_START\"
key_power = \"BTN_MODE\"

[imu]
xy_rotation_deg = 0.0
xz_rotation_deg = 0.0
yz_rotation_deg = 0.0

[transitions]
mode = \"hard_cut\"

[power]
idle_blank_after_sec = 0
";
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("system.toml");
    std::fs::write(&p, body).unwrap();

    let (cfg, _report) = load_system(&p).expect("config without [debug] must parse");
    assert!(
        cfg.debug.capture_on_present.is_none(),
        "absent [debug] section must default capture_on_present to None"
    );
}

#[test]
fn system_toml_debug_capture_on_present_parses() {
    // Setting [debug].capture_on_present is honoured as a path (mirrors the
    // --capture-on-present CLI flag).
    let body = "\
[display]
drm_driver = \"vkms\"
connector = \"Writeback-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[remote_render]
enabled = false
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"

[input]
system_controller_name_pattern = \"8BitDo*\"
key_back = \"BTN_SELECT\"
key_home = \"BTN_START\"
key_power = \"BTN_MODE\"

[imu]
xy_rotation_deg = 0.0
xz_rotation_deg = 0.0
yz_rotation_deg = 0.0

[transitions]
mode = \"hard_cut\"

[power]
idle_blank_after_sec = 0

[debug]
capture_on_present = \"/tmp/cube_fb.bin\"
";
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("system.toml");
    std::fs::write(&p, body).unwrap();

    let (cfg, _report) = load_system(&p).expect("config with [debug] must parse");
    assert_eq!(
        cfg.debug.capture_on_present.as_deref(),
        Some(std::path::Path::new("/tmp/cube_fb.bin")),
    );
}

#[test]
fn system_toml_missing_required_field_errors() {
    // Drop `display.drm_driver` (a required key) and expect ConfigError::Parse.
    let body = "\
[display]
connector = \"Cube-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[remote_render]
enabled = true
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"

[input]
system_controller_name_pattern = \"8BitDo*\"
key_back = \"Select\"
key_home = \"Start\"
key_power = \"Guide\"

[imu]
xy_rotation_deg = 0.0
xz_rotation_deg = 0.0
yz_rotation_deg = 0.0

[transitions]
mode = \"hard_cut\"

[power]
idle_blank_after_sec = 0
";
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("system.toml");
    std::fs::write(&p, body).unwrap();

    let err = load_system(&p).expect_err("missing drm_driver must error");
    match err {
        ConfigError::Parse { path, message } => {
            assert_eq!(path, p);
            assert!(
                message.contains("drm_driver") || message.to_lowercase().contains("missing"),
                "Parse message should mention the missing field, got: {message}"
            );
        }
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[test]
fn system_toml_malformed_toml_errors() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("system.toml");
    std::fs::write(&p, "this is = = not toml [\n").unwrap();

    let err = load_system(&p).expect_err("malformed toml must error");
    match err {
        ConfigError::Parse { path, message } => {
            assert_eq!(path, p);
            assert!(!message.is_empty(), "parse message must not be empty");
        }
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[test]
fn system_toml_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("does-not-exist.toml");

    let err = load_system(&p).expect_err("missing file must error");
    match err {
        ConfigError::Missing(missing) => assert_eq!(missing, p),
        other => panic!("expected ConfigError::Missing, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// `[apps] max_resident`
// ─────────────────────────────────────────────────────────────────────────────

fn write(toml: &str) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let dir = Box::leak(Box::new(dir));
    let p = dir.path().join("system.toml");
    std::fs::write(&p, toml).unwrap();
    p
}

/// A missing `[apps]` table defaults `max_resident` to `1`
/// (reproduces v5's implicit single-foreground behaviour).
#[test]
fn system_toml_apps_table_absent_defaults_max_resident_one() {
    let path = fixture("system_full.toml");
    let (cfg, _report) = load_system(&path).expect("fixture must parse");
    assert_eq!(cfg.apps.max_resident, 1);
}

/// An explicit `[apps] max_resident` value is honoured.
#[test]
fn system_toml_apps_max_resident_explicit_value() {
    let base = std::fs::read_to_string(fixture("system_full.toml")).unwrap();
    let body = format!("{base}\n[apps]\nmax_resident = 3\n");
    let p = write(&body);

    let (cfg, report) = load_system(&p).expect("system.toml with [apps] must parse");
    assert_eq!(cfg.apps.max_resident, 3);
    assert!(report.errors.is_empty());
}

/// `[apps] max_resident = 0` is invalid (every cube must run at
/// least one app).
#[test]
fn system_toml_apps_max_resident_zero_is_error() {
    let base = std::fs::read_to_string(fixture("system_full.toml")).unwrap();
    let body = format!("{base}\n[apps]\nmax_resident = 0\n");
    let p = write(&body);

    let err = load_system(&p).expect_err("max_resident = 0 must be rejected");
    match err {
        ConfigError::Parse { message, .. } => {
            assert!(
                message.to_lowercase().contains("max_resident")
                    || message.to_lowercase().contains("zero")
                    || message.to_lowercase().contains("at least"),
                "error message should mention max_resident / zero, got: {message}"
            );
        }
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// App-store (M1): `[apps] allow_remote_install` + `preview_timeout_secs`
//
// Two new `[apps]` keys, both with serde defaults so existing `system.toml`
// files (and an `[apps]` table that only sets `max_resident`) keep parsing:
//   - `allow_remote_install` (bool, default `true`)  — gates the apt_install verb
//   - `preview_timeout_secs` (u32,  default `15`)    — on-cube preview auto-hide
// ─────────────────────────────────────────────────────────────────────────────

/// App-store M1: a config with no `[apps]` table defaults the two new keys to
/// `allow_remote_install = true` and `preview_timeout_secs = 15`.
#[test]
fn system_toml_apps_install_preview_default_when_apps_absent_appstore() {
    let path = fixture("system_full.toml");
    let (cfg, report) = load_system(&path).expect("fixture without [apps] must parse");
    assert!(
        cfg.apps.allow_remote_install,
        "allow_remote_install must default to true"
    );
    assert_eq!(
        cfg.apps.preview_timeout_secs, 15,
        "preview_timeout_secs must default to 15"
    );
    assert!(report.errors.is_empty());
}

/// App-store M1: an `[apps]` table that sets only `max_resident` still defaults
/// the two new keys — proving they are independently `#[serde(default)]`.
#[test]
fn system_toml_apps_install_preview_default_when_keys_omitted_appstore() {
    let base = std::fs::read_to_string(fixture("system_full.toml")).unwrap();
    let body = format!("{base}\n[apps]\nmax_resident = 2\n");
    let p = write(&body);

    let (cfg, _report) = load_system(&p).expect("[apps] with only max_resident must parse");
    assert_eq!(cfg.apps.max_resident, 2);
    assert!(
        cfg.apps.allow_remote_install,
        "omitted key defaults to true"
    );
    assert_eq!(
        cfg.apps.preview_timeout_secs, 15,
        "omitted key defaults to 15"
    );
}

/// App-store M1: explicit `allow_remote_install` / `preview_timeout_secs` values
/// are honoured, alongside `max_resident`.
#[test]
fn system_toml_apps_install_preview_explicit_values_appstore() {
    let base = std::fs::read_to_string(fixture("system_full.toml")).unwrap();
    let body = format!(
        "{base}\n[apps]\nmax_resident = 3\nallow_remote_install = false\npreview_timeout_secs = 30\n"
    );
    let p = write(&body);

    let (cfg, report) = load_system(&p).expect("explicit [apps] keys must parse");
    assert_eq!(cfg.apps.max_resident, 3);
    assert!(
        !cfg.apps.allow_remote_install,
        "explicit false must be honoured"
    );
    assert_eq!(
        cfg.apps.preview_timeout_secs, 30,
        "explicit 30 must be honoured"
    );
    assert!(report.errors.is_empty());
}

/// App-store M1: `AppsConfig` serde round-trips — serialize non-default values,
/// parse them back, get the same values (the struct is `#[serde(default)]`).
#[test]
fn apps_config_serde_round_trip_appstore() {
    let original = AppsConfig {
        max_resident: 4,
        allow_remote_install: false,
        preview_timeout_secs: 42,
    };
    let text = toml::to_string(&original).expect("AppsConfig must serialize");
    let parsed: AppsConfig = toml::from_str(&text).expect("AppsConfig must deserialize");
    assert_eq!(parsed.max_resident, original.max_resident);
    assert_eq!(parsed.allow_remote_install, original.allow_remote_install);
    assert_eq!(parsed.preview_timeout_secs, original.preview_timeout_secs);
}

/// App-store M1: `AppsConfig::default` yields all three documented defaults.
#[test]
fn apps_config_default_values_appstore() {
    let d = AppsConfig::default();
    assert_eq!(d.max_resident, 1);
    assert!(d.allow_remote_install);
    assert_eq!(d.preview_timeout_secs, 15);
}

// ─────────────────────────────────────────────────────────────────────────────
// Capability-discovery `[input]` policy fields
//
// Discovery is capability-based; `system_controller_name_pattern` is demoted to
// an optional deprecated alias and `device_allow` / `device_deny` /
// `max_players` are new policy fields, all with serde defaults so existing and
// older `system.toml` files still parse.
// ─────────────────────────────────────────────────────────────────────────────

/// Build a full `system.toml` whose `[input]` table is exactly `input_block`
/// (the rest of the document is a minimal valid config).
fn system_with_input(input_block: &str) -> PathBuf {
    let body = format!(
        "\
[display]
drm_driver = \"vkms\"
connector = \"Writeback-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[remote_render]
enabled = false
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"

[input]
{input_block}

[imu]
xy_rotation_deg = 0.0
xz_rotation_deg = 0.0
yz_rotation_deg = 0.0

[transitions]
mode = \"hard_cut\"

[power]
idle_blank_after_sec = 0
"
    );
    write(&body)
}

/// an `[input]` block that omits the deprecated
/// `system_controller_name_pattern` parses, and the alias reads back as `None`.
#[test]
fn system_toml_parses_without_legacy_pattern_phase3() {
    let p = system_with_input(
        "\
key_back = \"Select\"
key_home = \"Start\"
key_power = \"Guide\"",
    );
    let (cfg, report) = load_system(&p).expect("policy-only [input] block must parse");
    assert_eq!(cfg.input.system_controller_name_pattern, None);
    assert!(report.errors.is_empty());
}

/// `device_allow` / `device_deny` / `max_players` carry serde
/// defaults, so an `[input]` block omitting them is valid and yields the
/// documented defaults (empty lists, `max_players = 8`).
#[test]
fn system_toml_policy_fields_default_when_absent_phase3() {
    let p = system_with_input(
        "\
key_back = \"Select\"
key_home = \"Start\"
key_power = \"Guide\"",
    );
    let (cfg, _report) = load_system(&p).expect("must parse with defaulted policy fields");
    assert!(
        cfg.input.device_allow.is_empty(),
        "device_allow defaults to empty, got {:?}",
        cfg.input.device_allow
    );
    assert!(
        cfg.input.device_deny.is_empty(),
        "device_deny defaults to empty, got {:?}",
        cfg.input.device_deny
    );
    assert_eq!(cfg.input.max_players, 8, "max_players defaults to 8");
}

/// explicit `device_allow` / `device_deny` / `max_players` values
/// are honoured (entries may be VID:PID or name-glob; here both forms appear).
#[test]
fn system_toml_policy_fields_explicit_values_phase3() {
    let p = system_with_input(
        "\
key_back = \"Select\"
key_home = \"Start\"
key_power = \"Guide\"
device_allow = [\"2dc8:9018\", \"8BitDo*\"]
device_deny = [\"054c:05c4\"]
max_players = 4",
    );
    let (cfg, _report) = load_system(&p).expect("explicit policy fields must parse");
    assert_eq!(cfg.input.device_allow, vec!["2dc8:9018", "8BitDo*"]);
    assert_eq!(cfg.input.device_deny, vec!["054c:05c4"]);
    assert_eq!(cfg.input.max_players, 4);
}
