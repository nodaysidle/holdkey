//! evdev fallback through `kbd-global` (GNOME 46/47, niri, anything without a portal).
//! Needs read access to `/dev/input/event*`, i.e. membership of the `input` group.

use std::{
    sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};

use holdkey_core::{Action, Capability, Error, Result, ShortcutBackend, ShortcutEvent, now_us};
use kbd::hotkey::Hotkey;
use kbd_global::{binding_guard::BindingGuard, manager::HotkeyManager};
use tokio::sync::mpsc::UnboundedSender;

// shortcut: kbd-global only reports presses, so release is found by polling its key state every
// 5 ms. A press+release+press inside one tick is read as one hold. Upgrade: read evdev directly.
const POLL: Duration = Duration::from_millis(5);

pub struct EvdevBackend {
    manager: Option<Arc<HotkeyManager>>,
    /// Dropping a guard unregisters its binding, so they live as long as the backend.
    guards: Vec<BindingGuard>,
}

impl EvdevBackend {
    pub fn new() -> Self {
        Self { manager: None, guards: Vec::new() }
    }
}

impl Default for EvdevBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub fn capability() -> Capability {
    if holdkey_core::input_readable() {
        Capability::ok("can read /dev/input/event* (hotplug handled by kbd-global)")
    } else {
        Capability::broken(
            "cannot read /dev/input/event*: not in the `input` group",
            "sudo usermod -aG input $USER, then log out and back in",
        )
    }
}

impl ShortcutBackend for EvdevBackend {
    fn name(&self) -> &'static str {
        "evdev"
    }
    fn capability(&self) -> Capability {
        capability()
    }
    fn start(&mut self, actions: Vec<Action>, tx: UnboundedSender<ShortcutEvent>) -> Result<()> {
        let manager = Arc::new(HotkeyManager::new().map_err(|e| Error::Backend(format!("kbd-global: {e}")))?);
        // Callbacks need the manager to poll key state, but are created before it is shared.
        let handle: Arc<OnceLock<Arc<HotkeyManager>>> = Arc::default();
        let mut guards = Vec::new();
        for a in actions {
            let hotkey: Hotkey = a.preferred_trigger.parse().map_err(|e: <Hotkey as std::str::FromStr>::Err| {
                Error::BadTrigger { trigger: a.preferred_trigger.clone(), reason: e.to_string() }
            })?;
            let (tx, handle, id, held) = (tx.clone(), handle.clone(), a.id.clone(), Arc::new(AtomicBool::new(false)));
            let guard = manager
                .register(hotkey, move || {
                    let Some(manager) = handle.get().cloned() else { return };
                    if held.swap(true, Ordering::AcqRel) {
                        return; // still held; the poller below will end it
                    }
                    let _ = tx.send(ShortcutEvent::Pressed { action_id: id.clone(), ts_us: now_us() });
                    let (tx, id, held) = (tx.clone(), id.clone(), held.clone());
                    thread::spawn(move || {
                        while still_down(&manager, hotkey) {
                            thread::sleep(POLL);
                        }
                        held.store(false, Ordering::Release);
                        let _ = tx.send(ShortcutEvent::Released { action_id: id, ts_us: now_us() });
                    });
                })
                .map_err(|e| Error::Backend(format!("register {:?}: {e}", a.id)))?;
            guards.push(guard);
        }
        handle.set(manager.clone()).ok();
        self.guards = guards;
        self.manager = Some(manager);
        Ok(())
    }
}

/// Key and all its modifiers still held. A stopped manager counts as released.
fn still_down(manager: &HotkeyManager, hotkey: Hotkey) -> bool {
    let (Ok(key), Ok(mods)) = (manager.is_key_pressed(hotkey.key()), manager.active_modifiers()) else {
        return false;
    };
    key && hotkey.modifiers().all(|m| mods.contains(m))
}
