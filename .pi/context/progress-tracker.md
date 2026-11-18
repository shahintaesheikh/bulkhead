# Progress Tracker

Update this file after every meaningful implementation change.

## Current Phase

- Build complete. Units 01–14 are landed; the build plan's milestones M0–M6 are done.

## Current Goal

- None. The next work is the captain's review, tuning from real use, and the open questions below.

## Completed

- Design spec: per-repo orchestrator, layout, `bh` command surface, orchestrator contract and dispatch policy, crew lifecycle and status protocol, plan ingress and triage, repo context, away-mode, persistence, Rust implementation plan, build order.
- Unit 01: Cargo workspace (`bh-core`, `bhd`, `bh`), `rust-toolchain.toml` pinned to 1.99.1, domain types (ids, time, `StatusVerb`, `PaneState`, `Event`, `Wake`/`WakeRecord`/`WakeKind`, `Task`, `Registry`/`RepoConfig`), versioned `bhd` ↔ extension protocol (`Envelope`, `ProtocolError`), serde round-trip tests. Spec: `specs/01-cargo-workspace.md`.
- Unit 02: `StatusLine`/`StatusLine::parse` with typed errors, `SupervisionState`/`RepoState`/`TaskState`, pure `classify(state, event)` with wake rules, status fixtures, parser and classifier tests. Spec: `specs/02-status-classifier.md`.
- Unit 03: `bh` lib+bin split; `bh init` (git detection, slug, registry, repo home, generated `config.toml`/`settings.json`/`mcp.json`, global symlinks); `bh doctor` (registry, config, JSON, broken symlinks, stray `AGENTS.md`); temp-home integration tests. `RegistryEntry.workspace` is now `Option`. Spec: `specs/03-init-agent-folder.md`.
- Unit 04: `bh::herdr::Herdr` CLI adapter (clean-env server start, workspace/tab/pane by id, typed JSON results); `bh::attach::ensure` (server → workspace → `orch` tab → orchestrator, workspace id persisted); `bh attach`/bare `bh`; `tests/fixtures/fake-herdr` scripted fake and integration tests. Spec: `specs/04-herdr-adapter.md`.
- Unit 05: orchestrator contract v1 (`prompts/orchestrator-contract.md`); `bulkhead-orch` Pi extension registering nine `bh_*` tools via `pi.exec`; `bh::orchestrator` embeds contract + extension with `include_str!`, installs them on init/attach, and renders `--append-system-prompt`. Spec: `specs/05-orchestrator-extension.md`.
- Unit 06: `prompts/crew-brief.md` and pure `bh-core::brief`; `bh::treehouse` (claim/release) and `tests/fixtures/fake-treehouse`; `bh::task` spawn/send/status/teardown; `Herdr::prompt_agent`/`close_tab`; `bh spawn|send|status|teardown`; round-trip integration test. Spec: `specs/06-crew-lifecycle.md`.
- Unit 07: `bh::config`; `bh::ghaxi::GhAxi` and `bh::no_mistakes::NoMistakes` adapters; `bh::delivery::land` with approval gate, three modes, `depends_on` stacking and teardown; `bh land`; fakes and tests. `bh_core::Task` gained `base`/`depends_on`. Spec: `specs/07-delivery-land.md`.
- Unit 08: `bh-core::frontmatter` and `bh-core::plan`; `bh::plan` (ingest/triage across inbox/accepted/declined), `bh::backlog`, `bh::lavish::LavishAxi`; `bh plan` and `bh task`; `prompts/triage-rules.md` installed with the contract; `bh_plan` tool extended to triage. Spec: `specs/08-plan-task-triage.md`.
- Unit 09: `bhd` lib+bin with `notify` watcher, durable per-repo `wakes.jsonl`, batcher, Unix socket server, synchronous `Core` loop, and the extension wake client (follow-up delivery, dedupe, reconnect). Pane env now carries `BULKHEAD_REPO`/`BULKHEAD_SOCKET`. Spec: `specs/09-bhd-daemon.md`.
- Unit 10: `bhd::herdr` pane-agent-status subscription over the herdr socket plus a `herdr pane list` polling fallback; `SourceEvent::HerdrPane` and workspace-to-repo mapping in `Core`. Spec: `specs/10-herdr-events.md`.
- Unit 11: `WakeKind::as_token`; `bhd` rolling `digest.md`; extension turn-end guard that defers wakes received mid-turn and acks at `agent_settled`; replay via reconnect. Spec: `specs/11-turnend-guard-digest.md`.
- Unit 12: pure `check_stalls` and `TaskState.stalled`; bhd config loading and tick stalls; `Push` behind `BULKHEAD_PUSH_BIN`; `bh away on|off` with return brief; orchestrator revival via `bh ensure` with a 60s cooldown; `Event::OrchestratorGone`; launchd plist. Spec: `specs/12-stalls-away-revival.md`.
- Unit 13: `bh::backpass::Backpass` and `bh backpass`; `bh::ls::summarize` and `bh ls` over the registry and herdr workspace list. Spec: `specs/13-backpass-ls.md`.
- Unit 14: `shared/prompts/bootstrap-context.md` and `shared/skills/spec-units/SKILL.md`; `agent_folder` links opted-in skills and all shared prompts. Spec: `specs/14-shared-assets.md`.

## In Progress

- None.

## Next Up

- None. Build complete.

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
- Unit 07 assumed the no-mistakes CLI: `no-mistakes land <pr-url>`. Confirm against the real tool.
- `delivery` and `yolo` are read from `<repo>/config.toml`; the registry's `delivery` is treated as a registration default. Decide whether the registry field should be dropped to avoid two sources of truth.
- Stacked delivery lands dependencies recursively and then tears each down. Confirm the captain approves a whole stack with one `--approve`, and that `base` should be set at spawn (currently it is only recorded).
- Unit 08 assumed the lavish-axi CLI: `publish <path>` prints a review URL and `poll <review>` prints feedback. Confirm against the real tool.
- Frontmatter is a flat `key: value` subset, not full YAML. Confirm no managed file needs lists or nesting.
- Plans are triaged with `bh plan --accept/--decline/--supersede`; the inbox watcher is in `bhd` (Unit 09) and the lavish poll loop is deferred to Unit 12.
- Unit 09 duplicates the `~/.bulkhead` path layout in `bh::paths` and `bhd::paths`. Consider a shared `bh-core` path module once the layout settles.
- `bhd` re-offers every pending wake on each batch tick until acked. Confirm the 30s cadence is acceptable, or add a per-wake retry backoff in Unit 12.
- The daemon restarts lose in-memory file offsets and seen-plan ids; on restart the first scan may re-read a status log from zero, producing duplicate wakes for old lines. A persisted per-file offset is a candidate for Unit 12.
- Unit 11 acks at `agent_settled`, so a long turn keeps a wake pending and re-offered every batch; the extension dedupes. Confirm that is preferable to acking on delivery.
- `digest.md` grows without bound; decide when/how the orchestrator compacts it.
- Unit 12 push is a single command behind `BULKHEAD_PUSH_BIN`. Confirm the vendor and message shape.
- Revival shells `bh ensure` from `bhd`; confirm `bh` is on the daemon's `PATH` (the launchd plist sets one) or pin it with `BULKHEAD_BH_BIN`.
- The launchd plist has placeholder paths; fill them when the install location is decided.
- Unit 13 assumed the backpass CLI: `backpass train <repo-root>`. Confirm against the real tool.
- Unit 13 does not schedule backpass; it is a captain command. Decide whether a per-repo cadence belongs in `bhd` or launchd.
- Unit 14 links every shared prompt into every repo, but shared skills only when listed. Confirm that prompt-as-default is acceptable, or add an explicit `prompts` list to `config.toml`.
- No end-to-end test drives a real Pi orchestrator/crew, herdr, treehouse, no-mistakes, lavish or backpass; all are behind adapters and scripted fakes. The success criteria in `project-overview.md` are the acceptance test once the tools are installed.

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
- Unit 07 landed. `bh land` gates on explicit approval (or `yolo`/`local`), merges via gh-axi or no-mistakes, lands `depends_on` first, and tears down. 40 tests pass.
- Unit 08 landed. Plan intake and triage and backlog creation are wired; HTML plans get a lavish review URL; triage rules ship with the contract. 47 tests pass; tsc clean.
- Unit 09 landed. `bhd` watches every repo, records durable wakes before delivery, serves the extension socket, and batches about every 30s; the extension delivers wakes as follow-ups, dedupes on id and acks. 52 tests pass; tsc clean.
- Unit 10 landed. `bhd` subscribes to herdr `pane.agent_status_changed` and falls back to polling `herdr pane list`; core maps workspace ids to repos.
- Unit 11 landed. Wakes arriving mid-turn are deferred and acked at settle; `digest.md` records every wake. 53 tests pass; tsc clean.
- Unit 12 landed. Stalls, away-mode with a return brief, a `BULKHEAD_PUSH_BIN` push channel, orchestrator revival and a launchd plist. 56 tests pass; tsc clean.
- Unit 13 landed. `bh ls` and `bh backpass`; the shared-asset linking lands in Unit 14. 57 tests pass.
- Unit 14 landed. Shared bootstrap prompt and spec-units skill ship, with opt-in skill linking and shared-prompt linking. Build complete: 58 tests pass; fmt, clippy and tsc clean.
