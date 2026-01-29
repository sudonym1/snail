# snail-parser

Pest-based parser for the Snail programming language.

## Purpose

This crate contains the parser that transforms Snail source code into a Snail AST. It uses the Pest parser generator with a PEG (Parsing Expression Grammar) to recognize Snail syntax and produce structured AST nodes with source location information.

## Key Components

- **parse_program()**: Parses regular Snail code into a `Program`
- **parse_awk_program()**: Parses awk-mode Snail code into an `AwkProgram`
- **snail.pest**: PEG grammar defining Snail's syntax
- **SnailParser**: Pest-generated parser from the grammar

## Grammar File

The `snail.pest` file defines all of Snail's syntax rules including:
- Curly-brace block structure
- Expression syntax (operators, calls, literals, etc.)
- Statement types (if/while/for/try/with/def/class)
- Snail-specific features (?, $(), @(), regex, structured accessors)
- Awk mode syntax (BEGIN/END blocks, pattern/action rules)
- String interpolation with {expr}

## Dependencies

- **snail-ast**: Produces `Program` and `AwkProgram` as output
- **snail-error**: Returns `ParseError` on syntax errors
- **pest**: Parser generator library (2.7)
- **pest_derive**: Procedural macro for grammar compilation

## Used By

- **snail-python**: Calls `parse_program()` or `parse_awk_program()` as the first compilation stage
- Tests validate AST structure from various Snail source inputs

## Design

The parser preserves complete source location information (`SourceSpan`) for every AST node, enabling precise error messages and tracebacks. It handles all Snail syntax including string interpolation in multiple string forms (quotes, regex, subprocess).
