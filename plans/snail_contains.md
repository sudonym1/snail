# __snail_contains__ + Chained Comparisons Plan

## Goals
- Allow `in` / `not in` to return non-bool truthy values when a user-provided
  `__snail_contains__` exists.
- Preserve Python comparison semantics when no `__snail_contains__` is present.
- Support chained comparisons (`a in b in c`, `a < b < c`) with single
  evaluation per operand and short-circuit behavior.

## Proposed Semantics

### __snail_contains__ lookup
- For `left in right` and `left not in right`:
  - If `right` has `__snail_contains__`, call it with `left`.
  - Otherwise, fall back to normal Python `in` / `not in` behavior.
- `__snail_contains__` may return any truthy value (not limited to `bool`).

### not in behavior
- When `__snail_contains__` is used, `not in` negates truthiness
  (i.e., `not bool(result)`), yielding a Python `bool`.
- When `__snail_contains__` is absent, `not in` stays standard Python.

### Chained comparisons
- Lower chained comparisons as a short-circuiting `and` sequence with temps to
  evaluate each operand once, mirroring Python.
- Each comparison within the chain uses the same rule:
  - For `in` / `not in`, prefer `__snail_contains__` on the right.
  - Otherwise, use Python comparison operators.

## Current State (for clean-slate context)
- Regex matching lowers to `__snail_regex_search(...)` in
  `crates/snail-lower/src/expr.rs` via `lower_regex_match`.
- Runtime regex helpers live in `python/snail/runtime/regex.py`:
  - `regex_search(value, pattern)` returns a tuple
    `(match.group(0),) + match.groups()` or `()` on no match.
  - `regex_compile(pattern)` returns a `SnailRegex` object (custom class).
  - `SnailRegex.search(value)` delegates to `regex_search`.
  - `SnailRegex.__contains__(value)` currently returns `bool(self.search(value))`.
- This means:
  - `"abc" in /123/` returns a tuple from `__snail_regex_search`.
  - `"abc" in p` where `p = /123/` currently returns `bool` because
    Python `in` uses `SnailRegex.__contains__`.
- Comparisons lower to a Python `Compare` AST node in
  `crates/snail-lower/src/expr.rs` (see `Expr::Compare` lowering).

## Implementation Notes

### Runtime helper
Introduce helpers in `python/snail/runtime/__init__.py` (and export them via
`install_helpers`):

- `__snail_contains__(left, right)`:
  - If `right` has `__snail_contains__`, return `right.__snail_contains__(left)`.
  - Else, return `left in right` (Python semantics).

- `__snail_contains_not__(left, right)`:
  - If `right` has `__snail_contains__`, return `not bool(right.__snail_contains__(left))`.
  - Else, return `left not in right` (Python semantics).

Note: no special-casing of regex helpers here; this stays generic.

### Lowering
Update `crates/snail-lower/src/expr.rs`:

- Detect comparison nodes with `CompareOp::In` / `CompareOp::NotIn`.
- For single comparisons:
  - Lower to a call to `__snail_contains__` (or `__snail_contains_not__`)
    instead of Python `Compare`, so `__snail_contains__` can return non-bool
    values.
- For chained comparisons (`a op1 b op2 c ...`):
  - Lower to temp assignments + short-circuiting `BoolOp::And` chain.
  - Evaluate each operand exactly once (match Python semantics).
  - Each comparison in the chain should use the same rule:
    - If `op` is `in`/`not in`, use the helper(s).
    - Otherwise use Python comparison operators.

Suggested lowering shape (pseudo-Python):
```
_tmp0 = a
_tmp1 = b
_ok = compare(_tmp0, op1, _tmp1)  # uses __snail_contains__ for in/not in
_tmp0 = _tmp1
_tmp1 = c
_ok = _ok and compare(_tmp0, op2, _tmp1)
...
```
Notes:
- Keep values as expressions in AST or assign to temps, but ensure each
  operand expression is evaluated once.
- Use a consistent temp naming scheme (see other helper temps in
  `crates/snail-lower/src/constants.rs` like `__snail_let_*`).

### Tests
Add CLI tests:
- `m = "abc" in /123/` should return the tuple from regex search.
- `p = /123/; m = "abc" in p` should return the same tuple as above.
- Chained comparisons with `in` should preserve semantics, e.g.
  `"a" in pat in [pat]` should short-circuit correctly.

### Docs
- Update `docs/REFERENCE.md` to describe `__snail_contains__` hook semantics.
- Add a short note in `README.md` if needed.
