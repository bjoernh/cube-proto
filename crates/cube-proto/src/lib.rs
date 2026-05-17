//! Wire-protocol types for the cubed control plane.
//!
//! See the SDS §6.1 and ARCH §8 for the normative definitions.
//!
//! This crate intentionally has no I/O surface — only types and their
//! serde implementations. Transport (Unix SOCK_SEQPACKET, framing,
//! SCM_RIGHTS) lives in the daemon and client crates that consume these
//! types.

// Module layout will be fleshed out by Wave 1 implementation.

#![allow(dead_code)]
