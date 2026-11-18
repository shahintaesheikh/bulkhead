# Unit 05 — `bulkhead-orch` Extension Skeleton and Contract v1

- **Milestone:** M0
- **Depends on:** Unit 04
- **Boundary:** extension TypeScript, `prompts/` and the `bh` launch wiring. No socket client, no wake delivery, no turn-end guard.

## Goal

Register the `bh_*` tool surface from a Pi extension and give the orchestrator
its contract, so that after `bh init` a repo's Pi session has the tools and the
rules even though the daemon path does not exist yet.

## Deliverables

1. `prompts/orchestrator-contract.md` — contract v1: scope, authority,
   dispatch policy, status protocol, plan triage, reporting, tools.
2. `extensions/bulkhead-orch/` — a Pi package (`package.json`, `tsconfig.json`,
   `src/extension.ts`) registering `bh_task`, `bh_plan`, `bh_spawn`, `bh_send`,
   `bh_status`, `bh_land`, `bh_teardown`, `bh_away`, `bh_ls` through
   `pi.registerTool`, each shelling out to `bh` with `pi.exec`.
3. `bh::orchestrator` — embeds the contract and extension with `include_str!`,
   writes them into the repo's agent folder, and renders
   `--append-system-prompt <contract>`.
4. `bh init` and `bh attach` install the contract and extension; `attach` passes
   the contract argument when starting the orchestrator.

## Design

### Embedding, not a runtime install location

There is no agreed install tree for bulkhead's own assets, so both files are
compiled into `bh` with `include_str!`. `bh init` and `bh attach` write them
into `<repo>/agent/`:

- `prompts/orchestrator-contract.md` — bulkhead-owned, always overwritten so it
  matches the running binary. Injected with `--append-system-prompt`.
- `extensions/bulkhead-orch/index.ts` — Pi auto-discovers
  `<agent-dir>/extensions/*/index.ts`, so no manifest or symlink is needed.

This is a packaging decision to revisit if bulkhead ever ships prebuilt. It
keeps the installed contract and tools versioned with the binary.

### The extension is transport only

The extension holds no classification, scheduling or watching. Each tool maps
its arguments to a `bh` argv array and runs `pi.exec(BH_BIN, argv, { signal })`
(no shell). `BULKHEAD_BH_BIN` overrides the binary, which tests and
odd installs use. `isError` is set from the exit code.

The nine tools name the contract's supervision actions. Their `bh` subcommands
arrive in Units 06 and 08; until then the tools exist and report the `bh` error
faithfully rather than being absent.

### Version pinning

`package.json` pins Pi `1.5.0`, `@earendil-works/pi-ai` `1.5.0`, `typebox`
`1.3.27` (the version Pi 1.5.0 bundles) and TypeScript `5.9.3` as
`devDependencies` for `tsc --noEmit`; the host packages are declared as
`peerDependencies: "*"` so a real install never bundles a second copy. Pi is
named in the extension header comment.

## Out of Scope

- The `bhd` socket client and wake delivery (Unit 09).
- The turn-end guard and wake replay (Unit 11).
- `bh spawn`/`send`/`status`/`teardown` (Unit 06) and `bh plan`/`task` (Unit 08).
- Opt-in shared skills and extensions under `~/.bulkhead/shared/` (Unit 14).

## Verify

- [ ] `bh init` writes `agent/prompts/orchestrator-contract.md` and
      `agent/extensions/bulkhead-orch/index.ts`.
- [ ] `bh attach` starts the orchestrator with `--append-system-prompt <contract>`.
- [ ] `extensions/bulkhead-orch` passes `tsc --noEmit`.
- [ ] `pi -e ./src/extension.ts --help` loads the extension without error.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
