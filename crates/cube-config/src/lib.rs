//! Configuration loaders for the cube system.
//!
//! (system.toml example), (manifest format),.4
//! (schema format), and (SIGHUP reload pattern).

#![deny(warnings)]

mod manifest;
mod schema;
mod system;

pub use manifest::{
    Accent, AppSection, Manifest, ManifestCategory, OverlaySection, Players, PowerSection,
    RequiresSection, load_manifest, manifest_to_toml,
};
pub use schema::{
    GamepadSchema, ParamDef, ParamType, Schema, SchemaBuilder, UiMeta, load_schema, schema_to_toml,
};
pub use system::{
    AppsConfig, ConfigError, DebugConfig, DisplayConfig, ImuConfig, InputConfig, PowerConfig,
    RemoteRenderConfig, SystemConfig, SystemConfigHandle, TransitionsConfig, ValidationReport,
    ValidationWarning, load_system,
};
