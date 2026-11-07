use std::{collections::HashMap, process::ExitCode};

use clap::{Parser, Subcommand};
use holdkey_core::{Action, Capability, doctor::{self, Environment}, hyprland};
use tokio::{io::{AsyncBufReadExt, BufReader}, net::UnixStream};
use zbus::Connection;

const BROKER: &str = "dev.holdkey.Broker1";
const PATH: &str = "/dev/holdkey/Broker1";
/// The app ID holdkeyd uses with the portal; Hyprland names its shortcuts `<this>:<action>`.
const PORTAL_APP_ID: &str = "dev.holdkey.Broker";

#[derive(Parser)]
#[command(name = "holdkey", version, about = "Global press/release shortcuts on Wayland")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Explain which backends work here and how to fix the ones that don't.
    Doctor {
        /// App ID Hyprland should list the actions under (checked for drift).
        #[arg(long, default_value = PORTAL_APP_ID)]
        app_id: String,
        /// Action ids to check for drift on Hyprland (repeatable).
        #[arg(long = "action")]
        actions: Vec<String>,
        /// Test PTT hold/release capability for a trigger (e.g. `space`, `Right Alt`).
        /// Exits 0 if capable, 3 if no path exists.
        #[arg(long, num_args = 0..=1, default_missing_value = "space")]
        ptt: Option<String>,
    },
    /// Print Pressed/Released events as JSON lines.
    Watch,
    /// Register one action with the running daemon.
    Bind {
        app_id: String,
        action_id: String,
        /// kbd trigger syntax, e.g. `Ctrl+Space` or `F13`.
        trigger: String,
        #[arg(long, default_value = "holdkey action")]
        description: String,
    },
    /// Show the daemon's backend capabilities.
    Status,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Cli::parse().cmd).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("holdkey: {e}");
            ExitCode::FAILURE
        }
    }
}

type Res<T> = Result<T, Box<dyn std::error::Error>>;

async fn run(cmd: Cmd) -> Res<ExitCode> {
    match cmd {
        Cmd::Doctor { app_id, actions, ptt } => {
            let env = make_env_here(&app_id, &actions, ptt.clone()).await;
            let rows = doctor::diagnose(&env);
            print!("{}", doctor::render(&rows));
            if let Some(ref trig) = ptt {
                if doctor::ptt_capable(&env, trig) {
                    Ok(ExitCode::SUCCESS)
                } else {
                    Ok(ExitCode::from(3u8))
                }
            } else {
                // Fail when nothing can deliver shortcuts, or a Hyprland binding has drifted.
                let delivers = |name: &str| rows.iter().any(|(n, c)| n == name && c.works);
                let drifted = rows.iter().any(|(n, _)| n == "drift");
                Ok(if drifted || !(delivers("portal") || delivers("evdev")) { ExitCode::FAILURE } else { ExitCode::SUCCESS })
            }
        }
        Cmd::Watch => {
            let path = std::path::Path::new(&std::env::var("XDG_RUNTIME_DIR")?).join("holdkey.sock");
            let stream = UnixStream::connect(&path)
                .await
                .map_err(|e| format!("cannot connect to {}: {e} (is holdkeyd running?)", path.display()))?;
            let mut lines = BufReader::new(stream).lines();
            while let Some(line) = lines.next_line().await? {
                println!("{line}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Bind { app_id, action_id, trigger, description } => {
            let session: String = broker().await?
                .call("Register", &(app_id, vec![(action_id, description, trigger)]))
                .await
                .map_err(|e| format!("Register failed: {e}"))?;
            println!("{session}");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Status => {
            let caps: HashMap<String, (bool, String, String)> = broker().await?
                .get_property("Capabilities")
                .await
                .map_err(|e| format!("cannot read Capabilities: {e}"))?;
            let mut rows: Vec<_> = caps.into_iter().map(|(n, (w, r, f))| (n, Capability { works: w, reason: r, fix_hint: f })).collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            print!("{}", doctor::render(&rows));
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn broker() -> Res<zbus::Proxy<'static>> {
    let conn = Connection::session().await?;
    let proxy = zbus::Proxy::new(&conn, BROKER, PATH, BROKER)
        .await
        .map_err(|e| format!("cannot reach {BROKER}: {e} (is holdkeyd running?)"))?;
    Ok(proxy)
}

/// Probe this machine and run the pure diagnosis.
async fn make_env_here(app_id: &str, action_ids: &[String], ptt_trigger: Option<String>) -> Environment {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let portal_global_shortcuts = async {
        let conn = Connection::session().await.ok()?;
        let p = zbus::Proxy::new(&conn, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.GlobalShortcuts").await.ok()?;
        p.get_property::<u32>("version").await.ok()
    }
    .await;
    let listed = hyprland::list_shortcuts();
    let actions: Vec<Action> = action_ids
        .iter()
        .map(|id| Action { id: id.clone(), description: String::new(), preferred_trigger: String::new() })
        .collect();
    let drift = listed
        .as_ref()
        .map(|l| hyprland::check_drift(l, app_id, &actions).iter().map(|d| d.describe(app_id)).collect())
        .unwrap_or_default();
    Environment {
        gnome_shell_major: gnome_shell_major(&desktop),
        desktop,
        portal_global_shortcuts,
        input_readable: holdkey_core::input_readable(),
        hyprland_ipc: listed.is_ok(),
        drift,
        ptt_trigger,
    }
}

fn gnome_shell_major(desktop: &str) -> Option<u32> {
    if !desktop.to_ascii_lowercase().contains("gnome") {
        return None;
    }
    let out = std::process::Command::new("gnome-shell").arg("--version").output().ok()?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().last()?.split('.').next()?.parse().ok()
}
