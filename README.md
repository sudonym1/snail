Snail is a new programming language that compiles to Python. The goal is to
keep Python's core semantics and runtime model while offering a syntax that
feels closer to Perl or awk for quick, incremental one-liners. It should be
comfortable for small text-processing tasks that grow into scripts, without
becoming whitespace sensitive.

Snail aims to:
- Preserve Python's behavior for data types, control flow, and evaluation.
- Provide concise syntax for one-liners and pipelines, inspired by Perl and awk.
- Favor terse, script-friendly syntax without introducing whitespace coupling.

The compiler/transpiler will generate Python source and execute it with the
Python interpreter. The implementation language is still open and should be
chosen based on parser ergonomics, ease of AST manipulation, and maintenance
cost.

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
- [ ] Provide a formatter or linter (optional, if syntax stabilizes).

Phase 5: Add all major python semantics
- [x] support for basic expressions
- [x] Support for basic flow control, classes, functions, etc.
- [x] Support for comprehensions
- [x] advanced support for strings
- [x] support for exceptions
- [x] Add `with` statements and context manager support.
- [x] Add `assert` and `del` statements.
- [ ] Add tuple and set literals plus slicing (`a[b:c]`, `a[:c]`, `a[b:]`).
- [ ] Add default parameters, `*args`, and `**kwargs`.
- [ ] Add `for`/`while` `else` blocks and `break`/`continue` in `try`.
- [ ] Add pattern matching (`match`/`case`) if keeping parity with Python 3.10+.
- [ ] Add support for if-expressions. e.g. `foo = x if y else z`

Phase 6: Snail Specific semantics
- [ ] Add first-class syntax for subprocess calls using `$(<command>)` and `@(<command>)`.
  The `<command>` body is treated as an implicit f-string (no quotes required), so
  `$(echo {name})` is valid. `$(<command>)` captures stdout and returns a string,
  raising on non-zero exit. `@(<command>)` does not capure output, but still
  raises an exception when the command fails. `@(<command>)` returns 0 on
  success.
  Both expand into expressions (not statements); complex cases should use Python's `subprocess`.
- [ ] Add compact exception swallowing expression: `<expr>?` would result in
  `None` in the case where `<expr>` raises an Exception. `<expr> ? <fallback
  expr>` would result in the fallback expression getting evaluated in the case
  the first expression fails.
- [ ] Classes can define the `__fallback__(self, exc)` method which, if present, will be
  invoked by the `?` operator in the casee where no explicit fallback
  expression is defined. For example the class that implements `@(command)`
  could implement `__fallback__` to return the CalledProcessError.returncode.

Phase 7: Interop and runtime features
- [ ] Ensure Snail functions/classes are normal Python callables.
- [ ] Handle globals/locals and module namespaces correctly.
- [ ] Define the standard library boundary and any Snail-specific helpers.
- [ ] Add integration tests that mix Snail and Python modules.

Phase 8: Performance and polish
- [ ] Cache compiled modules and improve incremental import speed.
- [ ] Optimize hot paths in parsing/lowering.
- [ ] Expand documentation, examples, and language reference.

Phase 0 decisions (executed)
- Implementation language: Rust (2024 edition).
- Target CPython: 3.12 initially, with a goal to keep 3.11+ compatible.
- Execution model: Snail -> Python AST -> compile() -> exec() in CPython.
- Interop: Python import hook for `import foo.snail`, Snail lowers to Python
  import nodes for direct Python module access.
- Tooling: Cargo with rustfmt and clippy; GitHub Actions CI.
- Layout: see `docs/DECISIONS.md` for details.
