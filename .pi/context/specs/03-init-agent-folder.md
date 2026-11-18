# Unit 03 — `bh init`, Repo Home and Agent Folder

- **Milestone:** M0
- **Depends on:** Unit 01
- **Boundary:** `bh` CLI I/O only. No herdr calls (Unit 04) and no extension (Unit 05).

## Goal

Make `bh init` register a repo and build its entire private home and Pi agent
folder, and make `bh doctor` prove the result is sound. After this unit a repo
has a stable `~/.bulkhead/<repo-id>/` and a `PI_CODING_AGENT_DIR` that inherits
the global layer but isolates skills, MCP servers and extensions.

## Deliverables

1. `bh init [path]`: detect root/origin/slug, write `registry.toml`, create the
   repo home, generate `config.toml`, seed `orchestrator.md` and `learnings.md`.
2. Agent-folder builder: symlink the global files that exist (`AGENTS.md`,
   `auth.json`, `models.json`, `themes/`), generate `settings.json` and
   `mcp.json`, create `skills/`, `extensions/`, `prompts/`.
3. `bh doctor`: check the registry, repo homes, config, JSON files, broken
   symlinks and stray `AGENTS.md`.
4. Tests against temporary homes and a throwaway git repo; nothing touches the
   real `~/.bulkhead` or `~/.pi`.

## Design

### Testability

`BulkheadPaths { home, global_agent }` is constructed explicitly in tests and
resolved once in `main` via `discover()`. `discover()` honors
`BULKHEAD_HOME` (default `~/.bulkhead`) and `BULKHEAD_AGENT_DIR` (default
`~/.pi/agent`). No function reaches for the user's home on its own.

### Registry workspace id is optional

The registry's `workspace` field becomes `Option<WorkspaceId>`. `bh init` runs
before the herdr adapter exists (Unit 04), so it registers with `None`; the
first attach fills it in. This narrows the Unit 01 `RegistryEntry` shape.

### Generated files

- `agent/settings.json` is seeded with `defaultProvider` and `defaultModel`
  copied from the global `settings.json` when present. Rationale: Pi resolves
  its whole global layer from `PI_CODING_AGENT_DIR`, so an unattended agent with
  an empty settings file has no default model. Flagged for review.
- `agent/mcp.json` starts as `{"mcpServers": {}}`; servers are opted in per repo.
- `config.toml`, `settings.json`, `mcp.json`, `orchestrator.md`, `learnings.md`
  are written only when absent, so re-running `init` never clobbers edits.

### Linking

`AGENTS.md`, `auth.json`, `models.json` and `themes/` are symlinked from
`global_agent` only when the source exists; a missing optional source is
skipped rather than linked into a dangling target. An existing symlink is
refreshed; an existing regular file is an error, never overwritten.

### Observed Pi 1.5.0 layout (recorded, see tracker)

`~/.pi/agent` contains `auth.json`, `mcp.json`, `settings.json`,
`models-store.json`, `bin/`, `install/`, `npm/`, `sessions/`. It has no
`AGENTS.md`, `models.json` or `themes/`, so the corresponding links are
legitimately absent on this machine.

### `bh doctor`

Per registered repo: repo path is a directory; home and agent dirs exist;
`config.toml` parses; `settings.json`/`mcp.json` parse; each link is a symlink
whose target resolves; any `AGENTS.md` in the agent dir that is not a symlink is
reported as "stray". Exit code is non-zero when any finding is an error.

## Out of Scope

- Shared-skill links and opt-in packages (Unit 14 / later).
- herdr workspace creation and the `workspace` id (Unit 04).
- Anything that runs as the orchestrator or crew (Unit 05+).

## Verify

- [ ] `repo::detect` reads the git root, origin and a slugified id.
- [ ] `init` creates every directory, the registry entry and owner-only state
      files; running it twice is safe.
- [ ] Present global files become symlinks; absent ones are skipped.
- [ ] `doctor` is clean after init and flags a broken symlink and a stray
      `AGENTS.md`.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
