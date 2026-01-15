# Plan: Pipeline Placeholder Syntax (`_`)

## Feature Overview

Add placeholder syntax to control where piped values are inserted in function calls:

```snail
"foo" | greet(_, "!")      # greet("foo", "!")  - placeholder marks insertion point
"foo" | greet("Hi", _)     # greet("Hi", "foo") - piped value as second arg
"foo" | greet()            # greet("foo")       - no placeholder = prepend (backward compat)
```

## Design Decisions

1. **No placeholder in call**: Prepend piped value (backward compatible)
2. **Multiple placeholders**: Error - only one `_` allowed per call
3. **`_` outside pipeline**: Error - `_` is only valid in pipeline RHS call arguments

## Implementation Steps

### Step 1: Add `_` to grammar

**File:** `crates/snail-parser/src/snail.pest`

Add placeholder rule and include in atom (before identifier to prevent `_` being parsed as identifier):

```pest
# Add new rule
placeholder = { "_" ~ !ident_continue }

# Modify atom to include placeholder before identifier
atom = _{
  literal | regex | subprocess | structured_accessor
  | exception_var | field_index_var | injected_var
  | placeholder  # <-- ADD before identifier
  | identifier | list_comp | list_literal | dict_comp
  | dict_literal | set_literal | tuple_literal
  | compound_expr | "(" ~ expr ~ ")"
}
```

### Step 2: Add Placeholder to AST

**File:** `crates/snail-ast/src/ast.rs`

Add new variant to `Expr` enum:

```rust
Placeholder {
    span: SourceSpan,
},
```

### Step 3: Parse placeholder in parser

**File:** `crates/snail-parser/src/expr.rs`

In `parse_atom()`, add handling for the new rule:

```rust
Rule::placeholder => Ok(Expr::Placeholder {
    span: pair_to_span(pair, source),
}),
```

Also update any exhaustive match statements in:
- `crates/snail-parser/src/lib.rs`
- `crates/snail-parser/src/util.rs`
- `crates/snail-parser/src/string.rs`

### Step 4: Update lowering for pipelines with placeholders

**File:** `crates/snail-lower/src/expr.rs`

Modify pipeline lowering to detect and substitute placeholders:

```rust
if *op == BinaryOp::Pipeline {
    let left_expr = lower_expr_with_exception(builder, left, exception_name)?;

    // Check if RHS is a call with placeholder(s)
    match right.as_ref() {
        Expr::Call { func, args, kwargs, span: call_span } => {
            // Count placeholders in args
            let placeholder_count = count_placeholders(args);

            if placeholder_count > 1 {
                return Err(LowerError::MultiplePlaceholders { span: *call_span });
            }

            if placeholder_count == 1 {
                // Substitute placeholder with piped value
                let new_args = substitute_placeholder(args, &left_expr);
                // Lower the call with substituted args
                lower_call(builder, func, &new_args, kwargs, call_span, exception_name)
            } else {
                // No placeholder - prepend piped value (current behavior)
                let func_expr = lower_expr_with_exception(builder, func, exception_name)?;
                let mut call_args = vec![left_expr];
                // ... add rest of args
                lower_call_with_prepended_arg(...)
            }
        }
        Expr::Subprocess { ... } => {
            // Existing subprocess handling
        }
        _ => {
            // RHS is a callable - call it with piped value
            let right_obj = lower_expr_with_exception(builder, right, exception_name)?;
            builder.call_node("Call", vec![right_obj, vec![left_expr], ...])
        }
    }
}
```

### Step 5: Error on placeholder outside pipeline

**File:** `crates/snail-lower/src/expr.rs`

Add match arm for standalone Placeholder that returns an error:

```rust
Expr::Placeholder { span } => {
    Err(LowerError::PlaceholderOutsidePipeline { span: *span })
}
```

### Step 6: Add new error types

**File:** `crates/snail-error/src/lib.rs`

Add new error variants:

```rust
pub enum LowerError {
    // ... existing variants
    PlaceholderOutsidePipeline { span: SourceSpan },
    MultiplePlaceholders { span: SourceSpan },
}
```

### Step 7: Update examples and tests

**File:** `examples/all_syntax.snail`

```snail
# Pipeline placeholder syntax
def greet(name, suffix) { return "Hello {name}{suffix}" }
result = "World" | greet(_, "!")
assert result == "Hello World!"

result2 = "!" | greet("World", _)
assert result2 == "Hello World!"
```

**Tests to add:**
- Parser tests for `_` in various positions
- Lowering tests for placeholder substitution
- Error tests for multiple placeholders
- Error tests for placeholder outside pipeline

## Files to Modify

| File | Change |
|------|--------|
| `crates/snail-parser/src/snail.pest` | Add `placeholder` rule, update `atom` |
| `crates/snail-ast/src/ast.rs` | Add `Placeholder` variant to `Expr` |
| `crates/snail-parser/src/expr.rs` | Parse placeholder in `parse_atom()` |
| `crates/snail-parser/src/lib.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-parser/src/util.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-parser/src/string.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-lower/src/expr.rs` | Placeholder substitution in pipeline, error on standalone |
| `crates/snail-error/src/lib.rs` | Add error types |
| `examples/all_syntax.snail` | Add examples |

## Verification

1. `cargo build` - Rust compiles
2. `cargo test` - All tests pass
3. `uv run -- python -m maturin develop` - Extension builds
4. `uv run -- snail 'def f(a,b){a+b}; "x" | f(_, "y")'` - Returns "xy"
5. `uv run -- snail 'def f(a,b){a+b}; "y" | f("x", _)'` - Returns "xy"
6. `uv run -- snail 'def f(a,b){a+b}; "x" | f("y")'` - Returns "xy" (prepend)
7. `uv run -- snail '_ = 5'` - Error: placeholder outside pipeline
8. `uv run -- snail '"x" | f(_, _)'` - Error: multiple placeholders
9. `make test` - Full CI passes
