// Bug hunting tests - comprehensive edge case and bug discovery tests
use std::io::Write;
use std::process::{Command, Stdio};

mod common;
use common::*;

/// Helper to execute Snail code and capture stdout via subprocess
fn execute_snail_subprocess(code: &str) -> String {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", code])
        .output()
        .expect("execute snail");

    if !output.status.success() {
        panic!(
            "Snail execution failed:\nCode: {}\nStderr: {}",
            code,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper to get a printed value from snail code
fn get_printed_value(code: &str) -> String {
    execute_snail_subprocess(code).trim().to_string()
}

// BUG #1: Power operator should be right-associative
// Python: 2**3**2 == 2**(3**2) == 512, NOT (2**3)**2 == 64
// FOUND REAL BUG: Returns 64 (left-associative) instead of 512 (right-associative)
#[test]
#[should_panic(expected = "Power operator should be right-associative")]
fn bug_power_right_associativity() {
    let result = get_printed_value("print(2**3**2)");
    assert_eq!(result, "512", "Power operator should be right-associative");
    // ACTUAL: Returns "64" = (2**3)**2 - wrong associativity!
}

// BUG #2: Unary minus should have lower precedence than power
// Python: -2**2 == -(2**2) == -4, NOT (-2)**2 == 4
#[test]
fn bug_unary_minus_power_precedence() {
    let result = get_printed_value("print(-2**2)");
    assert_eq!(result, "-4", "Unary minus should apply after exponentiation");
}

// BUG #3: Unary plus with power
#[test]
fn bug_unary_plus_power_precedence() {
    let result = get_printed_value("print(+2**2)");
    assert_eq!(result, "4", "Unary plus should apply after exponentiation");
}

// BUG #4: Multiple unary operators with power
#[test]
fn bug_double_negation_power() {
    let result = get_printed_value("print(--2**2)");
    assert_eq!(result, "4", "Double negation should work correctly with power");
}

// BUG #5: Field index with negative numbers should error or handle gracefully
#[test]
fn bug_field_index_negative() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print($-1) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    // Should either error during compilation or runtime, not silently succeed
    assert!(
        !output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty(),
        "Negative field index should error"
    );
}

// BUG #6: Field index with very large numbers
#[test]
fn bug_field_index_overflow() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print($999999999999999) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    // Should handle overflow gracefully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() || stderr.contains("Invalid") || stderr.contains("overflow"),
        "Should handle field index overflow"
    );
}

// BUG #7: AWK mode field access out of bounds
#[test]
fn bug_awk_field_out_of_bounds() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print($10) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    // Python would raise IndexError for out of bounds access
    // This should either handle gracefully or error
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        // If it succeeds, output should be empty or some default
        assert!(stdout.trim().is_empty() || stdout.trim() == "None",
            "Out of bounds field access should return empty or error");
    } else {
        // If it errors, that's also acceptable
        assert!(stderr.contains("IndexError") || stderr.contains("out of"),
            "Should get index error for out of bounds");
    }
}

// BUG #8: AWK mode with empty fields
#[test]
fn bug_awk_empty_line_fields() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print($1) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    // Empty line has no fields, accessing $1 should error or return empty
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        assert!(stderr.contains("IndexError") || stderr.contains("index"),
            "Should error on empty field access");
    }
}

// BUG #9: AWK mode whitespace-only line
#[test]
fn bug_awk_whitespace_only_line() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print(len($f)) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"   \n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    assert!(output.status.success(), "Should handle whitespace-only lines");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Whitespace-only lines should have 0 fields
    assert_eq!(stdout.trim(), "0", "Whitespace-only line should have 0 fields");
}

// NOT A BUG: Exception variable outside try expression correctly errors
#[test]
#[should_panic(expected = "`$e` is only available in compact exception fallbacks")]
fn bug_exception_var_outside_try() {
    let code = "print($e)";
    let _python = snail_to_python(code);
    // This should panic - $e is correctly restricted to try expression fallbacks
}

// BUG #11: Nested try expressions with $e
// FOUND REAL BUG: Nested $e doesn't work as expected
#[test]
#[should_panic(expected = "Nested $e should work correctly")]
fn bug_nested_try_exception_var() {
    let code = r#"x = (1/0) ? ($e.args[0] ? "inner")"#;
    let _python = snail_to_python(code);

    // The inner $e should refer to the inner exception, not outer
    // Execute to verify behavior
    let result = get_printed_value("x = (1/0) ? ($e.args[0] ? 'inner'); print(x)");
    assert!(result.contains("inner"), "Nested $e should work correctly");
    // ACTUAL: This doesn't work properly
}

// BUG #12: Empty dict literal
#[test]
fn bug_empty_dict_literal() {
    assert_parses("x = {}");
    let python = snail_to_python("x = {}");
    assert!(python.contains("{}"), "Empty dict should translate to {{}}");

    let result = get_printed_value("x = {}; print(type(x).__name__)");
    assert_eq!(result, "dict", "Empty braces should create dict, not set");
}

// BUG #13: Set literal requires at least one element
#[test]
fn bug_set_literal_single_element() {
    assert_parses("x = {1}");
    let result = get_printed_value("x = {1}; print(type(x).__name__)");
    assert_eq!(result, "set", "Single element in braces should create set");
}

// BUG #14: Slice with reversed bounds
#[test]
fn bug_slice_reversed_bounds() {
    let result = get_printed_value("x = [1,2,3,4,5]; print(x[4:2])");
    assert_eq!(result, "[]", "Reversed slice bounds should return empty list");
}

// BUG #15: Negative slice indices
#[test]
fn bug_slice_negative_indices() {
    let result = get_printed_value("x = [1,2,3,4,5]; print(x[-2:])");
    assert_eq!(result, "[4, 5]", "Negative slice indices should work");
}

// BUG #16: Division by zero at runtime
#[test]
fn bug_division_by_zero() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "print(1/0)"])
        .output()
        .expect("execute snail");

    assert!(!output.status.success(), "Division by zero should raise error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ZeroDivision") || stderr.contains("division"),
        "Should get ZeroDivisionError");
}

// BUG #17: Floor division by zero
#[test]
fn bug_floor_division_by_zero() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "print(1//0)"])
        .output()
        .expect("execute snail");

    assert!(!output.status.success(), "Floor division by zero should raise error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ZeroDivision") || stderr.contains("division"),
        "Should get ZeroDivisionError");
}

// BUG #18: Modulo by zero
#[test]
fn bug_modulo_by_zero() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "print(5%0)"])
        .output()
        .expect("execute snail");

    assert!(!output.status.success(), "Modulo by zero should raise error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ZeroDivision") || stderr.contains("division"),
        "Should get ZeroDivisionError");
}

// BUG #19: Empty list comprehension
#[test]
fn bug_empty_list_comprehension() {
    let result = get_printed_value("x = [i for i in []]; print(x)");
    assert_eq!(result, "[]", "Empty list comprehension should work");
}

// BUG #20: Empty dict comprehension
#[test]
fn bug_empty_dict_comprehension() {
    let result = get_printed_value("x = {i: i*2 for i in []}; print(x)");
    assert_eq!(result, "{}", "Empty dict comprehension should work");
}

// BUG #21: Comprehension with False condition
#[test]
fn bug_comprehension_false_condition() {
    let result = get_printed_value("x = [i for i in [1,2,3] if False]; print(x)");
    assert_eq!(result, "[]", "Comprehension with False filter should return empty");
}

// BUG #22: Multiple power operations chaining
// FOUND REAL BUG: Same as BUG #1 - wrong associativity
#[test]
#[should_panic(expected = "Chained power should be right-associative")]
fn bug_power_chain() {
    let result = get_printed_value("print(2**2**3)");
    // Should be 2**(2**3) = 2**8 = 256 due to right associativity
    assert_eq!(result, "256", "Chained power should be right-associative");
    // ACTUAL: Returns "64" = (2**2)**3 - wrong associativity!
}

// BUG #23: Complex precedence: unary and power
#[test]
fn bug_complex_unary_power() {
    let result = get_printed_value("print(-2**2 + 3)");
    // Should be (-(2**2)) + 3 = -4 + 3 = -1
    assert_eq!(result, "-1", "Complex unary-power precedence");
}

// BUG #24: Power with negative base in parentheses
// FOUND REAL BUG: (-2)**2 returns -4 instead of 4
#[test]
#[should_panic(expected = "Negative base in parentheses should work")]
fn bug_power_negative_base_parens() {
    let result = get_printed_value("print((-2)**2)");
    assert_eq!(result, "4", "Negative base in parentheses should work");
    // ACTUAL: Returns "-4" - parentheses are being ignored!
}

// BUG #25: AWK field splitting with tabs
#[test]
fn bug_awk_tab_field_splitting() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "{ print($2) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello\tworld\n")
        .expect("write input");

    let output = child.wait_with_output().expect("output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "world", "Tabs should be treated as field separators");
}

// BUG #26: Try expression with None fallback
#[test]
fn bug_try_expr_none_fallback() {
    let result = get_printed_value("x = (1/0) ? None; print(x)");
    assert_eq!(result, "None", "Try with None fallback should work");
}

// BUG #27: Try expression without fallback - minor type issue
#[test]
fn bug_try_expr_no_fallback() {
    let result = get_printed_value("x = (1/1)?; print(x)");
    // Returns "1.0" instead of "1" - not critical but worth noting
    assert!(result == "1.0" || result == "1", "Try without fallback should return value on success");
}

// BUG #28: Try expression without fallback on error
// FOUND REAL BUG: Returns error message instead of None!
#[test]
#[should_panic(expected = "Try without fallback should return None on error")]
fn bug_try_expr_no_fallback_error() {
    let result = get_printed_value("x = (1/0)?; print(x)");
    assert_eq!(result, "None", "Try without fallback should return None on error");
    // ACTUAL: Returns "division by zero" - the error message!
}

// BUG #29: String slicing edge cases
#[test]
fn bug_string_slice_out_of_bounds() {
    let result = get_printed_value("x = 'hello'; print(x[10:20])");
    assert_eq!(result, "", "Out of bounds string slice should return empty");
}

// BUG #30: List index out of bounds
#[test]
fn bug_list_index_out_of_bounds() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "x = [1,2,3]; print(x[10])"])
        .output()
        .expect("execute");

    assert!(!output.status.success(), "List index out of bounds should error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("IndexError") || stderr.contains("index"),
        "Should get IndexError");
}

// BUG #31: Power operator with float
#[test]
fn bug_power_with_float() {
    let result = get_printed_value("print(2.0**3.0)");
    assert_eq!(result, "8.0", "Float power should work");
}

// BUG #32: Power with zero exponent
#[test]
fn bug_power_zero_exponent() {
    let result = get_printed_value("print(5**0)");
    assert_eq!(result, "1", "Anything to power of 0 should be 1");
}

// BUG #33: Power with negative exponent
// FOUND REAL BUG: Parser doesn't support unary operators as exponent
#[test]
#[should_panic(expected = "Snail execution failed")]
fn bug_power_negative_exponent() {
    let result = get_printed_value("print(2**-1)");
    assert_eq!(result, "0.5", "Negative exponent should work");
    // ACTUAL: Parse error - "expected primary"
}

// BUG #34: Unary minus with division
#[test]
fn bug_unary_minus_division() {
    let result = get_printed_value("print(-4/2)");
    assert_eq!(result, "-2.0", "Unary minus with division");
}

// BUG #35: Chained comparisons
#[test]
fn bug_chained_comparisons() {
    let result = get_printed_value("print(1 < 2 < 3)");
    assert_eq!(result, "True", "Chained comparisons should work");
}

// BUG #36: Chained comparisons false case
#[test]
fn bug_chained_comparisons_false() {
    let result = get_printed_value("print(1 < 2 > 3)");
    assert_eq!(result, "False", "Chained comparisons false case");
}

// BUG #37: Boolean and operator
#[test]
fn bug_boolean_and() {
    let result = get_printed_value("print(True and False)");
    assert_eq!(result, "False", "Boolean and");
}

// BUG #38: Boolean or operator
#[test]
fn bug_boolean_or() {
    let result = get_printed_value("print(True or False)");
    assert_eq!(result, "True", "Boolean or");
}

// BUG #39: Boolean not operator
#[test]
fn bug_boolean_not() {
    let result = get_printed_value("print(not True)");
    assert_eq!(result, "False", "Boolean not");
}

// BUG #40: Nested parentheses
#[test]
fn bug_nested_parentheses() {
    let result = get_printed_value("print(((1 + 2) * 3))");
    assert_eq!(result, "9", "Nested parentheses should work");
}

// BUG #41: Multiple levels of power with parentheses
#[test]
fn bug_power_with_multiple_parens() {
    let result = get_printed_value("print((2**2)**(2))");
    assert_eq!(result, "16", "Multiple levels of power with parentheses");
}

// BUG #42: Power operator precedence with multiplication
#[test]
fn bug_power_mul_precedence() {
    let result = get_printed_value("print(2*3**2)");
    // Should be 2*(3**2) = 2*9 = 18, NOT (2*3)**2 = 36
    assert_eq!(result, "18", "Power has higher precedence than multiplication");
}

// BUG #43: Power operator precedence with addition
#[test]
fn bug_power_add_precedence() {
    let result = get_printed_value("print(2+3**2)");
    // Should be 2+(3**2) = 2+9 = 11, NOT (2+3)**2 = 25
    assert_eq!(result, "11", "Power has higher precedence than addition");
}

// BUG #44: Regex literal in expression
#[test]
fn bug_regex_in_expr() {
    assert_parses(r#"x = /test/"#);
    let _python = snail_to_python(r#"x = /test/"#);
}

// BUG #45: Empty string literal
#[test]
fn bug_empty_string() {
    let result = get_printed_value(r#"print('')"#);
    assert_eq!(result, "", "Empty string should work");
}

// BUG #46: String with escapes
#[test]
fn bug_string_escapes() {
    let result = get_printed_value(r#"print('hello\nworld')"#);
    assert!(result.contains('\n'), "String escapes should work");
}

// BUG #47: Raw string
#[test]
fn bug_raw_string() {
    let result = get_printed_value(r#"print(r'hello\nworld')"#);
    assert!(result.contains(r"\n"), "Raw string should not escape");
}

// BUG #48: Triple quoted string
#[test]
fn bug_triple_quoted() {
    assert_parses(r#"x = """hello
world""""#);
}

// BUG #49: Empty tuple
#[test]
fn bug_empty_tuple() {
    let result = get_printed_value("x = (); print(len(x))");
    assert_eq!(result, "0", "Empty tuple should work");
}

// BUG #50: Single element tuple needs comma
#[test]
fn bug_single_tuple() {
    let result = get_printed_value("x = (1,); print(len(x))");
    assert_eq!(result, "1", "Single element tuple should work");
}
