use std::fs;
use std::io::Write;
use std::process::Command;

use tempfile::{NamedTempFile, tempdir};

#[test]
fn passes_args_to_script_with_c_flag() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "import sys; print(sys.argv)", "arg1", "arg2"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "['-c', 'arg1', 'arg2']");
}

#[test]
fn passes_args_to_script_with_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut file = NamedTempFile::with_suffix(".snail").expect("create temp file");
    writeln!(file, "import sys; print(sys.argv)").expect("write to temp file");
    let path = file.path().to_str().unwrap();

    let output = Command::new(exe)
        .args([path, "arg1", "arg2"])
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
        .args([path, "-x", "--foo", "bar"])
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
fn combined_short_flags_work() {
    let exe = env!("CARGO_BIN_EXE_snail");
    // -pc should be equivalent to -p -c
    let output = Command::new(exe)
        .args(["-pc", "print('hello')"])
        .output()
        .expect("run snail");
    assert!(
        output.status.success(),
        "snail failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // -p outputs Python code, so we should see the translated print
    assert!(stdout.contains("print"), "expected python output: {stdout}");
}

#[test]
fn flushes_stdout_on_exit() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "import sys; sys.stdout.write('hi')"])
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
        .args(["-c", "import sys; sys.stderr.write('oops')"])
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
    let output = Command::new(exe)
        .args(["-c", "if { }"])
        .output()
        .expect("should run");

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

    let output = Command::new(exe).arg(path).output().expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("Error"));
}

#[test]
fn cli_handles_empty_input_with_c_flag() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", ""])
        .output()
        .expect("should run");

    // Empty input should succeed (nothing to execute)
    assert!(output.status.success());
}

#[test]
fn cli_reports_runtime_errors() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "raise ValueError('test error')"])
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
        .args(["-c", "x = 1 +"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
}

#[test]
fn format_check_outputs_diff() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("example.snail");
    fs::write(&path, "value = 1  \n").expect("write file");

    let output = Command::new(exe)
        .arg("--format")
        .arg(path.to_str().unwrap())
        .current_dir(dir.path())
        .output()
        .expect("run snail");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("---"));
    assert!(stdout.contains("+++"));
    assert!(stdout.contains("value = 1"));
}

#[test]
fn format_write_updates_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("example.snail");
    fs::write(&path, "value = 1  \nsecond\t\tline\t\t").expect("write file");

    let output = Command::new(exe)
        .arg("--format")
        .arg("--write")
        .arg(path.to_str().unwrap())
        .current_dir(dir.path())
        .output()
        .expect("run snail");

    assert!(output.status.success());
    let contents = fs::read_to_string(&path).expect("read formatted file");
    assert_eq!(contents, "value = 1\nsecond\t\tline\n");
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
fn cli_handles_missing_argument_for_c_flag() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe).arg("-c").output().expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

#[test]
fn cli_reports_multiline_parse_errors_correctly() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let source = "x = 1\ny = 2\nz = 3\nif {";
    let output = Command::new(exe)
        .args(["-c", source])
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
        .args(["-c", "café = 1 +"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
}

#[test]
fn cli_exits_with_nonzero_on_parse_error() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe)
        .args(["-c", "if {"])
        .output()
        .expect("should run");

    assert!(!output.status.success());
    assert!(output.status.code().is_some());
    assert_ne!(output.status.code().unwrap(), 0);
}

#[test]
fn cli_handles_directory_instead_of_file() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let output = Command::new(exe).arg("/tmp").output().expect("should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}
