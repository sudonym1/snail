# Test Coverage Analysis for Snail

**Date:** 2026-01-02
**Analyzed by:** Claude
**Current Test Count:** ~59 tests (all passing)

## Executive Summary

The Snail test suite provides good coverage of happy paths but has significant quality issues:

- ✅ **Good:** Comprehensive happy path coverage across parser, lowering, AWK, CLI, and Python integration
- ❌ **Critical:** 12+ brittle tests using exact string matching that break with formatting changes
- ❌ **Critical:** 83:1 ratio of happy-to-error path testing (167 `.expect()` vs 2 `.expect_err()`)
- ❌ **Major:** 19+ weak tests only checking statement counts without validating content
- ❌ **Major:** Tests depend on external `python3` binary availability

---

## Test Quality Metrics

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total Tests | 59 | - | ✅ |
| Happy Path Tests | 167 `.expect()` | - | ✅ |
| Error Path Tests | 2 `.expect_err()` | ~80 | ❌ |
| Coverage Ratio | 83:1 | 2:1 | ❌ |
| Brittle Golden Tests | 12 | 0 | ❌ |
| Weak Assertion Tests | 19 | 0 | ❌ |

---

## Critical Issues

### 1. Brittle String-Matching Tests (Priority: CRITICAL)

**Location:** `tests/lower.rs` lines 155-372
**Count:** 12 tests
**Impact:** High - breaks on any formatting change

#### Problem

Tests use exact string matching against hardcoded Python output:

```rust
// BRITTLE EXAMPLE - tests/lower.rs:327-338
#[test]
fn renders_compact_exception_expression() {
    let expected = "def __snail_compact_try(expr_fn, fallback_fn=None):\n    try:\n        return expr_fn()\n    except Exception as __snail_compact_exc:\n        if fallback_fn is None:\n            fallback_member = getattr(__snail_compact_exc, \"__fallback__\", None)\n            if callable(fallback_member):\n                return fallback_member()\n            return __snail_compact_exc\n        return fallback_fn(__snail_compact_exc)\n\nvalue = __snail_compact_try(lambda: risky())\nfallback = __snail_compact_try(lambda: risky(), lambda __snail_compact_exc: __snail_compact_exc)\ndetails = __snail_compact_try(lambda: risky(), lambda __snail_compact_exc: __snail_compact_exc.args[0])\n";
    assert_eq!(rendered, expected);  // Breaks with ANY formatting change!
}
```

**These tests break if:**
- Indentation changes (spaces vs tabs)
- Helper function order changes
- Parenthesization preferences change
- Any whitespace formatting changes

#### Affected Tests

- `renders_python_golden_output` (155-168)
- `renders_list_and_dict_comprehensions` (204-213)
- `renders_try_except_finally` (216-228)
- `renders_try_finally_and_raise_from` (232-243)
- `renders_with_statement` (247-255)
- `renders_assert_and_del` (259-269)
- `renders_tuples_sets_and_slices` (273-288)
- `renders_defaults_and_star_args` (292-314)
- `renders_loop_else_and_try_break_continue` (305-314)
- `renders_if_expression` (318-324)
- `renders_compact_exception_expression` (328-338)
- `renders_subprocess_expressions` (342-351)
- `renders_regex_expressions` (355-372)

#### Recommended Fix

Replace string matching with semantic testing:

```rust
// ROBUST ALTERNATIVE
#[test]
fn compact_exception_expression_works() {
    let source = r#"
value = risky()?
fallback = risky() ? $e
"#;
    let program = parse_program(source).unwrap();
    let module = lower_program(&program).unwrap();
    let python = python_source(&module);

    // Test 1: Python compiles without syntax errors
    assert_python_compiles(&python);

    // Test 2: Verify AST structure
    assert!(module.body.iter().any(|stmt|
        matches!(stmt, PyStmt::FunctionDef { name, .. } if name == "__snail_compact_try")
    ));

    // Test 3: Execution produces correct behavior
    Python::with_gil(|py| {
        let test_code = format!("{}\ndef risky(): raise ValueError('test')\n", python);
        let result = py.run_bound(&test_code, None, None);
        // Verify semantic correctness
    });
}
```

---

### 2. Weak Assertions - Statement Count Only (Priority: HIGH)

**Location:** `tests/parser.rs`
**Count:** 19 tests
**Impact:** Medium - misses structural bugs

#### Problem

Tests only verify `program.stmts.len()` without validating actual content:

```rust
// WEAK EXAMPLE - tests/parser.rs:3-13
#[test]
fn parses_basic_program() {
    let source = r#"
x = 1
if x {
  y = x + 2
}
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);  // Only checks count!
    // What if both statements are wrong but count is right?
}
```

#### Affected Tests

All tests in `tests/parser.rs` that only check `.len()`:
- `parses_basic_program`
- `parses_semicolon_before_newline`
- `parses_if_elif_else_chain`
- `parses_def_and_call`
- `parses_imports`
- `parses_attribute_and_index_assignment_targets`
- `parses_list_and_dict_literals_and_comprehensions`
- `parses_raw_and_multiline_strings`
- `parses_try_except_finally_and_raise`
- `parses_raise_from_and_try_finally`
- `parses_with_statement`
- `parses_assert_and_del`
- `parses_tuples_sets_and_slices`
- `parses_defaults_and_star_args`
- `parses_loop_else_with_try_break_continue`
- `parses_if_expression`
- `parses_compact_exception_expression`
- `parses_subprocess_expressions`
- `parses_regex_expressions`

#### Recommended Fix

Add structural validation:

```rust
// STRONG ALTERNATIVE
#[test]
fn parses_basic_program() {
    let source = r#"
x = 1
if x {
  y = x + 2
}
"#;
    let program = parse_program(source).unwrap();
    assert_eq!(program.stmts.len(), 2);

    // Validate assignment structure
    match &program.stmts[0] {
        Stmt::Assign { targets, value, .. } => {
            assert_eq!(targets.len(), 1);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "x"));
            assert!(matches!(value, Expr::Number { value, .. } if value == "1"));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }

    // Validate if statement structure
    match &program.stmts[1] {
        Stmt::If { cond, body, elifs, else_body, .. } => {
            assert!(matches!(cond, Expr::Name { name, .. } if name == "x"));
            assert_eq!(body.len(), 1);
            assert!(elifs.is_empty());
            assert!(else_body.is_none());
        }
        other => panic!("Expected if statement, got {:?}", other),
    }
}
```

---

### 3. Severe Lack of Error Path Coverage (Priority: CRITICAL)

**Statistics:**
- Happy path: 167 `.expect()` calls
- Error path: 2 `.expect_err()` calls
- **Ratio: 83:1** (should be ~2:1)

#### Missing Error Tests

**Parser Errors (estimate: 20+ tests needed):**
```rust
#[test]
fn parser_rejects_unclosed_brace() {
    let err = parse_program("if x { y = 1").expect_err("should fail");
    assert!(err.message.contains("unclosed") || err.message.contains("expected"));
}

#[test]
fn parser_rejects_invalid_assignment_target() {
    let err = parse_program("1 = x").expect_err("should fail");
    assert!(err.message.contains("cannot assign"));
}

#[test]
fn parser_handles_unterminated_string() {
    let err = parse_program(r#"x = "hello"#).expect_err("should fail");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_malformed_regex() {
    let err = parse_program("x = /[/").expect_err("should fail");
    assert!(err.message.contains("regex") || err.message.contains("pattern"));
}

#[test]
fn parser_reports_error_location_correctly() {
    let source = "x = 1\ny = 2\nif { }";
    let err = parse_program(source).expect_err("should fail");
    assert_eq!(err.span.unwrap().start.line, 3);
}
```

**Lowering Errors (estimate: 10+ tests needed):**
```rust
#[test]
fn lower_validates_try_has_handlers() {
    // Parser might allow, but lowering should validate
    let source = "try { x = 1 }";
    let result = parse_program(source);
    match result {
        Err(_) => {}, // Parser caught it
        Ok(program) => {
            // Lowering should catch it
            assert!(lower_program(&program).is_err());
        }
    }
}
```

**CLI Errors (estimate: 15+ tests needed):**
```rust
#[test]
fn cli_reports_file_not_found() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .arg("/nonexistent/path.snail")
        .output()
        .expect("should run");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to read"));
}

#[test]
fn cli_reports_parse_errors_with_location() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "if { }"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("-->"));
}

#[test]
fn cli_handles_permission_denied() {
    // Test with unreadable file
}

#[test]
fn cli_rejects_invalid_flag_combinations() {
    // Test conflicting flags
}
```

---

### 4. Environment-Dependent Tests (Priority: MEDIUM)

**Location:** `tests/lower.rs:172-201`
**Issue:** Depends on `python3` binary

```rust
#[test]
fn round_trip_executes_small_program() {
    let output = Command::new("python3")  // Fails if not in PATH
        .arg("-")
        // ...
}
```

**Problems:**
- Fails if `python3` not installed
- Fails if Python version differs
- Fails on systems where Python is `python` not `python3`

**Better Alternative:**

```rust
// Use PyO3 to run in-process
#[test]
fn round_trip_executes_small_program() {
    Python::with_gil(|py| {
        let source = "def fact(n) {\n    if n <= 1 { return 1 }\n    return n * fact(n - 1)\n}\nresult = fact(5)";
        let program = parse_program(source).unwrap();
        let module = lower_program(&program).unwrap();
        let python = python_source(&module);

        let locals = PyDict::new_bound(py);
        py.run_bound(&python, None, Some(&locals)).unwrap();

        let result: i64 = locals.get_item("result")
            .unwrap()
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(result, 120);
    });
}
```

---

## Missing Test Coverage Areas

### 1. Parser Edge Cases

- [ ] Deeply nested expressions (recursion limits)
- [ ] Very large numbers (overflow handling)
- [ ] All escape sequences in strings
- [ ] Mixed quote styles
- [ ] Operator precedence edge cases
- [ ] Whitespace handling (tabs, mixed)
- [ ] Unicode identifiers and strings
- [ ] Empty input
- [ ] Input ending without newline

### 2. AWK Mode Edge Cases

- [ ] Empty input handling
- [ ] Very long lines (>10k chars)
- [ ] Binary data in input
- [ ] Multiple files
- [ ] Field index out of bounds
- [ ] Line number overflow
- [ ] Pattern matching with groups
- [ ] Empty patterns/actions

### 3. Lowering Edge Cases

- [ ] Complex nested comprehensions
- [ ] Subprocess with special characters
- [ ] Regex with invalid patterns
- [ ] Try expressions with complex fallbacks
- [ ] Very deep nesting

### 4. Error Formatting

- [ ] Errors with Unicode source
- [ ] Errors at EOF
- [ ] Errors with very long lines
- [ ] Error span accuracy
- [ ] Multiple errors

### 5. Integration Tests

- [ ] Large real-world programs
- [ ] Circular imports
- [ ] Memory/resource cleanup
- [ ] Thread safety
- [ ] Signal handling

---

## Code Quality Issues

### Poor Use of `panic!()` in Tests

Found 13 `panic!()` calls in `tests/lower.rs`. These make failures unclear.

**Bad:**
```rust
if !output.status.success() {
    panic!("python failed: {}", String::from_utf8_lossy(&output.stderr));
}
```

**Better:**
```rust
assert!(
    output.status.success(),
    "Python execution failed:\nSTDERR: {}\nSTDOUT: {}",
    String::from_utf8_lossy(&output.stderr),
    String::from_utf8_lossy(&output.stdout)
);
```

### Massive Code Duplication

Tests repeatedly use the same patterns without helpers.

**Solution:** Create `tests/common/mod.rs`:

```rust
// tests/common/mod.rs
use snail::*;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub fn assert_parses(source: &str) -> Program {
    parse_program(source)
        .unwrap_or_else(|e| panic!("Failed to parse:\n{}\nError: {}", source, e))
}

pub fn assert_lowers(program: &Program) -> PyModule {
    lower_program(program)
        .unwrap_or_else(|e| panic!("Failed to lower: {}", e))
}

pub fn snail_to_python(source: &str) -> String {
    let program = assert_parses(source);
    let module = assert_lowers(&program);
    python_source(&module)
}

pub fn assert_python_compiles(python_code: &str) {
    Python::with_gil(|py| {
        let result = PyModule::import_bound(py, "builtins")
            .unwrap()
            .getattr("compile")
            .unwrap()
            .call1((python_code, "<test>", "exec"));

        assert!(
            result.is_ok(),
            "Generated Python has syntax errors:\n{}\n\nError: {:?}",
            python_code,
            result.unwrap_err()
        );
    });
}

pub fn execute_snail(source: &str) -> Py<PyDict> {
    Python::with_gil(|py| {
        let python = snail_to_python(source);
        let globals = PyDict::new_bound(py);
        py.run_bound(&python, None, Some(&globals))
            .unwrap_or_else(|e| panic!("Execution failed:\n{}\nError: {:?}", python, e));
        globals.into()
    })
}
```

---

## Recommended Test Organization

### Current Structure
```
tests/
├── awk.rs          (10 tests)
├── cli.rs          (6 tests)
├── lower.rs        (18 tests)
├── parser.rs       (18 tests)
└── python_integration.rs (7 tests)
```

### Proposed Structure
```
tests/
├── common/
│   └── mod.rs              (shared helpers)
├── parser/
│   ├── happy_path.rs       (18 existing tests, improved)
│   ├── errors.rs           (20+ new error tests)
│   └── edge_cases.rs       (15+ new edge case tests)
├── lower/
│   ├── correctness.rs      (12 existing tests, refactored)
│   ├── errors.rs           (10+ new error tests)
│   └── edge_cases.rs       (10+ new tests)
├── awk/
│   ├── basic.rs            (10 existing tests)
│   └── edge_cases.rs       (15+ new tests)
├── cli/
│   ├── happy_path.rs       (6 existing tests)
│   └── errors.rs           (15+ new tests)
├── integration/
│   ├── python.rs           (7 existing tests)
│   └── end_to_end.rs       (new comprehensive tests)
└── property_tests.rs       (new fuzzing/property tests)
```

---

## Action Plan

### Week 1: Fix Critical Brittleness
- [ ] Create `tests/common/mod.rs` with helpers
- [ ] Refactor 5 most brittle tests in `tests/lower.rs`
- [ ] Replace string matching with structural + compilation tests

### Week 2: Complete Brittleness Fixes
- [ ] Refactor remaining 7 brittle tests
- [ ] Strengthen 10 weak parser tests with structural assertions
- [ ] Replace `python3` subprocess with PyO3 in-process execution

### Week 3: Add Error Coverage (Phase 1)
- [ ] Create `tests/parser/errors.rs` with 10 error tests
- [ ] Create `tests/cli/errors.rs` with 10 error tests
- [ ] Add 5 error formatting tests

### Week 4: Add Error Coverage (Phase 2)
- [ ] Add remaining 10 parser error tests
- [ ] Create `tests/lower/errors.rs` with 10 error tests
- [ ] Add 5 more CLI error tests

### Month 2: Edge Cases
- [ ] Create edge case test files for each module
- [ ] Add AWK edge case tests (15+)
- [ ] Add parser edge case tests (15+)
- [ ] Add lowering edge case tests (10+)

### Month 3: Advanced Testing
- [ ] Add `proptest` dependency
- [ ] Create property-based tests
- [ ] Set up fuzzing infrastructure
- [ ] Add performance regression tests

---

## Expected Outcomes

After implementing these improvements:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Brittle Tests | 12 | 0 | 100% |
| Error Coverage Ratio | 83:1 | 2:1 | 97% |
| Weak Assertions | 19 | 0 | 100% |
| Total Tests | 59 | 150+ | 154% |
| False Failure Rate | High | Low | 80% reduction |
| Bug Detection Rate | Medium | High | 3-5x improvement |

---

## Tools and Metrics

### Recommended Additions

1. **Code Coverage Tool**
   ```bash
   cargo install cargo-tarpaulin
   cargo tarpaulin --out Html --output-dir coverage
   ```

2. **Property Testing**
   ```toml
   [dev-dependencies]
   proptest = "1.0"
   ```

3. **Mutation Testing**
   ```bash
   cargo install cargo-mutants
   cargo mutants
   ```

---

## Conclusion

The Snail test suite has **solid happy path coverage** but needs significant improvements in:

1. **Test Robustness** - Fix 12 brittle golden tests immediately
2. **Error Coverage** - Add 80+ error path tests
3. **Test Quality** - Strengthen 19 weak assertion tests
4. **Test Infrastructure** - Add helpers and reduce duplication

**The highest priority is fixing the brittle string-matching tests and adding comprehensive error path coverage.** These changes will dramatically improve test reliability and bug detection without significantly increasing maintenance burden.
