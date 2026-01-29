mod common;

use common::*;
use snail_ast::{
    BinaryOp, CompareOp, Expr, RegexPattern, Stmt, SubprocessKind, SubprocessPart, UnaryOp,
};

#[test]
fn parses_if_expression() {
    let source = "value = 1 if flag else 2";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            expect_name(test.as_ref(), "flag");
            expect_number(body.as_ref(), "1");
            expect_number(orelse.as_ref(), "2");
        }
        other => panic!("Expected if expression, got {other:?}"),
    }
}

#[test]
fn parses_yield_expressions() {
    let source = "def gen(items) { yield; yield 1; yield from items }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Def { body, .. } => {
            assert_eq!(body.len(), 3);
            match &body[0] {
                Stmt::Expr { value, .. } => match value {
                    Expr::Yield { value, .. } => assert!(value.is_none()),
                    other => panic!("Expected yield expression, got {other:?}"),
                },
                other => panic!("Expected expression statement, got {other:?}"),
            }
            match &body[1] {
                Stmt::Expr { value, .. } => match value {
                    Expr::Yield { value, .. } => match value.as_deref() {
                        Some(expr) => expect_number(expr, "1"),
                        None => panic!("Expected yield value"),
                    },
                    other => panic!("Expected yield expression, got {other:?}"),
                },
                other => panic!("Expected expression statement, got {other:?}"),
            }
            match &body[2] {
                Stmt::Expr { value, .. } => match value {
                    Expr::YieldFrom { expr, .. } => expect_name(expr.as_ref(), "items"),
                    other => panic!("Expected yield from expression, got {other:?}"),
                },
                other => panic!("Expected expression statement, got {other:?}"),
            }
        }
        other => panic!("Expected function def, got {other:?}"),
    }
}

#[test]
fn parses_compact_exception_expression() {
    let source = "value = risky()?\nfallback = risky():$e?\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::TryExpr { expr, fallback, .. } => {
            assert!(fallback.is_none());
            match expr.as_ref() {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "risky"),
                other => panic!("Expected risky() call, got {other:?}"),
            }
        }
        other => panic!("Expected try expression, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::TryExpr { expr, fallback, .. } => {
            match expr.as_ref() {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "risky"),
                other => panic!("Expected risky() call, got {other:?}"),
            }
            match fallback.as_deref() {
                Some(Expr::Name { name, .. }) => assert_eq!(name, "$e"),
                other => panic!("Expected $e fallback, got {other:?}"),
            }
        }
        other => panic!("Expected try expression, got {other:?}"),
    }
}

#[test]
fn compact_try_binds_before_infix_and_accessors() {
    let source = "result = a + b?\nchained = call()? .attr[0]\nleft = value? + other\n";
    let program = parse_ok(source);
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
    let program = parse_ok("result = a:0? + 1");
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
    let source = "name = \"snail\"\nout = $(echo {name})\ncode = @(echo ok)\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::Subprocess { kind, parts, .. } => {
            assert!(matches!(kind, SubprocessKind::Capture));
            assert!(
                parts
                    .iter()
                    .any(|part| matches!(part, SubprocessPart::Expr(_)))
            );
        }
        other => panic!("Expected subprocess capture, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[2]);
    match value {
        Expr::Subprocess { kind, parts, .. } => {
            assert!(matches!(kind, SubprocessKind::Status));
            assert!(
                parts
                    .iter()
                    .any(|part| matches!(part, SubprocessPart::Text(text) if text.contains("ok")))
            );
        }
        other => panic!("Expected subprocess status, got {other:?}"),
    }
}

#[test]
fn parses_regex_expressions() {
    let source = "text = \"value\"\nfound = text in /val(.)/\ncompiled = /abc/\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::RegexMatch { value, pattern, .. } => {
            expect_name(value.as_ref(), "text");
            match pattern {
                RegexPattern::Literal(pattern) => assert_eq!(pattern, "val(.)"),
                other => panic!("Expected literal regex, got {other:?}"),
            }
        }
        other => panic!("Expected regex match, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[2]);
    match value {
        Expr::Regex { pattern, .. } => match pattern {
            RegexPattern::Literal(pattern) => assert_eq!(pattern, "abc"),
            other => panic!("Expected literal regex, got {other:?}"),
        },
        other => panic!("Expected regex literal, got {other:?}"),
    }
}

#[test]
fn parses_not_in_regex_as_negated_match() {
    let source = "result = text not in /pattern/";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => match value {
            Expr::Unary { op, expr, .. } => {
                assert!(matches!(op, UnaryOp::Not));
                assert!(matches!(expr.as_ref(), Expr::RegexMatch { .. }));
            }
            other => panic!("Expected negated regex match, got {other:?}"),
        },
        other => panic!("Expected assignment, got {other:?}"),
    }
}

#[test]
fn parses_structured_accessor() {
    let program = parse_ok("result = $[foo.bar]");
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::StructuredAccessor { query, .. } => assert_eq!(query, "foo.bar"),
        other => panic!("Expected structured accessor, got {other:?}"),
    }
}

#[test]
fn parses_structured_accessor_with_pipeline() {
    let program = parse_ok("result = json() | $[users[0].name]");
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::Binary {
            op, left, right, ..
        } => {
            assert!(matches!(op, BinaryOp::Pipeline));
            match right.as_ref() {
                Expr::StructuredAccessor { query, .. } => assert_eq!(query, "users[0].name"),
                other => panic!("Expected structured accessor, got {other:?}"),
            }
            match left.as_ref() {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "json"),
                other => panic!("Expected json() call, got {other:?}"),
            }
        }
        other => panic!("Expected pipeline expression, got {other:?}"),
    }
}

#[test]
fn parses_empty_structured_accessor() {
    let program = parse_ok("result = $[]");
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::StructuredAccessor { query, .. } => assert_eq!(query, ""),
        other => panic!("Expected structured accessor, got {other:?}"),
    }
}

#[test]
fn parses_ternary_with_not_in_operator() {
    let source = "result = x if x not in y else z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::IfExpr { test, .. } => match test.as_ref() {
            Expr::Compare { ops, .. } => assert_eq!(ops, &[CompareOp::NotIn]),
            other => panic!("Expected comparison, got {other:?}"),
        },
        other => panic!("Expected if expression, got {other:?}"),
    }
}

#[test]
fn parses_ternary_with_is_not_operator() {
    let source = "result = x if x is not None else y";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::IfExpr { test, .. } => match test.as_ref() {
            Expr::Compare { ops, .. } => assert_eq!(ops, &[CompareOp::IsNot]),
            other => panic!("Expected comparison, got {other:?}"),
        },
        other => panic!("Expected if expression, got {other:?}"),
    }
}

#[test]
fn parses_boolean_precedence() {
    let program = parse_ok("result = a or b and c");
    let (_, value) = expect_assign(&program.stmts[0]);

    match value {
        Expr::Binary {
            op, left, right, ..
        } => {
            assert!(matches!(op, BinaryOp::Or));
            expect_name(left.as_ref(), "a");
            match right.as_ref() {
                Expr::Binary {
                    op, left, right, ..
                } => {
                    assert!(matches!(op, BinaryOp::And));
                    expect_name(left.as_ref(), "b");
                    expect_name(right.as_ref(), "c");
                }
                other => panic!("Expected and expression, got {other:?}"),
            }
        }
        other => panic!("Expected or expression, got {other:?}"),
    }
}

#[test]
fn parses_comparison_chain() {
    let program = parse_ok("result = a < b < c");
    let (_, value) = expect_assign(&program.stmts[0]);

    match value {
        Expr::Compare {
            left,
            ops,
            comparators,
            ..
        } => {
            expect_name(left.as_ref(), "a");
            assert_eq!(ops, &[CompareOp::Lt, CompareOp::Lt]);
            assert_eq!(comparators.len(), 2);
        }
        other => panic!("Expected comparison chain, got {other:?}"),
    }
}

#[test]
fn parses_call_attribute_index_chain() {
    let program = parse_ok("value = foo(1).bar[0]");
    let (_, value) = expect_assign(&program.stmts[0]);

    match value {
        Expr::Index { value, index, .. } => {
            expect_number(index.as_ref(), "0");
            match value.as_ref() {
                Expr::Attribute { value, attr, .. } => {
                    assert_eq!(attr, "bar");
                    match value.as_ref() {
                        Expr::Call { func, .. } => expect_name(func.as_ref(), "foo"),
                        other => panic!("Expected foo() call, got {other:?}"),
                    }
                }
                other => panic!("Expected attribute access, got {other:?}"),
            }
        }
        other => panic!("Expected index access, got {other:?}"),
    }
}

#[test]
fn parses_numeric_attribute_access() {
    let program = parse_ok("value = match.1");
    let (_, value) = expect_assign(&program.stmts[0]);

    match value {
        Expr::Attribute { value, attr, .. } => {
            expect_name(value.as_ref(), "match");
            assert_eq!(attr, "1");
        }
        other => panic!("Expected attribute access, got {other:?}"),
    }
}
