#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::Bound;
use pyo3::exceptions::{PyRuntimeError, PySyntaxError, PySystemExit};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use snail_core::{
    CompileMode, compile_snail_source_with_auto_print, format_snail_error, parse_awk_program,
    parse_program,
};

fn parse_mode(mode: &str) -> PyResult<CompileMode> {
    match mode {
        "snail" => Ok(CompileMode::Snail),
        "awk" => Ok(CompileMode::Awk),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown mode: {mode} (expected 'snail' or 'awk')"
        ))),
    }
}

fn compile_source(
    source: &str,
    mode: CompileMode,
    auto_print: bool,
    filename: &str,
) -> Result<String, PyErr> {
    compile_snail_source_with_auto_print(source, mode, auto_print)
        .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))
}

fn prepare_globals<'py>(
    py: Python<'py>,
    filename: &str,
    argv: &[String],
) -> PyResult<Bound<'py, PyDict>> {
    let globals = PyDict::new_bound(py);
    let builtins = py.import_bound("builtins")?;
    globals.set_item("__builtins__", &builtins)?;
    globals.set_item("__name__", "__main__")?;
    globals.set_item("__file__", filename)?;

    let sys = py.import_bound("sys")?;
    sys.setattr("argv", PyList::new_bound(py, argv))?;

    let runtime = py.import_bound("snail.runtime")?;
    runtime.call_method1("install_helpers", (globals.as_any(),))?;

    Ok(globals)
}

#[pyfunction(name = "compile")]
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>"))]
fn compile_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
) -> PyResult<PyObject> {
    let mode = parse_mode(mode)?;
    let python_source = compile_source(source, mode, auto_print, filename)?;
    let builtins = py.import_bound("builtins")?;
    let code = builtins
        .getattr("compile")?
        .call1((python_source, filename, "exec"))?;
    Ok(code.unbind())
}

#[pyfunction(name = "exec")]
#[pyo3(signature = (source, *, argv = Vec::new(), mode = "snail", auto_print = true, filename = "<snail>"))]
fn exec_py(
    py: Python<'_>,
    source: &str,
    argv: Vec<String>,
    mode: &str,
    auto_print: bool,
    filename: &str,
) -> PyResult<i32> {
    let mode = parse_mode(mode)?;
    let python_source = compile_source(source, mode, auto_print, filename)?;
    let builtins = py.import_bound("builtins")?;
    let code = builtins
        .getattr("compile")?
        .call1((python_source, filename, "exec"))?;
    let globals = prepare_globals(py, filename, &argv)?;

    match builtins
        .getattr("exec")?
        .call1((code.as_any(), globals.as_any()))
    {
        Ok(_) => Ok(0),
        Err(err) => {
            if err.is_instance_of::<PySystemExit>(py) {
                let code = err.value_bound(py).getattr("code")?;
                if code.is_none() {
                    return Ok(0);
                }
                if let Ok(exit_code) = code.extract::<i32>() {
                    return Ok(exit_code);
                }
                Ok(1)
            } else {
                Err(err)
            }
        }
    }
}

#[pyfunction(name = "parse")]
#[pyo3(signature = (source, *, mode = "snail", filename = "<snail>"))]
fn parse_py(source: &str, mode: &str, filename: &str) -> PyResult<()> {
    match parse_mode(mode)? {
        CompileMode::Snail => parse_program(source)
            .map(|_| ())
            .map_err(|err| PySyntaxError::new_err(format_snail_error(&err.into(), filename))),
        CompileMode::Awk => parse_awk_program(source)
            .map(|_| ())
            .map_err(|err| PySyntaxError::new_err(format_snail_error(&err.into(), filename))),
    }
}

#[pymodule]
fn _native(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(compile_py, module)?)?;
    module.add_function(wrap_pyfunction!(exec_py, module)?)?;
    module.add_function(wrap_pyfunction!(parse_py, module)?)?;
    module.add("__all__", vec!["compile", "exec", "parse"])?;
    Ok(())
}
