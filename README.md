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
or by starting a file with `#!/usr/bin/env -S snail --awk -f`. Awk sources are
written as pattern/action pairs evaluated for each input line. `BEGIN` and
`END` blocks run before and after the line loop, a lone pattern defaults to
printing matching lines, and a bare block runs for every line. Built-in
variables mirror awk but use short `$`-prefixed names: the current line as
`$l`, whitespace-split fields as `$f`, counters `$n` and `$fn` for global and
per-file line numbers, the current file path as `$p`, and `$m` for the last
regex match. These `$` names are injected by Snail itself; user-defined
identifiers cannot start with `$`.

The compiler/transpiler will generate Python source and execute it with the
Python interpreter. The implementation language is still open and should be
chosen based on parser ergonomics, ease of AST manipulation, and maintenance
cost.

Editor and shell integration

A comprehensive Vim/Neovim plugin is available in `extras/vim/` providing:
- Syntax highlighting for all Snail constructs
- Code formatting (`:SnailFormat`)
- Commands to run Snail code (`:SnailRun`) and view generated Python (`:SnailShowPython`)
- Filetype detection, indentation, and folding
- Tree-sitter grammar for Neovim (`extras/tree-sitter-snail/`)

Installation with vim-plug: `Plug 'sudonym1/snail', { 'rtp': 'extras/vim' }`

For manual installation, copy `extras/vim/` to `~/.vim/` (Vim) or
`~/.config/nvim/` (Neovim). See `extras/vim/README.md` for full details.

Development notes

- Snail uses pyo3 to execute generated Python code. A usable CPython must be on
  `PATH`. Set `PYO3_PYTHON=python3` (as CI does) if multiple Python versions are
  installed.

Project planning

The detailed project roadmap and development phases are documented in
`docs/PLANNING.md`.
