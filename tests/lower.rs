use snail::{PyBinaryOp, PyCompareOp, PyStmt, lower_program, parse_program};

#[test]
fn lowers_if_chain_into_nested_orelse() {
    let source = "if x { y = 1 }\nelif y { return y }\nelse { pass }";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    assert_eq!(module.body.len(), 1);
    let first = &module.body[0];
    if let PyStmt::If {
        test,
        body,
        orelse,
        span,
    } = first
    {
        assert_name_location(test, "x", 1, 4);
        assert_eq!(body.len(), 1);
        assert_eq!(span.start.line, 1);
        assert_eq!(span.end.line, 3);
        let nested = match &orelse[0] {
            PyStmt::If {
                test,
                body,
                orelse,
                span,
            } => {
                assert_name_location(test, "y", 2, 6);
                assert_eq!(body.len(), 1);
                assert!(orelse.len() == 1 && matches!(orelse[0], PyStmt::Pass { .. }));
                span
            }
            other => panic!("expected nested if, got {other:?}"),
        };
        assert_eq!(nested.start.line, 2);
        assert_eq!(nested.end.line, 3);
    } else {
        panic!("expected top-level if, got {first:?}");
    }
}

#[test]
fn lowers_assignment_and_binary_expr() {
    let source = "x = 1\ny = x + 2";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    assert_eq!(module.body.len(), 2);
    let second = &module.body[1];
    let assign = match second {
        PyStmt::Assign {
            targets,
            value,
            span,
        } => {
            assert_eq!(targets.len(), 1);
            assert_name_location(&targets[0], "y", 2, 1);
            assert_eq!(span.start.line, 2);
            value
        }
        other => panic!("expected assignment, got {other:?}"),
    };
    if let snail::PyExpr::Binary {
        left,
        op,
        right,
        span,
    } = assign
    {
        assert_eq!(*op, PyBinaryOp::Add);
        assert_eq!(span.start.line, 2);
        assert_name_location(left, "x", 2, 5);
        assert!(matches!(right.as_ref(), snail::PyExpr::Number { value, .. } if value == "2"));
    } else {
        panic!("expected binary expression, got {assign:?}");
    }
}

#[test]
fn lowers_comparisons_and_calls() {
    let source = "result = check(x) == True";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let first = &module.body[0];
    let value = match first {
        PyStmt::Assign { value, .. } => value,
        other => panic!("expected assignment, got {other:?}"),
    };
    if let snail::PyExpr::Compare {
        left,
        ops,
        comparators,
        ..
    } = value
    {
        assert_eq!(ops, &[PyCompareOp::Eq]);
        assert_eq!(comparators.len(), 1);
        assert!(matches!(
            comparators[0],
            snail::PyExpr::Bool { value: true, .. }
        ));
        if let snail::PyExpr::Call { func, args, .. } = left.as_ref() {
            assert_eq!(args.len(), 1);
            assert_name_location(func, "check", 1, 10);
        } else {
            panic!("expected call on comparison lhs, got {left:?}");
        }
    } else {
        panic!("expected comparison expression, got {value:?}");
    }
}

fn assert_name_location(expr: &snail::PyExpr, expected: &str, line: usize, column: usize) {
    match expr {
        snail::PyExpr::Name { id, span } => {
            assert_eq!(id, expected);
            assert_eq!(span.start.line, line);
            assert_eq!(span.start.column, column);
        }
        other => panic!("expected name expression, got {other:?}"),
    }
}
