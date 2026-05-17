//! Validation-warning tests for `SystemConfig` (SDS §6.4).
//!
//! SDS §6.4 says: "the default `remote_render_bind = "127.0.0.1"` means the
//! listener is reachable only from loopback. … To expose remote rendering on
//! the LAN both settings must be changed **and a WARN is logged.**". The same
//! applies to `tcp_control_bind`. The warning surfaces in the
//! `ValidationReport.warnings` slot with a stable, machine-readable code
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

fn cfg_with(network_overrides: &str) -> std::path::PathBuf {
    let body = format!(
        "\
[display]
drm_driver = \"vkms\"
connector = \"Writeback-1\"
mode = \"384x64@60\"
refresh_hz = 60
spi_clock_hz = 35000000

[network]
{network_overrides}

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
"
    );
    write(&body)
}

#[test]
fn system_warning_absent_on_loopback_defaults_sds_6_4() {
    let p = cfg_with(
        "\
tcp_control_enabled = true
tcp_control_bind = \"127.0.0.1\"
tcp_control_port = 2018
remote_render_enabled = true
remote_render_bind = \"127.0.0.1\"
remote_render_port = 2017",
    );
    let (_, report) = load_system(&p).unwrap();
    assert!(
        report.warnings.iter().all(|w| w.code != "NETWORK_PUBLIC_BIND"),
        "loopback-only binds must not trigger NETWORK_PUBLIC_BIND, got {:?}",
        report.warnings,
    );
}

#[test]
fn system_warning_on_public_tcp_control_bind_sds_6_4() {
    let p = cfg_with(
        "\
tcp_control_enabled = true
tcp_control_bind = \"0.0.0.0\"
tcp_control_port = 2018
remote_render_enabled = true
remote_render_bind = \"127.0.0.1\"
remote_render_port = 2017",
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
        msg.contains("tcp_control_bind"),
        "warning message should mention the field name, got {msg:?}"
    );
}

#[test]
fn system_warning_on_public_remote_render_bind_sds_6_4() {
    let p = cfg_with(
        "\
tcp_control_enabled = true
tcp_control_bind = \"127.0.0.1\"
tcp_control_port = 2018
remote_render_enabled = true
remote_render_bind = \"0.0.0.0\"
remote_render_port = 2017",
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
        msg.contains("remote_render_bind"),
        "warning message should mention the field name, got {msg:?}"
    );
}

#[test]
fn system_warning_on_both_public_binds_sds_6_4() {
    let p = cfg_with(
        "\
tcp_control_enabled = true
tcp_control_bind = \"0.0.0.0\"
tcp_control_port = 2018
remote_render_enabled = true
remote_render_bind = \"0.0.0.0\"
remote_render_port = 2017",
    );
    let (_, report) = load_system(&p).unwrap();
    let hits: Vec<_> = report
        .warnings
        .iter()
        .filter(|w| w.code == "NETWORK_PUBLIC_BIND")
        .collect();
    assert_eq!(
        hits.len(),
        2,
        "expected two NETWORK_PUBLIC_BIND warnings, got {:?}",
        report.warnings,
    );
}
