mod common;

use common::*;
use snail_ast::{Argument, AssignTarget, BinaryOp, Expr, Parameter, Stmt, StringDelimiter};

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
