# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About Snail

Snail is a programming language that compiles to Python, offering terse Perl/awk-like syntax for quick scripts while preserving Python's semantics. Key characteristics:
- Curly-brace blocks instead of indentation
- Compiles to Python AST, then executes via CPython
- Two modes: regular Snail and awk mode for line-oriented processing
- Written in Rust with pyo3 for Python integration

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

## High-Level Architecture

### Compilation Pipeline

Snail → Parser → AST → Lowering → Python AST → Python Source → CPython exec

1. **Parser** (`src/parser.rs`, `src/snail.pest`):
   - Uses Pest parser generator with grammar defined in `src/snail.pest`
   - Produces Snail AST with source spans for error reporting
   - Two entry points: `parse_program()` for regular Snail, `parse_awk_program()` for awk mode
   - All string forms (quotes, regex `/.../`, subprocess `$(...)`, `@(...)`) support `{expr}` interpolation

2. **AST** (`src/ast.rs`, `src/awk.rs`):
   - `Program`: Top-level Snail AST with statement list
   - `AwkProgram`: Separate structure with BEGIN/END blocks and pattern/action rules
   - All nodes carry `SourceSpan` for traceback accuracy
   - Awk mode has special `$`-prefixed variables (`$l`, `$f`, `$n`, `$fn`, `$p`, `$m`)

3. **Lowering** (`src/lower.rs`):
   - Transforms Snail AST into Python AST representation (`PyModule`, `PyStmt`, `PyExpr`)
   - Handles Snail-specific features by generating helper code:
     - `?` operator → compact try/except using `__snail_compact_try` helper
     - `$(cmd)` subprocess capture → `__snail_subprocess_capture` helper
     - `@(cmd)` subprocess status → `__snail_subprocess_status` helper
     - Regex expressions → `__snail_regex_search` and `__snail_regex_compile` helpers
   - Awk variables (`$l`, `$n`, etc.) map to Python names (`__snail_line`, `__snail_nr_user`, etc.)
   - Awk mode wrapping: lower_awk_program() generates a Python main loop over input files/stdin

4. **Python Code Generation** (`src/lower.rs`):
   - `python_source()` converts Python AST to executable Python source strings
   - Preserves indentation and Python semantics exactly

5. **Compilation API** (`src/python.rs`):
   - `compile_snail_source()`: compiles Snail source to Python source code
   - `compile_snail_source_with_auto_print()`: compiles with optional auto-print of last expression
   - Used by CLI to generate Python code for execution

6. **CLI** (`src/main.rs`):
   - Handles `-f file.snail`, one-liner execution, and `--awk` mode
   - Uses pyo3 to execute generated Python code via CPython
   - `--python` flag shows generated Python code for debugging
   - `-P` flag disables auto-printing of last expression
   - Awk mode can be triggered by `#!/usr/bin/env -S snail --awk -f` shebang

### Error Handling

- **ParseError** (`src/error.rs`): Wraps Pest errors with source context
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

- **Parser tests** (`tests/parser.rs`): Validate AST structure from source
- **Lowering tests** (`tests/lower.rs`): Verify Python AST generation and code output
- **Awk mode tests** (`tests/awk.rs`): Pattern matching, BEGIN/END, variables
- **CLI tests** (`tests/cli.rs`): End-to-end execution via CLI, command-line interface behavior

Set `PYO3_PYTHON=python3` if you have multiple Python versions installed.

## Important Development Notes

- **Always update `examples/all_syntax.snail`** when adding new syntax features
- **CI must pass**: format (`cargo fmt --check`), clippy (`cargo clippy -- -D warnings`), and all tests
- The grammar is in `src/snail.pest` - parser logic uses Pest's PEG syntax
- Keep Python semantics identical; only syntax differs
- User-defined identifiers cannot start with `$` (reserved for awk mode)
- Vim/Neovim syntax highlighting available in `extras/vim/`

## Phase-Based Development

When implementing a phase from the README plan:
1. Read the phase definition in README.md carefully
2. Update `examples/all_syntax.snail` with new syntax examples
3. Add parser tests, lowering tests, and integration tests
4. Update `docs/REFERENCE.md` if user-facing syntax changes
5. Ensure all CI checks pass before completion
