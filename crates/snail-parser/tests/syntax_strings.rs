mod common;

use common::*;
use snail_ast::{Expr, FStringPart, StringDelimiter};

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
                FStringPart::Expr(expr) => expect_name(expr, "name"),
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
                FStringPart::Expr(expr) => expect_name(expr, "y"),
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
