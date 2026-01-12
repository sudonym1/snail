# Python-first Snail plan

## Decisions
- Primary UX remains the CLI; still accepts `.snail` files and one-liners.
- Drop `--python` output flag.
- Distribute via pip only (no `cargo install`).
- Use `pyo3` with `maturin` and `abi3`, supporting Python >= 3.10.
- Helpers reimplemented as pure Python modules under `snail.runtime.*`.
- Python-style tracebacks preferred for runtime errors.

## Implementation Plan
1. **Python API + AST execution**
   - Add a `pyo3` module exposing `compile`, `exec`, and `parse`.
   - `compile` returns a Python code object (lowered AST -> `compile`).
   - `exec` runs in-process, sets `sys.argv`, `__file__`, `__name__`, and returns exit code.
   - Preserve `mode="snail" | "awk"` and `auto_print` semantics.

2. **Runtime helpers as Python modules**
   - Implement helpers in `snail.runtime.*`:
     - `snail.runtime.try`
     - `snail.runtime.regex`
     - `snail.runtime.subprocess`
     - `snail.runtime.structured_accessor`
     - `snail.runtime.awk`
   - Update lowering to reference these helpers instead of injecting code.

3. **CLI wrapper (primary UX)**
   - Provide `snail` console script via `snail.cli:main`.
   - Support `-f`, inline code, `--awk`, `--parse-only`, `--no-print`, `--version`.
   - Use Python tracebacks for runtime errors.

4. **Packaging via pip**
   - Add `pyproject.toml` with `maturin` config and `abi3` targeting Python 3.10+.
   - Build OS-agnostic wheels for Linux/macOS/Windows.

5. **Testing + migration**
   - Keep Rust parser/lowering tests.
   - Add Python CLI tests (pytest) for awk, parse-only, no-print, argv passing, and exit codes.
   - Remove `snail-cli` crate from the workspace.
