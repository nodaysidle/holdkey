//! `$XDG_RUNTIME_DIR/holdkey.sock`: one JSON event per line, for `holdkey watch`.

use std::path::{Path, PathBuf};

use holdkey_core::ShortcutEvent;
use tokio::{io::AsyncWriteExt, net::UnixListener, sync::broadcast};

pub fn default_path() -> Option<PathBuf> {
    Some(Path::new(&std::env::var_os("XDG_RUNTIME_DIR")?).join("holdkey.sock"))
}

pub async fn serve(path: PathBuf, hub: broadcast::Sender<ShortcutEvent>) -> std::io::Result<()> {
    let _ = std::fs::remove_file(&path); // stale socket from a previous run
    let listener = UnixListener::bind(&path)?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut rx = hub.subscribe();
        tokio::spawn(async move {
            while let Ok(ev) = rx.recv().await {
                let Ok(mut line) = serde_json::to_vec(&ev) else { continue };
                line.push(b'\n');
                if stream.write_all(&line).await.is_err() {
                    break; // client went away
                }
            }
        });
    }
}
