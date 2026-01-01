use std::io::Write;
use std::process::{Command, Stdio};

use snail::{PyBinaryOp, PyCompareOp, PyStmt, lower_program, parse_program, python_source};

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

#[test]
fn renders_python_golden_output() {
    let source = r"
import os as os_mod
from sys import path
class Greeter { def greet(name) { print('hi') } }
if x { y = 1 }
elif y { return y }
else { pass }
";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "import os as os_mod\nfrom sys import path\nclass Greeter:\n    def greet(name):\n        print(f'hi')\nif x:\n    y = 1\nelif y:\n    return y\nelse:\n    pass\n";
    assert_eq!(rendered, expected);
}

#[test]
fn round_trip_executes_small_program() {
    let source = "def fact(n) {\n    if n <= 1 { return 1 }\n    return n * fact(n - 1)\n}\nresult = fact(5)";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let python = python_source(&module);
    let code = format!("{}\nprint(result)", python);

    let output = Command::new("python3")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .as_mut()
                .expect("child stdin")
                .write_all(code.as_bytes())?;
            child.wait_with_output()
        })
        .expect("python should run");

    if !output.status.success() {
        panic!("python failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "120");
}

#[test]
fn renders_list_and_dict_comprehensions() {
    let source =
        "nums = [1, 2]\nvals = {n: n * 2 for n in nums if n > 1}\nlisty = [n for n in nums]";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected =
        "nums = [1, 2]\nvals = {n: (n * 2) for n in nums if (n > 1)}\nlisty = [n for n in nums]\n";
    assert_eq!(rendered, expected);
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
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "try:\n    risky()\nexcept ValueError as err:\n    raise err\nexcept:\n    raise\nelse:\n    ok = True\nfinally:\n    cleanup()\n";
    assert_eq!(rendered, expected);
}

#[test]
fn renders_try_finally_and_raise_from() {
    let source = r"
try { risky() }
finally { cleanup() }
raise ValueError('bad') from err
";
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected =
        "try:\n    risky()\nfinally:\n    cleanup()\nraise ValueError(f'bad') from err\n";
    assert_eq!(rendered, expected);
}

#[test]
fn renders_with_statement() {
    let source = r#"
with open("data") as f, lock() { line = f.read() }
"#;
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "with open(f\"data\") as f, lock():\n    line = f.read()\n";
    assert_eq!(rendered, expected);
}

#[test]
fn renders_assert_and_del() {
    let source = r#"
value = 1
assert value == 1, "ok"
del value
"#;
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "value = 1\nassert (value == 1), f\"ok\"\ndel value\n";
    assert_eq!(rendered, expected);
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
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "items = [1, 2, 3, 4]\npair = (1, 2)\nsingle = (1,)\nempty = ()\nflags = {True, False}\nmid = items[1:3]\nhead = items[:2]\ntail = items[2:]\n";
    assert_eq!(rendered, expected);
}

#[test]
fn renders_defaults_and_star_args() {
    let source = r#"
def join(a, b=1, *rest, **extras) { return a }
result = join(1, b=2, *rest, **extras)
"#;
    let program = parse_program(source).expect("program should parse");
    let module = lower_program(&program).expect("program should lower");
    let rendered = python_source(&module);
    let expected = "def join(a, b=1, *rest, **extras):\n    return a\nresult = join(1, b=2, *rest, **extras)\n";
    assert_eq!(rendered, expected);
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
