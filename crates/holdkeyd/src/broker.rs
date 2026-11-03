//! `dev.holdkey.Broker1`: apps `Register` actions and receive `Pressed` / `Released`.
//!
//! Action ids are global across apps: `Register` rejects an id another app already owns.

use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};

use holdkey_core::{Action, Capability, HoldTracker, ShortcutBackend, ShortcutEvent};
use tokio::sync::{broadcast, mpsc::unbounded_channel};
use zbus::{Connection, fdo, object_server::SignalEmitter};

pub const PATH: &str = "/dev/holdkey/Broker1";
pub const NAME: &str = "dev.holdkey.Broker1";

/// Picks and builds the backend for a registration; an `Err` is shown to the caller verbatim.
pub type Factory = Arc<dyn Fn() -> Result<Box<dyn ShortcutBackend>, String> + Send + Sync>;

pub struct Broker {
    hub: broadcast::Sender<ShortcutEvent>,
    factory: Factory,
    capabilities: Vec<(String, Capability)>,
    /// app id -> its action ids, plus the running backend (kept alive here).
    apps: Mutex<HashMap<String, (Vec<String>, Box<dyn ShortcutBackend>)>>,
}

impl Broker {
    pub fn new(factory: Factory, capabilities: Vec<(String, Capability)>) -> Self {
        Self { hub: broadcast::channel(256).0, factory, capabilities, apps: Mutex::default() }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ShortcutEvent> {
        self.hub.subscribe()
    }

    pub fn hub(&self) -> broadcast::Sender<ShortcutEvent> {
        self.hub.clone()
    }
}

#[zbus::interface(name = "dev.holdkey.Broker1")]
impl Broker {
    /// `actions` are `(id, description, preferred_trigger)`; returns a session id.
    async fn register(&self, app_id: String, actions: Vec<(String, String, String)>) -> fdo::Result<String> {
        let bad = |m: String| fdo::Error::InvalidArgs(m);
        if app_id.is_empty() || actions.is_empty() {
            return Err(bad("app_id and at least one action are required".into()));
        }
        let actions: Vec<Action> = actions
            .into_iter()
            .map(|(id, description, preferred_trigger)| Action { id, description, preferred_trigger })
            .collect();
        HoldTracker::new(&actions).map_err(|e| bad(e.to_string()))?; // validates every trigger up front
        {
            let apps = self.apps.lock().unwrap();
            if apps.contains_key(&app_id) {
                return Err(fdo::Error::Failed(format!("{app_id} is already registered")));
            }
            let mine: HashSet<_> = actions.iter().map(|a| &a.id).collect();
            if let Some((other, _)) = apps.iter().find(|(_, (ids, _))| ids.iter().any(|i| mine.contains(i))) {
                return Err(bad(format!("an action id is already registered by {other}")));
            }
        }
        let mut backend = (self.factory)().map_err(fdo::Error::Failed)?;
        let (tx, mut rx) = unbounded_channel();
        let ids = actions.iter().map(|a| a.id.clone()).collect();
        backend.start(actions, tx).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let hub = self.hub.clone();
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let _ = hub.send(ev); // no subscribers is fine
            }
        });
        tracing::info!(%app_id, backend = backend.name(), "registered");
        self.apps.lock().unwrap().insert(app_id.clone(), (ids, backend));
        Ok(format!("{app_id}#{}", self.apps.lock().unwrap().len()))
    }

    /// Backend name -> (works, reason, fix_hint).
    #[zbus(property)]
    fn capabilities(&self) -> HashMap<String, (bool, String, String)> {
        self.capabilities.iter().map(|(n, c)| (n.clone(), (c.works, c.reason.clone(), c.fix_hint.clone()))).collect()
    }

    #[zbus(signal)]
    async fn pressed(emitter: &SignalEmitter<'_>, action_id: &str, ts_us: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn released(emitter: &SignalEmitter<'_>, action_id: &str, ts_us: u64) -> zbus::Result<()>;
}

/// Turn hub events into D-Bus signals until the hub closes.
pub async fn emit_signals(conn: Connection, mut rx: broadcast::Receiver<ShortcutEvent>) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(&conn, PATH)?;
    loop {
        match rx.recv().await {
            Ok(ShortcutEvent::Pressed { action_id, ts_us }) => Broker::pressed(&emitter, &action_id, ts_us).await?,
            Ok(ShortcutEvent::Released { action_id, ts_us }) => Broker::released(&emitter, &action_id, ts_us).await?,
            Err(broadcast::error::RecvError::Lagged(n)) => tracing::warn!("dropped {n} events (slow consumer)"),
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}
