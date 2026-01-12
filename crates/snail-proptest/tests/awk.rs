#![cfg(feature = "run-proptests")]

use proptest::prelude::*;
use pyo3::prelude::*;
use snail_proptest::arbitrary::*;

fn assert_python_compiles(py: Python<'_>, module: &PyObject) {
    let ast = py.import_bound("ast").expect("failed to import ast");
    let fixed = ast
        .getattr("fix_missing_locations")
        .and_then(|fix| fix.call1((module.clone_ref(py),)))
        .expect("failed to fix locations");
    let builtins = py
        .import_bound("builtins")
        .expect("failed to import builtins");
    builtins
        .getattr("compile")
        .and_then(|compile| compile.call1((fixed, "<test>", "exec")))
        .expect("Generated Python AST has syntax errors");
}

// ========== AWK-Specific Properties ==========

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn awk_programs_always_lower(awk_program in awk_program()) {
        // AWK programs should always lower or return a LowerError (not panic)
        Python::with_gil(|py| {
            let _ = snail_lower::lower_awk_program(py, &awk_program);
        });
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn awk_programs_generate_valid_python(awk_program in awk_program()) {
        Python::with_gil(|py| {
            if let Ok(module) = snail_lower::lower_awk_program(py, &awk_program) {
                assert_python_compiles(py, &module);
            }
        });
    }
}
