//! A cancelled re-prompt must not unbind keys that were already approved (obs-rust #2636).

use std::sync::Mutex;

use holdkey_core::Action;
use holdkeyd::portal::{Bound, Portal, PortalError, reconcile};
use holdkeyd::store::Store;

fn act(id: &str, desc: &str) -> Action {
    Action { id: id.into(), description: desc.into(), preferred_trigger: "F13".into() }
}

/// Approves everything, or cancels any batch containing an id in `cancel`. Records each batch.
struct Stub {
    cancel: Vec<&'static str>,
    batches: Mutex<Vec<Vec<String>>>,
}

impl Portal for Stub {
    async fn bind(&self, actions: &[Action]) -> Result<Bound, PortalError> {
        self.batches.lock().unwrap().push(actions.iter().map(|a| a.id.clone()).collect());
        if actions.iter().any(|a| self.cancel.contains(&a.id.as_str())) {
            return Err(PortalError::Cancelled);
        }
        Ok(actions.iter().map(|a| (a.id.clone(), "F13".into())).collect())
    }
}

fn stub(cancel: &[&'static str]) -> Stub {
    Stub { cancel: cancel.to_vec(), batches: Mutex::default() }
}

#[tokio::test]
async fn cancelled_reprompt_keeps_existing_bindings() {
    let dir = std::env::temp_dir().join(format!("holdkey-approvals-{}", std::process::id()));
    let store = Store::new(dir.join("portal-sessions.json"));

    let first = reconcile(&stub(&[]), &store, &[act("ptt", "push to talk")]).await.unwrap();
    assert_eq!(first.bound, vec![("ptt".into(), "F13".into())]);

    // Second run adds "mute"; the user cancels its dialog.
    let p = stub(&["mute"]);
    let second = reconcile(&p, &store, &[act("ptt", "push to talk"), act("mute", "mute")]).await.unwrap();
    assert_eq!(second.bound, vec![("ptt".into(), "F13".into())], "ptt must stay bound");
    assert_eq!(second.unbound, vec!["mute".to_string()]);
    assert_eq!(*p.batches.lock().unwrap(), vec![vec!["ptt".to_string()], vec!["mute".to_string()]]);

    let saved = store.load().unwrap();
    assert!(saved.contains_key("ptt") && !saved.contains_key("mute"), "approvals only grow: {saved:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn changed_description_is_rebound_as_new() {
    let dir = std::env::temp_dir().join(format!("holdkey-approvals2-{}", std::process::id()));
    let store = Store::new(dir.join("s.json"));
    reconcile(&stub(&[]), &store, &[act("ptt", "old")]).await.unwrap();
    let p = stub(&["ptt"]);
    let out = reconcile(&p, &store, &[act("ptt", "new")]).await.unwrap();
    assert_eq!(out.unbound, vec!["ptt".to_string()]);
    assert_eq!(store.load().unwrap()["ptt"].description, "old", "old approval survives a cancel");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn corrupt_store_is_an_error_not_a_reset() {
    let dir = std::env::temp_dir().join(format!("holdkey-approvals3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("s.json"), "{not json").unwrap();
    assert!(Store::new(dir.join("s.json")).load().is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
