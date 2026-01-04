use std::io::Write;
use std::process::{Command, Stdio};

use snail::{lower_awk_program, parse_awk_program, python_source};

#[test]
fn awk_flag_filters_input() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "$l.startswith('a') { print($l) }"])
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
fn awk_directive_enables_mode() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "#!snail awk\nBEGIN { print('start') }\n$l.endswith('!')\nEND { print('done') }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hi\nthere!\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("start"));
    assert!(stdout.contains("there!"));
    assert!(stdout.contains("done"));
}

#[test]
fn awk_entries_allow_whitespace_separation() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "BEGIN {def err(x) { raise Exception(x) }} {print($0)}";

    let mut child = Command::new(exe)
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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
        .args(["--awk", "-c", source])
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

// Autodetection tests

#[test]
fn autodetect_begin_block() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "BEGIN { print('hello') }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"")
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
fn autodetect_end_block() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "END { print('goodbye') }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "goodbye\n");
}

#[test]
fn autodetect_bare_block() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "{ print($l) }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"test line\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "test line\n");
}

#[test]
fn autodetect_pattern_action() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "$l.startswith('a') { print('matched:', $l) }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"apple\nbanana\napricot\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "matched: apple\nmatched: apricot\n"
    );
}

#[test]
fn autodetect_pattern_only() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "/test/";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"testing\nno match\ntest again\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "testing\ntest again\n"
    );
}

#[test]
fn autodetect_no_false_positive_if_statement() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "x = True\nif x { print('hello') }";

    let output = Command::new(exe)
        .args(["-c", source])
        .output()
        .expect("spawn snail");

    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn autodetect_no_false_positive_function_def() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "def foo() { return 1 }\nprint(foo())";

    let output = Command::new(exe)
        .args(["-c", source])
        .output()
        .expect("spawn snail");

    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}

#[test]
fn autodetect_combined_awk_patterns() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source =
        "BEGIN { print('start') }\n$l.endswith('!') { print('found:', $l) }\nEND { print('done') }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"hello\nworld!\ntest\n")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("start"));
    assert!(stdout.contains("found: world!"));
    assert!(stdout.contains("done"));
}

#[test]
fn autodetect_shebang_still_takes_precedence() {
    let exe = env!("CARGO_BIN_EXE_snail");
    // This code looks like normal Snail but has awk shebang
    let source = "#!snail awk\nBEGIN { x = 1 }";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn autodetect_empty_program_handled() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "";

    let mut child = Command::new(exe)
        .args(["-c", source])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn snail");

    child
        .stdin
        .as_mut()
        .expect("stdin should be present")
        .write_all(b"")
        .expect("write input");

    let output = child.wait_with_output().expect("awk mode output");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}
