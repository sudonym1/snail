use crate::lower::lower_program;
use pyo3::prelude::*;
use snail_ast::Program;
use snail_error::SnailError;
use snail_parser::parse;

pub(crate) fn compile_source(
    py: Python<'_>,
    main_source: &str,
    auto_print_last: bool,
    capture_last: bool,
) -> Result<PyObject, SnailError> {
    let program = parse(main_source)?;
    let module = lower_program(py, &program, auto_print_last, capture_last)?;
    Ok(module)
}

pub(crate) fn parse_program(source: &str) -> Result<Program, SnailError> {
    Ok(parse(source)?)
}
