# Contributing

Thanks for contributing to Strata.

## Development Rules

- Keep behavior changes explicit and well-tested.
- Prefer small, focused refactors over broad rewrites.
- Keep domain, storage, and UI concerns separate.
- Avoid introducing runtime panics in non-test paths.

## Local Checks

Run all checks before submitting:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Project Structure

- `src/domain.rs`: pure business logic and report generation.
- `src/storage.rs`: persistence, schema handling, and file paths.
- `src/app.rs` and `src/app/*`: UI state, event handling, and rendering.
- `src/cli.rs`: CLI command paths.
- `src/sand/*`: sand simulation details.

## Refactor Guidance

- Preserve keybindings and interaction semantics.
- Keep commits focused on one concern.
- Add tests when changing invariants or sorting/report logic.
