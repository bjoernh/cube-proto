//! Golden wire fixtures for the compositor surface (plan R4.3, SDS v7 §5.13,
//! §6.1). This target is the **cross-SDK contract artifact**: it writes one
//! canonical JSON file per request/response/event the compositor adds to the
//! wire into `tests/fixtures/wire/*.json`, then reads every file back and proves
//! it round-trips through the typed `cube_proto` model byte-for-byte. libcube
//! and cubekit consume these same files verbatim (R5/R6/R7), so the daemon and
//! both SDKs share one source of truth for the exact bytes on the socket.
//!
//! Run `cargo test -p cube-proto --test wire_fixtures` to (re)generate the
//! files — the fixtures are committed, and a drift between the in-code canonical
//! table and the enum/serde definitions fails the round-trip assertion.

use std::path::PathBuf;

use serde_json::{Value, json};

use cube_proto::{Capabilities, Event, HelloResult, Request, Response, TransitionKind, TransitionSpec};

/// Which typed model a fixture decodes through.
#[derive(Clone, Copy)]
enum Kind {
    Request,
    Response,
    Event,
}

struct Fixture {
    /// File stem under `tests/fixtures/wire/` (no extension).
    name: &'static str,
    kind: Kind,
    /// The canonical wire JSON. Decoding it into the typed model and
    /// re-encoding MUST reproduce this value exactly.
    json: Value,
}

/// The canonical wire shapes for every request/response/event the compositor
/// surface touches (plan R4.3 enumerates the set). Ordered request → response →
/// event for readability.
fn fixtures() -> Vec<Fixture> {
    vec![
        // ── present with an overlay `layer` ──────────────────────────────────
        Fixture {
            name: "present_with_layer",
            kind: Kind::Request,
            json: json!({
                "id": 41, "cmd": "present", "seq": 1, "buffer_id": 2, "layer": 7
            }),
        },
        // ── overlay.acquire / overlay.release ────────────────────────────────
        Fixture {
            name: "overlay_acquire",
            kind: Kind::Request,
            json: json!({
                "id": 40, "cmd": "overlay.acquire", "z": 1, "input": "modal"
            }),
        },
        Fixture {
            name: "overlay_release",
            kind: Kind::Request,
            json: json!({"id": 42, "cmd": "overlay.release", "layer": 7}),
        },
        // ── admin text overlay.text / overlay.clear ──────────────────────────
        Fixture {
            name: "overlay_text",
            kind: Kind::Request,
            json: json!({
                "id": 50, "cmd": "overlay.text", "text": "Hello TEST",
                "duration_ms": 3000, "z": 1, "color": "#FFFFFF"
            }),
        },
        Fixture {
            name: "overlay_clear",
            kind: Kind::Request,
            json: json!({"id": 52, "cmd": "overlay.clear", "z": 1}),
        },
        // ── transition on launch / focus ─────────────────────────────────────
        Fixture {
            name: "transition_on_launch",
            kind: Kind::Request,
            json: json!({
                "id": 31, "cmd": "launch", "app": "pixelflow",
                "transition": {"kind": "cut"}
            }),
        },
        Fixture {
            name: "transition_on_focus",
            kind: Kind::Request,
            json: json!({
                "id": 30, "cmd": "focus", "app": "snake",
                "transition": {"kind": "crossfade", "duration_ms": 250}
            }),
        },
        // ── responses ────────────────────────────────────────────────────────
        Fixture {
            name: "overlay_acquire_ok",
            kind: Kind::Response,
            json: json!({"id": 40, "ok": true, "result": {"layer": 7}}),
        },
        Fixture {
            name: "hello_response_with_capabilities",
            kind: Kind::Response,
            json: json!({
                "id": 1, "ok": true,
                "result": {
                    "protocol_version": "1.2",
                    "surface": "v6",
                    "capabilities": {
                        "transition_kinds": [
                            "cut", "crossfade", "dissolve",
                            "dip_to_black", "particle_dissolve", "push"
                        ],
                        "overlay": true
                    }
                }
            }),
        },
        // ── events ───────────────────────────────────────────────────────────
        Fixture {
            name: "overlay_dismissed",
            kind: Kind::Event,
            json: json!({"event": "overlay.dismissed", "layer": 7, "reason": "released"}),
        },
        // buffer.release WITHOUT layer — the base-layer release, pre-compositor
        // wire shape (no `layer` key on the wire).
        Fixture {
            name: "buffer_release_base",
            kind: Kind::Event,
            json: json!({
                "event": "buffer.release", "buffer_id": 0, "seq": 100, "reason": "displayed"
            }),
        },
        // buffer.release WITH layer — an overlay-buffer release routed by `layer`.
        Fixture {
            name: "buffer_release_overlay",
            kind: Kind::Event,
            json: json!({
                "event": "buffer.release", "buffer_id": 2, "seq": 5,
                "reason": "replaced", "layer": 7
            }),
        },
    ]
}

/// Absolute path to `tests/fixtures/wire/` (created on demand).
fn wire_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("wire");
    p
}

/// Decode `json` into its typed model, re-encode, and assert equality. This is
/// the byte-for-byte contract check.
fn assert_roundtrip(f: &Fixture) {
    let back = match f.kind {
        Kind::Request => {
            let v: Request = serde_json::from_value(f.json.clone())
                .unwrap_or_else(|e| panic!("{}: Request decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
        Kind::Response => {
            let v: Response = serde_json::from_value(f.json.clone())
                .unwrap_or_else(|e| panic!("{}: Response decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
        Kind::Event => {
            let v: Event = serde_json::from_value(f.json.clone())
                .unwrap_or_else(|e| panic!("{}: Event decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
    };
    assert_eq!(back, f.json, "{}: did not round-trip", f.name);
}

/// Write every canonical fixture to `tests/fixtures/wire/<name>.json` (the
/// committed contract artifact), then read each file back and prove it
/// round-trips through the typed model. Write and read live in one test so the
/// two never race over the shared files under the parallel test runner; the
/// committed files ARE regenerated on every run.
#[test]
fn wire_fixtures_write_and_roundtrip() {
    let dir = wire_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures/wire dir");

    // 1. Write: the canonical table must be self-consistent before it is
    //    persisted, so a broken shape fails here rather than being written out.
    for f in fixtures() {
        assert_roundtrip(&f);
        let mut path = dir.clone();
        path.push(format!("{}.json", f.name));
        let pretty = serde_json::to_string_pretty(&f.json).expect("pretty");
        std::fs::write(&path, format!("{pretty}\n")).expect("write fixture");
    }

    // 2. Read back FROM DISK: the bytes on disk are the authoritative contract a
    //    consuming SDK mirrors. Each must equal the canonical shape and
    //    decode+re-encode through the typed `cube_proto` model unchanged.
    for f in fixtures() {
        let mut path = dir.clone();
        path.push(format!("{}.json", f.name));
        let on_disk: Value = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", f.name)),
        )
        .unwrap_or_else(|e| panic!("parse {}: {e}", f.name));
        assert_eq!(
            on_disk, f.json,
            "{}: file on disk drifted from the canonical table",
            f.name
        );
        assert_roundtrip(&Fixture {
            name: f.name,
            kind: f.kind,
            json: on_disk,
        });
    }
}

/// Lenient transition-kind decode (plan R4.2b, D8): an unknown `kind` token
/// does NOT hard-fail the enclosing `focus`/`launch` request — it decodes to
/// `cut`. The strict [`cube_proto::TransitionKind::from_wire`] is the daemon's
/// hook to detect the fallback and warn. Proves `deny_unknown_fields` on the
/// request/spec is not what rejects an unknown kind *value* (it only rejects
/// unknown *fields*).
#[test]
fn unknown_transition_kind_falls_back_to_cut() {
    // A whole `focus` request naming a kind this build does not know still
    // decodes — the request is accepted, the kind degrades to `cut`.
    let req: Request = serde_json::from_value(json!({
        "id": 30, "cmd": "focus", "app": "snake",
        "transition": {"kind": "warp_zoom", "duration_ms": 200}
    }))
    .expect("unknown kind must NOT fail the whole request");
    match req {
        Request::Focus { transition: Some(spec), .. } => {
            assert_eq!(spec.kind, TransitionKind::Cut, "unknown kind → cut");
            assert_eq!(spec.duration_ms, Some(200), "other fields survive");
        }
        other => panic!("expected Focus with transition, got {other:?}"),
    }

    // The warn hook: strict parse flags the unknown token, accepts known ones.
    assert_eq!(TransitionKind::from_wire("warp_zoom"), None);
    assert_eq!(
        TransitionKind::from_wire("crossfade"),
        Some(TransitionKind::Crossfade)
    );

    // Old client naming a still-known kind is unaffected.
    let ok: Request = serde_json::from_value(json!({
        "id": 31, "cmd": "focus", "app": "snake", "transition": {"kind": "dissolve"}
    }))
    .expect("known kind decodes");
    assert!(matches!(
        ok,
        Request::Focus { transition: Some(TransitionSpec { kind: TransitionKind::Dissolve, .. }), .. }
    ));

    // An unknown *field* on the spec is still rejected (deny_unknown_fields is
    // intact; it is orthogonal to the lenient kind value).
    let bad: Result<Request, _> = serde_json::from_value(json!({
        "id": 32, "cmd": "focus", "app": "snake",
        "transition": {"kind": "cut", "bogus": 1}
    }));
    assert!(bad.is_err(), "unknown spec field must still fail");
}

/// The `hello` capability body decodes into the strongly-typed
/// [`Capabilities`], and it advertises exactly the six kinds `cubed` supports
/// plus the overlay surface (plan R4.2a). This pins the capability contract the
/// SDKs read back.
#[test]
fn hello_capabilities_decode_typed() {
    let f = fixtures()
        .into_iter()
        .find(|f| f.name == "hello_response_with_capabilities")
        .expect("hello fixture present");
    let result = f.json.get("result").expect("result body").clone();
    let hello: HelloResult = serde_json::from_value(result).expect("HelloResult decode");
    let caps = hello.capabilities.expect("capabilities present");
    assert_eq!(caps, Capabilities::current());
    assert!(caps.overlay, "overlay surface advertised");
    assert_eq!(
        caps.transition_kinds.len(),
        6,
        "all six transition kinds advertised"
    );
}
