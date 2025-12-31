Phase 0 decisions

Language and runtime
- Implementation language: Rust (2024 edition).
- Target runtime: CPython 3.12 (aiming to keep 3.11+ compatible).
- Execution model: Snail -> Python AST -> compile() -> exec() in CPython.
- Interop:
  - Python import hook for `import foo.snail`.
  - Snail lowers `import` directly to Python `import` nodes.

Surface syntax goals
- Terse, script-friendly syntax inspired by Perl/awk.
- No whitespace-sensitive block structure.
- One-liners should scale into scripts without refactoring.

Non-goals (initially)
- Not a Python syntax replacement.
- No static type system in v0.
- No custom VM; execution stays inside CPython.

Examples (conceptual, not finalized)
- Assignment without strict whitespace: `x = 1; y = x + 2`.
- Pipeline-ish flow for quick scripts: `lines | filter /error/ | count`.
- Regex-literal-like convenience for text processing: `if /foo/ { ... }`.

Repository layout
- `src/`: Rust code.
- `docs/`: design docs and decisions.
- `.github/workflows/`: CI configs.

Tooling
- Build: `cargo build`
- Tests: `cargo test`
- Lint: `cargo fmt --check`, `cargo clippy -- -D warnings`
