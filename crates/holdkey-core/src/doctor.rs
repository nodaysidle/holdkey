//! `holdkey doctor`: turn a probed [`Environment`] into a capability matrix with a fix per failure.
//! Pure on purpose, so every desktop can be snapshot-tested without being installed.

use std::fmt::Write;

use crate::Capability;

#[derive(Debug, Clone, Default)]
pub struct Environment {
    /// `XDG_CURRENT_DESKTOP`, e.g. `ubuntu:GNOME` or `Hyprland`.
    pub desktop: String,
    pub gnome_shell_major: Option<u32>,
    /// Version of `org.freedesktop.portal.GlobalShortcuts` if the portal serves it.
    pub portal_global_shortcuts: Option<u32>,
    pub input_readable: bool,
    /// Hyprland IPC reachable.
    pub hyprland_ipc: bool,
    /// Pre-rendered app-ID drift messages from the Hyprland check.
    pub drift: Vec<String>,
    /// Requested PTT trigger shape (e.g. `space`, `Ctrl+Space`, or `Right Alt`).
    pub ptt_trigger: Option<String>,
}

#[derive(PartialEq, Eq)]
enum Desktop {
    Gnome,
    Kde,
    Hyprland,
    Niri,
    Other,
}

impl Environment {
    fn kind(&self) -> Desktop {
        let d = self.desktop.to_ascii_lowercase();
        let has = |n: &str| d.split(':').any(|p| p == n);
        if has("gnome") {
            Desktop::Gnome
        } else if has("kde") {
            Desktop::Kde
        } else if has("hyprland") {
            Desktop::Hyprland
        } else if has("niri") {
            Desktop::Niri
        } else {
            Desktop::Other
        }
    }
}

pub const EVDEV_FIX: &str = "join the `input` group: sudo usermod -aG input $USER, then log out and in";
pub const MODIFIER_ONLY_FIX: &str = "modifier-only → use the Hyprland bind or the evdev path";

/// Detect whether a trigger consists purely of modifier keys (e.g. `Right Alt`, `Alt`, `Ctrl`).
/// The XDG shortcuts specification defines triggers as modifier(s) together with a key;
/// bare modifier triggers cannot be expressed via xdg-desktop-portal.
pub fn is_modifier_only(trigger: &str) -> bool {
    let t = trigger.trim().to_ascii_lowercase();
    if t.is_empty() {
        return false;
    }
    let parts: Vec<&str> = t.split('+').map(|s| s.trim()).collect();
    parts.iter().all(|&part| {
        let p = part.replace([' ', '_', '-'], "");
        matches!(
            p.as_str(),
            "ctrl"
                | "control"
                | "ctrlr"
                | "ctrll"
                | "leftctrl"
                | "rightctrl"
                | "alt"
                | "altr"
                | "altl"
                | "leftalt"
                | "rightalt"
                | "ralt"
                | "lalt"
                | "shift"
                | "shiftr"
                | "shiftl"
                | "leftshift"
                | "rightshift"
                | "super"
                | "meta"
                | "logo"
                | "superl"
                | "superr"
                | "win"
        )
    })
}

/// Check if any backend can deliver hold/release events for this trigger.
pub fn ptt_capable(env: &Environment, trigger: &str) -> bool {
    let mod_only = is_modifier_only(trigger);
    if mod_only {
        // Portal cannot deliver modifier-only triggers.
        env.input_readable || (env.hyprland_ipc && env.drift.is_empty())
    } else {
        env.portal_global_shortcuts.is_some()
            || env.input_readable
            || (env.hyprland_ipc && env.drift.is_empty())
    }
}

pub fn diagnose(env: &Environment) -> Vec<(String, Capability)> {
    let kind = env.kind();
    let mut rows = Vec::new();

    let mod_only = env.ptt_trigger.as_deref().map(is_modifier_only).unwrap_or(false);

    rows.push(("portal".into(), if mod_only {
        let trig = env.ptt_trigger.as_deref().unwrap_or("modifier-only");
        Capability::broken(
            format!("portal cannot bind modifier-only `{trig}` (XDG shortcuts spec requires key+modifiers)"),
            MODIFIER_ONLY_FIX,
        )
    } else {
        match (env.portal_global_shortcuts, &kind) {
            (Some(v), _) => Capability::ok(format!("GlobalShortcuts portal v{v}")),
            (None, Desktop::Gnome) if env.gnome_shell_major.is_some_and(|m| m < 48) => Capability::broken(
                format!("GNOME {}: no GlobalShortcuts portal (added in xdg-desktop-portal-gnome 48)", env.gnome_shell_major.unwrap_or(0)),
                format!("evdev fallback: {EVDEV_FIX}"),
            ),
            (None, Desktop::Gnome) => Capability::broken("GNOME: GlobalShortcuts portal not found", "install xdg-desktop-portal-gnome >= 48"),
            (None, Desktop::Kde) => Capability::broken("KDE: GlobalShortcuts portal not found", "install xdg-desktop-portal-kde and restart the session"),
            (None, Desktop::Hyprland) => Capability::broken("Hyprland: GlobalShortcuts portal not found", "install xdg-desktop-portal-hyprland and restart the portals"),
            (None, Desktop::Niri) => Capability::broken(
                "niri: portal routing has no GlobalShortcuts backend (unverified)",
                format!("evdev fallback: {EVDEV_FIX}"),
            ),
            (None, Desktop::Other) => Capability::broken("no GlobalShortcuts portal found", format!("evdev fallback: {EVDEV_FIX}")),
        }
    }));

    rows.push(("evdev".into(), if env.input_readable {
        Capability::ok("can read /dev/input/event*")
    } else {
        Capability::broken("cannot read /dev/input/event*: not in the `input` group", EVDEV_FIX)
    }));

    if kind == Desktop::Hyprland {
        rows.push(("hyprland".into(), if env.hyprland_ipc {
            Capability::ok("Hyprland IPC reachable")
        } else {
            Capability::broken("Hyprland IPC unreachable", "run inside the Hyprland session so HYPRLAND_INSTANCE_SIGNATURE is set")
        }));
        rows.extend(env.drift.iter().map(|d| ("drift".to_string(), Capability::broken(d.clone(), "fix the app ID or bind in hyprland.conf"))));
    }

    if let Some(ref trig) = env.ptt_trigger {
        let capable = ptt_capable(env, trig);
        rows.push(("ptt".into(), if capable {
            Capability::ok(format!("hold/release path exists for `{trig}`"))
        } else {
            Capability::broken(
                format!("no hold/release path for `{trig}`"),
                if mod_only {
                    MODIFIER_ONLY_FIX
                } else {
                    EVDEV_FIX
                },
            )
        }));
    }

    // Informational: tells clients where text injection can work (holdkey itself never injects).
    rows.push(("inject".into(), match kind {
        Desktop::Gnome | Desktop::Kde => Capability::broken(
            "KWin/Mutter: no virtual-keyboard-v1",
            "use wdotool's libei path (wdotool / wdotool-core)",
        ),
        _ => Capability::ok("virtual-keyboard-v1 available (wdotool wlr backend)"),
    }));
    rows
}

pub fn render(rows: &[(String, Capability)]) -> String {
    let mut out = String::new();
    for (name, c) in rows {
        // `inject` is advice for client apps, not a holdkey failure.
        let label = match (c.works, name.as_str()) {
            (true, _) => "ok  ",
            (false, "inject") => "note",
            (false, _) => "FAIL",
        };
        let _ = writeln!(out, "{label} {name:<9} {}", c.reason);
        if !c.works && !c.fix_hint.is_empty() {
            let _ = writeln!(out, "     fix: {}", c.fix_hint);
        }
    }
    // Same preference order as holdkeyd, whatever order the rows arrive in.
    let pick = ["portal", "evdev"].into_iter().find(|b| rows.iter().any(|(n, c)| n == b && c.works));
    let _ = writeln!(out, "backend: {}", pick.unwrap_or("none"));
    out
}
