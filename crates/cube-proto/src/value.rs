//! Shared value types used in parameter commands and frame events.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ─────────────────────────────────────────────────────────────────────────────
// Damage
// ─────────────────────────────────────────────────────────────────────────────

/// Dirty-rectangle hint for a `present` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Damage {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Format
// ─────────────────────────────────────────────────────────────────────────────

/// Pixel format for a registered buffer.
///
/// Base/app layers are `RGB565` (no alpha); overlay layers are `ARGB8888`
/// (real alpha for `over` compositing — / changelog "Pixel
/// formats").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Format {
    #[serde(rename = "RGB565")]
    Rgb565,
    /// 4 bytes/pixel with a real alpha byte — the overlay-layer format.
    #[serde(rename = "ARGB8888")]
    Argb8888,
}

// ─────────────────────────────────────────────────────────────────────────────
// Color
// ─────────────────────────────────────────────────────────────────────────────

/// RGBA color. Serializes as `#RRGGBB` when α=0xFF, `#RRGGBBAA`
/// otherwise. Both forms are accepted on deserialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    #[must_use]
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// The `(r, g, b, a)` channels. Used by the system text-overlay path to
    /// drive the glyph renderer's foreground colour.
    #[must_use]
    pub fn rgba(&self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let hex = if self.a == 0xFF {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        };
        s.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex: String = Deserialize::deserialize(d)?;
        parse_color(&hex).ok_or_else(|| serde::de::Error::custom(format!("invalid color: {hex}")))
    }
}

fn parse_color(hex: &str) -> Option<Color> {
    let digits = hex.strip_prefix('#')?;
    match digits.len() {
        6 => {
            let red = u8::from_str_radix(&digits[0..2], 16).ok()?;
            let green = u8::from_str_radix(&digits[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&digits[4..6], 16).ok()?;
            Some(Color {
                r: red,
                g: green,
                b: blue,
                a: 0xFF,
            })
        }
        8 => {
            let red = u8::from_str_radix(&digits[0..2], 16).ok()?;
            let green = u8::from_str_radix(&digits[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&digits[4..6], 16).ok()?;
            let alpha = u8::from_str_radix(&digits[6..8], 16).ok()?;
            Some(Color {
                r: red,
                g: green,
                b: blue,
                a: alpha,
            })
        }
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Vec2 / Vec3
// ─────────────────────────────────────────────────────────────────────────────

/// 2-D float vector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// 3-D float vector.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// ParamValue
// ─────────────────────────────────────────────────────────────────────────────

/// Typed parameter value.
///
/// Tagged with `"type"` / `"value"` keys on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum ParamValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    #[serde(rename = "enum")]
    Enum(String),
    Color(Color),
    Vec2(Vec2),
    Vec3(Vec3),
}
