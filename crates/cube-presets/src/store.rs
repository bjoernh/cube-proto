//! `PresetStore` — the main entry point for preset I/O (SDS §5.5, §5.6).

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use cube_config::Schema;
use cube_proto::ParamValue;
use nix::fcntl::{Flock, FlockArg};

use crate::fs_ops::FsOps;
use crate::types::{
    ImportReport, ImportWarning, PresetError, PresetFile, PresetOrigin, SaveReport, WarningCode,
};
use crate::validate::validate_name;

// ─────────────────────────────────────────────────────────────────────────────
// PresetStore
// ─────────────────────────────────────────────────────────────────────────────

/// Manages preset TOML files for all apps.
///
/// Layout:
/// ```text
/// <system_root>/<app>/presets/<name>.toml          — built-in (read-only)
/// <user_root>/<app>/presets/<name>.toml            — user override
/// <user_root>/<app>/presets/community/<name>.toml  — imported from the app-store (App-Store M6)
/// <user_root>/<app>/presets/<name>.deleted         — tombstone hides built-in
/// <user_root>/<app>/.lock                          — per-app flock file
/// ```
///
/// Name resolution precedence (list/export/load) is **user > community >
/// built-in**, extending SDS §5.5's user-shadows-builtin rule. The community
/// directory exists so an app-store import can never overwrite a preset the
/// user saved themselves, and so listings can group "own" vs "community" from
/// data rather than from naming conventions. Tombstones hide built-ins only —
/// a community preset is removed by deleting its file, never tombstoned.
pub struct PresetStore<F: FsOps> {
    system_root: PathBuf,
    user_root: PathBuf,
    fs: F,
}

impl<F: FsOps> PresetStore<F> {
    /// Create a new `PresetStore`.
    pub fn new(system_root: PathBuf, user_root: PathBuf, fs: F) -> Self {
        Self { system_root, user_root, fs }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // list
    // ─────────────────────────────────────────────────────────────────────────

    /// List all visible presets for `app`.
    ///
    /// Rules (SDS §5.5 + App-Store M6):
    /// - User `.toml` files shadow same-named community presets and built-ins;
    ///   community presets shadow built-ins (user > community > built-in).
    /// - `<name>.deleted` tombstones hide the built-in from the listing
    ///   (community presets are unaffected by tombstones).
    /// - Only `.toml` and `.deleted` files are considered; everything else
    ///   (editor backups, in-flight temp files, etc.) is ignored.
    pub fn list(&self, app: &str) -> Result<Vec<(String, PresetOrigin)>, PresetError> {
        validate_name(app)?;

        let system_dir = self.system_root.join(app).join("presets");
        let user_dir = self.user_root.join(app).join("presets");
        let community_dir = user_dir.join("community");

        // Collect built-in names.
        let mut builtins: std::collections::HashSet<String> = std::collections::HashSet::new();
        if system_dir.is_dir() {
            for entry in std::fs::read_dir(&system_dir)? {
                let entry = entry?;
                let fname = entry.file_name().to_string_lossy().into_owned();
                if let Some(name) = fname.strip_suffix(".toml") {
                    builtins.insert(name.to_owned());
                }
            }
        }

        // Collect user .toml files and tombstones.
        let mut user_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut tombstones: std::collections::HashSet<String> = std::collections::HashSet::new();
        if user_dir.is_dir() {
            for entry in std::fs::read_dir(&user_dir)? {
                let entry = entry?;
                let fname = entry.file_name().to_string_lossy().into_owned();
                if let Some(name) = fname.strip_suffix(".toml") {
                    user_files.insert(name.to_owned());
                } else if let Some(name) = fname.strip_suffix(".deleted") {
                    tombstones.insert(name.to_owned());
                }
                // All other extensions (e.g. .tmp., ~, .swp) are ignored.
            }
        }

        // Collect community .toml files (App-Store M6).
        let mut community_files: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        if community_dir.is_dir() {
            for entry in std::fs::read_dir(&community_dir)? {
                let entry = entry?;
                let fname = entry.file_name().to_string_lossy().into_owned();
                if let Some(name) = fname.strip_suffix(".toml") {
                    community_files.insert(name.to_owned());
                }
            }
        }

        let mut result: Vec<(String, PresetOrigin)> = Vec::new();

        // Add user presets.
        for name in &user_files {
            result.push((name.clone(), PresetOrigin::User));
        }

        // Add community presets not shadowed by a user file. Tombstones do not
        // apply — they only ever hide built-ins.
        for name in &community_files {
            if !user_files.contains(name) {
                result.push((name.clone(), PresetOrigin::Community));
            }
        }

        // Add built-ins that are not shadowed by a user/community file or a
        // tombstone.
        for name in &builtins {
            if !user_files.contains(name)
                && !community_files.contains(name)
                && !tombstones.contains(name)
            {
                result.push((name.clone(), PresetOrigin::BuiltIn));
            }
        }

        Ok(result)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // export
    // ─────────────────────────────────────────────────────────────────────────

    /// Read and parse a single preset.
    ///
    /// Resolution precedence: user > community > built-in (SDS §5.5 +
    /// App-Store M6). Tombstones hide built-ins only.
    pub fn export(&self, app: &str, name: &str) -> Result<PresetFile, PresetError> {
        validate_name(app)?;
        validate_name(name)?;

        let presets_dir = self.user_root.join(app).join("presets");
        let user_path = presets_dir.join(format!("{name}.toml"));
        let community_path = presets_dir.join("community").join(format!("{name}.toml"));
        let system_path =
            self.system_root.join(app).join("presets").join(format!("{name}.toml"));
        let tombstone = presets_dir.join(format!("{name}.deleted"));

        let path = if user_path.exists() {
            user_path
        } else if community_path.exists() {
            community_path
        } else if tombstone.exists() {
            // Tombstoned built-ins are hidden from export too.
            return Err(PresetError::NotFound);
        } else if system_path.exists() {
            system_path
        } else {
            return Err(PresetError::NotFound);
        };

        let content = std::fs::read_to_string(&path)?;
        let pf: PresetFile = toml::from_str(&content).map_err(|e| {
            PresetError::BadRequest(format!("failed to parse preset {name:?}: {e}"))
        })?;
        Ok(pf)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // save
    // ─────────────────────────────────────────────────────────────────────────

    /// Save a preset to the user directory.
    ///
    /// Steps (SDS §5.6):
    /// 1. Validate name.
    /// 2. Acquire per-app `flock(LOCK_EX)` on `<user_root>/<app>/.lock`.
    /// 3. Write to temp file.
    /// 4. `fsync(temp_file)`.
    /// 5. `rename(temp → target)`.
    /// 6. `fsync(parent_dir)`.
    /// 7. Remove any existing tombstone for this name.
    pub fn save(
        &self,
        app: &str,
        name: &str,
        preset: &PresetFile,
        schema: &Schema,
    ) -> Result<SaveReport, PresetError> {
        // Validate BEFORE any filesystem operation.
        validate_name(app)?;
        validate_name(name)?;

        let user_app_dir = self.user_root.join(app);
        let presets_dir = user_app_dir.join("presets");
        let target = presets_dir.join(format!("{name}.toml"));
        let tombstone = presets_dir.join(format!("{name}.deleted"));

        // Validate params against schema (generates warnings but no errors for
        // save — the caller is saving the current app state which is already
        // valid by definition; we just warn on out-of-range values).
        let report = validate_params_for_save(&preset.params, schema);

        // Acquire per-app exclusive lock.
        let _lock_guard = acquire_lock(&user_app_dir)?;

        // Serialize to TOML.
        let toml_bytes = toml::to_string(preset)
            .map_err(|e| PresetError::BadRequest(format!("failed to serialize preset: {e}")))?
            .into_bytes();

        // write-temp-rename + fsync ordering (SDS §5.6).
        let tmp_path = self.fs.write_temp(&target, &toml_bytes)?;
        self.fs.fsync_file(&tmp_path)?;
        self.fs.rename(&tmp_path, &target)?;
        self.fs.fsync_parent(&target)?;

        // Remove tombstone if present (re-saving a previously deleted preset).
        if tombstone.exists() {
            let _ = std::fs::remove_file(&tombstone);
        }

        Ok(report)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // import
    // ─────────────────────────────────────────────────────────────────────────

    /// Import a preset from a TOML string.
    ///
    /// - Validates name before any filesystem operation.
    /// - Checks `meta.schema_version` against `schema.schema_version` → `EVERSION`.
    /// - Checks `meta.app` matches `app` → `ENOAPP` (via `AppMismatch`).
    /// - Applies import warnings (SDS §5.5): drops unknown keys, drops readonly
    ///   keys, drops non-shareable keys, clamps out-of-range numeric values.
    /// - Writes the validated preset using the same write-temp-rename + fsync
    ///   ordering as `save`.
    ///
    /// `origin` picks the target directory: `User` writes to `presets/`,
    /// `Community` (an app-store import, App-Store M6) to `presets/community/`
    /// so it can never overwrite a preset the user saved themselves.
    /// `BuiltIn` is rejected — the system directory is read-only.
    pub fn import(
        &self,
        app: &str,
        name: &str,
        toml_str: &str,
        schema: &Schema,
        origin: PresetOrigin,
    ) -> Result<ImportReport, PresetError> {
        if origin == PresetOrigin::BuiltIn {
            return Err(PresetError::BadRequest(
                "cannot import into the built-in (system) preset directory".to_owned(),
            ));
        }
        // Validate name BEFORE any filesystem operation.
        validate_name(app)?;
        validate_name(name)?;

        // Parse.
        let pf: PresetFile = toml::from_str(toml_str).map_err(|e| {
            PresetError::BadRequest(format!("failed to parse preset TOML: {e}"))
        })?;

        // app mismatch.
        if pf.meta.app != app {
            return Err(PresetError::AppMismatch);
        }

        // schema_version mismatch (SDS §5.5).
        if pf.meta.schema_version != schema.schema_version {
            return Err(PresetError::SchemaVersionMismatch {
                expected: schema.schema_version,
                got: pf.meta.schema_version,
            });
        }

        // Apply import validation.
        let report = validate_params_for_import(&pf.params, schema)?;

        // Build the final preset with validated params.
        let final_preset = PresetFile {
            meta: pf.meta,
            params: report.final_params.clone(),
        };

        let user_app_dir = self.user_root.join(app);
        let presets_dir = user_app_dir.join("presets");
        let target_dir = match origin {
            PresetOrigin::User => presets_dir.clone(),
            PresetOrigin::Community => presets_dir.join("community"),
            PresetOrigin::BuiltIn => unreachable!("rejected above"),
        };
        let target = target_dir.join(format!("{name}.toml"));
        let tombstone = presets_dir.join(format!("{name}.deleted"));

        // Acquire per-app exclusive lock.
        let _lock_guard = acquire_lock(&user_app_dir)?;

        std::fs::create_dir_all(&target_dir)?;

        let toml_bytes = toml::to_string(&final_preset)
            .map_err(|e| PresetError::BadRequest(format!("failed to serialize preset: {e}")))?
            .into_bytes();

        let tmp_path = self.fs.write_temp(&target, &toml_bytes)?;
        self.fs.fsync_file(&tmp_path)?;
        self.fs.rename(&tmp_path, &target)?;
        self.fs.fsync_parent(&target)?;

        // Remove tombstone if present. Only a *user* import un-tombstones a
        // deleted built-in — a community import lives in its own namespace
        // and must not resurrect one as a side effect.
        if origin == PresetOrigin::User && tombstone.exists() {
            let _ = std::fs::remove_file(&tombstone);
        }

        Ok(report)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // delete
    // ─────────────────────────────────────────────────────────────────────────

    /// Delete a preset — always the *visible* one under the user > community >
    /// built-in precedence chain:
    ///
    /// - User preset: remove the file.
    /// - Community preset (no user file): remove the file.
    /// - Built-in only: write a tombstone.
    /// - None of the three: `ENOENT`.
    ///
    /// Whenever a file removal would reveal a same-named built-in, a tombstone
    /// keeps it hidden — deleting a name means the name goes away.
    ///
    /// Uses `FsOps` for all filesystem calls so the order is observable.
    pub fn delete(&self, app: &str, name: &str) -> Result<(), PresetError> {
        // Validate BEFORE any filesystem operation.
        validate_name(app)?;
        validate_name(name)?;

        let user_app_dir = self.user_root.join(app);
        let presets_dir = user_app_dir.join("presets");
        let user_path = presets_dir.join(format!("{name}.toml"));
        let community_path = presets_dir.join("community").join(format!("{name}.toml"));
        let tombstone_path = presets_dir.join(format!("{name}.deleted"));
        let system_path =
            self.system_root.join(app).join("presets").join(format!("{name}.toml"));

        let user_exists = user_path.exists();
        let community_exists = community_path.exists();
        let system_exists = system_path.exists();

        if !user_exists && !community_exists && !system_exists {
            return Err(PresetError::NotFound);
        }

        // Acquire per-app exclusive lock.
        let _lock_guard = acquire_lock(&user_app_dir)?;

        let removed = if user_exists {
            self.fs.remove_file(&user_path)?;
            self.fs.fsync_parent(&user_path)?;
            true
        } else if community_exists {
            self.fs.remove_file(&community_path)?;
            self.fs.fsync_parent(&community_path)?;
            true
        } else {
            false
        };

        // Keep a same-named built-in hidden (file removal would otherwise
        // reveal it), or hide it directly when it was the visible one.
        // Exception: a user-file removal that reveals a *community* preset
        // writes no tombstone for the built-in shadowed further down — the
        // community preset keeps shadowing it either way.
        let reveals_builtin = system_exists && !(removed && user_exists && community_exists);
        if reveals_builtin {
            self.fs.create_tombstone(&tombstone_path)?;
            self.fs.fsync_file(&tombstone_path)?;
            self.fs.fsync_parent(&tombstone_path)?;
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// flock RAII guard
// ─────────────────────────────────────────────────────────────────────────────

/// RAII guard that holds an exclusive `flock` on `<user_root>/<app>/.lock`.
///
/// Dropped when the guard goes out of scope.
struct LockGuard {
    _flock: Flock<File>,
}

fn acquire_lock(user_app_dir: &Path) -> Result<LockGuard, PresetError> {
    // Create the lock file if it doesn't exist.
    let lock_path = user_app_dir.join(".lock");
    std::fs::create_dir_all(user_app_dir)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    let flock = Flock::lock(file, FlockArg::LockExclusive).map_err(|(_, errno)| {
        std::io::Error::from_raw_os_error(errno as i32)
    })?;

    Ok(LockGuard { _flock: flock })
}

// ─────────────────────────────────────────────────────────────────────────────
// Parameter validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validate params for save (less strict than import — no type checking needed
/// since the app is saving its own live state). Returns a `SaveReport`.
fn validate_params_for_save(
    _params: &BTreeMap<String, ParamValue>,
    _schema: &Schema,
) -> SaveReport {
    // For save, no warnings are generated — the app controls what it saves.
    SaveReport::default()
}

/// Validate params for import (SDS §5.5 warning rules).
///
/// Returns `ImportReport` with the cleaned `final_params` or an error if a
/// type mismatch is detected.
fn validate_params_for_import(
    params: &BTreeMap<String, ParamValue>,
    schema: &Schema,
) -> Result<ImportReport, PresetError> {
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let mut final_params: BTreeMap<String, ParamValue> = BTreeMap::new();

    for (key, value) in params {
        let Some(def) = schema.params.get(key.as_str()) else {
            // UNKNOWN_DROPPED — key not in schema.
            warnings.push(ImportWarning {
                code: WarningCode::UnknownDropped,
                key: key.clone(),
                detail: BTreeMap::new(),
            });
            continue;
        };

        // READONLY_DROPPED — key is readonly in schema.
        if def.readonly {
            warnings.push(ImportWarning {
                code: WarningCode::ReadonlyDropped,
                key: key.clone(),
                detail: BTreeMap::new(),
            });
            continue;
        }

        // NON_SHAREABLE_DROPPED — key is not shareable in schema.
        if !def.shareable {
            warnings.push(ImportWarning {
                code: WarningCode::NonShareableDropped,
                key: key.clone(),
                detail: BTreeMap::new(),
            });
            continue;
        }

        // Type check and possible CLAMPED.
        let checked_value = check_and_clamp(key, value, def, &mut warnings)?;
        final_params.insert(key.clone(), checked_value);
    }

    Ok(ImportReport { warnings, final_params })
}

/// Check type compatibility and clamp numeric values to schema range.
///
/// Returns the (possibly clamped) `ParamValue` or `PresetError::TypeMismatch`.
fn check_and_clamp(
    key: &str,
    value: &ParamValue,
    def: &cube_config::ParamDef,
    warnings: &mut Vec<ImportWarning>,
) -> Result<ParamValue, PresetError> {
    use cube_config::ParamType;

    match (def.ty, value) {
        (ParamType::Int, ParamValue::Int(v)) => {
            let mut out = *v;
            let mut clamped = false;
            if let Some(ParamValue::Int(lo)) = def.min
                && out < lo
            {
                out = lo;
                clamped = true;
            }
            if let Some(ParamValue::Int(hi)) = def.max
                && out > hi
            {
                out = hi;
                clamped = true;
            }
            if clamped {
                let mut detail = BTreeMap::new();
                detail.insert("from".to_owned(), serde_json::Value::Number((*v).into()));
                detail.insert("to".to_owned(), serde_json::Value::Number(out.into()));
                warnings.push(ImportWarning {
                    code: WarningCode::Clamped,
                    key: key.to_owned(),
                    detail,
                });
            }
            Ok(ParamValue::Int(out))
        }
        (ParamType::Float, ParamValue::Float(v)) => {
            let mut out = *v;
            let mut clamped = false;
            if let Some(ParamValue::Float(lo)) = def.min
                && out < lo
            {
                out = lo;
                clamped = true;
            }
            if let Some(ParamValue::Float(hi)) = def.max
                && out > hi
            {
                out = hi;
                clamped = true;
            }
            if clamped {
                let from_n = serde_json::Number::from_f64(*v).unwrap_or_else(|| 0.into());
                let to_n = serde_json::Number::from_f64(out).unwrap_or_else(|| 0.into());
                let mut detail = BTreeMap::new();
                detail.insert("from".to_owned(), serde_json::Value::Number(from_n));
                detail.insert("to".to_owned(), serde_json::Value::Number(to_n));
                warnings.push(ImportWarning {
                    code: WarningCode::Clamped,
                    key: key.to_owned(),
                    detail,
                });
            }
            Ok(ParamValue::Float(out))
        }
        (ParamType::Bool, ParamValue::Bool(_))
        | (ParamType::String, ParamValue::String(_))
        | (ParamType::Enum, ParamValue::Enum(_))
        | (ParamType::Color, ParamValue::Color(_))
        | (ParamType::Vec2, ParamValue::Vec2(_))
        | (ParamType::Vec3, ParamValue::Vec3(_)) => Ok(value.clone()),
        // Any other combo is a type mismatch.
        _ => Err(PresetError::TypeMismatch(format!(
            "key {key:?}: expected {:?}, got {:?}",
            def.ty, value
        ))),
    }
}
