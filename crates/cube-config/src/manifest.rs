//! Loader for `manifest.toml` (SDS §7.2).
//!
//! Validates the app `name` against the regex `^[a-zA-Z0-9._-]+$` and
//! additionally rejects the special values `.` and `..` (SDS §5.5).

use std::path::Path;

use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Deserializer, Serialize};

use crate::system::ConfigError;

// ─────────────────────────────────────────────────────────────────────────────
// Category enum
// ─────────────────────────────────────────────────────────────────────────────

/// Closed set of manifest categories (SDS §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManifestCategory {
    Game,
    Visualizer,
    Utility,
    Demo,
    System,
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest types
// ─────────────────────────────────────────────────────────────────────────────

/// Deserialized `manifest.toml`.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub app: AppSection,
    pub requires: Option<RequiresSection>,
}

/// `[app]` section of a `manifest.toml`.
#[derive(Debug, Clone)]
pub struct AppSection {
    pub name: String,
    pub display_name: String,
    pub version: Version,
    pub category: ManifestCategory,
    pub icon: Option<String>,
}

/// `[requires]` section of a `manifest.toml`.
#[derive(Debug, Clone)]
pub struct RequiresSection {
    pub libcube: Option<VersionReq>,
    pub inputs: Vec<String>,
    pub sensors: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw serde helpers (the TOML contains strings that need custom parsing)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawManifest {
    app: RawApp,
    requires: Option<RawRequires>,
}

#[derive(Deserialize)]
struct RawApp {
    name: String,
    display_name: String,
    version: String,
    category: ManifestCategory,
    icon: Option<String>,
}

#[derive(Deserialize)]
struct RawRequires {
    #[serde(default, deserialize_with = "de_opt_version_req")]
    libcube: Option<VersionReq>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    sensors: Vec<String>,
}

fn de_opt_version_req<'de, D>(d: D) -> Result<Option<VersionReq>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(ref v) => VersionReq::parse(v)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Name validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validate an app name per SDS §5.5.
///
/// Returns `Err` with a descriptive message if the name is invalid.
pub(crate) fn validate_app_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("app name must not be empty".to_owned());
    }
    if name == "." || name == ".." {
        return Err(format!(
            "invalid app name {name:?}: name must not be '.' or '..'"
        ));
    }
    // SDS §5.5 / same regex reused for preset names.
    let re = Regex::new(r"^[a-zA-Z0-9._-]+$").expect("valid regex");
    if !re.is_match(name) {
        return Err(format!(
            "invalid app name {name:?}: must match ^[a-zA-Z0-9._-]+$"
        ));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Loader
// ─────────────────────────────────────────────────────────────────────────────

/// Parse `manifest.toml` at the given path.
pub fn load_manifest(path: &Path) -> Result<Manifest, ConfigError> {
    load_manifest_inner(path)
}

fn load_manifest_inner(path: &Path) -> Result<Manifest, ConfigError> {
    let parse_err = |msg: String| ConfigError::Parse {
        path: path.to_owned(),
        message: msg,
    };

    if !path.exists() {
        return Err(ConfigError::Missing(path.to_owned()));
    }

    let raw_str = std::fs::read_to_string(path).map_err(|e| parse_err(e.to_string()))?;
    let raw: RawManifest = toml::from_str(&raw_str).map_err(|e| parse_err(e.to_string()))?;

    // Validate the app name.
    validate_app_name(&raw.app.name).map_err(|e| parse_err(format!("name: {e}")))?;

    // Parse semver version.
    let version = Version::parse(&raw.app.version).map_err(|e| {
        parse_err(format!(
            "version {:?} is not valid semver: {e}",
            raw.app.version
        ))
    })?;

    let app = AppSection {
        name: raw.app.name,
        display_name: raw.app.display_name,
        version,
        category: raw.app.category,
        icon: raw.app.icon,
    };

    let requires = raw.requires.map(|r| RequiresSection {
        libcube: r.libcube,
        inputs: r.inputs,
        sensors: r.sensors,
    });

    Ok(Manifest { app, requires })
}

