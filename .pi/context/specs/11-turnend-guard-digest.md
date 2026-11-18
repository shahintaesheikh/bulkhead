# Unit 11 — Turn-End Guard, Wake Replay and `digest.md`

- **Milestone:** M4
- **Depends on:** Unit 09
- **Boundary:** the extension guard and daemon bookkeeping. No stall or away policy (Unit 12).

## Goal

Make sure a wake that arrives while the orchestrator is mid-turn is never lost
at settle, that a restarted or reloaded orchestrator sees unprocessed wakes
again, and that each repo keeps a rolling digest of what woke it.

## Deliverables

1. `WakeKind::as_token()` in `bh-core`.
2. `bhd` appends one line per wake to `<repo>/state/digest.md`.
3. The extension turn-end guard: wakes received during a turn are held,
   delivered as one follow-up at `agent_settled`, and acked only then.
4. Wake replay through the existing reconnect: a `/new`, `/resume` or `/reload`
   reloads the extension and reconnects; `bhd` still holds unacked wakes and
   re-offers them.

## Design

### Turn-end guard

The client tracks `activeTurn` between `agent_start` and `agent_settled`.

- While idle, a wake is delivered immediately with
  `pi.sendUserMessage(..., { deliverAs: "followUp" })` and acked.
- While a turn is running, the wake is appended to a `deferred` list and left
  unacked.
- At `agent_settled`, the deferred wakes are delivered as one follow-up and
  acked together. If delivery throws, they stay deferred.

Because the ack happens only after delivery, a crash mid-turn leaves the wake
pending and `bhd` redelivers it; the id-dedupe keeps that from double
delivering.

### Replay

Replay needs no new mechanism: unacked wakes stay in `wakes.jsonl`, and every
batch tick re-offers them to whichever client is connected. A reloaded
extension starts with an empty dedupe set, so a still-pending wake is delivered
again.

### Digest

`Core::record` appends `- <time> <task> <kind>: <summary>` to `digest.md` right
after the durable wake append. It is append-only and human-readable.

## Out of Scope

- Stall thresholds, away-mode, phone push and revival (Unit 12).
- Digest compaction or summarization by the orchestrator.

## Verify

- [ ] A wake that arrives during a turn is not acked until settle.
- [ ] `digest.md` gains one line per wake with its kind token.
- [ ] `tsc --noEmit`, `cargo fmt --check`, `clippy -D warnings` and
      `cargo test --workspace` pass.
