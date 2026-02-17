# Unify Modes: `lines` and `files` as First-Class Loop Blocks

## Context

Snail currently has three separate execution modes (regular, awk, map) with distinct AST types, parsers, and lowering paths. The goal is to make `lines` and `files` first-class compound statements in regular Snail, so awk and map modes become convenience shortcuts rather than entirely separate syntaxes. This lets users compose line-oriented and file-oriented processing within a single script.

After this change:
- `snail --awk -f script.snail` is sugar for wrapping the script body in `lines { }`
- `snail --map -f script.snail` is sugar for wrapping the script body in `files { }`
- Both can be used inline in any regular Snail script
- BEGIN/END blocks are dropped (just put code before/after the loop block)
- Pattern/action rules are only valid inside `lines { }` blocks

## Examples

```snail
# Inline awk-style processing in a regular script
results = []
lines("server.log") {
    /ERROR (\d+)/ { results.append($m.1) }
}
print(f"Found {len(results)} errors")

# File-oriented processing
files(glob("src/*.py")) {
    print($src, len($text), "bytes")
}

# Composable nesting
files(glob("logs/*.log")) {
    print(f"=== {$src} ===")
    lines($fd) {
        /TODO/ { print(f"  line {$fn}: {$0}") }
    }
}

# Bare lines { } defaults to argv/stdin (same as --awk today)
lines {
    /pattern/ { print($0) }
}
```

## Phase 1: AST Changes

**File:** `crates/snail-ast/src/ast.rs`

Add three new variants to `Stmt`:

```rust
Lines {
    source: Option<Expr>,   // None = argv/stdin default
    body: Vec<Stmt>,        // can contain PatternAction stmts
    span: SourceSpan,
},
Files {
    source: Option<Expr>,   // None = argv paths default
    body: Vec<Stmt>,
    span: SourceSpan,
},
PatternAction {
    pattern: Option<Expr>,      // None = bare block (runs every line)
    action: Option<Vec<Stmt>>,  // None = bare pattern (prints matching lines)
    span: SourceSpan,
},
```

`PatternAction` mirrors the existing `AwkRule` struct (`crates/snail-ast/src/awk.rs:19-23`) but lives as a `Stmt` variant. Keep `AwkProgram`/`AwkRule` untouched for now so existing awk mode still compiles.

**Verify:** `cargo build`

## Phase 2: Grammar + Preprocessor

### Grammar — `crates/snail-parser/src/snail.pest`

Add `lines` and `files` to the keyword list (line 298-305):
```pest
keyword = _{
  (... | "lines" | "files"
  | "True" | "False" | "None") ~ !ident_continue
}
```

Add to `compound_stmt` (line 38):
```pest
compound_stmt = _{ if_stmt | while_stmt | for_stmt | def_stmt | class_stmt
                 | try_stmt | with_stmt | lines_stmt | files_stmt }
```

New rules:
```pest
// Lines block: iterate lines from a source
lines_stmt = { "lines" ~ lines_source? ~ "{" ~ stmt_sep* ~ lines_body? ~ stmt_sep* ~ "}" }
lines_source = { "(" ~ expr ~ ")" }
lines_body = { lines_entry ~ (stmt_sep* ~ lines_entry)* ~ stmt_sep* }
lines_entry = _{ pattern_action | stmt }

// Pattern/action (same shape as awk_rule on line 18)
pattern_action = { block | pattern_action_pattern ~ block? }
pattern_action_pattern = { expr }

// Files block: iterate files from a source
files_stmt = { "files" ~ files_source? ~ block }
files_source = { "(" ~ expr ~ ")" }
```

Note: `lines_stmt` uses an inline block definition (not the `block` rule) because its body allows `pattern_action` entries mixed with regular statements. `files_stmt` uses the standard `block` rule.

### Preprocessor — `crates/snail-parser/src/preprocess.rs`

Add to `classify_keyword` (line 581-607) as header-triggering continuations so the preprocessor doesn't inject a statement separator between `lines(...)` and `{`:
```rust
b"lines" | b"files" => (LastToken::Continuation, true),
```

**Verify:** `cargo build` (grammar compiles), then parser tests in Phase 3

## Phase 3: Parser

### Statement parsing — `crates/snail-parser/src/stmt.rs`

Add `Rule::lines_stmt` and `Rule::files_stmt` cases to `parse_stmt` (line 20-51). Add helper functions:
- `parse_lines_stmt` — extracts optional source expr, parses body entries distinguishing `Rule::pattern_action` from `Rule::stmt` variants, builds `Stmt::Lines`
- `parse_files_stmt` — extracts optional source expr, parses block body, builds `Stmt::Files`
- `parse_pattern_action` — mirrors existing awk rule parsing from `crates/snail-parser/src/awk.rs`, builds `Stmt::PatternAction`

### Validation — `crates/snail-parser/src/lib.rs`

Expand `ValidationMode` (line 285-289) to be context-aware:

```rust
enum ValidationMode {
    Main,           // no $-variables allowed
    Map,            // $src, $fd, $text allowed
    Lines,          // $0, $n, $fn, $f, $m, $src allowed (same as awk)
    Files,          // $src, $fd, $text allowed (same as map)
}
```

Update `validate_stmt_mode` (line 306) to handle the new variants:
- `Stmt::Lines { source, body, .. }` — validate `source` with current mode, validate `body` with `ValidationMode::Lines`
- `Stmt::Files { source, body, .. }` — validate `source` with current mode, validate `body` with `ValidationMode::Files`
- `Stmt::PatternAction { .. }` — only valid when mode is `Lines`; error otherwise

This naturally handles nesting: `files { lines($fd) { } }` validates the outer body as `Files`, then the inner body as `Lines`.

**Verify:** `cargo test parser` — add tests in `crates/snail-parser/tests/parser.rs`:
- `lines { print($0) }` parses
- `lines("file.txt") { /pat/ { action } }` parses with pattern/action
- `files { print($src) }` parses
- `$0` outside `lines { }` is rejected
- `$fd` inside `lines { }` without enclosing `files { }` is rejected

## Phase 4: Lowering

### Statement lowering — `crates/snail-python/src/lower/stmt.rs`

Add `Stmt::Lines`, `Stmt::Files`, `Stmt::PatternAction` match arms to `lower_stmt` (line 22). Delegate to new helper functions.

### Lines lowering — refactor from `crates/snail-python/src/lower/awk.rs`

The existing `lower_awk_file_loop` and `lower_awk_line_loop` generate exactly the Python code needed for `Stmt::Lines`. Refactor to extract a reusable `lower_lines_loop(builder, source, body, span)` that:

- **No source (`lines { }`):** generates the argv/stdin file loop (same as awk today)
- **With source (`lines(expr) { }`):** dispatches based on type at runtime via a helper:
  - String → open as file path
  - File-like → iterate lines directly
  - Iterable → iterate directly

Pattern/action rules in the body are lowered using the existing `lower_awk_rules` logic from `awk.rs`.

### Files lowering — refactor from `crates/snail-python/src/lower/map.rs`

The existing `lower_map_file_loop` generates the Python code for `Stmt::Files`. Extract a reusable `lower_files_loop(builder, source, body, span)` that:

- **No source (`files { }`):** iterates argv paths (same as map today)
- **With source (`files(expr) { }`):** iterates the expression directly

### Desugar pass — `crates/snail-python/src/lower/desugar.rs`

Add `Stmt::Lines`, `Stmt::Files`, `Stmt::PatternAction` cases to `desugar_stmt` — recurse into body/source/pattern/action to hoist lambdas.

### Validate pass — `crates/snail-python/src/lower/validate.rs`

Add `Stmt::Lines`, `Stmt::Files`, `Stmt::PatternAction` cases to `check_stmt` — recurse to check yield usage.

### Runtime helper — `python/snail/runtime/__init__.py`

Add `__snail_lines_iter(source)` for the `lines(expr)` case — handles str/file/iterable dispatch. Register in `install_helpers`.

### Nesting / variable scoping

Inner loops overwrite outer variable values. After the inner loop, the outer loop's next iteration re-sets its variables. This matches awk semantics (no block scoping). Document this behavior; add save/restore later if needed.

**Verify:** `cargo test` + `uv run -- python -m pytest python/tests` — add end-to-end tests:
- `snail 'lines { print($0) }' < input.txt` prints each line
- `snail 'lines("file.txt") { print($n, $0) }'` reads from file
- `snail 'files { print($src) }' file1.txt file2.txt` prints paths
- `snail 'lines { /pat/ { print($0) } }' < input.txt` pattern matching
- Nested `files/lines` test

## Phase 5: CLI Desugaring (follow-up)

Make `--awk` and `--map` desugar to the unified path. This can be a separate PR:
- `--awk` wraps source body in `lines { }`, extracts existing BEGIN/END to before/after
- `--map` wraps source body in `files { }`, extracts existing BEGIN/END to before/after
- Eventually deprecate `AwkProgram` and the separate parse/lower paths

## Phase 6: Docs + Examples

- Update `examples/all_syntax.snail` with `lines`/`files` examples
- Update `docs/REFERENCE.md`
- Update vim syntax highlighting in `extras/vim/`
- Update tree-sitter grammar in `extras/tree-sitter-snail/`

## Verification

After each phase, run `make test`. Final check before commit:
```bash
make test
```

## Files to Modify (Phases 1-4)

| File | Change |
|------|--------|
| `crates/snail-ast/src/ast.rs` | Add `Lines`, `Files`, `PatternAction` to `Stmt` |
| `crates/snail-parser/src/snail.pest` | Grammar rules + keywords |
| `crates/snail-parser/src/preprocess.rs` | `classify_keyword` for `lines`/`files` |
| `crates/snail-parser/src/stmt.rs` | Parse new statement types |
| `crates/snail-parser/src/lib.rs` | Context-aware validation |
| `crates/snail-python/src/lower/stmt.rs` | Lower new statement types |
| `crates/snail-python/src/lower/awk.rs` | Extract reusable `lower_lines_loop` |
| `crates/snail-python/src/lower/map.rs` | Extract reusable `lower_files_loop` |
| `crates/snail-python/src/lower/desugar.rs` | Handle new variants in lambda hoisting |
| `crates/snail-python/src/lower/validate.rs` | Handle new variants in yield checking |
| `python/snail/runtime/__init__.py` | Add `__snail_lines_iter` helper |
| `crates/snail-parser/tests/parser.rs` | Parser tests |
| `python/tests/test_cli.py` | End-to-end tests |
| `examples/all_syntax.snail` | Syntax examples |
