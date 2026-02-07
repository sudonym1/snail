# Validation Coverage Test Matrix (Main Baseline)

This document defines the exact tests to add on `main` (without this branch checked out) to improve coverage for parser validation paths refactored in:

- `a90652f` (`ValidationMode` and shared validation traversal)
- `9715b20` (`validate_expr_mode` arm consolidation + helper reuse)

The goal is to cover nested validation recursion and mode-specific reserved-name checks.

## Files To Update

- Parser/unit validation tests:
  - `crates/snail-parser/tests/errors.rs`
- CLI/end-to-end validation tests:
  - `python/tests/test_cli.py`

## Parser Tests (`crates/snail-parser/tests/errors.rs`)

Use existing helpers and style in this file:

- `parse_err(...)`
- `parse_map(...)`
- `assert!(err.to_string().contains(...))`

### A. Regular Mode Reserved-Name Inventory

1. `rejects_map_only_variables_in_regular_mode`
   - Case 1: `value = $fd`
   - Case 2: `value = $text`
   - Assert: message contains variable token and `--map`.

2. `rejects_additional_awk_only_variables_in_regular_mode`
   - Case 1: `value = $fn`
   - Case 2: `value = $m`
   - Case 3: `value = $f`
   - Assert: message contains variable token and `--awk`.

3. `rejects_src_in_regular_mode_plain_stmt`
   - Source: `print($src)`
   - Assert: message contains `$src` and `map or awk mode`.

### B. `validate_expr_mode` Nested Recursion (Regular Mode)

4. `rejects_awk_vars_in_unary_yieldfrom_paren`
   - Source: `def gen() { yield from (-($n)) }`
   - Assert: message contains `$n` and `--awk`.

5. `rejects_awk_vars_in_structural_exprs_and_compare`
   - Case 1: `x = (1; $n)` (compound expr tail)
   - Case 2: `x = [$n]` (list element)
   - Case 3: `x = ($n,)` (tuple element)
   - Case 4: `x = {$n}` (set element)
   - Case 5: `x = 1 < $n < 3` (comparator)
   - Assert: each case message contains `$n` and `--awk`.

6. `rejects_awk_vars_in_call_argument_forms`
   - Case 1: `x = f($n)` (positional)
   - Case 2: `x = f(k=$n)` (keyword)
   - Case 3: `x = f(*$n)` (star arg)
   - Case 4: `x = f(**$n)` (kw-star arg)
   - Assert: each case message contains `$n` and `--awk`.

7. `rejects_awk_vars_in_dict_index_slice_try_yield`
   - Case 1: `x = %{$n: 1}` (dict key)
   - Case 2: `x = %{"ok": $n}` (dict value)
   - Case 3: `x = items[$n]` (index expr)
   - Case 4: `x = items[$n:]` (slice start)
   - Case 5: `x = items[:$n]` (slice end)
   - Case 6: `x = risky():$n?` (try fallback)
   - Case 7: `def g() { yield $n }` (yield value)
   - Assert: each case message contains `$n` and `--awk`.

### C. Interpolation Recursion

8. `rejects_reserved_names_in_fstring_subprocess_regex_interpolation`
   - Case 1: `s = "{$src}"` (fstring expression)
   - Case 2: `out = $(echo {$src})` (subprocess interpolation)
   - Case 3: `ok = "x" in /{$src}/` (regex interpolation)
   - Assert: each case message contains `$src` and `map or awk mode`.

9. `rejects_awk_name_in_nested_format_spec`
   - Source: `s = "{value:{$n}.{prec}f}"`
   - Assert: message contains `$n` and `--awk`.

### D. Comprehension Paths

10. `rejects_reserved_names_in_list_comp_positions`
    - Case 1: `items = [$n for n in nums]` (element)
    - Case 2: `items = [n for n in $text]` (iter)
    - Case 3: `items = [n for n in nums if $src]` (ifs)
    - Assert:
      - `$n` cases contain `--awk`
      - `$text` contains `--map`
      - `$src` contains `map or awk mode`.

11. `rejects_reserved_names_in_dict_comp_positions`
    - Case 1: `lookup = %{$n: n for n in nums}` (key)
    - Case 2: `lookup = %{n: $fd for n in nums}` (value)
    - Case 3: `lookup = %{n: n for n in $text}` (iter)
    - Case 4: `lookup = %{n: n for n in nums if $src}` (ifs)
    - Assert by variable class (`--awk`, `--map`, `map or awk mode`).

### E. Map-Mode Specific Behavior

12. `map_allows_map_vars_in_nested_expr_contexts`
    - Source:
      - `s = "{$src}"`
      - `out = $(echo {$text})`
      - `ok = "x" in /{$src}/`
      - `items = [$src for n in $text if $fd]`
      - `lookup = %{$src: $fd for n in $text if $src}`
    - Assert: `parse_map(...)` succeeds.

13. `map_rejects_awk_vars_in_nested_expr_contexts`
    - Case 1: `items = [$n for n in nums if n > 0]`
    - Case 2: `items = [n for n in nums if $n]`
    - Case 3: `s = "{$n}"`
    - Case 4: `ok = "x" in /{$n}/`
    - Case 5: `x = items[$1]`
    - Assert: `parse_map(...)` errors and contains awk marker (`--awk` or token).

14. `map_begin_end_rejects_map_and_awk_vars_comprehensively`
    - Case 1: `BEGIN { print($fd) }\nprint($src)`
    - Case 2: `END { print($text) }\nprint($src)`
    - Case 3: `BEGIN { print($n) }\nprint($src)`
    - Case 4: `BEGIN { print($1) }\nprint($src)`
    - Assert: each case errors and contains offending token.

15. `program_begin_end_rejects_additional_reserved_vars`
    - Case 1: `BEGIN { print($fd) }`
    - Case 2: `END { print($text) }`
    - Case 3: `BEGIN { print($fn) }`
    - Case 4: `BEGIN { print($m) }`
    - Case 5: `BEGIN { print($f) }`
    - Assert: each case errors and contains offending token.

## CLI Tests (`python/tests/test_cli.py`)

Add focused end-to-end mirrors for nested validation recursion:

1. `test_map_identifiers_require_map_mode_in_fstring_interpolation`
   - Source: `print("{$src}")`
   - Assert: `main([...])` raises `SyntaxError` with `map or awk mode`.

2. `test_map_identifiers_require_map_mode_in_subprocess_interpolation`
   - Source: `x = $(echo {$src})`
   - Assert: `SyntaxError` with `map or awk mode`.

3. `test_map_identifiers_require_map_mode_in_regex_interpolation`
   - Source: `print("x" in /{$src}/)`
   - Assert: `SyntaxError` with `map or awk mode`.

4. `test_map_identifiers_require_map_mode_in_lambda_call_arguments`
   - Cases:
     - `f = def() { g($src) }`
     - `f = def() { g(k=$src) }`
     - `f = def() { g(*$src) }`
     - `f = def() { g(**$src) }`
   - Assert: each case raises `SyntaxError` with `map or awk mode`.

5. `test_map_begin_end_flags_reject_map_vars_fd_text`
   - Mirror existing begin-flag rejection but with `$fd` and `$text` (not only `$src`).
   - Assert: `SyntaxError`.

## Verification Commands

Run after implementing tests on `main`:

```bash
cargo test -p snail-parser
uv run -- python -m pytest python/tests/test_cli.py
make test
```

`make test` must remain the final command before commit.
