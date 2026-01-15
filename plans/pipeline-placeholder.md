# Plan: Pipeline Placeholder Syntax (`_`)

## Feature Overview

Add placeholder syntax to control where piped values are inserted in function calls:

```snail
"foo" | greet(_, "!")          # greet("foo", "!")       - placeholder marks insertion point
"foo" | greet("Hi", _)         # greet("Hi", "foo")      - piped value as second arg
"foo" | greet("Hi", suffix=_)  # greet("Hi", suffix="foo")
"foo" | identity                # identity("foo")         - callable with single param
```

## Design Decisions

1. **No placeholder in call**: Error - explicit placeholder required in call args
2. **Multiple placeholders**: Error - only one `_` allowed per call (positional or kwarg)
3. **Placeholder scope**: `_` is only a placeholder inside pipeline RHS call arguments
4. **`_` elsewhere**: `_` remains a normal identifier outside pipeline call args

## Implementation Steps

### Step 1: Add `_` to grammar

**File:** `crates/snail-parser/src/snail.pest`

Add placeholder rule and include in atom (before identifier so `_` can be parsed distinctly):

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

Modify pipeline lowering to detect and substitute placeholders (positional args and kwarg values):

```rust
if *op == BinaryOp::Pipeline {
    let left_expr = lower_expr_with_exception(builder, left, exception_name)?;

    // Check if RHS is a call with placeholder(s)
    match right.as_ref() {
        Expr::Call { func, args, kwargs, span: call_span } => {
            // Count placeholders in positional args and kwarg values
            let placeholder_count = count_placeholders(args, kwargs);

            if placeholder_count > 1 {
                return Err(LowerError::MultiplePlaceholders { span: offending_placeholder_span });
            }

            if placeholder_count == 1 {
                // Substitute placeholder with piped value
                let (new_args, new_kwargs) = substitute_placeholder(args, kwargs, &left_expr);
                // Lower the call with substituted args
                lower_call(builder, func, &new_args, &new_kwargs, call_span, exception_name)
            } else {
                // No placeholder - explicit placeholder required for call args
                return Err(LowerError::MissingPipelinePlaceholder { span: *call_span });
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

### Step 5: Lower placeholder outside pipeline as identifier

**File:** `crates/snail-lower/src/expr.rs`

Ensure a standalone placeholder lowers to the identifier `_` when not in a pipeline RHS call argument:

```rust
Expr::Placeholder { span } => {
    lower_identifier(builder, "_", *span)
}
```

### Step 6: Add new error types

**File:** `crates/snail-error/src/lib.rs`

Add new error variants:

```rust
pub enum LowerError {
    // ... existing variants
    MultiplePlaceholders { span: SourceSpan },
    MissingPipelinePlaceholder { span: SourceSpan },
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

result3 = "World" | greet("Hello ", suffix=_)
assert result3 == "Hello World"
```

**Tests to add:**
- Parser tests for `_` as placeholder and `_` as identifier
- Lowering tests for positional and kwarg placeholder substitution
- Error test for multiple placeholders in a call
- Error test for call missing placeholder
- Integration tests for placeholder usage and `_` identifier outside pipelines

## Files to Modify

| File | Change |
|------|--------|
| `crates/snail-parser/src/snail.pest` | Add `placeholder` rule, update `atom` |
| `crates/snail-ast/src/ast.rs` | Add `Placeholder` variant to `Expr` |
| `crates/snail-parser/src/expr.rs` | Parse placeholder in `parse_atom()` |
| `crates/snail-parser/src/lib.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-parser/src/util.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-parser/src/string.rs` | Handle Placeholder in exhaustive matches |
| `crates/snail-lower/src/expr.rs` | Placeholder substitution in pipeline, lower `_` as identifier elsewhere |
| `crates/snail-error/src/lib.rs` | Add error type |
| `examples/all_syntax.snail` | Add examples |

## Verification

1. `cargo build` - Rust compiles
2. `cargo test` - All tests pass
3. `uv run -- python -m maturin develop` - Extension builds
4. `uv run -- snail 'def f(a,b){a+b}; "x" | f(_, "y")'` - Returns "xy"
5. `uv run -- snail 'def f(a,b){a+b}; "y" | f("x", _)'` - Returns "xy"
6. `uv run -- snail 'def id(x){x}; "x" | id'` - Returns "x" (single-arg call)
7. `uv run -- snail 'def f(a,b){a+b}; "x" | f(a="y", b=_)'` - Returns "xy" (kwarg)
8. `uv run -- snail '_ = 5; _ + 1'` - Returns 6 (identifier)
9. `uv run -- snail '"x" | f("y")'` - Error: missing placeholder in call
10. `uv run -- snail '"x" | f(_, b=_)'` - Error: multiple placeholders
11. `make test` - Full CI passes
