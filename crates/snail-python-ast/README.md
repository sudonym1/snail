# snail-python-ast

Python AST (Abstract Syntax Tree) data structures for Snail's compilation target.

## Purpose

This crate defines the intermediate representation used by Snail when compiling to Python. It provides Python-specific AST nodes that mirror Python's syntax and semantics, serving as the bridge between Snail AST and executable Python code.

## Key Components

- **PyModule**: Top-level Python module containing statements
- **PyStmt**: Python statement types (If, While, For, FunctionDef, ClassDef, etc.)
- **PyExpr**: Python expression types (Name, Number, String, Call, etc.)
- **PyParameter**: Function parameter representations
- **PyArgument**: Function call argument types (positional, keyword, *args, **kwargs)
- **PyImportName**: Import statement components
- **PyWithItem**: Context manager items for with statements
- **PyExceptHandler**: Exception handler clauses

## Dependencies

- **snail-ast**: Uses `SourceSpan` for source location tracking and `StringDelimiter` for string literal formatting

## Used By

- **snail-lower**: Produces Python AST as its output when transforming Snail AST
- **snail-codegen**: Consumes Python AST and generates executable Python source code
- **snail-core**: Re-exports all types for the unified API

## Design

Python AST nodes preserve `SourceSpan` information from the original Snail source, enabling accurate error reporting even after transformation. The structure closely mirrors Python's actual AST to ensure semantic correctness of the generated code.
