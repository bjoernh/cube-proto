//! Wire-protocol types for the cubed control plane.
//!
//! See the SDS §6.1 and ARCH §8 for the normative definitions.
//!
//! This crate intentionally has no I/O surface — only types and their
//! serde implementations. Transport (Unix `SOCK_SEQPACKET`, framing,
//! `SCM_RIGHTS`) lives in the daemon and client crates that consume these
//! types.

pub mod doctor;
pub mod error;
pub mod event;
pub mod request;
pub mod response;
pub mod status;
pub mod value;

pub use doctor::{CheckLevel, CheckResult, DoctorReportPayload};
pub use error::{CubeErrno, CubeError};
pub use event::Event;
pub use request::Request;
pub use response::{Response, ResponseBody};
pub use status::{CubedStatus, DisplayStatus, FramesDropped, PerAppStatus, StatusReport};
pub use value::{Color, Damage, Format, ParamValue, Vec2, Vec3};

/// Maximum permitted wire-message size (SDS §5.3 / §6.1).
pub const MAX_MESSAGE_BYTES: usize = 65_536;

/// Errors returned by this crate's utility helpers.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("message exceeds MAX_MESSAGE_BYTES ({MAX_MESSAGE_BYTES}): {len} bytes")]
    Oversize { len: usize },
}

/// Return `Err(ProtoError::Oversize)` if `bytes` exceeds `MAX_MESSAGE_BYTES`.
///
/// Messages exactly at the limit are accepted.
pub fn enforce_max_size(bytes: &[u8]) -> Result<(), ProtoError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        Err(ProtoError::Oversize { len: bytes.len() })
    } else {
        Ok(())
    }
}
