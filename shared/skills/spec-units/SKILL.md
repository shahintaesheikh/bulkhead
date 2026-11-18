---
name: spec-units
description: "Format a plan as spec units: one verifiable unit of work at a time, each with a goal, deliverables, design, out-of-scope and a verify checklist. Use during plan triage or when turning an ask into backlog items."
---

# spec-units

Use this skill when a plan or an ask must become backlog items. A spec unit is
the smallest piece of work that lands and can be verified on its own.

## The format

Each unit has, in order:

1. `Goal` — one paragraph: the outcome, not the steps.
2. `Deliverables` — a numbered list of concrete artifacts.
3. `Design` — the decisions the implementer cannot infer, and why.
4. `Out of Scope` — what this unit deliberately does not do.
5. `Verify` — a checklist that is true when the unit is done.

Header fields: `Milestone`, `Depends on`, `Boundary`.

## Rules

- One unit per PR. If a unit cannot be verified end to end quickly, split it.
- Never mix a pure logic change with the I/O that calls it; land the logic and
  its tests first.
- Name the boundary: which crate or directory the unit may touch.
- Dependencies are explicit. A unit that needs another names it.
- `Out of Scope` is mandatory; it is how scope creep is caught.
- The `Verify` list is executable, not aspirational. Prefer commands.

## Worked shape

```
# Unit 07 — Delivery and `bh land`

- **Milestone:** M2
- **Depends on:** Unit 06
- **Boundary:** `bh` delivery I/O. No plan intake.

## Goal

Land an approved task through its repo's delivery mode.

## Deliverables
1. ...
2. ...

## Design
...

## Out of Scope
...

## Verify
- [ ] ...
```
