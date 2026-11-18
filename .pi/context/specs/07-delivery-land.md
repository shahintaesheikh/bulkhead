# Unit 07 — Delivery and `bh land`

- **Milestone:** M2
- **Depends on:** Unit 06
- **Boundary:** `bh` delivery I/O. No plan intake (Unit 08) and no daemon.

## Goal

Land an approved task through its repo's delivery mode, dependencies first, and
tear it down — refusing to merge anything the captain has not approved.

## Deliverables

1. `bh::config` — per-repo `config.toml` load/save.
2. `bh::ghaxi::GhAxi` — `pr create`, `pr merge` over `npx -y gh-axi`.
3. `bh::no_mistakes::NoMistakes` — `land <pr-url>`.
4. `bh::delivery::{land, pr_number}` and a `bh land` subcommand.
5. `fake-ghaxi` and `fake-no-mistakes` fixtures plus mode/approval/stack tests.

## Design

### Approval gate

`bh land <task> --approve` requires an explicit approval unless the repo's
`config.toml` sets `yolo = true`, or the delivery mode is `local` (nothing
leaves the machine). The orchestrator can never merge on its own.

### Modes

- `direct-pr` → `gh-axi pr merge <number> --method squash --delete-branch`.
- `no-mistakes` → `no-mistakes land <pr-url>`.
- `local` → no remote call.

The PR number comes from the task's `pr_url` (last URL segment).

### Stacked tasks

`bh_core::Task` gains `base` and `depends_on` (both optional). `land` recurses
into `depends_on` first, so a stack lands bottom-up, then closes the tab,
releases the worktree and drops a `landed` marker.

### Config is authoritative

The delivery mode and `yolo` are read from `<repo>/config.toml`. The registry
also carries a `delivery` field from registration, but the per-repo config is
what the captain edits; the registry value is treated as a default. Flagged as
an open question because the context lists the field in both places.

### no-mistakes interface (assumed)

Not installed while bulkhead is built, so the adapter expects
`no-mistakes land <pr-url>` and tests use `tests/fixtures/fake-no-mistakes`.
`BULKHEAD_NOMISTAKES_BIN` and `BULKHEAD_GHAXI_BIN` override the binaries.

## Out of Scope

- Plan intake and triage (Unit 08).
- Waking the orchestrator when a PR opens (Unit 09).
- Rebasing stale PRs (Unit 12).

## Verify

- [ ] `direct-pr` without `--approve` fails; with `--approve` merges the PR
      number and tears the task down.
- [ ] `local` lands without approval and never calls gh-axi.
- [ ] `no-mistakes` calls the gate with the PR URL.
- [ ] A task with `depends_on` lands the dependency first.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
