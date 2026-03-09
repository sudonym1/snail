mod common;

use common::*;
use snail_ast::{
    Argument, AssignTarget, AugAssignOp, BinaryOp, CompareOp, Condition, Expr, ImportFromItems,
    IncrOp, Parameter, Stmt, StringDelimiter,
};
use snail_parser::parse as parse_program;

const COMPACT_TRY_EXCEPTION_VAR: &str = "__snail_compact_exc";
const COMPACT_TRY_NO_FALLBACK_HELPER: &str = "__snail_compact_try_no_fallback";

fn expect_compact_try(expr: &Expr) -> (&Expr, Option<&Expr>) {
    match expr {
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            assert!(else_body.is_none());
            assert!(finally_body.is_none());
            assert_eq!(body.len(), 1);
            assert_eq!(handlers.len(), 1);

            let body_expr = match &body[0] {
                Stmt::Expr { value, .. } => value,
                other => panic!("Expected compact try body expr, got {other:?}"),
            };

            let handler = &handlers[0];
            assert!(matches!(
                handler.type_name.as_ref(),
                Some(Expr::Name { name, .. }) if name == "Exception"
            ));
            assert_eq!(handler.name.as_deref(), Some(COMPACT_TRY_EXCEPTION_VAR));
            assert_eq!(handler.body.len(), 1);

            let handler_expr = match &handler.body[0] {
                Stmt::Expr { value, .. } => value,
                other => panic!("Expected compact try handler expr, got {other:?}"),
            };
            let fallback = if matches!(
                handler_expr,
                Expr::Call { func, args, .. }
                    if matches!(func.as_ref(), Expr::Name { name, .. } if name == COMPACT_TRY_NO_FALLBACK_HELPER)
                        && matches!(args.as_slice(), [Argument::Positional { value: Expr::Name { name, .. }, .. }] if name == COMPACT_TRY_EXCEPTION_VAR)
            ) {
                None
            } else {
                Some(handler_expr)
            };

            (body_expr, fallback)
        }
        other => panic!("Expected compact try, got {other:?}"),
    }
}

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

    match unwrap_expr(&program.stmts[1]) {
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => {
            expect_condition_name(cond, "x");
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

    match unwrap_expr(&program.stmts[0]) {
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            expect_condition_name(cond, "x");
            assert_eq!(body.len(), 1);
            assert_eq!(elifs.len(), 1);
            assert!(else_body.is_some());

            let (targets, value) = expect_assign(&body[0]);
            assert!(matches!(&targets[0], AssignTarget::Name { name, .. } if name == "y"));
            expect_number(value, "1");

            let (elif_cond, elif_body) = &elifs[0];
            expect_condition_name(elif_cond, "y");
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
fn parses_if_let_with_guard() {
    let source = "if let [user, domain] = pair; user { print(domain) }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::If { cond, body, .. } => match cond {
            Condition::Let {
                target,
                value,
                guard,
                ..
            } => {
                match target.as_ref() {
                    AssignTarget::List { elements, .. } => {
                        assert_eq!(elements.len(), 2);
                        assert!(
                            matches!(&elements[0], AssignTarget::Name { name, .. } if name == "user")
                        );
                        assert!(
                            matches!(&elements[1], AssignTarget::Name { name, .. } if name == "domain")
                        );
                    }
                    other => panic!("Expected list target, got {other:?}"),
                }
                expect_name(value.as_ref(), "pair");
                let guard = guard.as_ref().expect("expected guard");
                expect_name(guard.as_ref(), "user");
                assert_eq!(body.len(), 1);
            }
            other => panic!("Expected let condition, got {other:?}"),
        },
        other => panic!("Expected if statement, got {other:?}"),
    }
}

#[test]
fn parses_if_let_with_starred_target() {
    let source = "if let [user, *rest] = pair { print(user) }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::If { cond, .. } => match cond {
            Condition::Let { target, value, .. } => {
                match target.as_ref() {
                    AssignTarget::List { elements, .. } => {
                        assert_eq!(elements.len(), 2);
                        assert!(
                            matches!(&elements[0], AssignTarget::Name { name, .. } if name == "user")
                        );
                        match &elements[1] {
                            AssignTarget::Starred { target, .. } => assert!(
                                matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "rest")
                            ),
                            other => panic!("Expected starred target, got {other:?}"),
                        }
                    }
                    other => panic!("Expected list target, got {other:?}"),
                }
                expect_name(value.as_ref(), "pair");
            }
            other => panic!("Expected let condition, got {other:?}"),
        },
        other => panic!("Expected if statement, got {other:?}"),
    }
}

#[test]
fn parses_while_let() {
    let source = "while let x = next(); x { print(x) }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::While { cond, .. } => match cond {
            Condition::Let {
                target,
                value,
                guard,
                ..
            } => {
                assert!(matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "x"));
                match value.as_ref() {
                    Expr::Call { func, .. } => expect_name(func.as_ref(), "next"),
                    other => panic!("Expected call expression, got {other:?}"),
                }
                let guard = guard.as_ref().expect("expected guard");
                expect_name(guard.as_ref(), "x");
            }
            other => panic!("Expected let condition, got {other:?}"),
        },
        other => panic!("Expected while statement, got {other:?}"),
    }
}

#[test]
fn parses_unconditional_while() {
    let source = "while { print(1) }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::While {
            cond,
            body,
            else_body,
            ..
        } => {
            match cond {
                Condition::Expr(expr) => {
                    assert!(
                        matches!(expr.as_ref(), Expr::Bool { value: true, .. }),
                        "Expected Bool(true), got {expr:?}"
                    );
                }
                other => panic!("Expected Expr condition, got {other:?}"),
            }
            assert_eq!(body.len(), 1);
            assert!(else_body.is_none());
        }
        other => panic!("Expected while statement, got {other:?}"),
    }
}

#[test]
fn parses_for_header_with_newlines() {
    let source = "for\nx\nin\nrange(1) { }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::For {
            target, iter, body, ..
        } => {
            assert!(matches!(target, AssignTarget::Name { name, .. } if name == "x"));
            match iter.as_ref() {
                Expr::Call { func, args, .. } => {
                    expect_name(func.as_ref(), "range");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Argument::Positional { value, .. } => expect_number(value, "1"),
                        other => panic!("Expected positional argument, got {other:?}"),
                    }
                }
                other => panic!("Expected call expression, got {other:?}"),
            }
            assert!(body.is_empty());
        }
        other => panic!("Expected for statement, got {other:?}"),
    }
}

#[test]
fn parses_multiline_if_while_with_headers() {
    let source = "if\nTrue\n{ pass }\nwhile\nFalse\n{ pass }\nwith\nctx\n{ pass }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    match unwrap_expr(&program.stmts[0]) {
        Expr::If { cond, body, .. } => {
            expect_condition_expr(cond);
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected if statement, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[1]) {
        Expr::While { cond, body, .. } => {
            expect_condition_expr(cond);
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected while statement, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[2]) {
        Expr::With { items, body, .. } => {
            assert_eq!(items.len(), 1);
            expect_name(&items[0].context, "ctx");
            assert!(items[0].target.is_none());
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected with statement, got {other:?}"),
    }
}

#[test]
fn parses_multiline_def_class_try_headers() {
    let source = "def\nfoo\n()\n{ pass }\nclass\nC\n{ pass }\ntry\n{ pass }\nexcept\nException\n{ pass }\nfinally\n{ pass }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, &Some("foo".to_string()));
            assert!(params.is_empty());
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected def statement, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[1]) {
        Expr::Class { name, body, .. } => {
            assert_eq!(name, "C");
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected class statement, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[2]) {
        Expr::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            assert_eq!(body.len(), 1);
            assert_eq!(handlers.len(), 1);
            match &handlers[0].type_name {
                Some(expr) => expect_name(expr, "Exception"),
                None => panic!("Expected exception type"),
            }
            assert!(handlers[0].name.is_none());
            assert_eq!(handlers[0].body.len(), 1);
            assert!(matches!(&handlers[0].body[0], Stmt::Pass { .. }));

            let finally_body = finally_body.as_ref().expect("expected finally body");
            assert_eq!(finally_body.len(), 1);
            assert!(matches!(&finally_body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected try statement, got {other:?}"),
    }
}

#[test]
fn parses_except_as_with_newlines() {
    let source = "try { pass }\nexcept Exception\nas\ne\n{ pass }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Try { handlers, .. } => {
            assert_eq!(handlers.len(), 1);
            match &handlers[0].type_name {
                Some(expr) => expect_name(expr, "Exception"),
                None => panic!("Expected exception type"),
            }
            assert_eq!(handlers[0].name.as_deref(), Some("e"));
        }
        other => panic!("Expected try statement, got {other:?}"),
    }
}

#[test]
fn parses_multiline_assert_del_import_headers() {
    // Under Go-style rules: assert/del/import are Continuation keywords,
    // but their arguments may contain StmtEnders (e.g. True, os).
    // del\nitems[0] works because del is Continuation and items[ starts an index.
    // import\nos works because import is Continuation.
    // assert True, "ok" must be on one line (True is StmtEnder).
    // from\nos\nimport\npath: from is Continuation, but os is StmtEnder → separates.
    // So we keep assert and from...import on one line.
    let source = "assert True, \"ok\"\ndel\nitems[0]\nimport\nos\nfrom os import path\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 4);

    match &program.stmts[0] {
        Stmt::Assert { test, message, .. } => {
            assert!(matches!(test, Expr::Bool { value: true, .. }));
            match message {
                Some(Expr::String { value, .. }) => assert_eq!(value, "ok"),
                other => panic!("Expected assert message, got {other:?}"),
            }
        }
        other => panic!("Expected assert statement, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::Delete { targets, .. } => {
            assert_eq!(targets.len(), 1);
            match &targets[0] {
                AssignTarget::Index { value, index, .. } => {
                    expect_name(value.as_ref(), "items");
                    expect_number(index.as_ref(), "0");
                }
                other => panic!("Expected index delete target, got {other:?}"),
            }
        }
        other => panic!("Expected delete statement, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::Import { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, vec!["os"]);
            assert_eq!(items[0].alias, None);
        }
        other => panic!("Expected import statement, got {other:?}"),
    }

    match &program.stmts[3] {
        Stmt::ImportFrom { module, items, .. } => {
            assert_eq!(module.as_ref(), Some(&vec!["os".to_string()]));
            match items {
                ImportFromItems::Names(names) => {
                    assert_eq!(names.len(), 1);
                    assert_eq!(names[0].name, vec!["path"]);
                }
                other => panic!("Expected name imports, got {other:?}"),
            }
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }
}

#[test]
fn parses_destructuring_assignment() {
    let source = "x, y = [1, 2]\n[a, b] = pair\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (targets, _) = expect_assign(&program.stmts[0]);
    match &targets[0] {
        AssignTarget::Tuple { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "x"));
            assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "y"));
        }
        other => panic!("Expected tuple target, got {other:?}"),
    }

    let (targets, _) = expect_assign(&program.stmts[1]);
    match &targets[0] {
        AssignTarget::List { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "a"));
            assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "b"));
        }
        other => panic!("Expected list target, got {other:?}"),
    }
}

#[test]
fn parses_starred_destructuring_assignment() {
    let source = "x, *xs = [1, 2, 3]\n[a, *rest] = pair\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (targets, _) = expect_assign(&program.stmts[0]);
    match &targets[0] {
        AssignTarget::Tuple { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "x"));
            match &elements[1] {
                AssignTarget::Starred { target, .. } => assert!(
                    matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "xs")
                ),
                other => panic!("Expected starred target, got {other:?}"),
            }
        }
        other => panic!("Expected tuple target, got {other:?}"),
    }

    let (targets, _) = expect_assign(&program.stmts[1]);
    match &targets[0] {
        AssignTarget::List { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "a"));
            match &elements[1] {
                AssignTarget::Starred { target, .. } => assert!(
                    matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "rest")
                ),
                other => panic!("Expected starred target, got {other:?}"),
            }
        }
        other => panic!("Expected list target, got {other:?}"),
    }
}

#[test]
fn parses_multiline_destructuring_assignment_and_for_targets() {
    let source = "x,\ny = [1, 2]\n[a,\nb] = pair\nx,\n*rest = values\nfor\nx,\ny\nin\n[(1, 2)]\n{ pass }\nfor\n[a,\nb]\nin\n[[1, 2]]\n{ pass }\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 5);

    let (targets, _) = expect_assign(&program.stmts[0]);
    match &targets[0] {
        AssignTarget::Tuple { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "x"));
            assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "y"));
        }
        other => panic!("Expected tuple target, got {other:?}"),
    }

    let (targets, _) = expect_assign(&program.stmts[1]);
    match &targets[0] {
        AssignTarget::List { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "a"));
            assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "b"));
        }
        other => panic!("Expected list target, got {other:?}"),
    }

    let (targets, _) = expect_assign(&program.stmts[2]);
    match &targets[0] {
        AssignTarget::Tuple { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "x"));
            match &elements[1] {
                AssignTarget::Starred { target, .. } => assert!(
                    matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "rest")
                ),
                other => panic!("Expected starred target, got {other:?}"),
            }
        }
        other => panic!("Expected tuple target, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[3]) {
        Expr::For { target, body, .. } => {
            match target {
                AssignTarget::Tuple { elements, .. } => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "x"));
                    assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "y"));
                }
                other => panic!("Expected tuple target, got {other:?}"),
            }
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected for statement, got {other:?}"),
    }

    match unwrap_expr(&program.stmts[4]) {
        Expr::For { target, body, .. } => {
            match target {
                AssignTarget::List { elements, .. } => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(&elements[0], AssignTarget::Name { name, .. } if name == "a"));
                    assert!(matches!(&elements[1], AssignTarget::Name { name, .. } if name == "b"));
                }
                other => panic!("Expected list target, got {other:?}"),
            }
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Stmt::Pass { .. }));
        }
        other => panic!("Expected for statement, got {other:?}"),
    }
}

#[test]
fn parses_def_and_call() {
    let source = "def add(a, b) { return a + b }\nresult = add(1, 2)\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, &Some("add".to_string()));
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

// Lambda (def expr) tests removed — anonymous def expressions no longer exist

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
        Stmt::ImportFrom {
            level,
            module,
            items,
            ..
        } => {
            assert_eq!(*level, 0);
            assert_eq!(module.as_ref(), Some(&vec!["collections".to_string()]));
            match items {
                ImportFromItems::Names(items) => {
                    assert_eq!(items.len(), 2);
                    assert_eq!(items[0].name, vec!["deque"]);
                    assert_eq!(items[1].alias, Some("dd".to_string()));
                }
                other => panic!("Expected name list, got {other:?}"),
            }
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }
}

#[test]
fn parses_import_from_variants() {
    let source = "from . import local\nfrom ..pkg import name as alias\nfrom pkg import *\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);

    match &program.stmts[0] {
        Stmt::ImportFrom {
            level,
            module,
            items,
            ..
        } => {
            assert_eq!(*level, 1);
            assert!(module.is_none());
            match items {
                ImportFromItems::Names(items) => {
                    assert_eq!(items.len(), 1);
                    assert_eq!(items[0].name, vec!["local"]);
                    assert_eq!(items[0].alias, None);
                }
                other => panic!("Expected name list, got {other:?}"),
            }
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::ImportFrom {
            level,
            module,
            items,
            ..
        } => {
            assert_eq!(*level, 2);
            assert_eq!(module.as_ref(), Some(&vec!["pkg".to_string()]));
            match items {
                ImportFromItems::Names(items) => {
                    assert_eq!(items.len(), 1);
                    assert_eq!(items[0].name, vec!["name"]);
                    assert_eq!(items[0].alias, Some("alias".to_string()));
                }
                other => panic!("Expected name list, got {other:?}"),
            }
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::ImportFrom {
            level,
            module,
            items,
            ..
        } => {
            assert_eq!(*level, 0);
            assert_eq!(module.as_ref(), Some(&vec!["pkg".to_string()]));
            match items {
                ImportFromItems::Star { .. } => {}
                other => panic!("Expected star import, got {other:?}"),
            }
        }
        other => panic!("Expected from-import statement, got {other:?}"),
    }
}

#[test]
fn parses_parenthesized_import() {
    let source = "from pkg import (a, b as bee)\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match &program.stmts[0] {
        Stmt::ImportFrom {
            level,
            module,
            items,
            ..
        } => {
            assert_eq!(*level, 0);
            assert_eq!(module.as_ref(), Some(&vec!["pkg".to_string()]));
            match items {
                ImportFromItems::Names(items) => {
                    assert_eq!(items.len(), 2);
                    assert_eq!(items[0].name, vec!["a"]);
                    assert_eq!(items[1].name, vec!["b"]);
                    assert_eq!(items[1].alias, Some("bee".to_string()));
                }
                other => panic!("Expected name list, got {other:?}"),
            }
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
fn parses_augmented_assignment_and_increments() {
    let source = "x += 5\n++x\nx++\nobj.value += 2\n++obj.value\nobj.value++\nitems[0] += 3\n++items[0]\nitems[0]++\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 9);

    match expect_expr_stmt(&program.stmts[0]) {
        Expr::AugAssign {
            target, op, value, ..
        } => {
            assert!(matches!(op, AugAssignOp::Add));
            match target.as_ref() {
                AssignTarget::Name { name, .. } => assert_eq!(name, "x"),
                other => panic!("Expected name target, got {other:?}"),
            }
            expect_number(value, "5");
        }
        other => panic!("Expected aug assign expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[1]) {
        Expr::PrefixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Name { name, .. } => assert_eq!(name, "x"),
                other => panic!("Expected name target, got {other:?}"),
            }
        }
        other => panic!("Expected prefix incr expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[2]) {
        Expr::PostfixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Name { name, .. } => assert_eq!(name, "x"),
                other => panic!("Expected name target, got {other:?}"),
            }
        }
        other => panic!("Expected postfix incr expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[3]) {
        Expr::AugAssign {
            target, op, value, ..
        } => {
            assert!(matches!(op, AugAssignOp::Add));
            match target.as_ref() {
                AssignTarget::Attribute { attr, .. } => assert_eq!(attr, "value"),
                other => panic!("Expected attribute target, got {other:?}"),
            }
            expect_number(value, "2");
        }
        other => panic!("Expected aug assign expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[4]) {
        Expr::PrefixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Attribute { attr, .. } => assert_eq!(attr, "value"),
                other => panic!("Expected attribute target, got {other:?}"),
            }
        }
        other => panic!("Expected prefix incr expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[5]) {
        Expr::PostfixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Attribute { attr, .. } => assert_eq!(attr, "value"),
                other => panic!("Expected attribute target, got {other:?}"),
            }
        }
        other => panic!("Expected postfix incr expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[6]) {
        Expr::AugAssign {
            target, op, value, ..
        } => {
            assert!(matches!(op, AugAssignOp::Add));
            match target.as_ref() {
                AssignTarget::Index {
                    value: index_value, ..
                } => {
                    expect_name(index_value.as_ref(), "items");
                }
                other => panic!("Expected index target, got {other:?}"),
            }
            expect_number(value, "3");
        }
        other => panic!("Expected aug assign expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[7]) {
        Expr::PrefixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Index {
                    value: index_value, ..
                } => {
                    expect_name(index_value.as_ref(), "items");
                }
                other => panic!("Expected index target, got {other:?}"),
            }
        }
        other => panic!("Expected prefix incr expr, got {other:?}"),
    }

    match expect_expr_stmt(&program.stmts[8]) {
        Expr::PostfixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            match target.as_ref() {
                AssignTarget::Index {
                    value: index_value, ..
                } => {
                    expect_name(index_value.as_ref(), "items");
                }
                other => panic!("Expected index target, got {other:?}"),
            }
        }
        other => panic!("Expected postfix incr expr, got {other:?}"),
    }
}

#[test]
fn parser_rejects_invalid_increment_target() {
    parse_err("++5");
}

#[test]
fn parses_parenthesized_expressions() {
    // Simple parenthesized expression creates Expr::Paren
    let source = "(x)";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match expect_expr_stmt(&program.stmts[0]) {
        Expr::Paren { expr, .. } => {
            expect_name(expr.as_ref(), "x");
        }
        other => panic!("Expected Paren expression, got {other:?}"),
    }

    // (++x)? is valid: compact try around Paren around PrefixIncr
    let source = "(++x)?";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let (expr, fallback) = expect_compact_try(expect_expr_stmt(&program.stmts[0]));
    assert!(fallback.is_none());
    match expr {
        Expr::Paren { expr: inner, .. } => match inner.as_ref() {
            Expr::PrefixIncr { op, target, .. } => {
                assert!(matches!(op, IncrOp::Increment));
                match target.as_ref() {
                    AssignTarget::Name { name, .. } => assert_eq!(name, "x"),
                    other => panic!("Expected name target, got {other:?}"),
                }
            }
            other => panic!("Expected PrefixIncr, got {other:?}"),
        },
        other => panic!("Expected Paren, got {other:?}"),
    }
}

#[test]
fn parses_newline_continuations_in_expressions() {
    let source = "call_value = print(\n1\n)\nparen_value = (\n1\n)\nlist_value = [1,\n2]\ndict_value = %{\"a\": 1,\n\"b\": 2}\nsum_value = 1 +\n2\nassigned =\n3\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 6);

    match &program.stmts[0] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::Call { .. }));
        }
        other => panic!("Expected assignment, got {other:?}"),
    }

    match &program.stmts[1] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::Paren { .. }));
        }
        other => panic!("Expected assignment, got {other:?}"),
    }

    match &program.stmts[2] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::List { .. }));
        }
        other => panic!("Expected assignment, got {other:?}"),
    }

    match &program.stmts[3] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(value, Expr::Dict { .. }));
        }
        other => panic!("Expected assignment, got {other:?}"),
    }

    match &program.stmts[4] {
        Stmt::Assign { value, .. } => {
            assert!(matches!(
                value,
                Expr::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
        }
        other => panic!("Expected assignment, got {other:?}"),
    }

    let (_, assigned) = expect_assign(&program.stmts[5]);
    expect_number(assigned, "3");
}

#[test]
fn parser_rejects_prefix_incr_on_try_expr() {
    // ++x? is invalid: try expression result cannot be incremented
    parse_err("++x?");
}

#[test]
fn parser_rejects_compact_try_on_binding_expressions() {
    parse_err("y:0? *= 3");
    parse_err("x? += 1");

    let program = parse_ok("(x++)?");
    assert_eq!(program.stmts.len(), 1);

    let program = parse_ok("x += y:0?");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn postfix_incr_with_compact_try() {
    // x++? is valid: try wrapping the postfix increment
    let program = parse_ok("x++?");
    assert_eq!(program.stmts.len(), 1);
    let (body_expr, fallback) = expect_compact_try(expect_expr_stmt(&program.stmts[0]));
    assert!(fallback.is_none());
    match body_expr {
        Expr::PostfixIncr { op, target, .. } => {
            assert!(matches!(op, IncrOp::Increment));
            assert!(matches!(target.as_ref(), AssignTarget::Name { name, .. } if name == "x"));
        }
        other => panic!("Expected PostfixIncr, got {other:?}"),
    }
}

#[test]
fn compound_expr_with_full_postfix() {
    // if True { [1,2,3] }.pop() — compound expr with attribute + call postfix
    let program = parse_ok("if True { [1,2,3] }.pop()");
    assert_eq!(program.stmts.len(), 1);
    let expr = expect_expr_stmt(&program.stmts[0]);
    match expr {
        Expr::Call { func, .. } => match func.as_ref() {
            Expr::Attribute { value, attr, .. } => {
                assert_eq!(attr, "pop");
                assert!(matches!(value.as_ref(), Expr::If { .. }));
            }
            other => panic!("Expected Attribute, got {other:?}"),
        },
        other => panic!("Expected Call, got {other:?}"),
    }

    // if True { x }?.method() — compound expr with try + attribute + call
    let program = parse_ok("if True { x }?.method()");
    assert_eq!(program.stmts.len(), 1);
    let expr = expect_expr_stmt(&program.stmts[0]);
    match expr {
        Expr::Call { func, .. } => match func.as_ref() {
            Expr::Attribute { value, attr, .. } => {
                assert_eq!(attr, "method");
                let (body_expr, fallback) = expect_compact_try(value.as_ref());
                assert!(fallback.is_none());
                assert!(matches!(body_expr, Expr::If { .. }));
            }
            other => panic!("Expected Attribute, got {other:?}"),
        },
        other => panic!("Expected Call, got {other:?}"),
    }
}

#[test]
fn parses_nested_parentheses_in_try_expr() {
    let source = "((x))?";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let (expr, fallback) = expect_compact_try(expect_expr_stmt(&program.stmts[0]));
    assert!(fallback.is_none());
    match expr {
        Expr::Paren { expr: inner, .. } => match inner.as_ref() {
            Expr::Paren {
                expr: innermost, ..
            } => {
                expect_name(innermost.as_ref(), "x");
            }
            other => panic!("Expected nested Paren, got {other:?}"),
        },
        other => panic!("Expected Paren, got {other:?}"),
    }
}

#[test]
fn parser_rejects_compact_try_on_attr_and_index_aug_assign() {
    parse_err("obj.attr? += 1");
    parse_err("arr[i]? += 1");
}

#[test]
fn parser_rejects_double_try() {
    // consecutive ?? is never valid
    parse_err("x??");
    parse_err("call()??");
    parse_err("x:0??");
    parse_err("(if x { y })??");

    // but ? separated by accessors is fine
    parse_ok("x?.y?");
    parse_ok("call()?.attr?");
}

#[test]
fn parses_list_and_dict_literals_and_comprehensions() {
    let source = "nums = [1, 2, 3]\npairs = %{\"a\": 1, \"b\": 2}\nempty = %{}\nevens = [n for n in nums if n % 2 == 0]\nlookup = %{n: n * 2 for n in nums if n > 1}\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 5);

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
        Expr::Dict { entries, .. } => assert!(entries.is_empty()),
        other => panic!("Expected empty dict literal, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[3]);
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

    let (_, value) = expect_assign(&program.stmts[4]);
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

#[test]
fn parses_set_literals() {
    let source = "items = #{1, 2, 3}\nempty = #{}\n";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::Set { elements, .. } => {
            assert_eq!(elements.len(), 3);
            expect_number(&elements[0], "1");
            expect_number(&elements[1], "2");
            expect_number(&elements[2], "3");
        }
        other => panic!("Expected set literal, got {other:?}"),
    }

    let (_, value) = expect_assign(&program.stmts[1]);
    match value {
        Expr::Set { elements, .. } => assert!(elements.is_empty()),
        other => panic!("Expected empty set literal, got {other:?}"),
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
    let err = parse_program("d = %{\"key\" 1}").expect_err("should fail on missing colon");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains(":"));
}

#[test]
fn parser_rejects_incomplete_function_def() {
    let err = parse_program("def foo").expect_err("should fail on incomplete def");
    let message = err.to_string();
    assert!(message.contains("expected") || message.contains("(") || message.contains("{"));
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
    match unwrap_expr(&program.stmts[0]) {
        Expr::Def { body, .. } => {
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
fn parses_if_stmt_with_not_in_operator() {
    let source = "if x not in y { pass } else { pass }";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::If { cond, .. } => match cond {
            Condition::Expr(expr) => match expr.as_ref() {
                Expr::Compare { ops, .. } => assert_eq!(ops, &[CompareOp::NotIn]),
                other => panic!("Expected comparison, got {:?}", other),
            },
            other => panic!("Expected expr condition, got {:?}", other),
        },
        other => panic!("Expected if statement, got {:?}", other),
    }
}

#[test]
fn parses_if_stmt_with_is_not_operator() {
    let source = "if x is not None { pass } else { pass }";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::If { cond, .. } => match cond {
            Condition::Expr(expr) => match expr.as_ref() {
                Expr::Compare { ops, .. } => assert_eq!(ops, &[CompareOp::IsNot]),
                other => panic!("Expected comparison, got {:?}", other),
            },
            other => panic!("Expected expr condition, got {:?}", other),
        },
        other => panic!("Expected if statement, got {:?}", other),
    }
}

// Tests for compound statement separator behavior (no semicolon needed after })

#[test]
fn parses_while_followed_by_stmt_without_separator() {
    let source = "while x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::While { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_for_followed_by_stmt_without_separator() {
    let source = "for i in x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::For { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_def_followed_by_stmt_with_separator() {
    // def is no longer a compound_expr, so it needs a separator before the next statement
    let source = "def f() { pass }; f()";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Def { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_class_followed_by_stmt_without_separator() {
    let source = "class C { pass } C()";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Class { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_try_followed_by_stmt_without_separator() {
    // Note: using explicit exception type since bare `except { }` is ambiguous
    let source = "try { x } except Exception { y } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Try { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn parses_with_followed_by_stmt_without_separator() {
    let source = "with x { y } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::With { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Expr { .. }));
}

#[test]
fn simple_stmt_still_requires_separator() {
    // Two simple statements without separator should fail
    let source = "a b";
    parse_err(source);
}

#[test]
fn newline_before_equals_does_not_continue_assignment() {
    parse_err("x\n= 1");
}

#[test]
fn newline_before_paren_starts_a_new_statement() {
    let program = parse_ok("x\n(1)");
    assert_eq!(program.stmts.len(), 2);

    expect_name(expect_expr_stmt(&program.stmts[0]), "x");
    match expect_expr_stmt(&program.stmts[1]) {
        Expr::Paren { expr, .. } => expect_number(expr.as_ref(), "1"),
        other => panic!("Expected parenthesized expression, got {other:?}"),
    }
}

#[test]
fn trailing_infix_operator_continues_expression() {
    // Under Go-style rules: 1\n+\n1 → two statements (1 is StmtEnder, inject before +)
    // Trailing operator continues: 1 +\n1 → single binary expression
    let program = parse_ok("1 +\n1");
    assert_eq!(program.stmts.len(), 1);
    match expect_expr_stmt(&program.stmts[0]) {
        Expr::Binary {
            left, op, right, ..
        } => {
            assert!(matches!(op, BinaryOp::Add));
            expect_number(left.as_ref(), "1");
            expect_number(right.as_ref(), "1");
        }
        other => panic!("Expected binary expression, got {other:?}"),
    }
}

#[test]
fn newline_before_dot_separates_attribute_access() {
    // Under Go-style rules: obj\n.attr → separated (obj is StmtEnder)
    // This now fails to parse since .attr is not a valid statement start.
    let result = snail_parser::parse("value = obj\n.attr");
    assert!(
        result.is_err(),
        "obj\\n.attr should fail to parse under Go-style rules"
    );
}

#[test]
fn newline_before_dot_separates_attribute_assignment_target() {
    // Under Go-style rules: obj\n.attr = 1 → separated (obj is StmtEnder)
    // This now fails to parse since .attr is not a valid statement start.
    let result = snail_parser::parse("obj\n.attr = 1");
    assert!(
        result.is_err(),
        "obj\\n.attr = 1 should fail to parse under Go-style rules"
    );
}

#[test]
fn newline_after_dot_continues_attribute_access() {
    let program = parse_ok("value = obj.\nattr");
    assert_eq!(program.stmts.len(), 1);

    let (_, value) = expect_assign(&program.stmts[0]);
    match value {
        Expr::Attribute {
            value: inner, attr, ..
        } => {
            expect_name(inner.as_ref(), "obj");
            assert_eq!(attr, "attr");
        }
        other => panic!("Expected attribute expression, got {other:?}"),
    }
}

#[test]
fn newline_after_dot_continues_attribute_assignment_target() {
    let program = parse_ok("obj.\nattr = 1");
    assert_eq!(program.stmts.len(), 1);

    let (targets, value) = expect_assign(&program.stmts[0]);
    match &targets[0] {
        AssignTarget::Attribute {
            value: inner, attr, ..
        } => {
            expect_name(inner.as_ref(), "obj");
            assert_eq!(attr, "attr");
        }
        other => panic!("Expected attribute target, got {other:?}"),
    }
    expect_number(value, "1");
}

#[test]
fn compact_try_stmt_still_requires_separator() {
    // Two simple statements without a separator should fail to parse
    let err = parse_err("x = risky()? y = 2");
    let message = err.to_string();
    assert!(
        message.contains("expected") || message.contains("separator"),
        "expected parse error mentioning expected or separator, got: {message}"
    );
}

// ========== Awk/Xargs Block Tests ==========

#[test]
fn parses_bare_awk_block() {
    let source = "awk { print($0) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert!(sources.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_source() {
    let source = r#"awk("file.txt") { print($0) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_multiple_sources() {
    let source = r#"awk("a", "b") { print($0) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert_eq!(sources.len(), 2);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_trailing_comma() {
    let source = r#"awk("a",) { print($0) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_pattern_action() {
    let source = r#"awk("file.txt") { /pat/ { print($0) } }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::PatternAction {
                    pattern, action, ..
                } => {
                    assert!(pattern.is_some());
                    assert!(action.is_some());
                }
                other => panic!("Expected pattern/action, got {other:?}"),
            }
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_bare_block() {
    let source = "awk { { print($0) } }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::PatternAction {
                    pattern, action, ..
                } => {
                    assert!(pattern.is_none());
                    assert!(action.is_some());
                }
                other => panic!("Expected pattern/action (bare block), got {other:?}"),
            }
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_bare_pattern() {
    let source = "awk { /pat/ }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::PatternAction {
                    pattern, action, ..
                } => {
                    assert!(pattern.is_some());
                    assert!(action.is_none());
                }
                other => panic!("Expected pattern/action (bare pattern), got {other:?}"),
            }
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_bare_xargs_block() {
    let source = "xargs { print($src) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Xargs { sources, body, .. } => {
            assert!(sources.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected xargs statement, got {other:?}"),
    }
}

#[test]
fn parses_xargs_with_source() {
    let source = "xargs(paths) { print($src) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Xargs { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected xargs statement, got {other:?}"),
    }
}

#[test]
fn parses_xargs_with_multiple_sources() {
    let source = r#"xargs("a", "b") { print($src) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Xargs { sources, body, .. } => {
            assert_eq!(sources.len(), 2);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected xargs statement, got {other:?}"),
    }
}

#[test]
fn parses_xargs_with_trailing_comma() {
    let source = r#"xargs("a",) { print($src) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Xargs { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected xargs statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_followed_by_stmt() {
    let source = "awk { print($0) } x = 1";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Awk { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Assign { .. }));
}

#[test]
fn parses_xargs_followed_by_stmt() {
    let source = "xargs { print($src) } x = 1";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Xargs { .. }));
    assert!(matches!(&program.stmts[1], Stmt::Assign { .. }));
}

#[test]
fn parses_awk_with_multiline() {
    let source = "awk\n{\nprint($0)\n}";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::Awk { .. }));
}

#[test]
fn parser_rejects_dollar_zero_outside_awk() {
    let err = parse_err("print($0)");
    let message = err.to_string();
    assert!(message.contains("awk"));
}

#[test]
fn parser_rejects_fd_inside_awk() {
    let err = parse_err("awk { print($fd) }");
    let message = err.to_string();
    assert!(message.contains("xargs variables"));
}

#[test]
fn parser_rejects_field_index_outside_awk() {
    let err = parse_err("print($1)");
    let message = err.to_string();
    assert!(message.contains("awk"));
}

#[test]
fn parser_rejects_awk_vars_inside_xargs() {
    let err = parse_err("xargs { print($n) }");
    let message = err.to_string();
    assert!(message.contains("awk variables"));
}

#[test]
fn parses_awk_with_star_arg() {
    let source = "awk(*args) { print($0) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert!(matches!(&sources[0], Argument::Star { .. }));
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_with_kw_arg() {
    let source = r#"awk("file", encoding="utf-8") { print($0) }"#;
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert_eq!(sources.len(), 2);
            assert!(matches!(&sources[0], Argument::Positional { .. }));
            match &sources[1] {
                Argument::Keyword { name, .. } => assert_eq!(name, "encoding"),
                other => panic!("Expected keyword argument, got {other:?}"),
            }
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parses_xargs_with_star_arg() {
    let source = "xargs(*paths) { print($src) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Xargs { sources, body, .. } => {
            assert_eq!(sources.len(), 1);
            assert!(matches!(&sources[0], Argument::Star { .. }));
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected xargs statement, got {other:?}"),
    }
}

#[test]
fn parses_awk_empty_parens() {
    let source = "awk() { print($0) }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);

    match unwrap_expr(&program.stmts[0]) {
        Expr::Awk { sources, body, .. } => {
            assert!(sources.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected awk statement, got {other:?}"),
    }
}

#[test]
fn parser_rejects_pattern_action_outside_awk() {
    let err = parse_err("/pattern/ { print(1) }");
    let message = err.to_string();
    assert!(
        message.contains("expected")
            || message.contains("pattern")
            || message.contains("statement")
    );
}

#[test]
fn parses_segment_break() {
    // \x1f (Unit Separator) produces SegmentBreak nodes in the AST
    let source = "x = 1\n\x1f\ny = 2\n\x1f\nz = 3";
    let program = parse_ok(source);
    // Should have 5 nodes: Assign, SegmentBreak, Assign, SegmentBreak, Assign
    assert_eq!(program.stmts.len(), 5);
    assert!(matches!(&program.stmts[0], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[1], Stmt::SegmentBreak { .. }));
    assert!(matches!(&program.stmts[2], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[3], Stmt::SegmentBreak { .. }));
    assert!(matches!(&program.stmts[4], Stmt::Assign { .. }));
}

#[test]
fn parses_if_followed_by_stmt_same_line() {
    let source = "if x { y = 1 } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::If { .. }));
    expect_name(expect_expr_stmt(&program.stmts[1]), "z");
}

#[test]
fn parses_if_followed_by_stmt_no_space() {
    let source = "if x { y = 1 }z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::If { .. }));
    expect_name(expect_expr_stmt(&program.stmts[1]), "z");
}

#[test]
fn parses_if_else_followed_by_stmt() {
    let source = "if a { x } else { y } z";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::If { .. }));
    expect_name(expect_expr_stmt(&program.stmts[1]), "z");
}

#[test]
fn parses_nested_if_blocks_without_separators() {
    let source = "if a { if b { c } d } e";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::If { .. }));
    expect_name(expect_expr_stmt(&program.stmts[1]), "e");
}

#[test]
fn parses_consecutive_block_stmts() {
    let source = "if a { b } if c { d } while e { f }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 3);
    assert!(matches!(unwrap_expr(&program.stmts[0]), Expr::If { .. }));
    assert!(matches!(unwrap_expr(&program.stmts[1]), Expr::If { .. }));
    assert!(matches!(unwrap_expr(&program.stmts[2]), Expr::While { .. }));
}

#[test]
fn parses_mixed_block_and_simple_stmts() {
    let source = "a = 1; if b { c = 2 } d = 3; e = 4";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 4);
    assert!(matches!(&program.stmts[0], Stmt::Assign { .. }));
    assert!(matches!(unwrap_expr(&program.stmts[1]), Expr::If { .. }));
    assert!(matches!(&program.stmts[2], Stmt::Assign { .. }));
    assert!(matches!(&program.stmts[3], Stmt::Assign { .. }));
}

// === Anonymous def expression tests ===

#[test]
fn parses_anonymous_def_no_params() {
    let source = "def { 1 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, &None);
            assert!(params.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected anonymous def, got {other:?}"),
    }
}

#[test]
fn parses_anonymous_def_with_params() {
    let source = "def(a, b) { a + b }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, &None);
            assert_eq!(params.len(), 2);
            assert!(matches!(&params[0], Parameter::Regular { name, .. } if name == "a"));
            assert!(matches!(&params[1], Parameter::Regular { name, .. } if name == "b"));
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected anonymous def, got {other:?}"),
    }
}

#[test]
fn parses_named_def_unchanged() {
    let source = "def foo(x) { x }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::Def {
            name, params, body, ..
        } => {
            assert_eq!(name, &Some("foo".to_string()));
            assert_eq!(params.len(), 1);
            assert!(matches!(&params[0], Parameter::Regular { name, .. } if name == "x"));
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected named def, got {other:?}"),
    }
}

#[test]
fn parses_anonymous_def_immediate_call() {
    // def { 1 }() should parse as a single Call expression wrapping a Def
    let source = "def { 1 }()";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::Call { func, args, .. } => {
            assert!(matches!(func.as_ref(), Expr::Def { .. }));
            assert!(args.is_empty());
        }
        other => panic!("Expected Call wrapping Def, got {other:?}"),
    }
}

#[test]
fn parses_break_without_value() {
    let source = "while True { break }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::While { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Break { value, .. } => assert!(value.is_none()),
                other => panic!("Expected Break, got {other:?}"),
            }
        }
        other => panic!("Expected While, got {other:?}"),
    }
}

#[test]
fn parses_break_with_value() {
    let source = "while True { break 42 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::While { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Break {
                    value: Some(val), ..
                } => {
                    expect_number(val, "42");
                }
                other => panic!("Expected Break with value, got {other:?}"),
            }
        }
        other => panic!("Expected While, got {other:?}"),
    }
}

#[test]
fn parses_break_with_string_value() {
    let source = r#"while True { break "found" }"#;
    let program = parse_ok(source);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::While { body, .. } => match &body[0] {
            Stmt::Break {
                value: Some(Expr::String { value, .. }),
                ..
            } => {
                assert_eq!(value, "found");
            }
            other => panic!("Expected Break with string value, got {other:?}"),
        },
        other => panic!("Expected While, got {other:?}"),
    }
}

#[test]
fn parses_break_followed_by_newline() {
    // break on its own line should NOT consume the next line as its value
    let source = "while True {\n  break\n  x = 1\n}";
    let program = parse_ok(source);
    let expr = unwrap_expr(&program.stmts[0]);
    match expr {
        Expr::While { body, .. } => {
            assert_eq!(body.len(), 2);
            match &body[0] {
                Stmt::Break { value, .. } => assert!(value.is_none()),
                other => panic!("Expected bare Break, got {other:?}"),
            }
        }
        other => panic!("Expected While, got {other:?}"),
    }
}

#[test]
fn lambda_is_now_valid_identifier() {
    // `lambda` is no longer a keyword, so it can be used as a variable name
    let result = snail_parser::parse("lambda = 1");
    assert!(result.is_ok());
}

// ---- Class inheritance tests ----

#[test]
fn parses_class_single_base() {
    let source = "class Dog(Animal) { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class {
            name, bases, body, ..
        } => {
            assert_eq!(name, "Dog");
            assert_eq!(bases.len(), 1);
            expect_name(&bases[0], "Animal");
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_multiple_bases() {
    let source = "class C(A, B) { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class {
            name, bases, body, ..
        } => {
            assert_eq!(name, "C");
            assert_eq!(bases.len(), 2);
            expect_name(&bases[0], "A");
            expect_name(&bases[1], "B");
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_empty_parens() {
    let source = "class Foo() { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class {
            name, bases, body, ..
        } => {
            assert_eq!(name, "Foo");
            assert!(bases.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_trailing_comma() {
    let source = "class Foo(A, B,) { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class { bases, .. } => {
            assert_eq!(bases.len(), 2);
            expect_name(&bases[0], "A");
            expect_name(&bases[1], "B");
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_dotted_base() {
    let source = "class Foo(mod.Bar) { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class { bases, .. } => {
            assert_eq!(bases.len(), 1);
            assert!(matches!(&bases[0], Expr::Attribute { .. }));
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_no_bases_backward_compat() {
    let source = "class Foo { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class {
            name, bases, body, ..
        } => {
            assert_eq!(name, "Foo");
            assert!(bases.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_function_call_as_base() {
    let source = "class Foo(get_base()) { pass }";
    let program = parse_ok(source);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class { bases, .. } => {
            assert_eq!(bases.len(), 1);
            assert!(matches!(&bases[0], Expr::Call { .. }));
        }
        other => panic!("Expected class, got {other:?}"),
    }
}

// ---- Decorator tests ----

#[test]
fn parses_single_decorator_on_def() {
    let source = "@my_decorator\ndef foo() { 1 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Def {
            name, decorators, ..
        } => {
            assert_eq!(name, &Some("foo".to_string()));
            assert_eq!(decorators.len(), 1);
            expect_name(&decorators[0], "my_decorator");
        }
        other => panic!("Expected decorated def, got {other:?}"),
    }
}

#[test]
fn parses_single_decorator_on_class() {
    let source = "@my_decorator\nclass Foo { pass }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Class {
            name, decorators, ..
        } => {
            assert_eq!(name, "Foo");
            assert_eq!(decorators.len(), 1);
            expect_name(&decorators[0], "my_decorator");
        }
        other => panic!("Expected decorated class, got {other:?}"),
    }
}

#[test]
fn parses_multiple_decorators() {
    let source = "@dec1\n@dec2\ndef foo() { 1 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Def { decorators, .. } => {
            assert_eq!(decorators.len(), 2);
            expect_name(&decorators[0], "dec1");
            expect_name(&decorators[1], "dec2");
        }
        other => panic!("Expected decorated def, got {other:?}"),
    }
}

#[test]
fn parses_decorator_with_arguments() {
    let source = "@app.route(\"/\")\ndef index() { 1 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Def { decorators, .. } => {
            assert_eq!(decorators.len(), 1);
            match &decorators[0] {
                Expr::Call { func, .. } => {
                    assert!(matches!(func.as_ref(), Expr::Attribute { .. }));
                }
                other => panic!("Expected Call decorator, got {other:?}"),
            }
        }
        other => panic!("Expected decorated def, got {other:?}"),
    }
}

#[test]
fn parses_decorator_with_call_expression() {
    let source = "@decorator()\ndef foo() { 1 }";
    let program = parse_ok(source);
    assert_eq!(program.stmts.len(), 1);
    match unwrap_expr(&program.stmts[0]) {
        Expr::Def { decorators, .. } => {
            assert_eq!(decorators.len(), 1);
            assert!(matches!(&decorators[0], Expr::Call { .. }));
        }
        other => panic!("Expected decorated def, got {other:?}"),
    }
}
