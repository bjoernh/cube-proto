//! Tests for `cube_config::SystemConfigHandle` atomic swap.
//!
//!: "Reads `/etc/cube/system.toml` once at startup into an
//! `Arc<SystemConfig>`. On SIGHUP, re-reads and atomically swaps the Arc.
//! Modules that need config snapshot the Arc on demand." A reader that
//! already holds an Arc snapshot must continue to see the *old* values even
//! after a swap (snapshot-on-demand semantics), and concurrent reads from
//! multiple threads must never tear (no torn reads, no partially-published
//! state).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cube_config::{
    AppsConfig, DisplayConfig, ImuConfig, InputConfig, PowerConfig, RemoteRenderConfig,
    SystemConfig, SystemConfigHandle, TransitionsConfig,
};

fn make_cfg(refresh_hz: u32) -> SystemConfig {
    SystemConfig {
        display: DisplayConfig {
            sim_target: None,
            drm_driver: "vkms".to_owned(),
            connector: "Writeback-1".to_owned(),
            mode: "384x64@60".to_owned(),
            refresh_hz,
            spi_clock_hz: 35_000_000,
        },
        remote_render: RemoteRenderConfig {
            enabled: false,
            bind: "127.0.0.1".into(),
            port: 2017,
            mtu_hint: "jumbo_recommended".into(),
        },
        input: InputConfig {
            system_controller_name_pattern: Some("8BitDo*".to_owned()),
            key_back: "Select".to_owned(),
            key_home: "Start".to_owned(),
            key_power: "Guide".to_owned(),
            device_allow: Vec::new(),
            device_deny: Vec::new(),
            max_players: 8,
            profiles_dir: None,
        },
        imu: ImuConfig {
            xy_rotation_deg: 0.0,
            xz_rotation_deg: 0.0,
            yz_rotation_deg: 0.0,
        },
        transitions: TransitionsConfig {
            mode: "hard_cut".to_owned(),
        },
        power: PowerConfig {
            idle_blank_after_sec: 0,
        },
        apps: AppsConfig::default(),
        debug: cube_config::DebugConfig::default(),
    }
}

#[test]
fn config_handle_load_returns_initial_snapshot_arch_4_11() {
    let h = SystemConfigHandle::new(make_cfg(60));
    let snap: Arc<SystemConfig> = h.load();
    assert_eq!(snap.display.refresh_hz, 60);
}

#[test]
fn config_handle_swap_replaces_arc_arch_4_11() {
    let h = SystemConfigHandle::new(make_cfg(60));
    let before = h.load();
    assert_eq!(before.display.refresh_hz, 60);

    h.swap(make_cfg(120));
    let after = h.load();
    assert_eq!(after.display.refresh_hz, 120);
}

#[test]
fn config_handle_old_snapshot_survives_swap_arch_4_11() {
    // The snapshot-on-demand contract: a holder of an old Arc must continue
    // to see the values from the pre-swap snapshot until they re-load.
    let h = SystemConfigHandle::new(make_cfg(60));
    let pinned = h.load();
    h.swap(make_cfg(120));
    assert_eq!(
        pinned.display.refresh_hz, 60,
        "old Arc snapshot must outlive a swap"
    );
    let fresh = h.load();
    assert_eq!(fresh.display.refresh_hz, 120);
}

#[test]
fn config_handle_swap_atomic_arch_4_11() {
    // Multiple reader threads race with a single writer that flips between
    // two configs in a hot loop. Every observed snapshot must be one of the
    // two well-formed configs — never a torn read.
    let handle = SystemConfigHandle::new(make_cfg(60));
    let stop = AtomicBool::new(false);

    std::thread::scope(|s| {
        let handle = &handle;
        let stop = &stop;

        // 16 reader threads.
        let mut readers = Vec::new();
        for _ in 0..16 {
            readers.push(s.spawn(move || {
                let mut seen_60 = 0u64;
                let mut seen_120 = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    let snap = handle.load();
                    match snap.display.refresh_hz {
                        60 => seen_60 += 1,
                        120 => seen_120 += 1,
                        other => panic!("torn read: observed refresh_hz = {other}"),
                    }
                    // Hammer a second field too — torn reads would scramble
                    // values across structs.
                    assert!(
                        snap.remote_render.bind == "127.0.0.1"
                            || snap.remote_render.bind == "0.0.0.0",
                        "torn read on remote_render.bind: {:?}",
                        snap.remote_render.bind,
                    );
                }
                (seen_60, seen_120)
            }));
        }

        // Single writer.
        let writer = s.spawn(move || {
            for i in 0..2_000 {
                let mut cfg = if i % 2 == 0 {
                    make_cfg(60)
                } else {
                    make_cfg(120)
                };
                if i % 4 == 0 {
                    cfg.remote_render.bind = "0.0.0.0".to_owned();
                }
                handle.swap(cfg);
            }
            stop.store(true, Ordering::Relaxed);
        });

        writer.join().unwrap();
        for r in readers {
            let _ = r.join().unwrap();
        }
    });
}
