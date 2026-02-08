use std::error::Error as _;

use snail_ast::{SourcePos, SourceSpan};
use snail_error::{LowerError, ParseError, SnailError, format_snail_error};

fn span(line: usize, column: usize) -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line,
            column,
        },
        end: SourcePos {
            offset: 0,
            line,
            column,
        },
    }
}

#[test]
fn parse_formatting_without_span() {
    let err = ParseError::new("unexpected token");
    let rendered = format_snail_error(&SnailError::from(err), "script.snail");
    assert_eq!(rendered, "error: unexpected token\n");
}

#[test]
fn parse_formatting_with_span_without_line_text() {
    let mut err = ParseError::new("unexpected token");
    err.span = Some(span(3, 5));

    let rendered = format_snail_error(&SnailError::from(err), "script.snail");
    assert_eq!(rendered, "error: unexpected token\n--> script.snail:3:5\n");
}

#[test]
fn parse_formatting_with_span_and_line_text() {
    let mut err = ParseError::new("unexpected token");
    err.span = Some(span(5, 8));
    err.line_text = Some("if x == {".to_string());

    let rendered = format_snail_error(&SnailError::from(err), "script.snail");
    assert_eq!(
        rendered,
        "error: unexpected token\n--> script.snail:5:8\n |\n   5 | if x == {\n |        ^\n"
    );
}

#[test]
fn parse_formatting_caret_edge_columns() {
    let mut col_one = ParseError::new("bad column");
    col_one.span = Some(span(1, 1));
    col_one.line_text = Some("abc".to_string());
    let col_one_rendered = format_snail_error(&SnailError::from(col_one), "script.snail");
    assert!(col_one_rendered.contains(" | ^\n"));

    let mut col_zero = ParseError::new("bad column");
    col_zero.span = Some(span(1, 0));
    col_zero.line_text = Some("abc".to_string());
    let col_zero_rendered = format_snail_error(&SnailError::from(col_zero), "script.snail");
    assert!(col_zero_rendered.contains(" | ^\n"));
}

#[test]
fn lower_error_formatting_uses_error_prefix() {
    let rendered = format_snail_error(
        &SnailError::from(LowerError::new("lower failed")),
        "script.snail",
    );
    assert_eq!(rendered, "error: lower failed");
}

#[test]
fn snail_error_from_and_source_preserve_inner_errors() {
    let parse_err = ParseError::new("parse failed");
    let parse_snail: SnailError = parse_err.clone().into();
    assert!(matches!(parse_snail, SnailError::Parse(_)));
    assert_eq!(parse_snail.to_string(), parse_err.to_string());
    let parse_source = parse_snail
        .source()
        .expect("parse source should be present");
    assert_eq!(parse_source.to_string(), parse_err.to_string());

    let lower_err = LowerError::new("lower failed");
    let lower_snail: SnailError = lower_err.clone().into();
    assert!(matches!(lower_snail, SnailError::Lower(_)));
    assert_eq!(lower_snail.to_string(), lower_err.to_string());
    let lower_source = lower_snail
        .source()
        .expect("lower source should be present");
    assert_eq!(lower_source.to_string(), lower_err.to_string());
}
