use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

#[test]
fn passes_args_to_script_with_code_arg() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["import sys; print(sys.argv)", "arg1", "arg2"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "['--', 'arg1', 'arg2']");
}

#[test]
fn passes_args_to_script_with_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "import sys; print(sys.argv)").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-f", path, "arg1", "arg2"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("'arg1', 'arg2'"),
        "expected args in output: {stdout}"
    );
}

#[test]
fn passes_hyphen_args_to_script() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "import sys; print(sys.argv)").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-f", path, "-x", "--foo", "bar"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("'-x', '--foo', 'bar'"),
        "expected hyphen args in output: {stdout}"
    );
}

#[test]
fn flushes_stdout_on_exit() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["import sys; x = sys.stdout.write('hi')"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"hi");
}

#[test]
fn flushes_stderr_on_exit() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["import sys; sys.stderr.write('oops')"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"oops");
}

// ========== Error Path Tests ==========

#[test]
fn cli_reports_file_not_found() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .arg("-f")
        .arg("/nonexistent/path/to/file.snail")
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to read")
            || stderr.contains("No such file")
            || stderr.contains("not found")
    );
}

#[test]
fn cli_reports_parse_errors_with_location() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe).args([")"]).output().expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("Error"));
    assert!(stderr.contains("-->") || stderr.contains("at"));
}

#[test]
fn cli_reports_parse_error_in_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "x = 1\nif {{ }}").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-f", path])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("Error"));
}

#[test]
fn cli_handles_empty_input() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe).args([""]).output().expect("should run");

    // Empty input should succeed (nothing to execute)
    assert!(output.status.success());
}

#[test]
fn cli_reports_runtime_errors() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["raise ValueError('test error')"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ValueError") || stderr.contains("test error"));
}

#[test]
fn cli_handles_syntax_error_in_generated_python() {
    let exe = env!("CARGO_BIN_EXE_snail");
    // This should parse correctly in Snail but might have issues
    let output = Command::new(exe)
        .args(["x = 1 +"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
}

#[test]
fn cli_handles_invalid_flag() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .arg("--invalid-flag-xyz")
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain some error about invalid flag/argument
    assert!(!stderr.is_empty());
}

#[test]
fn cli_reports_multiline_parse_errors_correctly() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "x = 1\ny = 2\nz = 3\nif {";
    let output = Command::new(exe)
        .args([source])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error"));
    // Should ideally show line 4 where the error is
}

#[test]
fn cli_handles_unicode_in_error_messages() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["café = 1 +"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
}

#[test]
fn cli_exits_with_nonzero_on_parse_error() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["if { }"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    assert!(output.status.code().is_some());
    assert_ne!(output.status.code().unwrap(), 0);
}

#[test]
fn cli_handles_directory_instead_of_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-f", "/tmp"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

// ========== Auto-Print Tests ==========

#[test]
fn auto_prints_last_expression_simple_number() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe).args(["42"]).output().expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn auto_prints_last_expression_list() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["[1, 2, 3]"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "[1, 2, 3]");
}

#[test]
fn auto_prints_last_expression_dict() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["{'a': 1, 'b': 2}"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("'a'") && stdout.contains("'b'"));
}

#[test]
fn auto_prints_expression_after_statements() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["x = 42; x + 1"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "43");
}

#[test]
fn does_not_print_none_from_function_call() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["print('hello')"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "hello");
}

#[test]
fn does_not_print_when_last_is_statement() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["x = 42; y = x + 1"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "");
}

#[test]
fn auto_prints_string_expression() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["'hello world'"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "'hello world'");
}

#[test]
fn auto_prints_when_running_from_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "42").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-f", path])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // File mode should auto-print by default
    assert_eq!(stdout.trim(), "42");
}

#[test]
fn auto_prints_file_with_expression_after_statements() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "x = 10").expect("write to temp file");
    writeln!(file, "y = 20").expect("write to temp file");
    writeln!(file, "x + y").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-f", path])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // File mode should auto-print by default
    assert_eq!(stdout.trim(), "30");
}

#[test]
fn flag_p_disables_auto_print_for_one_liner() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-P", "42"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "");
}

#[test]
fn flag_p_disables_auto_print_for_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "42").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args(["-P", "-f", path])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "");
}
