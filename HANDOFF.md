# holdkey — build status (2026-11-03)

Built from `SPEC.md` §5 (verdict PIVOT). Workspace: `holdkey-core`, `holdkeyd`, `holdkey-cli` (binary `holdkey`).
`cargo test --workspace`: 17 pass. Packaging in `packaging/` (PKGBUILD builds from this tree).

## Per-task status (machine: Arch, Hyprland 0.56.2, GlobalShortcuts portal v1 via xdph)

| # | Status | Evidence / gap |
|---|---|---|
| 1 | verified | `cargo build --workspace` passes; GitHub Actions CI green on first push (run 41727978563, ubuntu, build + 17 tests) |
| 2 | verified (HoldTracker only) | `tests/replay.rs`: mock + Hex #485/#638 replays. Neither live backend uses HoldTracker (portal uses Deactivated, evdev uses 5 ms polling), so this is not coverage of them. Release logic is holdkey's own thin layer on `kbd` (kbd only matches presses) |
| 3 | **verified on Hyprland/xdph; not KDE/GNOME 48+** | Live hold: one `pressed`, one `released` ~5.9 s later (epoch us). Needs ONE plain `bind = ..., global, dev.holdkey.Broker:ptt` in hyprland (a second `release = true` bind double-fires Released). xdph timestamps are seconds that ashpd reads as ms, so the daemon stamps events on arrival. An earlier note here claiming failure was a misread of that timestamp. |
| 4 | verified (stub) | `tests/approvals.rs`. Two-phase bind: previously approved shortcuts re-bound first, cancel of new ones can't unbind them. obs-rust #2636 text was not read; behavior inferred from the spec line |
| 5 | **partly** | Wrong app ID reported by `holdkey doctor --app-id X --action ptt`. **Held Alt+Space reaching `watch` not tested** (Alt+Space is owned by omacast here; `wtype` injection did not trigger binds). Hyprland is 0.56.2, spec says 0.57 |
| 6 | **verified on Hyprland (evdev forced via HOLDKEYD_BACKEND=evdev); not on GNOME 46/niri** | Live: hold1 pressed->released 4.64 s, USB keyboard unplugged/replugged, hold2 6.1 s, both clean, no duplicates, no restart. Earlier notes: `evdev.rs` via kbd-global. **Replug test not run** (needs a physical keyboard; `/dev/uinput` is root-only). Release is detected by 5 ms polling of kbd-global key state (kbd-global only reports presses) |
| 7 | verified | `busctl --user call ... Register` OK; `Pressed` seen in `busctl --user monitor` |
| 8 | verified | snapshot tests: GNOME 46, GNOME 48, KDE 6, Hyprland, niri |
| 9 | verified (container) | Clean `archlinux` Docker container: `makepkg -si --noconfirm` built (check() ran tests) and installed `/usr/bin/holdkey{,d}`, the user unit and the desktop file; `holdkey --version` and `holdkey doctor` ran and reported. Not a full VM (no systemd boot), so enabling the unit was not tested |

## Things to know
- The portal needs an app ID with a matching `.desktop` file. holdkeyd registers as `dev.holdkey.Broker` for all client apps; the PKGBUILD ships `dev.holdkey.Broker.desktop`. Without it: "App info not found".
- `ashpd` 0.13.13 pulls zbus 5.19, the daemon uses zbus 6.2.1 (two copies). The portal connection uses ashpd's zbus.
- `Pressed/Released` timestamps from the portal are the compositor clock, not epoch; evdev/mock use epoch µs.
- Action ids are global across apps; `Register` of an already-registered app_id is rejected (no unregister in MVP).
- PKGBUILD has no `source=`/`url=`; it builds the local tree. Needs a real source URL before AUR submission.

## To finish verification (needs you)
1. Hold a key: `holdkeyd &`, `holdkey bind org.example.T ptt F13`, add a `global` bind plus a `release = true` bind for `dev.holdkey.Broker:ptt` in hyprland.lua, run `holdkey watch`, hold and release the key: expect `pressed` then `released`.
   (`dev.holdkey.Broker.desktop` must exist in `~/.local/share/applications` or via the package.)
2. evdev replug (task 6) on GNOME 46 or niri, or here with `sudo` and a USB keyboard.
3. KDE 6 / GNOME 48 portal run (task 3). 4. Clean-VM `makepkg -si` (task 9).

## Leftovers
- `dev.holdkey.Broker:ptt` stays listed in `hyprctl globalshortcuts` after holdkeyd is killed (xdph keeps the session; holdkeyd doesn't close sessions on SIGTERM). Clears on `systemctl --user restart xdg-desktop-portal-hyprland`, which also drops other apps' shortcuts until they re-register - your call.

## State as of 2026-11-03 (end of session)
- Released: v0.1.0, public repo https://github.com/nodaysidle/holdkey, CI green, task 9 verified in a clean Arch container.
- Verified: Hyprland 0.56.2 (portal and evdev holds, USB replug). Untested: KDE Plasma 6, GNOME 46/48+, niri; held Alt+Space (task 5).
- Blocked on: AUR account registration is temporarily closed (HTTP 503 notice, 2026-11-03). Announcement list: aur-general. Do not script retries. Users install from source (see README).
- Waiting for: the AUR to reopen, and any volunteer with KDE/GNOME for `holdkey doctor` + an F12 hold test.

## AUR submission, fish shell (do this once registration reopens)
1. Register at https://aur.archlinux.org/register, then in "My Account" paste your SSH public key:
```fish
test -f ~/.ssh/id_ed25519; or ssh-keygen -t ed25519 -C "nodaysidle@tutanota.de"
cat ~/.ssh/id_ed25519.pub
```
2. Check the package name is free, then submit v0.1.0:
```fish
curl -s "https://aur.archlinux.org/rpc/v5/info?arg[]=holdkey" | jq '.resultcount'   # 0 = free
git clone ssh://aur@aur.archlinux.org/holdkey.git ~/dev/nodaysidle/aur-holdkey
cd ~/dev/nodaysidle/aur-holdkey
cp ~/dev/nodaysidle/holdkey/packaging/PKGBUILD.aur PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "holdkey 0.1.0"
git push
```
3. Later releases: `~/dev/nodaysidle/holdkey/packaging/release.sh 0.2.0` (needs `set -x NDS_ALLOW_PUSH 1` for your push hook), copy the new `PKGBUILD.aur` to the AUR clone, regenerate `.SRCINFO`, commit, push.
