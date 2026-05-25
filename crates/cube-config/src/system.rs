//! Loader for `/etc/cube/system.toml` (SDS §6.4) and SIGHUP-safe swap
//! handle (ARCH §4.11).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur when loading a configuration file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The configuration file does not exist.
    #[error("configuration file not found: {0}")]
    Missing(PathBuf),

    /// The file exists but could not be parsed (malformed TOML or missing
    /// required fields).
    #[error("failed to parse {path}: {message}")]
    Parse {
        path: PathBuf,
        message: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation report
// ─────────────────────────────────────────────────────────────────────────────

/// A single validation warning emitted alongside a successfully parsed config.
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Stable, machine-readable warning code (e.g. `NETWORK_PUBLIC_BIND`).
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Validation report returned alongside a parsed `SystemConfig`.
///
/// Warnings mean "parsed successfully but something looks suspicious".
/// Errors in this struct are *soft* validation errors that still allow the
/// config to be used (hard errors return `Err(ConfigError::...)`).
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationWarning>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SystemConfig and sub-structs (SDS §6.4)
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level system configuration parsed from `/etc/cube/system.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub display: DisplayConfig,
    pub remote_render: RemoteRenderConfig,
    pub input: InputConfig,
    pub imu: ImuConfig,
    pub transitions: TransitionsConfig,
    pub power: PowerConfig,
}

/// `[display]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub drm_driver: String,
    pub connector: String,
    pub mode: String,
    pub refresh_hz: u32,
    pub spi_clock_hz: u32,
}

/// `[remote_render]` section (SDS v5 §6.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRenderConfig {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
    pub mtu_hint: String,
}

/// `[input]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub system_controller_name_pattern: String,
    pub key_back: String,
    pub key_home: String,
    pub key_power: String,
}

/// `[imu]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuConfig {
    pub xy_rotation_deg: f64,
    pub xz_rotation_deg: f64,
    pub yz_rotation_deg: f64,
}

/// `[transitions]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionsConfig {
    pub mode: String,
}

/// `[power]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Seconds of inactivity before blanking. `0` means disabled (SDS §5.12).
    pub idle_blank_after_sec: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Loader
// ─────────────────────────────────────────────────────────────────────────────

/// Parse `/etc/cube/system.toml` (or any path) into a `SystemConfig`.
///
/// Returns `(config, report)` on success. `report.warnings` may be non-empty
/// even on success (e.g. `NETWORK_PUBLIC_BIND`).
pub fn load_system(path: &Path) -> Result<(SystemConfig, ValidationReport), ConfigError> {
    if !path.exists() {
        return Err(ConfigError::Missing(path.to_owned()));
    }

    let raw = std::fs::read_to_string(path).map_err(|e| ConfigError::Parse {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let cfg: SystemConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let report = validate_system(&cfg);
    Ok((cfg, report))
}

/// Emit validation warnings for the parsed `SystemConfig`.
fn validate_system(cfg: &SystemConfig) -> ValidationReport {
    let mut report = ValidationReport::default();

    // SDS §6.4: warn when bind is not a loopback address (i.e. exposed on the LAN).
    let loopback_prefixes = ["127.", "::1"];
    let is_loopback = |addr: &str| loopback_prefixes.iter().any(|p| addr.starts_with(p));

    if cfg.remote_render.enabled && !is_loopback(&cfg.remote_render.bind) {
        report.warnings.push(ValidationWarning {
            code: "NETWORK_PUBLIC_BIND".to_owned(),
            message: format!(
                "bind is set to {:?}, which is not a loopback address; \
                 the remote render stream will be reachable from the network",
                cfg.remote_render.bind
            ),
        });
    }

    report
}

// ─────────────────────────────────────────────────────────────────────────────
// SystemConfigHandle (ARCH §4.11)
// ─────────────────────────────────────────────────────────────────────────────

/// An `Arc<SystemConfig>` stored inside an `ArcSwap` so that it can be
/// atomically replaced on SIGHUP without blocking readers.
///
/// Callers snapshot the current config by calling `load()`; the returned
/// `Arc` remains valid even after a subsequent `swap()`.
pub struct SystemConfigHandle {
    inner: ArcSwap<SystemConfig>,
}

impl SystemConfigHandle {
    /// Create a new handle pre-loaded with the given config.
    pub fn new(cfg: SystemConfig) -> Self {
        Self {
            inner: ArcSwap::from_pointee(cfg),
        }
    }

    /// Snapshot the current config. Cheap — no lock, single atomic load.
    pub fn load(&self) -> Arc<SystemConfig> {
        self.inner.load_full()
    }

    /// Atomically replace the stored config (called on SIGHUP).
    pub fn swap(&self, cfg: SystemConfig) {
        self.inner.store(Arc::new(cfg));
    }
}
