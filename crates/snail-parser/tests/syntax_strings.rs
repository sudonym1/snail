mod common;

use common::*;
use snail_ast::{Expr, FStringConversion, FStringPart, StringDelimiter};

// ============================================================================
// Byte string tests
// ============================================================================

#[test]
fn parses_byte_string_double_quote() {
    let source = r#"x = b"hello""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, "hello", false, StringDelimiter::Double);
}

#[test]
fn parses_byte_string_single_quote() {
    let source = "x = b'hello'";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, "hello", false, StringDelimiter::Single);
}

#[test]
fn parses_raw_byte_string_rb_prefix() {
    let source = r#"x = rb"\n""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, r"\n", true, StringDelimiter::Double);
}

#[test]
fn parses_raw_byte_string_br_prefix() {
    let source = r#"x = br"\n""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, r"\n", true, StringDelimiter::Double);
}

#[test]
fn parses_triple_quoted_byte_string() {
    let source = r#"x = b"""multi
line""""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, "multi\nline", false, StringDelimiter::TripleDouble);
}

#[test]
fn parses_interpolated_byte_string() {
    // Byte strings support interpolation in Snail (unlike Python)
    let source = r#"x = b"hello {name}""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_fstring(value);

    // Verify it has the expected parts
    match value {
        Expr::FString { parts, bytes, .. } => {
            assert!(*bytes);
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                FStringPart::Text(text) => assert_eq!(text, "hello "),
                other => panic!("Expected text part, got {other:?}"),
            }
            match &parts[1] {
                FStringPart::Expr(expr) => {
                    expect_name(&expr.expr, "name");
                    assert_eq!(expr.conversion, FStringConversion::None);
                    assert!(expr.format_spec.is_none());
                }
                other => panic!("Expected expression part, got {other:?}"),
            }
        }
        other => panic!("Expected FString, got {other:?}"),
    }
}

#[test]
fn parses_raw_byte_string_no_interpolation() {
    // Raw byte strings should NOT interpolate
    let source = r#"x = rb"test {expr} more""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_byte_string(value, "test {expr} more", true, StringDelimiter::Double);
}

// ============================================================================
// Regular string tests
// ============================================================================

#[test]
fn parses_raw_and_multiline_strings() {
    let source = "text = r\"hello\\n\"\nblock = \"\"\"line1\nline2\"\"\"\nraw_block = r\"\"\"raw\\nline\"\"\"";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_string(value, "hello\\n", true, StringDelimiter::Double);

    let (_, value) = expect_assign(&program.stmts[1]);
    expect_string_contains(value, "line1\nline2", false, StringDelimiter::TripleDouble);

    let (_, value) = expect_assign(&program.stmts[2]);
    expect_string_contains(value, "raw\\nline", true, StringDelimiter::TripleDouble);
}

#[test]
fn parses_raw_string_with_curly_braces() {
    let source = r#"x = r"{ \"key\": \"value\" }""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_string(
        value,
        r#"{ \"key\": \"value\" }"#,
        true,
        StringDelimiter::Double,
    );
}

#[test]
fn parses_raw_string_without_interpolation() {
    let source = r#"x = r"test {expr} more""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_string(value, "test {expr} more", true, StringDelimiter::Double);
}

#[test]
fn parses_raw_triple_quoted_string_with_json() {
    let source = r#####"x = r"""
{
  "hook_event_name": "Status",
  "session_id": "abc123"
}
""""#####;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    expect_string_contains(
        value,
        "hook_event_name",
        true,
        StringDelimiter::TripleDouble,
    );
}

#[test]
fn parses_regular_string_with_interpolation() {
    let source = r#"x = "test {y} more""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::FString { parts, .. } => {
            assert_eq!(parts.len(), 3);
            match &parts[0] {
                FStringPart::Text(text) => assert_eq!(text, "test "),
                other => panic!("Expected text part, got {other:?}"),
            }
            match &parts[1] {
                FStringPart::Expr(expr) => {
                    expect_name(&expr.expr, "y");
                    assert_eq!(expr.conversion, FStringConversion::None);
                    assert!(expr.format_spec.is_none());
                }
                other => panic!("Expected expression part, got {other:?}"),
            }
            match &parts[2] {
                FStringPart::Text(text) => assert_eq!(text, " more"),
                other => panic!("Expected text part, got {other:?}"),
            }
        }
        other => panic!("Expected FString, got {other:?}"),
    }
}

#[test]
fn parses_fstring_conversion_and_format_spec() {
    let source = r#"x = "value {y!r:>8}""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::FString { parts, .. } => {
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                FStringPart::Text(text) => assert_eq!(text, "value "),
                other => panic!("Expected text part, got {other:?}"),
            }
            match &parts[1] {
                FStringPart::Expr(expr) => {
                    expect_name(&expr.expr, "y");
                    assert_eq!(expr.conversion, FStringConversion::Repr);
                    let spec = expr.format_spec.as_ref().expect("format spec");
                    assert_eq!(spec.len(), 1);
                    match &spec[0] {
                        FStringPart::Text(text) => assert_eq!(text, ">8"),
                        other => panic!("Expected text spec part, got {other:?}"),
                    }
                }
                other => panic!("Expected expression part, got {other:?}"),
            }
        }
        other => panic!("Expected FString, got {other:?}"),
    }
}

#[test]
fn parses_fstring_nested_format_spec() {
    let source = r#"x = "{value:{width}.{prec}f}""#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::FString { parts, .. } => {
            assert_eq!(parts.len(), 1);
            match &parts[0] {
                FStringPart::Expr(expr) => {
                    expect_name(&expr.expr, "value");
                    let spec = expr.format_spec.as_ref().expect("format spec");
                    assert_eq!(spec.len(), 4);
                    match &spec[0] {
                        FStringPart::Expr(spec_expr) => {
                            expect_name(&spec_expr.expr, "width");
                            assert_eq!(spec_expr.conversion, FStringConversion::None);
                        }
                        other => panic!("Expected width expression, got {other:?}"),
                    }
                    match &spec[1] {
                        FStringPart::Text(text) => assert_eq!(text, "."),
                        other => panic!("Expected dot text, got {other:?}"),
                    }
                    match &spec[2] {
                        FStringPart::Expr(spec_expr) => {
                            expect_name(&spec_expr.expr, "prec");
                            assert_eq!(spec_expr.conversion, FStringConversion::None);
                        }
                        other => panic!("Expected prec expression, got {other:?}"),
                    }
                    match &spec[3] {
                        FStringPart::Text(text) => assert_eq!(text, "f"),
                        other => panic!("Expected trailing text, got {other:?}"),
                    }
                }
                other => panic!("Expected expression part, got {other:?}"),
            }
        }
        other => panic!("Expected FString, got {other:?}"),
    }
}
