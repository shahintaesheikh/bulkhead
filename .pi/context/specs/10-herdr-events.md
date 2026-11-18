# Unit 10 — herdr Pane-Event Subscription with Polling Fallback

- **Milestone:** M4
- **Depends on:** Unit 09
- **Boundary:** `bhd` herdr integration. No stalls or away policy (Unit 12).

## Goal

Notice when a crewmate's herdr pane changes agent state, so the daemon can
react without the crew writing a status line — and keep noticing even if the
event socket is unavailable.

## Deliverables

1. `bhd::herdr` event subscriber over the herdr socket.
2. A polling fallback that reads `herdr pane list` every
   `BULKHEAD_HERDR_POLL_SECS` (default 10).
3. `SourceEvent::HerdrPane`, and workspace-to-repo mapping in `Core`.
4. Unit tests for frame parsing, pane-list parsing and the core mapping.

## Design

### Subscription

`events.subscribe` takes `{ subscriptions: [{ type: "pane.agent_status_changed" }] }`
and notifications carry `{ event, data: { workspace_id, pane_id, agent_status } }`
(confirmed against herdr 0.9.5, protocol 22). The subscriber connects to
`HERDR_SOCKET_PATH` (default `~/.config/herdr/herdr.sock`), sends the
subscription, and forwards each change. Any failure — no socket, a protocol
change — is logged and retried after two seconds; it never stops the daemon.

### Polling fallback

`run_polling` runs `herdr pane list` on a blocking task, parses
`.result.panes[].{workspace_id,pane_id,agent_status}` and forwards the same
events. It is always on, so a broken subscription only slows reaction, not
supervision.

### Workspace-to-repo mapping

Panes are workspace-qualified (`w1:p1`) and the registry binds a workspace id
to a repo, so `Core` builds `workspace -> repo` at startup and translates a
`HerdrPane` event into `Event::PaneStateChanged`. An unknown workspace is
ignored.

Pane-state changes update `RepoState.panes` without producing a wake; wake
decisions from pane state (a gone crewmate, a blocked pane) belong to Unit 12.

## Out of Scope

- Stall detection and crew-gone wakes (Unit 12).
- Acting on pane state beyond recording it.

## Verify

- [ ] A `pane.agent_status_changed` frame parses to workspace, pane and state.
- [ ] Unrelated frames and unknown status tokens are ignored.
- [ ] A `HerdrPane` event maps to its repo and produces no wake.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
