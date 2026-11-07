# Key-Shape Acceptance & PTT Capability Matrix

This document tracks how different trigger shapes are handled across Wayland compositors and portal backends using `ashpd 0.14.0` (`org.freedesktop.portal.GlobalShortcuts` v1/v2), Hyprland IPC, and Linux `evdev`.

## Matrix

| Trigger Shape | Example | XDG Trigger Spec | Hyprland 0.57+ (xdph 1.5+) | KDE Plasma 6.8+ | GNOME 48+ | Deactivated (Release) on Key-Up | Supported holdkey Path |
|---|---|---|---|---|---|---|---|
| **Chord (Mod + Key)** | `Ctrl+Space` | `CTRL+space` | Accepted | Accepted | Accepted | Yes | Portal (primary), evdev |
| **Chord (Logo + Key)** | `Super+V` | `LOGO+v` | Accepted | Accepted | Accepted | Yes | Portal (primary), evdev |
| **Bare Function Key** | `F13` | `F13` | Accepted | Accepted | Accepted | Yes | Portal (primary), evdev |
| **Bare Normal Key** | `space` | `space` | Reassigned / Prompt | Reassigned / Dialog | Rejected / Modal | Varies | Portal (if permitted), evdev |
| **Modifier-Only** | `Right Alt`, `Alt`, `Ctrl` | *Invalid* | **Rejected** | **Rejected** | **Rejected** | No | **Hyprland bind or evdev only** |

## Critical Findings

### 1. Modifier-Only PTT Cannot Use `xdg-desktop-portal`
The XDG shortcuts specification explicitly defines a trigger as modifiers **together with** a key (or a recognized key name). Bare modifier triggers (e.g. holding `Right Alt` alone, as popularized by Superwhisper or push-to-talk voice setups) cannot be registered via `ashpd` / `GlobalShortcuts.CreateSession`.

Attempting to register a modifier-only trigger with the portal fails:
```text
FAIL portal    portal cannot bind modifier-only `Right Alt` (XDG shortcuts spec requires key+modifiers)
     fix: modifier-only → use the Hyprland bind or the evdev path
```

### 2. Workarounds for Modifier-Only PTT
- **Hyprland:** Add a direct compositor bind in `hyprland.conf`:
  ```text
  bind = , Alt_R, global, dev.holdkey.Broker:ptt
  ```
- **Universal Linux (GNOME, KDE, niri, etc.):** Ensure your user belongs to the `input` group (`sudo usermod -aG input $USER`). `holdkeyd` will automatically use the `evdev` backend to track key-down and key-up for any key or modifier with sub-5ms polling.

## Verifying with `holdkey doctor --ptt`
You can verify whether a specific key shape can be used for push-to-talk on your current environment:
```bash
holdkey doctor --ptt             # checks default 'space'
holdkey doctor --ptt "Right Alt" # checks modifier-only trigger
echo $?                          # 0 if a hold/release path exists, 3 if none
```
