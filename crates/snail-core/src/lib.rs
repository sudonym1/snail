// Re-export all workspace crates for unified API
pub use snail_ast::*;
pub use snail_codegen::*;
pub use snail_error::*;
pub use snail_lower::*;
pub use snail_parser::*;
pub use snail_python_ast::*;

/// Compilation API
pub fn compile_snail_source(source: &str, mode: CompileMode) -> Result<String, SnailError> {
    compile_snail_source_with_auto_print(source, mode, false)
}

pub fn compile_snail_source_with_auto_print(
    source: &str,
    mode: CompileMode,
    auto_print_last: bool,
) -> Result<String, SnailError> {
    match mode {
        CompileMode::Snail => {
            let program = parse_program(source)?;
            let module = lower_program(&program)?;
            Ok(python_source_with_auto_print(&module, auto_print_last))
        }
        CompileMode::Awk => {
            let program = parse_awk_program(source)?;
            // For awk mode, auto-print is handled at the block level during lowering
            let module = lower_awk_program_with_auto_print(&program, auto_print_last)?;
            // Use python_source without module-level auto-print since it's already in the AST
            Ok(python_source(&module))
        }
    }
}
