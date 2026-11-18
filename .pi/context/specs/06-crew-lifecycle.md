# Unit 06 — `bh spawn`, `send`, `status`, `teardown`

- **Milestone:** M1
- **Depends on:** Units 02, 04
- **Boundary:** `bh` CLI I/O. No delivery gate (Unit 07) and no plan/task intake (Unit 08).

## Goal

Take a backlog task from a brief to a running crewmate, and give the
orchestrator the verbs to message it, inspect it and tear it down.

## Deliverables

1. `prompts/crew-brief.md` and a pure `bh-core::brief` renderer.
2. `bh::treehouse::Treehouse`: `claim` and `release`.
3. `bh::task`: `spawn`, `send`, `status`, `teardown`, `load_task`.
4. `bh spawn|send|status|teardown` subcommands.
5. `Herdr::prompt_agent` and `Herdr::close_tab`; a scripted fake treehouse and
   fake-herdr support for `tab close` and `agent prompt`.
6. An integration test covering the whole round trip.

## Design

### Brief rendering

`bh-core::brief::render` substitutes `{{key}}` placeholders from a
`BriefContext`; unknown placeholders are left visible. `bh` embeds
`prompts/crew-brief.md`, so a rendered brief always matches the binary, exactly
like the orchestrator contract.

### Spawn

1. Require a registered repo with a herdr workspace (from `bh attach`).
2. Read `backlog/<task-id>.md`; the first `#` heading is the title, the rest the
   body.
3. `treehouse claim` a worktree.
4. Write `tasks/<task-id>/brief.md`, create `status.log`, and hold the rest.
5. `herdr tab create` with the worktree as cwd and `PI_CODING_AGENT_DIR` as a
   pane env, then `herdr agent start <task-id> --kind pi` with
   `--append-system-prompt <brief>` and `--session-id <task-id>`.
6. Write `meta.toml` (`bh_core::Task`) with the tab, pane and session ref.

### send / status / teardown

- `send` runs `herdr agent prompt <pane> <message>`.
- `status` prints the task's metadata and the last ten status lines, or lists
  the repo's tasks.
- `teardown` runs `herdr tab close`, `treehouse release`, and drops a
  `teardown` marker so the task reads as finished without deleting history.

### Treehouse interface (assumed)

treehouse is not installed while bulkhead is built, so the adapter's contract
is assumed and exercised through `tests/fixtures/fake-treehouse`:

- `treehouse claim <repo-root> <task-id>` prints an absolute worktree path.
- `treehouse release <worktree>`.

`BULKHEAD_TREEHOUSE_BIN` overrides the binary. Revisit when the real CLI is
available.

## Out of Scope

- Delivery modes and `bh land` (Unit 07).
- `bh plan`/`bh task` and backlog frontmatter (Unit 08).
- Status-log side effects on the orchestrator (Unit 09).

## Verify

- [ ] `render` substitutes all placeholders and leaves unknown ones visible.
- [ ] `spawn` claims a worktree, writes brief/log/meta, and starts Pi in a
      fresh tab with the repo agent folder.
- [ ] `send` prompts the recorded pane; `status` echoes the last lines;
      `teardown` closes the tab, releases the worktree and leaves a marker.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
