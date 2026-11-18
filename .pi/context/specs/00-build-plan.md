# Build Plan

Units in build order. Each produces one verifiable result and stays inside one boundary. Write each unit's spec (`NN-name.md`, in the spec format) right before implementing it.

| Unit | Milestone | Builds | Depends on |
| --- | --- | --- | --- |
| 01 | M0 | Cargo workspace (`bh-core`, `bhd`, `bh`), `rust-toolchain.toml`, domain types (ids, `StatusVerb`, `PaneState`, `Event`, `Wake`, `Task`, `Config`), versioned `bhd` ↔ extension protocol types, serde round-trip tests | — |
| 02 | M0 | Status-log line parser and `classify(state, event)` in `bh-core`, with fixture tests for every verb | 01 |
| 03 | M0 | `bh init`: registry, repo home layout, agent-folder builder (symlinks, generated `settings.json` and `mcp.json`), `bh doctor` checks for stray `AGENTS.md` and broken symlinks | 01 |
| 04 | M0 | herdr adapter in `bh` (clean-env server start, workspace/tab/pane by id) and `bh` attach that starts the orchestrator in the `orch` tab | 03 |
| 05 | M0 | `bulkhead-orch` extension skeleton: contract file via `--append-system-prompt`, `bh_*` tools that shell out to `bh`; orchestrator contract v1 in `prompts/` | 04 |
| 06 | M1 | `bh spawn`, `send`, `status`, `teardown`: treehouse claim, brief rendering from the spec template, fresh pane launch with the repo's agent folder | 02, 04 |
| 07 | M2 | Delivery: `delivery` modes, `bh land` behind approval via gh-axi, stacked PRs for dependent tasks | 06 |
| 08 | M3 | `bh plan`, `bh task`, inbox triage states, lavish-axi review loop, triage rules in `prompts/` | 06 |
| 09 | M4 | `bhd` skeleton: tokio loop, `notify` source for status logs and inboxes, durable `wakes.jsonl`, batcher, Unix socket server; extension client delivering follow-ups with dedupe | 02, 05 |
| 10 | M4 | herdr pane-event subscription in `bhd`, with polling fallback | 09 |
| 11 | M4 | Turn-end guard on `agent_settled`, wake replay after `/new`, `/resume` or reload, `digest.md` upkeep | 09 |
| 12 | M5 | Stall detection, away policy, return brief, phone push, `bhd` orchestrator revival, launchd plist | 10, 11 |
| 13 | M6 | backpass per-repo cadence, `bh ls` over herdr state | 12 |
| 14 | — | `shared/prompts/bootstrap-context.md` and `shared/skills/spec-units/` | 08 |

Unit 14 can run any time after Unit 08; it does not block the daemon path.
