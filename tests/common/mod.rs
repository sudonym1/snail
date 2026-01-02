use pyo3::prelude::*;
use pyo3::types::PyDict;
use snail::{Program, lower_program, parse_program, python_source};

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
    Python::with_gil(|py| {
        let result = PyModule::import_bound(py, "builtins")
            .unwrap()
            .getattr("compile")
            .unwrap()
            .call1((python_code, "<test>", "exec"));

        assert!(
            result.is_ok(),
            "Generated Python has syntax errors:\n{}\n\nError: {:?}",
            python_code,
            result.unwrap_err()
        );
    });
}

/// Execute Snail source code and return the Python globals dict
#[allow(dead_code)]
pub fn execute_snail(source: &str) -> Py<PyDict> {
    Python::with_gil(|py| {
        let python = snail_to_python(source);
        let globals = PyDict::new_bound(py);
        py.run_bound(&python, None, Some(&globals))
            .unwrap_or_else(|e| panic!("Execution failed:\n{}\nError: {:?}", python, e));
        globals.into()
    })
}

/// Execute Snail source code with a setup function and return the Python globals
#[allow(dead_code)]
pub fn execute_snail_with_setup(source: &str, setup: &str) -> Py<PyDict> {
    Python::with_gil(|py| {
        let python = snail_to_python(source);
        let code = format!("{}\n{}", setup, python);
        let globals = PyDict::new_bound(py);
        py.run_bound(&code, None, Some(&globals))
            .unwrap_or_else(|e| panic!("Execution failed:\n{}\nError: {:?}", code, e));
        globals.into()
    })
}

/// Get a Python variable value from globals dict
pub fn get_py_var<'py, T: FromPyObject<'py>>(
    _py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    name: &str,
) -> T {
    globals
        .get_item(name)
        .unwrap_or_else(|e| panic!("Failed to get item {}: {:?}", name, e))
        .unwrap_or_else(|| panic!("Variable {} not found in globals", name))
        .extract()
        .unwrap_or_else(|e| panic!("Failed to extract {}: {:?}", name, e))
}
