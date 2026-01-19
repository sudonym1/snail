use pyo3::prelude::*;

// Re-export all workspace crates for unified API
pub use snail_ast::*;
pub use snail_error::*;
pub use snail_lower::*;
pub use snail_parser::*;

/// Compilation API
pub fn compile_snail_source(
    py: Python<'_>,
    source: &str,
    mode: CompileMode,
) -> Result<PyObject, SnailError> {
    compile_snail_source_with_auto_print(py, source, mode, false)
}

pub fn compile_snail_source_with_auto_print(
    py: Python<'_>,
    source: &str,
    mode: CompileMode,
    auto_print_last: bool,
) -> Result<PyObject, SnailError> {
    match mode {
        CompileMode::Snail => {
            let program = parse_program(source)?;
            let module = lower_program_with_auto_print(py, &program, auto_print_last)?;
            Ok(module)
        }
        CompileMode::Awk => {
            let program = parse_awk_program(source)?;
            let module = lower_awk_program_with_auto_print(py, &program, auto_print_last)?;
            Ok(module)
        }
    }
}

/// Compile an awk program with separate begin and end code blocks.
/// Each begin/end source is parsed as a regular Snail program.
pub fn compile_awk_source_with_begin_end(
    py: Python<'_>,
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
) -> Result<PyObject, SnailError> {
    let program = parse_awk_program_with_begin_end(main_source, begin_sources, end_sources)?;
    let module = lower_awk_program_with_auto_print(py, &program, auto_print_last)?;
    Ok(module)
}
