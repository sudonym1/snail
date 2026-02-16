#![allow(unsafe_op_in_unsafe_fn)]

mod compiler;
mod linecache;
mod lower;
mod profiling;

pub use lower::{
    lower_awk, lower_awk_main, lower_map, lower_map_auto, lower_map_main, lower_program,
    lower_program_auto, lower_program_main,
};
pub use pyo3::prelude::{PyObject, Python};

use compiler::merge_cli_blocks;
use linecache::{display_filename, register_linecache, strip_display_prefix};
use profiling::{log_profile, profile_enabled};
use pyo3::Bound;
use pyo3::exceptions::{PyRuntimeError, PySyntaxError, PySystemExit};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use snail_ast::{CompileMode, Stmt};
use snail_error::{ParseError, format_snail_error};
use snail_parser::preprocess;
use snail_parser::{parse, parse_awk, parse_awk_cli, parse_map};
use std::fmt::Debug;
use std::time::Instant;

type ProgramWithBlocks = (
    snail_ast::Program,
    Vec<Vec<snail_ast::Stmt>>,
    Vec<Vec<snail_ast::Stmt>>,
);
type ParseProgramFn = fn(&str) -> Result<ProgramWithBlocks, ParseError>;

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

fn parse_error_to_syntax(err: ParseError, filename: &str) -> PyErr {
    PySyntaxError::new_err(format_snail_error(&err.into(), filename))
}

#[allow(clippy::too_many_arguments)]
fn compile_source_to_code(
    py: Python<'_>,
    source: &str,
    mode: CompileMode,
    auto_print: bool,
    capture_last: bool,
    filename: &str,
    begin_code: &[String],
    end_code: &[String],
) -> PyResult<PyObject> {
    let profile = profile_enabled();
    let python_ast = compile_source(
        py,
        source,
        mode,
        auto_print,
        capture_last,
        filename,
        begin_code,
        end_code,
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
    Ok(code.unbind())
}

fn parse_program_ast(
    source: &str,
    filename: &str,
    begin_code: &[String],
    end_code: &[String],
    parse_program: ParseProgramFn,
) -> PyResult<ProgramWithBlocks> {
    let (program, begin_blocks, end_blocks) =
        parse_program(source).map_err(|err| parse_error_to_syntax(err, filename))?;
    merge_cli_blocks(begin_code, end_code, begin_blocks, end_blocks)
        .map_err(|err| parse_error_to_syntax(err, filename))
        .map(|(begin_blocks, end_blocks)| (program, begin_blocks, end_blocks))
}

fn format_program_ast<Ast: Debug>(
    program: snail_ast::Program,
    begin_blocks: Vec<Vec<snail_ast::Stmt>>,
    end_blocks: Vec<Vec<snail_ast::Stmt>>,
    wrap: impl FnOnce(snail_ast::Program, Vec<Vec<snail_ast::Stmt>>, Vec<Vec<snail_ast::Stmt>>) -> Ast,
) -> String {
    if begin_blocks.is_empty() && end_blocks.is_empty() {
        format!("{:#?}", program)
    } else {
        format!("{:#?}", wrap(program, begin_blocks, end_blocks))
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

#[allow(clippy::too_many_arguments)]
fn compile_source(
    py: Python<'_>,
    source: &str,
    mode: CompileMode,
    auto_print: bool,
    capture_last: bool,
    filename: &str,
    begin_code: &[String],
    end_code: &[String],
) -> Result<PyObject, PyErr> {
    let profile = profile_enabled();
    let total_start = Instant::now();
    let compile_start = Instant::now();

    let begin_refs: Vec<&str> = begin_code.iter().map(String::as_str).collect();
    let end_refs: Vec<&str> = end_code.iter().map(String::as_str).collect();
    let module = compiler::compile_source(
        py,
        source,
        mode,
        &begin_refs,
        &end_refs,
        auto_print,
        capture_last,
    )
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
    awk_field_separators: Option<String>,
    awk_include_whitespace: Option<bool>,
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
    let separators = awk_field_separators
        .as_deref()
        .filter(|value| !value.is_empty());
    let separators_value = match separators {
        Some(separators) => separators.into_py(py),
        None => py.None().into_py(py),
    };
    globals.set_item("__snail_awk_field_separators", separators_value)?;
    let include_whitespace = awk_include_whitespace.unwrap_or(separators.is_none());
    globals.set_item("__snail_awk_include_whitespace", include_whitespace)?;

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
    let code = compile_source_to_code(
        py,
        source,
        parse_mode(mode)?,
        auto_print,
        false,
        filename,
        &begin_code,
        &end_code,
    )?;
    if profile {
        log_profile("compile_py_total", total_start.elapsed());
    }
    Ok(code)
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
    compile_source(
        py,
        source,
        parse_mode(mode)?,
        auto_print,
        false,
        filename,
        &begin_code,
        &end_code,
    )
}

/// Check whether the main body of a program ends with a non-semicolon-terminated
/// expression (i.e. an expression that would be captured by AutoPrint/CaptureOnly).
fn has_tail_expression(source: &str, mode: CompileMode) -> bool {
    let parse_fn: fn(&str) -> Result<_, _> = match mode {
        CompileMode::Snail => parse,
        CompileMode::Map => parse_map,
        CompileMode::Awk => return false,
    };
    parse_fn(source)
        .map(|(program, _, _)| {
            matches!(
                program.stmts.last(),
                Some(Stmt::Expr {
                    semicolon_terminated: false,
                    ..
                })
            )
        })
        .unwrap_or(false)
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
    let parsed_mode = parse_mode(mode)?;

    // Pre-flight check: --test requires a trailing expression.
    // Reject before compiling/executing so no side effects occur.
    if test_last && !has_tail_expression(source, parsed_mode) {
        let sys = py.import_bound("sys")?;
        let stderr = sys.getattr("stderr")?;
        stderr.call_method1("write", ("snail: --test requires a trailing expression\n",))?;
        return Ok(2);
    }

    let capture_last = test_last && !auto_print;
    let code = compile_source_to_code(
        py,
        source,
        parsed_mode,
        auto_print,
        capture_last,
        filename,
        &begin_code,
        &end_code,
    )?;
    let builtins = py.import_bound("builtins")?;
    let globals_start = Instant::now();
    let globals = prepare_globals(
        py,
        strip_display_prefix(filename),
        &argv,
        auto_import,
        field_separators,
        include_whitespace,
    )?;
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

#[allow(dead_code)]
#[derive(Debug)]
struct MapAst {
    program: snail_ast::Program,
    begin_blocks: Vec<Vec<snail_ast::Stmt>>,
    end_blocks: Vec<Vec<snail_ast::Stmt>>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct SnailAst {
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
    match parse_mode(mode)? {
        CompileMode::Snail => {
            let (program, begin_blocks, end_blocks) =
                parse_program_ast(source, filename, &begin_code, &end_code, parse)?;
            Ok(format_program_ast(
                program,
                begin_blocks,
                end_blocks,
                |program, begin_blocks, end_blocks| SnailAst {
                    program,
                    begin_blocks,
                    end_blocks,
                },
            ))
        }
        CompileMode::Awk => {
            let begin_refs: Vec<&str> = begin_code.iter().map(String::as_str).collect();
            let end_refs: Vec<&str> = end_code.iter().map(String::as_str).collect();
            let program = parse_awk_cli(source, &begin_refs, &end_refs)
                .map_err(|err| parse_error_to_syntax(err, filename))?;
            Ok(format!("{:#?}", program))
        }
        CompileMode::Map => {
            let (program, begin_blocks, end_blocks) =
                parse_program_ast(source, filename, &begin_code, &end_code, parse_map)?;
            Ok(format_program_ast(
                program,
                begin_blocks,
                end_blocks,
                |program, begin_blocks, end_blocks| MapAst {
                    program,
                    begin_blocks,
                    end_blocks,
                },
            ))
        }
    }
}

#[pyfunction(name = "preprocess")]
fn preprocess_py(source: &str) -> PyResult<String> {
    preprocess::preprocess(source).map_err(|err| parse_error_to_syntax(err, "<snail>"))
}

#[pyfunction(name = "parse")]
#[pyo3(signature = (source, *, mode = "snail", filename = "<snail>"))]
fn parse_py(source: &str, mode: &str, filename: &str) -> PyResult<()> {
    match parse_mode(mode)? {
        CompileMode::Snail => parse(source)
            .map(|_| ())
            .map_err(|err| parse_error_to_syntax(err, filename)),
        CompileMode::Awk => parse_awk(source)
            .map(|_| ())
            .map_err(|err| parse_error_to_syntax(err, filename)),
        CompileMode::Map => parse_map(source)
            .map(|_| ())
            .map_err(|err| parse_error_to_syntax(err, filename)),
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
