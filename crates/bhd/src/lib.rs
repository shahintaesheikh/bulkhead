//! `bhd` library surface and runtime entry point.

pub mod clock;
pub mod config;
pub mod core;
pub mod herdr;
pub mod paths;
pub mod push;
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

    let event_sender = sender.clone();
    let socket = herdr::default_socket();
    tokio::spawn(async move { herdr::run_events(socket, event_sender).await });

    let herdr_bin = std::env::var_os("BULKHEAD_HERDR_BIN").map_or_else(
        || std::path::PathBuf::from("herdr"),
        std::path::PathBuf::from,
    );
    let poll_secs = std::env::var("BULKHEAD_HERDR_POLL_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10);
    let poll_sender = sender.clone();
    tokio::spawn(async move {
        herdr::run_polling(herdr_bin, Duration::from_secs(poll_secs), poll_sender).await
    });

    let registry = paths::load_registry(&paths)?;
    let mut core = Core::new(paths.clone(), &registry, push::Push::from_env())?;

    let bh_bin = std::env::var_os("BULKHEAD_BH_BIN")
        .map_or_else(|| std::path::PathBuf::from("bh"), std::path::PathBuf::from);
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
                for (repo, repo_path) in core.needs_revival() {
                    core.note_revival(&repo)?;
                    let bin = bh_bin.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = std::process::Command::new(bin)
                            .arg("ensure")
                            .arg(repo_path)
                            .status();
                    });
                }
            }
            _ = &mut shutdown => {
                break;
            }
        }
    }

    Ok(())
}
