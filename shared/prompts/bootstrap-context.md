# /bootstrap-context

Write or refresh this managed repo's committed context so bulkhead and its crew
can read the repo on demand. Produce exactly three things.

## 1. `context/project-overview.md`

- What this product is, in one paragraph.
- Goals, as a short numbered list.
- The core user flow, numbered.
- Features, grouped.
- Scope: what is in and what is out.
- Success criteria.

## 2. `context/architecture.md`

- The stack, as a small table (layer, technology, role).
- System boundaries: which directory holds which responsibility.
- The storage model: every file bulkhead or the repo writes, and where.
- Auth and access: who can read and write what.
- Invariants: the rules a change must not break, numbered.

## 3. A pointer table in `AGENTS.md`

Append a short table that points at the two files above:

```
| Read this | When |
| --- | --- |
| context/project-overview.md | Before deciding what to build |
| context/architecture.md | Before deciding how to build it |
```

## Rules

- Read the repo first. Do not invent a stack or a boundary.
- Keep both files short enough to read in one sitting.
- Invariants are facts, not wishes. If you cannot name one, leave the section out.
- Commit only these three files. Nothing else.
