//! Binary frame-stream header (SDS §6.2).
//!
//! Each binary frame sent over the remote-render channel is prefixed by a
//! 16-byte `FrameHeader` encoded in little-endian byte order.

/// Magic bytes identifying a valid frame header.
/// `0x43554246` encodes the ASCII string `"CUBF"` in big-endian; on the wire
/// it is stored little-endian as `[0x46, 0x42, 0x55, 0x43]`.
pub const FRAME_HEADER_MAGIC: u32 = 0x4355_4246;

/// Byte length of the encoded frame header.
pub const FRAME_HEADER_BYTES: usize = 16;

/// Errors returned by [`FrameHeader::decode`] and [`SeqGuard::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameHeaderError {
    #[error("buffer is shorter than FRAME_HEADER_BYTES ({FRAME_HEADER_BYTES})")]
    ShortBuffer,
    #[error("magic bytes do not match FRAME_HEADER_MAGIC")]
    BadMagic,
    #[error("reserved field is non-zero")]
    NonZeroReserved,
    #[error("sequence number went backwards")]
    BackwardsSeq,
}

/// 16-byte binary frame header.
///
/// Layout (all fields little-endian, 4 bytes each):
/// `[magic u32][seq u32][reserved u32][reserved u32]`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub magic: u32,
    pub seq: u32,
    pub reserved2: u32,
    pub reserved: u32,
}

impl FrameHeader {
    /// Encode the header to a 16-byte array (little-endian).
    pub fn encode(self) -> [u8; FRAME_HEADER_BYTES] {
        let mut buf = [0u8; FRAME_HEADER_BYTES];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&self.seq.to_le_bytes());
        buf[8..12].copy_from_slice(&self.reserved2.to_le_bytes());
        buf[12..16].copy_from_slice(&self.reserved.to_le_bytes());
        buf
    }

    /// Decode a header from a byte slice.
    ///
    /// Returns [`FrameHeaderError::ShortBuffer`] if `bytes` has fewer than
    /// [`FRAME_HEADER_BYTES`] bytes.  Validation checks magic, size, and
    /// reserved in that order.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameHeaderError> {
        if bytes.len() < FRAME_HEADER_BYTES {
            return Err(FrameHeaderError::ShortBuffer);
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let seq = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let reserved2 = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let reserved = u32::from_le_bytes(bytes[12..16].try_into().unwrap());

        if magic != FRAME_HEADER_MAGIC {
            return Err(FrameHeaderError::BadMagic);
        }
        if reserved2 != 0 {
            return Err(FrameHeaderError::NonZeroReserved);
        }
        if reserved != 0 {
            return Err(FrameHeaderError::NonZeroReserved);
        }
        Ok(Self { magic, seq, reserved2, reserved })
    }
}

/// Monotonic sequence-number guard.
///
/// Rejects any sequence number that is less than or equal to the last accepted
/// one (after the first call).
pub struct SeqGuard {
    last: Option<u32>,
}

impl SeqGuard {
    /// Create a new guard.  `_initial` is currently unused but kept for API
    /// forward-compatibility.
    pub fn new(_initial: u32) -> Self {
        Self { last: None }
    }

    /// Accept `seq` if it is strictly greater than the previously seen value.
    ///
    /// The very first call always succeeds regardless of value.
    pub fn check(&mut self, seq: u32) -> Result<(), FrameHeaderError> {
        if let Some(last) = self.last
            && seq <= last
        {
            return Err(FrameHeaderError::BackwardsSeq);
        }
        self.last = Some(seq);
        Ok(())
    }
}
