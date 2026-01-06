mod common;

use common::*;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use snail::{PyBinaryOp, PyCompareOp, PyStmt, PyUnaryOp, lower_program, parse_program};

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
fn lowers_compound_expression_to_tuple_index() {
    let source = "result = (1; 2; 3)";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");

    let value = match &module.body[0] {
        PyStmt::Assign { value, .. } => value,
        other => panic!("expected assignment, got {other:?}"),
    };

    if let snail::PyExpr::Index { value, index, .. } = value {
        if let snail::PyExpr::Tuple { elements, .. } = value.as_ref() {
            assert_eq!(elements.len(), 3);
        } else {
            panic!("expected tuple in compound lowering, got {value:?}");
        }

        if let snail::PyExpr::Unary { op, operand, .. } = index.as_ref() {
            assert_eq!(*op, PyUnaryOp::Minus);
            assert!(matches!(
                operand.as_ref(),
                snail::PyExpr::Number { value, .. } if value == "1"
            ));
        } else {
            panic!("expected unary index, got {index:?}");
        }
    } else {
        panic!("expected index expression, got {value:?}");
    }
}

#[test]
fn lowers_attribute_and_index_assignment_targets() {
    let source = "obj.value = 1\nitems[0] = 2";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    assert_eq!(module.body.len(), 2);

    let first = &module.body[0];
    if let PyStmt::Assign { targets, .. } = first {
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            snail::PyExpr::Attribute { value, attr, .. } => {
                assert_eq!(attr, "value");
                assert_name_location(value, "obj", 1, 1);
            }
            other => panic!("expected attribute target, got {other:?}"),
        }
    } else {
        panic!("expected assignment, got {first:?}");
    }

    let second = &module.body[1];
    if let PyStmt::Assign { targets, .. } = second {
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            snail::PyExpr::Index { value, index, .. } => {
                assert_name_location(value, "items", 2, 1);
                assert!(matches!(
                    index.as_ref(),
                    snail::PyExpr::Number { value, .. } if value == "0"
                ));
            }
            other => panic!("expected index target, got {other:?}"),
        }
    } else {
        panic!("expected assignment, got {second:?}");
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

#[test]
fn renders_python_with_imports_and_class() {
    let source = r"
import os as os_mod
from sys import path
class Greeter { def greet(name) { print('hi') } }
x = True
y = False
if x { y = 1 }
elif y { y = 2 }
else { pass }
";
    let python = snail_to_python(source);

    // Test that Python compiles without syntax errors
    assert_python_compiles(&python);

    // Test that it contains expected elements (without being brittle about formatting)
    assert!(python.contains("import os as os_mod"));
    assert!(python.contains("from sys import path"));
    assert!(python.contains("class Greeter"));
    assert!(python.contains("def greet"));
    assert!(python.contains("if x:"));
    assert!(python.contains("elif y:"));
    assert!(python.contains("else:"));
    assert!(python.contains("pass"));
}

#[test]
fn round_trip_executes_small_program() {
    let source = "def fact(n) {\n    if n <= 1 { return 1 }\n    return n * fact(n - 1)\n}\nresult = fact(5)";

    Python::with_gil(|py| {
        let python = snail_to_python(source);

        // Execute the generated Python code with both globals and locals as the same dict
        let globals = PyDict::new_bound(py);
        py.run_bound(&python, Some(&globals), Some(&globals))
            .unwrap_or_else(|e| panic!("Execution failed:\n{}\nError: {:?}", python, e));

        // Verify the result
        let result: i64 = get_py_var(py, &globals, "result");
        assert_eq!(result, 120);
    });
}

#[test]
fn renders_list_and_dict_comprehensions() {
    let source =
        "nums = [1, 2]\nvals = {n: n * 2 for n in nums if n > 1}\nlisty = [n for n in nums]";

    Python::with_gil(|py| {
        let python = snail_to_python(source);
        assert_python_compiles(&python);

        let globals = PyDict::new_bound(py);
        py.run_bound(&python, Some(&globals), Some(&globals))
            .unwrap();

        // Verify semantic correctness
        let vals: Bound<PyDict> = globals
            .get_item("vals")
            .unwrap()
            .unwrap()
            .downcast_into()
            .unwrap();
        let val: i64 = vals.get_item(2i64).unwrap().unwrap().extract().unwrap();
        assert_eq!(val, 4);

        let listy: Vec<i64> = get_py_var(py, &globals, "listy");
        assert_eq!(listy, vec![1, 2]);
    });
}

#[test]
fn renders_try_except_finally() {
    let source = r"
try { risky() }
except ValueError as err { raise err }
except { raise }
else { ok = True }
finally { cleanup() }
";
    let python = snail_to_python(source);
    assert_python_compiles(&python);

    // Verify structure without exact string matching
    assert!(python.contains("try:"));
    assert!(python.contains("except ValueError as err:"));
    assert!(python.contains("except:"));
    assert!(python.contains("else:"));
    assert!(python.contains("finally:"));
}

#[test]
fn renders_try_finally_and_raise_from() {
    let source = r"
try { risky() }
finally { cleanup() }
raise ValueError('bad') from err
";
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("try:"));
    assert!(python.contains("finally:"));
    assert!(python.contains("raise ValueError"));
    assert!(python.contains("from err"));
}

#[test]
fn renders_with_statement() {
    let source = r#"
with open("data") as f, lock() { line = f.read() }
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("with "));
    assert!(python.contains("open"));
    assert!(python.contains("as f"));
    assert!(python.contains("lock()"));
}

#[test]
fn renders_assert_and_del() {
    let source = r#"
value = 1
assert value == 1, "ok"
del value
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("assert"));
    assert!(python.contains("del value"));
}

#[test]
fn renders_tuples_sets_and_slices() {
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
    Python::with_gil(|py| {
        let python = snail_to_python(source);
        assert_python_compiles(&python);

        let globals = PyDict::new_bound(py);
        py.run_bound(&python, Some(&globals), Some(&globals))
            .unwrap();

        // Verify semantic correctness
        let pair: (i64, i64) = get_py_var(py, &globals, "pair");
        assert_eq!(pair, (1, 2));

        let single: (i64,) = get_py_var(py, &globals, "single");
        assert_eq!(single, (1,));

        let mid: Vec<i64> = get_py_var(py, &globals, "mid");
        assert_eq!(mid, vec![2, 3]);
    });
}

#[test]
fn renders_defaults_and_star_args() {
    let source = r#"
def join(a, b=1, *rest, **extras) { return a }
result = join(1, b=2, *rest, **extras)
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("def join"));
    assert!(python.contains("*rest"));
    assert!(python.contains("**extras"));
}

#[test]
fn renders_loop_else_and_try_break_continue() {
    let source = r#"
for n in nums { try { break } finally { cleanup() } } else { done = True }
while flag { try { continue } finally { cleanup() } } else { done = False }
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("for n in nums:"));
    assert!(python.contains("while flag:"));
    assert!(python.contains("break"));
    assert!(python.contains("continue"));
    assert!(python.contains("else:"));
}

#[test]
fn renders_if_expression() {
    let source = "value = 1 if flag else 2";
    let python = snail_to_python(source);
    assert_python_compiles(&python);
    assert!(python.contains("if flag else"));
}

#[test]
fn renders_compact_exception_expression() {
    let source = r#"
value = risky()?
fallback = risky():$e?
details = risky():$e.args[0]?
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);

    // Verify helper function is generated
    assert!(python.contains("def __snail_compact_try"));
    assert!(python.contains("lambda: risky()"));

    // Verify it works semantically
    Python::with_gil(|py| {
        let setup = r#"
def risky():
    raise ValueError('test')
"#;
        let code = format!("{}\n{}", setup, python);
        let globals = PyDict::new_bound(py);

        // The compact try should handle the exception
        py.run_bound(&code, Some(&globals), Some(&globals)).unwrap();

        // Verify the exception is caught and returned
        let value_str = globals.get_item("value").unwrap().unwrap().to_string();
        assert!(value_str.contains("ValueError") || value_str.contains("test"));
    });
}

#[test]
fn renders_subprocess_expressions() {
    let source = r#"
out = $(echo {name})
code = @(echo ok)
"#;
    let python = snail_to_python(source);
    assert_python_compiles(&python);

    // Verify helper classes are generated
    assert!(python.contains("import subprocess"));
    assert!(python.contains("class __SnailSubprocessCapture"));
    assert!(python.contains("class __SnailSubprocessStatus"));
    assert!(python.contains("def __pipeline__"));
    assert!(python.contains("shell=True"));
}

#[test]
fn renders_regex_expressions() {
    let source = r#"
text = "value"
found = text in /val(.)/
compiled = /abc/
"#;
    Python::with_gil(|py| {
        let python = snail_to_python(source);
        assert_python_compiles(&python);

        // Verify helper functions are generated
        assert!(python.contains("import re"));
        assert!(python.contains("def __snail_regex_search"));
        assert!(python.contains("def __snail_regex_compile"));

        // Verify semantic correctness
        let globals = PyDict::new_bound(py);
        py.run_bound(&python, Some(&globals), Some(&globals))
            .unwrap();

        // Check that regex search worked
        let found = globals.get_item("found").unwrap().unwrap();
        assert!(!found.is_none()); // Should have found a match

        // Check that compiled pattern is a regex object
        let compiled_str = globals.get_item("compiled").unwrap().unwrap().to_string();
        assert!(compiled_str.contains("re.compile"));
    });
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

#[test]
fn lowers_structured_accessor() {
    let source = r#"result = $[foo.bar]"#;
    let python = snail_to_python(source);

    assert!(python.contains("__SnailStructuredAccessor"));
    assert!(python.contains("__SnailStructuredAccessor(\"foo.bar\")"));
}

#[test]
fn lowers_json_with_structured_accessor() {
    let source = r#"result = json() | $[users[0]]"#;
    let python = snail_to_python(source);

    assert!(python.contains("def json("));
    assert!(python.contains("class __SnailJsonObject"));
    assert!(python.contains("def __structured__"));
    assert!(python.contains("__SnailStructuredAccessor(\"users[0]\")"));
    // Should NOT contain old __SnailJsonQuery
    assert!(!python.contains("__SnailJsonQuery"));
}

#[test]
fn lowers_json_call_without_structured_accessor() {
    let source = r#"json()"#;
    let python = snail_to_python(source);

    assert!(python.contains("def json("));
    assert!(python.contains("class __SnailJsonObject"));
    assert!(python.contains("def __structured__"));
    assert!(python.contains("class __SnailStructuredAccessor"));
}
