#![allow(unsafe_op_in_unsafe_fn)]

mod compiler;
mod linecache;
mod lower;
mod profiling;

pub use lower::{
    lower_awk_program, lower_awk_program_with_auto_print, lower_map_program,
    lower_map_program_with_auto_print, lower_map_program_with_begin_end, lower_program,
    lower_program_with_auto_print,
};
pub use pyo3::prelude::{PyObject, Python};

use compiler::{compile_awk_source_with_begin_end, compile_map_source_with_begin_end};
use compiler::{compile_snail_source_with_auto_print, merge_map_cli_blocks};
use linecache::{display_filename, register_linecache, strip_display_prefix};
use profiling::{log_profile, profile_enabled};
use pyo3::Bound;
use pyo3::exceptions::{PyRuntimeError, PySyntaxError, PySystemExit};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use snail_ast::CompileMode;
use snail_error::{ParseError, format_snail_error};
use snail_parser::{
    parse_awk_program, parse_awk_program_with_begin_end, parse_map_program_with_begin_end,
    parse_program,
};
use std::time::Instant;

fn parse_mode(mode: &str) -> PyResult<CompileMode> {
    match mode {
        "snail" => Ok(CompileMode::Snail),
        "awk" => Ok(CompileMode::Awk),
        "map" => Ok(CompileMode::Map),
        _ => Err(PyRuntimeError::new_err(format!(
            "unknown mode: {mode} (expected 'snail', 'awk', or 'map')"
        ))),
    }
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
    begin_code: &[String],
    end_code: &[String],
) -> Result<PyObject, PyErr> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let compile_start = Instant::now();

    // If mode is awk/map and we have begin/end code, use the specialized function
    let module = if !begin_code.is_empty() || !end_code.is_empty() {
        let begin_refs: Vec<&str> = begin_code.iter().map(|s| s.as_str()).collect();
        let end_refs: Vec<&str> = end_code.iter().map(|s| s.as_str()).collect();
        match mode {
            CompileMode::Awk => {
                compile_awk_source_with_begin_end(py, source, &begin_refs, &end_refs, auto_print)
                    .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?
            }
            CompileMode::Map => {
                compile_map_source_with_begin_end(py, source, &begin_refs, &end_refs, auto_print)
                    .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?
            }
            _ => compile_snail_source_with_auto_print(py, source, mode, auto_print)
                .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?,
        }
    } else {
        compile_snail_source_with_auto_print(py, source, mode, auto_print)
            .map_err(|err| PySyntaxError::new_err(format_snail_error(&err, filename)))?
    };

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
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new()))]
fn compile_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
) -> PyResult<PyObject> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let mode = parse_mode(mode)?;
    let python_ast = compile_source(
        py,
        source,
        mode,
        auto_print,
        filename,
        &begin_code,
        &end_code,
    )?;
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
        log_profile("compile_py_total", total_start.elapsed());
    }
    Ok(code.unbind())
}

#[pyfunction(name = "compile_ast")]
#[pyo3(signature = (source, *, mode = "snail", auto_print = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new()))]
fn compile_ast_py(
    py: Python<'_>,
    source: &str,
    mode: &str,
    auto_print: bool,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
) -> PyResult<PyObject> {
    let mode = parse_mode(mode)?;
    let python_ast = compile_source(
        py,
        source,
        mode,
        auto_print,
        filename,
        &begin_code,
        &end_code,
    )?;
    Ok(python_ast)
}

#[pyfunction(name = "exec")]
#[pyo3(signature = (source, *, argv = Vec::new(), mode = "snail", auto_print = true, auto_import = true, filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new()))]
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
) -> PyResult<i32> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let mode = parse_mode(mode)?;
    let python_ast = compile_source(
        py,
        source,
        mode,
        auto_print,
        filename,
        &begin_code,
        &end_code,
    )?;
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
    let globals_start = Instant::now();
    let globals = prepare_globals(py, strip_display_prefix(filename), &argv, auto_import)?;
    if profile {
        log_profile("prepare_globals", globals_start.elapsed());
    }

    let exec_start = Instant::now();
    let exec_result = builtins.getattr("exec")?.call1((code.as_any(), &globals));
    if profile {
        log_profile("py_exec", exec_start.elapsed());
    }
    let result = match exec_result {
        Ok(_) => Ok(0),
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

#[allow(dead_code)]
#[derive(Debug)]
struct MapAst {
    program: snail_ast::Program,
    begin_blocks: Vec<Vec<snail_ast::Stmt>>,
    end_blocks: Vec<Vec<snail_ast::Stmt>>,
}

#[pyfunction(name = "parse_ast")]
#[pyo3(signature = (source, *, mode = "snail", filename = "<snail>", begin_code = Vec::new(), end_code = Vec::new()))]
fn parse_ast_py(
    source: &str,
    mode: &str,
    filename: &str,
    begin_code: Vec<String>,
    end_code: Vec<String>,
) -> PyResult<String> {
    let err_to_syntax =
        |err: ParseError| PySyntaxError::new_err(format_snail_error(&err.into(), filename));
    match parse_mode(mode)? {
        CompileMode::Snail => parse_program(source)
            .map(|program| format!("{:#?}", program))
            .map_err(err_to_syntax),
        CompileMode::Awk => {
            let program = if begin_code.is_empty() && end_code.is_empty() {
                parse_awk_program(source).map_err(err_to_syntax)?
            } else {
                let begin_refs: Vec<&str> = begin_code.iter().map(|s| s.as_str()).collect();
                let end_refs: Vec<&str> = end_code.iter().map(|s| s.as_str()).collect();
                parse_awk_program_with_begin_end(source, &begin_refs, &end_refs)
                    .map_err(err_to_syntax)?
            };
            Ok(format!("{:#?}", program))
        }
        CompileMode::Map => {
            let (program, begin_blocks, end_blocks) =
                parse_map_program_with_begin_end(source).map_err(err_to_syntax)?;
            let (begin_blocks, end_blocks) =
                merge_map_cli_blocks(&begin_code, &end_code, begin_blocks, end_blocks)
                    .map_err(err_to_syntax)?;

            if begin_blocks.is_empty() && end_blocks.is_empty() {
                return Ok(format!("{:#?}", program));
            }

            let map_ast = MapAst {
                program,
                begin_blocks,
                end_blocks,
            };
            Ok(format!("{:#?}", map_ast))
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
        CompileMode::Map => parse_map_program_with_begin_end(source)
            .map(|_| ())
            .map_err(|err| PySyntaxError::new_err(format_snail_error(&err.into(), filename))),
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
    module.add("__build_info__", build_info_dict(py)?)?;
    module.add(
        "__all__",
        vec![
            "compile",
            "compile_ast",
            "exec",
            "parse_ast",
            "parse",
            "__build_info__",
        ],
    )?;
    if profile {
        log_profile("module_init", total_start.elapsed());
    }
    Ok(())
}
