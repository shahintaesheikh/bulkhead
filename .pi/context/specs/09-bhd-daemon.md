# Unit 09 — `bhd` Daemon Skeleton and Wake Delivery

- **Milestone:** M4
- **Depends on:** Units 02, 05
- **Boundary:** the daemon and the extension's socket client. No herdr event subscription (Unit 10) and no turn-end guard (Unit 11).

## Goal

Run one supervisor daemon that watches every repo at zero token cost, turns
status and plan events into durable wakes, and delivers them to the repo's
orchestrator over a Unix socket with at-least-once semantics.

## Deliverables

1. `bhd` as a lib+bin with `paths`, `clock`, `wake_store`, `socket`, `source`,
   `core`, and `run`.
2. A recursive `notify` watcher for `tasks/*/status.log`, `plans/inbox/*.md`
   and `state/away.json`.
3. A Unix socket server with newline-delimited `Envelope` framing, user-only
   permissions, and a per-client outbound channel.
4. The `Core` loop: the only owner of `SupervisionState`, wake stores, client
   channels, file offsets and seen-plan ids.
5. The extension `startWakeClient`: connect, `hello`, deliver a wake with
   `pi.sendUserMessage(..., { deliverAs: "followUp" })`, dedupe on id, `ack`,
   reconnect.
6. `BULKHEAD_REPO` and `BULKHEAD_SOCKET` in each pane's env.

## Design

### Pure core, async shell

`Core::handle(SourceEvent)` is synchronous: sources only send path lists,
connections, messages, ticks or shutdown. `handle` reads changed files itself
(so it can own offsets), parses at the boundary, and calls the pure
`bh_core::classify`. Wakes are appended to `<repo>/state/wakes.jsonl` before
they are offered to a client; an `ack` appends a `Delivered` record.

### Delivery

Each connected extension owns an unbounded outbound channel. On every batch
tick the core re-offers every undelivered wake to its repo's client, so a
dropped connection or a missed ack only causes a redelivery. The extension
dedupes on the wake id, which is what makes the at-least-once guarantee safe.

### Rehydration

`WakeStore::load` reads `wakes.jsonl`, tolerates a truncated trailing line,
recomputes the highest `w-N` sequence, and drops wakes that already have a
`Delivered` record. `Core::new` seeds `SupervisionState` with the max sequence
so wake ids stay unique across restarts.

### Batching

The core ticks every `BULKHEAD_BATCH_SECS` (default 30) and flushes then.
Status lines from one burst are appended as separate wakes; coalescing is left
to the orchestrator's turn.

## Out of Scope

- herdr pane-event subscription (Unit 10).
- Turn-end guard, wake replay, `digest.md` (Unit 11).
- Stall detection, away policy, phone push, revival (Unit 12).

## Verify

- [ ] A status line appended to `status.log` becomes one durable wake and is
      delivered to a connected client.
- [ ] The wake is on disk before delivery; an ack appends `Delivered` and
      clears it from pending.
- [ ] A repeated plan file event does not create a second wake.
- [ ] `wakes.jsonl` reloads with the right max sequence and pending set.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test --workspace` and `tsc --noEmit` pass.
