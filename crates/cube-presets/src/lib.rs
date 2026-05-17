//! Preset TOML I/O for cubed.
//!
//! See SDS §5.5 (preset semantics, import warnings, schema_version) and
//! §5.6 (persistence: flock, write-temp-rename, fsync ordering).

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
