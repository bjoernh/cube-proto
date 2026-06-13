//! JSON-Lines control-plane transport for `/run/cube/admin` (SDS §5.3, §6.1).
//! Connects over a Unix STREAM socket, performs the `hello` handshake, and
//! issues `set` / `preset.load` / `get-all` requests.
//!
//! Unlike a naive write-one-read-one client, the admin socket *auto-subscribes*
//! every connection after hello and interleaves unsolicited [`Event`] lines
//! with command [`Response`] lines on the same stream (SDS §6.2; mirrored by
//! `cubed/src/control_plane/admin.rs`). So this client splits the connection
//! into a **writer task** (drains serialized requests) and a **reader task**
//! that demultiplexes each inbound line: responses are matched to the waiting
//! caller by `id`, events are forwarded out-of-band on an events channel.
//!
//! It is deliberately *not* a `cubectl` subprocess: a fader sweep is hundreds
//! of CC/sec and one process spawn per event is a non-starter.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cube_proto::{Event, ParamValue, Request, Response, ResponseBody};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};

use crate::error::ClientError;

/// How long a single request waits for its matching response before giving up.
/// A missing reply must not wedge the translate loop forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Bound on the buffered events channel. The reader never blocks on a full
/// channel (it drops the event) so response delivery is never starved — a
/// dropped `param.changed` is harmless because LEDs re-render from the latest
/// value anyway (matches the daemon's broadcast "lag and continue").
const EVENTS_CHANNEL_CAP: usize = 256;

/// `id` used for the inline `hello` handshake; live request ids start after it.
const HELLO_ID: u64 = 1;

/// Protocol version sent in `hello` (matches the daemon's major; SDS §6.1).
pub const CLIENT_PROTOCOL_VERSION: &str = "1.0";

/// Default admin socket path (SDS §5.3 / §7.1).
pub const DEFAULT_ADMIN_SOCKET: &str = "/run/cube/admin";

/// A point-in-time read of an app's parameters (the result of `get-all`),
/// used to seed encoder accumulators and LEDs on (re)connect.
#[derive(Debug, Clone, Default)]
pub struct ParamSnapshot {
    /// The app's change sequence at the time of the snapshot.
    pub seq: u64,
    pub values: Vec<(String, ParamValue)>,
}

/// Resolve a `$CUBE_HOST` / `--host` value to a Unix socket path. Accepts
/// `unix:<path>` (mirrors `cubectl::Host::parse`) or a bare absolute path;
/// rejects `tcp:` and other schemes.
pub fn resolve_host(spec: Option<&str>) -> Result<PathBuf, ClientError> {
    let Some(s) = spec else {
        return Ok(PathBuf::from(DEFAULT_ADMIN_SOCKET));
    };
    if let Some(path) = s.strip_prefix("unix:") {
        return Ok(PathBuf::from(path));
    }
    if s.starts_with("tcp:") {
        return Err(ClientError::Host(
            "tcp host endpoints are not supported".into(),
        ));
    }
    if s.starts_with('/') {
        return Ok(PathBuf::from(s));
    }
    Err(ClientError::Host(format!("invalid host: {s}")))
}

/// Shared table mapping an in-flight request `id` to the caller awaiting it.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Response>>>>;

/// A connected, hello-completed control-plane client. The connection is driven
/// by a background reader/writer task pair; this handle issues requests and
/// hands out the [`Event`] stream via [`ControlClient::take_events`].
pub struct ControlClient {
    write_tx: mpsc::UnboundedSender<Vec<u8>>,
    pending: Pending,
    events_rx: Option<mpsc::Receiver<Event>>,
    next_id: u64,
}

impl ControlClient {
    /// Connect to `path`, complete the `hello` handshake inline, then spawn the
    /// reader/writer tasks. Returns an error if the handshake is rejected.
    pub async fn connect(path: &std::path::Path) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(path).await?;
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        // ── Inline hello ────────────────────────────────────────────────────
        // The daemon answers hello with a single response *before* it wires the
        // event forwarder (admin.rs steps 1→2), so reading exactly one line
        // here is race-free; events only start after this point.
        let hello = Request::Hello {
            id: HELLO_ID,
            protocol_version: CLIENT_PROTOCOL_VERSION.to_string(),
        };
        let mut line =
            serde_json::to_vec(&hello).map_err(|e| ClientError::Protocol(e.to_string()))?;
        line.push(b'\n');
        write_half.write_all(&line).await?;
        write_half.flush().await?;

        let mut buf = String::new();
        let n = reader.read_line(&mut buf).await?;
        if n == 0 {
            return Err(ClientError::Io(io::Error::new(
                ErrorKind::UnexpectedEof,
                "control plane closed before hello response",
            )));
        }
        let resp = serde_json::from_str::<Response>(buf.trim_end())
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        if !resp.ok {
            return Err(ClientError::Handshake(format!("{:?}", resp.body)));
        }

        // ── Spawn writer + reader tasks ──────────────────────────────────────
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(bytes) = write_rx.recv().await {
                if write_half.write_all(&bytes).await.is_err() {
                    break;
                }
                if write_half.flush().await.is_err() {
                    break;
                }
            }
        });

        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (events_tx, events_rx) = mpsc::channel::<Event>(EVENTS_CHANNEL_CAP);
        let pending_reader = Arc::clone(&pending);
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break, // EOF or I/O error
                    Ok(_) => {}
                }
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue; // skip unparseable lines rather than tearing down
                };
                if val.get("event").is_some() {
                    if let Ok(ev) = serde_json::from_value::<Event>(val) {
                        // Drop on full: never block the reader, or responses
                        // would stop being delivered and `set` would hang.
                        let _ = events_tx.try_send(ev);
                    }
                } else if let Ok(resp) = serde_json::from_value::<Response>(val)
                    && let Some(tx) = pending_reader.lock().unwrap().remove(&resp.id)
                {
                    let _ = tx.send(resp);
                }
            }
            // Connection gone: drop every waiter so awaiting requests error out
            // (the daemon then reconnects).
            pending_reader.lock().unwrap().clear();
        });

        Ok(Self {
            write_tx,
            pending,
            events_rx: Some(events_rx),
            next_id: HELLO_ID + 1,
        })
    }

    /// Take ownership of the inbound [`Event`] stream. Returns `None` if it has
    /// already been taken. Used by the bidirectional sync loop (Phase 5) to
    /// follow `param.changed` events; ignored when only one-way `set`s matter.
    pub fn take_events(&mut self) -> Option<mpsc::Receiver<Event>> {
        self.events_rx.take()
    }

    pub(crate) fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Send `set app.key = value`. Returns the parsed response (caller decides
    /// how to surface a non-`ok` result; the daemon logs it and continues).
    pub async fn set(
        &mut self,
        app: &str,
        key: &str,
        value: ParamValue,
    ) -> Result<Response, ClientError> {
        let id = self.alloc_id();
        self.request(
            id,
            &Request::Set {
                id,
                app: app.to_string(),
                key: key.to_string(),
                value,
            },
        )
        .await
    }

    /// Send `set-many app {key: value, …}` — one request for a whole coalescing
    /// window.
    pub async fn set_many(
        &mut self,
        app: &str,
        values: BTreeMap<String, ParamValue>,
    ) -> Result<Response, ClientError> {
        let id = self.alloc_id();
        self.request(
            id,
            &Request::SetMany {
                id,
                app: app.to_string(),
                values,
            },
        )
        .await
    }

    /// Send `preset.load app name`.
    pub async fn preset_load(&mut self, app: &str, name: &str) -> Result<Response, ClientError> {
        let id = self.alloc_id();
        self.request(
            id,
            &Request::PresetLoad {
                id,
                app: app.to_string(),
                name: name.to_string(),
            },
        )
        .await
    }

    /// Read all current parameter values for `app`. Returns an empty snapshot
    /// if the daemon rejects the request (e.g. the app is not running), so the
    /// caller can seed best-effort without special-casing.
    pub async fn get_all(&mut self, app: &str) -> Result<ParamSnapshot, ClientError> {
        let id = self.alloc_id();
        let resp = self
            .request(
                id,
                &Request::GetAll {
                    id,
                    app: app.to_string(),
                },
            )
            .await?;
        if !resp.ok {
            return Ok(ParamSnapshot::default());
        }
        let ResponseBody::Result { result: Some(val) } = resp.body else {
            return Ok(ParamSnapshot::default());
        };
        let seq = val
            .get("seq")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let values = val
            .get("values")
            .and_then(serde_json::Value::as_object)
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        serde_json::from_value::<ParamValue>(v.clone())
                            .ok()
                            .map(|pv| (k.clone(), pv))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(ParamSnapshot { seq, values })
    }

    /// Register a waiter for `id`, send the serialized request through the
    /// writer task, and await the matching response (bounded by
    /// [`REQUEST_TIMEOUT`]). The reader task routes the response back by `id`.
    pub(crate) async fn request(
        &mut self,
        id: u64,
        req: &Request,
    ) -> Result<Response, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        let mut line = serde_json::to_vec(req).map_err(|e| ClientError::Protocol(e.to_string()))?;
        line.push(b'\n');
        if self.write_tx.send(line).is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(ClientError::Io(io::Error::new(
                ErrorKind::BrokenPipe,
                "control plane writer closed",
            )));
        }

        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(resp)) => Ok(resp),
            // Reader task dropped the sender: the connection closed.
            Ok(Err(_)) => Err(ClientError::Io(io::Error::new(
                ErrorKind::UnexpectedEof,
                "control plane closed",
            ))),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(ClientError::Timeout)
            }
        }
    }
}
