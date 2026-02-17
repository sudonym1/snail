use crate::lower::lower_program;
use pyo3::prelude::*;
use snail_ast::{AwkProgram, CompileMode, Program, Stmt};
use snail_error::{ParseError, SnailError};
use snail_parser::{parse, parse_awk_cli, parse_main, parse_map};

type BlockList = Vec<Vec<Stmt>>;
type BeginEndBlocks = (BlockList, BlockList);

pub(crate) fn compile_source(
    py: Python<'_>,
    main_source: &str,
    mode: CompileMode,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
    capture_last: bool,
) -> Result<PyObject, SnailError> {
    match mode {
        CompileMode::Snail => {
            let (program, begin_blocks, end_blocks) = parse(main_source)?;
            let begin_blocks = merge_cli_begin_blocks(begin_sources, begin_blocks)?;
            let end_blocks = merge_cli_end_blocks(end_sources, end_blocks)?;
            let module = lower_program(
                py,
                &program,
                &begin_blocks,
                &end_blocks,
                auto_print_last,
                capture_last,
            )?;
            Ok(module)
        }
        CompileMode::Awk => {
            let awk_program = parse_awk_cli(main_source, begin_sources, end_sources)?;
            let (program, begin_blocks, end_blocks) = desugar_awk_to_program(&awk_program);
            let module = lower_program(py, &program, &begin_blocks, &end_blocks, false, false)?;
            Ok(module)
        }
        CompileMode::Map => {
            let (program, begin_blocks, end_blocks) = parse_map(main_source)?;
            let begin_blocks = merge_cli_begin_blocks(begin_sources, begin_blocks)?;
            let end_blocks = merge_cli_end_blocks(end_sources, end_blocks)?;
            let program = desugar_map_to_program(&program);
            let module = lower_program(
                py,
                &program,
                &begin_blocks,
                &end_blocks,
                auto_print_last,
                capture_last,
            )?;
            Ok(module)
        }
    }
}

/// Desugar an `AwkProgram` into a regular `Program` wrapping the rules in `lines { }`.
///
/// `--awk` source becomes: `BEGIN { ... }` + `lines { pattern/action... }` + `END { ... }`
pub(crate) fn desugar_awk_to_program(program: &AwkProgram) -> (Program, BlockList, BlockList) {
    let span = program.span.clone();

    // Convert AwkRules to PatternAction statements
    let body: Vec<Stmt> = program
        .rules
        .iter()
        .map(|rule| Stmt::PatternAction {
            pattern: rule.pattern.clone(),
            action: rule.action.clone(),
            span: rule.span.clone(),
        })
        .collect();

    // Wrap in lines { }
    let lines_stmt = Stmt::Lines {
        source: None,
        body,
        span: span.clone(),
    };

    let desugared = Program {
        stmts: vec![lines_stmt],
        span,
    };

    (
        desugared,
        program.begin_blocks.clone(),
        program.end_blocks.clone(),
    )
}

/// Desugar a map-mode `Program` by wrapping its body in `files { }`.
///
/// `--map` source becomes: `files { original_body }`.
pub(crate) fn desugar_map_to_program(program: &Program) -> Program {
    let span = program.span.clone();

    let files_stmt = Stmt::Files {
        source: None,
        body: program.stmts.clone(),
        span: span.clone(),
    };

    Program {
        stmts: vec![files_stmt],
        span,
    }
}

pub(crate) fn merge_cli_blocks(
    begin_sources: &[String],
    end_sources: &[String],
    begin_blocks: BlockList,
    end_blocks: BlockList,
) -> Result<BeginEndBlocks, ParseError> {
    let begin_refs: Vec<&str> = begin_sources.iter().map(String::as_str).collect();
    let end_refs: Vec<&str> = end_sources.iter().map(String::as_str).collect();
    let begin_blocks = merge_cli_begin_blocks(&begin_refs, begin_blocks)?;
    let end_blocks = merge_cli_end_blocks(&end_refs, end_blocks)?;
    Ok((begin_blocks, end_blocks))
}

fn merge_cli_begin_blocks(
    cli_sources: &[&str],
    existing: BlockList,
) -> Result<BlockList, ParseError> {
    merge_cli_blocks_with_position(cli_sources, existing, true)
}

fn merge_cli_end_blocks(
    cli_sources: &[&str],
    existing: BlockList,
) -> Result<BlockList, ParseError> {
    merge_cli_blocks_with_position(cli_sources, existing, false)
}

fn merge_cli_blocks_with_position(
    cli_sources: &[&str],
    mut existing: BlockList,
    prepend: bool,
) -> Result<BlockList, ParseError> {
    let mut cli_blocks = parse_cli_blocks(cli_sources)?;
    if prepend {
        cli_blocks.extend(existing);
        Ok(cli_blocks)
    } else {
        existing.extend(cli_blocks);
        Ok(existing)
    }
}

fn parse_cli_blocks(sources: &[&str]) -> Result<BlockList, ParseError> {
    let mut blocks = Vec::new();
    for source in sources {
        let program = parse_main(source)?;
        if !program.stmts.is_empty() {
            blocks.push(program.stmts);
        }
    }
    Ok(blocks)
}
