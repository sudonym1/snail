use snail::{Program, lower_program, parse_program, python_source};
use std::process::Command;

/// Parse source code and panic with a helpful message if it fails
pub fn assert_parses(source: &str) -> Program {
    parse_program(source).unwrap_or_else(|e| panic!("Failed to parse:\n{}\nError: {}", source, e))
}

/// Lower a program and panic with a helpful message if it fails
pub fn assert_lowers(program: &Program) -> snail::PyModule {
    lower_program(program).unwrap_or_else(|e| panic!("Failed to lower: {}", e))
}

/// Parse and lower source code in one step
pub fn snail_to_python(source: &str) -> String {
    let program = assert_parses(source);
    let module = assert_lowers(&program);
    python_source(&module)
}

/// Verify that Python code compiles without syntax errors
pub fn assert_python_compiles(python_code: &str) {
    // Use Python's compile() to check syntax without executing
    let check_code = format!(
        r#"import sys
try:
    compile({}, '<test>', 'exec')
except SyntaxError as e:
    print(f'SyntaxError: {{e}}', file=sys.stderr)
    sys.exit(1)"#,
        format_python_string(python_code)
    );

    let output = Command::new("python3")
        .arg("-c")
        .arg(&check_code)
        .output()
        .expect("failed to execute python3");

    assert!(
        output.status.success(),
        "Generated Python has syntax errors:\n{}\n\nStderr: {}",
        python_code,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Format a string for inclusion in Python code
fn format_python_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

/// Execute Snail source code and return a variable as JSON
#[allow(dead_code)]
pub fn execute_snail_get_var<T: serde::de::DeserializeOwned>(source: &str, var_name: &str) -> T {
    let python = snail_to_python(source);
    let code = format!(
        r#"import json
{}
print(json.dumps({}))"#,
        python, var_name
    );

    let output = Command::new("python3")
        .arg("-c")
        .arg(&code)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute Python:\n{}\nError: {}", code, e));

    assert!(
        output.status.success(),
        "Execution failed:\n{}\nStderr: {}",
        code,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from {}: {}\nOutput: {}",
            var_name, e, stdout
        )
    })
}

/// Execute Snail source code with setup and return a variable as JSON
#[allow(dead_code)]
pub fn execute_snail_with_setup_get_var<T: serde::de::DeserializeOwned>(
    source: &str,
    setup: &str,
    var_name: &str,
) -> T {
    let python = snail_to_python(source);
    let code = format!(
        r#"import json
{}
{}
print(json.dumps({}))"#,
        setup, python, var_name
    );

    let output = Command::new("python3")
        .arg("-c")
        .arg(&code)
        .output()
        .unwrap_or_else(|e| panic!("Failed to execute Python:\n{}\nError: {}", code, e));

    assert!(
        output.status.success(),
        "Execution failed:\n{}\nStderr: {}",
        code,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from {}: {}\nOutput: {}",
            var_name, e, stdout
        )
    })
}
