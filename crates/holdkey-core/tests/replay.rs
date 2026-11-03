use holdkey_core::{Action, Edge, Error, HoldTracker, MockBackend, ShortcutBackend, ShortcutEvent};
use kbd::key::Key;
use kbd::key_state::KeyTransition::{Press, Release, Repeat};
use tokio::sync::mpsc::unbounded_channel;

fn act(id: &str, trigger: &str) -> Action {
    Action { id: id.into(), description: String::new(), preferred_trigger: trigger.into() }
}
fn p(id: &str) -> Edge { Edge::Pressed(id.into()) }
fn r(id: &str) -> Edge { Edge::Released(id.into()) }

fn replay(trigger: &str, script: &[(Key, kbd::key_state::KeyTransition)]) -> Vec<Edge> {
    let mut t = HoldTracker::new(&[act("ptt", trigger)]).unwrap();
    script.iter().flat_map(|&(k, tr)| t.feed(k, tr)).collect()
}

#[test]
fn mock_backend_emits_pressed_then_released() {
    let (tx, mut rx) = unbounded_channel();
    let mut b = MockBackend::new(vec![(Key::F13, Press), (Key::F13, Release)]);
    assert!(b.capability().works);
    b.start(vec![act("ptt", "F13")], tx).unwrap();
    let kinds: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(matches!(&kinds[..], [
        ShortcutEvent::Pressed { action_id: a, .. },
        ShortcutEvent::Released { action_id: b, .. },
    ] if a == "ptt" && b == "ptt"));
}

// Hex #485: Ctrl released before the trigger key must still end the hold, exactly once.
#[test]
fn hex_485_ctrl_released_first() {
    let edges = replay("Ctrl+Space", &[
        (Key::CONTROL_LEFT, Press), (Key::SPACE, Press),
        (Key::CONTROL_LEFT, Release), (Key::SPACE, Release),
    ]);
    assert_eq!(edges, vec![p("ptt"), r("ptt")]);
}

// Hex #638: a fast double tap is two clean press/release pairs, never overlapping.
#[test]
fn hex_638_fast_double_tap() {
    let edges = replay("F13", &[
        (Key::F13, Press), (Key::F13, Release), (Key::F13, Press), (Key::F13, Release),
    ]);
    assert_eq!(edges, vec![p("ptt"), r("ptt"), p("ptt"), r("ptt")]);
}

#[test]
fn autorepeat_does_not_refire_and_lost_release_is_reedged() {
    assert_eq!(replay("F13", &[(Key::F13, Press), (Key::F13, Repeat), (Key::F13, Repeat), (Key::F13, Release)]),
        vec![p("ptt"), r("ptt")]);
    assert_eq!(replay("F13", &[(Key::F13, Press), (Key::F13, Press)]), vec![p("ptt"), r("ptt"), p("ptt")]);
}

#[test]
fn unrelated_keys_emit_nothing() {
    assert!(replay("Ctrl+Space", &[(Key::SPACE, Press), (Key::SPACE, Release)]).is_empty());
}

#[test]
fn bad_and_duplicate_triggers_are_errors() {
    assert!(matches!(HoldTracker::new(&[act("a", "Ctrl+")]), Err(Error::BadTrigger { .. })));
    assert!(matches!(HoldTracker::new(&[act("a", "F13"), act("b", "F13")]), Err(Error::DuplicateTrigger(_))));
}
