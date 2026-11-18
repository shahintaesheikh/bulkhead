//! Event sources: the filesystem watcher and the Unix socket server.

use std::path::PathBuf;

use anyhow::{Context, Result};
use bh_core::{Envelope, ProtocolMessage, RepoId};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use crate::socket::{read_envelope, write_envelope};

/// One input to the core loop.
pub enum SourceEvent {
    /// Paths the watcher reported changed.
    Filesystem(Vec<PathBuf>),
    /// An extension connected and is ready for wakes.
    Connected {
        /// The repo the client orchestrates.
        repo: RepoId,
        /// Channel the core writes wake envelopes to.
        sender: mpsc::UnboundedSender<Envelope>,
    },
    /// A message from a connected extension.
    Message {
        /// The client's repo.
        repo: RepoId,
        /// The frame received.
        message: Envelope,
    },
    /// The batch interval elapsed.
    Tick,
    /// Stop.
    Shutdown,
}

/// Starts a recursive watcher and forwards changed paths to the core.
pub fn spawn_watcher(
    home: PathBuf,
    sender: mpsc::UnboundedSender<SourceEvent>,
) -> Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result {
            let _ = sender.send(SourceEvent::Filesystem(event.paths));
        }
    })
    .context("creating the file watcher")?;
    watcher
        .watch(&home, RecursiveMode::Recursive)
        .with_context(|| format!("watching {}", home.display()))?;
    Ok(watcher)
}

/// Serves the extension socket forever.
pub async fn run_socket(path: PathBuf, sender: mpsc::UnboundedSender<SourceEvent>) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = std::fs::remove_file(&path);
    let listener =
        UnixListener::bind(&path).with_context(|| format!("binding {}", path.display()))?;
    restrict_permissions(&path);

    loop {
        let (stream, _) = listener.accept().await.context("accepting a client")?;
        let sender = sender.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_client(stream, sender).await {
                tracing::warn!("client connection ended: {error}");
            }
        });
    }
}

async fn handle_client(
    stream: UnixStream,
    sender: mpsc::UnboundedSender<SourceEvent>,
) -> Result<()> {
    let (read, write) = stream.into_split();
    let mut reader = BufReader::new(read);

    let Some(hello) = read_envelope(&mut reader).await? else {
        return Ok(());
    };
    let repo = match hello.message {
        ProtocolMessage::Hello { repo, .. } => repo,
        _ => return Ok(()),
    };

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Envelope>();
    let _ = sender.send(SourceEvent::Connected {
        repo: repo.clone(),
        sender: out_tx,
    });

    let mut writer = write;
    let writer_task = tokio::spawn(async move {
        while let Some(envelope) = out_rx.recv().await {
            if write_envelope(&mut writer, &envelope).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = read_envelope(&mut reader).await? {
        let _ = sender.send(SourceEvent::Message {
            repo: repo.clone(),
            message,
        });
    }

    writer_task.abort();
    Ok(())
}

fn restrict_permissions(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(path, permissions);
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}
