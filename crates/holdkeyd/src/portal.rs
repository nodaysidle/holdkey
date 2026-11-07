//! GlobalShortcuts portal backend (KDE, GNOME >= 48, Hyprland via xdph).

use std::path::PathBuf;

use ashpd::desktop::{
    Session,
    global_shortcuts::{GlobalShortcuts, NewShortcut},
};
use futures_util::StreamExt;
// The portal's own timestamp is unusable: xdph sends seconds, ashpd reads it as ms. Stamp on arrival.
use holdkey_core::{Action, Capability, Error, Result, ShortcutBackend, ShortcutEvent, now_us};
use tokio::sync::mpsc::UnboundedSender;

use crate::store::{Approved, Store};

#[derive(Debug, thiserror::Error)]
pub enum PortalError {
    #[error("the user cancelled the shortcut dialog")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

/// What the portal reports as bound: `(action id, trigger description)`.
pub type Bound = Vec<(String, String)>;

/// Seam between the reconcile logic and the real portal, so cancellation can be tested with a stub.
#[allow(async_fn_in_trait)]
pub trait Portal {
    async fn bind(&self, actions: &[Action]) -> std::result::Result<Bound, PortalError>;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub bound: Bound,
    /// Actions whose approval dialog was cancelled or failed; they stay unbound, nothing else does.
    pub unbound: Vec<String>,
}

/// Bind in two phases so a cancelled dialog can only affect *new* shortcuts:
/// 1. actions already approved (per `store`) are re-bound, which the portal does without a prompt;
/// 2. the rest go in a second request, whose cancellation is recorded and otherwise ignored.
/// Approvals are only ever added to the store, never removed.
pub async fn reconcile<P: Portal>(portal: &P, store: &Store, actions: &[Action]) -> Result<Outcome> {
    let io = |e: std::io::Error| Error::Backend(format!("approval store: {e}"));
    let approved = store.load().map_err(io)?;
    let (known, fresh): (Vec<_>, Vec<_>) = actions
        .iter()
        .cloned()
        .partition(|a| approved.get(&a.id).is_some_and(|p| p.description == a.description));

    let mut out = Outcome::default();
    for (batch, label) in [(known, "approved"), (fresh, "new")] {
        if batch.is_empty() {
            continue;
        }
        match portal.bind(&batch).await {
            Ok(bound) => out.bound.extend(bound),
            Err(e) => {
                tracing::warn!("binding {label} shortcuts failed, keeping existing bindings: {e}");
                out.unbound.extend(batch.into_iter().map(|a| a.id));
            }
        }
    }
    let descriptions: std::collections::HashMap<_, _> = actions.iter().map(|a| (&a.id, &a.description)).collect();
    store
        .merge(out.bound.iter().map(|(id, trigger)| {
            let description = descriptions.get(id).map(|d| (*d).clone()).unwrap_or_default();
            (id.clone(), Approved { description, trigger: trigger.clone() })
        }))
        .map_err(io)?;
    Ok(out)
}

/// kbd trigger (`Ctrl+Space`) to the XDG shortcuts spec (`CTRL+space`).
pub fn to_xdg_trigger(kbd: &str) -> String {
    kbd.split('+')
        .map(|p| match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "CTRL".into(),
            "shift" => "SHIFT".into(),
            "alt" => "ALT".into(),
            "super" | "meta" | "logo" => "LOGO".into(),
            "space" => "space".into(),
            _ if p.len() == 1 => p.to_ascii_lowercase(),
            _ => p.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

struct AshpdPortal<'a> {
    gs: &'a GlobalShortcuts,
    session: &'a Session<GlobalShortcuts>,
}

impl Portal for AshpdPortal<'_> {
    async fn bind(&self, actions: &[Action]) -> std::result::Result<Bound, PortalError> {
        let shortcuts: Vec<_> = actions
            .iter()
            .map(|a| NewShortcut::new(&a.id, &a.description).preferred_trigger(to_xdg_trigger(&a.preferred_trigger).as_str()))
            .collect();
        let map = |e: ashpd::Error| match e {
            ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled) => PortalError::Cancelled,
            e => PortalError::Other(e.to_string()),
        };
        let req = self.gs.bind_shortcuts(self.session, &shortcuts, None, Default::default()).await.map_err(map)?;
        let resp = req.response().map_err(map)?;
        Ok(resp.shortcuts().iter().map(|s| (s.id().to_string(), s.trigger_description().to_string())).collect())
    }
}

pub struct PortalBackend {
    capability: Capability,
    store_path: PathBuf,
}

/// Portals key permissions on an app ID and refuse host processes without one. The ID must have a
/// matching `<id>.desktop` file (the PKGBUILD installs it). All client apps share this one ID.
pub const PORTAL_APP_ID: &str = "dev.holdkey.Broker";

impl PortalBackend {
    pub fn new(capability: Capability, store_path: PathBuf) -> Self {
        Self { capability, store_path }
    }

    /// Ask the session bus whether the portal and its GlobalShortcuts interface exist.
    pub async fn probe(store_path: PathBuf) -> Self {
        let capability = match GlobalShortcuts::new().await {
            Ok(gs) => Capability::ok(format!("GlobalShortcuts portal v{}", gs.version())),
            Err(e) => Capability::broken(
                format!("no GlobalShortcuts portal: {e}"),
                "install/upgrade your xdg-desktop-portal backend (GNOME needs xdg-desktop-portal-gnome >= 48), or use the evdev fallback",
            ),
        };
        Self::new(capability, store_path)
    }
}

impl ShortcutBackend for PortalBackend {
    fn name(&self) -> &'static str {
        "portal"
    }
    fn capability(&self) -> Capability {
        self.capability.clone()
    }
    fn start(&mut self, actions: Vec<Action>, tx: UnboundedSender<ShortcutEvent>) -> Result<()> {
        let store = Store::new(&self.store_path);
        tokio::spawn(async move {
            if let Err(e) = run(actions, store, tx).await {
                tracing::error!("portal backend stopped: {e}");
            }
        });
        Ok(())
    }
}

async fn run(actions: Vec<Action>, store: Store, tx: UnboundedSender<ShortcutEvent>) -> Result<()> {
    let be = |e: ashpd::Error| Error::Backend(e.to_string());
    // A private connection per app: the host registry ties one app ID to one D-Bus connection.
    let conn = ashpd::zbus::connection::Builder::session()
        .build()
        .await
        .map_err(|e| Error::Backend(e.to_string()))?;
    let id = ashpd::AppID::try_from(PORTAL_APP_ID).map_err(|e| Error::Backend(e.to_string()))?;
    ashpd::register_host_app_with_connection(conn.clone(), id).await.map_err(be)?;
    let gs = GlobalShortcuts::with_connection(conn).await.map_err(be)?;
    let session = gs.create_session(Default::default()).await.map_err(be)?;
    // Subscribe before binding so no early press is missed.
    let mut activated = gs.receive_activated().await.map_err(be)?;
    let mut deactivated = gs.receive_deactivated().await.map_err(be)?;

    let outcome = reconcile(&AshpdPortal { gs: &gs, session: &session }, &store, &actions).await?;
    tracing::info!(bound = ?outcome.bound, unbound = ?outcome.unbound, "portal shortcuts ready");

    loop {
        let ev = tokio::select! {
            Some(a) = activated.next() => ShortcutEvent::Pressed { action_id: a.shortcut_id().into(), ts_us: now_us() },
            Some(d) = deactivated.next() => ShortcutEvent::Released { action_id: d.shortcut_id().into(), ts_us: now_us() },
            else => break,
        };
        if tx.send(ev).is_err() {
            break;
        }
    }
    let _ = session.close().await;
    Ok(())
}
