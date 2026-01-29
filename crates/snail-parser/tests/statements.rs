mod common;

use common::*;
use snail_ast::{Argument, AssignTarget, Expr, Parameter, Stmt, StringDelimiter};

#[test]
fn parses_try_except_finally_and_raise() {
    let source = "try { risky() }\nexcept ValueError as err { raise err }\nexcept { raise }\nelse { ok = True }\nfinally { cleanup() }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            assert_eq!(body.len(), 1);
            match expect_expr_stmt(&body[0]) {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "risky"),
                other => panic!("Expected risky() call, got {other:?}"),
            }

            assert_eq!(handlers.len(), 2);
            let handler = &handlers[0];
            match &handler.type_name {
                Some(Expr::Name { name, .. }) => assert_eq!(name, "ValueError"),
                other => panic!("Expected ValueError handler, got {other:?}"),
            }
            assert_eq!(handler.name.as_deref(), Some("err"));
            match &handler.body[0] {
                Stmt::Raise { value, from, .. } => {
                    expect_name(value.as_ref().expect("raise value"), "err");
                    assert!(from.is_none());
                }
                other => panic!("Expected raise, got {other:?}"),
            }

            let handler = &handlers[1];
            assert!(handler.type_name.is_none());
            assert!(handler.name.is_none());
            match &handler.body[0] {
                Stmt::Raise { value, from, .. } => {
                    assert!(value.is_none());
                    assert!(from.is_none());
                }
                other => panic!("Expected bare raise, got {other:?}"),
            }

            let else_body = else_body.as_ref().expect("expected else body");
            let (_, value) = expect_assign(&else_body[0]);
            match value {
                Expr::Bool { value, .. } => assert!(*value),
                other => panic!("Expected True, got {other:?}"),
            }

            let finally_body = finally_body.as_ref().expect("expected finally body");
            match expect_expr_stmt(&finally_body[0]) {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "cleanup"),
                other => panic!("Expected cleanup() call, got {other:?}"),
            }
        }
        other => panic!("Expected try statement, got {other:?}"),
    }
}

#[test]
fn parses_raise_from_and_try_finally() {
    let source = "try { risky() }\nfinally { cleanup() }\nraise ValueError(\"bad\") from err\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match &program.stmts[0] {
        Stmt::Try {
            handlers,
            finally_body,
            ..
        } => {
            assert!(handlers.is_empty());
            let finally_body = finally_body.as_ref().expect("expected finally body");
            match expect_expr_stmt(&finally_body[0]) {
                Expr::Call { func, .. } => expect_name(func.as_ref(), "cleanup"),
                other => panic!("Expected cleanup() call, got {other:?}"),
            }
        }
        other => panic!("Expected try statement, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::Raise { value, from, .. } => {
            match value.as_ref().expect("raise value") {
                Expr::Call { func, args, .. } => {
                    expect_name(func.as_ref(), "ValueError");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected ValueError call, got {other:?}"),
            }
            expect_name(from.as_ref().expect("raise from"), "err");
        }
        other => panic!("Expected raise, got {other:?}"),
    }
}

#[test]
fn parses_with_statement() {
    let source = "with open(\"data\") as f { line = f.read() }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::With { items, body, .. } => {
            assert_eq!(items.len(), 1);
            let item = &items[0];
            match &item.context {
                Expr::Call { func, args, .. } => {
                    expect_name(func.as_ref(), "open");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected open() call, got {other:?}"),
            }
            match item.target.as_ref().expect("with target") {
                AssignTarget::Name { name, .. } => assert_eq!(name, "f"),
                other => panic!("Expected name target, got {other:?}"),
            }

            let (targets, value) = expect_assign(&body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "line"));
            match value {
                Expr::Call { func, .. } => match func.as_ref() {
                    Expr::Attribute { value, attr, .. } => {
                        expect_name(value.as_ref(), "f");
                        assert_eq!(attr, "read");
                    }
                    other => panic!("Expected attribute call, got {other:?}"),
                },
                other => panic!("Expected call, got {other:?}"),
            }
        }
        other => panic!("Expected with statement, got {other:?}"),
    }
}

#[test]
fn parses_assert_and_del() {
    let source = "value = 1\nassert value == 1, \"ok\"\ndel value\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    let (targets, value) = expect_assign(&program.stmts[0]);
    assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "value"));
    expect_number(value, "1");

    match &program.stmts[1] {
        Stmt::Assert { test, message, .. } => {
            match test {
                Expr::Compare {
                    ops, comparators, ..
                } => {
                    assert_eq!(ops.len(), 1);
                    assert_eq!(comparators.len(), 1);
                }
                other => panic!("Expected comparison, got {other:?}"),
            }
            expect_string(
                message.as_ref().expect("assert message"),
                "ok",
                false,
                StringDelimiter::Double,
            );
        }
        other => panic!("Expected assert, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::Delete { targets, .. } => match &targets[0] {
            AssignTarget::Name { name, .. } => assert_eq!(name, "value"),
            other => panic!("Expected delete target, got {other:?}"),
        },
        other => panic!("Expected delete, got {other:?}"),
    }
}

#[test]
fn parses_tuples_and_slices() {
    let source = "items = [1, 2, 3, 4]\npair = (1, 2)\nsingle = (1,)\nempty = ()\nmid = items[1:3]\nhead = items[:2]\ntail = items[2:]\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 7);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::List { elements, .. } => assert_eq!(elements.len(), 4),
        other => panic!("Expected list, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::Tuple { elements, .. } => assert_eq!(elements.len(), 2),
        other => panic!("Expected tuple, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[2]);
    match value {
        Expr::Tuple { elements, .. } => assert_eq!(elements.len(), 1),
        other => panic!("Expected single tuple, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[3]);
    match value {
        Expr::Tuple { elements, .. } => assert!(elements.is_empty()),
        other => panic!("Expected empty tuple, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[4]);
    match value {
        Expr::Index { value, index, .. } => {
            expect_name(value.as_ref(), "items");
            match index.as_ref() {
                Expr::Slice { start, end, .. } => {
                    expect_number(start.as_ref().expect("slice start"), "1");
                    expect_number(end.as_ref().expect("slice end"), "3");
                }
                other => panic!("Expected slice, got {other:?}"),
            }
        }
        other => panic!("Expected index, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[5]);
    match value {
        Expr::Index { index, .. } => match index.as_ref() {
            Expr::Slice { start, end, .. } => {
                assert!(start.is_none());
                expect_number(end.as_ref().expect("slice end"), "2");
            }
            other => panic!("Expected slice, got {other:?}"),
        },
        other => panic!("Expected index, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[6]);
    match value {
        Expr::Index { index, .. } => match index.as_ref() {
            Expr::Slice { start, end, .. } => {
                expect_number(start.as_ref().expect("slice start"), "2");
                assert!(end.is_none());
            }
            other => panic!("Expected slice, got {other:?}"),
        },
        other => panic!("Expected index, got {other:?}"),
    }
}

#[test]
fn parses_defaults_and_star_args() {
    let source =
        "def join(a, b=1, *rest, **extras) { return a }\nresult = join(1, b=2, *rest, **extras)\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match &program.stmts[0] {
        Stmt::Def { params, .. } => {
            assert_eq!(params.len(), 4);
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
                    expect_number(default.as_ref().expect("default"), "1");
                }
                other => panic!("Expected regular param, got {other:?}"),
            }
            match &params[2] {
                Parameter::VarArgs { name, .. } => assert_eq!(name, "rest"),
                other => panic!("Expected var args, got {other:?}"),
            }
            match &params[3] {
                Parameter::KwArgs { name, .. } => assert_eq!(name, "extras"),
                other => panic!("Expected kw args, got {other:?}"),
            }
        }
        other => panic!("Expected def, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::Call { args, .. } => {
            assert_eq!(args.len(), 4);
            match &args[0] {
                Argument::Positional { value, .. } => expect_number(value, "1"),
                other => panic!("Expected positional arg, got {other:?}"),
            }
            match &args[1] {
                Argument::Keyword { name, value, .. } => {
                    assert_eq!(name, "b");
                    expect_number(value, "2");
                }
                other => panic!("Expected keyword arg, got {other:?}"),
            }
            assert!(matches!(args[2], Argument::Star { .. }));
            assert!(matches!(args[3], Argument::KwStar { .. }));
        }
        other => panic!("Expected call, got {other:?}"),
    }
}

#[test]
fn parses_loop_else_with_try_break_continue() {
    let source = "for n in nums { try { break } finally { cleanup() } } else { done = True }\nwhile flag { try { continue } finally { cleanup() } } else { done = False }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match &program.stmts[0] {
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            match target {
                AssignTarget::Name { name, .. } => assert_eq!(name, "n"),
                other => panic!("Expected name target, got {other:?}"),
            }
            expect_name(iter, "nums");
            match &body[0] {
                Stmt::Try { finally_body, .. } => {
                    let finally_body = finally_body.as_ref().expect("expected finally body");
                    match &finally_body[0] {
                        Stmt::Expr { value, .. } => match value {
                            Expr::Call { func, .. } => expect_name(func.as_ref(), "cleanup"),
                            other => panic!("Expected cleanup() call, got {other:?}"),
                        },
                        other => panic!("Expected expr, got {other:?}"),
                    }
                }
                other => panic!("Expected try body, got {other:?}"),
            }

            let else_body = else_body.as_ref().expect("expected else body");
            let (_, value) = expect_assign(&else_body[0]);
            match value {
                Expr::Bool { value, .. } => assert!(*value),
                other => panic!("Expected True, got {other:?}"),
            }
        }
        other => panic!("Expected for loop, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::While {
            cond,
            body,
            else_body,
            ..
        } => {
            expect_condition_name(cond, "flag");
            match &body[0] {
                Stmt::Try { finally_body, .. } => {
                    let finally_body = finally_body.as_ref().expect("expected finally body");
                    match &finally_body[0] {
                        Stmt::Expr { value, .. } => match value {
                            Expr::Call { func, .. } => expect_name(func.as_ref(), "cleanup"),
                            other => panic!("Expected cleanup() call, got {other:?}"),
                        },
                        other => panic!("Expected expr, got {other:?}"),
                    }
                }
                other => panic!("Expected try body, got {other:?}"),
            }

            let else_body = else_body.as_ref().expect("expected else body");
            let (_, value) = expect_assign(&else_body[0]);
            match value {
                Expr::Bool { value, .. } => assert!(!*value),
                other => panic!("Expected False, got {other:?}"),
            }
        }
        other => panic!("Expected while loop, got {other:?}"),
    }
}

#[test]
fn parses_empty_function_body() {
    let program = parse_ok("def foo() { }");
    assert_eq!(program.stmts.len(), 1);
    match &program.stmts[0] {
        Stmt::Def { body, .. } => assert!(body.is_empty()),
        other => panic!("Expected function def, got {other:?}"),
    }
}

#[test]
fn parses_empty_function_body_without_params() {
    let program = parse_ok("def foo { }");
    assert_eq!(program.stmts.len(), 1);
    match &program.stmts[0] {
        Stmt::Def { params, body, .. } => {
            assert!(params.is_empty());
            assert!(body.is_empty());
        }
        other => panic!("Expected function def, got {other:?}"),
    }
}
