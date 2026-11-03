//! Shared types for holdkey: the backend trait, capability reporting and the
//! press/release tracker built on `kbd`.

mod backend;
pub mod doctor;
pub mod hyprland;
mod tracker;

pub use backend::{Action, Capability, MockBackend, ShortcutBackend, ShortcutEvent, now_us};
pub use tracker::{Edge, HoldTracker};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad trigger {trigger:?}: {reason}")]
    BadTrigger { trigger: String, reason: String },
    #[error("duplicate trigger {0:?}")]
    DuplicateTrigger(String),
    #[error("backend: {0}")]
    Backend(String),
}

/// True if at least one `/dev/input/event*` can be opened for reading (i.e. we are in the `input` group).
pub fn input_readable() -> bool {
    std::fs::read_dir("/dev/input").is_ok_and(|d| {
        d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("event") && std::fs::File::open(e.path()).is_ok())
    })
}

pub type Result<T> = std::result::Result<T, Error>;
