//! Loader for `manifest.toml` (SDS §7.2).
//!
//! Validates the app `name` against the regex `^[a-zA-Z0-9._-]+$` and
//! additionally rejects the special values `.` and `..` (SDS §5.5).

use std::path::Path;

use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Deserializer, Serialize};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, value};

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
// Accent palette (SDS v7.1 §A2)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed, core-owned canonical accent palette (SDS v7.1 §A2).
///
/// These are palette-neutral colour tokens the core owns, deliberately
/// decoupled from the companion's internal theme naming. The set is closed:
/// an unknown token fails deserialization so `cubectl doctor` surfaces it
/// (SDS §11.1). New tokens may be appended without a schema version bump; the
/// companion degrades any token it does not theme to a fallback accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    Red,
    Amber,
    Yellow,
    Green,
    Cyan,
    Blue,
    Violet,
    Magenta,
}

// ─────────────────────────────────────────────────────────────────────────────
// Player count (SDS v7.1 §A1/§A4)
// ─────────────────────────────────────────────────────────────────────────────

/// Structured player count for an app (SDS v7.1 §A1).
///
/// On the wire and in TOML this is an inline table `{ min = N, max = M }`.
/// The loader validates `min >= 1` and `min <= max`; it is carried structured
/// all the way to the `list` wire (§A4), never a pre-formatted display string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Players {
    pub min: u32,
    pub max: u32,
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
    pub overlay: Option<OverlaySection>,
}

/// `[app]` section of a `manifest.toml`.
#[derive(Debug, Clone)]
pub struct AppSection {
    pub name: String,
    pub display_name: String,
    pub version: Version,
    pub category: ManifestCategory,
    pub icon: Option<String>,
    /// Optional accent token from the closed core palette (SDS v7.1 §A1/§A2).
    pub accent: Option<Accent>,
    /// Optional short blurb (companion Arcade sub-line) (SDS v7.1 §A1).
    pub description: Option<String>,
    /// Optional preview image/gif filename, resolved flat in the app's install
    /// directory alongside `manifest.toml` (SDS v7.1 §A1/§A3).
    pub preview: Option<String>,
    /// Optional structured player count; validated `min >= 1, min <= max`
    /// (SDS v7.1 §A1). Omitted for non-player apps (visualizers/utilities).
    pub players: Option<Players>,
}

/// `[requires]` section of a `manifest.toml`.
///
/// SDS v6 §7.2: `libcube`, `cubekit` and `cubego` are alternative
/// SDK-compatibility fields — an app declares **exactly one**, matching the
/// SDK it links (C++, Rust and Go respectively). This is enforced by
/// [`load_manifest`] only when a `[requires]` table is present at all; a
/// manifest with no `[requires]` table remains valid (v5 compat).
#[derive(Debug, Clone)]
pub struct RequiresSection {
    pub libcube: Option<VersionReq>,
    pub cubekit: Option<VersionReq>,
    /// Go SDK compatibility requirement (cube-system#7).
    pub cubego: Option<VersionReq>,
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

/// `[overlay]` section of a `manifest.toml` (SDS v7 §5.13, §6.1).
///
/// Optional; a missing `[overlay]` table is equivalent to `provides = false`.
#[derive(Debug, Clone)]
pub struct OverlaySection {
    /// When `true`, this app may `overlay.acquire` a client overlay over
    /// **itself** (SDS v7 §5.13). Defaults to `false`.
    pub provides: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw serde helpers (the TOML contains strings that need custom parsing)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawManifest {
    app: RawApp,
    requires: Option<RawRequires>,
    power: Option<RawPower>,
    overlay: Option<RawOverlay>,
}

#[derive(Deserialize)]
struct RawApp {
    name: String,
    display_name: String,
    version: String,
    category: ManifestCategory,
    #[serde(default)]
    icon: Option<String>,
    /// SDS v7.1 §A1/§A2: optional; unknown token fails deserialization (closed
    /// set), surfaced as a `ConfigError::Parse` by the loader.
    #[serde(default)]
    accent: Option<Accent>,
    /// SDS v7.1 §A1: optional short blurb.
    #[serde(default)]
    description: Option<String>,
    /// SDS v7.1 §A1/§A3: optional preview filename.
    #[serde(default)]
    preview: Option<String>,
    /// SDS v7.1 §A1: optional inline `{ min, max }`; validated by the loader.
    #[serde(default)]
    players: Option<Players>,
}

#[derive(Deserialize)]
struct RawRequires {
    #[serde(default, deserialize_with = "de_opt_version_req")]
    libcube: Option<VersionReq>,
    #[serde(default, deserialize_with = "de_opt_version_req")]
    cubekit: Option<VersionReq>,
    #[serde(default, deserialize_with = "de_opt_version_req")]
    cubego: Option<VersionReq>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverlay {
    /// SDS v7 §5.13: optional, default `false`. A non-bool value (e.g.
    /// `provides = "yes"`) is rejected by the loader (the bug `cubectl doctor`
    /// must surface — SDS §11.1), as is any unknown key under `[overlay]`.
    #[serde(default)]
    provides: bool,
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

/// Serialize a [`Manifest`] to `manifest.toml` text (SDS §7.2, §7.1 §A1).
///
/// The output is deterministic and key-sorted (each table's values are sorted
/// by key) and is accepted and round-tripped by [`load_manifest`]. This is the
/// emission half of "agree by construction" (cubekit-spec §9.4): the same crate
/// that loads `manifest.toml` also emits it, so emitter and loader cannot drift.
/// `Option` fields (`icon`, the four §A1 metadata fields `accent`/`description`/
/// `preview`/`players`, `requires`, `power`, and the SDK-compat
/// `libcube`/`cubekit`/
/// `cubego` fields) are omitted entirely when `None`. `players` is emitted as
/// an inline table `{ min = N, max = M }` (SDS v7.1 §A1).
#[must_use]
pub fn manifest_to_toml(m: &Manifest) -> String {
    let mut doc = DocumentMut::new();

    // [app]
    let mut app = Table::new();
    app.insert("name", value(m.app.name.clone()));
    app.insert("display_name", value(m.app.display_name.clone()));
    app.insert("version", value(m.app.version.to_string()));
    // `ManifestCategory` serializes (rename_all = "lowercase") to its token.
    app.insert("category", value(lowercase_token(m.app.category)));
    if let Some(icon) = &m.app.icon {
        app.insert("icon", value(icon.clone()));
    }
    // §A1 metadata fields — emitted only when `Some`.
    if let Some(accent) = &m.app.accent {
        app.insert("accent", value(lowercase_token(*accent)));
    }
    if let Some(description) = &m.app.description {
        app.insert("description", value(description.clone()));
    }
    if let Some(preview) = &m.app.preview {
        app.insert("preview", value(preview.clone()));
    }
    if let Some(players) = &m.app.players {
        let mut inline = InlineTable::new();
        inline.insert("min", i64::from(players.min).into());
        inline.insert("max", i64::from(players.max).into());
        app.insert("players", value(inline));
    }
    app.sort_values();
    doc.insert("app", Item::Table(app));

    // [requires] — emitted only when present; libcube/cubekit/cubego each only
    // if Some.
    if let Some(req) = &m.requires {
        let mut requires = Table::new();
        if let Some(libcube) = &req.libcube {
            requires.insert("libcube", value(libcube.to_string()));
        }
        if let Some(cubekit) = &req.cubekit {
            requires.insert("cubekit", value(cubekit.to_string()));
        }
        if let Some(cubego) = &req.cubego {
            requires.insert("cubego", value(cubego.to_string()));
        }
        let mut inputs = Array::new();
        for s in &req.inputs {
            inputs.push(s.as_str());
        }
        requires.insert("inputs", value(inputs));
        let mut sensors = Array::new();
        for s in &req.sensors {
            sensors.push(s.as_str());
        }
        requires.insert("sensors", value(sensors));
        requires.insert("network", value(req.network));
        requires.sort_values();
        doc.insert("requires", Item::Table(requires));
    }

    // [power]
    if let Some(power) = &m.power {
        let mut power_tbl = Table::new();
        power_tbl.insert("idle_blank", value(power.idle_blank));
        doc.insert("power", Item::Table(power_tbl));
    }

    // [overlay]
    if let Some(overlay) = &m.overlay {
        let mut overlay_tbl = Table::new();
        overlay_tbl.insert("provides", value(overlay.provides));
        doc.insert("overlay", Item::Table(overlay_tbl));
    }

    doc.as_table_mut().sort_values();
    doc.to_string()
}

/// Lowercase wire token for a `rename_all = "lowercase"` serde enum
/// (`ManifestCategory`, `Accent`), via the serde impl so the emitter and the
/// closed loader enum cannot spell a token differently.
fn lowercase_token<T: Serialize>(v: T) -> String {
    match toml::Value::try_from(v) {
        Ok(toml::Value::String(s)) => s,
        other => unreachable!("lowercase enum must serialize to a TOML string, got {other:?}"),
    }
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

    // SDS v7.1 §A1: validate `players` when present (min >= 1, min <= max).
    // (An invalid `accent` token is already rejected earlier by the closed
    // enum during `toml::from_str`, surfaced as `ConfigError::Parse`.)
    if let Some(players) = raw.app.players {
        if players.min < 1 {
            return Err(parse_err(format!(
                "[app] players: min must be >= 1, got min = {}",
                players.min
            )));
        }
        if players.min > players.max {
            return Err(parse_err(format!(
                "[app] players: min must be <= max, got min = {}, max = {}",
                players.min, players.max
            )));
        }
    }

    let app = AppSection {
        name: raw.app.name,
        display_name: raw.app.display_name,
        version,
        category: raw.app.category,
        icon: raw.app.icon,
        accent: raw.app.accent,
        description: raw.app.description,
        preview: raw.app.preview,
        players: raw.app.players,
    };

    let requires = raw
        .requires
        .map(|r| {
            // SDS v6 §7.2: `libcube`, `cubekit` and `cubego` are alternative
            // SDK-compatibility fields — exactly one must be declared when
            // `[requires]` is present at all.
            let declared = usize::from(r.libcube.is_some())
                + usize::from(r.cubekit.is_some())
                + usize::from(r.cubego.is_some());
            match declared {
                0 => Err(parse_err(
                    "[requires]: must declare exactly one of `libcube`, `cubekit` or `cubego`"
                        .to_owned(),
                )),
                1 => Ok(RequiresSection {
                    libcube: r.libcube,
                    cubekit: r.cubekit,
                    cubego: r.cubego,
                    inputs: r.inputs,
                    sensors: r.sensors,
                    network: r.network,
                }),
                _ => Err(parse_err(
                    "[requires]: declare exactly one of `libcube`, `cubekit` or `cubego`, not more"
                        .to_owned(),
                )),
            }
        })
        .transpose()?;

    let power = raw.power.map(|p| PowerSection {
        idle_blank: p.idle_blank,
    });

    let overlay = raw.overlay.map(|o| OverlaySection {
        provides: o.provides,
    });

    Ok(Manifest {
        app,
        requires,
        power,
        overlay,
    })
}
