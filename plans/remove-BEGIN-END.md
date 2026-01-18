# Plan: Replace BEGIN/END Blocks with CLI Flags

## Summary
Remove `BEGIN { }` and `END { }` syntax from awk mode grammar and replace with `-b <code>` and `-e <code>` CLI flags. The semantics remain the same: `-b` code runs before the main loop, `-e` code runs after.

## Key Design Decisions
1. **Keep AST structure unchanged** - `AwkProgram` retains `begin_blocks` and `end_blocks` fields; only how they're populated changes
2. **Parse begin/end code as regular Snail programs** - The `-b` and `-e` code is parsed using `parse_program()` and the statements are extracted
3. **Add new parameters to Rust API** - `exec_py`, `compile_py`, and `compile_ast_py` get optional `begin_code` and `end_code` parameters
4. **Interleaved flag parsing** - Support argument order like `snail -b 'foo' 'main' -e 'bar' /file` by continuing to parse flags after the main program

## Implementation Steps

### 1. Grammar (`crates/snail-parser/src/snail.pest`)
Remove lines 8-9:
```diff
 awk_entry_list = { awk_entry ~ (stmt_sep* ~ awk_entry)* ~ stmt_sep* }
-awk_entry = _{ awk_begin | awk_end | awk_rule }
-awk_begin = { "BEGIN" ~ block }
-awk_end = { "END" ~ block }
+awk_entry = _{ awk_rule }
 awk_rule = { block | awk_pattern ~ block? }
```

### 2. Parser (`crates/snail-parser/src/lib.rs`)
- Remove `Rule::awk_begin` and `Rule::awk_end` handling from `parse_awk_program()` (around lines 56-72)
- Add new public function:
```rust
pub fn parse_awk_program_with_begin_end(
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
) -> Result<AwkProgram, ParseError>
```
- Update `parse_awk_program()` to delegate to the new function with empty slices

### 3. Core API (`crates/snail-core/src/lib.rs`)
Add new compilation function:
```rust
pub fn compile_awk_source_with_begin_end(
    py: Python<'_>,
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
) -> Result<PyObject, SnailError>
```

### 4. Rust Extension (`crates/snail-python/src/lib.rs`)
Update function signatures to accept optional begin/end code:

**`exec_py`** (line 201):
```rust
#[pyo3(signature = (source, *, argv = Vec::new(), mode = "snail", auto_print = true,
                    auto_import = true, filename = "<snail>",
                    begin_code = Vec::new(), end_code = Vec::new()))]
fn exec_py(..., begin_code: Vec<String>, end_code: Vec<String>) -> PyResult<i32>
```

**`compile_py`** (line 156) and **`compile_ast_py`** (line 187): Same pattern.

Update `compile_source()` helper to branch on awk mode and call `compile_awk_source_with_begin_end()`.

### 5. CLI (`python/snail/cli.py`)

**Update `_Args` class** (line 86):
```python
self.begin_code: list[str] = []
self.end_code: list[str] = []
```

**Update `_parse_args()`** (line 113):
- Add handling for `-b` and `-e` flags (similar to `-f`)
- **Critical change**: Continue parsing flags after the main program is found to support `-b 'x' 'main' -e 'y'` order. The current logic stops at the first positional arg.

**Update `_print_help()`** (line 98):
```python
print("  -b <code>               code to run before main loop (awk mode)", file=file)
print("  -e <code>               code to run after main loop (awk mode)", file=file)
```

**Update `main()`** (line 182):
- Validate `-b`/`-e` only used with `--awk` mode
- Pass `begin_code` and `end_code` to `exec()` and `compile_ast()`

### 6. Update Examples

**`examples/awk.snail`**: Remove BEGIN/END blocks, keep only the rules:
```snail
#!/usr/bin/env -S snail --awk -b 'print("demo begin")' -e 'print("demo end")' -f
$0.startswith("d") { print("matched:", $0) }
/de(mo)/ { print("regex:", $m.1) }
{ print($fn); print($0) }
```

### 7. Tests

**Parser tests** (`crates/snail-parser/tests/parser.rs`):
- Test that BEGIN/END keywords are no longer valid in awk programs
- Test `parse_awk_program_with_begin_end()` with multiple begin/end sources

**CLI tests** (`python/tests/test_cli.py`):
- Test `-b` flag alone
- Test `-e` flag alone
- Test multiple `-b` and `-e` flags
- Test interleaved argument order: `-b 'x' 'main' -e 'y'`
- Test that `-b`/`-e` without `--awk` produces an error
- Update existing awk tests that use BEGIN/END syntax

### 8. Documentation

**`docs/REFERENCE.md`** (lines 282-284):
Update awk mode section from:
> `BEGIN` and `END` blocks run before and after the line loop...

To:
> Use `-b <code>` flags for code to run before the main loop and `-e <code>` flags for code to run after...

Add CLI usage examples showing `-b` and `-e` flags.

**`README.md`** (lines 38-48):
Update the Awk Mode example to use the new CLI flag syntax:
```snail-awk("5\n4\n3\n2\n1\nbanana\n")
# Run with: snail --awk -b 'total = 0' -e 'print("Sum:", total); assert total == 15'
/^[0-9]+/ { total = total + int($1) }
```

Or show the CLI invocation with `-b` and `-e` flags in prose.

## Files to Modify
| File | Changes |
|------|---------|
| `crates/snail-parser/src/snail.pest` | Remove `awk_begin`, `awk_end` rules |
| `crates/snail-parser/src/lib.rs` | Remove BEGIN/END handling, add new function |
| `crates/snail-core/src/lib.rs` | Add `compile_awk_source_with_begin_end()` |
| `crates/snail-python/src/lib.rs` | Add `begin_code`/`end_code` params to functions |
| `python/snail/cli.py` | Add `-b`/`-e` flag parsing |
| `examples/awk.snail` | Update to use new CLI flags |
| `docs/REFERENCE.md` | Update awk mode section for new flags |
| `README.md` | Update awk mode example for new flags |
| `crates/snail-parser/tests/parser.rs` | Update/add tests |
| `python/tests/test_cli.py` | Add tests for new flags |

## Verification
1. `cargo fmt`
2. `RUSTFLAGS="-D warnings" cargo build`
3. `cargo build --features run-proptests`
4. `cargo clippy -- -D warnings`
5. `cargo test`
6. `uv run -- python -m maturin develop`
7. `uv run -- python -m pytest python/tests`
8. **`make test`** (final verification before commit)

Manual testing:
```bash
# Basic begin/end
echo -e "a\nb" | uv run -- snail --awk -b 'print("start")' -e 'print("end")' '{ print($0) }'
# Expected: start\na\nb\nend

# Multiple begin/end
echo "x" | uv run -- snail --awk -b 'x=0' -b 'y=0' -e 'print(x,y)' '{ x=x+1 }'
# Expected: 1 0
```
