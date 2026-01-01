use snail::parse_program;

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
}

#[test]
fn parses_semicolon_before_newline() {
    let source = "x = 1;\ny = 2";
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
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
fn parses_if_elif_else_chain() {
    let source = r#"
if x { y = 1 }
elif y { y = 2 }
else { y = 3 }
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 1);
}

#[test]
fn parses_def_and_call() {
    let source = r#"
def add(a, b) { return a + b }
result = add(1, 2)
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
}

#[test]
fn parses_imports() {
    let source = r#"
import sys, os as operating_system
from collections import deque, defaultdict as dd
"#;
    let program = parse_program(source).expect("program should parse");
    assert_eq!(program.stmts.len(), 2);
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
