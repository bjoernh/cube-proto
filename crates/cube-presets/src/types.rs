//! Public types for cube-presets.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use cube_proto::ParamValue;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Custom TOML datetime serialization for chrono::DateTime<Utc>
// ─────────────────────────────────────────────────────────────────────────────

mod chrono_datetime {
    use chrono::{DateTime, Datelike, Timelike, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use toml::value::{Date, Datetime, Offset, Time};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Datetime {
            date: Some(Date {
                year: dt.year() as u16,
                month: dt.month() as u8,
                day: dt.day() as u8,
            }),
            time: Some(Time {
                hour: dt.hour() as u8,
                minute: dt.minute() as u8,
                second: dt.second() as u8,
                nanosecond: dt.nanosecond(),
            }),
            offset: Some(Offset::Z),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use chrono::{FixedOffset, NaiveDate, NaiveTime, TimeZone};

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DatetimeInput {
            Datetime(Datetime),
            String(String),
        }
        match DatetimeInput::deserialize(deserializer)? {
            DatetimeInput::Datetime(d) => {
                // A native TOML datetime may legally omit the date (local-time)
                // or the time (local-date); a preset `created` needs both.
                let date = d
                    .date
                    .ok_or_else(|| serde::de::Error::custom("`created` is missing a date"))?;
                let time = d
                    .time
                    .ok_or_else(|| serde::de::Error::custom("`created` is missing a time"))?;

                let naive_date =
                    NaiveDate::from_ymd_opt(i32::from(date.year), date.month.into(), date.day.into())
                        .ok_or_else(|| serde::de::Error::custom("`created` has an invalid date"))?;
                let naive_time = NaiveTime::from_hms_nano_opt(
                    time.hour.into(),
                    time.minute.into(),
                    time.second.into(),
                    time.nanosecond,
                )
                .ok_or_else(|| serde::de::Error::custom("`created` has an invalid time"))?;
                let naive = naive_date.and_time(naive_time);

                // Honour the wire offset so a non-Z datetime maps to the right
                // instant. A local datetime (no offset) is taken as UTC — cubed
                // only ever emits `Z`, and UTC is the least-surprising default.
                let offset = match d.offset {
                    None | Some(Offset::Z) => FixedOffset::east_opt(0).unwrap(),
                    Some(Offset::Custom { minutes }) => {
                        FixedOffset::east_opt(i32::from(minutes) * 60).ok_or_else(|| {
                            serde::de::Error::custom("`created` has an invalid UTC offset")
                        })?
                    }
                };
                offset
                    .from_local_datetime(&naive)
                    .single()
                    .ok_or_else(|| serde::de::Error::custom("`created` is an ambiguous local time"))
                    .map(|dt| dt.with_timezone(&Utc))
            }
            DatetimeInput::String(v) => DateTime::parse_from_rfc3339(&v)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(serde::de::Error::custom),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetOrigin
// ─────────────────────────────────────────────────────────────────────────────

/// Where a preset comes from: the system directory (built-in), the user
/// directory (saved on this cube), or the community subdirectory (imported
/// from the app-store, App-Store M6). Name resolution precedence is
/// user > community > built-in — see `PresetStore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetOrigin {
    BuiltIn,
    User,
    Community,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetMeta
// ─────────────────────────────────────────────────────────────────────────────

/// `[meta]` section of a preset TOML file (SDS §5.5 format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetMeta {
    pub app: String,
    pub schema_version: u32,
    pub preset_name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(with = "chrono_datetime")]
    pub created: DateTime<Utc>,
    pub version: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetFile
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed preset TOML file with `[meta]` and `[params]` sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetFile {
    pub meta: PresetMeta,
    pub params: BTreeMap<String, ParamValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WarningCode
// ─────────────────────────────────────────────────────────────────────────────

/// Import warning codes (SDS §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    UnknownDropped,
    Clamped,
    ReadonlyDropped,
    NonShareableDropped,
}

// ─────────────────────────────────────────────────────────────────────────────
// ImportWarning
// ─────────────────────────────────────────────────────────────────────────────

/// A single warning emitted during import validation.
#[derive(Debug, Clone)]
pub struct ImportWarning {
    pub code: WarningCode,
    pub key: String,
    pub detail: BTreeMap<String, serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ImportReport
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a successful `import` operation.
#[derive(Debug, Clone)]
pub struct ImportReport {
    pub warnings: Vec<ImportWarning>,
    /// The final parameter map after all drops and clamps have been applied.
    pub final_params: BTreeMap<String, ParamValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SaveReport
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a successful `save` operation.
#[derive(Debug, Clone, Default)]
pub struct SaveReport {
    pub warnings: Vec<ImportWarning>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PresetError
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by `PresetStore` operations.
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("preset not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("schema version mismatch: expected {expected}, got {got}")]
    SchemaVersionMismatch { expected: u32, got: u32 },

    #[error("meta.app does not match the requested app")]
    AppMismatch,

    #[error("type mismatch: {0}")]
    TypeMismatch(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
