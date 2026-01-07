use pest::Parser;
use pest_derive::Parser;

use snail_ast::*;
use snail_error::ParseError;

mod awk;
mod expr;
mod literal;
mod stmt;
mod string;
mod util;

use awk::parse_awk_rule;
use stmt::{parse_block, parse_stmt_list};
use util::{full_span, parse_error_from_pest};

#[derive(Parser)]
#[grammar = "snail.pest"]
pub struct SnailParser;

pub fn parse_program(source: &str) -> Result<Program, ParseError> {
    let mut pairs = SnailParser::parse(Rule::program, source)
        .map_err(|err| parse_error_from_pest(err, source))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing program root"))?;
    let span = full_span(source);
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            stmts = parse_stmt_list(inner, source)?;
        }
    }
    Ok(Program { stmts, span })
}

pub fn parse_awk_program(source: &str) -> Result<AwkProgram, ParseError> {
    let mut pairs = SnailParser::parse(Rule::awk_program, source)
        .map_err(|err| parse_error_from_pest(err, source))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing awk program root"))?;
    let span = full_span(source);

    let mut begin_blocks = Vec::new();
    let mut rules = Vec::new();
    let mut end_blocks = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::awk_entry_list {
            for entry in inner.into_inner() {
                match entry.as_rule() {
                    Rule::awk_begin => {
                        let block = entry
                            .into_inner()
                            .find(|pair| pair.as_rule() == Rule::block)
                            .ok_or_else(|| {
                                util::error_with_span("missing BEGIN block", span.clone(), source)
                            })?;
                        begin_blocks.push(parse_block(block, source)?);
                    }
                    Rule::awk_end => {
                        let block = entry
                            .into_inner()
                            .find(|pair| pair.as_rule() == Rule::block)
                            .ok_or_else(|| {
                                util::error_with_span("missing END block", span.clone(), source)
                            })?;
                        end_blocks.push(parse_block(block, source)?);
                    }
                    Rule::awk_rule => rules.push(parse_awk_rule(entry, source)?),
                    _ => {}
                }
            }
        }
    }

    Ok(AwkProgram {
        begin_blocks,
        rules,
        end_blocks,
        span,
    })
}
