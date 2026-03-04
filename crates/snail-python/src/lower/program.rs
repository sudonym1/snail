use pyo3::prelude::*;
use snail_ast::*;
use snail_error::LowerError;

use super::desugar::Desugarer;
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::{TailBehavior, lower_block_with_tail};
use super::validate::validate_yield_usage_program;

pub fn lower_program_main(py: Python<'_>, program: &Program) -> Result<PyObject, LowerError> {
    lower_program_auto(py, program, false)
}

pub fn lower_program_auto(
    py: Python<'_>,
    program: &Program,
    auto_print_last: bool,
) -> Result<PyObject, LowerError> {
    lower_program(py, program, auto_print_last, false)
}

pub fn lower_program(
    py: Python<'_>,
    program: &Program,
    auto_print_last: bool,
    capture_last: bool,
) -> Result<PyObject, LowerError> {
    let mut hoister = Desugarer::new();
    let program = hoister.desugar_program(program);
    validate_yield_usage_program(&program)?;
    let builder = AstBuilder::new(py).map_err(py_err_to_lower)?;
    let span = program.span.clone();

    let tail = if auto_print_last {
        TailBehavior::AutoPrint
    } else if capture_last {
        TailBehavior::CaptureOnly
    } else {
        TailBehavior::None
    };
    let body = lower_block_with_tail(&builder, &program.stmts, tail, &span)?;

    builder.module(body, &span).map_err(py_err_to_lower)
}
