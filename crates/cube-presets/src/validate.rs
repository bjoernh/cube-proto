//! Name validation for presets and apps (SDS §5.5).

use crate::types::PresetError;

/// Validate a preset or app name.
///
/// Accepted: `^[a-zA-Z0-9._-]+$`, not `.` or `..`.
///
/// Returns `Err(PresetError::BadRequest)` BEFORE any filesystem operation.
pub(crate) fn validate_name(name: &str) -> Result<(), PresetError> {
    if name.is_empty() {
        return Err(PresetError::BadRequest(
            "name must not be empty".to_owned(),
        ));
    }
    if name == "." || name == ".." {
        return Err(PresetError::BadRequest(format!(
            "name must not be '.' or '..', got {name:?}"
        )));
    }
    // SDS §5.5 regex: ^[a-zA-Z0-9._-]+$
    // Also reject '/' and control characters.
    for ch in name.chars() {
        if ch == '/' {
            return Err(PresetError::BadRequest(format!(
                "name must not contain '/': {name:?}"
            )));
        }
        if ch.is_control() {
            return Err(PresetError::BadRequest(format!(
                "name must not contain control characters: {name:?}"
            )));
        }
        if !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_' && ch != '-' {
            return Err(PresetError::BadRequest(format!(
                "name contains invalid character '{ch}': {name:?}"
            )));
        }
    }
    Ok(())
}
