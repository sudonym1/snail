use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn awk_flag_filters_input() {
    let exe = env!("CARGO_BIN_EXE_snail");
    let mut child = Command::new(exe)
        .args(["--awk", "-c", "line.startswith('a') { print(line) }"])
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
    let source = "#!snail awk\nBEGIN { print('start') }\nline.endswith('!')\nEND { print('done') }";

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
