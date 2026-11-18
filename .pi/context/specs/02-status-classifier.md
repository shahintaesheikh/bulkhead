# Unit 02 — Status-Log Parser and `classify(state, event)`

- **Milestone:** M0
- **Depends on:** Unit 01
- **Boundary:** `bh-core` only. No file watching, no batching, no socket.

## Goal

Turn raw status-log text into typed values, and define the pure supervision
transition `classify(state, event) -> (state, Vec<Wake>)` that decides which
events deserve an orchestrator wake.

## Deliverables

1. `StatusLine { verb, message }` and `StatusLine::parse`, with a typed
   `StatusParseError` (`MissingSeparator`, `EmptyVerb`).
2. `classify` plus the state it evolves: `SupervisionState`, `RepoState`,
   `TaskState`.
3. Wakes for actionable status verbs (`needs-decision`, `blocked`, `pr`,
   `done`) and for plan arrivals.
4. Fixtures under `tests/fixtures/status/` and tests covering every verb,
   malformed lines, and each event kind.

## Design

### Parser (`status.rs`)

`StatusLine::parse("working: doing X")` splits on the *first* colon, so a
message may contain colons. It trims outer whitespace, matches the verb token
case-sensitively, and maps anything unknown to `StatusVerb::Unknown(raw)`.
Blank or separator-less lines return `StatusParseError::MissingSeparator`; an
empty verb returns `EmptyVerb`. The caller skips truly blank lines.

### State (`classify.rs`)

- `SupervisionState`: `repos: BTreeMap<RepoId, RepoState>` plus a monotonic
  `next_wake_seq`. Wake ids are `w-1`, `w-2`, … so classification stays
  deterministic and clock-free; `bhd` seeds the sequence from `wakes.jsonl` on
  restart (Unit 09).
- `RepoState`: `away`, `tasks`, `panes`, `pending_plans`.
- `TaskState`: `last_verb`, `last_status_at` (recorded now so Unit 12's stall
  check has an input).

`classify` takes and returns the whole state by value. It never reads the
clock, files or a process; the event carries its own timestamp.

### Wake rules

| Event | State change | Wake |
| --- | --- | --- |
| `StatusLine` | set `last_verb`, `last_status_at` | `needs-decision`→`NeedsDecision`, `blocked`→`Blocked`, `pr`→`PrOpened`, `done`→`TaskDone`; `working` and `unknown` do not wake |
| `PlanArrived` | insert into `pending_plans` | `PlanInbox`, once per pending plan (re-reads do not re-wake) |
| `PaneStateChanged` | record pane state | none (pane-driven wakes are Unit 10) |
| `AwayChanged` | set `away` | none (away policy is Unit 12) |
| `WakeAck` / `Tick` / `Shutdown` | none | none |

Wake summaries render as `<task>: <verb> — <message>` for status wakes and
`plan <plan> arrived` for plan wakes. Exact wording is a prompt surface and
open to change.

### Event timestamp

`Event::StatusLine` and `Event::PlanArrived` gain an `at: Timestamp` so
`classify` can fill `Wake.created_at` and `TaskState.last_status_at` without a
clock. This modifies the Unit 01 event shape; no other Unit 01 type changes.

## Out of Scope

- Reading the status log or tracking byte offsets (Unit 09).
- Pane-driven wakes and `CrewGone` (Unit 10).
- Stall detection, away policy, return brief (Unit 12).
- Wake batching and durable delivery (Unit 09).

## Verify

- [ ] `parse_status_line` handles all five verbs, an unknown verb, colons in
      the message, and both error variants from fixtures.
- [ ] `classify` emits exactly the wake each verb warrants and records state.
- [ ] A repeated `PlanArrived` for a still-pending plan wakes once.
- [ ] `PaneStateChanged` and `AwayChanged` update state without waking.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
