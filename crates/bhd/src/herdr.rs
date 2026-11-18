//! herdr pane-agent-status events, with a polling fallback.
//!
//! Confirmed against herdr 0.9.5, protocol 22 (`herdr api schema`):
//! `events.subscribe` takes `{ subscriptions: [{ type: "pane.agent_status_changed" }] }`
//! and notifications carry `{ event, data: { workspace_id, pane_id, agent_status } }`.
//!
//! The subscription is best-effort: if the socket is unavailable or the framing
//! changes, [`run_polling`] still reports pane states from `herdr pane list`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bh_core::{PaneId, PaneState, WorkspaceId};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use crate::source::SourceEvent;

/// The default herdr socket.
pub fn default_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("HERDR_SOCKET_PATH") {
        return PathBuf::from(path);
    }
    let home = std::env::var_os("HOME").map_or_else(PathBuf::new, PathBuf::from);
    home.join(".config/herdr/herdr.sock")
}

/// Maps a herdr agent-status token to a [`PaneState`].
pub fn pane_state_from_token(token: &str) -> Option<PaneState> {
    match token {
        "idle" => Some(PaneState::Idle),
        "working" => Some(PaneState::Working),
        "blocked" => Some(PaneState::Blocked),
        "done" => Some(PaneState::Done),
        "unknown" => Some(PaneState::Unknown),
        _ => None,
    }
}

/// Extracts a pane-status change from one socket frame, if it is one.
pub fn parse_frame(value: &Value) -> Option<(WorkspaceId, PaneId, PaneState)> {
    let event = value.get("event")?.as_str()?;
    if event != "pane.agent_status_changed" && event != "pane_agent_status_changed" {
        return None;
    }
    let data = value.get("data")?;
    let workspace = data.get("workspace_id")?.as_str()?;
    let pane = data.get("pane_id")?.as_str()?;
    let status = data.get("agent_status")?.as_str()?;
    let state = pane_state_from_token(status)?;
    Some((WorkspaceId::new(workspace), PaneId::new(pane), state))
}

/// Reads `herdr pane list` output into pane states.
pub fn parse_panes(value: &Value) -> Vec<(WorkspaceId, PaneId, PaneState)> {
    let mut panes = Vec::new();
    let Some(list) = value
        .get("result")
        .and_then(|result| result.get("panes"))
        .and_then(Value::as_array)
    else {
        return panes;
    };
    for pane in list {
        let Some(workspace) = pane.get("workspace_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = pane.get("pane_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(status) = pane.get("agent_status").and_then(Value::as_str) else {
            continue;
        };
        if let Some(state) = pane_state_from_token(status) {
            panes.push((WorkspaceId::new(workspace), PaneId::new(id), state));
        }
    }
    panes
}

/// Subscribes forever, reconnecting on any failure.
pub async fn run_events(socket: PathBuf, sender: mpsc::UnboundedSender<SourceEvent>) {
    loop {
        if let Err(error) = subscribe_once(&socket, &sender).await {
            tracing::debug!("herdr subscription unavailable: {error}");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn subscribe_once(socket: &Path, sender: &mpsc::UnboundedSender<SourceEvent>) -> Result<()> {
    let mut stream = UnixStream::connect(socket)
        .await
        .with_context(|| format!("connecting to {}", socket.display()))?;

    let request = json!({
        "id": "bhd-subscribe",
        "method": "events.subscribe",
        "params": { "subscriptions": [ { "type": "pane.agent_status_changed" } ] },
    });
    let mut line = request.to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).await?;
    stream.flush().await?;

    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();
    loop {
        buffer.clear();
        if reader.read_line(&mut buffer).await? == 0 {
            bail!("herdr closed the subscription");
        }
        let Ok(value) = serde_json::from_str::<Value>(&buffer) else {
            continue;
        };
        if let Some((workspace, pane, state)) = parse_frame(&value) {
            let _ = sender.send(SourceEvent::HerdrPane {
                workspace,
                pane,
                state,
            });
        }
    }
}

/// Polls `herdr pane list` forever and forwards pane states.
pub async fn run_polling(
    bin: PathBuf,
    interval: Duration,
    sender: mpsc::UnboundedSender<SourceEvent>,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let bin = bin.clone();
        let Ok(result) = tokio::task::spawn_blocking(move || poll_panes(&bin)).await else {
            continue;
        };
        let Ok(panes) = result else {
            continue;
        };
        for (workspace, pane, state) in panes {
            let _ = sender.send(SourceEvent::HerdrPane {
                workspace,
                pane,
                state,
            });
        }
    }
}

fn poll_panes(bin: &Path) -> Result<Vec<(WorkspaceId, PaneId, PaneState)>> {
    let output = Command::new(bin)
        .args(["pane", "list"])
        .output()
        .with_context(|| format!("running {} pane list", bin.display()))?;
    if !output.status.success() {
        bail!("herdr pane list failed");
    }
    let value: Value = serde_json::from_slice(&output.stdout).context("parsing herdr pane list")?;
    Ok(parse_panes(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_pane_status_frame() {
        let frame = json!({
            "event": "pane.agent_status_changed",
            "data": { "workspace_id": "w1", "pane_id": "w1:p2", "agent_status": "blocked" },
        });
        let (workspace, pane, state) = parse_frame(&frame).unwrap();
        assert_eq!(workspace.as_str(), "w1");
        assert_eq!(pane.as_str(), "w1:p2");
        assert_eq!(state, PaneState::Blocked);
    }

    #[test]
    fn ignores_unrelated_frames() {
        assert!(parse_frame(&json!({ "id": "1", "result": {} })).is_none());
        assert!(parse_frame(&json!({ "event": "tab.created", "data": {} })).is_none());
    }

    #[test]
    fn parses_pane_list() {
        let value = json!({
            "result": { "panes": [
                { "workspace_id": "w1", "pane_id": "w1:p1", "agent_status": "working" },
                { "workspace_id": "w1", "pane_id": "w1:p2", "agent_status": "done" },
            ] },
        });
        let panes = parse_panes(&value);
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].2, PaneState::Working);
    }
}
