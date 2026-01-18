# Map Mode Implementation Plan

## Overview

Add a new "map mode" to Snail that processes files one at a time, exposing special variables for file path, handle, and lazy content.

## Requirements

- **CLI flag**: `--map` / `-m`
- **Input**: File paths from command-line args, or from stdin (one per line) if no args provided
- **Execution**: Code runs once per file (no BEGIN/END blocks)
- **Special variables**:
  - `$src` - source file path as string
  - `$fd` - open file handle (text mode, read)
  - `$text` - lazy file content (read on first access, then cached)
- **Helpers should be map-mode aware**: for example, js() should consume $fd
  by default in map-mode. ** CRITICAL TODO ** this feature should be split out
  and also consider the plans in plans/remove-BEGIN-END.md

## Generated Python Structure

```python
import sys

if sys.argv[1:]:
    __snail_paths = sys.argv[1:]
else:
    __snail_paths = [line.rstrip('\n') for line in sys.stdin if line.rstrip('\n')]

for __snail_src in __snail_paths:
    with open(__snail_src, 'r') as __snail_fd:
        __snail_text = __SnailLazyText(__snail_fd)
        # User code here
```

## Implementation Steps

### 1. Add `CompileMode::Map` variant
**File**: `crates/snail-ast/src/awk.rs`

```rust
pub enum CompileMode {
    Snail,
    Awk,
    Map,  // Add this
}
```

### 2. Add map variable constants
**File**: `crates/snail-lower/src/constants.rs`

Add constants:
```rust
pub(crate) const SNAIL_MAP_SRC: &str = "$src";
pub(crate) const SNAIL_MAP_FD: &str = "$fd";
pub(crate) const SNAIL_MAP_TEXT: &str = "$text";
pub(crate) const SNAIL_MAP_SRC_PYVAR: &str = "__snail_src";
pub(crate) const SNAIL_MAP_FD_PYVAR: &str = "__snail_fd";
pub(crate) const SNAIL_MAP_TEXT_PYVAR: &str = "__snail_text";
pub const SNAIL_LAZY_TEXT_CLASS: &str = "__SnailLazyText";
```

Update `injected_py_name()` to handle map variables.

### 3. Add LazyText runtime helper
**File**: `python/snail/runtime/lazy_text.py` (new file)

```python
class LazyText:
    """Lazily reads file content on first access."""
    __slots__ = ('_fd', '_text')

    def __init__(self, fd):
        self._fd = fd
        self._text = None

    def _ensure_loaded(self):
        if self._text is None:
            self._text = self._fd.read()
        return self._text

    def __str__(self):
        return self._ensure_loaded()

    def __repr__(self):
        return repr(str(self))

    def __eq__(self, other):
        if isinstance(other, LazyText):
            return str(self) == str(other)
        return str(self) == other

    def __hash__(self):
        return hash(str(self))

    def __len__(self):
        return len(str(self))

    def __iter__(self):
        return iter(str(self))

    def __contains__(self, item):
        return item in str(self)

    def __add__(self, other):
        return str(self) + other

    def __radd__(self, other):
        return other + str(self)

    def __getattr__(self, name):
        return getattr(str(self), name)
```

**File**: `python/snail/runtime/__init__.py`

Add import and install in `install_helpers()`:
```python
globals_dict["__SnailLazyText"] = LazyText
```

### 4. Add parser validation for map mode
**File**: `crates/snail-parser/src/lib.rs`

Add map-only validation constants:
```rust
const MAP_ONLY_NAMES: [&str; 3] = ["$src", "$fd", "$text"];
const MAP_ONLY_MESSAGE: &str = "map variables are only valid in map mode; use --map";
```

Add `parse_map_program()` function that:
1. Reuses `parse_program()` parsing logic
2. Rejects awk-only variables (`$n`, `$fn`, `$p`, `$m`, `$0`, `$1`, etc.)
3. Allows map variables (`$src`, `$fd`, `$text`)

Update `validate_expr()` to reject map variables in snail mode (add to the `Expr::Name` match arm).

### 5. Implement map mode lowering
**File**: `crates/snail-lower/src/map.rs` (new file)

Create `lower_map_program_with_auto_print()`:
1. Generate `import sys`
2. Generate paths source logic (args or stdin)
3. Generate file loop with `with open()` context manager
4. Generate `__snail_text = __SnailLazyText(__snail_fd)`
5. Lower user code inside the loop

### 6. Update snail-lower exports
**File**: `crates/snail-lower/src/lib.rs`

Add module and exports:
```rust
mod map;
pub use program::lower_map_program_with_auto_print;
```

### 7. Update core compilation API
**File**: `crates/snail-core/src/lib.rs`

Add Map mode dispatch:
```rust
CompileMode::Map => {
    let program = parse_map_program(source)?;
    let module = lower_map_program_with_auto_print(py, &program, auto_print_last)?;
    Ok(module)
}
```

### 8. Update Python extension mode parsing
**File**: `crates/snail-python/src/lib.rs`

Update `parse_mode()`:
```rust
"map" => Ok(CompileMode::Map),
```

Update error message to include "map".

### 9. Update CLI
**File**: `python/snail/cli.py`

- Add `self.map = False` to `_Args.__init__()`
- Add help text for `-m, --map`
- Add argument parsing for `-m` / `--map`
- Add mutual exclusivity check: `--awk` and `--map` cannot be used together
- Update mode selection: `mode = "map" if namespace.map else ("awk" if namespace.awk else "snail")`

### 10. Add example file
**File**: `examples/map.snail` (new file)

```snail
#!/usr/bin/env -S snail --map -f
# Process files passed as arguments or via stdin
print("File:", $src)
print("Size:", len($text), "bytes")
print("First line:", $text.split('\n')[0] if $text else "(empty)")
print("---")
```

## Critical Files to Modify

| File | Change |
|------|--------|
| `crates/snail-ast/src/awk.rs` | Add `Map` variant to `CompileMode` |
| `crates/snail-lower/src/constants.rs` | Add map variable constants |
| `crates/snail-parser/src/lib.rs` | Add `parse_map_program()`, update validation |
| `crates/snail-lower/src/map.rs` | New file: map lowering logic |
| `crates/snail-lower/src/lib.rs` | Export map lowering |
| `crates/snail-core/src/lib.rs` | Add Map mode dispatch |
| `crates/snail-python/src/lib.rs` | Update mode parsing |
| `python/snail/cli.py` | Add `--map` flag |
| `python/snail/runtime/__init__.py` | Install LazyText helper |
| `python/snail/runtime/lazy_text.py` | New file: LazyText class |

## Testing Strategy

### Parser Tests
`crates/snail-parser/tests/map.rs` (new file):
- `parse_map_allows_map_variables` - `$src`, `$fd`, `$text` accepted
- `parse_map_rejects_awk_variables` - `$0`, `$n`, etc. rejected
- `parse_snail_rejects_map_variables` - `$src` rejected in snail mode

### CLI Integration Tests
`python/tests/test_cli.py`:
- `test_map_mode_from_args` - files passed as CLI args
- `test_map_mode_from_stdin` - files passed via stdin
- `test_map_mode_text_content` - verify `$text` contains file content
- `test_map_mode_fd_access` - verify `$fd` is readable handle
- `test_map_mode_lazy_text` - verify `$text` is lazy
- `test_map_identifiers_require_map_mode` - `$src` rejected without `--map`
- `test_awk_and_map_mutually_exclusive` - both flags = error

## Verification

1. Build: `cargo build`
2. Run all checks: `make test`
3. Manual test:
   ```bash
   # Test with args
   echo "hello" > /tmp/a.txt
   echo "world" > /tmp/b.txt
   uv run -- snail --map 'print($src, len($text))' /tmp/a.txt /tmp/b.txt

   # Test with stdin
   echo -e "/tmp/a.txt\n/tmp/b.txt" | uv run -- snail --map 'print($src)'
   ```
