# snail-core

Unified compilation API for the Snail programming language.

## Purpose

This crate serves as the main entry point for Snail compilation. It re-exports all types from the workspace crates and provides high-level compilation functions that orchestrate the complete pipeline from source code to executable Python.

## Key Components

- **compile_snail_source()**: One-step compilation from Snail source to Python source
- **compile_snail_source_with_auto_print()**: Compilation with optional auto-print for CLI mode
- Re-exports from all workspace crates:
  - `snail-ast`: All AST types
  - `snail-python-ast`: All Python AST types
  - `snail-error`: Error types and formatting
  - `snail-parser`: Parsing functions
  - `snail-lower`: Lowering functions
  - `snail-codegen`: Code generation functions

## Compilation Pipeline

The `compile_snail_source()` function orchestrates the complete compilation:

1. **Parse**: `parse_program()` or `parse_awk_program()` → Snail AST
2. **Lower**: `lower_program()` or `lower_awk_program()` → Python AST
3. **Codegen**: `python_source()` → Python source string

Each stage can fail with appropriate error types (`ParseError` or `LowerError`), wrapped in the unified `SnailError` type.

## Mode Selection

The `CompileMode` enum determines parsing and compilation behavior:
- **CompileMode::Snail**: Regular Snail mode with curly-brace syntax
- **CompileMode::Awk**: Awk mode with pattern/action processing

## Auto-Print Feature

When `auto_print_last` is true:
- In regular mode: Last expression is captured and pretty-printed
- In awk mode: Auto-print is handled at the block level during lowering

This enables REPL-like behavior for interactive use and CLI one-liners.

## Dependencies

- **snail-ast**: Core AST types
- **snail-python-ast**: Python AST types
- **snail-error**: Error handling
- **snail-parser**: Parsing logic
- **snail-lower**: AST transformation
- **snail-codegen**: Python code generation

## Used By

- **snail-cli**: Uses the compilation API to compile and execute Snail programs
- Library consumers can use this crate to embed Snail compilation in other tools
- Tests validate end-to-end compilation correctness

## Design

This crate provides a clean, minimal API surface for Snail compilation. Users only need to import `snail-core` to access all compilation functionality without managing individual workspace crates.
