//! Golden wire fixtures for the compositor surface. This target is the **cross-SDK contract artifact**: it writes one
//! canonical JSON file per request/response/event the compositor adds to the
//! wire into `tests/fixtures/wire/*.json`, then reads every file back and proves
//! it round-trips through the typed `cube_proto` model byte-for-byte. libcube
//! and cubekit consume these same files verbatim (R5/R6/R7), so the daemon and
//! both SDKs share one source of truth for the exact bytes on the socket.
//!
//! The fixtures are committed. A normal `cargo test -p cube-proto` run only
//! *verifies* the committed files against the in-code canonical table (a drift
//! between the two, or between a file and the enum/serde definitions, fails the
//! round-trip assertion) — it never writes into the source tree, so it works
//! under a read-only workspace (e.g. CI). To (re)generate the files after
//! changing the canonical table, run with `UPDATE_FIXTURES=1`:
//! `UPDATE_FIXTURES=1 cargo test -p cube-proto --test wire_fixtures`.

use std::path::PathBuf;

use serde_json::{Value, json};

use cube_proto::{
    Capabilities, Event, HelloResult, Request, Response, TransitionKind, TransitionSpec,
};

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
/// surface touches. Ordered request → response →
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

/// A fixture whose committed bytes intentionally do NOT re-encode to
/// themselves: the wire carries a token this build does not recognize, which the
/// typed model normalizes to a forward-compat catch-all. The file on disk holds
/// the *foreign* token a newer daemon actually emits; the consuming SDKs
/// (libcube R5, cubekit R6) mirror this file verbatim and must likewise decode
/// it to their own `Unknown`.
struct DecodeFixture {
    /// File stem under `tests/fixtures/wire/` (no extension).
    name: &'static str,
    kind: Kind,
    /// Bytes as they appear on the wire / in the committed file.
    on_wire: Value,
    /// What the typed model re-encodes them to (the normalized catch-all form).
    normalized: Value,
}

/// Forward-compat fixtures: an unknown wire token that must decode (never
/// hard-fail) and normalize to a catch-all variant.
fn decode_fixtures() -> Vec<DecodeFixture> {
    vec![
        // ── overlay.dismissed with a reason this build does not know ─────────
        // A future `cubed` reason token (here `evicted`) must decode to
        // `OverlayDismissReason::Unknown` (`#[serde(other)]`), which re-encodes
        // to `"unknown"`. Proves an added reason is a non-breaking, always
        // decodable change and both SDKs converge on the same Unknown.
        DecodeFixture {
            name: "overlay_dismissed_unknown_reason",
            kind: Kind::Event,
            on_wire: json!({"event": "overlay.dismissed", "layer": 7, "reason": "evicted"}),
            normalized: json!({"event": "overlay.dismissed", "layer": 7, "reason": "unknown"}),
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

/// Prove every committed fixture in `tests/fixtures/wire/<name>.json` (the
/// contract artifact) matches the in-code canonical table and round-trips
/// through the typed model. With `UPDATE_FIXTURES` set, the canonical table is
/// first (re)written to those files — an opt-in that keeps the default run
/// (including CI's read-only workspace mount) from ever touching the source
/// tree. Write and read live in one test so the two never race over the shared
/// files under the parallel test runner.
#[test]
fn wire_fixtures_write_and_roundtrip() {
    let dir = wire_dir();
    // Only regenerate the committed files when explicitly asked; a normal run
    // just verifies them (step 2) so it works read-only.
    let update = std::env::var_os("UPDATE_FIXTURES").is_some_and(|v| !v.is_empty());

    // 1. The canonical table must be self-consistent before it is trusted, so a
    //    broken shape fails here rather than being read/written. When updating,
    //    persist each fixture to disk.
    if update {
        std::fs::create_dir_all(&dir).expect("create fixtures/wire dir");
    }
    for f in fixtures() {
        assert_roundtrip(&f);
        if update {
            let mut path = dir.clone();
            path.push(format!("{}.json", f.name));
            let pretty = serde_json::to_string_pretty(&f.json).expect("pretty");
            std::fs::write(&path, format!("{pretty}\n")).expect("write fixture");
        }
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

/// Decode a `DecodeFixture`'s wire bytes through the typed model and re-encode,
/// asserting the result equals the *normalized* form (not the input). This is
/// the forward-compat contract: an unknown token is accepted and folded to the
/// catch-all variant.
fn assert_normalizes(f: &DecodeFixture, on_wire: &Value) {
    let back = match f.kind {
        Kind::Request => {
            let v: Request = serde_json::from_value(on_wire.clone())
                .unwrap_or_else(|e| panic!("{}: Request decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
        Kind::Response => {
            let v: Response = serde_json::from_value(on_wire.clone())
                .unwrap_or_else(|e| panic!("{}: Response decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
        Kind::Event => {
            let v: Event = serde_json::from_value(on_wire.clone())
                .unwrap_or_else(|e| panic!("{}: Event decode: {e}", f.name));
            serde_json::to_value(&v).unwrap()
        }
    };
    assert_eq!(
        back, f.normalized,
        "{}: did not normalize as expected",
        f.name
    );
}

/// Prove every committed forward-compat fixture (a foreign/unknown wire token)
/// decodes through the typed model rather than hard-failing, and folds to the
/// catch-all variant. Same `UPDATE_FIXTURES` gating as the byte-for-byte set so
/// the default run stays read-only. The committed files hold the *foreign*
/// token verbatim — the cross-SDK contract artifact both SDKs mirror.
#[test]
fn wire_forward_compat_fixtures_write_and_roundtrip() {
    let dir = wire_dir();
    let update = std::env::var_os("UPDATE_FIXTURES").is_some_and(|v| !v.is_empty());

    if update {
        std::fs::create_dir_all(&dir).expect("create fixtures/wire dir");
    }
    for f in decode_fixtures() {
        // The canonical table must be self-consistent before it is trusted.
        assert_normalizes(&f, &f.on_wire);
        if update {
            let mut path = dir.clone();
            path.push(format!("{}.json", f.name));
            let pretty = serde_json::to_string_pretty(&f.on_wire).expect("pretty");
            std::fs::write(&path, format!("{pretty}\n")).expect("write fixture");
        }
    }

    // Read back FROM DISK — the committed foreign token is the authoritative
    // contract the SDKs mirror; it must still decode and normalize.
    for f in decode_fixtures() {
        let mut path = dir.clone();
        path.push(format!("{}.json", f.name));
        let on_disk: Value = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", f.name)),
        )
        .unwrap_or_else(|e| panic!("parse {}: {e}", f.name));
        assert_eq!(
            on_disk, f.on_wire,
            "{}: file on disk drifted from the canonical table",
            f.name
        );
        assert_normalizes(&f, &on_disk);
    }
}

/// Lenient transition-kind decode: an unknown `kind` token
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
        Request::Focus {
            transition: Some(spec),
            ..
        } => {
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
        Request::Focus {
            transition: Some(TransitionSpec {
                kind: TransitionKind::Dissolve,
                ..
            }),
            ..
        }
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
/// plus the overlay surface. This pins the capability contract the
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
