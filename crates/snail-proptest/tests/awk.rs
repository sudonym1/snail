#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use snail_proptest::arbitrary::*;

// ========== AWK-Specific Properties ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn awk_programs_always_lower(awk_program in awk_program()) {
        // AWK programs should always lower or return a LowerError (not panic)
        let _ = snail_lower::lower_awk_program(&awk_program);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn awk_programs_generate_valid_python(awk_program in awk_program()) {
        if let Ok(module) = snail_lower::lower_awk_program(&awk_program) {
            let python_code = snail_codegen::python_source(&module);

            // Should compile as valid Python
            let check_code = format!(
                r#"import sys
try:
    compile({}, '<test>', 'exec')
except SyntaxError as e:
    print(f'SyntaxError: {{e}}', file=sys.stderr)
    sys.exit(1)"#,
                format_python_string(&python_code)
            );

            let output = std::process::Command::new("python3")
                .arg("-c")
                .arg(&check_code)
                .output()
                .expect("failed to execute python3");

            assert!(
                output.status.success(),
                "AWK generated Python has syntax errors:\n{}\n\nStderr: {}",
                python_code,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

fn format_python_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
