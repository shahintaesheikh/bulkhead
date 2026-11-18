# bulkhead

## Overview

bulkhead gives every git repo its own persistent Pi orchestrator that parallelizes work inside that repo only. The pane you type into is the address: the orchestrator for a repo can only dispatch into that repo, so prompts never carry routing. Each repo gets an isolated Pi agent folder (its own skills, MCP servers and extensions on top of the captain's global `AGENTS.md`), a home outside git for plans and task state, and a crew of Pi agents in treehouse worktrees, all laid out as herdr workspaces. A Rust daemon, `bhd`, watches every repo at zero token cost and wakes an orchestrator only when a crewmate finishes, blocks or stalls, so work keeps moving while the captain is away. It is built for one developer (the captain) on macOS who runs several repos at once and wants per-repo visibility instead of one fleet-wide liaison.

## Goals

1. Dispatch into a repo without naming the repo: chat in its orchestrator tab, or run `bh task "<ask>"` from any shell inside it.
2. Get a plan from anywhere (Claude desktop, a lavish HTML artifact, a plain `.md` file) into a repo's inbox with no git push, and have every plan explicitly accepted or declined.
3. Keep each repo's agent context isolated: global `AGENTS.md` everywhere, but skills, MCP servers and extensions only where the repo opts in.
4. Run up to `max_crew` (default 3) crewmates per repo in parallel across system boundaries, sequentially within one.
5. Supervise at zero tokens while idle, and survive orchestrator crashes and herdr restarts without losing a wake.
6. Learn Rust by building a real async daemon and CLI.

## Core User Flow

1. The captain runs `bh init` once in a repo. bulkhead registers it, creates `~/.bulkhead/<repo-id>/` and builds the repo's agent folder.
2. The captain runs `bh`. bulkhead attaches the shared `bulkhead` herdr session, focuses the repo's workspace, and starts the orchestrator in its `orch` tab if it is not running.
3. The captain drops a plan with `bh plan spec.md` (or Claude desktop writes it into the inbox) or adds work with `bh task "<ask>"`.
4. The orchestrator triages each plan into backlog items or declines it with a reason, and writes a brief in the spec format for each task.
5. The orchestrator spawns crewmates by the dispatch policy: each claims a treehouse worktree and runs Pi in a fresh herdr pane with the repo's agent folder.
6. Crewmates append status lines (`working:`, `needs-decision:`, `blocked:`, `pr:`, `done:`). `bhd` classifies them, batches actionable ones, and wakes the orchestrator.
7. The orchestrator answers decisions it is allowed to make, escalates the rest to the captain, and reports PRs.
8. The captain approves; `bh land` merges via gh-axi and tears the task down.
9. Before leaving, the captain runs `bh away on`. On return, `bh away off` prints what landed, what waits on the captain, and what failed.

## Features

### Per-repo orchestration

- Persistent Pi orchestrator per repo, bound to that repo by its herdr workspace
- Orchestrator contract injected by the `bulkhead-orch` Pi extension, never written into the repo
- Authority table: the orchestrator dispatches, messages and restarts crew alone; merging and discarding need the captain
- Dispatch policy: parallel across `architecture.md` boundaries, sequential within one, stacked PRs for dependent tasks, scouts uncapped

### Crew lifecycle

- `bh spawn` claims a treehouse worktree, renders `brief.md`, opens a herdr tab with a fresh pane, and launches Pi
- Append-only status protocol per task
- Delivery through no-mistakes, direct PR via gh-axi, or local-only branches
- `bh land` and `bh teardown` return worktrees to the treehouse pool

### Context isolation

- Per-repo `PI_CODING_AGENT_DIR` with symlinked global `AGENTS.md`, `auth.json`, `models.json` and themes
- Per-repo `settings.json`, `mcp.json`, `skills/`, `extensions/` and `prompts/`
- Opt-in shared skills from `~/.bulkhead/shared/`

### Plans

- `bh plan` and a watched inbox for `.md` and lavish `.html` plans
- Triage states: inbox, accepted, needs-review, declined, superseded
- lavish-axi review loop for HTML plans

### Supervision and away-mode

- `bhd`: one tokio daemon per machine watching status logs, inboxes, `lavish-axi poll` and herdr pane events
- Durable wakes in `wakes.jsonl`, batched about every 30 seconds, delivered over a Unix socket to the extension
- Turn-end guard, stall detection, away policy, phone push, return brief
- Orchestrator revival by `bhd`; launchd keeps `bhd` alive

### Repo context tooling

- `/bootstrap-context` Pi prompt template that writes a managed repo's `architecture.md`, `project-overview.md` and pointer table
- `spec-units` Pi skill loaded during plan triage
- backpass per-repo training of committed context

## Scope

### In Scope

- macOS, one captain, many repos
- Pi as the only harness, for both orchestrator and crew
- herdr as the only multiplexer
- treehouse, no-mistakes, gh-axi, lavish-axi and backpass used as-is through their CLIs
- `bh` CLI and `bhd` daemon in Rust; `bulkhead-orch` extension in TypeScript

### Out of Scope

- Cross-repo coordination or linked tasks (handled manually; revisit if it becomes weekly)
- A fleet-wide liaison agent that dispatches across repos (`bh ls` is read-only)
- Harnesses other than Pi (Claude Code, Codex, opencode)
- tmux or any multiplexer other than herdr
- Linux and Windows support
- Remote machines or multiple captains
- A cheaper-model side branch for routine away wakes (deferred to "Later")
- Writing anything into `~/.pi/agent`
- A web or GUI dashboard

## Success Criteria

1. `bh` from any registered repo lands the captain in that repo's herdr workspace with a running orchestrator.
2. Three crew tasks in different boundaries of one repo run in parallel and all tear down cleanly, with worktrees back in the treehouse pool.
3. A PR goes from `bh task` to merged with the captain never typing into a crew pane.
4. A plan written by Claude desktop and a lavish artifact each become backlog items with no git push.
5. A repo opted into a LangGraph skill loads it; a second repo running at the same time does not.
6. For a full working day, the captain never polls a crew pane to find out whether it needs attention.
7. A four-hour absence with live tasks ends in an accurate return brief, with no lost or duplicated wakes, even if an orchestrator was killed during it.
