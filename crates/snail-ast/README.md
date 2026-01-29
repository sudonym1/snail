# snail-ast

Core AST (Abstract Syntax Tree) data structures for the Snail programming language.

## Purpose

This crate defines the foundational data structures that represent Snail programs after parsing. It contains pure data types with no dependencies on other crates, making it the base layer of the Snail compilation pipeline.

## Key Components

- **Program**: Top-level representation of a Snail program containing a list of statements
- **Stmt**: Enumeration of all statement types (if, while, for, def, class, try, etc.)
- **Expr**: Enumeration of all expression types (literals, operators, calls, etc.)
- **AwkProgram**: Specialized AST for awk mode with BEGIN/END blocks and pattern/action rules
- **AwkRule**: Pattern/action pairs used in awk mode
- **SourceSpan/SourcePos**: Source location tracking for error reporting

## Dependencies

None - this crate has no dependencies and provides pure data structures.

## Used By

- **snail-error**: Uses `SourceSpan` for error reporting with source locations
- **snail-parser**: Produces `Program` and `AwkProgram` as output
- **snail-lower**: Consumes Snail AST and transforms it to Python `ast` nodes via pyo3
- **snail-python**: Uses AST types as part of the compilation pipeline

## Design

All AST nodes carry `SourceSpan` information to enable precise error messages and tracebacks that point to the original Snail source code.
