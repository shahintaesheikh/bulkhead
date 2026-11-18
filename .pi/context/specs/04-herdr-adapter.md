# Unit 04 — herdr Adapter and `bh` Attach

- **Milestone:** M0
- **Depends on:** Unit 03
- **Boundary:** `bh` I/O only. No `bhd` subscription (Unit 10) and no Pi extension contract (Unit 05).

## Goal

Give `bh` a by-id herdr adapter and a `bh attach` flow that ensures the shared
`bulkhead` session, the repo's workspace, its `orch` tab and a live orchestrator,
then attaches the captain to it.

## herdr recon (recorded here and in the tracker)

Confirmed against the installed tool on this machine:

- `herdr 0.9.5`, **protocol 22** — the context's "protocol 16+" requirement is
  satisfied; the version numbers are unrelated.
- The socket is JSON-RPC-like: `request { id, method, params }`,
  `success_response { id, result }`, `error_response { id, error }`, and
  `subscription_event { event, data }` (`herdr api schema`). Unit 10 uses those
  event frames.
- CLI result keys: `workspace list` → `.result.workspaces`; `workspace create`
  → `.result.{workspace,tab,root_pane}`; `tab list` → `.result.tabs`;
  `tab create` → `.result.{tab,root_pane}`; `pane list` → `.result.panes`;
  `agent list` → `.result.agents`.
- Workspace/tab/pane ids are `w1`, `w1:t1`, `w1:p1`; agent lifecycle states are
  `idle`, `working`, `blocked`, `done`, `unknown` (matches `PaneState`).
- `herdr status server` prints `status: running`; `herdr --session <s> server`
  runs the headless server.

## Deliverables

1. `bh::herdr::Herdr`: a CLI adapter taking an argument array and parsing JSON,
   with `server_running`, `start_server`, `list_workspaces`, `create_workspace`,
   `list_tabs`, `create_tab`, `list_panes`, `start_agent`, `attach`.
2. `bh::attach::ensure`: the by-id orchestration that ensures server →
   workspace → `orch` tab → orchestrator, and persists the workspace id.
3. `bh attach` plus bare `bh`; bare `bh` attaches the current repo.
4. `tests/fixtures/fake-herdr`, a stateful scripted herdr, and integration tests.

## Design

### Addressing

Everything after discovery is by id (invariant 4). The only label lookup is a
one-time fallback: if the registry has no workspace id, a workspace whose label
equals the repo id is adopted; otherwise one is created. Either way the id is
persisted to `registry.toml`.

### Clean environment (invariant 5)

`start_server` runs `herdr --session <s> server` with `env_clear()`, re-adding
only `PATH`, `HOME` and explicit `Herdr::with_child_env` values. An ambient
`PI_CODING_AGENT_DIR` can never reach the shared server. Tests cannot safely
start a real server, so this is enforced in code and reviewed rather than
asserted.

### Pane environment

`PI_CODING_AGENT_DIR` is passed as `--env KEY=VALUE` to `workspace create` and
`tab create`, so it is part of each pane's launch, never the server's.

### Orchestrator start

`ensure` starts Pi with `herdr agent start orch --kind pi --pane <id>`. Extra
Pi arguments are supported by the adapter but empty in Unit 04; Unit 05 adds the
`--append-system-prompt` contract and the extension.

### The fake herdr

`tests/fixtures/fake-herdr` is a Python script that keeps state in
`$FAKE_HERDR_STATE` and logs every argument array to `calls.log`, so tests can
assert exactly what `bh` sent (including `--session bulkhead` and the pane
`--env`). The adapter injects that state path through `with_child_env`; no
process-wide environment is mutated and no real herdr is contacted.

## Out of Scope

- The `bulkhead-orch` extension and contract (Unit 05).
- `bhd` herdr event subscription (Unit 10).
- Spawning crew panes and treehouse worktrees (Unit 06).

## Verify

- [ ] `ensure` starts the server, creates workspace/tab/pane and the
      orchestrator on a cold start, and is idempotent on a warm one.
- [ ] A workspace labeled with the repo id is adopted and its id persisted.
- [ ] A recorded workspace that disappeared is recreated.
- [ ] `--session bulkhead` and `--env PI_CODING_AGENT_DIR=...` appear in the
      exact calls the adapter makes.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
