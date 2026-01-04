use std::io::Write;
use std::process::{Command, Stdio};

use snail::{lower_awk_program, parse_awk_program, python_source};

#[test]
fn awk_flag_filters_input() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "$l.startswith('a') { print($l) }"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    let stdin = child.stdin.as_mut().expect("stdin should be available");
    stdin
        .write_all(b"alpha\nbeta\napple\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "alpha\napple\n");
}

#[test]
fn awk_entries_allow_whitespace_separation() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "BEGIN {def err(x) { raise Exception(x) }} {print($0)}";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn awk_fstring_interpolates_field_vars() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print(\"{$1}\") }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn awk_subprocess_interpolates_field_vars_in_string_literals() {
    let source = r#"{ print("{$($2)}") }"#;
    let program = parse_awk_program(source).expect("awk program should parse");
    let module = lower_awk_program(&program).expect("awk program should lower");
    let python = python_source(&module);

    assert!(python.contains("__snail_subprocess_capture"));
    assert!(python.contains("__snail_fields[1]"));
}

#[test]
fn awk_regex_pattern_sets_match() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "/a.+c/ { print($m.group(0)) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"abc\nzzz\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "abc\n");
}

#[test]
fn awk_field_index_zero_is_whole_line() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($0) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello world\n");
}

#[test]
fn awk_field_index_basic() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($1, $2) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello world\n");
}

#[test]
fn awk_field_index_high_numbers() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($10) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"0 1 2 3 4 5 6 7 8 9 10 11\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "9\n");
}

#[test]
fn awk_field_index_mixed_with_injected_vars() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($0, $1, len($f)) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello world\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello world hello 2\n"
    );
}

#[test]
fn awk_field_index_in_expressions() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($1 + ' ' + $2) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"a b c\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "a b\n");
}

#[test]
fn awk_field_index_multiple_lines() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($2) }";

    let mut child = Command::new(exe)
        .args(["--awk", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"first second\nthird fourth\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "second\nfourth\n");
}
