use crate::lower::lower_program;
use pyo3::prelude::*;
use snail_ast::{CompileMode, Program, SourcePos, SourceSpan, Stmt};
use snail_error::SnailError;
use snail_parser::{parse_for_files, parse_lines_program, parse_main};

pub(crate) fn compile_source(
    py: Python<'_>,
    main_source: &str,
    mode: CompileMode,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
    capture_last: bool,
) -> Result<PyObject, SnailError> {
    let span = default_span();

    // Collect begin stmts from -b code
    let mut stmts = Vec::new();
    for source in begin_sources {
        let program = parse_main(source)?;
        stmts.extend(program.stmts);
    }

    // Parse and wrap main source according to mode
    match mode {
        CompileMode::Snail => {
            let program = parse_main(main_source)?;
            stmts.extend(program.stmts);
        }
        CompileMode::Awk => {
            let body = parse_lines_program(main_source)?;
            stmts.push(Stmt::Lines {
                source: None,
                body,
                span: span.clone(),
            });
        }
        CompileMode::Map => {
            let program = parse_for_files(main_source)?;
            stmts.push(Stmt::Files {
                source: None,
                body: program.stmts,
                span: span.clone(),
            });
        }
    }

    // Collect end stmts from -e code
    for source in end_sources {
        let program = parse_main(source)?;
        stmts.extend(program.stmts);
    }

    let program = Program {
        stmts,
        span: span.clone(),
    };

    // Awk mode never auto-prints
    let (auto_print, capture) = match mode {
        CompileMode::Awk => (false, false),
        _ => (auto_print_last, capture_last),
    };

    let module = lower_program(py, &program, auto_print, capture)?;
    Ok(module)
}

fn default_span() -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line: 1,
            column: 1,
        },
        end: SourcePos {
            offset: 0,
            line: 1,
            column: 1,
        },
    }
}
