## Application Building Context

bulkhead is a per-repo Pi orchestrator distro: one persistent orchestrator per repo, a parallel crew in treehouse worktrees, herdr panes, and a Rust daemon (`bhd`) for zero-token supervision.

Read the following files in order before implementing or making any architectural decision:

1. `context/project-overview.md` — product definition, goals, features, and scope
2. `context/architecture.md` — workspace layout, boundaries, storage model, and invariants
3. `context/code-standards.md` — Rust, TypeScript, and prompt conventions
4. `context/ai-workflow-rules.md` — development workflow, scoping rules, and delivery approach
5. `context/progress-tracker.md` — current phase, completed work, open questions, and next steps

The build plan and unit specs live in `context/specs/`. Implement one unit at a time, exactly as its spec says.

Update `context/progress-tracker.md` after each meaningful implementation change.

If implementation changes the architecture, scope, or standards documented in the context files, stop and propose the change to the relevant file before continuing. Changes to the invariants in `context/architecture.md` need the captain's explicit approval.

The captain is learning Rust through this project. Any time there is a good learning experience for learning Rust definitions and Syntax, skeleton it, and then mention it obviously to the captain to go in and implement it. 

Work on only separate worktrees, any time you want to implement a change, other than the initial commit, open a separate worktree and then open it as a PR once the work on that worktree is completed. 
