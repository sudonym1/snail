# snail-codegen

Python code generation from Python AST.

## Purpose

This crate is the final stage of the Snail compilation pipeline. It converts Python AST nodes into executable Python source code strings, handling proper indentation, syntax formatting, and injection of runtime helper functions.

## Key Components

- **python_source()**: Converts a `PyModule` to Python source code
- **python_source_with_auto_print()**: Generates code with optional auto-print of last expression
- **PythonWriter**: Internal writer that manages indentation and code generation
- **Helper injection functions**:
  - `write_snail_try_helper()`: Injects the compact try (`?`) helper function
  - `write_snail_regex_helpers()`: Injects regex search and compile helpers
  - `write_snail_subprocess_helpers()`: Injects subprocess capture/status classes
  - `write_structured_accessor_helpers()`: Injects structured data accessor classes and vendored jmespath
  - `write_vendored_jmespath()`: Embeds jmespath library for structured queries

## Smart Helper Injection

The codegen scans the Python AST to detect which Snail features are used, and only injects the necessary helper functions:
- Only inject try helper if `__snail_compact_try` is referenced
- Only inject regex helpers if `__snail_regex_search` or `__snail_regex_compile` are used
- Only inject subprocess helpers if subprocess classes are used
- Only inject structured accessor helpers if `__SnailStructuredAccessor` or `json()` are used

This minimizes generated code size and avoids unused imports.

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

The vendored jmespath library is embedded as inline string data to avoid runtime dependencies, making Snail programs self-contained.
