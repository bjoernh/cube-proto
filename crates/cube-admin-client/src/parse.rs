//! Shared value-string and `--scope` parsing for the admin control surface.
//!
//! These parsers turn CLI/tool argument strings into typed [`cube_proto`]
//! values. They live here — in the neutral transport crate — rather than in
//! `cubectl`, so every admin client (`cubectl`, `cube-mcp`) shares one
//! implementation instead of the MCP server welding to the CLI's crate.

use cube_proto::{Color, ParamValue, Vec2, Vec3};

/// Schema-type hint used by `set` / `set-many` value parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetType {
    Bool,
    Int,
    Float,
    String,
    Enum,
    Color,
    Vec2,
    Vec3,
}

// ─────────────────────────────────────────────────────────────────────────────
// Value parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by [`parse_param_value`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValueParseError {
    #[error("invalid bool literal: {0}")]
    Bool(String),
    #[error("invalid int literal: {0}")]
    Int(String),
    #[error("invalid float literal: {0}")]
    Float(String),
    #[error("invalid color literal: {0}")]
    Color(String),
    #[error("invalid vec literal: {0}")]
    Vec(String),
}

/// Infer a [`ParamValue`] from a raw string without a type hint.
///
/// Detection order: bool → int → float → string.
/// Use `parse_param_value` with an explicit hint to force a specific type.
#[must_use]
pub fn parse_param_value_auto(raw: &str) -> ParamValue {
    match raw {
        "true" => return ParamValue::Bool(true),
        "false" => return ParamValue::Bool(false),
        _ => {}
    }
    if let Ok(n) = raw.parse::<i64>() {
        return ParamValue::Int(n);
    }
    if let Ok(f) = raw.parse::<f64>() {
        return ParamValue::Float(f);
    }
    ParamValue::String(raw.to_owned())
}

/// Parse a CLI value string under a schema-type hint. Implements
/// argument forms for every `ParamValue` variant.
pub fn parse_param_value(ty: SetType, raw: &str) -> Result<ParamValue, ValueParseError> {
    match ty {
        SetType::Bool => match raw {
            "true" | "1" => Ok(ParamValue::Bool(true)),
            "false" | "0" => Ok(ParamValue::Bool(false)),
            _ => Err(ValueParseError::Bool(raw.to_owned())),
        },
        SetType::Int => raw
            .parse::<i64>()
            .map(ParamValue::Int)
            .map_err(|_| ValueParseError::Int(raw.to_owned())),
        SetType::Float => raw
            .parse::<f64>()
            .map(ParamValue::Float)
            .map_err(|_| ValueParseError::Float(raw.to_owned())),
        SetType::String => Ok(ParamValue::String(raw.to_owned())),
        SetType::Enum => Ok(ParamValue::Enum(raw.to_owned())),
        SetType::Color => parse_color_value(raw),
        SetType::Vec2 => parse_vec2_value(raw),
        SetType::Vec3 => parse_vec3_value(raw),
    }
}

fn parse_color_value(raw: &str) -> Result<ParamValue, ValueParseError> {
    let digits = raw
        .strip_prefix('#')
        .ok_or_else(|| ValueParseError::Color(raw.to_owned()))?;
    match digits.len() {
        6 => {
            let r = u8::from_str_radix(&digits[0..2], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            let g = u8::from_str_radix(&digits[2..4], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            let b = u8::from_str_radix(&digits[4..6], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            Ok(ParamValue::Color(Color::from_rgba(r, g, b, 0xFF)))
        }
        8 => {
            let r = u8::from_str_radix(&digits[0..2], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            let g = u8::from_str_radix(&digits[2..4], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            let b = u8::from_str_radix(&digits[4..6], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            let a = u8::from_str_radix(&digits[6..8], 16)
                .map_err(|_| ValueParseError::Color(raw.to_owned()))?;
            Ok(ParamValue::Color(Color::from_rgba(r, g, b, a)))
        }
        _ => Err(ValueParseError::Color(raw.to_owned())),
    }
}

fn parse_vec2_value(raw: &str) -> Result<ParamValue, ValueParseError> {
    // Reject any spaces
    if raw.contains(' ') {
        return Err(ValueParseError::Vec(raw.to_owned()));
    }
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 2 {
        return Err(ValueParseError::Vec(raw.to_owned()));
    }
    let x = parts[0]
        .parse::<f32>()
        .map_err(|_| ValueParseError::Vec(raw.to_owned()))?;
    let y = parts[1]
        .parse::<f32>()
        .map_err(|_| ValueParseError::Vec(raw.to_owned()))?;
    Ok(ParamValue::Vec2(Vec2 { x, y }))
}

fn parse_vec3_value(raw: &str) -> Result<ParamValue, ValueParseError> {
    if raw.contains(' ') {
        return Err(ValueParseError::Vec(raw.to_owned()));
    }
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 3 {
        return Err(ValueParseError::Vec(raw.to_owned()));
    }
    let x = parts[0]
        .parse::<f32>()
        .map_err(|_| ValueParseError::Vec(raw.to_owned()))?;
    let y = parts[1]
        .parse::<f32>()
        .map_err(|_| ValueParseError::Vec(raw.to_owned()))?;
    let z = parts[2]
        .parse::<f32>()
        .map_err(|_| ValueParseError::Vec(raw.to_owned()))?;
    Ok(ParamValue::Vec3(Vec3 { x, y, z }))
}

/// Parse a single `key=value` argument for `set-many`. Returns the split halves
/// without applying a schema type yet.
pub fn parse_kv(arg: &str) -> Result<(String, String), ValueParseError> {
    let pos = arg
        .find('=')
        .ok_or_else(|| ValueParseError::Vec(format!("expected key=value, got: {arg}")))?;
    Ok((arg[..pos].to_owned(), arg[pos + 1..].to_owned()))
}

// ─────────────────────────────────────────────────────────────────────────────
// `--scope` parsing — cube-gamepad Tier 2
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by [`parse_scope`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScopeParseError {
    #[error("invalid --scope {0:?}: expected \"global\" or \"game:APP\" with a non-empty APP name")]
    Invalid(String),
}

/// Parse a `cubectl input …` `--scope` string into a [`cube_proto::BindingScope`].
///
/// `"global"` → [`cube_proto::BindingScope::Global`]; `"game:NAME"` with a
/// non-empty `NAME` → [`cube_proto::BindingScope::Game`]`(NAME)`; anything
/// else is a [`ScopeParseError`].
///
/// This is the shared scope syntax for cube-gamepad "Tier 2" admin verbs
/// and is reused by `cube-mcp`'s input tools.
pub fn parse_scope(s: &str) -> Result<cube_proto::BindingScope, ScopeParseError> {
    if s == "global" {
        return Ok(cube_proto::BindingScope::Global);
    }
    if let Some(name) = s.strip_prefix("game:")
        && !name.is_empty()
    {
        return Ok(cube_proto::BindingScope::Game(name.to_owned()));
    }
    Err(ScopeParseError::Invalid(s.to_owned()))
}
