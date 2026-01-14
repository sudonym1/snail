mod common;

use common::*;
use snail_ast::{Expr, FStringPart, StringDelimiter};

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
