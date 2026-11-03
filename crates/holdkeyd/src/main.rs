use std::sync::Arc;

use holdkey_core::ShortcutBackend;
use holdkeyd::{broker::{self, Broker}, evdev, portal::PortalBackend, socket, store::Store};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let reg = tracing_subscriber::registry().with(filter);
    match tracing_journald::layer() {
        Ok(j) => reg.with(j).init(),
        Err(_) => reg.with(tracing_subscriber::fmt::layer()).init(), // not under systemd
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let store_path = Store::default_path().ok_or("cannot locate $HOME or $XDG_STATE_HOME")?;

    let portal = PortalBackend::probe(store_path.clone()).await;
    let portal_cap = portal.capability();
    let evdev_cap = evdev::capability();
    let caps = vec![
        ("portal".to_string(), portal_cap.clone()),
        ("evdev".to_string(), evdev_cap.clone()),
        ("hyprland".to_string(), holdkey_core::hyprland::capability()),
    ];
    for (name, c) in &caps {
        tracing::info!(backend = %name, works = c.works, reason = %c.reason, "capability");
    }

    // Portal first (keeps the compositor in charge of keys); evdev when there is no portal.
    // HOLDKEYD_BACKEND=evdev forces the evdev path (for testing it where a portal exists).
    let force_evdev = std::env::var("HOLDKEYD_BACKEND").is_ok_and(|v| v == "evdev");
    let factory: broker::Factory = Arc::new(move || {
        if portal_cap.works && !force_evdev {
            Ok(Box::new(PortalBackend::new(portal_cap.clone(), store_path.clone())) as Box<dyn ShortcutBackend>)
        } else if evdev_cap.works {
            Ok(Box::new(evdev::EvdevBackend::new()))
        } else {
            Err(format!("no usable backend. portal: {} ({}). evdev: {} ({})",
                portal_cap.reason, portal_cap.fix_hint, evdev_cap.reason, evdev_cap.fix_hint))
        }
    });
    let broker = Broker::new(factory, caps);
    let (hub, rx) = (broker.hub(), broker.subscribe());

    let conn = zbus::connection::Builder::session()
        .name(broker::NAME)
        .serve_at(broker::PATH, broker)
        .build()
        .await?;
    tokio::spawn(broker::emit_signals(conn.clone(), rx));
    if let Some(path) = socket::default_path() {
        tokio::spawn(async move {
            if let Err(e) = socket::serve(path, hub).await {
                tracing::error!("watch socket stopped: {e}");
            }
        });
    }
    tracing::info!("holdkeyd ready on {}", broker::NAME);

    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
    if let Some(p) = socket::default_path() {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}
