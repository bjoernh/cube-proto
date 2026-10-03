//! subscription helpers layered on the [`ControlClient`] transport.
//!
//! The daemon delivers a `subscribe` ack as an id-matched [`Response`] (carrying
//! a [`cube_proto::SubscribeResult`]) and then streams subsequent [`Event`]
//! lines out-of-band on the same connection. [`ControlClient::subscribe`] issues
//! the request and hands back the ack plus the live event stream;
//! [`ControlClient::subscribe_collect`] drives a *bounded* subscription and
//! drains exactly its batch (the daemon closes it with `subscription.ended`).

use std::time::Duration;

use cube_proto::{Event, Request, ResponseBody, SubscribeResult};
use tokio::sync::mpsc;

use crate::client::ControlClient;
use crate::error::ClientError;

/// Builder for a `subscribe` request. The `events` selector is required
/// (the class/event filter); everything else has a wire default — `app` is
/// `"*"` (all apps) and the remaining fields are omitted unless set.
#[derive(Debug, Clone)]
pub struct SubscribeOptions {
    events: Vec<String>,
    app: String,
    snapshot: Option<bool>,
    interval_ms: Option<u32>,
    max_events: Option<u32>,
    timeout_ms: Option<u64>,
}

impl SubscribeOptions {
    /// Subscribe to the given event/class selectors (e.g. `["lifecycle",
    /// "brightness"]`). `app` defaults to `"*"`; the remaining fields are unset.
    #[must_use]
    pub fn new(events: &[&str]) -> Self {
        Self {
            events: events.iter().map(|e| (*e).to_string()).collect(),
            app: "*".to_string(),
            snapshot: None,
            interval_ms: None,
            max_events: None,
            timeout_ms: None,
        }
    }

    /// Restrict the subscription to one app (default `"*"` — all apps).
    #[must_use]
    pub fn app(mut self, app: &str) -> Self {
        self.app = app.to_string();
        self
    }

    /// Request a baseline [`cube_proto::Snapshot`] in the ack.
    #[must_use]
    pub fn snapshot(mut self, snapshot: bool) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    /// Coalescing cadence for telemetry-class events, in milliseconds.
    #[must_use]
    pub fn interval_ms(mut self, interval_ms: u32) -> Self {
        self.interval_ms = Some(interval_ms);
        self
    }

    /// Bound the subscription to at most `max_events` events ( "Bounded
    /// subscriptions"); the daemon ends it with `subscription.ended`.
    #[must_use]
    pub fn max_events(mut self, max_events: u32) -> Self {
        self.max_events = Some(max_events);
        self
    }

    /// Bound the subscription to a daemon-side `timeout_ms` window.
    #[must_use]
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

impl ControlClient {
    /// Issue a `subscribe` and return the [`SubscribeResult`] ack together
    /// with the live [`Event`] stream. The ack comes back through the id-matched
    /// response path; subsequent events arrive on the returned receiver (the
    /// connection's single out-of-band event channel — taken here, so a later
    /// [`ControlClient::take_events`] yields `None`).
    pub async fn subscribe(
        &mut self,
        opts: SubscribeOptions,
    ) -> Result<(SubscribeResult, mpsc::Receiver<Event>), ClientError> {
        let id = self.alloc_id();
        let req = Request::Subscribe {
            id,
            app: opts.app,
            events: Some(opts.events),
            interval_ms: opts.interval_ms,
            snapshot: opts.snapshot,
            max_events: opts.max_events,
            timeout_ms: opts.timeout_ms,
        };
        // request surfaces a closed/EOF connection as Err rather than hanging.
        let resp = self.request(id, &req).await?;
        if !resp.ok {
            return Err(ClientError::Protocol(format!(
                "subscribe rejected: {:?}",
                resp.body
            )));
        }
        let ResponseBody::Result { result: Some(val) } = resp.body else {
            return Err(ClientError::Protocol(
                "subscribe ok response carried no result".to_string(),
            ));
        };
        let result: SubscribeResult =
            serde_json::from_value(val).map_err(|e| ClientError::Protocol(e.to_string()))?;
        let events = self
            .take_events()
            .ok_or_else(|| ClientError::Protocol("event stream already taken".to_string()))?;
        Ok((result, events))
    }

    /// Run a *bounded* subscription and collect its batch. Drains the event
    /// stream until a `subscription.ended` event arrives (pushed, then stop) or
    /// `window` elapses, whichever comes first. On a window timeout the events
    /// collected so far are returned (not an error). The ack itself is not
    /// returned — this mirrors `cube-mcp`'s `cube_watch` "collect a batch" shape.
    pub async fn subscribe_collect(
        &mut self,
        opts: SubscribeOptions,
        window: Duration,
    ) -> Result<Vec<Event>, ClientError> {
        let (_ack, mut events) = self.subscribe(opts).await?;
        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + window;
        // Loop ends when the stream closes (connection gone) or the window
        // elapses — `Ok(None)` / `Err(_)` fall out of the `while let` — and we
        // return what we collected rather than erroring.
        while let Ok(Some(ev)) = tokio::time::timeout_at(deadline, events.recv()).await {
            let ended = matches!(ev, Event::SubscriptionEnded { .. });
            collected.push(ev);
            if ended {
                break;
            }
        }
        Ok(collected)
    }
}
