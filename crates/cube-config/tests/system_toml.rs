//! Tests for `cube_config::system::SystemConfig` parsing (SDS §6.4).
//!
//! These are RED tests authored in Wave 2: the production code does not
//! exist yet. They pin the parsing surface against the verbatim SDS §6.4
//! example fixture stored at `tests/fixtures/system_full.toml`.

use std::path::PathBuf;

use cube_config::{ConfigError, load_system};

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p
}

#[test]
fn system_toml_parses_full_example_sds_6_4() {
    let path = fixture("system_full.toml");
    let (cfg, report) = load_system(&path).expect("SDS §6.4 fixture must parse");

    // Display
    assert_eq!(cfg.display.drm_driver, "cube-fpga-rpispi");
    assert_eq!(cfg.display.connector, "Cube-1");
    assert_eq!(cfg.display.mode, "384x64@60");
    assert_eq!(cfg.display.refresh_hz, 60);
    assert_eq!(cfg.display.spi_clock_hz, 35_000_000);

    // Remote render (SDS v5 §6.4)
    assert!(cfg.remote_render.enabled);
    assert_eq!(cfg.remote_render.bind, "127.0.0.1");
    assert_eq!(cfg.remote_render.port, 2017);
    assert_eq!(cfg.remote_render.mtu_hint, "jumbo_recommended");

    // Input — the §6.4 example uses the (now deprecated) legacy name pattern;
    // it still parses, and cube-gamepad Phase 3 (M8) folds it into `device_allow`
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

    // Apps (SDS v6 §1/§7.2): the fixture has no [apps] table, so
    // max_resident must default to 1 (v5-equivalent behaviour).
    assert_eq!(cfg.apps.max_resident, 1);

    // Default-bind fixture must not produce a public-bind warning (the loopback
    // bind is safe). The fixture's legacy `system_controller_name_pattern` does
    // trigger the one-time `INPUT_DEPRECATED_NAME_PATTERN` deprecation warning.
    assert!(
        report.warnings.iter().all(|w| w.code != "NETWORK_PUBLIC_BIND"),
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
fn system_toml_missing_required_field_errors_sds_6_4() {
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
fn system_toml_malformed_toml_errors_sds_6_4() {
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
fn system_toml_missing_file_errors_sds_6_4() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("does-not-exist.toml");

    let err = load_system(&p).expect_err("missing file must error");
    match err {
        ConfigError::Missing(missing) => assert_eq!(missing, p),
        other => panic!("expected ConfigError::Missing, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDS v6 §1/§7.2: `[apps] max_resident`
// ─────────────────────────────────────────────────────────────────────────────

fn write(toml: &str) -> PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let dir = Box::leak(Box::new(dir));
    let p = dir.path().join("system.toml");
    std::fs::write(&p, toml).unwrap();
    p
}

/// SDS v6 §1: a missing `[apps]` table defaults `max_resident` to `1`
/// (reproduces v5's implicit single-foreground behaviour).
#[test]
fn system_toml_apps_table_absent_defaults_max_resident_one_sds_6_4() {
    let path = fixture("system_full.toml");
    let (cfg, _report) = load_system(&path).expect("fixture must parse");
    assert_eq!(cfg.apps.max_resident, 1);
}

/// SDS v6 §1: an explicit `[apps] max_resident` value is honoured.
#[test]
fn system_toml_apps_max_resident_explicit_value_sds_6_4() {
    let base = std::fs::read_to_string(fixture("system_full.toml")).unwrap();
    let body = format!("{base}\n[apps]\nmax_resident = 3\n");
    let p = write(&body);

    let (cfg, report) = load_system(&p).expect("system.toml with [apps] must parse");
    assert_eq!(cfg.apps.max_resident, 3);
    assert!(report.errors.is_empty());
}

/// SDS v6 §1: `[apps] max_resident = 0` is invalid (every cube must run at
/// least one app).
#[test]
fn system_toml_apps_max_resident_zero_is_error_sds_6_4() {
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
// cube-gamepad Phase 3 (M8): capability-discovery `[input]` policy fields
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

/// Phase 3 (M8): an `[input]` block that omits the deprecated
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

/// Phase 3 (M8): `device_allow` / `device_deny` / `max_players` carry serde
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

/// Phase 3 (M8): explicit `device_allow` / `device_deny` / `max_players` values
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
