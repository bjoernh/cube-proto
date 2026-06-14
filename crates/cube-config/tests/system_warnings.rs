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
