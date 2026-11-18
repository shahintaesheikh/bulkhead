# Unit 13 — backpass Cadence and `bh ls`

- **Milestone:** M6
- **Depends on:** Unit 12
- **Boundary:** `bh` tooling. No new daemon behavior.

## Goal

Train committed context per repo, and give the captain one read-only view of
every registered repo.

## Deliverables

1. `bh::backpass::Backpass` and a `bh backpass [path]` command.
2. `bh::ls::summarize` and a `bh ls` command over the registry and herdr state.

## Design

### backpass

`backpass train <repo-root>` is the assumed contract behind
`BULKHEAD_BACKPASS_BIN`; the tool is not installed while bulkhead is built.
Running it is a captain action (`bh backpass`) or a cron/launchd cadence, not a
daemon loop, so it never competes with supervision.

### `bh ls`

Read-only and runnable from anywhere: it loads the registry and herdr's
workspace list, counts each repo's task folders, and prints one row per repo.
It never mutates state and never needs a repo on the current path. If the herdr
server is down, rows still print with no workspace label.

## Out of Scope

- A fleet-wide liaison or cross-repo dispatch.
- Automatically syncing shared assets (Unit 14).

## Verify

- [ ] `bh ls` summarizes a registered repo's tasks and matches its workspace
      label.
- [ ] `bh backpass` runs the configured binary against the repo root.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
