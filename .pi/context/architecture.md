# Architecture Context

## Stack

| Layer | Technology | Role |
| --- | --- | --- |
| Language (core, daemon, CLI) | Rust, edition 2024, one Cargo workspace | All bulkhead logic and mechanics |
| Async runtime | tokio (`current_thread`) | Multiplexes `bhd` event sources |
| File watching | `notify` (+ optional `notify-debouncer-full`) | Status logs, plan inboxes |
| Serialization | `serde`, `serde_json`, `toml` | Config, task metadata, wakes, socket messages |
| CLI | `clap` | `bh` command surface |
| Errors and logs | `thiserror` (core), `anyhow` (binaries), `tracing` | Typed errors in logic, context in binaries |
| Pi extension | TypeScript, Pi extension API | `bh_*` tools, socket client to `bhd`, follow-up delivery, turn-end guard |
| Agent harness | Pi (pinned version) | Orchestrator and crew |
| Multiplexer | herdr (pinned; protocol 16+), Pi integration installed | Workspaces, tabs, panes, native agent state, push events |
| Worktrees | treehouse | Pooled task worktrees |
| Delivery | no-mistakes, gh-axi | PR gate, PR creation and merge, stacked PRs |
| Plan review | lavish-axi | HTML plan review and feedback polling |
| Memory | backpass | Per-repo training of committed context |
| Supervision host | launchd | Keeps `bhd` running |

## System Boundaries

- `crates/bh-core/` — Pure domain logic: types (status verbs, pane states, events, wakes, tasks, plans, config), `classify(state, event) -> (state, Vec<Wake>)`, stall checks, brief rendering, the `bhd` ↔ extension protocol types. No I/O, no async, no process spawning.
- `crates/bhd/` — The daemon: tokio event sources, the single core loop, wake batching and durable delivery, the Unix socket server for extensions, herdr event subscription, orchestrator revival, phone push.
- `crates/bh/` — The CLI: registry and home management, agent-folder builder, herdr adapter (create and address workspaces, tabs and panes by id), treehouse, gh-axi and no-mistakes wrappers, `spawn`, `send`, `status`, `land`, `teardown`, `plan`, `task`, `away`, `ls`, `doctor`.
- `extensions/bulkhead-orch/` — TypeScript Pi extension: registers `bh_*` tools that shell out to `bh`, connects to `bhd`, delivers wakes with `pi.sendUserMessage(..., { deliverAs: "followUp" })`, runs the turn-end guard on `agent_settled`. No supervision logic of its own.
- `prompts/` — Orchestrator contract, crew brief template, status protocol wording, triage rules. Text, versioned with the code.
- `shared/` — Installable opt-in Pi assets: `skills/spec-units/`, `prompts/bootstrap-context.md`.
- `launchd/` — The `bhd` launch agent plist.
- `tests/fixtures/` — Status logs, herdr event streams, plan files, and a scripted fake herdr CLI and socket.

## Storage Model

All state is plain files under `~/.bulkhead/`; there is no database.

- **`registry.toml`**: repo id → repo path, origin URL, herdr workspace id (absent until the first attach creates the workspace, Unit 04), delivery mode.
- **`shared/`**: one copy of opt-in skills and extensions, linked into repos by name.
- **`<repo-id>/config.toml`**: per-repo settings (`delivery`, `max_crew`, `stall_after`, `rebase_after`, away policy, opted-in shared skills, optional `+yolo`).
- **`<repo-id>/agent/`**: the repo's `PI_CODING_AGENT_DIR`. Symlinks for `AGENTS.md`, `auth.json`, `models.json` and `themes/` into `~/.pi/agent`; generated `settings.json` and `mcp.json`; per-repo `skills/`, `extensions/`, `prompts/`.
- **`<repo-id>/orchestrator.md`, `learnings.md`**: private repo notes and curated learnings.
- **`<repo-id>/plans/{inbox,accepted,declined}/`**: plans with triage state in frontmatter or a `.meta.toml` sidecar.
- **`<repo-id>/backlog/<task-id>.md`**: one work item per file, state in frontmatter.
- **`<repo-id>/tasks/<task-id>/`**: `brief.md`, append-only `status.log`, `meta.toml` (worktree path, herdr tab and pane ids, Pi session ref, PR URL, timestamps), `report.md` for scouts.
- **`<repo-id>/state/`**: `wakes.jsonl` (append-only, with delivered markers), `away.json` (present = away-mode on), `digest.md` (rolling orchestrator summary).
- **`run/`**: `bhd` socket and pid file, user-only permissions.
- **Committed in managed repos**: `context/architecture.md`, `context/project-overview.md`, and a pointer table in the repo's `AGENTS.md`. Nothing else bulkhead writes lands in a repo.

## Auth and Access Model

- bulkhead has no accounts. It runs as the captain's macOS user, and every file and socket it creates is readable and writable by that user only.
- GitHub access is whatever `gh` is authenticated as; bulkhead never stores tokens.
- Model credentials come from `~/.pi/agent/auth.json` through a symlink; bulkhead never copies or reads them.
- The orchestrator's authority is fixed by the contract: it may read the repo, triage plans, create backlog items, spawn, message and restart crew, and open PRs. Merging, discarding work, force-pushing, deleting branches, and changing `config.toml` or `learnings.md` need the captain's word. A per-repo `+yolo` flag may allow merge on green for that repo only.
- A crewmate may write only inside its own worktree and its own task folder.

## Invariants

1. `bh-core` performs no I/O, no async and no process spawning. Every decision about wakes, stalls and dispatch is a pure function tested without a runtime.
2. In `bhd`, the core loop is the only owner of mutable state. Event sources communicate only by sending `Event`s through the channel; there are no shared locks.
3. A wake is appended to `wakes.jsonl` before it is delivered and marked delivered only after the extension acknowledges it. Delivery is at-least-once, and every wake carries an id the extension dedupes on.
4. herdr workspaces, tabs and panes are addressed only by id, never by label.
5. The herdr server starts with a clean environment. `PI_CODING_AGENT_DIR` is set in each pane's launch command, never in the server's environment.
6. Every crewmate runs in a freshly created pane whose cwd is its worktree path. A relaunched orchestrator resumes its recorded Pi session with `--session <ref>`.
7. The orchestrator never edits files in a primary checkout. Only crewmates write code, and only in their own treehouse worktrees.
8. Nothing is merged, discarded, force-pushed or deleted without the captain's approval, away-mode included, except where a repo's `+yolo` flag allows merge on green.
9. bulkhead never writes to `~/.pi/agent`.
10. The `bhd` ↔ extension protocol carries a version, and both sides refuse an unknown major version instead of guessing.
11. External tools are invoked with argument arrays, never through a shell string.
