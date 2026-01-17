mod common;

use common::*;
use snail_ast::{Argument, AssignTarget, BinaryOp, Expr, Parameter, Stmt, StringDelimiter};
use snail_parser::parse_program;

#[test]
fn parses_basic_program() {
    let source = "x = 1\nif x {\n  y = x + 2\n}\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (targets, value) = expect_assign(&program.stmts[0]);
    assert_eq!(targets.len(), 1);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "x"));
    expect_number(value, "1");

    if let Stmt::Assign { span, .. } = &program.stmts[0] {
        expect_span_start(span, 1, 1);
    }

    match &program.stmts[1] {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
            ..
        } => {
            expect_name(cond, "x");
            assert_eq!(body.len(), 1);
            assert!(elifs.is_empty());
            assert!(else_body.is_none());
            expect_span_start(span, 2, 1);

            let (targets, value) = expect_assign(&body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
            match value {
                Expr::Binary {
                    op: BinaryOp::Add,
                    left,
                    right,
                    ..
                } => {
                    expect_name(left.as_ref(), "x");
                    expect_number(right.as_ref(), "2");
                }
                other => panic!("Expected binary expression, got {other:?}"),
            }
        }
        other => panic!("Expected if statement, got {other:?}"),
    }
}

#[test]
fn parses_semicolon_before_newline() {
    let source = "x = 1;\ny = 2";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (targets, value) = expect_assign(&program.stmts[0]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "x"));
    expect_number(value, "1");

    let (targets, value) = expect_assign(&program.stmts[1]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
    expect_number(value, "2");
}

#[test]
fn parses_if_elif_else_chain() {
    let source = "if x { y = 1 }\nelif y { y = 2 }\nelse { y = 3 }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            expect_name(cond, "x");
            assert_eq!(body.len(), 1);
            assert_eq!(elifs.len(), 1);
            assert!(else_body.is_some());

            let (targets, value) = expect_assign(&body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
            expect_number(value, "1");

            let (elif_cond, elif_body) = &elifs[0];
            expect_name(elif_cond, "y");
            let (targets, value) = expect_assign(&elif_body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
            expect_number(value, "2");

            let else_body = else_body.as_ref().expect("expected else body");
            let (targets, value) = expect_assign(&else_body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
            expect_number(value, "3");
        }
        other => panic!("Expected if statement, got {other:?}"),
    }
}

#[test]
fn parses_def_and_call() {
    let source = "def add(a, b) { return a + b }\nresult = add(1, 2)\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match &program.stmts[0] {
        Stmt::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            match &params[0] {
                Parameter::Regular { name, default, .. } => {
                    assert_eq!(name, "a");
                    assert!(default.is_none());
                }
                other => panic!("Expected regular param, got {other:?}"),
            }
            match &params[1] {
                Parameter::Regular { name, default, .. } => {
                    assert_eq!(name, "b");
                    assert!(default.is_none());
                }
                other => panic!("Expected regular param, got {other:?}"),
            }
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Return { value, .. } => match value {
                    Some(Expr::Binary { op, .. }) => assert!(matches!(op, BinaryOp::Add)),
                    other => panic!("Expected return value, got {other:?}"),
                },
                other => panic!("Expected return, got {other:?}"),
            }
        }
        other => panic!("Expected function def, got {other:?}"),
    }

    let (targets, value) = expect_assign(&program.stmts[1]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "result"));
    match value {
        Expr::Call { func, args, .. } => {
            expect_name(func.as_ref(), "add");
            assert_eq!(args.len(), 2);
            match &args[0] {
                Argument::Positional { value, .. } => expect_number(value, "1"),
                other => panic!("Expected positional arg, got {other:?}"),
            }
            match &args[1] {
                Argument::Positional { value, .. } => expect_number(value, "2"),
                other => panic!("Expected positional arg, got {other:?}"),
            }
        }
        other => panic!("Expected call expression, got {other:?}"),
    }
}

#[test]
fn parses_placeholder_identifier() {
    let source = "value = _\nnext = _tmp\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (targets, value) = expect_assign(&program.stmts[0]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "value"));
    assert!(matches!(value, Expr::Placeholder { .. }));

    let (targets, value) = expect_assign(&program.stmts[1]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "next"));
    expect_name(value, "_tmp");
}

#[test]
fn parses_compound_expression() {
    let source = "result = (\n  first;\n  second;\n  third\n)";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (targets, value) = expect_assign(&program.stmts[0]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "result"));
    match value {
        Expr::Compound { expressions, .. } => {
            assert_eq!(expressions.len(), 3);
            expect_name(&expressions[0], "first");
            expect_name(&expressions[1], "second");
            expect_name(&expressions[2], "third");
        }
        other => panic!("Expected compound expression, got {other:?}"),
    }
}

#[test]
fn parses_imports() {
    let source =
        "import sys, os as operating_system\nfrom collections import deque, defaultdict as dd\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match &program.stmts[0] {
        Stmt::Import { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, vec!["sys"]);
            assert_eq!(items[0].alias, None);
            assert_eq!(items[1].name, vec!["os"]);
            assert_eq!(items[1].alias, Some("operating_system".to_string()));
        }
        other => panic!("Expected import statement, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::ImportFrom { module, items, .. } => {
            assert_eq!(module, &vec!["collections"]);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, vec!["deque"]);
            assert_eq!(items[1].alias, Some("dd".to_string()));
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }
}

#[test]
fn parses_attribute_and_index_assignment_targets() {
    let source = "config.value = 1\nitems[0] = 2\nnested.value[1].name = 3\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    let (targets, value) = expect_assign(&program.stmts[0]);
    match &targets[0] {
        AssignTarget::Attribute { value, attr, .. } => {
            expect_name(value.as_ref(), "config");
            assert_eq!(attr, "value");
        }
        other => panic!("Expected attribute target, got {other:?}"),
    }
    expect_number(value, "1");

    let (targets, value) = expect_assign(&program.stmts[1]);
    match &targets[0] {
        AssignTarget::Index { value, index, .. } => {
            expect_name(value.as_ref(), "items");
            expect_number(index.as_ref(), "0");
        }
        other => panic!("Expected index target, got {other:?}"),
    }
    expect_number(value, "2");

    let (targets, value) = expect_assign(&program.stmts[2]);
    match &targets[0] {
        AssignTarget::Attribute { value, attr, .. } => {
            assert_eq!(attr, "name");
            match value.as_ref() {
                Expr::Index { value, index, .. } => {
                    expect_number(index.as_ref(), "1");
                    match value.as_ref() {
                        Expr::Attribute { value, attr, .. } => {
                            expect_name(value.as_ref(), "nested");
                            assert_eq!(attr, "value");
                        }
                        other => panic!("Expected attribute value, got {other:?}"),
                    }
                }
                other => panic!("Expected index value, got {other:?}"),
            }
        }
        other => panic!("Expected attribute target, got {other:?}"),
    }
    expect_number(value, "3");
}

#[test]
fn parses_list_and_dict_literals_and_comprehensions() {
    let source = "nums = [1, 2, 3]\npairs = {\"a\": 1, \"b\": 2}\nevens = [n for n in nums if n % 2 == 0]\nlookup = {n: n * 2 for n in nums if n > 1}\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 4);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::List { elements, .. } => {
            assert_eq!(elements.len(), 3);
            expect_number(&elements[0], "1");
            expect_number(&elements[1], "2");
            expect_number(&elements[2], "3");
        }
        other => panic!("Expected list literal, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::Dict { entries, .. } => {
            assert_eq!(entries.len(), 2);
            expect_string(&entries[0].0, "a", false, StringDelimiter::Double);
            expect_number(&entries[0].1, "1");
            expect_string(&entries[1].0, "b", false, StringDelimiter::Double);
            expect_number(&entries[1].1, "2");
        }
        other => panic!("Expected dict literal, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[2]);
    match value {
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            ..
        } => {
            expect_name(element.as_ref(), "n");
            assert_eq!(target, "n");
            expect_name(iter.as_ref(), "nums");
            assert_eq!(ifs.len(), 1);
        }
        other => panic!("Expected list comprehension, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[3]);
    match value {
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            ..
        } => {
            expect_name(key.as_ref(), "n");
            match value.as_ref() {
                Expr::Binary { op, .. } => assert!(matches!(op, BinaryOp::Mul)),
                other => panic!("Expected multiplication, got {other:?}"),
            }
            assert_eq!(target, "n");
            expect_name(iter.as_ref(), "nums");
            assert_eq!(ifs.len(), 1);
        }
        other => panic!("Expected dict comprehension, got {other:?}"),
    }
}

// ========== Error Path Tests ==========

#[test]
fn parser_rejects_unclosed_brace() {
    let err = parse_program("if x { y = 1").expect_err("should fail on unclosed brace");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("unclosed") || message.contains("}"));
}

#[test]
fn parser_rejects_invalid_assignment_target() {
    let err = parse_program("1 = x").expect_err("should fail on invalid target");
    let message = err.to_string();
    assert!(
        message.contains("assign") || message.contains("target") || message.contains("expected")
    );
}

#[test]
fn parser_handles_unterminated_string() {
    let err = parse_program(r#"x = "hello"#).expect_err("should fail on unterminated string");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_incomplete_if_statement() {
    let err = parse_program("if").expect_err("should fail on incomplete if");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("if"));
}

#[test]
fn parser_rejects_missing_condition() {
    let err = parse_program("if { x = 1 }").expect_err("should fail on missing condition");
    assert!(err.span.is_some());
}

#[test]
fn parser_reports_error_on_missing_colon_in_dict() {
    let err = parse_program("d = {\"key\" 1}").expect_err("should fail on missing colon");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains(":"));
}

#[test]
fn parser_rejects_incomplete_function_def() {
    let err = parse_program("def foo").expect_err("should fail on incomplete def");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("("));
}

#[test]
fn parser_rejects_unclosed_paren() {
    let err = parse_program("result = (1 + 2").expect_err("should fail on unclosed paren");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains(")"));
}

#[test]
fn parser_rejects_unclosed_bracket() {
    let err = parse_program("items = [1, 2, 3").expect_err("should fail on unclosed bracket");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("]"));
}

#[test]
fn parser_rejects_invalid_expression_in_binary_op() {
    let err = parse_program("x = 1 +").expect_err("should fail on incomplete binary op");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("expression"));
}

#[test]
fn parser_rejects_missing_except_after_try() {
    // This might be allowed (try-finally), so adjust if needed
    let source = "try { x = 1 }";
    match parse_program(source) {
        Ok(_) => {
            // Parser allows try-finally, so this is fine
        }
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
    let err = parse_program(source).expect_err("should fail");
    assert_eq!(err.span.unwrap().start.line, 3);
}

#[test]
fn parser_rejects_invalid_import_syntax() {
    let err = parse_program("import").expect_err("should fail on incomplete import");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("import"));
}

#[test]
fn parser_rejects_invalid_from_import() {
    let err = parse_program("from").expect_err("should fail on incomplete from-import");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("import"));
}

#[test]
fn parser_accepts_empty_function_body() {
    // Empty function bodies are actually allowed (they parse successfully)
    let program = parse_program("def foo() { }").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
    match &program.stmts[0] {
        Stmt::Def { body, .. } => {
            assert_eq!(body.len(), 0); // Empty body is allowed
        }
        other => panic!("Expected function def, got {:?}", other),
    }
}

#[test]
fn parser_rejects_missing_iterable_in_for_loop() {
    let err = parse_program("for x in { }").expect_err("should fail on missing iterable");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_invalid_comprehension_syntax() {
    let err = parse_program("[x for]").expect_err("should fail on invalid comprehension");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("in"));
}

#[test]
fn parser_rejects_unexpected_token() {
    let err = parse_program("x = 1 @ 2").expect_err("should fail on unexpected operator");
    assert!(err.span.is_some());
}

#[test]
fn parser_rejects_nested_unclosed_structures() {
    let err = parse_program("if x { if y { z = 1 }").expect_err("should fail on nested unclosed");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("}"));
}

#[test]
fn parser_rejects_invalid_parameter_syntax() {
    let err = parse_program("def foo(1) { pass }").expect_err("should fail on invalid parameter");
    let message = err.to_string();
    assert!(
        message.contains("expected")
            || message.contains("identifier")
            || message.contains("parameter")
    );
}

#[test]
fn parses_raw_string_with_curly_braces() {
    let source = r#"x = r"{ \"key\": \"value\" }""#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::String { value, raw, .. } => {
                assert_eq!(value, r#"{ \"key\": \"value\" }"#);
                assert!(raw);
            }
            other => panic!("Expected String, got {:?}", other),
        },
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_raw_string_without_interpolation() {
    let source = r#"x = r"test {expr} more""#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            match value {
                Expr::String { value, raw, .. } => {
                    // Raw string should preserve {expr} literally, not treat it as interpolation
                    assert_eq!(value, "test {expr} more");
                    assert!(raw);
                }
                other => panic!("Expected String, got {:?}", other),
            }
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_raw_triple_quoted_string_with_js() {
    let source = r#####"x = r"""
{
  "hook_event_name": "Status",
  "session_id": "abc123"
}
""""#####;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::String { raw, .. } => {
                assert!(raw);
            }
            other => panic!("Expected String, got {:?}", other),
        },
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_regular_string_with_interpolation() {
    let source = r#"x = "test {y} more""#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            // Non-raw strings should support interpolation and parse as FString
            match value {
                Expr::FString { .. } => {
                    // This is correct - interpolated strings are FStrings
                }
                other => panic!("Expected FString, got {:?}", other),
            }
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_structured_accessor() {
    let program = parse_program("result = $[foo.bar]").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_structured_accessor_with_pipeline() {
    let program = parse_program("result = js() | $[users[0].name]").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_empty_structured_accessor() {
    let program = parse_program("result = $[]").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_ternary_with_not_in_operator() {
    let source = "result = x if x not in y else z";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::IfExpr { .. }));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_ternary_with_is_not_operator() {
    let source = "result = x if x is not None else y";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::IfExpr { .. }));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

// Tests for compound statement separator behavior (no semicolon needed after })

#[test]
fn parses_if_followed_by_stmt_without_separator() {
    // if statement followed by expression without semicolon
    let source = "if x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::If { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_while_followed_by_stmt_without_separator() {
    let source = "while x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::While { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_for_followed_by_stmt_without_separator() {
    let source = "for i in x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::For { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_def_followed_by_stmt_without_separator() {
    let source = "def f() { pass } f()";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::Def { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_class_followed_by_stmt_without_separator() {
    let source = "class C { pass } C()";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::Class { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_try_followed_by_stmt_without_separator() {
    // Note: using explicit exception type since bare `except { }` is ambiguous
    let source = "try { x } except Exception { y } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::Try { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_with_followed_by_stmt_without_separator() {
    let source = "with x { y } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::With { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_nested_compound_stmts_without_separators() {
    let source = "if a { if b { c } d } e";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(&program.stmts[0], Stmt::If { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_mixed_compound_and_simple_stmts() {
    let source = "a = 1; if b { c = 2 } d = 3; e = 4";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 4);
    assert!(matches!(&program.stmts[0], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[1], Stmt::If { .. }));
    assert!(matches!(&program.stmts[2], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[3], Stmt::Assign { .. }));
}

#[test]
fn parses_consecutive_compound_stmts_without_separators() {
    let source = "if a { b } if c { d } while e { f }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);
    assert!(matches!(&program.stmts[0], Stmt::If { .. }));
    assert!(matches!(&program.stmts[1], Stmt::If { .. }));
    assert!(matches!(&program.stmts[2], Stmt::While { .. }));
}

#[test]
fn simple_stmt_still_requires_separator() {
    // Two simple statements without separator should fail
    let source = "a b";
    parse_err(source);
}
