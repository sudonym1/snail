# Release notes (v0.6.0 → v0.7.0)

## Highlights
- New syntax for dicts and sets: dict literals are now `%{...}` and set literals are `#{...}`.
- Anonymous functions are expressed as `def` blocks, and functions implicitly return the last non-semicolon expression.
- Generators are supported (`yield` / `yield from`) and f-strings now accept conversions and format specs.
- Awk/map begin/end blocks can live in source (`BEGIN { ... }`, `END { ... }`) and CLI `--begin/--end` works for both.
- Map mode now opens files lazily (errors only when `$fd`/`$text` are accessed).

## Breaking / Behavior Changes
- Dict literal syntax changed to `%{...}` and dict comprehensions use `%{k: v ...}`; `{...}` is always a block.
- `BEGIN` and `END` are reserved keywords in all modes (not valid identifiers).
- Compact try (`?`) is rejected on binding expressions.

## Language & Runtime
- Implicit returns: functions return the last non-semicolon expression by default; top-level auto‑print mirrors this (disable with `-P/--no-print`).
- Anonymous function expressions use `def` blocks (`def { ... }`, `def(x) { ... }`).
- Generator support with `yield` and `yield from` as expressions inside function bodies.
- Augmented assignment operators (`+=`, `-=`, `*=`, `/=`, `//=`, `%=` , `**=`) plus pre/post `++` and `--` on names, attributes, and indexes.
- String interpolation now supports conversions (`!r`, `!s`, `!a`) and format specs (`:spec`, including nested expressions).
- Import syntax expanded: parenthesized `from` imports, `from x import *`, and relative imports.
- Yield validation tightened for f-strings and set literals.

## Awk / Map Mode & CLI
- `BEGIN`/`END` blocks can be authored in source for awk and map modes; CLI `--begin/--end` runs before/after in-file blocks.
- `-b/--begin` and `-e/--end` flags are accepted after positional args.
- New `--debug-snail-ast` flag to print the Snail AST without executing.

## Tooling & Internals
- Lowering consolidated under `snail-python`; internal crates reorganized.
- Added linecache integration for better tracebacks and profiling hooks.
- Removed the proptest crate; grammar definitions updated (tree-sitter and Vim).

## Docs & Examples
- Language reference and README updated with map mode, dict/set literals, implicit returns, f-string specs, and new import forms.
- Examples expanded (including `examples/map.snail` and updated `examples/all_syntax.snail`).
