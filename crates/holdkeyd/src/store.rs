//! Persisted portal approvals (`portal-sessions.json`).

use std::{collections::BTreeMap, fs, io, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approved {
    pub description: String,
    pub trigger: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    /// action id -> what the portal last approved.
    approved: BTreeMap<String, Approved>,
}

pub struct Store(PathBuf);

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// `$XDG_STATE_HOME/holdkey/portal-sessions.json`, falling back to `~/.local/state`.
    pub fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/state")))?;
        Some(base.join("holdkey/portal-sessions.json"))
    }

    /// A missing file is an empty store; a corrupt one is an error so approvals are never silently dropped.
    pub fn load(&self) -> io::Result<BTreeMap<String, Approved>> {
        match fs::read(&self.0) {
            Ok(b) => serde_json::from_slice::<File>(&b)
                .map(|f| f.approved)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(e),
        }
    }

    /// Merge `new` over the stored approvals. Never removes entries.
    pub fn merge(&self, new: impl IntoIterator<Item = (String, Approved)>) -> io::Result<()> {
        let mut approved = self.load()?;
        approved.extend(new);
        if let Some(dir) = self.0.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.0.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(&File { approved })?)?;
        fs::rename(tmp, &self.0)
    }
}
