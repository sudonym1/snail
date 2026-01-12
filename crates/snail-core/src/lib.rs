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
