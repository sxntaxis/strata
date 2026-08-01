# Notebook workflow

1. Recover current authority from `project.meta.toml`, `project.notebook.toml`, `notebook/NOW.md`, and the smallest relevant accepted document.
2. Verify current source, tests, CI, issue, and runtime state before planning implementation.
3. Define one bounded work unit with explicit non-goals and closure evidence.
4. Work on an isolated branch and pull request; never write directly to `main`.
5. Keep facts, decisions, proposals, and unknowns distinct.
6. Certify the exact final review tree, not a generated or temporary assembly tree.
7. Merge only when scope, tests, documentation, and issue state agree.
8. Update the owning Notebook frontier after a program closes.

GitHub issues remain implementation/closure units. Notebook work records own sequencing and context but must not duplicate volatile PR status.
