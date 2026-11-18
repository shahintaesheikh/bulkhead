# Progress Tracker

Update this file after every meaningful implementation change.

## Current Phase

- M0 (Skeleton) in progress. Units 01–05 build the workspace, core logic, `bh init`, the herdr adapter and the extension skeleton.

## Current Goal

- Unit 07: delivery modes, `bh land` behind approval via gh-axi, stacked PRs for dependent tasks.

## Completed

- Design spec: per-repo orchestrator, layout, `bh` command surface, orchestrator contract and dispatch policy, crew lifecycle and status protocol, plan ingress and triage, repo context, away-mode, persistence, Rust implementation plan, build order.
- Unit 01: Cargo workspace (`bh-core`, `bhd`, `bh`), `rust-toolchain.toml` pinned to 1.99.1, domain types (ids, time, `StatusVerb`, `PaneState`, `Event`, `Wake`/`WakeRecord`/`WakeKind`, `Task`, `Registry`/`RepoConfig`), versioned `bhd` ↔ extension protocol (`Envelope`, `ProtocolError`), serde round-trip tests. Spec: `specs/01-cargo-workspace.md`.
- Unit 02: `StatusLine`/`StatusLine::parse` with typed errors, `SupervisionState`/`RepoState`/`TaskState`, pure `classify(state, event)` with wake rules, status fixtures, parser and classifier tests. Spec: `specs/02-status-classifier.md`.
- Unit 03: `bh` lib+bin split; `bh init` (git detection, slug, registry, repo home, generated `config.toml`/`settings.json`/`mcp.json`, global symlinks); `bh doctor` (registry, config, JSON, broken symlinks, stray `AGENTS.md`); temp-home integration tests. `RegistryEntry.workspace` is now `Option`. Spec: `specs/03-init-agent-folder.md`.
- Unit 04: `bh::herdr::Herdr` CLI adapter (clean-env server start, workspace/tab/pane by id, typed JSON results); `bh::attach::ensure` (server → workspace → `orch` tab → orchestrator, workspace id persisted); `bh attach`/bare `bh`; `tests/fixtures/fake-herdr` scripted fake and integration tests. Spec: `specs/04-herdr-adapter.md`.
- Unit 05: orchestrator contract v1 (`prompts/orchestrator-contract.md`); `bulkhead-orch` Pi extension registering nine `bh_*` tools via `pi.exec`; `bh::orchestrator` embeds contract + extension with `include_str!`, installs them on init/attach, and renders `--append-system-prompt`. Spec: `specs/05-orchestrator-extension.md`.
- Unit 06: `prompts/crew-brief.md` and pure `bh-core::brief`; `bh::treehouse` (claim/release) and `tests/fixtures/fake-treehouse`; `bh::task` spawn/send/status/teardown; `Herdr::prompt_agent`/`close_tab`; `bh spawn|send|status|teardown`; round-trip integration test. Spec: `specs/06-crew-lifecycle.md`.

## In Progress

- None yet.

## Next Up

- Unit 07: delivery and `bh land`.

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
- Should `StatusVerb::Unknown` wake the orchestrator (protocol drift is worth knowing) or stay silent to avoid noise from forward-compatible crew builds?
- Unit 03 observed Pi 1.5.0: `~/.pi/agent` has no `AGENTS.md`, `models.json` or `themes/`, and carries `models-store.json` instead; the builder links only sources that exist, so those links are legitimately absent. Confirm this is the right behavior when a global file appears later (`bh doctor` warns).
- Generated per-repo `settings.json` is seeded with the global `defaultProvider`/`defaultModel` because Pi reads the whole global layer from `PI_CODING_AGENT_DIR` and settings is generated, not symlinked. Confirm this inheritance is intended.
- `bh doctor`'s "stray AGENTS.md" check currently means an `AGENTS.md` in the repo agent folder that is a regular file rather than a symlink. Confirm that matches the intended check.
- `BULKHEAD_HOME` and `BULKHEAD_AGENT_DIR` are the env overrides used for tests and relocation; decide whether to document them as public.
- Unit 04 herdr recon: installed `herdr 0.9.5` reports **protocol 22** (context said 16+, satisfied); socket frames are JSON-RPC-like `request`/`success_response`/`error_response`/`subscription_event` per `herdr api schema`. Recorded in `specs/04-herdr-adapter.md`.
- Unit 04 assumes `herdr --session <s> status server` does not print `status: running` (and exits non-zero) when the server is down; unverified live because stopping the real server is out of bounds. Confirm when a safe session exists.
- Clean-env server start (`env_clear` + `PATH`/`HOME` + explicit child env) is enforced in code but not asserted by tests; add a live check once a disposable server can be started.
- Unit 05 fixed the earlier `--append-system-prompt` gap: `bh attach` now starts the orchestrator with `--append-system-prompt <repo>/agent/prompts/orchestrator-contract.md`; the extension is auto-discovered from `<repo>/agent/extensions/bulkhead-orch/index.ts` (no `-e`).
- Confirm contract v1 wording, especially the authority table and dispatch policy; it is the prompt surface every later wake feeds into.
- The nine `bh_*` tools exist before their subcommands (Units 06, 08). Confirm that is preferable to registering them only as their subcommands land.
- Extension assets are embedded in the binary with `include_str!` and written into the repo agent folder rather than installed from a dist tree. Revisit when bulkhead ships prebuilt.
- Unit 06 assumed the treehouse CLI: `claim <repo-root> <task-id>` prints a worktree path and `release <worktree>`. Confirm against the real tool.
- Crew branch naming is assumed to be `bh/<task-id>` and the brief says so; confirm treehouse/crew actually use it.
- Crew Pi is started with `--session-id <task-id>` so `meta.toml` can record a stable session ref; confirm that is the intended resume handle.
- `teardown` leaves a marker file and keeps `brief.md`/`status.log`/`meta.toml` for history rather than deleting the task folder. Confirm that matches the intended lifecycle.

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
- Unit 02 landed. `Event::StatusLine` and `Event::PlanArrived` gained an `at: Timestamp` so `classify` can stamp wakes without reading a clock; no other Unit 01 shape changed. Wake ids are deterministic (`w-1`, `w-2`, …) from a counter in `SupervisionState`, seeded by `bhd` from `wakes.jsonl` on restart. Unknown status verbs update state but do not wake; flagged as an open question.
- Unit 03 landed. `bh` is a lib+bin crate so integration tests can drive `init`/`doctor`; `BulkheadPaths` is injected, never discovered in tests. `bh init` links only present global files and generates per-repo `settings.json`/`mcp.json`; `bh doctor` reports errors but does not mutate. A manual CLI run against a temp home passed (`demo-repo` registered, doctor clean).
- Unit 03 changed the Unit 01 `RegistryEntry.workspace` field to `Option<WorkspaceId>` (init runs before the herdr adapter exists). Flagged for review.
- Unit 04 landed. All herdr access is by id; the fake herdr is Python and logs exact argument arrays. `bh attach` orchestrates but the interactive `exec` is untested (it replaces the process). 33 tests pass.
- Unit 05 landed. `extensions/bulkhead-orch` typechecks with `tsc --noEmit` and loads under `pi -e ... --help`. Contract v1 and the extension are embedded in `bh` and written on init/attach so the installed copies match the binary. No socket or wake work yet.
- Unit 06 landed. Crew lifecycle is wired end to end against fakes: spawn claims a worktree, renders the brief, starts Pi in a tab with the repo agent folder, and records meta.toml; send/status/teardown round-trip. 36 tests pass.
