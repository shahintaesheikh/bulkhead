# Unit 08 — Plan and Task Intake with Triage

- **Milestone:** M3
- **Depends on:** Unit 06
- **Boundary:** `bh` intake and prompts. No file watching (Unit 09) and no daemon triage.

## Goal

Get a plan from anywhere into a repo inbox and force it to an explicit triage
state, and turn an ask into a backlog item the orchestrator can spawn.

## Deliverables

1. `bh-core::frontmatter` — a small `key: value` frontmatter parser/renderer.
2. `bh-core::plan` — `TriageState` and the `Plan` sidecar type.
3. `bh::plan` — `ingest`, `set_state`, `list` across `plans/{inbox,accepted,declined}`.
4. `bh::backlog` — `create`/`list` for `backlog/<task-id>.md`.
5. `bh::lavish::LavishAxi` — `publish`/`poll`, plus `fake-lavish`.
6. `bh plan` and `bh task` subcommands; `prompts/triage-rules.md` installed with
   the contract; the `bh_plan` extension tool extended to triage actions.

## Design

### Frontmatter

bulkhead's markdown files start with a `---` block of flat `key: value` lines.
`parse` returns the fields and the body; text without frontmatter is all body.
This keeps plan and backlog state human-readable with `cat` and needs no YAML
dependency.

### Triage state

`TriageState`: `inbox`, `accepted`, `needs-review`, `declined`, `superseded`.
`bucket()` maps a state to one of the three on-disk directories (`needs-review`
stays in `inbox`, `superseded` goes to `declined`). Each plan is a file plus a
`<id>.meta.toml`; moving states moves both and rewrites the sidecar. Every
plan is explicitly accepted or declined; nothing lingers in the inbox.

### HTML plans

An `.html` plan is published with `lavish-axi publish <path>` and the returned
review URL is stored on the sidecar. Polling is the daemon's job (Unit 09); the
adapter exposes `poll` for it. The lavish CLI shape is assumed because the tool
is not installed while bulkhead is built.

### Task intake

`bh task "<ask>"` writes `backlog/<id>.md` with `title`/`state` frontmatter;
the id is the slugged title, deduped with `-2`, `-3`. `bh spawn` reads the
title from frontmatter when present.

## Out of Scope

- Watching inboxes and waking the orchestrator (Unit 09).
- Polling lavish and turning feedback into wakes (Unit 09/12).

## Verify

- [ ] `bh task` writes a frontmatter backlog item and dedupes ids.
- [ ] `bh plan` moves a plan from inbox to accepted/declined, moving file and
      sidecar and recording the reason.
- [ ] An HTML plan gets a lavish review URL.
- [ ] `prompts/triage-rules.md` is installed into the agent folder.
- [ ] `tsc --noEmit`, `cargo fmt --check`, `clippy -D warnings` and
      `cargo test --workspace` pass.
