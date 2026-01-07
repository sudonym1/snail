# snail-lower

AST transformation layer that lowers Snail AST to Python AST.

## Purpose

This crate is the semantic transformation core of the Snail compiler. It takes Snail AST nodes and transforms them into equivalent Python AST nodes, handling all Snail-specific features by generating appropriate Python code patterns and helper function calls.

## Key Components

- **lower_program()**: Transforms a regular `Program` to `PyModule`
- **lower_awk_program()**: Transforms an `AwkProgram` to `PyModule` with awk runtime
- **lower_awk_program_with_auto_print()**: Awk lowering with optional auto-print
- Helper constants for generated Python code:
  - `SNAIL_TRY_HELPER`: Name for the `?` operator helper function
  - `SNAIL_SUBPROCESS_CAPTURE_CLASS`: Class for `$(cmd)` subprocess capture
  - `SNAIL_SUBPROCESS_STATUS_CLASS`: Class for `@(cmd)` subprocess status
  - `SNAIL_REGEX_SEARCH/COMPILE`: Regex helper functions
  - `SNAIL_STRUCTURED_ACCESSOR_CLASS`: Class for structured data queries

## Snail Feature Transformations

- **Compact try operator** (`expr?`): Transformed into `__snail_compact_try(lambda: expr)` call
- **Compact try with fallback** (`expr ? fallback`): Transformed with fallback lambda
- **Subprocess capture** (`$(cmd)`): Transformed into `__SnailSubprocessCapture(cmd)` instance
- **Subprocess status** (`@(cmd)`): Transformed into `__SnailSubprocessStatus(cmd)` instance
- **Regex expressions** (`/pattern/`): Transformed into `__snail_regex_compile(pattern)` call
- **Regex matching** (`string in /pattern/`): Transformed into `__snail_regex_search(string, pattern)` call
- **Structured accessors** (`$[query]`): Transformed into `__SnailStructuredAccessor(query)` instance
- **Awk variables**: `$l`, `$f`, `$n`, `$fn`, `$p`, `$m` mapped to Python variable names

## Awk Mode Lowering

When lowering awk programs, generates a complete Python program that:
1. Imports `sys` for accessing command-line arguments and stdin
2. Executes BEGIN blocks before processing input
3. Creates a main loop that reads lines from files or stdin
4. Updates awk variables (`$l`, `$f`, `$n`, etc.) for each line
5. Evaluates patterns and executes actions for matching lines
6. Executes END blocks after all input is processed

## Dependencies

- **snail-ast**: Consumes Snail `Program` and `AwkProgram`
- **snail-python-ast**: Produces Python `PyModule` and related nodes
- **snail-error**: Returns `LowerError` on transformation failures

## Used By

- **snail-codegen**: Consumes the Python AST to generate source code
- **snail-core**: Calls lowering functions as part of the compilation pipeline
- Tests validate transformation correctness

## Design

The lowering process preserves Python semantics exactly - only syntax differs. All `SourceSpan` information is preserved through the transformation to enable accurate error reporting in the generated Python code.
