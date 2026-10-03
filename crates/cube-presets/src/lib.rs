//! Preset TOML I/O for cubed.
//!
//! (preset semantics, import warnings, `schema_version`) and
//! (persistence: flock, write-temp-rename, fsync ordering).

mod fs_ops;
mod store;
mod types;
mod validate;

pub use fs_ops::{FsCall, FsOps, RealFsOps, RecordingFsOps};
pub use store::PresetStore;
pub use types::{
    ImportReport, ImportWarning, PresetError, PresetFile, PresetMeta, PresetOrigin, SaveReport,
    WarningCode,
};
