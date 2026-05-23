//! Session-token validation (SDS §6.2).
//!
//! Session tokens are URL-safe base64url (no padding) encoded strings
//! representing at least 16 bytes of random material.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Errors returned by [`validate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionTokenError {
    #[error("token is not valid base64url")]
    InvalidBase64,
    #[error("token decodes to fewer than 16 bytes")]
    TooShort,
}

/// Validate a session token string.
///
/// A valid token must be non-empty, decode as URL-safe base64url (no padding),
/// and represent at least 16 bytes of raw data.
pub fn validate(token: &str) -> Result<(), SessionTokenError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| SessionTokenError::InvalidBase64)?;
    if decoded.len() < 16 {
        return Err(SessionTokenError::TooShort);
    }
    Ok(())
}
