//! Loader for `manifest.toml` (SDS §7.2).
//!
//! Validates the app `name` against the regex `^[a-zA-Z0-9._-]+$` and
//! additionally rejects the special values `.` and `..` (SDS §5.5).

use std::path::Path;

use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Deserializer, Serialize};
use toml::value::{Table, Value};

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
    Wellness,
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest types
// ─────────────────────────────────────────────────────────────────────────────

/// Deserialized `manifest.toml`.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub app: AppSection,
    pub requires: Option<RequiresSection>,
    pub power: Option<PowerSection>,
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
///
/// SDS v6 §7.2: `libcube` and `cubekit` are alternative SDK-compatibility
/// fields — an app declares **exactly one**, matching the SDK it links. This
/// is enforced by [`load_manifest`] only when a `[requires]` table is
/// present at all; a manifest with no `[requires]` table remains valid
/// (v5 compat).
#[derive(Debug, Clone)]
pub struct RequiresSection {
    pub libcube: Option<VersionReq>,
    pub cubekit: Option<VersionReq>,
    pub inputs: Vec<String>,
    pub sensors: Vec<String>,
    /// `true` if the app declares a need for outbound network access
    /// (SDS v6 §5.2, §7.2). Defaults to `false`.
    pub network: bool,
}

/// `[power]` section of a `manifest.toml` (SDS v6 §7.2).
///
/// Optional; a missing `[power]` table is equivalent to
/// `idle_blank = false`.
#[derive(Debug, Clone)]
pub struct PowerSection {
    /// When `true`, the system idle-blank timer also runs while this app is
    /// focused (SDS v6 §5.12).
    pub idle_blank: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw serde helpers (the TOML contains strings that need custom parsing)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawManifest {
    app: RawApp,
    requires: Option<RawRequires>,
    power: Option<RawPower>,
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
    #[serde(default, deserialize_with = "de_opt_version_req")]
    cubekit: Option<VersionReq>,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    sensors: Vec<String>,
    /// SDS v6 §7.2: optional, default `false`.
    #[serde(default)]
    network: bool,
}

#[derive(Deserialize)]
struct RawPower {
    /// SDS v6 §7.2: optional, default `false`.
    #[serde(default)]
    idle_blank: bool,
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
// Emission (the "agree by construction" counterpart of the loader; SDS §7.2,
// cubekit-spec §9.4)
// ─────────────────────────────────────────────────────────────────────────────

/// Serialize a [`Manifest`] to `manifest.toml` text (SDS §7.2).
///
/// The output is deterministic and key-sorted (a `toml::value::Table` is a
/// sorted map) and is accepted and round-tripped by [`load_manifest`]. This is
/// the emission half of "agree by construction" (cubekit-spec §9.4): the same
/// crate that loads `manifest.toml` also emits it, so emitter and loader cannot
/// drift. `Option` fields (`icon`, `requires`, `power`, and the SDK-compat
/// `libcube`/`cubekit` fields) are omitted entirely when `None`.
#[must_use]
pub fn manifest_to_toml(m: &Manifest) -> String {
    let mut root = Table::new();

    // [app]
    let mut app = Table::new();
    app.insert("name".into(), Value::String(m.app.name.clone()));
    app.insert(
        "display_name".into(),
        Value::String(m.app.display_name.clone()),
    );
    app.insert("version".into(), Value::String(m.app.version.to_string()));
    // `ManifestCategory` serializes (rename_all = "lowercase") to its token.
    app.insert(
        "category".into(),
        Value::try_from(m.app.category).expect("ManifestCategory serializes to a TOML string"),
    );
    if let Some(icon) = &m.app.icon {
        app.insert("icon".into(), Value::String(icon.clone()));
    }
    root.insert("app".into(), Value::Table(app));

    // [requires] — emitted only when present; libcube/cubekit each only if Some.
    if let Some(req) = &m.requires {
        let mut requires = Table::new();
        if let Some(libcube) = &req.libcube {
            requires.insert("libcube".into(), version_req_to_value(libcube));
        }
        if let Some(cubekit) = &req.cubekit {
            requires.insert("cubekit".into(), version_req_to_value(cubekit));
        }
        requires.insert(
            "inputs".into(),
            Value::Array(req.inputs.iter().map(|s| Value::String(s.clone())).collect()),
        );
        requires.insert(
            "sensors".into(),
            Value::Array(req.sensors.iter().map(|s| Value::String(s.clone())).collect()),
        );
        requires.insert("network".into(), Value::Boolean(req.network));
        root.insert("requires".into(), Value::Table(requires));
    }

    // [power]
    if let Some(power) = &m.power {
        let mut power_tbl = Table::new();
        power_tbl.insert("idle_blank".into(), Value::Boolean(power.idle_blank));
        root.insert("power".into(), Value::Table(power_tbl));
    }

    toml::to_string_pretty(&Value::Table(root)).expect("emitted manifest is a valid toml::Table")
}

/// Serialize a [`VersionReq`] to its canonical TOML string (e.g. `"^0.1"`).
fn version_req_to_value(req: &VersionReq) -> Value {
    Value::try_from(req).expect("VersionReq serializes to a TOML string")
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

    let requires = raw
        .requires
        .map(|r| {
            // SDS v6 §7.2: `libcube` and `cubekit` are alternative
            // SDK-compatibility fields — exactly one must be declared when
            // `[requires]` is present at all.
            match (&r.libcube, &r.cubekit) {
                (Some(_), Some(_)) => Err(parse_err(
                    "[requires]: declare exactly one of `libcube` or `cubekit`, not both"
                        .to_owned(),
                )),
                (None, None) => Err(parse_err(
                    "[requires]: must declare exactly one of `libcube` or `cubekit`".to_owned(),
                )),
                _ => Ok(RequiresSection {
                    libcube: r.libcube,
                    cubekit: r.cubekit,
                    inputs: r.inputs,
                    sensors: r.sensors,
                    network: r.network,
                }),
            }
        })
        .transpose()?;

    let power = raw.power.map(|p| PowerSection {
        idle_blank: p.idle_blank,
    });

    Ok(Manifest {
        app,
        requires,
        power,
    })
}

