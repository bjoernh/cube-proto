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

    // Input
    assert_eq!(cfg.input.system_controller_name_pattern, "8BitDo*");
    assert_eq!(cfg.input.key_back, "BTN_SELECT");
    assert_eq!(cfg.input.key_home, "BTN_START");
    assert_eq!(cfg.input.key_power, "BTN_MODE");

    // IMU
    assert!((cfg.imu.xy_rotation_deg - 0.0).abs() < f64::EPSILON);
    assert!((cfg.imu.xz_rotation_deg - 45.0).abs() < f64::EPSILON);
    assert!((cfg.imu.yz_rotation_deg - 0.0).abs() < f64::EPSILON);

    // Transitions
    assert_eq!(cfg.transitions.mode, "hard_cut");

    // Power
    assert_eq!(cfg.power.idle_blank_after_sec, 0);

    // Default-bind fixture must produce zero warnings.
    assert!(
        report.warnings.is_empty(),
        "expected no warnings for default-bind SDS §6.4 example, got {:?}",
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
