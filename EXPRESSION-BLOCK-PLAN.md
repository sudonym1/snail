# Generalize Compound Expression Block Values

## Context

Currently, only `if` and `block` expressions get tail behavior propagation (auto-print, capture, implicit return). All other compound expressions (`for`, `while`, `try`, `with`) fall through to `lower_expr_as_stmt()` with no tail propagation. The desugarer also excludes `for`/`while` from value capture (`NoCapture`). This means these expressions can't produce values in tail or expression contexts.

The goal is to make ALL compound expressions with blocks share the same lowering mechanism for:
- **AutoPrint**: Last expression in body gets printed (program tail)
- **CaptureOnly**: Last expression assigned to `__snail_last_result` (awk mode)
- **ImplicitReturn**: Last expression becomes the return value (function tail)

## Approach: Add `tail: TailBehavior` to existing lowering functions

Rather than creating duplicate `*_with_tail` functions, modify existing lowering functions to accept a `TailBehavior` parameter. When called from `lower_expr_as_stmt`, pass `TailBehavior::None` (preserving current behavior). When called from `lower_tail_expr`, pass the actual tail.

## Tail propagation rules

| Tail Behavior   | for/while body | for/while else | try body | except handlers | try else | try finally | with body |
|------------------|----------------|----------------|----------|-----------------|----------|-------------|-----------|
| AutoPrint        | YES            | YES            | YES      | YES             | YES      | NO          | YES       |
| CaptureOnly      | YES            | YES            | YES      | YES             | YES      | NO          | YES       |
| ImplicitReturn   | NO             | YES            | YES      | YES             | YES      | NO          | YES       |

**Key decision**: `ImplicitReturn` does NOT propagate into loop bodies. Propagating `return <expr>` into a loop body would cause the function to return after the first iteration, which is almost never the intended behavior. Instead, for loops at function tail, the else_body (if present) can produce the return value, and the loop body runs normally.

## Implementation Steps

### Step 1: Add `tail` parameter to lowering functions

**`crates/snail-python/src/lower/expr.rs`:**
- `lower_for_stmt()` (line 2063): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, body, tail, span)` (but `TailBehavior::None` if `tail == ImplicitReturn`)
  - Else: `lower_block_with_tail(builder, items, tail, span)`
- `lower_try_stmt()` (line 2143): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, body, tail, span)`
  - Else: `lower_block_with_tail(builder, items, tail, span)`
  - Finally: stays `lower_block()` (never propagates tail)
  - Handlers: pass `tail` to `lower_except_handler()`
- `lower_with_stmt()` (line 2181): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, body, tail, span)`

**`crates/snail-python/src/lower/stmt.rs`:**
- `lower_while_stmt()` (line 274): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, body, tail, span)` (but `TailBehavior::None` if `tail == ImplicitReturn`)
  - Else: `lower_block_with_tail(builder, items, tail, span)`
  - Pass `tail` through to `lower_while_let()`
- `lower_while_let()` (line 320): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, body, tail, span)` (same ImplicitReturn guard)
  - Else: `lower_block_with_tail(builder, items, tail, span)`
- `lower_except_handler()` (line 475): Add `tail: TailBehavior`
  - Body: `lower_block_with_tail(builder, &handler.body, tail, &handler.span)`

### Step 2: Update call sites to pass `TailBehavior::None`

In `lower_expr_as_stmt()` (expr.rs:2406-2474), update all calls:
- `lower_while_stmt(builder, cond, body, else_body, TailBehavior::None, span)`
- `lower_for_stmt(builder, target, iter, body, else_body, TailBehavior::None, span)`
- `lower_try_stmt(builder, body, handlers, else_body, finally_body, TailBehavior::None, span)`
- `lower_with_stmt(builder, items, body, TailBehavior::None, span)`

Also update `lower_try_stmt`'s call to `lower_except_handler` to pass `tail`.

### Step 3: Update `lower_tail_expr()` dispatch (expr.rs:2479-2575)

Replace the non-propagating block (lines 2541-2552) with proper dispatch:

```rust
match expr {
    Expr::While { cond, body, else_body, span } => {
        return lower_while_stmt(builder, cond, body, else_body, tail, span);
    }
    Expr::For { target, iter, body, else_body, span } => {
        return lower_for_stmt(builder, target, iter, body, else_body, tail, span);
    }
    Expr::Try { body, handlers, else_body, finally_body, span } => {
        return lower_try_stmt(builder, body, handlers, else_body, finally_body, tail, span);
    }
    Expr::With { items, body, span, .. } => {
        return lower_with_stmt(builder, items, body, tail, span);
    }
    // Def, Class still don't propagate
    Expr::Def { .. } | Expr::Class { .. } | Expr::Awk { .. } | Expr::Xargs { .. } => {
        return lower_expr_as_stmt(builder, expr, span);
    }
    _ => {}
}
```

### Step 4: Update desugarer (`crates/snail-python/src/lower/desugar.rs`)

**a) `capture_branch_tail()` (line 28):**
- Line 45: Change `Expr::While { .. } | Expr::For { .. } => Action::NoCapture` to `Action::CaptureCompound`
- Add cases in the `CaptureCompound` match arm for For and While:
  - Apply `capture_branch_tail` to body and else_body

**b) `inline_compound_expr()` (line 155):**
- Add For and While cases (currently fall through to "other" at line 247):
  - Apply `capture_branch_tail` to body and else_body
  - Push modified compound as statement to prelude

### Step 5: Tests and examples

**`examples/all_syntax.snail`:** Add compound expression value examples:
- `for_result = for x in [1,2,3] { x * 2 }` → 6 (last iteration)
- `while_result = ...` → last iteration value
- `try_result = try { ... } except { ... }` → branch value
- `with_result = with ... { ... }` → body value

**`python/tests/test_cli.py`:** Add integration tests for:
- Auto-print of for/while/try/with at program tail
- Implicit return from try/with in function bodies
- Expression-context capture: `x = for ...`, `x = while ...`, etc.
- Nested compound tails
- Empty iteration (zero iterations → None)

## Verification

1. `cargo test` — all Rust tests pass
2. `uv run -- python -m pytest python/tests` — all Python tests pass
3. `make test` — full CI check
4. Manual verification:
   - `snail 'for x in [1,2,3] { x * 2 }'` prints 2, 4, 6
   - `snail 'try { 1/0 } except { "error" }'` prints "error"
   - `snail 'with open("/dev/null") as f { "opened" }'` prints "opened"
