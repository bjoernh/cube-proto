//! Validation-warning tests for `SystemConfig` (SDS §6.4).
//!
//! SDS §6.4 says: "the default `bind = "127.0.0.1"` means the listener is
//! reachable only from loopback. … To expose remote rendering on the LAN the
//! setting must be changed **and a WARN is logged.**". The warning surfaces in
//! the `ValidationReport.warnings` slot with a stable, machine-readable code
//! `NETWORK_PUBLIC_BIND`.

use cube_config::load_system;

fn write(toml: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    // intentionally keep the dir alive for the test duration via Box::leak
    let dir = Box::leak(Box::new(dir));
    let p = dir.path().join("system.toml");
    std::fs::write(&p, toml).unwrap();
    p
}

fn cfg_with(remote_render_overrides: &str) -> std::path::PathBuf {
    let body = format!(
        "\
[display]
drm_driver = \"vkms\"
connector = \"Writeback-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[remote_render]
{remote_render_overrides}

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
"
    );
    write(&body)
}

#[test]
fn system_warning_absent_on_loopback_defaults_sds_6_4() {
    let p = cfg_with(
        "\
enabled = true
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"",
    );
    let (_, report) = load_system(&p).unwrap();
    assert!(
        report.warnings.iter().all(|w| w.code != "NETWORK_PUBLIC_BIND"),
        "loopback-only binds must not trigger NETWORK_PUBLIC_BIND, got {:?}",
        report.warnings,
    );
}

#[test]
fn system_warning_on_public_remote_render_bind_sds_6_4() {
    let p = cfg_with(
        "\
enabled = true
bind = \"0.0.0.0\"
port = 2017
mtu_hint = \"jumbo_recommended\"",
    );
    let (_, report) = load_system(&p).unwrap();
    let hits: Vec<_> = report
        .warnings
        .iter()
        .filter(|w| w.code == "NETWORK_PUBLIC_BIND")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one NETWORK_PUBLIC_BIND warning, got {:?}",
        report.warnings,
    );
    let msg = &hits[0].message;
    assert!(
        msg.contains("bind"),
        "warning message should mention the field name, got {msg:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// cube-gamepad Phase 3 (M8): deprecated `system_controller_name_pattern` alias
// ─────────────────────────────────────────────────────────────────────────────

/// Phase 3 (M8) — RED. A legacy `system_controller_name_pattern` is a
/// **deprecated alias**: on load it must (1) seed `device_allow` so capability
/// discovery still honours the old single-controller filter, and (2) surface a
/// one-time deprecation warning under the stable code
/// `INPUT_DEPRECATED_NAME_PATTERN`.
///
/// `cfg_with` already emits `system_controller_name_pattern = "8BitDo*"` in its
/// `[input]` block, so this exercises the alias path. Red until Phase 3 GREEN
/// implements the fold in `load_system` / `validate_system`.
#[test]
fn legacy_name_pattern_seeds_device_allow_and_warns_phase3() {
    let p = cfg_with(
        "\
enabled = false
bind = \"127.0.0.1\"
port = 2017
mtu_hint = \"jumbo_recommended\"",
    );
    let (cfg, report) = load_system(&p).expect("legacy [input] block must still parse");

    // (1) The deprecated pattern seeds the capability-discovery allowlist.
    assert!(
        cfg.input.device_allow.iter().any(|e| e == "8BitDo*"),
        "legacy system_controller_name_pattern must seed device_allow, got {:?}",
        cfg.input.device_allow,
    );

    // (2) …and is reported exactly once for one-time deprecation logging.
    let hits: Vec<_> = report
        .warnings
        .iter()
        .filter(|w| w.code == "INPUT_DEPRECATED_NAME_PATTERN")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected one INPUT_DEPRECATED_NAME_PATTERN warning, got {:?}",
        report.warnings,
    );
}
