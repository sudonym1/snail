mod common;

use common::*;
use snail_parser::{parse, parse_awk, parse_awk_cli, parse_main, parse_map};

fn assert_regular_mode_error(source: &str, token: &str, mode_hint: &str) {
    let err = parse_err(source);
    let message = err.to_string();
    assert!(message.contains(token), "{source:?} => {message:?}");
    assert!(message.contains(mode_hint), "{source:?} => {message:?}");
}

fn assert_map_mode_error(source: &str, token: &str, mode_hint: &str) {
    let err = parse_map(source).expect_err("source should fail in map mode");
    let message = err.to_string();
    assert!(message.contains(token), "{source:?} => {message:?}");
    assert!(message.contains(mode_hint), "{source:?} => {message:?}");
}

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
    match parse_main(source) {
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

// ========== Regular Mode BEGIN/END Parser Tests ==========

#[test]
fn program_begin_end_parsed_as_blocks() {
    let (program, begin_blocks, end_blocks) =
        parse("BEGIN { print(1) }\nprint(2)\nEND { print(3) }").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
    assert_eq!(begin_blocks.len(), 1);
    assert_eq!(end_blocks.len(), 1);

    let (program, begin_blocks, end_blocks) = parse("x END { foo; }").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
    assert!(begin_blocks.is_empty());
    assert_eq!(end_blocks.len(), 1);
}

#[test]
fn program_begin_end_rejects_awk_map_vars() {
    let err = parse("BEGIN { print($0) }").expect_err("should reject awk vars");
    assert!(err.to_string().contains("$0"));

    let err = parse("BEGIN { print($src) }").expect_err("should reject map vars");
    assert!(err.to_string().contains("$src"));
}

// ========== AWK Mode Parser Tests ==========

#[test]
fn awk_begin_end_parsed_as_blocks() {
    let program =
        parse_awk("BEGIN { print(1) } /foo/ { print($0) } END { print(2) }").expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert_eq!(program.begin_blocks.len(), 1);
    assert_eq!(program.end_blocks.len(), 1);
}

#[test]
fn awk_parses_simple_rules() {
    let program = parse_awk("/foo/ { print($0) }").expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert!(program.begin_blocks.is_empty());
    assert!(program.end_blocks.is_empty());
}

#[test]
fn awk_with_begin_end_injects_blocks() {
    let program = parse_awk_cli("/foo/ { print($0) }", &["x = 1", "y = 2"], &["print(x)"])
        .expect("should parse");

    assert_eq!(program.rules.len(), 1);
    assert_eq!(program.begin_blocks.len(), 2);
    assert_eq!(program.end_blocks.len(), 1);
}

#[test]
fn awk_with_empty_begin_end() {
    let program = parse_awk_cli("/foo/", &[], &[]).expect("should parse");
    assert_eq!(program.rules.len(), 1);
    assert!(program.begin_blocks.is_empty());
    assert!(program.end_blocks.is_empty());
}

#[test]
fn awk_begin_end_rejects_awk_vars() {
    let err = parse_awk("BEGIN { print($0) }").expect_err("should reject awk vars");
    assert!(err.to_string().contains("$0"));
}

#[test]
fn awk_begin_end_rejects_additional_reserved_vars() {
    for (source, token) in [
        ("BEGIN { print($1) } /foo/ { print($0) }", "$1"),
        ("BEGIN { print($n) } /foo/ { print($0) }", "$n"),
        ("BEGIN { print($fn) } /foo/ { print($0) }", "$fn"),
        ("BEGIN { print($m) } /foo/ { print($0) }", "$m"),
        ("BEGIN { print($f) } /foo/ { print($0) }", "$f"),
        ("BEGIN { print($src) } /foo/ { print($0) }", "$src"),
        ("BEGIN { print($fd) } /foo/ { print($0) }", "$fd"),
        ("BEGIN { print($text) } /foo/ { print($0) }", "$text"),
    ] {
        let err = parse_awk(source).expect_err("BEGIN/END should reject reserved variables");
        let message = err.to_string();
        assert!(message.contains(token), "{source:?} => {message:?}");
    }
}

#[test]
fn awk_cli_begin_end_rejects_reserved_vars() {
    let err = parse_awk_cli("/foo/ { print($0) }", &["print($src)"], &[])
        .expect_err("CLI BEGIN should reject reserved variables");
    assert!(err.to_string().contains("$src"));

    let err = parse_awk_cli("/foo/ { print($0) }", &[], &["print($n)"])
        .expect_err("CLI END should reject reserved variables");
    assert!(err.to_string().contains("$n"));
}

#[test]
fn map_begin_end_parsed_as_blocks() {
    let (program, begin_blocks, end_blocks) =
        parse_map("BEGIN { print(1) } print($src) END { print(2) }").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
    assert_eq!(begin_blocks.len(), 1);
    assert_eq!(end_blocks.len(), 1);
}

#[test]
fn map_begin_end_rejects_map_vars() {
    let err = parse_map("BEGIN { print($src) }\nprint($src)")
        .expect_err("should reject map vars in BEGIN/END");
    assert!(err.to_string().contains("$src"));
}

#[test]
fn map_requires_separators_between_simple_statements() {
    let err = parse_map("print($src) print($src)").expect_err("should reject missing separators");
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
    parse_ok(r#"s = "{items[1:2]}""#);
    parse_ok(r#"s = "{%{"k": 1}}""#);
    parse_ok(r#"s = "{f(g(h()))}""#);
}

#[test]
fn rejects_map_only_variables_in_regular_mode() {
    for (source, token) in [
        ("value = $fd", "$fd"),
        ("value = $text", "$text"),
        ("x = risky():$fd?", "$fd"),
    ] {
        assert_regular_mode_error(source, token, "--map");
    }
}

#[test]
fn rejects_additional_awk_only_variables_in_regular_mode() {
    for (source, token) in [
        ("value = $fn", "$fn"),
        ("value = $m", "$m"),
        ("value = $f", "$f"),
    ] {
        assert_regular_mode_error(source, token, "--awk");
    }
}

#[test]
fn rejects_reserved_names_in_assignment_target_positions_in_regular_mode() {
    for (source, token, mode_hint) in [
        ("items[$n] = 1", "$n", "--awk"),
        ("items[$1] = 1", "$1", "--awk"),
        ("items[$n] += 1", "$n", "--awk"),
        ("items[$1]++", "$1", "--awk"),
        ("++items[$n]", "$n", "--awk"),
        ("items[$src] = 1", "$src", "map or awk mode"),
        ("items[$fd] += 1", "$fd", "--map"),
        ("items[$text]++", "$text", "--map"),
        ("++items[$src]", "$src", "map or awk mode"),
    ] {
        assert_regular_mode_error(source, token, mode_hint);
    }
}

#[test]
fn rejects_src_in_regular_mode_plain_stmt() {
    assert_regular_mode_error("print($src)", "$src", "map or awk mode");
}

#[test]
fn rejects_awk_vars_in_unary_yieldfrom_paren() {
    assert_regular_mode_error("def gen() { yield from (-($n)) }", "$n", "--awk");
}

#[test]
fn rejects_awk_vars_in_structural_exprs_and_compare() {
    for source in [
        "x = (1; $n)",
        "x = [$n]",
        "x = ($n,)",
        "x = #{$n}",
        "x = 1 < $n < 3",
    ] {
        assert_regular_mode_error(source, "$n", "--awk");
    }
}

#[test]
fn rejects_awk_vars_in_call_argument_forms() {
    for source in ["x = f($n)", "x = f(k=$n)", "x = f(*$n)", "x = f(**$n)"] {
        assert_regular_mode_error(source, "$n", "--awk");
    }
}

#[test]
fn rejects_awk_vars_in_dict_index_slice_try_yield() {
    for source in [
        "x = %{$n: 1}",
        "x = %{\"ok\": $n}",
        "x = items[$n]",
        "x = items[$n:]",
        "x = items[:$n]",
        "x = risky():$n?",
        "def g() { yield $n }",
    ] {
        assert_regular_mode_error(source, "$n", "--awk");
    }
}

#[test]
fn rejects_reserved_names_in_fstring_subprocess_regex_interpolation() {
    for source in [
        "s = \"{$src}\"",
        "out = $(echo {$src})",
        "ok = \"x\" in /{$src}/",
    ] {
        assert_regular_mode_error(source, "$src", "map or awk mode");
    }
}

#[test]
fn rejects_awk_field_indices_in_interpolation_contexts_regular_mode() {
    for (source, token) in [
        ("s = \"{$1}\"", "$1"),
        ("out = $(echo {$0})", "$0"),
        ("ok = \"x\" in /{$1}/", "$1"),
    ] {
        assert_regular_mode_error(source, token, "--awk");
    }
}

#[test]
fn rejects_reserved_names_in_nested_format_spec() {
    assert_regular_mode_error("s = \"{value:{$n}.{prec}f}\"", "$n", "--awk");
    assert_regular_mode_error("s = \"{value:{$src}.{prec}f}\"", "$src", "map or awk mode");
    assert_regular_mode_error("s = \"{value:{$fd}.{prec}f}\"", "$fd", "--map");
}

#[test]
fn rejects_reserved_names_in_list_comp_positions() {
    assert_regular_mode_error("items = [$n for n in nums]", "$n", "--awk");
    assert_regular_mode_error("items = [n for n in $text]", "$text", "--map");
    assert_regular_mode_error(
        "items = [n for n in nums if $src]",
        "$src",
        "map or awk mode",
    );
}

#[test]
fn rejects_reserved_names_in_dict_comp_positions() {
    assert_regular_mode_error("lookup = %{$n: n for n in nums}", "$n", "--awk");
    assert_regular_mode_error("lookup = %{n: $fd for n in nums}", "$fd", "--map");
    assert_regular_mode_error("lookup = %{n: n for n in $text}", "$text", "--map");
    assert_regular_mode_error(
        "lookup = %{n: n for n in nums if $src}",
        "$src",
        "map or awk mode",
    );
}

#[test]
fn map_allows_map_vars_in_nested_expr_contexts() {
    for source in [
        "s = \"{$src}\"",
        "out = $(echo {$text})",
        "ok = \"x\" in /{$src}/",
        "items = [$src for n in $text if $fd]",
        "lookup = %{$src: $fd for n in $text if $src}",
    ] {
        parse_map(source).expect("map mode source should parse");
    }
}

#[test]
fn map_rejects_awk_vars_in_nested_expr_contexts() {
    for source in [
        "items = [$n for n in nums if n > 0]",
        "items = [n for n in nums if $n]",
        "s = \"{$n}\"",
        "s = \"{$1}\"",
        "ok = \"x\" in /{$n}/",
        "ok = \"x\" in /{$1}/",
        "out = $(echo {$0})",
        "x = items[$1]",
        "x = risky():$n?",
    ] {
        let token = if source.contains("$1") {
            "$1"
        } else if source.contains("$0") {
            "$0"
        } else {
            "$n"
        };
        assert_map_mode_error(source, token, "--awk");
    }
}

#[test]
fn map_rejects_awk_names_in_assignment_target_positions() {
    for (source, token) in [
        ("items[$n] = 1", "$n"),
        ("items[$1] = 1", "$1"),
        ("items[$n] += 1", "$n"),
        ("items[$1]++", "$1"),
        ("++items[$n]", "$n"),
    ] {
        assert_map_mode_error(source, token, "--awk");
    }
}

#[test]
fn map_begin_end_rejects_map_and_awk_vars_comprehensively() {
    for (source, token) in [
        ("BEGIN { print($fd) }\nprint($src)", "$fd"),
        ("END { print($text) }\nprint($src)", "$text"),
        ("BEGIN { print($0) }\nprint($src)", "$0"),
        ("BEGIN { print($n) }\nprint($src)", "$n"),
        ("BEGIN { print($fn) }\nprint($src)", "$fn"),
        ("BEGIN { print($m) }\nprint($src)", "$m"),
        ("BEGIN { print($f) }\nprint($src)", "$f"),
        ("BEGIN { print($1) }\nprint($src)", "$1"),
    ] {
        let err = parse_map(source).expect_err("BEGIN/END variables should be rejected");
        let message = err.to_string();
        assert!(message.contains(token), "{source:?} => {message:?}");
    }
}

#[test]
fn program_begin_end_rejects_additional_reserved_vars() {
    for (source, token) in [
        ("BEGIN { print($fd) }", "$fd"),
        ("END { print($text) }", "$text"),
        ("BEGIN { print($fn) }", "$fn"),
        ("BEGIN { print($m) }", "$m"),
        ("BEGIN { print($f) }", "$f"),
    ] {
        let err = parse(source).expect_err("BEGIN/END reserved variables should be rejected");
        let message = err.to_string();
        assert!(message.contains(token), "{source:?} => {message:?}");
    }
}
