# Plan: Ripgrep-Backed Regex + Awk Fast Path

## Goal
- Replace Snail regex literals and `string in /pattern/` with a Rust backend using ripgrep crates.
- Add a native awk fast path so `snail -a '/foo/ {bar}'` approaches ripgrep single-threaded speed.

## Non-goals
- Multi-threaded search.
- New Snail syntax.
- Full PCRE/Python `re` feature parity unless explicitly chosen.

## Assumptions (confirmed)
- Use Rust regex flavor only (no backrefs/lookaround). No Python `re` fallback.
- Use BurntSushi `grep` crate stack for regex/searcher behavior.
- Strict UTF-8 input handling; invalid UTF-8 is a runtime error.
- Awk fast path can be limited to regex-only patterns and still call Python for actions.

## Decisions (confirmed)
- Non-UTF-8 input is a runtime error.
- Unmatched capture groups return `None` in the match tuple.
- Regex compile errors are raised at runtime.
- Regex literals compile eagerly at module load.
- Non-awk regex matches compile patterns before search; compiled regex objects are passed to search helpers.
- Use ripgrep's `grep` stack (`grep-searcher` + `grep-regex` via the `grep` crate) for streaming scans; avoid `grep-cli`/`grep-printer`.

## Plan
1. Confirm regex semantics and edge cases
   - Codify runtime error behavior for invalid UTF-8 input in native awk scan and in regex helpers.
   - Ensure match tuple uses `None` for unmatched capture groups (Rust `regex` yields `None` for absent captures).
   - Emit runtime errors for compile failures (no parse-time validation beyond syntax).
   - Inventory regex usage and patterns in tests/examples; note any Python-only constructs.
   - Commands:
     - `rg -n "Regex|/pattern/|regex" crates python docs examples`
2. Implement native regex backend (Snail-wide, no fallback)
   - Add dependencies to `crates/snail-python/Cargo.toml`:
     - `grep`, `regex`, `regex-syntax`, `once_cell`, `lru` (or `hashbrown`-based cache).
   - Remove `python/snail/runtime/regex.py` usage; route all regex helpers to `_native`.
   - Compile regex literals eagerly when the Python AST is executed (e.g., during module init codegen).
   - Create `crates/snail-python/src/regex.rs`:
     - `#[pyclass] SnailRegex { pattern: String, compiled: Regex/RegexMatcher }`
     - `search(value) -> tuple` with group 0 + groups, empty tuple if no match.
     - `__snail_contains__`, `__contains__`, `__repr__` consistent with current Python runtime.
     - `regex_search(value, pattern)` and `regex_compile(pattern)` with an LRU cache for string patterns.
     - Ensure lowering compiles regex patterns first so searches always use compiled regex objects.
     - If `pattern` is a Python object with `search`, call it as a fallback (optional, for compatibility).
   - Export new functions from `snail._native` and wire them in `python/snail/runtime/__init__.py`:
     - Import `_native` regex functions directly; no Python fallback.
   - Commands:
     - `rg -n "__snail_regex_(search|compile)" crates python`
3. Add awk native fast path for regex-only rules
   - Define a native scanner in `snail._native` (e.g., `awk_scan`):
     - Uses `grep` searcher to stream UTF-8 lines and match with compiled regex.
     - Validate UTF-8 per line; raise runtime error on invalid input.
     - Updates `$0`, `$m`, `$n`, `$fn`, `$p`, fields, and invokes Python callbacks for rule actions.
     - Calls back into Python only on matches to reduce overhead.
   - Update awk lowering to detect regex-only rules:
     - If all patterns are regex literals or `expr in /regex/` with `$0`, emit a call to `__snail_awk_scan`.
     - Otherwise, fall back to existing Python line loop.
   - Provide a runtime flag/env var to disable native fast path for debugging.
   - Commands:
     - `rg -n "lower_awk" crates/snail-lower/src/awk.rs`
4. Tests and docs
   - Add Rust unit tests for native regex tuple semantics and caching behavior.
   - Update `python/tests/test_cli.py` regex tests if behavior changes.
   - Add awk-mode tests to ensure `$m` and field vars are correct when native path is used.
   - Update `docs/REFERENCE.md` to describe regex flavor, limitations, and awk fast path.
   - Commands:
     - `rg -n "regex" python/tests docs/REFERENCE.md`
5. Performance validation and tuning
   - Add a benchmark script or documented manual steps:
     - `time uv run -- snail -a '/foo/ {print($0)}' data.txt`
     - `time rg --no-messages --threads 1 'foo' data.txt`
   - Tune searcher settings (line terminator, binary detection) and regex caching.
   - Compare match rate effects and document expected perf characteristics.

## Verification
- Functional:
  - `cargo test`
  - `uv run -- python -m pytest python/tests`
- Perf sanity:
  - `time uv run -- snail -a '/foo/ {print($0)}' data.txt`
  - `time rg --threads 1 'foo' data.txt`
- Before any commit/push/PR: `make test` (must be last command).
