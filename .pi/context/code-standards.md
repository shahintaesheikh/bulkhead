# Code Standards

## General

- Keep modules small and single-purpose; one module per concept (`status`, `wake`, `stall`, `brief`).
- Prefer architectural clarity over clever one-liners: explicit types and named steps beat dense iterator chains or macro tricks.
- Fix root causes; do not layer retries or sleeps over a race you have not explained.
- Logic goes in `bh-core`; binaries only wire I/O to it.
- When porting logic from firstmate (MIT), credit the source file in a comment.

## Rust

- Edition 2024, stable toolchain pinned in `rust-toolchain.toml`.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- Model closed sets as enums (`StatusVerb`, `PaneState`, `WakeKind`) and match exhaustively; no wildcard arm on enums bulkhead owns.
- Newtypes for ids (`RepoId`, `TaskId`, `PaneId`, `WakeId`) so they cannot be mixed up.
- No `unwrap()` or `expect()` outside tests, except for invariants documented in a comment at the call site.
- Errors: `thiserror` enums in `bh-core`; `anyhow` with `.context(...)` in `bhd` and `bh`.
- Parse external input (status lines, herdr messages, TOML, socket messages) into typed values at the boundary; unknown status verbs become a typed `Unknown` variant, not a panic.
- Use `std::process::Command` / `tokio::process::Command` with argument arrays; never `sh -c`.

## Async (bhd)

- tokio `current_thread` runtime.
- One task per event source; each sends `Event`s into a single `mpsc` channel. The core loop owns all state.
- Never block inside async code. Bridge `notify`'s synchronous callback with `blocking_send`; use `spawn_blocking` for any unavoidable blocking call.
- Every loop and long-lived task respects a `CancellationToken`; shutdown drains the batcher and flushes `wakes.jsonl`.
- Keep `select!` branches cancel-safe: do not hold partially processed state across an await inside a branch.

## TypeScript (bulkhead-orch)

- `strict` mode; `tsc --noEmit` must pass.
- The extension is transport and guard only: no classification, scheduling or file watching.
- Validate every message from `bhd` against the protocol version before acting on it.
- Pin the Pi version the extension is tested against in `package.json` and in a comment next to each hook used.

## Prompts and Templates

- Contract, brief template and status protocol live in `prompts/` and are versioned with the code.
- Write rules as imperatives, one per line; no motivational prose.
- A change to the status protocol changes `bh-core`'s parser, the brief template and the fixtures in the same unit.

## Data and Storage

- All persisted formats are TOML, Markdown with frontmatter, or JSONL, readable by a human with `cat`.
- Append-only files (`status.log`, `wakes.jsonl`) are only ever appended to; rewrite them only during an explicit archive step.
- Write files atomically (write to a temp file in the same directory, then rename).
- Paths are built from the registry and `RepoId`, never from user-provided strings without validation.

## Testing

- `bh-core`: unit tests plus fixture tests from `tests/fixtures/` for every status verb, pane state and stall case.
- `bhd` and `bh`: integration tests against the scripted fake herdr and temporary home directories; no test touches the real `~/.bulkhead`, `~/.pi` or a real herdr server.
- Each bug fix adds a fixture that reproduces it.

## File Organization

- `crates/bh-core/` — domain types and pure logic
- `crates/bhd/` — daemon binary
- `crates/bh/` — CLI binary
- `extensions/bulkhead-orch/` — TypeScript Pi extension
- `prompts/` — orchestrator contract, brief template, triage rules
- `shared/` — opt-in Pi skills and prompt templates
- `launchd/` — launch agent plist
- `tests/fixtures/` — status logs, herdr event streams, plans, fake herdr
- `context/` — these context files and `specs/`
