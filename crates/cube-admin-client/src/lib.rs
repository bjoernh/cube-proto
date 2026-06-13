//! Shared JSON-Lines client for `cubed`'s admin socket (`/run/cube/admin`).
//!
//! This is the reusable control-plane transport extracted from the MIDI bridge,
//! so every client (the MIDI bridge, `cubectl`, `cube-mcp`) speaks the admin
//! protocol through one implementation rather than re-deriving the framing.
//!
//! - **Transport** (SDS §5.3, §6.1): [`ControlClient`] connects over a Unix
//!   STREAM socket, performs the `hello` handshake, and runs a writer/reader
//!   task pair that demultiplexes id-matched [`cube_proto::Response`]s from the
//!   out-of-band [`cube_proto::Event`] stream the daemon interleaves on the same
//!   connection. Request helpers (`set`/`set_many`/`preset_load`/`get_all`) and
//!   [`resolve_host`] live here.
//! - **Subscriptions** (SDS §5.13): [`SubscribeOptions`] plus
//!   [`ControlClient::subscribe`] (open-ended: ack + live event stream) and
//!   [`ControlClient::subscribe_collect`] (bounded: drain exactly the batch,
//!   ended by `subscription.ended`).

mod client;
mod error;
mod subscribe;

pub use client::{
    CLIENT_PROTOCOL_VERSION, ControlClient, DEFAULT_ADMIN_SOCKET, ParamSnapshot, resolve_host,
};
pub use error::ClientError;
pub use subscribe::SubscribeOptions;
