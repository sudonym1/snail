#![allow(dead_code)]

use snail_ast::{AssignTarget, Condition, Expr, Program, SourceSpan, Stmt, StringDelimiter};
use snail_error::ParseError;
use snail_parser::parse_program;

pub fn parse_ok(source: &str) -> Program {
    parse_program(source).expect("program should parse")
}

pub fn parse_err(source: &str) -> ParseError {
    parse_program(source).expect_err("program should fail")
}

pub fn expect_assign(stmt: &Stmt) -> (&Vec<AssignTarget>, &Expr) {
    match stmt {
        Stmt::Assign { targets, value, .. } => (targets, value),
        other => panic!("Expected assignment, got {other:?}"),
    }
}

pub fn expect_expr_stmt(stmt: &Stmt) -> &Expr {
    match stmt {
        Stmt::Expr { value, .. } => value,
        other => panic!("Expected expression statement, got {other:?}"),
    }
}

pub fn expect_name(expr: &Expr, expected: &str) {
    match expr {
        Expr::Name { name, .. } => assert_eq!(name, expected),
        other => panic!("Expected name {expected}, got {other:?}"),
    }
}

pub fn expect_condition_expr(cond: &Condition) -> &Expr {
    match cond {
        Condition::Expr(expr) => expr.as_ref(),
        other => panic!("Expected expression condition, got {other:?}"),
    }
}

pub fn expect_condition_name(cond: &Condition, expected: &str) {
    let expr = expect_condition_expr(cond);
    expect_name(expr, expected);
}

pub fn expect_number(expr: &Expr, expected: &str) {
    match expr {
        Expr::Number { value, .. } => assert_eq!(value, expected),
        other => panic!("Expected number {expected}, got {other:?}"),
    }
}

pub fn expect_string(expr: &Expr, expected: &str, raw: bool, delimiter: StringDelimiter) {
    match expr {
        Expr::String {
            value,
            raw: is_raw,
            delimiter: actual_delimiter,
            ..
        } => {
            assert_eq!(value, expected);
            assert_eq!(*is_raw, raw);
            assert_eq!(*actual_delimiter, delimiter);
        }
        other => panic!("Expected string, got {other:?}"),
    }
}

pub fn expect_string_contains(expr: &Expr, snippet: &str, raw: bool, delimiter: StringDelimiter) {
    match expr {
        Expr::String {
            value,
            raw: is_raw,
            delimiter: actual_delimiter,
            ..
        } => {
            assert!(value.contains(snippet), "{value:?} missing {snippet:?}");
            assert_eq!(*is_raw, raw);
            assert_eq!(*actual_delimiter, delimiter);
        }
        other => panic!("Expected string, got {other:?}"),
    }
}

pub fn expect_byte_string(expr: &Expr, expected: &str, raw: bool, delimiter: StringDelimiter) {
    match expr {
        Expr::String {
            value,
            raw: is_raw,
            bytes,
            delimiter: actual_delimiter,
            ..
        } => {
            assert_eq!(value, expected);
            assert_eq!(*is_raw, raw);
            assert!(*bytes, "Expected byte string (bytes=true)");
            assert_eq!(*actual_delimiter, delimiter);
        }
        other => panic!("Expected byte string, got {other:?}"),
    }
}

pub fn expect_byte_fstring(expr: &Expr) {
    match expr {
        Expr::FString { bytes, .. } => {
            assert!(*bytes, "Expected byte f-string (bytes=true)");
        }
        other => panic!("Expected byte f-string, got {other:?}"),
    }
}

pub fn expect_span_start(span: &SourceSpan, line: usize, column: usize) {
    assert_eq!(span.start.line, line);
    assert_eq!(span.start.column, column);
}

pub fn expect_err_span(err: &ParseError, line: usize, column: usize) {
    let span = err.span.as_ref().expect("expected error span");
    assert_eq!(span.start.line, line);
    assert_eq!(span.start.column, column);
}
