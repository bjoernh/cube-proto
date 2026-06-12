//! Loader for `schema.toml` (SDS §5.4).
//!
//! Uses `IndexMap` to preserve the insertion order of parameters as they
//! appear in the TOML source. Also provides a `Schema::builder()` API used
//! by the cube-presets crate.

use std::path::Path;

use cube_proto::ParamValue;
use indexmap::IndexMap;
use serde::Deserialize;
use toml::value::{Table, Value};

use crate::system::ConfigError;

// ─────────────────────────────────────────────────────────────────────────────
// ParamType
// ─────────────────────────────────────────────────────────────────────────────

/// The type of a schema parameter (SDS §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    Bool,
    Int,
    Float,
    String,
    #[serde(rename = "enum")]
    Enum,
    Color,
    Vec2,
    Vec3,
}

// ─────────────────────────────────────────────────────────────────────────────
// UiMeta
// ─────────────────────────────────────────────────────────────────────────────

/// Optional UI metadata for a parameter (SDS §5.4).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UiMeta {
    pub group: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ParamDef
// ─────────────────────────────────────────────────────────────────────────────

/// Definition of a single parameter in the schema (SDS §5.4).
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub key: String,
    pub ty: ParamType,
    pub default: ParamValue,
    pub min: Option<ParamValue>,
    pub max: Option<ParamValue>,
    pub step: Option<ParamValue>,
    pub unit: Option<String>,
    pub required: bool,
    pub readonly: bool,
    pub shareable: bool,
    /// Only present when `ty == Enum`.
    pub enum_values: Option<Vec<String>>,
    pub ui: UiMeta,
}

// ─────────────────────────────────────────────────────────────────────────────
// Schema
// ─────────────────────────────────────────────────────────────────────────────

/// Parsed `schema.toml` (SDS §5.4).
#[derive(Debug)]
pub struct Schema {
    /// App name this schema belongs to (empty string when loaded from file).
    pub app: String,
    pub schema_version: u32,
    /// Parameters in the order they appear in the source file.
    pub params: IndexMap<String, ParamDef>,
}

impl Schema {
    /// Start building a schema for `app` with the given `schema_version`.
    pub fn builder(app: impl Into<String>, schema_version: u32) -> SchemaBuilder {
        SchemaBuilder {
            app: app.into(),
            schema_version,
            params: IndexMap::new(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SchemaBuilder
// ─────────────────────────────────────────────────────────────────────────────

/// Fluent builder for `Schema`, used by cube-presets tests.
#[must_use]
pub struct SchemaBuilder {
    app: String,
    schema_version: u32,
    params: IndexMap<String, ParamDef>,
}

impl SchemaBuilder {
    /// Add an `int` parameter with the given inclusive range.
    pub fn int_range(mut self, key: impl Into<String>, min: i64, max: i64) -> Self {
        let key = key.into();
        self.params.insert(
            key.clone(),
            ParamDef {
                key: key.clone(),
                ty: ParamType::Int,
                default: ParamValue::Int(min),
                min: Some(ParamValue::Int(min)),
                max: Some(ParamValue::Int(max)),
                step: None,
                unit: None,
                required: true,
                readonly: false,
                shareable: true,
                enum_values: None,
                ui: UiMeta::default(),
            },
        );
        self
    }

    /// Add a `float` parameter that is marked `readonly`.
    pub fn float_readonly(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.params.insert(
            key.clone(),
            ParamDef {
                key: key.clone(),
                ty: ParamType::Float,
                default: ParamValue::Float(0.0),
                min: None,
                max: None,
                step: None,
                unit: None,
                required: true,
                readonly: true,
                shareable: true,
                enum_values: None,
                ui: UiMeta::default(),
            },
        );
        self
    }

    /// Add a `string` parameter that is marked `shareable = false`.
    pub fn string_non_shareable(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.params.insert(
            key.clone(),
            ParamDef {
                key: key.clone(),
                ty: ParamType::String,
                default: ParamValue::String(String::new()),
                min: None,
                max: None,
                step: None,
                unit: None,
                required: true,
                readonly: false,
                shareable: false,
                enum_values: None,
                ui: UiMeta::default(),
            },
        );
        self
    }

    /// Finish building the schema.
    pub fn build(self) -> Schema {
        Schema {
            app: self.app,
            schema_version: self.schema_version,
            params: self.params,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Raw serde types (TOML deserialization layer)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawSchema {
    schema_version: Option<u32>,
    #[serde(default)]
    params: IndexMap<String, RawParamDef>,
}

#[derive(Deserialize)]
struct RawParamDef {
    key: String,
    #[serde(rename = "type")]
    ty: ParamType,
    /// The raw TOML value (must be present per SDS §5.4).
    default: Option<toml::Value>,
    min: Option<toml::Value>,
    max: Option<toml::Value>,
    step: Option<toml::Value>,
    unit: Option<String>,
    #[serde(default = "default_true")]
    required: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default = "default_true")]
    shareable: bool,
    values: Option<Vec<String>>,
    #[serde(default)]
    ui: UiMeta,
}

const fn default_true() -> bool {
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// TOML value -> ParamValue conversion helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Read a float from a TOML value, accepting integer as well.
///
/// # Precision
/// The precision loss from `i64` to `f64` is acceptable for config values.
#[allow(clippy::cast_precision_loss)]
fn toml_as_float(v: &toml::Value) -> Option<f64> {
    match v {
        toml::Value::Float(f) => Some(*f),
        toml::Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

/// Convert a TOML table field to `f64`, accepting both float and integer.
#[allow(clippy::cast_precision_loss)]
fn table_float(tbl: &toml::map::Map<String, toml::Value>, field: &str) -> Option<f64> {
    tbl.get(field).and_then(|v| match v {
        toml::Value::Float(f) => Some(*f),
        toml::Value::Integer(i) => Some(*i as f64),
        _ => None,
    })
}

fn convert_vec2(v: &toml::Value) -> Result<ParamValue, String> {
    let tbl = v
        .as_table()
        .ok_or_else(|| format!("expected table {{x, y}} for vec2, got {v:?}"))?;
    if tbl.contains_key("z") {
        return Err(
            "vec2 default must not have a 'z' component (dimensionality mismatch)".to_owned(),
        );
    }
    let x = table_float(tbl, "x").ok_or_else(|| "vec2 default missing 'x' field".to_owned())?;
    let y = table_float(tbl, "y").ok_or_else(|| "vec2 default missing 'y' field".to_owned())?;
    // f64->f32 precision loss is acceptable for config values.
    #[allow(clippy::cast_possible_truncation)]
    Ok(ParamValue::Vec2(cube_proto::Vec2 {
        x: x as f32,
        y: y as f32,
    }))
}

fn convert_vec3(v: &toml::Value) -> Result<ParamValue, String> {
    let tbl = v
        .as_table()
        .ok_or_else(|| format!("expected table {{x, y, z}} for vec3, got {v:?}"))?;
    let x = table_float(tbl, "x").ok_or_else(|| "vec3 default missing 'x' field".to_owned())?;
    let y = table_float(tbl, "y").ok_or_else(|| "vec3 default missing 'y' field".to_owned())?;
    let z = table_float(tbl, "z").ok_or_else(|| "vec3 default missing 'z' field".to_owned())?;
    // f64->f32 precision loss is acceptable for config values.
    #[allow(clippy::cast_possible_truncation)]
    Ok(ParamValue::Vec3(cube_proto::Vec3 {
        x: x as f32,
        y: y as f32,
        z: z as f32,
    }))
}

fn toml_to_param_value(ty: ParamType, v: &toml::Value) -> Result<ParamValue, String> {
    match ty {
        ParamType::Bool => {
            let b = v
                .as_bool()
                .ok_or_else(|| format!("expected bool, got {v:?}"))?;
            Ok(ParamValue::Bool(b))
        }
        ParamType::Int => {
            let i = v
                .as_integer()
                .ok_or_else(|| format!("expected integer, got {v:?}"))?;
            Ok(ParamValue::Int(i))
        }
        ParamType::Float => {
            let f = toml_as_float(v).ok_or_else(|| format!("expected float, got {v:?}"))?;
            Ok(ParamValue::Float(f))
        }
        ParamType::String => {
            let s = v
                .as_str()
                .ok_or_else(|| format!("expected string, got {v:?}"))?;
            Ok(ParamValue::String(s.to_owned()))
        }
        ParamType::Enum => {
            let s = v
                .as_str()
                .ok_or_else(|| format!("expected string, got {v:?}"))?;
            Ok(ParamValue::Enum(s.to_owned()))
        }
        ParamType::Color => {
            let s = v
                .as_str()
                .ok_or_else(|| format!("expected color hex string, got {v:?}"))?;
            // Parse via serde using the Color type from cube-proto.
            let color: cube_proto::Color =
                serde::Deserialize::deserialize(toml::Value::String(s.to_owned()))
                    .map_err(|e: toml::de::Error| format!("invalid color {s:?}: {e}"))?;
            Ok(ParamValue::Color(color))
        }
        ParamType::Vec2 => convert_vec2(v),
        ParamType::Vec3 => convert_vec3(v),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Numeric comparison helpers for min/max validation
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if `a > b` for numeric `ParamValue`s (Int, Float only).
fn param_gt(a: &ParamValue, b: &ParamValue) -> bool {
    match (a, b) {
        (ParamValue::Int(x), ParamValue::Int(y)) => x > y,
        (ParamValue::Float(x), ParamValue::Float(y)) => x > y,
        _ => false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Loader
// ─────────────────────────────────────────────────────────────────────────────

/// Parse `schema.toml` at the given path.
pub fn load_schema(path: &Path) -> Result<Schema, ConfigError> {
    let parse_err = |msg: String| ConfigError::Parse {
        path: path.to_owned(),
        message: msg,
    };

    if !path.exists() {
        return Err(ConfigError::Missing(path.to_owned()));
    }

    let raw_str = std::fs::read_to_string(path).map_err(|e| parse_err(e.to_string()))?;
    let raw: RawSchema = toml::from_str(&raw_str).map_err(|e| parse_err(e.to_string()))?;

    let schema_version = raw
        .schema_version
        .ok_or_else(|| parse_err("missing required field `schema_version`".to_owned()))?;

    let mut params = IndexMap::with_capacity(raw.params.len());

    for (map_key, rp) in raw.params {
        let def = convert_raw_param(rp, &parse_err)?;
        params.insert(map_key, def);
    }

    Ok(Schema {
        app: String::new(), // not stored in schema.toml; set by caller if needed
        schema_version,
        params,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Emission (the "agree by construction" counterpart of the loader; SDS §5.4,
// cubekit-spec §9.4)
// ─────────────────────────────────────────────────────────────────────────────

/// Lowercase TOML token for a [`ParamType`] (`[params.<key>] type`).
fn param_type_token(ty: ParamType) -> &'static str {
    match ty {
        ParamType::Bool => "bool",
        ParamType::Int => "int",
        ParamType::Float => "float",
        ParamType::String => "string",
        ParamType::Enum => "enum",
        ParamType::Color => "color",
        ParamType::Vec2 => "vec2",
        ParamType::Vec3 => "vec3",
    }
}

/// Map a [`ParamValue`] to a TOML scalar for the schema's `default`/`min`/`max`/
/// `step` fields. Int→integer, Float→float, Bool→bool, String/Enum→string,
/// Color→`#RRGGBB` hex string, Vec2/Vec3→`{ x, y[, z] }` tables.
fn param_value_to_toml(v: &ParamValue) -> Value {
    match v {
        ParamValue::Bool(b) => Value::Boolean(*b),
        ParamValue::Int(i) => Value::Integer(*i),
        ParamValue::Float(f) => Value::Float(*f),
        ParamValue::String(s) | ParamValue::Enum(s) => Value::String(s.clone()),
        ParamValue::Color(c) => {
            // `Color` has no `Display`, but its `Serialize` emits the `#RRGGBB`
            // hex form the schema loader parses back.
            Value::try_from(c).expect("Color serializes to a TOML string")
        }
        ParamValue::Vec2(p) => {
            let mut t = Table::new();
            t.insert("x".into(), Value::Float(f64::from(p.x)));
            t.insert("y".into(), Value::Float(f64::from(p.y)));
            Value::Table(t)
        }
        ParamValue::Vec3(p) => {
            let mut t = Table::new();
            t.insert("x".into(), Value::Float(f64::from(p.x)));
            t.insert("y".into(), Value::Float(f64::from(p.y)));
            t.insert("z".into(), Value::Float(f64::from(p.z)));
            Value::Table(t)
        }
    }
}

/// Serialize a [`Schema`] to `schema.toml` text (SDS §5.4).
///
/// The output is deterministic and key-sorted (a `toml::value::Table` is a
/// sorted map) and is accepted and round-tripped by [`load_schema`]. This is
/// the emission half of "agree by construction" (cubekit-spec §9.4): the same
/// crate that loads `schema.toml` also emits it, so emitter and loader cannot
/// drift.
#[must_use]
pub fn schema_to_toml(s: &Schema) -> String {
    let mut root = Table::new();
    root.insert(
        "schema_version".into(),
        Value::Integer(i64::from(s.schema_version)),
    );

    let mut params_tbl = Table::new();
    for (key, def) in &s.params {
        let mut t = Table::new();
        t.insert("key".into(), Value::String(def.key.clone()));
        t.insert("type".into(), Value::String(param_type_token(def.ty).into()));
        t.insert("default".into(), param_value_to_toml(&def.default));

        if let Some(min) = &def.min {
            t.insert("min".into(), param_value_to_toml(min));
        }
        if let Some(max) = &def.max {
            t.insert("max".into(), param_value_to_toml(max));
        }
        if let Some(step) = &def.step {
            t.insert("step".into(), param_value_to_toml(step));
        }
        if let Some(unit) = &def.unit {
            t.insert("unit".into(), Value::String(unit.clone()));
        }

        t.insert("required".into(), Value::Boolean(def.required));
        t.insert("readonly".into(), Value::Boolean(def.readonly));
        t.insert("shareable".into(), Value::Boolean(def.shareable));

        if let Some(values) = &def.enum_values {
            let arr = values.iter().map(|s| Value::String(s.clone())).collect();
            t.insert("values".into(), Value::Array(arr));
        }

        // UI metadata lives in a nested `ui` table (matching `RawParamDef::ui`).
        let mut ui = Table::new();
        if let Some(group) = &def.ui.group {
            ui.insert("group".into(), Value::String(group.clone()));
        }
        if let Some(label) = &def.ui.label {
            ui.insert("label".into(), Value::String(label.clone()));
        }
        if let Some(description) = &def.ui.description {
            ui.insert("description".into(), Value::String(description.clone()));
        }
        if !ui.is_empty() {
            t.insert("ui".into(), Value::Table(ui));
        }

        params_tbl.insert(key.clone(), Value::Table(t));
    }
    root.insert("params".into(), Value::Table(params_tbl));

    toml::to_string_pretty(&Value::Table(root)).expect("emitted schema is a valid toml::Table")
}

fn convert_raw_param(
    rp: RawParamDef,
    parse_err: &impl Fn(String) -> ConfigError,
) -> Result<ParamDef, ConfigError> {
    // `default` is required per SDS §5.4.
    let default_toml = rp
        .default
        .ok_or_else(|| parse_err(format!("param {:?}: missing required field `default`", rp.key)))?;

    let default = toml_to_param_value(rp.ty, &default_toml)
        .map_err(|e| parse_err(format!("param {:?} default: {e}", rp.key)))?;

    let min = rp
        .min
        .as_ref()
        .map(|v| toml_to_param_value(rp.ty, v))
        .transpose()
        .map_err(|e| parse_err(format!("param {:?} min: {e}", rp.key)))?;

    let max = rp
        .max
        .as_ref()
        .map(|v| toml_to_param_value(rp.ty, v))
        .transpose()
        .map_err(|e| parse_err(format!("param {:?} max: {e}", rp.key)))?;

    // min > max is a validation error (SDS §5.4).
    if let (Some(mn), Some(mx)) = (&min, &max)
        && param_gt(mn, mx)
    {
        return Err(parse_err(format!(
            "param {:?}: min ({mn:?}) > max ({mx:?})",
            rp.key
        )));
    }

    let step = rp
        .step
        .as_ref()
        .map(|v| toml_to_param_value(rp.ty, v))
        .transpose()
        .map_err(|e| parse_err(format!("param {:?} step: {e}", rp.key)))?;

    // Enum type requires a non-empty `values` list (SDS §5.4).
    if rp.ty == ParamType::Enum && rp.values.as_ref().is_none_or(Vec::is_empty) {
        return Err(parse_err(format!(
            "param {:?}: type 'enum' requires a non-empty `values` array",
            rp.key
        )));
    }

    Ok(ParamDef {
        key: rp.key,
        ty: rp.ty,
        default,
        min,
        max,
        step,
        unit: rp.unit,
        required: rp.required,
        readonly: rp.readonly,
        shareable: rp.shareable,
        enum_values: rp.values,
        ui: rp.ui,
    })
}
