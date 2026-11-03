# holdkey

A small per-user daemon that gives any app **global push-to-talk style shortcuts on Wayland**: it tells the app when a key combo is pressed and when it is released.

## Why
Wayland stops apps from seeing keys pressed while another window has focus. Dictation, voice chat and recording tools all need exactly that, so each one rebuilds its own workaround (portal code, compositor config, raw evdev), and each breaks differently per desktop. holdkey is that layer once: apps call `Register` over D-Bus and receive `Pressed` / `Released` signals, whatever the desktop.

## What works today (verified on Arch, Hyprland 0.56.2)
- D-Bus API `dev.holdkey.Broker1`: `Register`, `Pressed`, `Released`, `Capabilities`.
- GlobalShortcuts portal backend: live hold test gives one press, one release when the key is let go.
- evdev fallback (kbd-global): live hold test OK, survives unplugging and replugging a USB keyboard.
- Portal approvals are kept when a re-prompt is cancelled (stub-portal test).
- `holdkey doctor` says which backend works and how to fix the others; snapshot-tested for GNOME 46/48, KDE 6, Hyprland, niri.
- `holdkey watch | bind | status`, PKGBUILD and systemd user unit (package builds).

## Not yet verified
KDE Plasma 6 and GNOME 48+ portal runs; evdev on GNOME 46 / niri; clean-VM `makepkg -si`. CI is green.
See `HANDOFF.md` for the per-task table.

## Goals
1. Confirm the portal path on KDE and GNOME 48+, evdev on GNOME 46 and niri.
2. Make holdkeyd choose the backend automatically per desktop and explain it in `doctor`.
3. Publish to the AUR; let apps (dictation tools, voice chat, OBS helpers) depend on one stable API instead of their own hacks.
4. Later: settings window and on-screen indicator (designed, deferred).

Text injection is out of scope; use wdotool.

## Hyprland setup
Add one plain bind (no `release = true` bind, it double-fires Released):
`bind = ALT, F12, global, dev.holdkey.Broker:ptt`
