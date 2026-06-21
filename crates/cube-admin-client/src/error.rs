//! Error type for the shared admin-socket client (SDS §5.3 / §6.1).

/// Errors from the control-plane client. Re-exported by `cube-midi-bridge` as
/// `cube_midi_bridge::error::ClientError` so the bridge's existing call sites
/// keep resolving after the transport core moved into this crate.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("control-plane I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid host endpoint: {0}")]
    Host(String),
    #[error("handshake rejected: {0}")]
    Handshake(String),
    #[error("malformed response: {0}")]
    Protocol(String),
    #[error("control-plane request timed out")]
    Timeout,
}
