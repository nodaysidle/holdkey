//! Hyprland IPC: read `globalshortcuts` and flag app-ID drift.
//!
//! One connection per request: open, write, read to EOF, close. Hyprland can
//! freeze for up to 5 s if a client leaves a connection open.

use std::{io::{self, Read, Write}, os::unix::net::UnixStream, path::{Path, PathBuf}, time::Duration};

use crate::{Action, Capability};

pub fn socket_path() -> Option<PathBuf> {
    let rt = std::env::var_os("XDG_RUNTIME_DIR")?;
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(Path::new(&rt).join("hypr").join(sig).join(".socket.sock"))
}

pub fn request(socket: &Path, command: &str) -> io::Result<String> {
    let mut s = UnixStream::connect(socket)?;
    s.set_read_timeout(Some(Duration::from_secs(2)))?;
    s.set_write_timeout(Some(Duration::from_secs(2)))?;
    s.write_all(command.as_bytes())?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    Ok(out) // `s` drops here, closing the connection.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub app_id: String,
    pub id: String,
    pub description: String,
}

/// Parse `appid:shortcut-id -> description` lines.
pub fn parse_shortcuts(text: &str) -> Vec<Listed> {
    text.lines()
        .filter_map(|l| {
            let (name, description) = l.split_once(" -> ")?;
            let (app_id, id) = name.split_once(':')?;
            Some(Listed { app_id: app_id.into(), id: id.into(), description: description.into() })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drift {
    /// Nothing is registered under `app_id:action` (and nothing under another app ID either).
    Missing { action: String },
    /// The shortcut exists, but under a different app ID than expected.
    WrongAppId { action: String, found: String },
}

impl Drift {
    pub fn describe(&self, app_id: &str) -> String {
        match self {
            Self::Missing { action } => format!(
                "no Hyprland global shortcut `{app_id}:{action}`; add `bind = <keys>, global, {app_id}:{action}` to hyprland.conf and make sure the app registered it"
            ),
            Self::WrongAppId { action, found } => format!(
                "shortcut `{action}` is registered as `{found}:{action}` but the app ID is `{app_id}`; fix the app ID in hyprland.conf or the registration"
            ),
        }
    }
}

pub fn check_drift(listed: &[Listed], app_id: &str, actions: &[Action]) -> Vec<Drift> {
    actions
        .iter()
        .filter(|a| !listed.iter().any(|l| l.app_id == app_id && l.id == a.id))
        .map(|a| match listed.iter().find(|l| l.id == a.id) {
            Some(l) => Drift::WrongAppId { action: a.id.clone(), found: l.app_id.clone() },
            None => Drift::Missing { action: a.id.clone() },
        })
        .collect()
}

pub fn list_shortcuts() -> io::Result<Vec<Listed>> {
    let path = socket_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not running under Hyprland"))?;
    Ok(parse_shortcuts(&request(&path, "globalshortcuts")?))
}

pub fn capability() -> Capability {
    match list_shortcuts() {
        Ok(l) => Capability::ok(format!("Hyprland IPC reachable, {} global shortcuts registered", l.len())),
        Err(e) => Capability::broken(
            format!("Hyprland IPC unavailable: {e}"),
            "run holdkeyd inside the Hyprland session so HYPRLAND_INSTANCE_SIGNATURE is set (systemd --user import-environment)",
        ),
    }
}
