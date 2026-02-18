#![allow(unsafe_op_in_unsafe_fn)]

mod compiler;
mod linecache;
mod lower;
mod profiling;

pub use lower::{lower_program, lower_program_auto, lower_program_main};
pub use pyo3::prelude::{PyObject, Python};

use linecache::{display_filename, register_linecache, strip_display_prefix};
use profiling::{log_profile, profile_enabled};
use pyo3::Bound;
use pyo3::exceptions::{PyRuntimeError, PySyntaxError, PySystemExit};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use snail_ast::Stmt;
use snail_error::{ParseError, format_snail_error};
use snail_parser::preprocess;
use std::time::Instant;

fn parse_error_to_syntax(err: ParseError, filename: &str) -> PyErr {
    PySyntaxError::new_err(format_snail_error(&err.into(), filename))
}

#[allow(clippy::too_many_arguments)]
fn compile_source_to_code(
    py: Python<'_>,
    source: &str,
    auto_print: bool,
    capture_last: bool,
    filename: &str,
) -> PyResult<PyObject> {
    let profile = profile_enabled();
    let python_ast = compile_source(py, source, auto_print, capture_last, filename)?;
    let display = display_filename(filename);
    let linecache_start = Instant::now();
    register_linecache(py, &display, source)?;
    if profile {
        log_profile("register_linecache", linecache_start.elapsed());
    }
    let compile_start = Instant::now();
    let builtins = py.import_bound("builtins")?;
    let code = builtins
        .getattr("compile")?
        .call1((python_ast, display, "exec"))?;
    if profile {
        log_profile("py_compile", compile_start.elapsed());
    }
    Ok(code.unbind())
}

fn build_info_dict(py: Python<'_>) -> PyResult<PyObject> {
    let info = pyo3::types::PyDict::new_bound(py);
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
    auto_print: bool,
    capture_last: bool,
    filename: &str,
) -> Result<PyObject, PyErr> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let compile_start = Instant::now();

    let module = compiler::compile_source(py, source, auto_print, capture_last)
        .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?;

    if profile {
        log_profile("compile_snail_source", compile_start.elapsed());
    }
    let ast_start = Instant::now();
    let ast = py.import_bound("ast")?;
    let fixed = ast
        .getattr("fix_missing_locations")?
        .call1((module.clone_ref(py),))?;
    if profile {
        log_profile("fix_missing_locations", ast_start.elapsed());
        log_profile("compile_source_total", total_start.elapsed());
    }
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
        pyo3::types::PyDict::new_bound(py).into_any()
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

/// Check whether the program ends with a non-semicolon-terminated expression.
fn has_tail_expression(source: &str) -> bool {
    snail_parser::parse(source)
        .map(|program| {
            matches!(
                program
                    .stmts
                    .iter()
                    .rev()
                    .find(|s| !matches!(s, Stmt::SegmentBreak { .. })),
                Some(Stmt::Expr {
                    semicolon_terminated: false,
                    ..
                })
            )
        })
        .unwrap_or(false)
}

#[pyfunction(name = "compile")]
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new(), field_separators = None, include_whitespace = None))]
#[allow(clippy::too_many_arguments)]
fn compile_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
    field_separators: Option<String>,
    include_whitespace: Option<bool>,
) -> PyResult<PyObject> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let wrapped = wrap_source(
        source,
        mode,
        &begin_code,
        &end_code,
        field_separators.as_deref(),
        include_whitespace,
    )?;
    let code = compile_source_to_code(py, &wrapped, auto_print, false, filename)?;
    if profile {
        log_profile("compile_py_total", total_start.elapsed());
    }
    Ok(code)
}

#[pyfunction(name = "compile_ast")]
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new(), field_separators = None, include_whitespace = None))]
#[allow(clippy::too_many_arguments)]
fn compile_ast_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
    field_separators: Option<String>,
    include_whitespace: Option<bool>,
) -> PyResult<PyObject> {
    let wrapped = wrap_source(
        source,
        mode,
        &begin_code,
        &end_code,
        field_separators.as_deref(),
        include_whitespace,
    )?;
    compile_source(py, &wrapped, auto_print, false, filename)
}

#[pyfunction(name = "exec")]
#[pyo3(signature = (source, *, argv = Vec::new(), mode = "snail", auto_print = true, auto_import = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new(), field_separators = None, include_whitespace = None, test_last = false))]
#[allow(clippy::too_many_arguments)]
fn exec_py(
    py: Python<'_>,
    source: &str,
    argv: Vec<String>,
    mode: &str,
    auto_print: bool,
    auto_import: bool,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
    field_separators: Option<String>,
    include_whitespace: Option<bool>,
    test_last: bool,
) -> PyResult<i32> {
    let profile = profile_enabled();
    let total_start = Instant::now();

    let wrapped = wrap_source(
        source,
        mode,
        &begin_code,
        &end_code,
        field_separators.as_deref(),
        include_whitespace,
    )?;

    // Pre-flight check: --test requires a trailing expression.
    if test_last && !has_tail_expression(&wrapped) {
        let sys = py.import_bound("sys")?;
        let stderr = sys.getattr("stderr")?;
        stderr.call_method1("write", ("snail: --test requires a trailing expression\n",))?;
        return Ok(2);
    }

    let capture_last = test_last && !auto_print;
    let code = compile_source_to_code(py, &wrapped, auto_print, capture_last, filename)?;
    let builtins = py.import_bound("builtins")?;
    let globals_start = Instant::now();
    let globals = prepare_globals(py, strip_display_prefix(filename), &argv, auto_import)?;
    if profile {
        log_profile("prepare_globals", globals_start.elapsed());
    }

    let exec_start = Instant::now();
    let exec_result = builtins.getattr("exec")?.call1((code.bind(py), &globals));
    if profile {
        log_profile("py_exec", exec_start.elapsed());
    }
    let result = match exec_result {
        Ok(_) => {
            if test_last {
                let key = "__snail_last_result";
                let value = globals.get_item(key)?;
                if value.is_truthy()? { Ok(0) } else { Ok(1) }
            } else {
                Ok(0)
            }
        }
        Err(err) => {
            if err.is_instance_of::<PySystemExit>(py) {
                let code = err.value_bound(py).getattr("code")?;
                if code.is_none() {
                    Ok(0)
                } else if let Ok(exit_code) = code.extract::<i32>() {
                    Ok(exit_code)
                } else {
                    Ok(1)
                }
            } else {
                Err(err)
            }
        }
    };
    if profile {
        log_profile("exec_py_total", total_start.elapsed());
    }
    result
}

#[pyfunction(name = "parse_ast")]
#[pyo3(signature = (source, *, mode = "snail", filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new(), field_separators = None, include_whitespace = None))]
#[allow(clippy::too_many_arguments)]
fn parse_ast_py(
    source: &str,
    mode: &str,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
    field_separators: Option<String>,
    include_whitespace: Option<bool>,
) -> PyResult<String> {
    let wrapped = wrap_source(
        source,
        mode,
        &begin_code,
        &end_code,
        field_separators.as_deref(),
        include_whitespace,
    )?;
    let program = compiler::parse_program(&wrapped)
        .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?;
    Ok(format!("{:#?}", program))
}

#[pyfunction(name = "preprocess")]
fn preprocess_py(source: &str) -> PyResult<String> {
    preprocess::preprocess(source).map_err(|err| parse_error_to_syntax(err, "<snail>"))
}

#[pyfunction(name = "parse")]
#[pyo3(signature = (source, *, mode = "snail", filename = "<snail>", field_separators = None, include_whitespace = None))]
fn parse_py(
    source: &str,
    mode: &str,
    filename: &str,
    field_separators: Option<String>,
    include_whitespace: Option<bool>,
) -> PyResult<()> {
    let wrapped = wrap_source(
        source,
        mode,
        &[],
        &[],
        field_separators.as_deref(),
        include_whitespace,
    )?;
    snail_parser::parse(&wrapped)
        .map(|_| ())
        .map_err(|err| parse_error_to_syntax(err, filename))
}

/// Wrap source code based on mode. For awk mode, wraps in `lines { ... }`.
/// For map mode, wraps in `files { ... }`. Begin/end code is prepended/appended.
fn wrap_source(
    source: &str,
    mode: &str,
    begin_code: &[String],
    end_code: &[String],
    field_separators: Option<&str>,
    include_whitespace: Option<bool>,
) -> PyResult<String> {
    match mode {
        "snail" => {
            if begin_code.is_empty() && end_code.is_empty() {
                Ok(source.to_string())
            } else {
                let mut parts: Vec<&str> = Vec::new();
                for b in begin_code {
                    parts.push(b);
                }
                parts.push(source);
                for e in end_code {
                    parts.push(e);
                }
                Ok(parts.join("\n\x1f"))
            }
        }
        "awk" => {
            let mut segments: Vec<String> = Vec::new();
            for b in begin_code {
                segments.push(b.clone());
            }
            // Build kwargs for lines()
            let mut kwargs = Vec::new();
            if let Some(sep) = field_separators {
                let escaped = lower::escape_for_python_string(sep);
                kwargs.push(format!("sep=\"{escaped}\""));
            }
            if let Some(ws) = include_whitespace
                && ws
            {
                kwargs.push("ws=True".to_string());
            }
            let kwargs_str = if kwargs.is_empty() {
                String::new()
            } else {
                format!("({})", kwargs.join(", "))
            };
            segments.push(format!("lines{kwargs_str} {{\n{source}\n}}"));
            for e in end_code {
                segments.push(e.clone());
            }
            Ok(segments.join("\n\x1f"))
        }
        "map" => {
            let mut segments: Vec<String> = Vec::new();
            for b in begin_code {
                segments.push(b.clone());
            }
            segments.push(format!("files {{\n{source}\n}}"));
            for e in end_code {
                segments.push(e.clone());
            }
            Ok(segments.join("\n\x1f"))
        }
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown mode: {mode} (expected 'snail', 'awk', or 'map')"
        ))),
    }
}

#[pymodule]
fn _native(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    module.add_function(wrap_pyfunction!(compile_py, module)?)?;
    module.add_function(wrap_pyfunction!(compile_ast_py, module)?)?;
    module.add_function(wrap_pyfunction!(exec_py, module)?)?;
    module.add_function(wrap_pyfunction!(parse_ast_py, module)?)?;
    module.add_function(wrap_pyfunction!(parse_py, module)?)?;
    module.add_function(wrap_pyfunction!(preprocess_py, module)?)?;
    module.add("__build_info__", build_info_dict(py)?)?;
    module.add(
        "__all__",
        vec![
            "compile",
            "compile_ast",
            "exec",
            "parse_ast",
            "parse",
            "preprocess",
            "__build_info__",
        ],
    )?;
    if profile {
        log_profile("module_init", total_start.elapsed());
    }
    Ok(())
}
