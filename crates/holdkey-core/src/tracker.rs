use std::sync::{Arc, Mutex};

use kbd::action::Action as KbdAction;
use kbd::dispatcher::{Dispatcher, MatchResult};
use kbd::hotkey::{Hotkey, Modifier};
use kbd::key::Key;
use kbd::key_state::{KeyState, KeyTransition};

use crate::{Action, Error, Result};

/// A press/release edge for one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edge {
    Pressed(String),
    Released(String),
}

struct Held {
    action_id: String,
    trigger: Hotkey,
}

/// `kbd` only matches on press; this adds the release half. An action is held
/// from the press that matches its trigger until the trigger key or any of its
/// modifiers goes up, whichever is first (so Ctrl-released-first still ends it).
pub struct HoldTracker {
    dispatcher: Dispatcher,
    state: KeyState,
    matched: Arc<Mutex<Vec<String>>>,
    triggers: Vec<(String, Hotkey)>,
    held: Vec<Held>,
}

impl HoldTracker {
    pub fn new(actions: &[Action]) -> Result<Self> {
        let matched = Arc::new(Mutex::new(Vec::new()));
        let mut dispatcher = Dispatcher::new();
        let mut triggers = Vec::new();
        for a in actions {
            let hotkey: Hotkey = a.preferred_trigger.parse().map_err(|e: <Hotkey as std::str::FromStr>::Err| {
                Error::BadTrigger { trigger: a.preferred_trigger.clone(), reason: e.to_string() }
            })?;
            let (sink, id) = (matched.clone(), a.id.clone());
            let action = KbdAction::from(move || sink.lock().unwrap().push(id.clone()));
            dispatcher
                .register(hotkey, action)
                .map_err(|_| Error::DuplicateTrigger(a.preferred_trigger.clone()))?;
            triggers.push((a.id.clone(), hotkey));
        }
        Ok(Self { dispatcher, state: KeyState::default(), matched, triggers, held: Vec::new() })
    }

    /// Feed one physical key event; returns the edges it caused.
    pub fn feed(&mut self, key: Key, transition: KeyTransition) -> Vec<Edge> {
        self.state.apply_device_event(0, key, transition);
        let mut edges = Vec::new();
        match transition {
            KeyTransition::Press => {
                let hk = Hotkey::with_modifiers(key, self.state.active_modifiers().into_iter());
                if let MatchResult::Matched { action: KbdAction::Callback(f), .. } =
                    self.dispatcher.process(hk, transition)
                {
                    f();
                    let ids: Vec<String> = self.matched.lock().unwrap().drain(..).collect();
                    for id in ids {
                        // A double tap released in time arrives as release+press, so a
                        // press while still held means a lost release: re-edge it.
                        if let Some(pos) = self.held.iter().position(|h| h.action_id == id) {
                            self.held.remove(pos);
                            edges.push(Edge::Released(id.clone()));
                        }
                        let trigger = self.triggers.iter().find(|(i, _)| *i == id).map(|(_, t)| *t);
                        if let Some(trigger) = trigger {
                            self.held.push(Held { action_id: id.clone(), trigger });
                            edges.push(Edge::Pressed(id));
                        }
                    }
                }
            }
            KeyTransition::Release => {
                let modifier = Modifier::from_key(key);
                let still_down = self.state.active_modifiers();
                self.held.retain(|h| {
                    let ends = h.trigger.key() == key
                        || modifier.is_some_and(|m| h.trigger.has_modifier(m) && !still_down.contains(m));
                    if ends {
                        edges.push(Edge::Released(h.action_id.clone()));
                    }
                    !ends
                });
            }
            _ => {}
        }
        edges
    }
}
