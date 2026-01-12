# snail-codegen

Python code generation from Python AST.

## Purpose

This crate is the final stage of the Snail compilation pipeline. It converts Python AST nodes into executable Python source code strings, handling proper indentation and syntax formatting.

## Key Components

- **python_source()**: Converts a `PyModule` to Python source code
- **python_source_with_auto_print()**: Generates code with optional auto-print of last expression
- **PythonWriter**: Internal writer that manages indentation and code generation

## Auto-Print Feature

When `auto_print_last` is true, the codegen wraps the last expression statement to:
1. Capture the result in a temporary variable
2. Print strings directly
3. Pretty-print non-None objects using `pprint`
4. Do nothing if the result is None
5. Skip auto-print if the statement was terminated with semicolon

This provides REPL-like behavior for CLI one-liners.

## Dependencies

- **snail-ast**: Uses `StringDelimiter` for proper string formatting
- **snail-python-ast**: Consumes `PyModule`, `PyStmt`, `PyExpr` and generates source
- **snail-lower**: Uses helper constant names (e.g., `SNAIL_TRY_HELPER`)

## Used By

- **snail-core**: Calls `python_source()` or `python_source_with_auto_print()` to generate final output
- Tests validate that generated Python code is syntactically correct

## Design

The code generator maintains proper Python indentation (4 spaces) and formats all Python constructs correctly. It handles edge cases like single-element tuples `(x,)`, raw strings, f-strings with escaping, and elif chains.

Runtime helpers (including vendored jmespath) live in the Python package under `python/snail/runtime`.
