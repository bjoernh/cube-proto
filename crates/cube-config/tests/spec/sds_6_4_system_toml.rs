//! Verbatim SDS §6.4 worked-example pinning test.
//!
//! The fixture at `tests/fixtures/system_full.toml` is a byte-for-byte copy of
//! the §6.4 example block. This test asserts:
//!
//! 1. The fixture text loaded from disk matches the documented values.
//! 2. Round-tripping through serialize → re-parse yields a structurally
//!    equivalent `SystemConfig` (TOML key ordering may be re-emitted by the
//!    `toml` crate, so we compare by re-parse rather than byte equality).

use std::path::PathBuf;

use cube_config::{SystemConfig, load_system};

fn fixture_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("system_full.toml");
    p
}

#[test]
fn sds_6_4_fixture_matches_normative_values_sds_6_4() {
    let (cfg, _report) = load_system(&fixture_path()).expect("verbatim §6.4 example must parse");

    // Display — every literal in the §6.4 example.
    assert_eq!(cfg.display.drm_driver, "cube-fpga-rpispi");
    assert_eq!(cfg.display.connector, "Cube-1");
    assert_eq!(cfg.display.mode, "384x64@60");
    assert_eq!(cfg.display.refresh_hz, 60);
    assert_eq!(cfg.display.spi_clock_hz, 35_000_000);

    // Network.
    assert!(cfg.network.tcp_control_enabled);
    assert_eq!(cfg.network.tcp_control_bind, "127.0.0.1");
    assert_eq!(cfg.network.tcp_control_port, 2018);
    assert!(cfg.network.remote_render_enabled);
    assert_eq!(cfg.network.remote_render_bind, "127.0.0.1");
    assert_eq!(cfg.network.remote_render_port, 2017);

    // Input.
    assert_eq!(cfg.input.system_controller_name_pattern, "8BitDo*");
    assert_eq!(cfg.input.key_back, "BTN_SELECT");
    assert_eq!(cfg.input.key_home, "BTN_START");
    assert_eq!(cfg.input.key_power, "BTN_MODE");

    // IMU.
    assert!((cfg.imu.xy_rotation_deg - 0.0).abs() < f64::EPSILON);
    assert!((cfg.imu.xz_rotation_deg - 45.0).abs() < f64::EPSILON);
    assert!((cfg.imu.yz_rotation_deg - 0.0).abs() < f64::EPSILON);

    // Transitions.
    assert_eq!(cfg.transitions.mode, "hard_cut");

    // Power.
    assert_eq!(cfg.power.idle_blank_after_sec, 0);
}

#[test]
fn sds_6_4_fixture_round_trips_through_serialization_sds_6_4() {
    // Parse → serialize → re-parse → expect structural equivalence.
    let (cfg, _) = load_system(&fixture_path()).unwrap();

    let serialized = toml::to_string(&cfg).expect("SystemConfig must be Serialize");

    // Write into a tempfile and re-load through the public API so we exercise
    // the same code path on the round-trip side.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("roundtrip.toml");
    std::fs::write(&p, &serialized).unwrap();
    let (cfg2, _) = load_system(&p).expect("re-serialized SystemConfig must re-parse");

    assert_eq_systemconfig(&cfg, &cfg2);
}

fn assert_eq_systemconfig(a: &SystemConfig, b: &SystemConfig) {
    assert_eq!(a.display.drm_driver, b.display.drm_driver);
    assert_eq!(a.display.connector, b.display.connector);
    assert_eq!(a.display.mode, b.display.mode);
    assert_eq!(a.display.refresh_hz, b.display.refresh_hz);
    assert_eq!(a.display.spi_clock_hz, b.display.spi_clock_hz);

    assert_eq!(a.network.tcp_control_enabled, b.network.tcp_control_enabled);
    assert_eq!(a.network.tcp_control_bind, b.network.tcp_control_bind);
    assert_eq!(a.network.tcp_control_port, b.network.tcp_control_port);
    assert_eq!(
        a.network.remote_render_enabled,
        b.network.remote_render_enabled
    );
    assert_eq!(a.network.remote_render_bind, b.network.remote_render_bind);
    assert_eq!(a.network.remote_render_port, b.network.remote_render_port);

    assert_eq!(
        a.input.system_controller_name_pattern,
        b.input.system_controller_name_pattern
    );
    assert_eq!(a.input.key_back, b.input.key_back);
    assert_eq!(a.input.key_home, b.input.key_home);
    assert_eq!(a.input.key_power, b.input.key_power);

    assert!((a.imu.xy_rotation_deg - b.imu.xy_rotation_deg).abs() < f64::EPSILON);
    assert!((a.imu.xz_rotation_deg - b.imu.xz_rotation_deg).abs() < f64::EPSILON);
    assert!((a.imu.yz_rotation_deg - b.imu.yz_rotation_deg).abs() < f64::EPSILON);

    assert_eq!(a.transitions.mode, b.transitions.mode);
    assert_eq!(a.power.idle_blank_after_sec, b.power.idle_blank_after_sec);
}
