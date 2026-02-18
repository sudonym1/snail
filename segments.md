# Fix per-segment auto-print for `-b` and `-e`

## Context

After commit 7382700 (unify awk/map modes), `-b` and `-e` segments lost independent auto-printing. `wrap_source()` joins all segments with `\n` into a single source string, compiled as one program. `lower_block_with_tail()` only auto-prints the final expression of the entire combined program. Each segment's last bare expression should auto-print independently.

## Approach: AST-level Segment Breaks via `\x1f`

Introduce `Stmt::SegmentBreak` as an AST marker. Use `\x1f` (ASCII Unit Separator) between segments in the wrapped source. The parser emits `SegmentBreak` nodes, and the lowering applies tail behavior (auto-print) to expressions preceding each break AND to the final expression.

## Changes

### 1. AST: Add `SegmentBreak` variant
**File:** `crates/snail-ast/src/ast.rs` (after `PatternAction`, ~line 127)

Add `SegmentBreak { span: SourceSpan }` to the `Stmt` enum.

### 2. Grammar: Add `segment_break` rule
**File:** `crates/snail-parser/src/snail.pest`

- Add rule after `stmt_sep` (line 10): `segment_break = { "\x1F" ~ stmt_sep* }`
- Change `stmt_list` (line 7): `stmt_list = { (stmt_item | segment_break)* ~ final_stmt? }`

### 3. Parser: Emit `SegmentBreak` nodes
**File:** `crates/snail-parser/src/stmt.rs` — `parse_stmt_list()` (lines 12-18)

Check `inner.as_rule() == Rule::segment_break` and push `Stmt::SegmentBreak { span }`.

### 4. `wrap_source()`: Use `\x1f` separator
**File:** `crates/snail-python/src/lib.rs` — `wrap_source()` (lines 300-352)

Change `parts.join("\n")` to `parts.join("\n\x1f")` in all three modes (snail, awk, map). The `\n` triggers preprocessor `\x1e` injection; the `\x1f` produces `SegmentBreak` in the AST.

### 5. Lowering: Apply tail behavior at segment boundaries
**File:** `crates/snail-python/src/lower/stmt.rs` — `lower_block_with_tail()` (lines 386-478)

- Skip `SegmentBreak` nodes (`continue`)
- Change the tail-application guard from `if is_last` to `if is_last || next_is_break` where `next_is_break = matches!(block.get(idx + 1), Some(Stmt::SegmentBreak { .. }))`
- Add `Stmt::SegmentBreak` to `lower_stmt()` (~line 347) as an error case like `PatternAction`

### 6. Exhaustive match updates (add `SegmentBreak` arm)
- `crates/snail-parser/src/lib.rs` — `validate_stmt_mode()` (~line 171): add to the `Break | Continue | Pass | Import | ImportFrom` no-op arm
- `crates/snail-python/src/lower/desugar.rs` — `desugar_stmt()` (~line 244): passthrough `SegmentBreak { span } => SegmentBreak { span: span.clone() }`
- `crates/snail-python/src/lower/validate.rs` — `check_stmt()` (~line 136): add to the `Break | Continue | Pass | Import | ImportFrom` no-op arm

### 7. Fix `has_tail_expression()`
**File:** `crates/snail-python/src/lib.rs` (lines 137-149)

Skip trailing `SegmentBreak` nodes when finding the last statement — use `.iter().rev().find(|s| !matches!(s, Stmt::SegmentBreak { .. }))`.

### 8. Tests
- **Parser test** (`crates/snail-parser/tests/parser.rs`): source with `\x1f` produces `SegmentBreak` nodes
- **Python CLI tests** (`python/tests/test_cli.py`): `snail -b x=1 -b x '10' -e x -e x` prints expected output; single-segment behavior unchanged; semicolon-terminated expressions before breaks are NOT auto-printed

## Verification
```bash
cargo test          # parser + lowering tests
uv run -- python -m pytest python/tests  # CLI tests
make test           # full CI
# Manual check:
uv run -- snail -b 'x=1' -b x -b x '10' -e x -e x
```
