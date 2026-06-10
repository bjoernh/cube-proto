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
pub mod frame_header;
pub mod handshake_datagram;
pub mod request;
pub mod response;
pub mod session_token;
pub mod status;
pub mod value;

pub use doctor::{CheckLevel, CheckResult, DoctorReportPayload};
pub use error::{CubeErrno, CubeError};
pub use event::{ChangeSource, Event, FocusLostReason, PowerState, PresentDroppedReason, ReleaseReason};
pub use handshake_datagram::{HandshakeDatagram, HandshakeError, HANDSHAKE_BYTES, HANDSHAKE_MAGIC};
pub use request::{HelloResult, Request};
pub use response::{Response, ResponseBody};
pub use status::{CubedStatus, DisplayStatus, FramesDropped, PerAppStatus, StatusReport};
pub use value::{Color, Damage, Format, ParamValue, Vec2, Vec3};

/// Maximum permitted wire-message size (SDS §5.3 / §6.1).
pub const MAX_MESSAGE_BYTES: usize = 65_536;

/// Control-protocol major version (SDS §5.3 `hello`/`EVERSION` gate).
///
/// `EVERSION` rejects a `hello` whose major component differs from this
/// value. v6 is a **minor** bump within major 1 (delta §7): v5/`libcube`
/// clients sending `hello {protocol_version: "1.0"}` (or any `"1.x"`)
/// remain compatible and need no changes.
pub const PROTOCOL_MAJOR: u32 = 1;

/// Control-protocol minor version for the v6 surface (SDS v6; delta §4–§7).
///
/// Bumped from v5's `0` to `1` to mark the addition of `focus.lost`,
/// `focus.gained`, `power.blank`, `power.wake`, and the residency model.
/// A client (e.g. `cubekit`) detects the v6 surface by reading `cubed`'s
/// advertised [`PROTOCOL_VERSION`] from the `hello` OK response
/// ([`HelloResult`]) and checking `minor >= PROTOCOL_MINOR`.
pub const PROTOCOL_MINOR: u32 = 1;

/// Full protocol version string advertised by `cubed` in the `hello` OK
/// response (`"<major>.<minor>"`). Not to be confused with the app/manifest
/// `version` field (SDS §7.2, semver `"2.0.0"` in the worked example) — that
/// is an unrelated per-app version, not the control-protocol version.
pub const PROTOCOL_VERSION: &str = "1.1";

/// Expected payload size for a single remote frame (SDS §6.2).
pub const REMOTE_FRAME_PAYLOAD_BYTES: usize = 49_152;

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
