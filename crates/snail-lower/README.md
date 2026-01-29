# snail-lower

Compatibility shim that re-exports lowering APIs from `snail-python`.

## Purpose

This crate preserves the historical `snail-lower` API surface for downstream
tests and tools, while keeping the actual lowering implementation inside
`snail-python` (the only crate that depends directly on pyo3).

## Key Components

- **lower_program()**: Transforms a regular `Program` to a Python `ast.Module`
- **lower_awk_program()**: Transforms an `AwkProgram` to a Python `ast.Module` with awk runtime
- **lower_awk_program_with_auto_print()**: Awk lowering with optional auto-print

## Snail Feature Transformations

- **Compact try operator** (`expr?`): Transformed into `__snail_compact_try(lambda: expr)` call
- **Compact try with fallback** (`expr:fallback?`): Transformed with fallback lambda
- **Subprocess capture** (`$(cmd)`): Transformed into `__SnailSubprocessCapture(cmd)` instance
- **Subprocess status** (`@(cmd)`): Transformed into `__SnailSubprocessStatus(cmd)` instance
- **Regex expressions** (`/pattern/`): Transformed into `__snail_regex_compile(pattern)` call
- **Regex matching** (`string in /pattern/`): Transformed into `__snail_regex_search(string, pattern)` call
- **Structured accessors** (`$[query]`): Transformed into `__SnailStructuredAccessor(query)` instance
- **Awk variables**: `$0`, `$<num>`, `$n`, `$fn`, `$f`, `$p`, `$m` mapped to Python variable names

## Awk Mode Lowering

When lowering awk programs, generates a complete Python AST that:
1. Imports `sys` for accessing command-line arguments and stdin
2. Executes BEGIN blocks before processing input
3. Creates a main loop that reads lines from files or stdin
4. Updates awk variables (`$0`, `$<num>`, `$n`, etc.) for each line
5. Evaluates patterns and executes actions for matching lines
6. Executes END blocks after all input is processed

## Dependencies

- **snail-python**: Owns the lowering implementation and pyo3 bindings

## Design

The lowering process preserves Python semantics exactly - only syntax differs. `SourceSpan` information is used to populate Python AST location metadata for accurate error reporting.
