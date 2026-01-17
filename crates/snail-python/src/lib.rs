#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::Bound;
use pyo3::exceptions::{PyRuntimeError, PySyntaxError, PySystemExit};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyTuple};
use snail_core::{
    CompileMode, compile_snail_source_with_auto_print, format_snail_error, parse_awk_program,
    parse_program,
};

const SNAIL_TRACE_PREFIX: &str = "snail:";

fn parse_mode(mode: &str) -> PyResult<CompileMode> {
    match mode {
        "snail" => Ok(CompileMode::Snail),
        "awk" => Ok(CompileMode::Awk),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown mode: {mode} (expected 'snail' or 'awk')"
        ))),
    }
}

fn display_filename(filename: &str) -> String {
    if filename.starts_with(SNAIL_TRACE_PREFIX) {
        filename.to_string()
    } else {
        format!("{SNAIL_TRACE_PREFIX}{filename}")
    }
}

fn strip_display_prefix(filename: &str) -> &str {
    filename
        .strip_prefix(SNAIL_TRACE_PREFIX)
        .unwrap_or(filename)
}

fn split_source_lines(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            let end = idx + 1;
            lines.push(source[start..end].to_string());
            start = end;
        }
    }
    if start < source.len() {
        lines.push(source[start..].to_string());
    }
    lines
}

fn register_linecache(py: Python<'_>, filename: &str, source: &str) -> PyResult<()> {
    let linecache = py.import_bound("linecache")?;
    let cache = linecache.getattr("cache")?;
    let lines = split_source_lines(source);
    let entry = PyTuple::new_bound(
        py,
        vec![
            source.len().into_py(py),
            py.None().into_py(py),
            PyList::new_bound(py, lines).into_py(py),
            filename.into_py(py),
        ],
    );
    cache.set_item(filename, entry)?;
    Ok(())
}

fn build_info_dict(py: Python<'_>) -> PyResult<PyObject> {
    let info = PyDict::new_bound(py);
    if let Some(rev) = option_env!("SNAIL_GIT_SHA") {
        info.set_item("git_rev", rev)?;
    }
    if let Some(dirty) = option_env!("SNAIL_GIT_DIRTY") {
        info.set_item("dirty", dirty == "true")?;
    }
    if let Some(untagged) = option_env!("SNAIL_GIT_UNTAGGED") {
        info.set_item("untagged", untagged == "true")?;
    }
    Ok(info.into_py(py))
}

fn compile_source(
    py: Python<'_>,
    source: &str,
    mode: CompileMode,
    auto_print: bool,
    filename: &str,
) -> Result<PyObject, PyErr> {
    let module = compile_snail_source_with_auto_print(py, source, mode, auto_print)
        .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?;
    let ast = py.import_bound("ast")?;
    let fixed = ast
        .getattr("fix_missing_locations")?
        .call1((module.clone_ref(py),))?;
    Ok(fixed.into_py(py))
}

fn prepare_globals<'py>(
    py: Python<'py>,
    filename: &str,
    argv: &[String],
    auto_import: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let runtime = py.import_bound("snail.runtime")?;

    // Create either an AutoImportDict or a regular dict
    let globals: Bound<'py, PyAny> = if auto_import {
        runtime.getattr("AutoImportDict")?.call0()?
    } else {
        PyDict::new_bound(py).into_any()
    };

    let builtins = py.import_bound("builtins")?;
    globals.set_item("__builtins__", &builtins)?;
    globals.set_item("__name__", "__main__")?;
    globals.set_item("__file__", filename)?;

    let sys = py.import_bound("sys")?;
    sys.setattr("argv", PyList::new_bound(py, argv))?;

    runtime.call_method1("install_helpers", (&globals,))?;

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
    let python_ast = compile_source(py, source, mode, auto_print, filename)?;
    let display = display_filename(filename);
    register_linecache(py, &display, source)?;
    let builtins = py.import_bound("builtins")?;
    let code = builtins
        .getattr("compile")?
        .call1((python_ast, display, "exec"))?;
    Ok(code.unbind())
}

#[pyfunction(name = "compile_ast")]
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>"))]
fn compile_ast_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
) -> PyResult<PyObject> {
    let mode = parse_mode(mode)?;
    let python_ast = compile_source(py, source, mode, auto_print, filename)?;
    Ok(python_ast)
}

#[pyfunction(name = "exec")]
#[pyo3(signature = (source, *, argv = Vec::new(), mode = "snail", auto_print = true, auto_import = true, filename = "<snail>"))]
fn exec_py(
    py: Python<'_>,
    source: &str,
    argv: Vec<String>,
    mode: &str,
    auto_print: bool,
    auto_import: bool,
    filename: &str,
) -> PyResult<i32> {
    let mode = parse_mode(mode)?;
    let python_ast = compile_source(py, source, mode, auto_print, filename)?;
    let display = display_filename(filename);
    register_linecache(py, &display, source)?;
    let builtins = py.import_bound("builtins")?;
    let code = builtins
        .getattr("compile")?
        .call1((python_ast, display, "exec"))?;
    let globals = prepare_globals(py, strip_display_prefix(filename), &argv, auto_import)?;

    match builtins.getattr("exec")?.call1((code.as_any(), &globals)) {
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
    module.add_function(wrap_pyfunction!(compile_ast_py, module)?)?;
    module.add_function(wrap_pyfunction!(exec_py, module)?)?;
    module.add_function(wrap_pyfunction!(parse_py, module)?)?;
    module.add("__build_info__", build_info_dict(_py)?)?;
    module.add(
        "__all__",
        vec!["compile", "compile_ast", "exec", "parse", "__build_info__"],
    )?;
    Ok(())
}
