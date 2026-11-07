use holdkey_core::doctor::{Environment, diagnose, render};

fn show(env: Environment) -> String {
    render(&diagnose(&env))
}

fn envs() -> [(&'static str, Environment); 5] {
    [
        ("gnome46", Environment { desktop: "ubuntu:GNOME".into(), gnome_shell_major: Some(46), input_readable: false, ..Default::default() }),
        ("gnome48", Environment { desktop: "GNOME".into(), gnome_shell_major: Some(48), portal_global_shortcuts: Some(2), input_readable: true, ..Default::default() }),
        ("kde6", Environment { desktop: "KDE".into(), portal_global_shortcuts: Some(2), input_readable: true, ..Default::default() }),
        ("hypr", Environment { desktop: "Hyprland".into(), portal_global_shortcuts: Some(1), input_readable: true, hyprland_ipc: true, drift: vec!["shortcut `ptt` is registered as `hex:ptt` but the app ID is `wrong`".into()], ..Default::default() }),
        ("niri", Environment { desktop: "niri".into(), input_readable: true, ..Default::default() }),
    ]
}

#[test]
fn snapshot_gnome46() {
    let e = envs().into_iter().find(|(n, _)| *n == "gnome46").unwrap().1;
    assert_eq!(show(e), r#"FAIL portal    GNOME 46: no GlobalShortcuts portal (added in xdg-desktop-portal-gnome 48)
     fix: evdev fallback: join the `input` group: sudo usermod -aG input $USER, then log out and in
FAIL evdev     cannot read /dev/input/event*: not in the `input` group
     fix: join the `input` group: sudo usermod -aG input $USER, then log out and in
note inject    KWin/Mutter: no virtual-keyboard-v1
     fix: use wdotool's libei path (wdotool / wdotool-core)
backend: none
"#);
}

#[test]
fn snapshot_gnome48() {
    let e = envs().into_iter().find(|(n, _)| *n == "gnome48").unwrap().1;
    assert_eq!(show(e), r#"ok   portal    GlobalShortcuts portal v2
ok   evdev     can read /dev/input/event*
note inject    KWin/Mutter: no virtual-keyboard-v1
     fix: use wdotool's libei path (wdotool / wdotool-core)
backend: portal
"#);
}

#[test]
fn snapshot_kde6() {
    let e = envs().into_iter().find(|(n, _)| *n == "kde6").unwrap().1;
    assert_eq!(show(e), r#"ok   portal    GlobalShortcuts portal v2
ok   evdev     can read /dev/input/event*
note inject    KWin/Mutter: no virtual-keyboard-v1
     fix: use wdotool's libei path (wdotool / wdotool-core)
backend: portal
"#);
}

#[test]
fn snapshot_hypr() {
    let e = envs().into_iter().find(|(n, _)| *n == "hypr").unwrap().1;
    assert_eq!(show(e), r#"ok   portal    GlobalShortcuts portal v1
ok   evdev     can read /dev/input/event*
ok   hyprland  Hyprland IPC reachable
FAIL drift     shortcut `ptt` is registered as `hex:ptt` but the app ID is `wrong`
     fix: fix the app ID or bind in hyprland.conf
ok   inject    virtual-keyboard-v1 available (wdotool wlr backend)
backend: portal
"#);
}

#[test]
fn snapshot_niri() {
    let e = envs().into_iter().find(|(n, _)| *n == "niri").unwrap().1;
    assert_eq!(show(e), r#"FAIL portal    niri: portal routing has no GlobalShortcuts backend (unverified)
     fix: evdev fallback: join the `input` group: sudo usermod -aG input $USER, then log out and in
ok   evdev     can read /dev/input/event*
ok   inject    virtual-keyboard-v1 available (wdotool wlr backend)
backend: evdev
"#);
}

#[test]
fn snapshot_ptt_modifier_only_hint() {
    let mut e = envs().into_iter().find(|(n, _)| *n == "gnome48").unwrap().1;
    e.ptt_trigger = Some("Right Alt".into());
    assert_eq!(show(e), r#"FAIL portal    portal cannot bind modifier-only `Right Alt` (XDG shortcuts spec requires key+modifiers)
     fix: modifier-only → use the Hyprland bind or the evdev path
ok   evdev     can read /dev/input/event*
ok   ptt       hold/release path exists for `Right Alt`
note inject    KWin/Mutter: no virtual-keyboard-v1
     fix: use wdotool's libei path (wdotool / wdotool-core)
backend: evdev
"#);
}

#[test]
fn ptt_capability_matrix_and_exit_codes() {
    use holdkey_core::doctor::ptt_capable;

    // GNOME 46 (no portal, no input group): cannot deliver PTT -> exit 3
    let g46 = envs().into_iter().find(|(n, _)| *n == "gnome46").unwrap().1;
    assert!(!ptt_capable(&g46, "space"));
    assert!(!ptt_capable(&g46, "Right Alt"));

    // GNOME 48 (portal v2, input readable):
    let g48 = envs().into_iter().find(|(n, _)| *n == "gnome48").unwrap().1;
    assert!(ptt_capable(&g48, "space")); // portal can deliver standard keys
    assert!(ptt_capable(&g48, "Right Alt")); // evdev can deliver modifier-only

    // GNOME 48 without evdev:
    let g48_no_evdev = Environment {
        desktop: "GNOME".into(),
        gnome_shell_major: Some(48),
        portal_global_shortcuts: Some(2),
        input_readable: false,
        ..Default::default()
    };
    assert!(ptt_capable(&g48_no_evdev, "space")); // portal delivers
    assert!(!ptt_capable(&g48_no_evdev, "Right Alt")); // portal CANNOT deliver modifier-only -> exit 3

    // KDE 6:
    let kde6 = envs().into_iter().find(|(n, _)| *n == "kde6").unwrap().1;
    assert!(ptt_capable(&kde6, "CTRL+space"));
    assert!(ptt_capable(&kde6, "F13"));
    assert!(ptt_capable(&kde6, "Alt"));

    // Hyprland:
    let hypr = envs().into_iter().find(|(n, _)| *n == "hypr").unwrap().1;
    assert!(ptt_capable(&hypr, "space"));
    assert!(ptt_capable(&hypr, "Right Alt"));

    // niri:
    let niri = envs().into_iter().find(|(n, _)| *n == "niri").unwrap().1;
    assert!(ptt_capable(&niri, "space"));
    assert!(ptt_capable(&niri, "Right Alt"));
}
