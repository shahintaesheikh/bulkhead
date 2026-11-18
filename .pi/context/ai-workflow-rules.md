# AI Workflow Rules

## Approach

Build bulkhead incrementally with a spec-driven workflow. The context files define what to build, how to build it, and where the build stands. Implement each unit exactly as its spec in `context/specs/` says. Do not infer or invent behavior that the context files or the unit spec do not define.

bulkhead is built sequentially, one unit at a time, because it does not exist yet to parallelize itself. Once M5 is complete, bulkhead may manage its own repo.

## Scoping Rules

- Work on one unit at a time.
- Prefer small, verifiable increments over large speculative changes.
- Do not combine unrelated system boundaries in one implementation step.
- Do not add a crate dependency before the unit that first needs it.
- Do not add a feature listed as out of scope in `project-overview.md`.

## When to Split Work

Split an implementation step if it combines:

- Pure logic in `bh-core` with I/O in `bhd` or `bh` (land the core and its tests first)
- `bhd` event sources with the TypeScript extension
- A status-protocol change with unrelated behavior
- Behavior not clearly defined in the context files or the unit spec

If a change cannot be verified end to end quickly, the scope is too broad: split it.

## Handling Missing Requirements

- Do not invent product behavior not defined in the context files.
- If a requirement is ambiguous, propose the resolution in the relevant context file and wait for the captain before implementing.
- If a requirement is missing, add it as an open question in `progress-tracker.md` and stop work on that part.
- If herdr, Pi, treehouse or another external tool behaves differently than the context files assume, record the observed behavior and the tool version in `progress-tracker.md` before working around it.

## Protected Files

Do not modify the following unless explicitly instructed:

- The Invariants section of `context/architecture.md`
- `rust-toolchain.toml` and pinned Pi and herdr versions
- Anything under `~/.pi/agent`, `~/.bulkhead` or a real herdr session during development or tests
- `tests/fixtures/` files that reproduce a past bug (add new fixtures instead)

## Keeping Docs in Sync

Update the relevant context file whenever implementation changes:

- System boundaries or crate responsibilities
- The storage layout under `~/.bulkhead/`
- The status protocol, wake kinds or the `bhd` ↔ extension protocol
- Code conventions or standards
- Feature scope

Correct facts that a change made wrong. Do not add new rules or sections without the captain's approval.

## Learning Mode

- The captain is learning Rust through this project. If the captain says they will write a unit themselves, review their code against the spec instead of implementing it.
- When a change relies on a non-obvious Rust concept, explain it briefly in the PR description.

## Before Moving to the Next Unit

1. The current unit works end to end within its defined scope, and every item in its spec's verify checklist is checked.
2. No invariant in `architecture.md` was violated.
3. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --workspace` pass.
4. `tsc --noEmit` passes if the extension changed.
5. `progress-tracker.md` reflects the completed work.
