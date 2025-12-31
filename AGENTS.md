# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Rust sources (currently `main.rs` plus the `src/snail.pest` grammar).
- `docs/`: design notes and decisions (see `docs/DECISIONS.md`).
- `.github/workflows/`: CI definitions (format, lint, test).
- `README.md`: project overview and phased plan.

## Build, Test, and Development Commands
- `cargo build`: build the project in debug mode.
- `cargo test`: run unit/integration tests (Rust’s built-in test harness).
- `cargo fmt --check`: verify formatting matches `rustfmt`.
- `cargo clippy -- -D warnings`: lint and treat warnings as errors.

## Coding Style & Naming Conventions
- Use `rustfmt` defaults; 4-space indentation from formatter output.
- Rust naming: `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Grammar files live under `src/` and use `.pest` extension.
- Keep identifiers ASCII; avoid new Unicode unless required by the file.

## Testing Guidelines
- Use Rust’s built-in test framework (`#[test]`).
- Prefer small unit tests near the module they cover.
- Name tests by behavior, e.g., `parses_if_stmt` or `lowers_import`.
- Run the full suite with `cargo test`.

## Commit & Pull Request Guidelines
- Recent commits use short, direct messages (e.g., “Initial commit”).
- Follow the same style: concise, imperative, no prefixes required.
- PRs should describe the change, include relevant test commands run, and link any related issue if one exists.

## Notes for Contributors
- The grammar is currently “Python with curly braces”; avoid expanding syntax until the core pipeline (parse → Python AST → exec) is working.
