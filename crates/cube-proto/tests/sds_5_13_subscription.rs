//! SDS v6 §5.13 — event subscription & telemetry: wire-surface round-trips.
//!
//! Pins the §5.13 protocol surface: the extended `subscribe` request (events /
//! interval_ms / snapshot / max_events / timeout_ms), the reshaped
//! `unsubscribe` (sub_id | all | legacy app), the new `subscriptions`
//! introspection verb, the new event payloads (`app.state`, `focus.changed`,
//! `app.stats`, `brightness.changed`, `subscription.ended`), the optional
//! `event_seq` on reliable events, `pause_causes` on `PerAppStatus`, the
//! `SubscribeResult`/`Snapshot` response model, and the protocol bump to 1.2.
//!
//! Shape conventions follow the spec (`cube-event-subscription.md`) with the
//! code-reality corrections from planning: stop reasons use the daemon's
//! `wire_reason()` vocabulary, `last_focused` is the monotonic u64 stamp from
//! `status`, and session `state` strings are the v6 lifecycle set.
//!
//! Naming convention: `<thing>_roundtrips_sds_5_13` per the house test plan;
//! helpers mirror `wire_roundtrip.rs`.

use serde_json::{Value, json};

use cube_proto::{
    Event, PauseCauses, PowerState, Request, Response, ResponseBody, Snapshot, SubscribeResult,
    SubscriptionEndReason,
};

// ─────────────────────────────────────────────────────────────────────────────
// helpers (mirroring wire_roundtrip.rs)
// ─────────────────────────────────────────────────────────────────────────────

fn roundtrip_request(j: Value) -> Request {
    let req: Request = serde_json::from_value(j.clone()).expect("Request decode");
    let back = serde_json::to_value(&req).expect("Request encode");
    assert_eq!(back, j, "Request did not round-trip");
    req
}

fn roundtrip_event(j: Value) -> Event {
    let ev: Event = serde_json::from_value(j.clone()).expect("Event decode");
    let back = serde_json::to_value(&ev).expect("Event encode");
    assert_eq!(back, j, "Event did not round-trip");
    ev
}

// ─────────────────────────────────────────────────────────────────────────────
// subscribe — extended request (§5.13 "The commands")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn subscribe_full_form_roundtrips_sds_5_13() {
    // The spec's worked example, open-ended form (no bounds).
    let j = json!({
        "id": 17,
        "cmd": "subscribe",
        "events": ["lifecycle", "telemetry", "brightness"],
        "app": "*",
        "interval_ms": 1000,
        "snapshot": true
    });
    let req = roundtrip_request(j);
    match req {
        Request::Subscribe { id, app, events, interval_ms, snapshot, max_events, timeout_ms } => {
            assert_eq!(id, 17);
            assert_eq!(app, "*");
            assert_eq!(
                events.as_deref(),
                Some(&["lifecycle".to_owned(), "telemetry".to_owned(), "brightness".to_owned()][..])
            );
            assert_eq!(interval_ms, Some(1000));
            assert_eq!(snapshot, Some(true));
            assert_eq!(max_events, None);
            assert_eq!(timeout_ms, None);
        }
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

#[test]
fn subscribe_bounded_form_roundtrips_sds_5_13() {
    // Bounded subscription (the MCP cube_watch mapping).
    let j = json!({
        "id": 18,
        "cmd": "subscribe",
        "events": ["param.changed", "params.changed"],
        "app": "snake",
        "max_events": 16,
        "timeout_ms": 3000
    });
    let req = roundtrip_request(j);
    match req {
        Request::Subscribe { max_events, timeout_ms, snapshot, interval_ms, .. } => {
            assert_eq!(max_events, Some(16));
            assert_eq!(timeout_ms, Some(3000));
            assert_eq!(snapshot, None);
            assert_eq!(interval_ms, None);
        }
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

#[test]
fn subscribe_legacy_param_form_roundtrips_unchanged_sds_5_13() {
    // Back-compat: today's param-watch shape must round-trip byte-identically —
    // none of the new optional fields may appear on the wire when unset.
    let j = json!({
        "id": 4,
        "cmd": "subscribe",
        "app": "snake"
    });
    let req = roundtrip_request(j);
    match req {
        Request::Subscribe { id, app, events, interval_ms, snapshot, max_events, timeout_ms } => {
            assert_eq!(id, 4);
            assert_eq!(app, "snake");
            assert_eq!(events, None);
            assert_eq!(interval_ms, None);
            assert_eq!(snapshot, None);
            assert_eq!(max_events, None);
            assert_eq!(timeout_ms, None);
        }
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

#[test]
fn subscribe_without_app_defaults_to_star_sds_5_13() {
    // `app` may be omitted by new clients subscribing to global classes only;
    // it defaults to "*" (parse-only: serialization always carries `app`).
    let j = json!({
        "id": 19,
        "cmd": "subscribe",
        "events": ["power", "brightness"]
    });
    let req: Request = serde_json::from_value(j).expect("Request decode");
    match req {
        Request::Subscribe { app, .. } => assert_eq!(app, "*"),
        other => panic!("expected Subscribe, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// unsubscribe / subscriptions (§5.13 "The commands")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn unsubscribe_by_sub_id_roundtrips_sds_5_13() {
    let j = json!({ "id": 18, "cmd": "unsubscribe", "sub_id": "s-3" });
    let req = roundtrip_request(j);
    match req {
        Request::Unsubscribe { id, sub_id, app, all } => {
            assert_eq!(id, 18);
            assert_eq!(sub_id.as_deref(), Some("s-3"));
            assert_eq!(app, None);
            assert!(!all);
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn unsubscribe_all_roundtrips_sds_5_13() {
    let j = json!({ "id": 18, "cmd": "unsubscribe", "all": true });
    let req = roundtrip_request(j);
    match req {
        Request::Unsubscribe { sub_id, app, all, .. } => {
            assert_eq!(sub_id, None);
            assert_eq!(app, None);
            assert!(all);
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn unsubscribe_legacy_app_form_roundtrips_unchanged_sds_5_13() {
    // Back-compat: the existing `{id, app}` unsubscribe keeps its exact shape.
    let j = json!({ "id": 9, "cmd": "unsubscribe", "app": "snake" });
    let req = roundtrip_request(j);
    match req {
        Request::Unsubscribe { app, sub_id, all, .. } => {
            assert_eq!(app.as_deref(), Some("snake"));
            assert_eq!(sub_id, None);
            assert!(!all);
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn subscriptions_request_roundtrips_sds_5_13() {
    let j = json!({ "id": 19, "cmd": "subscriptions" });
    let req = roundtrip_request(j);
    assert!(matches!(req, Request::Subscriptions { id: 19 }));
}

// ─────────────────────────────────────────────────────────────────────────────
// lifecycle events (§5.13 "Event payloads — lifecycle")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn app_state_event_with_cause_roundtrips_sds_5_13() {
    let j = json!({
        "event": "app.state",
        "app": "snake",
        "from": "focused",
        "to": "paused",
        "cause": { "focus_lost": true, "blanked": false },
        "event_seq": 5015
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppState { app, from, to, cause, event_seq } => {
            assert_eq!(app, "snake");
            assert_eq!(from, "focused");
            assert_eq!(to, "paused");
            let cause = cause.expect("cause present on focus/blank pauses");
            assert!(cause.focus_lost);
            assert!(!cause.blanked);
            assert_eq!(event_seq, Some(5015));
        }
        other => panic!("expected AppState, got {other:?}"),
    }
}

#[test]
fn app_state_event_without_cause_omits_field_sds_5_13() {
    // `cause` is present only on focus/blank pause edges; other transitions
    // (e.g. paused → stopping) omit it entirely on the wire.
    let j = json!({
        "event": "app.state",
        "app": "snake",
        "from": "paused",
        "to": "stopping",
        "event_seq": 5021
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppState { cause, .. } => assert_eq!(cause, None),
        other => panic!("expected AppState, got {other:?}"),
    }
}

#[test]
fn focus_changed_event_roundtrips_sds_5_13() {
    let j = json!({
        "event": "focus.changed",
        "focused_app": "snake",
        "previous": "launcher",
        "event_seq": 5016
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::FocusChanged { focused_app, previous, event_seq } => {
            assert_eq!(focused_app.as_deref(), Some("snake"));
            assert_eq!(previous.as_deref(), Some("launcher"));
            assert_eq!(event_seq, Some(5016));
        }
        other => panic!("expected FocusChanged, got {other:?}"),
    }
}

#[test]
fn focus_changed_event_omits_absent_apps_sds_5_13() {
    // First focus after boot has no `previous`; both fields are optional and
    // omitted when absent.
    let j = json!({
        "event": "focus.changed",
        "focused_app": "launcher",
        "event_seq": 1
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::FocusChanged { focused_app, previous, .. } => {
            assert_eq!(focused_app.as_deref(), Some("launcher"));
            assert_eq!(previous, None);
        }
        other => panic!("expected FocusChanged, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// event_seq on existing reliable events (additive, omitted when None)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn app_started_v5_shape_unchanged_sds_5_13() {
    // The v5 wire shape must survive byte-identically when event_seq is None.
    let j = json!({ "event": "app.started", "app": "snake" });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppStarted { app, event_seq } => {
            assert_eq!(app, "snake");
            assert_eq!(event_seq, None);
        }
        other => panic!("expected AppStarted, got {other:?}"),
    }
}

#[test]
fn app_started_with_event_seq_roundtrips_sds_5_13() {
    let j = json!({ "event": "app.started", "app": "snake", "event_seq": 5013 });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppStarted { event_seq, .. } => assert_eq!(event_seq, Some(5013)),
        other => panic!("expected AppStarted, got {other:?}"),
    }
}

#[test]
fn app_stopped_with_reason_and_event_seq_roundtrips_sds_5_13() {
    // Reason strings are the daemon's wire_reason() vocabulary; "evicted" is
    // the LRU-eviction case §5.13 needs to distinguish.
    let j = json!({
        "event": "app.stopped",
        "app": "snake",
        "reason": "evicted",
        "event_seq": 5014
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppStopped { app, reason, event_seq } => {
            assert_eq!(app, "snake");
            assert_eq!(reason.as_deref(), Some("evicted"));
            assert_eq!(event_seq, Some(5014));
        }
        other => panic!("expected AppStopped, got {other:?}"),
    }
}

#[test]
fn power_state_with_event_seq_roundtrips_sds_5_13() {
    let j = json!({ "event": "power.state", "state": "blanked", "event_seq": 5017 });
    let ev = roundtrip_event(j);
    match ev {
        Event::PowerState { state, event_seq } => {
            assert_eq!(state, PowerState::Blanked);
            assert_eq!(event_seq, Some(5017));
        }
        other => panic!("expected PowerState, got {other:?}"),
    }
}

#[test]
fn power_state_v5_shape_unchanged_sds_5_13() {
    let j = json!({ "event": "power.state", "state": "active" });
    let ev = roundtrip_event(j);
    match ev {
        Event::PowerState { event_seq, .. } => assert_eq!(event_seq, None),
        other => panic!("expected PowerState, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// telemetry / brightness / subscription.ended (§5.13 "Event payloads")
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn app_stats_event_roundtrips_sds_5_13() {
    // fps values are chosen binary-exact so the f32 JSON round-trip is stable.
    let j = json!({
        "event": "app.stats",
        "app": "snake",
        "fps": 59.5,
        "drops": 12,
        "drops_delta": 0,
        "frame_seq": 218_833
    });
    let ev = roundtrip_event(j);
    match ev {
        Event::AppStats { app, fps, drops, drops_delta, frame_seq } => {
            assert_eq!(app, "snake");
            assert!((fps - 59.5).abs() < f32::EPSILON);
            assert_eq!(drops, 12);
            assert_eq!(drops_delta, 0);
            assert_eq!(frame_seq, 218_833);
        }
        other => panic!("expected AppStats, got {other:?}"),
    }
}

#[test]
fn brightness_changed_event_roundtrips_sds_5_13() {
    // Coalescible class: carries no event_seq, ever.
    let j = json!({ "event": "brightness.changed", "value": 200 });
    let ev = roundtrip_event(j);
    match ev {
        Event::BrightnessChanged { value } => assert_eq!(value, 200),
        other => panic!("expected BrightnessChanged, got {other:?}"),
    }
}

#[test]
fn subscription_ended_roundtrips_all_reasons_sds_5_13() {
    for (reason_str, reason) in [
        ("max_events", SubscriptionEndReason::MaxEvents),
        ("timeout", SubscriptionEndReason::Timeout),
        ("overflow", SubscriptionEndReason::Overflow),
    ] {
        let j = json!({
            "event": "subscription.ended",
            "sub_id": "s-3",
            "reason": reason_str
        });
        let ev = roundtrip_event(j);
        match ev {
            Event::SubscriptionEnded { sub_id, reason: r } => {
                assert_eq!(sub_id, "s-3");
                assert_eq!(r, reason);
            }
            other => panic!("expected SubscriptionEnded, got {other:?}"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// pause_causes on PerAppStatus (§5.13 prerequisite 3 — additive)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn per_app_status_without_pause_causes_still_parses_sds_5_13() {
    // The pre-§5.13 session shape (no pause_causes) must keep deserializing,
    // and serializing a None must omit the field (old clients ignore nothing).
    let j = json!({
        "pid": 4242,
        "systemd_unit": "cube-app@picture.service",
        "connection_state": "running",
        "state": "focused",
        "last_focused": 7,
        "last_present_seq": 1234,
        "inflight_buffers": 1,
        "fps_submitted": 60.0,
        "fps_displayed": 59.5,
        "dropped_frames": {
            "displayed": 100, "replaced": 1, "dropped": 0,
            "focus_lost": 2, "blanked": 0
        },
        "parameter_seq": 7,
        "input_events_forwarded": 99,
        "present_to_displayed_latency_mean_ms": 12.5,
        "present_to_displayed_latency_p95_ms": 18.75,
        "remote_sender_drops": 0,
        "total_remote_sender_dropped": 0,
        "last_seq": 0
    });
    let status: cube_proto::PerAppStatus =
        serde_json::from_value(j.clone()).expect("PerAppStatus decode");
    assert_eq!(status.pause_causes, None);
    let back = serde_json::to_value(&status).expect("PerAppStatus encode");
    assert_eq!(back, j, "None pause_causes must stay off the wire");
}

#[test]
fn per_app_status_with_pause_causes_roundtrips_sds_5_13() {
    let j = json!({
        "pid": 4242,
        "systemd_unit": "cube-app@picture.service",
        "connection_state": "running",
        "state": "paused",
        "last_focused": 7,
        "last_present_seq": 1234,
        "inflight_buffers": 1,
        "fps_submitted": 60.0,
        "fps_displayed": 59.5,
        "dropped_frames": {
            "displayed": 100, "replaced": 1, "dropped": 0,
            "focus_lost": 2, "blanked": 0
        },
        "parameter_seq": 7,
        "input_events_forwarded": 99,
        "present_to_displayed_latency_mean_ms": 12.5,
        "present_to_displayed_latency_p95_ms": 18.75,
        "remote_sender_drops": 0,
        "total_remote_sender_dropped": 0,
        "last_seq": 0,
        "pause_causes": { "focus_lost": true, "blanked": false }
    });
    let status: cube_proto::PerAppStatus =
        serde_json::from_value(j.clone()).expect("PerAppStatus decode");
    let causes = status.pause_causes.expect("pause_causes present");
    assert!(causes.focus_lost);
    assert!(!causes.blanked);
    let back = serde_json::to_value(&status).expect("PerAppStatus encode");
    assert_eq!(back, j);
}

// ─────────────────────────────────────────────────────────────────────────────
// SubscribeResult + Snapshot (§5.13 "Snapshot semantics" — normative schema)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn subscribe_result_minimal_roundtrips_sds_5_13() {
    // Without snapshot:true the result carries sub_id + the seq baseline only.
    let j = json!({ "sub_id": "s-3", "event_seq": 5012 });
    let r: SubscribeResult = serde_json::from_value(j.clone()).expect("SubscribeResult decode");
    assert_eq!(r.sub_id, "s-3");
    assert_eq!(r.event_seq, 5012);
    assert!(r.snapshot.is_none());
    let back = serde_json::to_value(&r).expect("SubscribeResult encode");
    assert_eq!(back, j, "absent snapshot must stay off the wire");
}

#[test]
fn subscribe_result_with_full_snapshot_roundtrips_sds_5_13() {
    // The spec's normative snapshot schema, with code-reality types:
    // last_focused is the monotonic u64 stamp `status` already reports.
    let j = json!({
        "sub_id": "s-3",
        "event_seq": 5012,
        "snapshot": {
            "lifecycle": {
                "max_resident": 4,
                "resident_apps": 2,
                "focused_app": "snake",
                "sessions": [
                    {
                        "app": "snake", "state": "focused", "pid": 5123,
                        "last_focused": 9,
                        "pause_causes": { "focus_lost": false, "blanked": false }
                    },
                    {
                        "app": "launcher", "state": "paused", "pid": 4001,
                        "last_focused": 8,
                        "pause_causes": { "focus_lost": true, "blanked": false }
                    }
                ]
            },
            "telemetry": { "app": "snake", "fps": 59.5, "drops": 12, "frame_seq": 218_833 },
            "brightness": { "value": 200 },
            "power": { "state": "active" }
        }
    });
    let r: SubscribeResult = serde_json::from_value(j.clone()).expect("SubscribeResult decode");
    let snap = r.snapshot.as_ref().expect("snapshot present");
    let lifecycle = snap.lifecycle.as_ref().expect("lifecycle section");
    assert_eq!(lifecycle.max_resident, 4);
    assert_eq!(lifecycle.resident_apps, 2);
    assert_eq!(lifecycle.focused_app.as_deref(), Some("snake"));
    assert_eq!(lifecycle.sessions.len(), 2);
    assert_eq!(lifecycle.sessions[0].app, "snake");
    assert_eq!(lifecycle.sessions[0].state, "focused");
    assert_eq!(lifecycle.sessions[0].pid, Some(5123));
    assert!(!lifecycle.sessions[0].pause_causes.focus_lost);
    assert!(lifecycle.sessions[1].pause_causes.focus_lost);
    let telemetry = snap.telemetry.as_ref().expect("telemetry section");
    assert_eq!(telemetry.app, "snake");
    assert_eq!(telemetry.drops, 12);
    assert_eq!(snap.brightness.as_ref().expect("brightness section").value, 200);
    assert_eq!(snap.power.as_ref().expect("power section").state, PowerState::Active);
    let back = serde_json::to_value(&r).expect("SubscribeResult encode");
    assert_eq!(back, j);
}

#[test]
fn snapshot_omits_unsubscribed_sections_sds_5_13() {
    // Only subscribed classes appear: a power+brightness-only snapshot has
    // exactly those keys on the wire.
    let snap = Snapshot {
        lifecycle: None,
        telemetry: None,
        brightness: Some(cube_proto::BrightnessSnapshot { value: 128 }),
        power: Some(cube_proto::PowerSnapshot { state: PowerState::Blanked }),
    };
    let v = serde_json::to_value(&snap).expect("Snapshot encode");
    assert_eq!(v, json!({ "brightness": { "value": 128 }, "power": { "state": "blanked" } }));
}

#[test]
fn subscribe_ok_response_with_result_roundtrips_sds_5_13() {
    // The full response envelope a §5.13 subscriber sees on the wire.
    let j = json!({
        "id": 17,
        "ok": true,
        "result": { "sub_id": "s-1", "event_seq": 0 }
    });
    let resp: Response = serde_json::from_value(j.clone()).expect("Response decode");
    let back = serde_json::to_value(&resp).expect("Response encode");
    assert_eq!(back, j);
    match resp.body {
        ResponseBody::Result { result: Some(result) } => {
            let r: SubscribeResult = serde_json::from_value(result).expect("SubscribeResult");
            assert_eq!(r.sub_id, "s-1");
            assert_eq!(r.event_seq, 0);
        }
        other => panic!("expected Result body, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PauseCauses standalone + protocol bump
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn pause_causes_roundtrips_sds_5_13() {
    let j = json!({ "focus_lost": true, "blanked": true });
    let p: PauseCauses = serde_json::from_value(j.clone()).expect("PauseCauses decode");
    assert!(p.focus_lost);
    assert!(p.blanked);
    assert_eq!(serde_json::to_value(&p).unwrap(), j);
}

#[test]
fn protocol_version_bumped_to_1_2_sds_5_13() {
    // §5.13 "Compatibility and migration": the admin protocol version is
    // bumped so clients (and the companion gateway's hard gate) can
    // feature-detect the subscription surface.
    assert_eq!(cube_proto::PROTOCOL_MAJOR, 1);
    assert_eq!(cube_proto::PROTOCOL_MINOR, 2);
    assert_eq!(cube_proto::PROTOCOL_VERSION, "1.2");
}
