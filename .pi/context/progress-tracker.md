# Progress Tracker

Update this file after every meaningful implementation change.

## Current Phase

- M0 (Skeleton) in progress. Units 01–05 build the workspace, core logic, `bh init`, the herdr adapter and the extension skeleton.

## Current Goal

- Unit 02: status-log parser and `classify(state, event)` in `bh-core`, with fixture tests for every verb.

## Completed

- Design spec: per-repo orchestrator, layout, `bh` command surface, orchestrator contract and dispatch policy, crew lifecycle and status protocol, plan ingress and triage, repo context, away-mode, persistence, Rust implementation plan, build order.
- Unit 01: Cargo workspace (`bh-core`, `bhd`, `bh`), `rust-toolchain.toml` pinned to 1.99.1, domain types (ids, time, `StatusVerb`, `PaneState`, `Event`, `Wake`/`WakeRecord`/`WakeKind`, `Task`, `Registry`/`RepoConfig`), versioned `bhd` ↔ extension protocol (`Envelope`, `ProtocolError`), serde round-trip tests. Spec: `specs/01-cargo-workspace.md`.

## In Progress

- None yet.

## Next Up

- Unit 02: status-log parser and classifier with fixtures.

## Open Questions

- Does a pooled treehouse worktree inherit Pi's project trust from the main clone, or will an unattended crewmate stall on a trust prompt?
- How does herdr frame messages on its socket? Confirm in herdr's socket API docs before Unit 04.
- Which Pi and herdr versions to pin? `agent_settled`, `sendUserMessage({ deliverAs: "followUp" })` and `session_start` are the hooks bulkhead depends on.
- Delivery default: no-mistakes everywhere, or direct PR for BytePlus PoC repos?
- Stall threshold: 20 minutes is a guess; tune from M4 logs.
- Phone push channel: ntfy, Pushover, or a Slack DM?
- Token cost per wake across several persistent orchestrators; does away-mode need a cheaper model?
- Is per-repo isolation also a security boundary for BytePlus repos (separate gh tokens, ModelArk keys), or only context hygiene?
- After each Pi upgrade: do loaded extensions match the agent folder plus `-e` paths? (Pi issue #776 once loaded extensions despite `--no-extensions`.)
- Confirm the Unit 01 `WakeKind` set (`PlanInbox`, `NeedsDecision`, `Blocked`, `PrOpened`, `TaskDone`, `TaskStalled`, `CrewGone`, `OrchestratorRevived`, `AwayBrief`) as the full list; it was derived from the feature list.
- Confirm `rebase_after = 1h`; the context names the setting but gives no value.
- Does bulkhead need an explicit "pane gone" state, or does herdr signal pane closure separately from the five agent states?

## Architecture Decisions

- **One orchestrator per repo, not one fleet liaison.** The captain's pain was addressing overhead, not context pollution; a repo-bound orchestrator makes dispatch deterministic.
- **Repo homes outside git (`~/.bulkhead/<repo-id>/`).** Git-excluded files never reach treehouse worktrees; absolute paths do, and nothing is committed by accident.
- **Per-repo `PI_CODING_AGENT_DIR`.** Pi resolves its whole global layer from one folder; symlinking global `AGENTS.md`, credentials, models and themes keeps L0 while isolating skills, MCP servers and extensions per repo. Disable flags were rejected: `-nc` also drops the repo's `AGENTS.md`, `-ne` also drops declared packages.
- **herdr instead of tmux.** Native per-pane agent state through Pi's lifecycle hooks, push events, and a sidebar that doubles as the cross-repo dashboard. One shared session, one workspace per repo.
- **Rust daemon (`bhd`) instead of an in-process watcher.** Wakes survive orchestrator crashes and `bhd` can revive orchestrators; chosen from day one partly to learn Rust. The TypeScript extension stays transport and guard only.
- **Pure core, async shell.** `bh-core` holds all decisions as pure functions; tokio only moves events.
- **Dispatch policy.** Parallel across `architecture.md` boundaries, sequential within one, stacked PRs for dependencies, scouts uncapped; `max_crew` defaults to 3, sized to review capacity.
- **Managed repos commit two context files.** `architecture.md` and `project-overview.md`, read on demand through a pointer table. Progress trackers, workflow rules and UI files are not used in managed repos.
- **Unit specs use a fixed format.** `Goal`, `Deliverables`, `Design`, `Out of Scope`, `Verify`, under a header with milestone, depends-on and boundary. `00-build-plan.md` is the index; each unit gets `NN-name.md` written before implementation. Proposed in Unit 01 and open to the captain's change.

## Session Notes

- The design spec ("bulkhead — per-repo Pi orchestrator spec") is the origin of these files. These context files now supersede it.
- firstmate's `docs/herdr-backend.md` and `bin/fm-brief.sh` are the best references for herdr edge cases and status-protocol wording.
- Unit 01 landed. The repo was not a git checkout, so no commit was made; the captain owns `git init`. `rust-toolchain.toml` pins 1.99.1, installed on first build. `bh-core` ids use one `string_id!` macro for eight newtypes; flagged for review against the "no macro tricks" standard.
- Unit 01 review points: `WakeKind` members, `rebase_after` default, and the protocol envelope shape (no `flatten`, so messages nest under `"message"`).
