use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

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
