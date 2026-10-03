//! — the shared admin client's subscription helpers.
//!
//! Pins the surface that `cubectl watch --events` and `cube-mcp cube_watch`
//! build on: an open-ended `subscribe` returning the [`cube_proto::SubscribeResult`]
//! ack plus a live event stream, and a bounded `subscribe_collect` that drains
//! exactly the bounded batch (the daemon ends it with `subscription.ended`).
//! A scripted in-test admin server stands in for `cubed`.

use std::time::Duration;

use cube_admin_client::{ControlClient, SubscribeOptions};
use cube_proto::Event;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

// ─────────────────────────────────────────────────────────────────────────────
// scripted admin server
// ─────────────────────────────────────────────────────────────────────────────

/// Answer `hello`, then for the first `subscribe` reply with an ack carrying
/// `sub_id`/`event_seq` (optional `snapshot`), then stream `events`, then —
/// if `end_after` is set — a terminal `subscription.ended`.
fn spawn_admin(
    listener: UnixListener,
    ack: serde_json::Value,
    events: Vec<String>,
    end_after: Option<&'static str>,
) {
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut line = String::new();

        // hello
        reader.read_line(&mut line).await.expect("hello");
        let hello: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        let hid = hello["id"].as_u64().unwrap();
        write
            .write_all(format!("{{\"id\":{hid},\"ok\":true}}\n").as_bytes())
            .await
            .unwrap();
        write.flush().await.unwrap();

        // subscribe
        line.clear();
        reader.read_line(&mut line).await.expect("subscribe");
        let sub: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(
            sub["cmd"], "subscribe",
            "client must send a subscribe: {sub}"
        );
        let sid = sub["id"].as_u64().unwrap();
        let mut ack = ack;
        ack["id"] = serde_json::json!(sid);
        ack["ok"] = serde_json::json!(true);
        write
            .write_all(format!("{ack}\n").as_bytes())
            .await
            .unwrap();
        write.flush().await.unwrap();

        for ev in events {
            write.write_all(format!("{ev}\n").as_bytes()).await.unwrap();
            write.flush().await.unwrap();
        }
        if let Some(sub_id) = end_after {
            let ended = format!(
                "{{\"event\":\"subscription.ended\",\"sub_id\":\"{sub_id}\",\"reason\":\"max_events\"}}\n"
            );
            write.write_all(ended.as_bytes()).await.unwrap();
            write.flush().await.unwrap();
        }

        // Keep the socket open so an open-ended client's reader stays alive.
        let mut sink = String::new();
        let _ = reader.read_line(&mut sink).await;
    });
}

fn bind() -> (tempfile::TempDir, std::path::PathBuf, UnixListener) {
    let dir = tempfile::tempdir().unwrap();
    let sock = dir.path().join("admin.sock");
    let listener = UnixListener::bind(&sock).unwrap();
    (dir, sock, listener)
}

// ─────────────────────────────────────────────────────────────────────────────
// subscribe (open-ended)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn subscribe_returns_ack_and_streams_events() {
    let (_dir, sock, listener) = bind();
    spawn_admin(
        listener,
        serde_json::json!({"result": {"sub_id": "s-1", "event_seq": 5012}}),
        vec![
            "{\"event\":\"app.started\",\"app\":\"snake\",\"event_seq\":5013}".into(),
            "{\"event\":\"brightness.changed\",\"value\":200}".into(),
        ],
        None,
    );

    let mut client = ControlClient::connect(&sock).await.expect("connect+hello");
    let opts = SubscribeOptions::new(&["lifecycle", "brightness"]).app("*");
    let (ack, mut events) = client.subscribe(opts).await.expect("subscribe");

    assert_eq!(ack.sub_id, "s-1");
    assert_eq!(ack.event_seq, 5012);
    assert!(ack.snapshot.is_none());

    let first = tokio::time::timeout(Duration::from_secs(2), events.recv())
        .await
        .expect("event arrives")
        .expect("stream open");
    match first {
        Event::AppStarted { app, event_seq } => {
            assert_eq!(app, "snake");
            assert_eq!(event_seq, Some(5013));
        }
        other => panic!("expected app.started, got {other:?}"),
    }
    let second = tokio::time::timeout(Duration::from_secs(2), events.recv())
        .await
        .expect("event arrives")
        .expect("stream open");
    assert!(matches!(second, Event::BrightnessChanged { value: 200 }));
}

#[tokio::test]
async fn subscribe_with_snapshot_surfaces_it() {
    let (_dir, sock, listener) = bind();
    spawn_admin(
        listener,
        serde_json::json!({"result": {
            "sub_id": "s-1", "event_seq": 0,
            "snapshot": {"power": {"state": "active"}, "brightness": {"value": 128}}
        }}),
        vec![],
        None,
    );

    let mut client = ControlClient::connect(&sock).await.expect("connect+hello");
    let opts = SubscribeOptions::new(&["power", "brightness"]).snapshot(true);
    let (ack, _events) = client.subscribe(opts).await.expect("subscribe");

    let snapshot = ack.snapshot.expect("snapshot present");
    assert_eq!(
        snapshot.power.expect("power").state,
        cube_proto::PowerState::Active
    );
    assert_eq!(snapshot.brightness.expect("brightness").value, 128);
}

// ─────────────────────────────────────────────────────────────────────────────
// subscribe_collect (bounded)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn subscribe_collect_drains_bounded_batch() {
    let (_dir, sock, listener) = bind();
    spawn_admin(
        listener,
        serde_json::json!({"result": {"sub_id": "s-1", "event_seq": 0}}),
        vec![
            "{\"event\":\"app.started\",\"app\":\"a\",\"event_seq\":1}".into(),
            "{\"event\":\"app.started\",\"app\":\"b\",\"event_seq\":2}".into(),
        ],
        Some("s-1"),
    );

    let mut client = ControlClient::connect(&sock).await.expect("connect+hello");
    let opts = SubscribeOptions::new(&["lifecycle"]).max_events(2);
    let collected = client
        .subscribe_collect(opts, Duration::from_secs(3))
        .await
        .expect("collect");

    // The two app.started events, then the terminal subscription.ended.
    assert_eq!(collected.len(), 3, "{collected:?}");
    assert!(matches!(&collected[0], Event::AppStarted { app, .. } if app == "a"));
    assert!(matches!(&collected[1], Event::AppStarted { app, .. } if app == "b"));
    assert!(matches!(
        &collected[2],
        Event::SubscriptionEnded { sub_id, .. } if sub_id == "s-1"
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// clean EOF surfacing
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn request_after_server_eof_errors_cleanly() {
    let (_dir, sock, listener) = bind();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("hello");
        let hid: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        let hid = hid["id"].as_u64().unwrap();
        write
            .write_all(format!("{{\"id\":{hid},\"ok\":true}}\n").as_bytes())
            .await
            .unwrap();
        write.flush().await.unwrap();
        // Drop the connection immediately after hello.
        drop(write);
        drop(reader);
    });

    let mut client = ControlClient::connect(&sock).await.expect("connect+hello");
    // Give the server's drop time to propagate.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let opts = SubscribeOptions::new(&["lifecycle"]);
    let err = client
        .subscribe(opts)
        .await
        .expect_err("subscribe after EOF must error");
    // A surfaced error, not a panic or an infinite wait.
    let _ = err;
}

/// The connect helper accepts a `unix:`-prefixed host and a bare path
/// (mirrors `cubectl::Host`), and rejects `tcp:`.
#[test]
fn resolve_host_accepts_unix_and_path_rejects_tcp() {
    use cube_admin_client::resolve_host;
    assert_eq!(
        resolve_host(None).unwrap(),
        std::path::PathBuf::from("/run/cube/admin")
    );
    assert_eq!(
        resolve_host(Some("unix:/tmp/a.sock")).unwrap(),
        std::path::PathBuf::from("/tmp/a.sock")
    );
    assert_eq!(
        resolve_host(Some("/tmp/b.sock")).unwrap(),
        std::path::PathBuf::from("/tmp/b.sock")
    );
    assert!(resolve_host(Some("tcp:1.2.3.4:9")).is_err());
}

// Keep an unused import from tripping the suite if a helper is trimmed.
#[allow(dead_code)]
async fn _connect_compiles(p: &std::path::Path) {
    let _ = UnixStream::connect(p).await;
}
