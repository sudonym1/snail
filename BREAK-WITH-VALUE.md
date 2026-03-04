# Break With Value

## Context

Loops (`for`, `while`) are already expressions in Snail — they can return their last iteration's tail value. This feature adds `break expr` so loops can exit early with a specific value, like Rust's `break` with value. Bare `break` (no value) yields `None`.

## Design

Since Python's `break` doesn't accept a value, we desugar `break expr` into `capture_var = expr; break` at the AST level. A shared utility (`rewrite_breaks_in_block`) walks loop bodies and rewrites all break statements, recursing into compound blocks (if, try, with, block) but NOT into nested for/while (they have their own break scope).

Two code paths need this:
1. **Desugarer** (expression context, e.g. `x = while { break 1 }`): uses `__snail_expr_N`
2. **Lowering** (tail position, e.g. last stmt in program/function): uses `__snail_last_result`

## Changes

### 1. AST — `crates/snail-ast/src/ast.rs`
Add optional value to Break:
```rust
Break {
    value: Option<Expr>,
    span: SourceSpan,
}
```

### 2. Grammar — `crates/snail-parser/src/snail.pest`
```pest
break_stmt = { "break" ~ expr? }
```
No preprocessor changes needed — `break` is already a `StmtEnder`, so `break` on its own line is `break;` and `break expr` on the same line works naturally.

### 3. Parser — `crates/snail-parser/src/stmt.rs`
Parse optional expression after `break`, following the `return_stmt` pattern:
```rust
Rule::break_stmt => {
    let span = span_from_pair(&pair, source);
    let value = pair.into_inner().next()
        .map(|p| parse_expr_pair(p, source)).transpose()?;
    Ok(Stmt::Break { value, span })
}
```

### 4. Break rewrite utility — `crates/snail-python/src/lower/break_rewrite.rs` (new)
```rust
pub(crate) fn rewrite_breaks_in_block(stmts: &mut Vec<Stmt>, capture_var: &str, span: &SourceSpan)
```
- `break expr` → `[capture_var = expr; break]`
- `break` (no value) → `[capture_var = None; break]`
- Recurses into: if/elif/else bodies, try/except/else/finally bodies, with bodies, block bodies
- Does NOT recurse into for/while bodies (nested loop = separate break scope)

Register in `crates/snail-python/src/lower/mod.rs`: `mod break_rewrite;`

### 5. Desugarer — `crates/snail-python/src/lower/desugar.rs`
- **`inline_compound_expr`**: For `Expr::For` and `Expr::While` arms, call `rewrite_breaks_in_block(&mut body, &tmp_name, span)` BEFORE `capture_branch_tail`
- **`capture_branch_tail`**: For `Expr::For` and `Expr::While` in `CaptureCompound`, also call `rewrite_breaks_in_block` before `capture_branch_tail` recursion (handles loops nested inside other captured compounds)
- **`desugar_stmt`**: Update `Stmt::Break` arm to desugar the optional value expression

### 6. Lowering — `crates/snail-python/src/lower/stmt.rs` and `expr.rs`
- **`lower_while_stmt`** (stmt.rs): When `tail != None`, clone body, call `rewrite_breaks_in_block(&mut rewritten, "__snail_last_result", span)`, then lower the rewritten body
- **`lower_while_let`** (stmt.rs): Same treatment when `tail != None`
- **`lower_for_stmt`** (expr.rs): Same treatment when `tail != None`
- **`lower_stmt`** (stmt.rs): Update Break match for `value: None` case (unchanged behavior)
- **`lower_stmt_to_stmts`** (stmt.rs): Add match arm for `Stmt::Break { value: Some(expr) }` — evaluate expr for side effects, then emit Break (handles non-capturing loop case)

### 7. Validation — `crates/snail-python/src/lower/validate.rs`
Update `Stmt::Break` match to validate the break value expression via `check_expr`.

### 8. Tests
**Parser tests** (`crates/snail-parser/tests/parser.rs`):
- `break` without value still parses
- `break expr` parses with correct AST
- `break` followed by newline doesn't consume next line

**CLI integration tests** (`python/tests/test_cli.py`):
- `x = while { break "found" }; assert x == "found"`
- `x = for i in range(5) { if i == 3 { break i } }; assert x == 3`
- `x = while { break }; assert x is None`
- Nested loops: inner break doesn't clobber outer loop value
- Break with loop else clause
- Break inside try/finally in a capturing loop

### 9. Examples and docs
- `examples/all_syntax.snail`: Add break-with-value examples
- `docs/REFERENCE.md`: Document `break expr` syntax

## Verification
```bash
cargo test parser     # parser tests
cargo test            # all Rust tests
make test             # full CI (fmt, clippy, cargo test, pytest)
```
Manual smoke test:
```bash
uv run -- snail 'x = while { break 42 }; print(x)'       # should print 42
uv run -- snail 'x = for i in range(10) { if i == 5 { break i } }; print(x)'  # should print 5
uv run -- snail 'x = while { break }; print(x)'           # should print None
```
