# Plan triage rules

These rules are bulkhead's, not the repo's. Apply them to every plan.

## Read first

- Read the whole plan before deciding anything.
- Read `context/architecture.md` and `context/project-overview.md` for the repo's boundaries.
- A plan with an open question is not declined; it is accepted with a note, or marked needs-review.

## Decide

- Accept a plan by writing one backlog item per independent unit of work.
- Decline a plan only with a concrete reason. Never decline silently.
- Mark a plan superseded when a newer plan replaces it, and name the replacement.
- Every plan ends accepted or declined. Nothing stays in the inbox.

## Split

- One backlog item per unit that can land on its own.
- Respect the boundaries in `context/architecture.md`: parallel work across boundaries, sequential within one.
- Record a dependency between items when one cannot start before another.
- Prefer small items that land in one PR.

## Write the item

- Title is an imperative phrase.
- Body states the outcome, the boundary, and the verification.
- No implementation steps that only the crew can know.
