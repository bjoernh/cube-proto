//! Shared helpers for the cube-presets integration tests.
//!
//! Wave 3 RED scaffolding: these helpers reference items in
//! `cube_presets::*` and `cube_config::*` that do not yet exist. The tests
//! that import this module are intentionally non-compiling until the
//! implementation lands.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use cube_config::Schema;
use cube_presets::{PresetStore, RealFsOps};

/// Build the standard test layout under `root`:
///
/// ```text
/// <root>/system/apps/<app>/presets/...
/// <root>/user/apps/<app>/presets/...
/// <root>/user/apps/<app>/.lock
/// ```
///
/// Returns `(system_root, user_root)`.
pub fn make_layout(root: &Path, app: &str) -> (PathBuf, PathBuf) {
    let system_root = root.join("system").join("apps");
    let user_root = root.join("user").join("apps");
    std::fs::create_dir_all(system_root.join(app).join("presets")).unwrap();
    std::fs::create_dir_all(user_root.join(app).join("presets")).unwrap();
    (system_root, user_root)
}

/// Construct a `PresetStore<RealFsOps>` over the standard layout.
pub fn make_store(root: &Path, app: &str) -> (PresetStore<RealFsOps>, PathBuf, PathBuf) {
    let (system_root, user_root) = make_layout(root, app);
    let store = PresetStore::new(system_root.clone(), user_root.clone(), RealFsOps::default());
    (store, system_root, user_root)
}

/// A schema describing four shareable, mutable params:
///   - `brightness`: int, range [0, 10], shareable=true, readonly=false
///   - `speed`:      int, range [0, 10], shareable=true, readonly=false
///   - `gamma`:      float, shareable=true, readonly=true
///   - `device_id`:  string, shareable=false, readonly=false
///
/// Used by `import_warnings.rs` to exercise the four warning codes.
pub fn schema_for_warnings(app: &str, version: u32) -> Schema {
    Schema::builder(app, version)
        .int_range("brightness", 0, 10)
        .int_range("speed", 0, 10)
        .float_readonly("gamma")
        .string_non_shareable("device_id")
        .build()
}

/// A schema with just `speed`, used by tests that don't care about warning kinds.
pub fn schema_minimal(app: &str, version: u32) -> Schema {
    Schema::builder(app, version).int_range("speed", 0, 10).build()
}
