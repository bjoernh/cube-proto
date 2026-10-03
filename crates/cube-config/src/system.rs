//! Loader for `/etc/cube/system.toml` and SIGHUP-safe swap
//! handle.

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
    Parse { path: PathBuf, message: String },
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
// SystemConfig and sub-structs
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
    /// `[apps]` section. Optional; a missing table parses
    /// as the default (`max_resident = 1`).
    #[serde(default)]
    pub apps: AppsConfig,
    /// `[debug]` section. Optional — omitting it disables all debug taps.
    #[serde(default)]
    pub debug: DebugConfig,
}

/// `[display]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub drm_driver: String,
    pub connector: String,
    pub mode: String,
    pub refresh_hz: u32,
    pub spi_clock_hz: u32,
    /// cube-sim viewer address (`host:port`) for `--drm-backend sim`
    /// (cube-sim design). Optional so existing system.toml files parse;
    /// the `CUBED_SIM_TARGET` env var overrides, and the backend falls back
    /// to `127.0.0.1:2323` when neither is set.
    #[serde(default)]
    pub sim_target: Option<String>,
}

/// `[remote_render]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRenderConfig {
    pub enabled: bool,
    pub bind: String,
    pub port: u16,
    pub mtu_hint: String,
}

/// `[input]` section.
///
/// Reserved-key values (`key_back`/`key_home`/`key_power`) are now **canonical
/// button names** (`Select`/`Start`/`Guide`); the
/// legacy evdev spellings (`BTN_SELECT`/`BTN_START`/`BTN_MODE`) are still
/// accepted by the classifier for one release with a deprecation warning (M7).
///
/// Discovery is **capability-based**: any
/// standard gamepad is adopted, filtered only by the `device_allow`/
/// `device_deny` policy. The legacy `system_controller_name_pattern` glob is
/// demoted to an optional deprecated alias (`Option`, `#[serde(default)]`) so
/// existing `system.toml` files still parse; the policy fields all carry serde
/// defaults so a config that omits them is valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    /// **Deprecated** legacy single-controller name glob.
    /// Retained as an optional alias for one release: when
    /// `Some`, it seeds [`device_allow`](Self::device_allow) and a one-time
    /// deprecation warning is emitted. New configs use the capability-based
    /// `device_allow`/`device_deny` policy instead and omit this key (then it
    /// parses as `None`).
    #[serde(default)]
    pub system_controller_name_pattern: Option<String>,
    /// Reserved "back" key in canonical button terms (`Select`); Player 1 only.
    pub key_back: String,
    /// Reserved "home" key in canonical button terms (`Start`); Player 1 only.
    pub key_home: String,
    /// Reserved "power" key in canonical button terms (`Guide`); Player 1 only.
    pub key_power: String,
    /// Capability-discovery **allowlist**.
    /// Each entry is either a `VID:PID` string (e.g. `"2dc8:9018"`) matched
    /// against the device's vendor:product, or a name glob matched against the
    /// evdev device name. **Empty = allow any gamepad.** The deprecated
    /// `system_controller_name_pattern`, when present, is folded in here.
    #[serde(default)]
    pub device_allow: Vec<String>,
    /// Capability-discovery **denylist** (same entry forms as
    /// [`device_allow`](Self::device_allow)). A deny match **wins over** an
    /// allow match.
    #[serde(default)]
    pub device_deny: Vec<String>,
    /// Maximum number of player slots. Sizes
    /// the Phase-4 per-player arrays; stored now, consumed there. Defaults to
    /// [`InputConfig::DEFAULT_MAX_PLAYERS`].
    #[serde(default = "default_max_players")]
    pub max_players: u8,
    /// Local override directory for gamepad-profile TOMLs.
    /// Shipped read-only profiles live in
    /// `/usr/share/cube/gamepad-profiles`; this is the local overlay that wins
    /// on conflict. Absent → [`InputConfig::DEFAULT_PROFILES_DIR`].
    #[serde(default)]
    pub profiles_dir: Option<String>,
}

/// serde default for [`InputConfig::max_players`].
fn default_max_players() -> u8 {
    InputConfig::DEFAULT_MAX_PLAYERS
}

impl InputConfig {
    /// The standard local profiles directory used when `profiles_dir` is unset.
    pub const DEFAULT_PROFILES_DIR: &'static str = "/etc/cube/gamepad-profiles";

    /// Default maximum number of player slots when `max_players` is absent.
    pub const DEFAULT_MAX_PLAYERS: u8 = 8;

    /// The configured local profiles directory, or the standard default when
    /// `profiles_dir` is absent from `system.toml`.
    #[must_use]
    pub fn resolved_profiles_dir(&self) -> &str {
        self.profiles_dir
            .as_deref()
            .unwrap_or(Self::DEFAULT_PROFILES_DIR)
    }

    /// The effective capability-discovery allowlist: [`device_allow`] with the
    /// deprecated [`system_controller_name_pattern`] folded in as an extra
    /// name-glob entry.
    ///
    /// [`load_system`] seeds [`device_allow`] from the legacy pattern at parse
    /// time, but configs built in-process (e.g. tests) may carry the pattern
    /// without that fold. This method makes the discovery predicate robust to
    /// both: the fold is **idempotent** (a pattern already present in
    /// `device_allow` is not duplicated), so callers can apply it unconditionally.
    ///
    /// [`device_allow`]: Self::device_allow
    /// [`system_controller_name_pattern`]: Self::system_controller_name_pattern
    #[must_use]
    pub fn effective_device_allow(&self) -> Vec<String> {
        let mut allow = self.device_allow.clone();
        if let Some(pattern) = &self.system_controller_name_pattern
            && !allow.iter().any(|e| e == pattern)
        {
            allow.push(pattern.clone());
        }
        allow
    }
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
    /// Seconds of inactivity before blanking. `0` means disabled.
    pub idle_blank_after_sec: u32,
}

/// `[apps]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    /// The maximum number of concurrently resident (focused + paused)
    /// non-launcher app sessions. Default `1`, which reproduces v5's
    /// implicit single-foreground/eviction behaviour. Must be `>= 1`.
    pub max_resident: u32,
    /// Gates the remote `apt_install` verb (App Store remote install).
    /// Default `true`.
    pub allow_remote_install: bool,
    /// On-cube app-preview auto-hide timeout, in seconds. Default `15`.
    pub preview_timeout_secs: u32,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self {
            max_resident: 1,
            allow_remote_install: true,
            preview_timeout_secs: 15,
        }
    }
}

/// `[debug]` section — optional taps for diagnosing rendering behaviour.
///
/// Every field here is re-loadable on SIGHUP so a debug tap can
/// be turned on and off on a running daemon without a restart.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    /// When set, every presented frame's pixels (packed XRGB8888) are written
    /// to this path, overwriting it in place. This mirrors the
    /// `--capture-on-present <path>` CLI flag; the CLI flag, when given, takes
    /// precedence over this value. Unset (or removed on SIGHUP) disables the
    /// tap. Only the real DRM backend honours it (the mock backend ignores it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_on_present: Option<PathBuf>,
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

    let mut cfg: SystemConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    // `[apps] max_resident` must be at least 1 — every cube runs
    // at least one (foreground) app.
    if cfg.apps.max_resident == 0 {
        return Err(ConfigError::Parse {
            path: path.to_owned(),
            message: "[apps] max_resident must be at least 1 (got 0)".to_owned(),
        });
    }

    let mut report = validate_system(&cfg);

    // The legacy `system_controller_name_pattern` is a deprecated alias. When present it
    // seeds the capability-discovery `device_allow` list (so the old
    // single-controller filter still applies under capability-based discovery)
    // and surfaces a one-time deprecation warning.
    if cfg.input.system_controller_name_pattern.is_some() {
        cfg.input.device_allow = cfg.input.effective_device_allow();
        report.warnings.push(ValidationWarning {
            code: "INPUT_DEPRECATED_NAME_PATTERN".to_owned(),
            message: "`[input] system_controller_name_pattern` is deprecated; \
                      it has been folded into `device_allow` for capability-based \
                      discovery. Replace it with `device_allow`/`device_deny` \
                      policy entries — the alias will be removed in a future release."
                .to_owned(),
        });
    }

    Ok((cfg, report))
}

/// Emit validation warnings for the parsed `SystemConfig`.
fn validate_system(cfg: &SystemConfig) -> ValidationReport {
    let mut report = ValidationReport::default();

    // Warn when bind is not a loopback address (i.e. exposed on the LAN).
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
// SystemConfigHandle
// ─────────────────────────────────────────────────────────────────────────────

/// An `Arc<SystemConfig>` stored inside an `ArcSwap` so that it can be
/// atomically replaced on SIGHUP without blocking readers.
///
/// Callers snapshot the current config by calling `load`; the returned
/// `Arc` remains valid even after a subsequent `swap`.
pub struct SystemConfigHandle {
    inner: ArcSwap<SystemConfig>,
}

impl SystemConfigHandle {
    /// Create a new handle pre-loaded with the given config.
    #[must_use]
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
