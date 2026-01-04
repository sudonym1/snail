<p align="center">
  <img src="logo.png" alt="Snail logo" width="200">
</p>

What do you get when you shove a snake in a shell?

Snail is a new programming language that compiles to Python. The goal is to
keep Python's core semantics and runtime model while offering a syntax that
feels closer to Perl or awk for quick, incremental one-liners. It should be
comfortable for small text-processing tasks that grow into scripts, without
becoming whitespace sensitive.

Snail aims to:
- Preserve Python's behavior for data types, control flow, and evaluation.
- Provide concise syntax for one-liners and pipelines, inspired by Perl and awk.
- Favor terse, script-friendly syntax without introducing whitespace coupling.

Documentation and examples live in `docs/REFERENCE.md`,
`examples/all_syntax.snail`, and `examples/awk.snail`. The reference walks
through the syntax surface and runtime behaviors, while the example files
provide runnable tours that mirror the language features. Both stay current as
phases are delivered.

Awk mode is available for line-oriented scripts. Enable it with `snail --awk`
or by starting a file with `#!/usr/bin/env snail --awk`. Awk sources are written as
pattern/action pairs evaluated for each input line. `BEGIN` and `END` blocks run
before and after the line loop, a lone pattern defaults to printing matching
lines, and a bare block runs for every line. Built-in variables mirror awk but
use short `$`-prefixed names: the current line as `$l`, whitespace-split fields
as `$f`, counters `$n` and `$fn` for global and per-file line numbers, the
current file path as `$p`, and `$m` for the last regex match. These `$` names
are injected by Snail itself; user-defined identifiers cannot start with `$`.

The compiler/transpiler will generate Python source and execute it with the
Python interpreter. The implementation language is still open and should be
chosen based on parser ergonomics, ease of AST manipulation, and maintenance
cost.

Editor and shell integration

- Vim syntax highlighting lives in `extras/vim/syntax/snail.vim`. Copy it into
  your Vim runtime (e.g., `~/.vim/syntax/`) and add a simple ftdetect rule such
  as `au BufRead,BufNewFile *.snail setfiletype snail`.
- Neovim uses the same syntax file; drop it into
  `~/.config/nvim/syntax/snail.vim` and add either the same ftdetect autocommand
  under `~/.config/nvim/ftdetect/snail.vim` or a Lua rule such as
  `vim.filetype.add({ extension = { snail = "snail" } })` in your config.

Development notes

- Python integration tests expect a usable CPython on `PATH`. Set `PYO3_PYTHON=
  python3` (as CI does) if multiple Python versions are installed.

Project plan

Phase 0: Project scaffold and decisions
- [x] Choose implementation language and target Python version.
- [x] Define the Snail surface syntax goals (short examples and non-goals).
- [x] Establish repository layout and build tooling.
- [x] Set up CI for linting, tests, and minimal integration checks.

Phase 1: Parser and AST
- [x] Use the initial "Python with curly braces" grammar as-is.
- [x] Build a Snail AST with source spans (line/column).
- [x] Add error reporting with friendly messages and snippets.
- [x] Create fixture tests for parsing and error cases.
- [x] Own f-string parsing for all string-like forms (`"..."`, `r"..."`, regex `/.../`, subprocess `$(...)`, `@(...)`) to support `{expr}` interpolation consistently.

Phase 2: Lowering to Python AST
- [x] Map Snail AST nodes to Python AST nodes.
- [x] Preserve source locations for accurate tracebacks.
- [x] Validate round-trips for small programs (Snail -> Python AST -> exec).
- [x] Add golden tests for Python AST output.
- [x] Defer new syntax features until core pipeline is working.

Phase 3: CPython integration
- [x] Implement a Python extension module (Rust + pyo3).
- [x] Provide a module API for compiling and executing Snail code.
- [x] Add a Python import hook so `import foo.snail` works.
- [x] Ensure Snail code can import Python modules directly.

Phase 4: CLI and tooling
- [x] Build a `snail` CLI for running files and one-liners.
- [x] Add error formatting suitable for terminal output.

Phase 5: Add all major python semantics
- [x] support for basic expressions
- [x] Support for basic flow control, classes, functions, etc.
- [x] Support for comprehensions
- [x] advanced support for strings
- [x] support for exceptions
- [x] Add `with` statements and context manager support.
- [x] Add `assert` and `del` statements.
- [x] Add tuple and set literals plus slicing (`a[b:c]`, `a[:c]`, `a[b:]`).
- [x] Add default parameters, `*args`, and `**kwargs`.
- [x] Add `for`/`while` `else` blocks and `break`/`continue` in `try`.
- [x] Add support for if-expressions. e.g. `foo = x if y else z`

Phase 6: Interop and runtime features
- [x] Ensure Snail functions/classes are normal Python callables.
- [x] Handle globals/locals and module namespaces correctly.
- [x] Define the standard library boundary and any Snail-specific helpers.
- [x] Add integration tests that mix Snail and Python modules.

Phase 7: Snail Specific semantics
- [x] Add compact exception swallowing expression: `<expr>?` yields the
  exception object when `<expr>` raises. `<expr> ? <fallback expr>` evaluates
  the fallback when `<expr>` raises; the exception object is available as
  `$e`. Example: `value = risky()?`, `fallback = risky() ? $e`.
- [x] Add first-class syntax for subprocess calls using `$(<command>)` and `@(<command>)`.
  The `<command>` body is treated as an implicit f-string (no quotes required), so
  `$(echo {name})` is valid. `$(<command>)` captures stdout and returns a string,
  raising on non-zero exit. `@(<command>)` does not capure output, but still
  raises an exception when the command fails. `@(<command>)` returns 0 on
  success. These are regular python subprocesses, but when they throw a
  CalledProcessError, that error is intercepted and a __fallback__ method is
  injected, which in the case of the `@()` form returns the exception return
  code. In the case of the `$()` form, the __fallback__ method re-raises the
  exception forcing users to provide a fallback value.
  Both expand into expressions (not statements); complex cases should use Python's `subprocess` directly.
- [x] add a regex expression. `string in /<pattern>/` performes an re.search and returns any match object.
- [x] Add compound expressions `(expr1; expr2; exprN)` that evaluate to the final
  expression, enabling chained usage with the `?` operator.
  - [x] Extend the grammar to parse semicolon-delimited expression groups inside
    parentheses and ensure precedence/associativity integrate with existing
    expressions.
  - [x] Update AST nodes and lowering to produce the correct Python expression
    sequence (e.g., using tuples or blocks) while returning the last value.
  - [x] Add parser and lowering tests plus examples in
    `examples/all_syntax.snail` and `docs/REFERENCE.md` demonstrating `?`
    interplay.

Follow-up work on `?` operator precedence
- [x] Tighten precedence so postfix `?` binds to the immediately preceding
  expression before other infix operators or trailing accessors.
- [x] Confirm the fallback stops before following infix operators unless
  parentheses are used (e.g., `a?0 + 1` parses as `(a?0) + 1`).
- [x] Add grammar tests that cover combinations like `a + b?`, `call()?`, and
  `value? + other` to lock in left-binding behavior.
- [x] Update the parser and lowering (e.g., `src/snail.pest` and expression
  lowering) to match the new precedence rules.
- [x] Refresh documentation and examples once the binding changes land to show
  the expected parse.

Phase 8: Documentation and utilities
- [x] Expand documentation, examples, and language reference.
- [ ] Provide useful utilities to help users adopt Snail.
  - [x] Syntax highlighting for Vim.
  - [ ] Easy installation path (PyPI package and/or Homebrew formula).

Phase 9: Awk-style line processing
- [x] Add an awk mode that evaluates pattern/action pairs across input lines.
- [x] Provide syntactic sugar for common awk idioms (e.g., default actions, begin/end hooks).
- [x] Surface a clear entry point for enabling awk mode (CLI flag or file directive) and document usage.
- [x] add support for the regex expression as a pattern. if no string is provided `$l` is implicit. just the pattern is valid. the match object should be made available to the action.
- [x] Support Snail `{expr}` interpolation in string literals for awk-mode variables (e.g., `{print("{$1}")}`).
  - [x] Inspect current string parsing/lowering to see where interpolation is lost or rejected.
  - [x] Define supported `{expr}` interpolation (including awk `$` vars and escaping rules), then update parser/lowering and add tests.
  - [x] Validate end-to-end with `{print("{$1}")}` and refresh docs/examples.

Phase 10: Pipeline operator and first-class JSON support with JMESPath
- [ ] Repurpose bitwise operators for Snail-specific semantics.
  - [ ] Reserve `|`, `<<`, `>>`, `&`, `^`, `~` operators - remove from Python compatibility.
  - [ ] Update grammar in `src/snail.pest` to parse these operators but make `<<`, `>>`, `&`, `^`, `~` compilation errors for now (reserved for future use).
  - [ ] Implement `|` as the pipeline operator with proper precedence (lower than arithmetic/comparison, higher than boolean ops).
- [ ] Implement generic pipeline operator `|` using `__pipeline__` dunder method.
  - [ ] Define `x | y` to lower to `y.__pipeline__(x)` in generated Python code.
  - [ ] This allows any object to define how it consumes pipeline input by implementing `__pipeline__(self, input)`.
  - [ ] Update AST in `src/ast.rs` to represent pipeline expressions (binary operator with left/right operands).
  - [ ] Update lowering in `src/lower.rs` to generate `__pipeline__` method calls for pipeline expressions.
- [ ] Add `@j(<JMESPath expression>)` syntax for JSON querying with four forms:
  - [ ] `@j(query)` - read JSON from stdin and apply JMESPath query.
  - [ ] `<file-like object> | @j(query)` - read JSON from file/file-like object via pipeline.
  - [ ] `<JSON-native object> | @j(query)` - query Python dicts/lists directly via pipeline.
  - [ ] `@j(query) | @j(query)` - chain JMESPath queries via pipeline.
  - [ ] Extend grammar to recognize `@j(...)` as special expression form (similar to `@(...)` subprocess syntax).
  - [ ] Support `{expr}` interpolation within JMESPath expressions for dynamic queries.
- [ ] Implement JSON query lowering using `__pipeline__` pattern.
  - [ ] Generate `__SnailJsonQuery` helper class in lowered Python code with `__pipeline__(self, data)` method.
  - [ ] `@j(query)` lowers to `__SnailJsonQuery(query).__pipeline__(None)` (stdin case).
  - [ ] `x | @j(query)` lowers to `__SnailJsonQuery(query).__pipeline__(x)` via pipeline operator.
  - [ ] The `__pipeline__` implementation handles multiple input types:
    - `None` - read JSON from stdin
    - `str` - treat as file path, open and read JSON
    - File-like object (has `read()` method) - read and parse JSON
    - JSON-native types (dict, list, str, int, float, bool, None) - query directly
    - Other types - raise `TypeError` (only JSON-native types allowed)
  - [ ] Use Python's `jmespath` library to apply queries and return results.
  - [ ] Handle errors gracefully (invalid JSON, JMESPath syntax errors, file not found, non-JSON-native types).
- [ ] Add comprehensive tests for pipeline operator and JSON queries.
  - [ ] Parser tests in `tests/parser.rs`: validate `|` operator parsing, `@j(...)` syntax, precedence.
  - [ ] Lowering tests in `tests/lower.rs`: confirm `__pipeline__` calls generate correctly.
  - [ ] Integration tests in `tests/python_integration.rs`:
    - Test `@j(query)` reading from stdin.
    - Test `file | @j(query)` and `'path' | @j(query)` forms.
    - Test querying Python dicts/lists directly: `{'a': 1} | @j('a')`.
    - Test chained queries: `@j('users') | @j('[0].name')`.
    - Test error cases (malformed JSON, invalid JMESPath, wrong types, missing files).
    - Test that bitwise operators (`<<`, `>>`, etc.) raise compilation errors.
- [ ] Update documentation and examples.
  - [ ] Update `examples/all_syntax.snail` with pipeline operator examples and all four `@j(...)` forms.
  - [ ] Update `docs/REFERENCE.md` with:
    - Pipeline operator `|` documentation and `__pipeline__` dunder method pattern.
    - JSON/JMESPath syntax for all four forms with examples.
    - Note that bitwise operators are reserved/disabled.
    - Document dependency on Python's `jmespath` library and installation requirements (`pip install jmespath`).
