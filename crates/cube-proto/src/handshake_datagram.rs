//! UDP handshake datagram (SDS v5 §6.2).
//!
//! The first datagram sent on the UDP frame socket carries magic `CUBH`,
//! the `session_token`, and the negotiated format/dimensions. Distinct
//! from per-frame datagrams (magic `CUBF`).

/// Magic bytes identifying a handshake datagram: ASCII "CUBH" as LE u32.
pub const HANDSHAKE_MAGIC: u32 = 0x4355_4248;

/// Total size of the handshake datagram in bytes.
pub const HANDSHAKE_BYTES: usize = 36;

/// Errors returned by [`HandshakeDatagram::decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("buffer is shorter than HANDSHAKE_BYTES ({HANDSHAKE_BYTES})")]
    ShortBuffer,
    #[error("magic bytes do not match HANDSHAKE_MAGIC")]
    BadMagic,
    #[error("flags field is non-zero")]
    NonZeroFlags,
    #[error("unsupported format tag (expected 1 = RGB565)")]
    UnsupportedFormat,
    #[error("unexpected dimensions (expected 384x64)")]
    BadDimensions,
    #[error("expected_payload_bytes mismatch")]
    BadPayloadSize,
}

/// 36-byte handshake datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeDatagram {
    pub magic: u32,
    pub flags: u32,
    pub session_token: [u8; 16],
    pub format_tag: u32,
    pub width: u16,
    pub height: u16,
    pub expected_payload_bytes: u32,
}

impl HandshakeDatagram {
    /// Encode to a 36-byte array (all fields little-endian).
    pub fn encode(self) -> [u8; HANDSHAKE_BYTES] {
        let mut buf = [0u8; HANDSHAKE_BYTES];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&self.flags.to_le_bytes());
        buf[8..24].copy_from_slice(&self.session_token);
        buf[24..28].copy_from_slice(&self.format_tag.to_le_bytes());
        buf[28..30].copy_from_slice(&self.width.to_le_bytes());
        buf[30..32].copy_from_slice(&self.height.to_le_bytes());
        buf[32..36].copy_from_slice(&self.expected_payload_bytes.to_le_bytes());
        buf
    }

    /// Decode from a byte slice. Validates magic, flags, format_tag,
    /// dimensions, and expected_payload_bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, HandshakeError> {
        if bytes.len() < HANDSHAKE_BYTES {
            return Err(HandshakeError::ShortBuffer);
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != HANDSHAKE_MAGIC {
            return Err(HandshakeError::BadMagic);
        }
        let flags = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if flags != 0 {
            return Err(HandshakeError::NonZeroFlags);
        }
        let mut session_token = [0u8; 16];
        session_token.copy_from_slice(&bytes[8..24]);
        let format_tag = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        if format_tag != 1 {
            return Err(HandshakeError::UnsupportedFormat);
        }
        let width = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
        let height = u16::from_le_bytes(bytes[30..32].try_into().unwrap());
        if width != 384 || height != 64 {
            return Err(HandshakeError::BadDimensions);
        }
        let expected_payload_bytes = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        if expected_payload_bytes != 49152 {
            return Err(HandshakeError::BadPayloadSize);
        }
        Ok(HandshakeDatagram {
            magic,
            flags,
            session_token,
            format_tag,
            width,
            height,
            expected_payload_bytes,
        })
    }
}
