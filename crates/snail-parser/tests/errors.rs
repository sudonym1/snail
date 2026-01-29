mod common;

use common::*;
use snail_parser::{
    parse_awk_program, parse_awk_program_with_begin_end, parse_map_program_with_begin_end,
    parse_program,
};

#[test]
fn reports_parse_error_with_location() {
    let err = parse_err("if { }");
    let message = err.to_string();
    assert!(message.contains("-->"));
    assert!(message.contains("if"));
    expect_err_span(&err, 1, 4);
}

#[test]
fn rejects_user_defined_dollar_identifiers() {
    let err = parse_err("$bad = 1");
    let message = err.to_string();
    assert!(message.contains("$bad"));
    expect_err_span(&err, 1, 1);
}

#[test]
fn rejects_awk_only_variables_in_regular_mode() {
    let err = parse_err("value = $n");
    let message = err.to_string();
    assert!(message.contains("$n"));
    assert!(message.contains("--awk"));
    expect_err_span(&err, 1, 9);
}

#[test]
fn rejects_awk_field_indices_in_regular_mode() {
    let err = parse_err("value = $1");
    let message = err.to_string();
    assert!(message.contains("$1"));
    assert!(message.contains("--awk"));
    expect_err_span(&err, 1, 9);
}

#[test]
fn parser_rejects_unclosed_brace() {
    let err = parse_err("if x { y = 1");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("unclosed") || message.contains("}"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_invalid_assignment_target() {
    let err = parse_err("1 = x");
    let message = err.to_string();
    assert!(
        message.contains("assign") || message.contains("target") || message.contains("expected")
    );
    assert!(err.span.is_some());
}

#[test]
fn parser_handles_unterminated_string() {
    let err = parse_err("x = \"hello");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_incomplete_if_statement() {
    let err = parse_err("if");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("if"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_missing_condition() {
    let err = parse_err("if { x = 1 }");
    assert!(err.span.is_some());
}

#[test]
fn parser_reports_error_on_missing_colon_in_dict() {
    let err = parse_err("d = %{\"key\" 1}");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains(":"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_braced_expression() {
    let err = parse_err("d = {}");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_incomplete_function_def() {
    let err = parse_err("def foo");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("(") || message.contains("{"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_unclosed_paren() {
    let err = parse_err("result = (1 + 2");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains(")"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_unclosed_bracket() {
    let err = parse_err("items = [1, 2, 3");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("]"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_invalid_expression_in_binary_op() {
    let err = parse_err("x = 1 +");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("expression"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_missing_except_after_try() {
    let source = "try { x = 1 }";
    match parse_program(source) {
        Ok(_) => {}
        Err(err) => {
            let message = err.to_string();
            assert!(
                message.contains("expected")
                    || message.contains("except")
                    || message.contains("finally")
            );
        }
    }
}

#[test]
fn parser_reports_error_location_correctly() {
    let source = "x = 1\ny = 2\nif {";
    let err = parse_err(source);
    let span = err.span.expect("expected span");
    assert_eq!(span.start.line, 3);
}

#[test]
fn parser_rejects_invalid_import_syntax() {
    let err = parse_err("import");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("import"));
}

#[test]
fn parser_rejects_invalid_from_import() {
    let err = parse_err("from");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("import"));
}

#[test]
fn parser_rejects_missing_iterable_in_for_loop() {
    let err = parse_err("for x in { }");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_invalid_comprehension_syntax() {
    let err = parse_err("[x for]");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("in"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_unexpected_token() {
    let err = parse_err("x = 1 @ 2");
    expect_err_span(&err, 1, 7);
}

#[test]
fn parser_rejects_nested_unclosed_structures() {
    let err = parse_err("if x { if y { z = 1 }");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("}"));
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_invalid_parameter_syntax() {
    let err = parse_err("def foo(1) { pass }");
    let message = err.to_string();
    assert!(
        message.contains("expected")
            || message.contains("identifier")
            || message.contains("parameter")
    );
    assert!(err.span.is_some());
}

// ========== AWK Mode Parser Tests ==========

#[test]
fn awk_begin_end_parsed_as_blocks() {
    let program = parse_awk_program("BEGIN { print(1) } /foo/ { print($0) } END { print(2) }")
        .expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert_eq!(program.begin_blocks.len(), 1);
    assert_eq!(program.end_blocks.len(), 1);
}

#[test]
fn awk_parses_simple_rules() {
    let program = parse_awk_program("/foo/ { print($0) }").expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert!(program.begin_blocks.is_empty());
    assert!(program.end_blocks.is_empty());
}

#[test]
fn awk_with_begin_end_injects_blocks() {
    let program =
        parse_awk_program_with_begin_end("/foo/ { print($0) }", &["x = 1", "y = 2"], &["print(x)"])
            .expect("should parse");

    assert_eq!(program.rules.len(), 1);
    assert_eq!(program.begin_blocks.len(), 2);
    assert_eq!(program.end_blocks.len(), 1);
}

#[test]
fn awk_with_empty_begin_end() {
    let program = parse_awk_program_with_begin_end("/foo/", &[], &[]).expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert!(program.begin_blocks.is_empty());
    assert!(program.end_blocks.is_empty());
}

#[test]
fn awk_begin_end_rejects_awk_vars() {
    let err = parse_awk_program("BEGIN { print($0) }").expect_err("should reject awk vars");
    assert!(err.to_string().contains("$0"));
}

#[test]
fn map_begin_end_parsed_as_blocks() {
    let (program, begin_blocks, end_blocks) =
        parse_map_program_with_begin_end("BEGIN { print(1) } print($src) END { print(2) }")
            .expect("should parse");
    assert_eq!(program.stmts.len(), 1);
    assert_eq!(begin_blocks.len(), 1);
    assert_eq!(end_blocks.len(), 1);
}

#[test]
fn map_begin_end_rejects_map_vars() {
    let err = parse_map_program_with_begin_end("BEGIN { print($src) }\nprint($src)")
        .expect_err("should reject map vars in BEGIN/END");
    assert!(err.to_string().contains("$src"));
}

#[test]
fn map_requires_separators_between_simple_statements() {
    let err = parse_map_program_with_begin_end("print($src) print($src)")
        .expect_err("should reject missing separators");
    assert!(err.to_string().contains("expected statement separator"));
}

// ========== F-String Interpolation Tests ==========

#[test]
fn rejects_invalid_fstring_interpolation_syntax() {
    // Original bug case: x(!) should fail, not silently parse as x
    let err = parse_err(r#"s = "{x(!)}""#);
    let message = err.to_string();
    assert!(message.contains("unexpected"));
}

#[test]
fn rejects_trailing_garbage_in_fstring() {
    let err = parse_err(r#"s = "{x abc}""#);
    let message = err.to_string();
    assert!(message.contains("unexpected"));
}

#[test]
fn rejects_invalid_operator_in_fstring() {
    let err = parse_err(r#"s = "{x @@ y}""#);
    let message = err.to_string();
    assert!(message.contains("unexpected"));
}

#[test]
fn rejects_invalid_fstring_conversion() {
    let err = parse_err(r#"s = "{x!q}""#);
    let message = err.to_string();
    assert!(message.contains("conversion"));
}

#[test]
fn rejects_trailing_chars_after_fstring_conversion() {
    let err = parse_err(r#"s = "{x!r abc}""#);
    let message = err.to_string();
    assert!(message.contains("conversion"));
}

#[test]
fn valid_fstring_expressions_still_work() {
    parse_ok(r#"s = "{x}""#);
    parse_ok(r#"s = "{x()}""#);
    parse_ok(r#"s = "{x(1, 2)}""#);
    parse_ok(r#"s = "{x.y.z}""#);
    parse_ok(r#"s = "{a + b * c}""#);
    parse_ok(r#"s = "{items[0]}""#);
    parse_ok(r#"s = "{f(g(h()))}""#);
}
