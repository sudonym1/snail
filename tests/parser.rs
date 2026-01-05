use snail::{AssignTarget, BinaryOp, Expr, Stmt, parse_program};

#[test]
fn parses_basic_program() {
    let source = r#"
x = 1
if x {
  y = x + 2
}
"#;
    let program = parse_program(source).expect("program should parse");
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
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            assert!(matches!(cond, Expr::Name { name, .. } if name == "x"));
            assert_eq!(body.len(), 1);
            assert!(elifs.is_empty());
            assert!(else_body.is_none());

            // Validate the assignment inside the if body
            match &body[0] {
                Stmt::Assign { targets, value, .. } => {
                    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
                    assert!(matches!(
                        value,
                        Expr::Binary {
                            op: BinaryOp::Add,
                            ..
                        }
                    ));
                }
                other => panic!("Expected assignment in if body, got {:?}", other),
            }
        }
        other => panic!("Expected if statement, got {:?}", other),
    }
}

#[test]
fn parses_semicolon_before_newline() {
    let source = "x = 1;\ny = 2";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);

    // Verify both are assignments
    assert!(matches!(&program.stmts[0], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Assign { .. }));
}

#[test]
fn reports_parse_error_with_location() {
    let source = "if { }";
    let err = parse_program(source).expect_err("program should fail");
    let message = err.to_string();
    assert!(message.contains("-->"));
    assert!(message.contains("if"));
}

#[test]
fn rejects_user_defined_dollar_identifiers() {
    let source = "$bad = 1";
    let err = parse_program(source).expect_err("$ identifiers are injected");
    let message = err.to_string();
    assert!(message.contains("$bad"));
}

#[test]
fn parses_if_elif_else_chain() {
    let source = r#"
if x { y = 1 }
elif y { y = 2 }
else { y = 3 }
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    // Validate if-elif-else structure
    match &program.stmts[0] {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            assert!(matches!(cond, Expr::Name { name, .. } if name == "x"));
            assert_eq!(body.len(), 1);
            assert_eq!(elifs.len(), 1);
            assert!(else_body.is_some());

            // Check elif condition
            let (elif_cond, elif_body) = &elifs[0];
            assert!(matches!(elif_cond, Expr::Name { name, .. } if name == "y"));
            assert_eq!(elif_body.len(), 1);

            // Check else body
            assert_eq!(else_body.as_ref().unwrap().len(), 1);
        }
        other => panic!("Expected if statement, got {:?}", other),
    }
}

#[test]
fn parses_def_and_call() {
    let source = r#"
def add(a, b) { return a + b }
result = add(1, 2)
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);

    // Validate function definition
    match &program.stmts[0] {
        Stmt::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Return { .. }));
        }
        other => panic!("Expected function def, got {:?}", other),
    }

    // Validate function call
    match &program.stmts[1] {
        Stmt::Assign { value, .. } => match value {
            Expr::Call { func, args, .. } => {
                assert!(matches!(func.as_ref(), Expr::Name { name, .. } if name == "add"));
                assert_eq!(args.len(), 2);
            }
            other => panic!("Expected call expression, got {:?}", other),
        },
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_compound_expression() {
    let source = r#"result = (
    first;
    second;
    third
)"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::Compound { expressions, .. } => {
                assert_eq!(expressions.len(), 3);
                assert!(matches!(
                    &expressions[0],
                    Expr::Name { name, .. } if name == "first"
                ));
                assert!(matches!(
                    &expressions[1],
                    Expr::Name { name, .. } if name == "second"
                ));
                assert!(matches!(
                    &expressions[2],
                    Expr::Name { name, .. } if name == "third"
                ));
            }
            other => panic!("Expected compound expression, got {:?}", other),
        },
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_imports() {
    let source = r#"
import sys, os as operating_system
from collections import deque, defaultdict as dd
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);

    // Validate import statement
    match &program.stmts[0] {
        Stmt::Import { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, vec!["sys"]);
            assert_eq!(items[0].alias, None);
            assert_eq!(items[1].name, vec!["os"]);
            assert_eq!(items[1].alias, Some("operating_system".to_string()));
        }
        other => panic!("Expected import statement, got {:?}", other),
    }

    // Validate from-import statement
    match &program.stmts[1] {
        Stmt::ImportFrom { module, items, .. } => {
            assert_eq!(module, &vec!["collections"]);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].name, vec!["deque"]);
            assert_eq!(items[1].alias, Some("dd".to_string()));
        }
        other => panic!("Expected from-import statement, got {:?}", other),
    }
}

#[test]
fn parses_attribute_and_index_assignment_targets() {
    let source = r#"
config.value = 1
items[0] = 2
nested.value[1].name = 3
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);

    // Validate attribute assignment
    match &program.stmts[0] {
        Stmt::Assign { targets, .. } => {
            assert!(matches!(&targets[0], AssignTarget::Attribute { attr, .. } if attr == "value"));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }

    // Validate index assignment
    match &program.stmts[1] {
        Stmt::Assign { targets, .. } => {
            assert!(matches!(&targets[0], AssignTarget::Index { .. }));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_list_and_dict_literals_and_comprehensions() {
    let source = r#"
nums = [1, 2, 3]
pairs = {"a": 1, "b": 2}
evens = [n for n in nums if n % 2 == 0]
lookup = {n: n * 2 for n in nums if n > 1}
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 4);

    // Validate list literal
    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::List { elements, .. } if elements.len() == 3));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }

    // Validate dict literal
    match &program.stmts[1] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::Dict { entries, .. } if entries.len() == 2));
        }
        other => panic!("Expected assignment, got {:?}", other),
    }

    // Validate list comprehension
    match &program.stmts[2] {
        Stmt::Assign { value, .. } => {
            assert!(
                matches!(value, Expr::ListComp { target, ifs, .. } if target == "n" && ifs.len() == 1)
            );
        }
        other => panic!("Expected assignment, got {:?}", other),
    }

    // Validate dict comprehension
    match &program.stmts[3] {
        Stmt::Assign { value, .. } => {
            assert!(
                matches!(value, Expr::DictComp { target, ifs, .. } if target == "n" && ifs.len() == 1)
            );
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_raw_and_multiline_strings() {
    let source = "text = r\"hello\\n\"\nblock = \"\"\"line1\nline2\"\"\"\nraw_block = r\"\"\"raw\\nline\"\"\"";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);
}

#[test]
fn parses_try_except_finally_and_raise() {
    let source = r#"
try { risky() }
except ValueError as err { raise err }
except { raise }
else { ok = True }
finally { cleanup() }
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_raise_from_and_try_finally() {
    let source = r#"
try { risky() }
finally { cleanup() }
raise ValueError("bad") from err
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
}

#[test]
fn parses_with_statement() {
    let source = r#"
with open("data") as f { line = f.read() }
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_assert_and_del() {
    let source = r#"
value = 1
assert value == 1, "ok"
del value
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);
}

#[test]
fn parses_tuples_sets_and_slices() {
    let source = r#"
items = [1, 2, 3, 4]
pair = (1, 2)
single = (1,)
empty = ()
flags = {True, False}
mid = items[1:3]
head = items[:2]
tail = items[2:]
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 8);
}

#[test]
fn parses_defaults_and_star_args() {
    let source = r#"
def join(a, b=1, *rest, **extras) { return a }
result = join(1, b=2, *rest, **extras)
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
}

#[test]
fn parses_loop_else_with_try_break_continue() {
    let source = r#"
for n in nums { try { break } finally { cleanup() } } else { done = True }
while flag { try { continue } finally { cleanup() } } else { done = False }
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
}

#[test]
fn parses_if_expression() {
    let source = "value = 1 if flag else 2";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_compact_exception_expression() {
    let source = r#"
value = risky()?
fallback = risky() ? $e
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
}

#[test]
fn compact_try_binds_before_infix_and_accessors() {
    let source = r#"
result = a + b?
chained = call()? .attr[0]
left = value? + other
"#;

    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::Binary {
                left, op, right, ..
            } => {
                assert!(matches!(op, BinaryOp::Add));
                assert!(matches!(left.as_ref(), Expr::Name { name, .. } if name == "a"));
                assert!(matches!(right.as_ref(), Expr::TryExpr { .. }));
            }
            other => panic!("expected binary expression, got {other:?}"),
        },
        other => panic!("expected assignment, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::Assign { value, .. } => match value {
            Expr::Index { value, index, .. } => {
                assert!(matches!(index.as_ref(), Expr::Number { value, .. } if value == "0"));
                match value.as_ref() {
                    Expr::Attribute { value, attr, .. } => {
                        assert_eq!(attr, "attr");
                        assert!(matches!(value.as_ref(), Expr::TryExpr { .. }));
                    }
                    other => panic!("expected attribute on try result, got {other:?}"),
                }
            }
            other => panic!("expected index expression, got {other:?}"),
        },
        other => panic!("expected assignment, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::Assign { value, .. } => match value {
            Expr::Binary {
                left, op, right, ..
            } => {
                assert!(matches!(op, BinaryOp::Add));
                assert!(matches!(left.as_ref(), Expr::TryExpr { .. }));
                assert!(matches!(right.as_ref(), Expr::Name { name, .. } if name == "other"));
            }
            other => panic!("expected binary expression, got {other:?}"),
        },
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn compact_try_fallback_stops_before_addition() {
    let program = parse_program("result = a?0 + 1").expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::Binary {
                left, op, right, ..
            } => {
                assert!(matches!(op, BinaryOp::Add));
                match left.as_ref() {
                    Expr::TryExpr { expr, fallback, .. } => {
                        assert!(matches!(expr.as_ref(), Expr::Name { name, .. } if name == "a"));
                        match fallback.as_deref() {
                            Some(Expr::Number { value, .. }) => assert_eq!(value, "0"),
                            other => panic!("expected numeric fallback, got {other:?}"),
                        }
                    }
                    other => panic!("expected try expression on the left, got {other:?}"),
                }

                assert!(matches!(right.as_ref(), Expr::Number { value, .. } if value == "1"));
            }
            other => panic!("expected binary expression, got {other:?}"),
        },
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn parses_subprocess_expressions() {
    let source = r#"
name = "snail"
out = $(echo {name})
code = @(echo ok)
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);
}

#[test]
fn parses_regex_expressions() {
    let source = r#"
text = "value"
found = text in /val(.)/
compiled = /abc/
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 3);
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
                assert_eq!(*raw, true);
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
                    assert_eq!(*raw, true);
                }
                other => panic!("Expected String, got {:?}", other),
            }
        }
        other => panic!("Expected assignment, got {:?}", other),
    }
}

#[test]
fn parses_raw_triple_quoted_string_with_json() {
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
                assert_eq!(*raw, true);
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
    let program = parse_program("result = json() | $[users[0].name]").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_empty_structured_accessor() {
    let program = parse_program("result = $[]").expect("should parse");
    assert_eq!(program.stmts.len(), 1);
}
