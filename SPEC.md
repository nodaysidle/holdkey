# Verified Project Blueprint: holdkey (red-team audit of 03-draft-blueprint.md)

- **Audit date:** 2026-11-03 (Europe/Ljubljana)
- **Input:** `03-draft-blueprint.md` (not modified)
- **Method:** each lookup below was a live query run on 2026-11-03 against the AUR RPC v5, the archlinux.org package JSON, the crates.io API, the Arch Wiki API, GitHub (cursor-github MCP `search_repositories`, plus api.github.com/raw.githubusercontent.com for commit dates and source files), and wayland.app. Dates are UTC as the APIs return them.

---

## 1. Verdict

**PIVOT.** Two of the draft's three pillars already exist. Text injection is covered by **wdotool / wdotool-core** (libei via the RemoteDesktop portal, wlr virtual-keyboard, KWin, GNOME). The evdev press/release-with-hotplug engine is covered by **kbd + kbd-global**, **handy-keys** and **hotkey-listener**. Nobody ships the remaining ~10%: a shared session broker that gives any app press/release events over D-Bus across portal, Hyprland and evdev, keeps portal approvals after a cancelled dialog, and has a `doctor` that explains failures. Build only that. The MVP was cut 43.75% (16 → 9 tasks).

---

## 2. Prior Art Checked

Overlap % is my estimate of how much of the **draft's** feature it covers.

| Source | Exact query | Top hits (URL) | Overlap |
|---|---|---|---|
| AUR | `rpc/v5/search/push-to-talk?by=name-desc` (21 results) | voxtype 1.1.1 https://aur.archlinux.org/packages/voxtype ; wayland-push-to-talk-fix-git https://aur.archlinux.org/packages/wayland-push-to-talk-fix-git ; ptt-fix 0.9.2 https://aur.archlinux.org/packages/ptt-fix | 30% (apps or Discord-only fixes, no broker) |
| AUR | `rpc/v5/search/hotkey?by=name-desc` (78) | obs-wayland-hotkeys-git https://aur.archlinux.org/packages/obs-wayland-hotkeys-git ; crossmacro 1.5.2 https://aur.archlinux.org/packages/crossmacro | 10% |
| AUR | `rpc/v5/search/wayland-hotkey` (2), `global-shortcut` (0), `input-emulation` (0) | obs-wayland-hotkeys 1.1.1 https://aur.archlinux.org/packages/obs-wayland-hotkeys | 5% (OBS only) |
| AUR | `rpc/v5/search/dotool` (25) | dotool 1.6 https://aur.archlinux.org/packages/dotool ; kdotool 0.3.0 https://aur.archlinux.org/packages/kdotool ; wdotool-bin 0.5.3 https://aur.archlinux.org/packages/wdotool-bin | **90% of the injection half** (wdotool) |
| AUR | `rpc/v5/search/wtype` (6), `swhkd` (2), `keyd` (21), `dictation` (63) | kwtype-git https://aur.archlinux.org/packages/kwtype-git ; swhkd-git https://aur.archlinux.org/packages/swhkd-git ; tuxtalk-appimage 0.20.0 https://aur.archlinux.org/packages/tuxtalk-appimage | 20% |
| Official repos | `packages/search/json/?q=ydotool`, `q=wtype`, `q=dotool`, `q=keyd` | extra/ydotool 1.0.4-3 https://archlinux.org/packages/extra/x86_64/ydotool/ ; extra/wtype 0.4-2 https://archlinux.org/packages/extra/x86_64/wtype/ ; extra/keyd 2.6.0-5 https://archlinux.org/packages/extra/x86_64/keyd/ | 40% of injection (uinput/vk-v1 only, no KWin/Mutter path for wtype) |
| Official repos | `q=xdg-desktop-portal`, `q=libei`, `q=wl-clipboard`, `q=swhkd` (0) | extra/xdg-desktop-portal 1.22.1 ; extra/xdg-desktop-portal-gnome 51.0 ; extra/libei 1.6.0 ; extra/wl-clipboard 2.3.0 https://archlinux.org/packages/extra/x86_64/wl-clipboard/ | building blocks, 0% of the broker |
| crates.io | `crates?q=global-shortcut&sort=downloads` (16) | tauri-plugin-global-shortcut 2.4.1 https://crates.io/crates/tauri-plugin-global-shortcut ; kbd-global 0.2.1 https://crates.io/crates/kbd-global | **kbd + kbd-global: 80% of the chord engine + evdev fallback** |
| crates.io | `crates?q=hotkey&sort=downloads` (550) | global-hotkey 0.9.0 https://crates.io/crates/global-hotkey ; handy-keys 0.3.4 https://crates.io/crates/handy-keys (evdev, hotplug, blocking via uinput) | 70% of the evdev fallback |
| crates.io | `crates?q=push-to-talk&sort=downloads` (64), `q=global-hotkey` (12) | push2talk 1.3.2 https://crates.io/crates/push2talk (last release 2024-06-18) ; keytap 0.4.0 https://crates.io/crates/keytap ; hotkey-listener 0.3.2 https://crates.io/crates/hotkey-listener (evdev Pressed/Released + reconnect, F-keys only) | 60% of the evdev fallback |
| crates.io | `crates?q=wayland-hotkey` (1), `q=virtual-keyboard` (13), `q=input-injection` (4) | Simple-Wayland-HotKey-Daemon 0.0.0 (2022) https://crates.io/crates/Simple-Wayland-HotKey-Daemon ; zwp-virtual-keyboard 0.0.0 (2022) https://crates.io/crates/zwp-virtual-keyboard | 10% |
| Arch Wiki | `srsearch=XDG Desktop Portal` (4), `Input remap utilities` (2); `push to talk`, `push-to-talk`, `global shortcuts`, `GlobalShortcuts`, `Wayland hotkey`, `ydotool`, `uinput`, `wtype` (0 hits each) | https://wiki.archlinux.org/title/XDG_Desktop_Portal ; https://wiki.archlinux.org/title/Input_remap_utilities | 0%. The API answered every query; the wiki has no push-to-talk or shortcut-broker page |
| GitHub | `push-to-talk wayland` sort:stars (152) | peteonrails/voxtype 1734★ https://github.com/peteonrails/voxtype ; Rush/wayland-push-to-talk-fix 117★ https://github.com/Rush/wayland-push-to-talk-fix ; cyrinux/push2talk 61★ https://github.com/cyrinux/push2talk ; ronb1964/TuxTalk 36★ https://github.com/ronb1964/TuxTalk | 30% (each rebuilds its own layer, which supports the draft's thesis) |
| GitHub | `wayland hotkey daemon` (16) | aaronamk/hkd 25★ https://github.com/aaronamk/hkd ; DraconDev/obs-wayland-hotkey https://github.com/DraconDev/obs-wayland-hotkey ; glowfi/ghkd https://github.com/glowfi/ghkd | 25% (run commands; no press/release API to apps) |
| GitHub | `GlobalShortcuts portal` (4) | oBoolt/obs-global-shortcuts https://github.com/oBoolt/obs-global-shortcuts ; marvin1099/python-global-shortcut-portal https://github.com/marvin1099/python-global-shortcut-portal | 15% (single-app or single-language portal wrappers) |
| GitHub | `global hotkey linux wayland language:rust` (9) | Glaicer/Glossa https://github.com/Glaicer/Glossa ; germanua/Linux-SoundBoard https://github.com/germanua/Linux-SoundBoard | 10% |
| GitHub | `wayland type text virtual keyboard libei` (1) + direct repo check | cushycush/wdotool 40★, pushed 2026-05-23 https://github.com/cushycush/wdotool ; Sporif/KWtype https://github.com/Sporif/KWtype ; bloodynite/opendeck-input-text https://github.com/bloodynite/opendeck-input-text | **90% of injection** |
| GitHub (topic) | `topic:push-to-talk` (554) | peteonrails/voxtype ; primaprashant/awesome-voice-typing https://github.com/primaprashant/awesome-voice-typing | 30% |
| GitHub (topic) | `topic:wayland topic:hotkey` (15) ; `topic:wayland topic:input-emulation` (1) | kalix127/secret-stripper https://github.com/kalix127/secret-stripper ; bnema/uinputd-go https://github.com/bnema/uinputd-go | 5% |
| GitHub (direct) | README of joshuadavidthomas/kbd, martintrojer/hotkey-listener, handy-computer/handy-keys | https://github.com/joshuadavidthomas/kbd (tap-hold, sequences, layers; kbd-global reads evdev directly) ; https://github.com/martintrojer/hotkey-listener ; https://github.com/handy-computer/handy-keys | see the crates.io rows |

**What prior art kills in the draft:**
- **Text injection** (draft tasks 9, 10, 11 and the uinput path): wdotool already does libei via the RemoteDesktop portal, wlr virtual-keyboard on Sway, Hyprland, river and Wayfire, KWin scripting, and a GNOME Shell extension. Its README says its KDE and GNOME backends are "experimental" and not smoke-tested. Send fixes upstream; don't fork.
- **Chord state machine** (draft task 3): `kbd` already has tap-hold, sequences, press/release transitions and device-source tracking.
- **evdev with hotplug** (part of draft task 7): kbd-global, handy-keys ("Keyboards plugged in while the listener is running are picked up automatically") and hotkey-listener ("Automatic keyboard reconnection").

**Unsolved, and the pivot target:** a **session daemon with a stable D-Bus API** (`Register`, `Pressed`, `Released`, `Capabilities`). It picks the portal, Hyprland or evdev path for each desktop, keeps portal approvals across a cancelled dialog, and its `doctor` explains failures. No hit in any source provides this as a shared service.

---

## 3. Dependency Check

Rule: STALE means no release **and** no commit since 2025-11-03. DEAD means not found.

### Crates (crates.io latest stable; date is that version's publish date; none yanked)

| Item | Type | Latest version / commit | Date | Status | URL |
|---|---|---|---|---|---|
| tokio | crate | 1.54.1 | 2026-10-21 | OK | https://crates.io/crates/tokio |
| zbus | crate | 6.2.1 | 2026-11-01 | OK | https://crates.io/crates/zbus |
| ashpd | crate | 0.13.13 (repo commit 2026-11-03) | 2026-07-17 | OK | https://crates.io/crates/ashpd |
| reis | crate | 0.7.1 | 2026-07-30 | OK (not used in MVP; injection goes to wdotool) | https://crates.io/crates/reis |
| wayland-client | crate | 0.31.15 | 2026-07-22 | OK (not used in MVP) | https://crates.io/crates/wayland-client |
| wayland-protocols | crate | 0.32.13 | 2026-06-19 | OK (not used in MVP) | https://crates.io/crates/wayland-protocols |
| wayland-protocols-misc | crate | 0.3.12 | 2026-03-31 | OK (not used in MVP) | https://crates.io/crates/wayland-protocols-misc |
| xkbcommon | crate | 0.9.0; last repo commit 6c37936 on 2025-10-25 | 2025-08-09 | **STALE**, cut (only needed for the injection path, now delegated) | https://crates.io/crates/xkbcommon |
| xkbcommon-dl (checked as a replacement) | crate | 0.4.2; last commit 2024-02-11 | 2024-02-11 | **STALE**, rejected | https://crates.io/crates/xkbcommon-dl |
| evdev | crate | 0.13.2; repo emberian/evdev commit f90916d on 2026-05-28 | 2025-09-15 | OK (by commit) | https://crates.io/crates/evdev |
| tokio-udev | crate | 0.10.0; last commit 4dcaf26 on 2024-12-13 | 2025-10-08 | **STALE**, cut | https://crates.io/crates/tokio-udev |
| udev (checked as a replacement) | crate | 0.9.3; last commit 2025-01-23 | 2025-01-23 | **STALE**, rejected; hotplug comes from kbd-global or handy-keys instead | https://crates.io/crates/udev |
| wl-clipboard-rs | crate | 0.9.5 | 2026-10-26 | OK (the task that used it was cut) | https://crates.io/crates/wl-clipboard-rs |
| hyprland | crate | stable 0.3.13 (2024-02-03); newest 0.4.0-beta.3 (2025-09-24); repo commit c7037d4 on 2026-07-26 | 2024-02-03 | OK by commit, but the release line is stale. **Not used.** Talk to Hyprland's `.socket.sock` directly (2 commands) | https://crates.io/crates/hyprland |
| niri-ipc | crate | 26.4.0 | 2026-04-25 | OK (not used in MVP: it can't send key press/release to an app) | https://crates.io/crates/niri-ipc |
| swayipc-async | crate | 3.0.0; repo commit 587e5ff on 2026-09-23 | 2025-10-26 | OK by commit (not used in MVP, for the same reason) | https://crates.io/crates/swayipc-async |
| serde | crate | 1.0.229 | 2026-07-18 | OK | https://crates.io/crates/serde |
| toml | crate | 1.1.8+spec-1.1.0 | 2026-10-09 | OK | https://crates.io/crates/toml |
| clap | crate | 4.6.7 | 2026-09-14 | OK | https://crates.io/crates/clap |
| tracing | crate | 0.1.44 | 2025-12-18 | OK | https://crates.io/crates/tracing |
| tracing-journald | crate | 0.3.2 | 2025-11-26 | OK | https://crates.io/crates/tracing-journald |
| thiserror | crate | 2.0.21 | 2026-09-23 | OK | https://crates.io/crates/thiserror |
| tauri | crate | 2.12.5 | 2026-10-21 | OK (UI deferred past MVP) | https://crates.io/crates/tauri |
| cxx-qt | crate | 0.10.0 | 2026-08-24 | OK (HUD deferred past MVP) | https://crates.io/crates/cxx-qt |
| cxx-qt-lib | crate | 0.10.0 | 2026-08-24 | OK (deferred) | https://crates.io/crates/cxx-qt-lib |
| kbd (new) | crate | 0.2.1 | 2026-09-20 | OK. Low adoption (1394 downloads, 6★); pin the version | https://crates.io/crates/kbd |
| kbd-global (new) | crate | 0.2.1 | 2026-09-20 | OK. Low adoption (319 downloads); handy-keys is the fallback | https://crates.io/crates/kbd-global |
| handy-keys (new, fallback) | crate | 0.3.4 | 2026-08-07 | OK | https://crates.io/crates/handy-keys |
| wdotool-core (new, recommended to clients) | crate | 0.5.3 | 2026-05-21 | OK | https://crates.io/crates/wdotool-core |
| pipewire (reference-client task) | crate | 0.10.1 | 2026-08-19 | OK (the task that used it was cut) | https://crates.io/crates/pipewire |

### Wayland protocols (wayland.app compositor support, Hyprland first)

| Item | Type | Support found | Date | Status | URL |
|---|---|---|---|---|---|
| virtual-keyboard-unstable-v1 (`zwp_virtual_keyboard_v1`) | Wayland protocol | Hyprland 0.52.1 ✓, niri 26.04 ✓, Sway 1.11 ✓, COSMIC ✓; **KWin 6.7 ✗, Mutter 51 ✗** (matches evidence 7) | page read 2026-11-03 | OK (not used directly; wdotool uses it) | https://wayland.app/protocols/virtual-keyboard-unstable-v1 |
| wlr-layer-shell-unstable-v1 | Wayland protocol | Hyprland ✓ v5, KWin 6.7 ✓, niri ✓, Sway ✓; **Mutter ✗** | page read 2026-11-03 | OK (HUD deferred) | https://wayland.app/protocols/wlr-layer-shell-unstable-v1 |
| xdg-shell | Wayland protocol | All listed compositors, including Mutter 51 | page read 2026-11-03 | OK (deferred) | https://wayland.app/protocols/xdg-shell |
| hyprland-global-shortcuts-v1 | Wayland protocol | Hyprland only. Its source sends `pressed`/`released` (src/protocols/GlobalShortcuts.cpp) | page read 2026-11-03 | OK (reached through xdph) | https://wayland.app/protocols/hyprland-global-shortcuts-v1 |
| kde-fake-input (used by KWtype) | Wayland protocol | wayland.app: "No compositor support found" (KWin-private) | page read 2026-11-03 | not used | https://wayland.app/protocols/kde-fake-input |

### Portals, compositor IPC, system pieces

| Item | Type | Latest version / commit | Date | Status | URL |
|---|---|---|---|---|---|
| org.freedesktop.portal.GlobalShortcuts | D-Bus portal | Interface version 2; `Activated` and `Deactivated` signals exist | docs read 2026-11-03 | OK | https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html |
| org.freedesktop.portal.RemoteDesktop (`ConnectToEIS`) | D-Bus portal | version 2 | docs read 2026-11-03 | OK (wdotool's path) | https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html |
| xdg-desktop-portal | system | 1.22.1 (Arch extra); commit 8c3efad on 2026-10-30 | 2026-06-21 | OK | https://github.com/flatpak/xdg-desktop-portal |
| xdg-desktop-portal-gnome GlobalShortcuts | portal backend | `src/globalshortcuts.c` first added in commit 0e49f6f (2025-02-26). Present at tag 48.0, **absent at 47.3**. Latest 51.0 | 2026-10-19 (last commit) | OK on GNOME ≥48. **GNOME 46/47 has no portal**, which confirms evidence 5; evdev fallback required | https://github.com/GNOME/xdg-desktop-portal-gnome |
| xdg-desktop-portal-hyprland | portal backend | `src/portals` has GlobalShortcuts, InputCapture, Screencopy, Screenshot; **no RemoteDesktop**. Commit 8545044 on 2026-11-01 | 2026-11-01 | OK. Libei-via-portal injection does not exist on Hyprland | https://github.com/hyprwm/xdg-desktop-portal-hyprland |
| niri GlobalShortcuts | portal routing | niri-portals.conf: `default=gnome;gtk;`. No niri-specific GlobalShortcuts implementation found | read 2026-11-03 | UNVERIFIED. MVP uses evdev on niri | https://github.com/niri-wm/niri |
| hyprctl `globalshortcuts` | compositor IPC | hyprctl Strings.hpp: "globalshortcuts → Lists all global shortcuts". `global(string)` dispatcher documented. Hyprland v0.57.1 released 2026-11-03; extra/hyprland 0.57.0 | 2026-11-03 | OK | https://github.com/hyprwm/Hyprland |
| Hyprland IPC socket `$XDG_RUNTIME_DIR/hypr/[HIS]/.socket.sock` | compositor IPC | Documented. Warning: an unclosed connection **freezes Hyprland** up to 5 s | wiki read 2026-11-03 | OK. Open, write and close for every request | https://github.com/hyprwm/hyprland-wiki/blob/main/content/ipc/_index.md |
| kglobalaccel | KDE D-Bus service | extra/kglobalaccel 6.31.0; commit on 2026-10-30 | 2026-10-12 | OK (not used: the KDE portal covers it) | https://github.com/KDE/kglobalaccel |
| layer-shell-qt (LayerShellQt) | Qt6 library | extra/layer-shell-qt 6.8.2; commit on 2026-09-14 | 2026-10-28 | OK (deferred) | https://github.com/KDE/layer-shell-qt |
| PipeWire | system | extra/pipewire 1.6.10; commit on 2026-11-03 | 2026-10-22 | OK (reference-client task cut) | https://github.com/PipeWire/pipewire |
| libei | system | extra/libei 1.6.0 | 2026-10-18 | OK | https://archlinux.org/packages/extra/x86_64/libei/ |
| wtype | tool | extra/wtype 0.4-2 (built 2024-07-12); repo commit 4b9cc29 on 2026-10-20 | 2026-10-20 | OK (fails on KWin and Mutter per the vk-v1 table) | https://github.com/atx/wtype |
| ydotool | tool | extra/ydotool 1.0.4-3; commit 708e96f on 2025-12-22 | 2025-12-22 | OK (uinput, needs root or a udev rule) | https://github.com/ReimuNotMoe/ydotool |
| swhkd | tool | commit cbbfc4a on 2026-04-26 | 2026-04-26 | OK | https://github.com/waycrate/swhkd |
| keyd | tool | extra/keyd 2.6.0-5; commit f564288 on 2026-06-01 | 2026-06-01 | OK | https://github.com/rvaiya/keyd |
| wdotool | tool | 0.5.3; commit 662d707 on 2026-05-23 | 2026-05-23 | OK | https://github.com/cushycush/wdotool |

**RE hooks (ptrace, eBPF, LD_PRELOAD, Frida):** none proposed in the draft, none added, so there is nothing to verify.

**Draft evidence spot-check:** Hex #781 (2026-11-01), #1023 (2026-11-02), #485 (2026-10-30) and #638 (2026-10-31) all exist. Their titles match the draft's claims.

---

## 4. Scope Cut

Before: **16** MVP tasks. Cut: **7**. After: **9**. **Cut = 7/16 = 43.75%.**

| # (draft) | Task | Decision | Reason |
|---|---|---|---|
| 3 | Own chord state machine | **CUT** | Duplicates `kbd` (tap-hold, sequences, press/release). Replaced by a test-only replay of Hex #485/#638 through `kbd`, folded into surviving task 2 |
| 9 | virtual-keyboard-v1 injection backend | **CUT** | Duplicates wdotool's wlr virtual-keyboard backend. Also needed `xkbcommon` (STALE) |
| 10 | RemoteDesktop + libei injection backend | **CUT** | Duplicates wdotool's libei/RemoteDesktop path. xdph has no RemoteDesktop portal, so it can't work on Hyprland anyway |
| 11 | Clipboard + paste fallback | **CUT** | Nice-to-have injection fallback. Out of scope once injection is delegated |
| 13 | Tauri v2 settings window | **CUT** (deferred) | Nice-to-have. `holdkey doctor`/`watch` give the same diagnostics. tauri 2.12.5 is verified OK for a later phase |
| 14 | Qt6 LayerShellQt HUD | **CUT** (deferred) | Nice-to-have. No layer-shell on Mutter, so it needs a second code path. cxx-qt 0.10.0 and layer-shell-qt 6.8.2 are verified OK for later |
| 15 | PipeWire reference client | **CUT** | Nice-to-have demo. `busctl --user monitor` and `holdkey watch` prove the API |
| 7 | evdev fallback | **KEPT, re-scoped** | The separate `holdkey-evdevd` system service, `tokio-udev` (STALE) and the uinput path are dropped. In-process `kbd-global`, with `input` group membership checked by `doctor` |
| 8 | D-Bus Broker1 | **KEPT, re-scoped** | `Type()` is removed. Injection is wdotool's job |
| 6 | Hyprland backend | **KEPT, re-scoped** | The `hyprland` crate (stable release from 2024) is replaced by direct open/write/close on `.socket.sock` |

Also cut, though none was a numbered task: the niri-ipc and swayipc-async backends (their IPC gives an app no key press/release; unverified value), the kglobalaccel backend (the KDE portal covers it), and the Flatpak D-Bus name work.

---

## 5. Surviving Spec

### Concept (after pivot)

`holdkeyd` is a **per-user session broker for global press/release shortcuts on Wayland**. An app calls `Register` once over D-Bus and receives `Pressed` and `Released` signals. The daemon picks a path for the desktop:
- GlobalShortcuts portal on KDE, GNOME ≥48 and Hyprland/xdph
- Hyprland IPC checks for app-ID drift
- evdev via `kbd-global` on GNOME 46/47, niri and anything without a portal

It **keeps portal approvals** so a cancelled dialog never unbinds keys. `holdkey doctor` names the exact reason a key doesn't fire and the fix.

**Not in scope:** text injection (point clients to **wdotool / wdotool-core**), chord parsing beyond what `kbd` provides, dictation, remapping (keyd), any GUI in the MVP.

### Architecture (verified pieces only)

| Component | Stack (verified version) | Runs as | Notes |
|---|---|---|---|
| `holdkeyd` | Rust, tokio 1.54.1, zbus 6.2.1, ashpd 0.13.13, kbd 0.2.1, kbd-global 0.2.1, serde 1.0.229, toml 1.1.8, tracing 0.1.44, tracing-journald 0.3.2, thiserror 2.0.21 | systemd **user** service | Owns sessions and bindings, emits events, persists portal approvals |
| `holdkey` CLI | Rust, clap 4.6.7 | user | `doctor`, `watch`, `bind`, `status` |
| Portal backend | ashpd → `org.freedesktop.portal.GlobalShortcuts` v2 (`Activated`/`Deactivated`) | in `holdkeyd` | KDE, GNOME ≥48 (globalshortcuts.c in xdg-desktop-portal-gnome 48.0+), Hyprland via xdph |
| Hyprland backend | Raw UnixStream to `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock` (open, write, close per request), `globalshortcuts` and `global` dispatcher | in `holdkeyd` | Detects app-ID/trigger drift (evidence 6). No `hyprland` crate |
| evdev fallback | kbd-global 0.2.1 (handy-keys 0.3.4 is the drop-in alternative), evdev 0.13.2 underneath | in `holdkeyd` | Needs the `input` group, which `doctor` checks. Hotplug must pass the replug test |
| Wayland (direct) | none in MVP | n/a | wayland.app tables are used only by `doctor` hints (vk-v1 missing on KWin/Mutter, so recommend wdotool's libei path there) |
| Tauri v2 / Qt6 | tauri 2.12.5 / cxx-qt 0.10.0 + layer-shell-qt 6.8.2 | **post-MVP** | Verified OK; not built in this sprint |

D-Bus API `dev.holdkey.Broker1`: `Register(app_id, actions[{id, description, preferred_trigger}]) -> session`, signals `Pressed(action_id, ts_us)` and `Released(action_id, ts_us)`, property `Capabilities`. Socket: `$XDG_RUNTIME_DIR/holdkey.sock` NDJSON for `watch`. State: `~/.local/state/holdkey/portal-sessions.json`.

### MVP Tasks (9)

1. Cargo workspace with `holdkey-core`, `holdkeyd`, `holdkey-cli`. Test: `cargo build --workspace` passes and CI is green.
2. `ShortcutBackend` trait, `Capability { works, reason, fix_hint }` and a mock backend, with `kbd` for trigger parsing and hold/tap. Test: the mock emits Pressed/Released, and the Hex #485 (Ctrl-first release) and #638 (fast double tap) sequences replayed through `kbd` give the expected results.
3. GlobalShortcuts portal backend via ashpd. Test: on KDE Plasma 6 and GNOME 48+, `holdkey watch` prints Pressed and Released for a bound F13.
4. Persist portal approvals so a cancelled re-prompt keeps the keys bound. Test: a stubbed-portal integration test reproduces obs-rust #2636, and the keys stay bound.
5. Hyprland backend: read `globalshortcuts` over `.socket.sock` and flag app-ID/trigger drift. Test: on Hyprland 0.57, a held Alt+Space reaches `holdkey watch`, and a deliberately wrong app ID is reported by `doctor`.
6. evdev fallback through kbd-global. Test: on GNOME 46 or niri, unplug and replug a USB keyboard while a key is bound, and the hotkey still works without a restart (hold-dictate #1). If kbd-global fails, swap in handy-keys and re-run.
7. D-Bus `dev.holdkey.Broker1` (Register, Pressed/Released, Capabilities). Test: a `busctl --user` call registers an action, and its signals arrive in `busctl --user monitor`.
8. `holdkey doctor`: capability matrix plus a fix hint per failure ("GNOME 46: no GlobalShortcuts portal (added in xdg-desktop-portal-gnome 48) → evdev fallback, join `input` group"; "KWin/Mutter: no virtual-keyboard-v1 → use wdotool libei path"). Test: snapshot tests for GNOME 46, GNOME 48, KDE 6, Hyprland and niri.
9. AUR PKGBUILD plus the systemd user unit. Test: on a clean Arch VM, `makepkg -si` then `holdkey doctor` runs and reports.

### Evidence URLs carried over from the draft

- https://x.com/juhakall/status/2116865229380301285
- https://x.com/ABG__UB/status/2116789201228460084
- https://x.com/MeetKlaraAI/status/2116539853072429122
- https://x.com/Kizuno18/status/2115628256816959589
- https://github.com/anomalyco/hex/issues/781
- https://github.com/anomalyco/hex/issues/1023
- https://github.com/mkbula/sotto/issues/10
- https://github.com/ronb1964/TuxTalk/issues/26
- https://github.com/zburtle/Yaparr/issues/69
- https://github.com/Can-0f-Tuna/hold-dictate/issues/1
- https://github.com/obs-rust/obs-studio/issues/2636
- https://news.ycombinator.com/item?id=48436160
- Task references (verified to exist): https://github.com/anomalyco/hex/issues/485 , https://github.com/anomalyco/hex/issues/638
