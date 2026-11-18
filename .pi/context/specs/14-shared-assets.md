# Unit 14 — Shared Bootstrap Prompt and `spec-units` Skill

- **Milestone:** — (can run any time after Unit 08)
- **Depends on:** Unit 08
- **Boundary:** opt-in Pi assets and the linking that installs them.

## Goal

Give a managed repo a way to *become* managed — a `/bootstrap-context` prompt
that writes its committed context — and give plan triage a reusable
`spec-units` skill, both opted into per repo.

## Deliverables

1. `shared/prompts/bootstrap-context.md` — the `/bootstrap-context` prompt.
2. `shared/skills/spec-units/SKILL.md` — the spec-unit format and rules.
3. `agent_folder` links each skill named in the repo's `config.toml` `skills`
   list, and every shared prompt template.

## Design

### Shared assets are a pool, not a default

`~/.bulkhead/shared/` holds one copy of each opt-in asset. `bh init` and
`bh attach` link into the repo's agent folder:

- a skill is linked only when the repo's `skills` list names it;
- every file in `shared/prompts/` is linked, because an unused slash command
  costs no context until it is invoked.

Linking is idempotent and, like the global-files linking, never overwrites a
real file in the agent folder.

### The prompt

`/bootstrap-context` reads the repo and writes exactly
`context/project-overview.md`, `context/architecture.md` and a pointer table in
`AGENTS.md` — the managed layout from the architecture. It is a captain-run
command, not something the orchestrator triggers.

### The skill

`spec-units` encodes the unit format used in `context/specs/`: goal,
deliverables, design, out of scope, verify. It is loaded during triage so the
orchestrator can split a plan into units that land independently.

## Out of Scope

- Installing assets to `~/.bulkhead/shared/` automatically.
- Any daemon behavior.

## Verify

- [ ] A repo that lists `spec-units` gets `agent/skills/spec-units` as a symlink.
- [ ] Every shared prompt appears under `agent/prompts/`.
- [ ] A repo that does not list a skill does not get it.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test --workspace` pass.
