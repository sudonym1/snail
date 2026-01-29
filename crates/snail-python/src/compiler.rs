use crate::lower::{
    lower_awk_program_with_auto_print, lower_map_program_with_begin_end,
    lower_program_with_auto_print,
};
use pyo3::prelude::*;
use snail_ast::{CompileMode, Stmt};
use snail_error::{ParseError, SnailError};
use snail_parser::{
    parse_awk_program, parse_awk_program_with_begin_end, parse_map_program_with_begin_end,
    parse_program,
};

type BlockList = Vec<Vec<Stmt>>;
type BeginEndBlocks = (BlockList, BlockList);

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
        CompileMode::Map => {
            let (program, begin_blocks, end_blocks) = parse_map_program_with_begin_end(source)?;
            let module = lower_map_program_with_begin_end(
                py,
                &program,
                &begin_blocks,
                &end_blocks,
                auto_print_last,
            )?;
            Ok(module)
        }
    }
}

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

pub fn compile_map_source_with_begin_end(
    py: Python<'_>,
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
    auto_print_last: bool,
) -> Result<PyObject, SnailError> {
    let (program, begin_blocks, end_blocks) = parse_map_program_with_begin_end(main_source)?;
    let begin_blocks = merge_cli_begin_blocks(begin_sources, begin_blocks)?;
    let end_blocks = merge_cli_end_blocks(end_sources, end_blocks)?;
    let module = lower_map_program_with_begin_end(
        py,
        &program,
        &begin_blocks,
        &end_blocks,
        auto_print_last,
    )?;
    Ok(module)
}

pub fn merge_map_cli_blocks(
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
    let mut cli_blocks = parse_cli_blocks(cli_sources)?;
    cli_blocks.extend(existing);
    Ok(cli_blocks)
}

fn merge_cli_end_blocks(
    cli_sources: &[&str],
    mut existing: BlockList,
) -> Result<BlockList, ParseError> {
    let cli_blocks = parse_cli_blocks(cli_sources)?;
    existing.extend(cli_blocks);
    Ok(existing)
}

fn parse_cli_blocks(sources: &[&str]) -> Result<BlockList, ParseError> {
    let mut blocks = Vec::new();
    for source in sources {
        let program = parse_program(source)?;
        if !program.stmts.is_empty() {
            blocks.push(program.stmts);
        }
    }
    Ok(blocks)
}
