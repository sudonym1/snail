# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## About Snail

Snail is a programming language that compiles to Python, offering terse Perl/awk-like syntax for quick scripts while preserving Python's semantics. Key characteristics:
- Curly-brace blocks instead of indentation
- Compiles to Python AST, then executes via CPython
- Two modes: regular Snail and awk mode for line-oriented processing

## Build and Test Commands

```bash
# Build the project
cargo build

# Run all tests (includes parser, lowering, awk mode, CLI tests)
cargo test

# Run tests for a specific module
cargo test parser
cargo test awk

# Run a specific test by name
cargo test parses_basic_program

# Build and run the CLI
cargo run -- "print('hello')"
cargo run -- -f examples/all_syntax.snail
cargo run -- --awk -f examples/awk.snail

# Using the built binary directly
./target/debug/snail "print('hello')"

# Format code
cargo fmt

# Check formatting without changes
cargo fmt --check

# Lint with clippy
cargo clippy -- -D warnings
```

## ⚠️ MANDATORY: CI Requirements Before Committing/Pushing

**CRITICAL**: Before creating ANY commit, push, or pull request, you MUST run all four CI checks below and ensure they ALL pass. No exceptions.

### Required CI Checks (ALL must pass):

```bash
# 1. FORMATTING - Must pass with no changes
cargo fmt --check

# 2. BUILD - Must pass with NO compiler warnings (warnings treated as errors)
RUSTFLAGS="-D warnings" cargo build

# 3. LINTING - Must pass with NO clippy warnings
cargo clippy -- -D warnings

# 4. TESTS - Must pass completely
RUSTFLAGS="-D warnings" cargo test
```

### Pre-Commit/Pre-PR Checklist:

- [ ] `cargo fmt --check` passes (or run `cargo fmt` to fix formatting)
- [ ] `RUSTFLAGS="-D warnings" cargo build` passes with zero compiler warnings
- [ ] `cargo clippy -- -D warnings` passes with zero clippy warnings
- [ ] `cargo test` passes with all tests succeeding
- [ ] If adding new syntax: `examples/all_syntax.snail` updated
- [ ] Appropriate tests added for new functionality

**DO NOT**:
- ❌ Skip any CI check "to save time"
- ❌ Commit/push without running all four checks
- ❌ Create a PR without verifying all checks pass
- ❌ Assume tests/build still pass without running them

**If any check fails**: Fix the issues before proceeding. Do not create commits or PRs with failing CI checks.

## Repository Structure

The repository is organized as a Cargo workspace with the following crates:

- **`snail-ast`**: Snail AST definitions (Program, AwkProgram, statements, expressions)
- **`snail-parser`**: Pest-based parser that converts Snail source to AST
- **`snail-lower`**: Lowers Snail AST to Python AST representation
- **`snail-python-ast`**: Python AST node definitions (PyModule, PyStmt, PyExpr, etc.)
- **`snail-codegen`**: Generates Python source code from Python AST
- **`snail-error`**: Error types (ParseError, LowerError, SnailError)
- **`snail-core`**: High-level compilation API (compile_snail_source, etc.)
- **`snail-cli`**: Command-line interface and integration tests

## High-Level Architecture

### Compilation Pipeline

Snail → Parser → AST → Lowering → Python AST → Python Source → subprocess exec

1. **Parser** (`crates/snail-parser/`):
   - Uses Pest parser generator with grammar defined in `crates/snail-parser/src/snail.pest`
   - Produces Snail AST with source spans for error reporting
   - Two entry points: `parse_program()` for regular Snail, `parse_awk_program()` for awk mode
   - All string forms (quotes, regex `/.../`, subprocess `$(...)`, `@(...)`) support `{expr}` interpolation

2. **AST** (`crates/snail-ast/`):
   - `Program`: Top-level Snail AST with statement list (`crates/snail-ast/src/ast.rs`)
   - `AwkProgram`: Separate structure with BEGIN/END blocks and pattern/action rules (`crates/snail-ast/src/awk.rs`)
   - All nodes carry `SourceSpan` for traceback accuracy
   - Awk mode has special `$`-prefixed variables (`$l`, `$f`, `$n`, `$fn`, `$p`, `$m`)

3. **Lowering** (`crates/snail-lower/`):
   - Transforms Snail AST into Python AST representation (`PyModule`, `PyStmt`, `PyExpr`)
   - Handles Snail-specific features by generating helper code:
     - `?` operator → compact try/except using `__snail_compact_try` helper
     - `$(cmd)` subprocess capture → `__snail_subprocess_capture` helper
     - `@(cmd)` subprocess status → `__snail_subprocess_status` helper
     - Regex expressions → `__snail_regex_search` and `__snail_regex_compile` helpers
   - Awk variables (`$l`, `$n`, etc.) map to Python names (`__snail_line`, `__snail_nr_user`, etc.)
   - Awk mode wrapping: lower_awk_program() generates a Python main loop over input files/stdin

4. **Python AST** (`crates/snail-python-ast/`):
   - Defines Python AST node types used as intermediate representation
   - Structures mirror Python's AST for accurate code generation

5. **Python Code Generation** (`crates/snail-codegen/`):
   - `python_source()` converts Python AST to executable Python source strings
   - Preserves indentation and Python semantics exactly

6. **Compilation API** (`crates/snail-core/`):
   - `compile_snail_source()`: compiles Snail source to Python source code
   - `compile_snail_source_with_auto_print()`: compiles with optional auto-print of last expression
   - Used by CLI to generate Python code for execution

7. **CLI** (`crates/snail-cli/src/main.rs`):
   - Handles `-f file.snail`, one-liner execution, and `--awk` mode
   - Executes generated Python code via subprocess (respects virtual environments)
   - Uses `python3` by default, configurable via `PYTHON` environment variable
   - `--python` flag shows generated Python code for debugging
   - `-P` flag disables auto-printing of last expression
   - Awk mode can be triggered by `#!/usr/bin/env -S snail --awk -f` shebang

### Error Handling

- **ParseError** (`crates/snail-error/`): Wraps Pest errors with source context
- **LowerError**: Raised when AST can't be lowered to Python
- **SnailError**: Unified error enum wrapping both
- All errors preserve source spans for precise diagnostics

## Key Snail Features

### Snail-Specific Syntax
- **Compact try operator**: `expr?` returns exception on failure; `expr ? fallback` evaluates fallback with exception in `$e`
- **Subprocess syntax**: `$(cmd)` captures stdout, `@(cmd)` runs without capture (both raise on non-zero exit)
- **Regex expressions**: `string in /pattern/` performs `re.search()` and returns match object
- **Compound expressions**: `(stmt1; stmt2; expr)` evaluates to final expression
- All string forms support `{expr}` interpolation like f-strings

### Awk Mode
- Pattern/action pairs: `pattern { action }` evaluated per input line
- `BEGIN { }` and `END { }` blocks
- Built-in variables (all `$`-prefixed, reserved by Snail):
  - `$l`: current line
  - `$f`: whitespace-split fields array
  - `$n`: global line number
  - `$fn`: per-file line number
  - `$p`: current file path
  - `$m`: last regex match object
- Bare pattern prints matching lines; bare block runs for every line
- Regex patterns: `/pattern/` matches against `$l` implicitly

## Testing Strategy

- **Parser tests** (`crates/snail-parser/tests/parser.rs`): Validate AST structure from source
- **Lowering tests** (`crates/snail-cli/tests/lower.rs`): Verify Python AST generation and code output
- **Awk mode tests** (`crates/snail-cli/tests/awk.rs`): Pattern matching, BEGIN/END, variables
- **CLI tests** (`crates/snail-cli/tests/cli.rs`): End-to-end execution via CLI, command-line interface behavior
- **Quote interpolation tests** (`crates/snail-cli/tests/quotes_in_expressions.rs`): String interpolation in various contexts

**Note on virtual environments:** The CLI executes Python via subprocess, automatically respecting any active virtual environment.

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
5. **RUN ALL MANDATORY CI CHECKS** (see "MANDATORY: CI Requirements" section):
   - `cargo fmt --check` (fix with `cargo fmt` if needed)
   - `RUSTFLAGS="-D warnings" cargo build` (must pass with zero compiler warnings)
   - `cargo clippy -- -D warnings` (must pass with zero clippy warnings)
   - `RUSTFLAGS="-D warnings" cargo test` (all tests must pass)
6. Only commit/push after ALL CI checks pass
