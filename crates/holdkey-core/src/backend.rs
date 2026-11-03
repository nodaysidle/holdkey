use std::time::{SystemTime, UNIX_EPOCH};

use kbd::key::Key;
use kbd::key_state::KeyTransition;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{Edge, HoldTracker, Result};

/// One bindable action as registered by an app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub description: String,
    /// kbd trigger syntax, e.g. `Ctrl+Space` or `F13`.
    pub preferred_trigger: String,
}

/// What the backends send to the broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum ShortcutEvent {
    Pressed { action_id: String, ts_us: u64 },
    Released { action_id: String, ts_us: u64 },
}

impl ShortcutEvent {
    pub fn from_edge(edge: Edge, ts_us: u64) -> Self {
        match edge {
            Edge::Pressed(action_id) => Self::Pressed { action_id, ts_us },
            Edge::Released(action_id) => Self::Released { action_id, ts_us },
        }
    }
}

/// Whether a backend can work on this machine, and if not why and what to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub works: bool,
    pub reason: String,
    pub fix_hint: String,
}

impl Capability {
    pub fn ok(reason: impl Into<String>) -> Self {
        Self { works: true, reason: reason.into(), fix_hint: String::new() }
    }
    pub fn broken(reason: impl Into<String>, fix_hint: impl Into<String>) -> Self {
        Self { works: false, reason: reason.into(), fix_hint: fix_hint.into() }
    }
}

pub fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_micros() as u64)
}

/// A way to get global press/release events. Dyn-compatible: `start` spawns
/// whatever tasks it needs and pushes events into `tx`.
pub trait ShortcutBackend: Send {
    fn name(&self) -> &'static str;
    fn capability(&self) -> Capability;
    fn start(&mut self, actions: Vec<Action>, tx: UnboundedSender<ShortcutEvent>) -> Result<()>;
}

/// Test backend: replays a scripted key sequence through a [`HoldTracker`].
pub struct MockBackend {
    script: Vec<(Key, KeyTransition)>,
}

impl MockBackend {
    pub fn new(script: Vec<(Key, KeyTransition)>) -> Self {
        Self { script }
    }
}

impl ShortcutBackend for MockBackend {
    fn name(&self) -> &'static str {
        "mock"
    }
    fn capability(&self) -> Capability {
        Capability::ok("scripted events")
    }
    fn start(&mut self, actions: Vec<Action>, tx: UnboundedSender<ShortcutEvent>) -> Result<()> {
        let mut tracker = HoldTracker::new(&actions)?;
        for (key, tr) in self.script.drain(..) {
            for edge in tracker.feed(key, tr) {
                // Receiver gone means the broker shut down; nothing left to deliver.
                if tx.send(ShortcutEvent::from_edge(edge, now_us())).is_err() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}
