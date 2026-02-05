# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## About Snail

Snail is a programming language that compiles to Python, offering terse Perl/awk-like syntax for quick scripts while preserving Python's semantics. Key characteristics:
- Curly-brace blocks instead of indentation
- Compiles to Python AST, then executes via CPython
- Two modes: regular Snail and awk mode for line-oriented processing

## Build and Test Commands

```bash
# Sync Python tooling (installs dev extras into .venv)
uv sync --extra dev

# Build the Rust crates
cargo build

# Build/install the Python extension in the uv environment
uv run -- python -m maturin develop

# Run all Rust tests (parser, lowering, awk mode)
cargo test

# Run Python CLI tests
uv run -- python -m pytest python/tests

# Run tests for a specific module
cargo test parser
cargo test awk

# Run a specific test by name
cargo test parses_basic_program

# Run the CLI (after maturin develop)
uv run -- snail "print('hello')"
uv run -- snail -f examples/all_syntax.snail
uv run -- snail --awk -f examples/awk.snail

# Format code
cargo fmt

# Check formatting without changes
cargo fmt --check

# Lint with clippy
cargo clippy -- -D warnings

```

## Planning Requirements

**CRITICAL**: When creating a medium or large plan, offer to store the full
context to execute that plan as markdown under the "plans" folder. Make the
folder if it is missing and required.

## ⚠️ MANDATORY: CI Requirements Before Committing/Pushing

**CRITICAL**: Before creating ANY commit, push, or pull request, you MUST run `make test` as the **final** command and ensure it passes. No exceptions.
**CRITICAL**: Make test will only need permissions once to run uv --sync. Do not ask for permission when running make test.

### When to Run Which Checks

- **Formatting**: Run `cargo fmt` during iteration; `make test` runs `cargo fmt --check`.
- **Rust build**: Run `RUSTFLAGS="-D warnings" cargo build` when touching Rust code; `make test` runs it.
- **Linting**: Run `cargo clippy -- -D warnings` before final verification; `make test` runs it.
- **Rust tests**: Run targeted `cargo test <name>` as needed; `make test` runs `cargo test`.
- **Python CLI tests**: Run `uv run -- python -m pytest python/tests` when touching the CLI; `make test` runs it.

IMPORTANT: after making any edit, always run at least some test that covers
that edit. If it isn't immediately obvious which test to run, just run `make
test`.

### Required Final CI Step

```bash
# Must be the last check before commit/push/PR
make test
```

### Pre-Commit/Pre-PR Checklist:

- [ ] `make test` passes (run **last** before commit/push/PR)
- [ ] If adding new syntax: `examples/all_syntax.snail` updated
- [ ] Appropriate tests added for new functionality

**DO NOT**:
- ❌ Skip the final `make test` run
- ❌ Commit/push without `make test` passing
- ❌ Create a PR without verifying `make test` passes
- ❌ Assume tests/build still pass without running them

**If any check fails**: Fix the issues before proceeding. Do not create commits or PRs with failing CI checks.

## Branching Policy

Any new branch created by an agent must start with `fix/` or `feat/` to indicate fixes or features.

## Repository Structure

The repository is organized as a Cargo workspace with the following crates:

- **`snail-ast`**: Snail AST definitions (Program, AwkProgram, statements, expressions)
- **`snail-parser`**: Pest-based parser that converts Snail source to AST
- **`snail-lower`**: Lowers Snail AST to Python `ast` nodes via pyo3
- **`snail-error`**: Error types (ParseError, LowerError, SnailError)
- **`snail-python`**: Compilation API plus the pyo3 module used by the Python package and CLI

## High-Level Architecture

### Compilation Pipeline

Snail → Parser → AST → Lowering → Python AST → in-process exec

1. **Parser** (`crates/snail-parser/`):
   - Uses Pest parser generator with grammar defined in `crates/snail-parser/src/snail.pest`
   - Produces Snail AST with source spans for error reporting
   - Two entry points: `parse_program()` for regular Snail, `parse_awk_program()` for awk mode
   - All string forms (quotes, regex `/.../`, subprocess `$(...)`, `@(...)`) support `{expr}` interpolation

2. **AST** (`crates/snail-ast/`):
   - `Program`: Top-level Snail AST with statement list (`crates/snail-ast/src/ast.rs`)
   - `AwkProgram`: Separate structure with BEGIN/END blocks and pattern/action rules (`crates/snail-ast/src/awk.rs`)
   - All nodes carry `SourceSpan` for traceback accuracy
   - Awk mode has special `$`-prefixed variables (`$0`, `$1`, `$n`, `$fn`, `$f`, `$src`, `$m`)

3. **Lowering** (`crates/snail-lower/`):
   - Transforms Snail AST into Python `ast` nodes via pyo3
   - Handles Snail-specific features by generating helper calls (provided by `snail.runtime`):
     - `?` operator → compact try/except using `__snail_compact_try`
     - `$(cmd)` subprocess capture → `__SnailSubprocessCapture`
     - `@(cmd)` subprocess status → `__SnailSubprocessStatus`
     - Regex expressions → `__snail_regex_search` and `__snail_regex_compile`
   - Awk variables (`$0`, `$n`, etc.) map to Python names (`__snail_line`, `__snail_nr_user`, etc.)
   - Awk mode wrapping: lower_awk_program() generates a Python main loop over input files/stdin

4. **Python AST**:
   - Uses Python's built-in `ast` nodes constructed in Rust via pyo3

5. **Compilation API** (`crates/snail-python/`):
   - `compile_snail_source_with_auto_print()`: compiles Snail source to a Python AST module, with optional auto-print of the last expression
   - Used by the Python module to execute code in-process

6. **Python CLI** (`python/snail/cli.py`):
   - Handles `-f file.snail`, one-liner execution, and `--awk` mode
   - Executes generated Python code in-process via the `snail` extension module
   - `-P` flag disables auto-printing of last expression
   - Awk mode can be triggered by `#!/usr/bin/env -S snail --awk -f` shebang

### Error Handling

- **ParseError** (`crates/snail-error/`): Wraps Pest errors with source context
- **LowerError**: Raised when AST can't be lowered to Python
- **SnailError**: Unified error enum wrapping both
- All errors preserve source spans for precise diagnostics

## Key Snail Features

### Snail-Specific Syntax
- **Compact try operator**: `expr?` returns exception on failure; `expr:fallback?` evaluates fallback with exception in `$e`
- **Subprocess syntax**: `$(cmd)` captures stdout, `@(cmd)` runs without capture (both raise on non-zero exit)
- **Regex expressions**: `string in /pattern/` performs `re.search()` and returns match object
- **Compound expressions**: `(stmt1; stmt2; expr)` evaluates to final expression
- All string forms support `{expr}` interpolation like f-strings

### Awk Mode
- Pattern/action pairs: `pattern { action }` evaluated per input line
- `BEGIN { }` and `END { }` blocks
- Built-in variables (all `$`-prefixed, reserved by Snail):
  - `$0`: current line
  - `$1`, `$2`, ...: whitespace-split fields
  - `$f`: all fields as a list
  - `$n`: global line number
  - `$fn`: per-file line number
  - `$src`: current file path
  - `$m`: last regex match object
- Bare pattern prints matching lines; bare block runs for every line
- Regex patterns: `/pattern/` matches against `$0` implicitly

## Testing Strategy

- **Parser tests** (`crates/snail-parser/tests/parser.rs`): Validate AST structure from source
- **Python CLI tests** (`python/tests/test_cli.py`): End-to-end execution via CLI, command-line interface behavior

**Note on virtual environments:** Python tools run inside the uv-managed environment; prefer `uv run -- ...` for Python commands.

## Important Development Notes

- **Always update `examples/all_syntax.snail`** when adding new syntax features
- **MANDATORY CI checks must ALL pass** before any commit/push/PR - see "MANDATORY: CI Requirements" section above
- The grammar is in `crates/snail-parser/src/snail.pest` - parser logic uses Pest's PEG syntax
- Keep Python semantics identical; only syntax differs
- User-defined identifiers cannot start with `$` (reserved for awk mode)
- Vim/Neovim syntax highlighting available in `extras/vim/`
- Tree-sitter grammar available in `extras/tree-sitter-snail/`

## Phase-Based Development

When implementing a phase from the project plan:
1. Read the phase definition in `docs/PLANNING.md` carefully
2. Update `examples/all_syntax.snail` with new syntax examples
3. Add parser tests, lowering tests, and integration tests
4. Update `docs/REFERENCE.md` if user-facing syntax changes
5. **RUN ALL MANDATORY CI CHECKS** (see "MANDATORY: CI Requirements" section).
   This is just `make test`
6. Only commit/push after ALL CI checks pass
