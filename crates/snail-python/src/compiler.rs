use crate::lower::{lower_awk, lower_map, lower_program};
use pyo3::prelude::*;
use snail_ast::{CompileMode, Program, Stmt};
use snail_error::{LowerError, ParseError, SnailError};
use snail_parser::{parse, parse_awk_cli, parse_main, parse_map};

type BlockList = Vec<Vec<Stmt>>;
type BeginEndBlocks = (BlockList, BlockList);
type ParseFn = fn(&str) -> Result<(Program, BlockList, BlockList), ParseError>;
type LowerFn = fn(
    Python<'_>,
    &Program,
    &[Vec<Stmt>],
    &[Vec<Stmt>],
    bool,
    bool,
) -> Result<PyObject, LowerError>;

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
        CompileMode::Snail => compile_program_mode(
            py,
            main_source,
            begin_sources,
            end_sources,
            auto_print_last,
            capture_last,
            parse,
            lower_program,
        ),
        CompileMode::Awk => {
            let program = parse_awk_cli(main_source, begin_sources, end_sources)?;
            let module = lower_awk(py, &program, auto_print_last, capture_last)?;
            Ok(module)
        }
        CompileMode::Map => compile_program_mode(
            py,
            main_source,
            begin_sources,
            end_sources,
            auto_print_last,
            capture_last,
            parse_map,
            lower_map,
        ),
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

#[allow(clippy::too_many_arguments)]
fn compile_program_mode(
    py: Python<'_>,
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
    capture_last: bool,
    parse_program: ParseFn,
    lower_program: LowerFn,
) -> Result<PyObject, SnailError> {
    let (program, begin_blocks, end_blocks) = parse_program(main_source)?;
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
