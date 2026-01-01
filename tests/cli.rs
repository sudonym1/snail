use std::process::Command;

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
