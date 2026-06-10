//! Configuration loaders for the cube system.
//!
//! See SDS §6.4 (system.toml example), §7.2 (manifest format), §5.4
//! (schema format), and ARCH §4.11 (SIGHUP reload pattern).

#![deny(warnings)]

mod manifest;
mod schema;
mod system;

pub use manifest::{
    AppSection, Manifest, ManifestCategory, PowerSection, RequiresSection, load_manifest,
};
pub use schema::{ParamDef, ParamType, Schema, SchemaBuilder, UiMeta, load_schema};
pub use system::{
    AppsConfig, ConfigError, DisplayConfig, ImuConfig, InputConfig, PowerConfig,
    RemoteRenderConfig, SystemConfig, SystemConfigHandle, TransitionsConfig, ValidationReport,
    ValidationWarning, load_system,
};
