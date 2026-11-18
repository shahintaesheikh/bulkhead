//! `bhd` library surface and runtime entry point.

pub mod clock;
pub mod core;
pub mod paths;
pub mod socket;
pub mod source;
pub mod wake_store;

pub use core::Core;
pub use paths::Paths;
pub use source::SourceEvent;

use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;

/// Runs the daemon: watcher, socket server, batcher and core loop.
pub async fn run(paths: Paths, batch: Duration) -> Result<()> {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let _watcher = source::spawn_watcher(paths.home.clone(), sender.clone())?;

    let socket_path = paths.socket_file();
    let socket_sender = sender.clone();
    tokio::spawn(async move {
        if let Err(error) = source::run_socket(socket_path, socket_sender).await {
            tracing::error!("socket server stopped: {error}");
        }
    });

    let registry = paths::load_registry(&paths)?;
    let mut core = Core::new(paths.clone(), registry.repos.keys().cloned())?;
    let mut ticker = tokio::time::interval(batch);
    let mut shutdown = Box::pin(tokio::signal::ctrl_c());

    loop {
        tokio::select! {
            Some(event) = receiver.recv() => {
                let stop = matches!(event, SourceEvent::Shutdown);
                core.handle(event)?;
                if stop {
                    break;
                }
            }
            _ = ticker.tick() => {
                core.handle(SourceEvent::Tick)?;
            }
            _ = &mut shutdown => {
                break;
            }
        }
    }

    Ok(())
}
