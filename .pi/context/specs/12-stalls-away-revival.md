# Unit 12 — Stalls, Away Policy, Push, Revival and launchd

- **Milestone:** M5
- **Depends on:** Units 10, 11
- **Boundary:** daemon policy and the captain-facing away flow.

## Goal

Keep zero-token supervision honest: notice silent crewmates, push the captain
when away, bring a dead orchestrator back, and keep the daemon running.

## Deliverables

1. `bh_core::check_stalls` — at most one `TaskStalled` wake per silent window,
   with `TaskState.stalled` re-armed by a new status line.
2. `bhd` loads per-repo `config.toml` and runs the stall check on every tick.
3. `bhd::push::Push` — a `BULKHEAD_PUSH_BIN` channel; away repos push each wake
   when `away.push` is set.
4. `bh away on|off`: `on` writes `state/away.json`, `off` removes it and prints
   the return brief from `digest.md` and the task list.
5. Orchestrator revival: `Core::needs_revival`/`note_revival` and `bh ensure`,
   run by `bhd` on each tick with a 60-second cooldown.
6. `launchd/dev.bulkhead.bhd.plist` with `KeepAlive`.
7. `Event::OrchestratorGone` and its `OrchestratorRevived` wake.

## Design

### Stall detection

`check_stalls(state, now, thresholds)` is pure. A task stalls when its last
status is older than its repo's `stall_after` and was not `done`. The wake is
emitted once; `TaskState.stalled` is cleared by the next status line.

### Push

The push channel is an open question, so the adapter is one command:
`BULKHEAD_PUSH_BIN <message>`. With it unset, pushes are disabled. Anything
(ntfy, Pushover, a webhook script) can sit behind it. Only away repos push, and
only when `away.push` is true.

### Revival

A repo with a workspace but no connected extension client is a candidate for
revival. On each tick `bhd` runs `bh ensure <repo-path>`, which is
`attach::ensure` without the interactive attach, and records an
`OrchestratorRevived` wake at most once per 60 seconds per repo.

### launchd

The plist keeps `bhd` alive across crashes and login. Paths are placeholders
because the install location is not fixed yet.

## Out of Scope

- The shared skills and bootstrap prompt (Unit 14).
- Choosing a concrete push vendor.

## Verify

- [ ] A silent task produces one stall wake; a new status re-arms it.
- [ ] An away repo pushes each wake through `BULKHEAD_PUSH_BIN`.
- [ ] A connected client suppresses revival; a fresh revival is inside its
      cooldown.
- [ ] `bh away off` prints a brief with digest lines and tasks.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test --workspace` and `tsc --noEmit` pass.
