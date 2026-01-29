# snail-error

Error types and formatting utilities for the Snail compiler.

## Purpose

This crate provides unified error handling across all stages of Snail compilation. It defines error types for parsing and lowering failures, along with formatting utilities that produce user-friendly error messages with source location context.

## Key Components

- **ParseError**: Errors from the parsing stage (syntax errors, invalid tokens, etc.)
- **LowerError**: Errors from the lowering/transformation stage
- **SnailError**: Unified error enum wrapping both parse and lower errors
- **format_snail_error()**: Formats errors with file name and source context
- **format_parse_error()**: Specialized formatting for parse errors with line numbers and carets

## Dependencies

- **snail-ast**: Uses `SourceSpan` to provide precise error locations

## Used By

- **snail-parser**: Returns `ParseError` when parsing fails
- **snail-lower**: Returns `LowerError` when transformation fails
- **snail-python**: Uses `format_snail_error()` to display errors to users

## Error Format

Parse errors include:
- Error message describing the problem
- File name and line:column location
- The problematic line of source code
- A caret (^) pointing to the error location

Example:
```
error: unexpected token
--> script.snail:5:12
 |
5 | if x == {
 |            ^
```

## Design

All error types implement the standard `Error` trait and provide Display implementations. The `SourceSpan` integration enables precise error reporting that helps users quickly locate and fix issues in their Snail code.
