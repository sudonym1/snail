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
use stmt::{parse_block, parse_stmt};
use util::{error_with_span, full_span, parse_error_from_pest, span_from_offset, span_from_pair};

#[derive(Parser)]
#[grammar = "snail.pest"]
pub struct SnailParser;

pub type MapProgramWithBeginEnd = (Program, Vec<Vec<Stmt>>, Vec<Vec<Stmt>>);
pub type ProgramWithBeginEnd = (Program, Vec<Vec<Stmt>>, Vec<Vec<Stmt>>);
type ValidateProgramFn = fn(&Program, &str) -> Result<(), ParseError>;

pub fn parse_main(source: &str) -> Result<Program, ParseError> {
    let (program, _, _) = parse(source)?;
    Ok(program)
}

/// Parses a regular Snail program with in-file BEGIN/END blocks.
/// BEGIN/END blocks are parsed as regular Snail statement blocks (no map/awk vars).
pub fn parse(source: &str) -> Result<ProgramWithBeginEnd, ParseError> {
    parse_program_entries(
        source,
        Rule::program,
        Rule::program_entry_list,
        Rule::program_entry,
        Rule::program_begin,
        Rule::program_end,
        "missing program root",
        "missing program entry",
        validate_no_awk_syntax,
    )
}

pub fn parse_awk(source: &str) -> Result<AwkProgram, ParseError> {
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
                        let block = parse_begin_end_block(entry, source, "BEGIN")?;
                        if !block.is_empty() {
                            begin_blocks.push(block);
                        }
                    }
                    Rule::awk_end => {
                        let block = parse_begin_end_block(entry, source, "END")?;
                        if !block.is_empty() {
                            end_blocks.push(block);
                        }
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

/// Parses an awk program with separate CLI begin and end code sources.
/// Each begin/end source is parsed as a regular Snail program and merged so CLI BEGIN
/// blocks run before in-file BEGIN blocks, and CLI END blocks run after in-file END blocks.
pub fn parse_awk_cli(
    main_source: &str,
    begin_sources: &[&str],
    end_sources: &[&str],
) -> Result<AwkProgram, ParseError> {
    let mut program = parse_awk(main_source)?;

    let mut cli_begin_blocks = Vec::new();
    for source in begin_sources {
        let begin_program = parse_main(source)?;
        if !begin_program.stmts.is_empty() {
            cli_begin_blocks.push(begin_program.stmts);
        }
    }
    cli_begin_blocks.extend(program.begin_blocks);
    program.begin_blocks = cli_begin_blocks;

    let mut end_blocks = program.end_blocks;
    for source in end_sources {
        let end_program = parse_main(source)?;
        if !end_program.stmts.is_empty() {
            end_blocks.push(end_program.stmts);
        }
    }
    program.end_blocks = end_blocks;

    Ok(program)
}

const AWK_ONLY_NAMES: [&str; 4] = ["$n", "$fn", "$m", "$f"];
const AWK_ONLY_MESSAGE: &str = "awk variables are only valid in awk mode; use --awk";

const MAP_ONLY_NAMES: [&str; 2] = ["$fd", "$text"];
const MAP_ONLY_MESSAGE: &str = "map variables are only valid in map mode; use --map";
const MAP_OR_AWK_NAMES: [&str; 1] = ["$src"];
const MAP_OR_AWK_MESSAGE: &str =
    "map/awk variables are only valid in map or awk mode; use --map or --awk";

/// Parses a map program that processes files one at a time.
/// Allows map variables ($src, $fd, $text) but rejects awk variables.
/// In-file BEGIN/END blocks are validated but not returned; use
/// `parse_map` to access them.
pub fn parse_map_main(source: &str) -> Result<Program, ParseError> {
    let (program, _, _) = parse_map(source)?;
    Ok(program)
}

fn validate_no_awk_syntax_for_map(program: &Program, source: &str) -> Result<(), ParseError> {
    for stmt in &program.stmts {
        validate_stmt_for_map(stmt, source)?;
    }
    Ok(())
}

/// Parses a map program with in-file BEGIN/END blocks.
/// BEGIN/END blocks are parsed as regular Snail statement blocks (no map/awk vars).
pub fn parse_map(source: &str) -> Result<MapProgramWithBeginEnd, ParseError> {
    parse_program_entries(
        source,
        Rule::map_program,
        Rule::map_entry_list,
        Rule::map_entry,
        Rule::map_begin,
        Rule::map_end,
        "missing map program root",
        "missing map entry",
        validate_no_awk_syntax_for_map,
    )
}

#[allow(clippy::too_many_arguments)]
fn parse_program_entries(
    source: &str,
    root_rule: Rule,
    entry_list_rule: Rule,
    entry_rule: Rule,
    begin_rule: Rule,
    end_rule: Rule,
    missing_root: &str,
    missing_entry: &str,
    validate_program: ValidateProgramFn,
) -> Result<ProgramWithBeginEnd, ParseError> {
    let mut pairs =
        SnailParser::parse(root_rule, source).map_err(|err| parse_error_from_pest(err, source))?;
    let pair = pairs.next().ok_or_else(|| ParseError::new(missing_root))?;
    let span = full_span(source);
    let mut stmts = Vec::new();
    let mut begin_blocks = Vec::new();
    let mut end_blocks = Vec::new();
    let mut entries = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() != entry_list_rule {
            continue;
        }
        for entry in inner.into_inner() {
            if entry.as_rule() != entry_rule {
                continue;
            }
            let entry_span = span_from_pair(&entry, source);
            let mut entry_inner = entry.into_inner();
            let entry_pair = entry_inner
                .next()
                .ok_or_else(|| error_with_span(missing_entry, entry_span.clone(), source))?;
            let parsed_rule = entry_pair.as_rule();
            if parsed_rule == begin_rule {
                let block = parse_begin_end_block(entry_pair, source, "BEGIN")?;
                if !block.is_empty() {
                    begin_blocks.push(block);
                }
                entries.push((entry_span, EntryKind::BeginEnd));
            } else if parsed_rule == end_rule {
                let block = parse_begin_end_block(entry_pair, source, "END")?;
                if !block.is_empty() {
                    end_blocks.push(block);
                }
                entries.push((entry_span, EntryKind::BeginEnd));
            } else {
                let stmt = parse_stmt(entry_pair, source)?;
                entries.push((entry_span, entry_kind_for_stmt(&stmt)));
                stmts.push(stmt);
            }
        }
    }

    validate_entry_separators(&entries, source)?;

    let program = Program { stmts, span };
    validate_program(&program, source)?;
    Ok((program, begin_blocks, end_blocks))
}

fn parse_begin_end_block(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    label: &str,
) -> Result<Vec<Stmt>, ParseError> {
    let span = span_from_pair(&pair, source);
    let mut inner = pair.into_inner();
    let block_pair = inner
        .next()
        .ok_or_else(|| error_with_span(format!("missing {label} block"), span.clone(), source))?;
    let block = parse_block(block_pair, source)?;
    validate_block(&block, source)?;
    Ok(block)
}

#[derive(Clone, Copy)]
enum EntryKind {
    BeginEnd,
    Simple,
    Compound,
}

fn entry_kind_for_stmt(stmt: &Stmt) -> EntryKind {
    match stmt {
        Stmt::If { .. }
        | Stmt::While { .. }
        | Stmt::For { .. }
        | Stmt::Def { .. }
        | Stmt::Class { .. }
        | Stmt::Try { .. }
        | Stmt::With { .. } => EntryKind::Compound,
        _ => EntryKind::Simple,
    }
}

fn validate_entry_separators(
    entries: &[(SourceSpan, EntryKind)],
    source: &str,
) -> Result<(), ParseError> {
    for window in entries.windows(2) {
        let (prev_span, prev_kind) = &window[0];
        let (next_span, next_kind) = &window[1];
        let gap = &source[prev_span.end.offset..next_span.start.offset];
        let has_sep = gap.contains('\n') || gap.contains(';');
        let can_omit_sep = matches!(next_kind, EntryKind::BeginEnd);
        if !has_sep && matches!(prev_kind, EntryKind::Simple) && !can_omit_sep {
            return Err(error_with_span(
                "expected statement separator",
                span_from_offset(next_span.start.offset, next_span.start.offset, source),
                source,
            ));
        }
    }
    Ok(())
}

fn validate_stmt_for_map(stmt: &Stmt, source: &str) -> Result<(), ParseError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            validate_condition_for_map(cond, source)?;
            validate_block_for_map(body, source)?;
            for (elif_cond, elif_body) in elifs {
                validate_condition_for_map(elif_cond, source)?;
                validate_block_for_map(elif_body, source)?;
            }
            if let Some(body) = else_body {
                validate_block_for_map(body, source)?;
            }
        }
        Stmt::While {
            cond,
            body,
            else_body,
            ..
        } => {
            validate_condition_for_map(cond, source)?;
            validate_block_for_map(body, source)?;
            if let Some(body) = else_body {
                validate_block_for_map(body, source)?;
            }
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            validate_assign_target_for_map(target, source)?;
            validate_expr_for_map(iter, source)?;
            validate_block_for_map(body, source)?;
            if let Some(body) = else_body {
                validate_block_for_map(body, source)?;
            }
        }
        Stmt::Def { params, body, .. } => {
            for param in params {
                validate_param_for_map(param, source)?;
            }
            validate_block_for_map(body, source)?;
        }
        Stmt::Class { body, .. } => {
            validate_block_for_map(body, source)?;
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            validate_block_for_map(body, source)?;
            for handler in handlers {
                validate_except_handler_for_map(handler, source)?;
            }
            if let Some(body) = else_body {
                validate_block_for_map(body, source)?;
            }
            if let Some(body) = finally_body {
                validate_block_for_map(body, source)?;
            }
        }
        Stmt::With { items, body, .. } => {
            for item in items {
                validate_with_item_for_map(item, source)?;
            }
            validate_block_for_map(body, source)?;
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                validate_expr_for_map(value, source)?;
            }
        }
        Stmt::Raise { value, from, .. } => {
            if let Some(value) = value {
                validate_expr_for_map(value, source)?;
            }
            if let Some(from) = from {
                validate_expr_for_map(from, source)?;
            }
        }
        Stmt::Assert { test, message, .. } => {
            validate_expr_for_map(test, source)?;
            if let Some(message) = message {
                validate_expr_for_map(message, source)?;
            }
        }
        Stmt::Delete { targets, .. } => {
            for target in targets {
                validate_assign_target_for_map(target, source)?;
            }
        }
        Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Pass { .. } => {}
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                validate_assign_target_for_map(target, source)?;
            }
            validate_expr_for_map(value, source)?;
        }
        Stmt::Expr { value, .. } => {
            validate_expr_for_map(value, source)?;
        }
    }
    Ok(())
}

fn validate_block_for_map(block: &[Stmt], source: &str) -> Result<(), ParseError> {
    for stmt in block {
        validate_stmt_for_map(stmt, source)?;
    }
    Ok(())
}

fn validate_with_item_for_map(item: &WithItem, source: &str) -> Result<(), ParseError> {
    validate_expr_for_map(&item.context, source)?;
    if let Some(target) = &item.target {
        validate_assign_target_for_map(target, source)?;
    }
    Ok(())
}

fn validate_except_handler_for_map(
    handler: &ExceptHandler,
    source: &str,
) -> Result<(), ParseError> {
    if let Some(expr) = &handler.type_name {
        validate_expr_for_map(expr, source)?;
    }
    validate_block_for_map(&handler.body, source)?;
    Ok(())
}

fn validate_param_for_map(param: &Parameter, source: &str) -> Result<(), ParseError> {
    match param {
        Parameter::Regular { default, .. } => {
            if let Some(default) = default {
                validate_expr_for_map(default, source)?;
            }
        }
        Parameter::VarArgs { .. } | Parameter::KwArgs { .. } => {}
    }
    Ok(())
}

fn validate_argument_for_map(arg: &Argument, source: &str) -> Result<(), ParseError> {
    match arg {
        Argument::Positional { value, .. }
        | Argument::Keyword { value, .. }
        | Argument::Star { value, .. }
        | Argument::KwStar { value, .. } => validate_expr_for_map(value, source),
    }
}

fn validate_assign_target_for_map(target: &AssignTarget, source: &str) -> Result<(), ParseError> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Attribute { value, .. } => validate_expr_for_map(value, source),
        AssignTarget::Index { value, index, .. } => {
            validate_expr_for_map(value, source)?;
            validate_expr_for_map(index, source)
        }
        AssignTarget::Starred { target, .. } => validate_assign_target_for_map(target, source),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                validate_assign_target_for_map(element, source)?;
            }
            Ok(())
        }
    }
}

fn validate_condition_for_map(cond: &Condition, source: &str) -> Result<(), ParseError> {
    match cond {
        Condition::Expr(expr) => validate_expr_for_map(expr, source),
        Condition::Let {
            target,
            value,
            guard,
            ..
        } => {
            validate_assign_target_for_map(target, source)?;
            validate_expr_for_map(value, source)?;
            if let Some(guard) = guard {
                validate_expr_for_map(guard, source)?;
            }
            Ok(())
        }
    }
}

fn validate_expr_for_map(expr: &Expr, source: &str) -> Result<(), ParseError> {
    match expr {
        Expr::Name { name, span } => {
            // Reject awk-only names but allow map names
            if AWK_ONLY_NAMES.contains(&name.as_str()) {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
            }
            // Map variables are allowed in map mode
        }
        Expr::Placeholder { .. } => {}
        Expr::FieldIndex { span, .. } => {
            return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
        }
        Expr::FString { parts, .. } => {
            for part in parts {
                validate_fstring_part_for_map(part, source)?;
            }
        }
        Expr::Unary { expr, .. } => {
            validate_expr_for_map(expr, source)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_expr_for_map(left, source)?;
            validate_expr_for_map(right, source)?;
        }
        Expr::AugAssign { target, value, .. } => {
            validate_assign_target_for_map(target, source)?;
            validate_expr_for_map(value, source)?;
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            validate_assign_target_for_map(target, source)?;
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            validate_expr_for_map(left, source)?;
            for expr in comparators {
                validate_expr_for_map(expr, source)?;
            }
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            validate_expr_for_map(test, source)?;
            validate_expr_for_map(body, source)?;
            validate_expr_for_map(orelse, source)?;
        }
        Expr::TryExpr { expr, fallback, .. } => {
            validate_expr_for_map(expr, source)?;
            if let Some(fallback) = fallback {
                validate_expr_for_map(fallback, source)?;
            }
        }
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                validate_expr_for_map(value, source)?;
            }
        }
        Expr::YieldFrom { expr, .. } => {
            validate_expr_for_map(expr, source)?;
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                validate_param_for_map(param, source)?;
            }
            validate_block_for_map(body, source)?;
        }
        Expr::Compound { expressions, .. } => {
            for expr in expressions {
                validate_expr_for_map(expr, source)?;
            }
        }
        Expr::Regex { pattern, .. } => {
            validate_regex_pattern_for_map(pattern, source)?;
        }
        Expr::RegexMatch { value, pattern, .. } => {
            validate_expr_for_map(value, source)?;
            validate_regex_pattern_for_map(pattern, source)?;
        }
        Expr::Subprocess { parts, .. } => {
            for part in parts {
                validate_fstring_part_for_map(part, source)?;
            }
        }
        Expr::StructuredAccessor { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. } => {}
        Expr::Call { func, args, .. } => {
            validate_expr_for_map(func, source)?;
            for arg in args {
                validate_argument_for_map(arg, source)?;
            }
        }
        Expr::Attribute { value, .. } => {
            validate_expr_for_map(value, source)?;
        }
        Expr::Index { value, index, .. } => {
            validate_expr_for_map(value, source)?;
            validate_expr_for_map(index, source)?;
        }
        Expr::Paren { expr, .. } => {
            validate_expr_for_map(expr, source)?;
        }
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                validate_expr_for_map(expr, source)?;
            }
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                validate_expr_for_map(key, source)?;
                validate_expr_for_map(value, source)?;
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_for_map(start, source)?;
            }
            if let Some(end) = end {
                validate_expr_for_map(end, source)?;
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            validate_expr_for_map(element, source)?;
            validate_expr_for_map(iter, source)?;
            for expr in ifs {
                validate_expr_for_map(expr, source)?;
            }
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            validate_expr_for_map(key, source)?;
            validate_expr_for_map(value, source)?;
            validate_expr_for_map(iter, source)?;
            for expr in ifs {
                validate_expr_for_map(expr, source)?;
            }
        }
    }
    Ok(())
}

fn validate_regex_pattern_for_map(pattern: &RegexPattern, source: &str) -> Result<(), ParseError> {
    if let RegexPattern::Interpolated(parts) = pattern {
        for part in parts {
            validate_fstring_part_for_map(part, source)?;
        }
    }
    Ok(())
}

fn validate_fstring_part_for_map(part: &FStringPart, source: &str) -> Result<(), ParseError> {
    if let FStringPart::Expr(expr) = part {
        validate_expr_for_map(&expr.expr, source)?;
        if let Some(spec) = &expr.format_spec {
            for spec_part in spec {
                validate_fstring_part_for_map(spec_part, source)?;
            }
        }
    }
    Ok(())
}

fn validate_no_awk_syntax(program: &Program, source: &str) -> Result<(), ParseError> {
    for stmt in &program.stmts {
        validate_stmt(stmt, source)?;
    }
    Ok(())
}

fn validate_stmt(stmt: &Stmt, source: &str) -> Result<(), ParseError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            validate_condition(cond, source)?;
            validate_block(body, source)?;
            for (elif_cond, elif_body) in elifs {
                validate_condition(elif_cond, source)?;
                validate_block(elif_body, source)?;
            }
            if let Some(body) = else_body {
                validate_block(body, source)?;
            }
        }
        Stmt::While {
            cond,
            body,
            else_body,
            ..
        } => {
            validate_condition(cond, source)?;
            validate_block(body, source)?;
            if let Some(body) = else_body {
                validate_block(body, source)?;
            }
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            validate_assign_target(target, source)?;
            validate_expr(iter, source)?;
            validate_block(body, source)?;
            if let Some(body) = else_body {
                validate_block(body, source)?;
            }
        }
        Stmt::Def { params, body, .. } => {
            for param in params {
                validate_param(param, source)?;
            }
            validate_block(body, source)?;
        }
        Stmt::Class { body, .. } => {
            validate_block(body, source)?;
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            validate_block(body, source)?;
            for handler in handlers {
                validate_except_handler(handler, source)?;
            }
            if let Some(body) = else_body {
                validate_block(body, source)?;
            }
            if let Some(body) = finally_body {
                validate_block(body, source)?;
            }
        }
        Stmt::With { items, body, .. } => {
            for item in items {
                validate_with_item(item, source)?;
            }
            validate_block(body, source)?;
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                validate_expr(value, source)?;
            }
        }
        Stmt::Raise { value, from, .. } => {
            if let Some(value) = value {
                validate_expr(value, source)?;
            }
            if let Some(from) = from {
                validate_expr(from, source)?;
            }
        }
        Stmt::Assert { test, message, .. } => {
            validate_expr(test, source)?;
            if let Some(message) = message {
                validate_expr(message, source)?;
            }
        }
        Stmt::Delete { targets, .. } => {
            for target in targets {
                validate_assign_target(target, source)?;
            }
        }
        Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Pass { .. } => {}
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                validate_assign_target(target, source)?;
            }
            validate_expr(value, source)?;
        }
        Stmt::Expr { value, .. } => {
            validate_expr(value, source)?;
        }
    }
    Ok(())
}

fn validate_block(block: &[Stmt], source: &str) -> Result<(), ParseError> {
    for stmt in block {
        validate_stmt(stmt, source)?;
    }
    Ok(())
}

fn validate_with_item(item: &WithItem, source: &str) -> Result<(), ParseError> {
    validate_expr(&item.context, source)?;
    if let Some(target) = &item.target {
        validate_assign_target(target, source)?;
    }
    Ok(())
}

fn validate_except_handler(handler: &ExceptHandler, source: &str) -> Result<(), ParseError> {
    if let Some(expr) = &handler.type_name {
        validate_expr(expr, source)?;
    }
    validate_block(&handler.body, source)?;
    Ok(())
}

fn validate_param(param: &Parameter, source: &str) -> Result<(), ParseError> {
    match param {
        Parameter::Regular { default, .. } => {
            if let Some(default) = default {
                validate_expr(default, source)?;
            }
        }
        Parameter::VarArgs { .. } | Parameter::KwArgs { .. } => {}
    }
    Ok(())
}

fn validate_argument(arg: &Argument, source: &str) -> Result<(), ParseError> {
    match arg {
        Argument::Positional { value, .. }
        | Argument::Keyword { value, .. }
        | Argument::Star { value, .. }
        | Argument::KwStar { value, .. } => validate_expr(value, source),
    }
}

fn validate_assign_target(target: &AssignTarget, source: &str) -> Result<(), ParseError> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Attribute { value, .. } => validate_expr(value, source),
        AssignTarget::Index { value, index, .. } => {
            validate_expr(value, source)?;
            validate_expr(index, source)
        }
        AssignTarget::Starred { target, .. } => validate_assign_target(target, source),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                validate_assign_target(element, source)?;
            }
            Ok(())
        }
    }
}

fn validate_condition(cond: &Condition, source: &str) -> Result<(), ParseError> {
    match cond {
        Condition::Expr(expr) => validate_expr(expr, source),
        Condition::Let {
            target,
            value,
            guard,
            ..
        } => {
            validate_assign_target(target, source)?;
            validate_expr(value, source)?;
            if let Some(guard) = guard {
                validate_expr(guard, source)?;
            }
            Ok(())
        }
    }
}

fn validate_expr(expr: &Expr, source: &str) -> Result<(), ParseError> {
    match expr {
        Expr::Name { name, span } => {
            if AWK_ONLY_NAMES.contains(&name.as_str()) {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
            }
            if MAP_ONLY_NAMES.contains(&name.as_str()) {
                return Err(error_with_span(MAP_ONLY_MESSAGE, span.clone(), source));
            }
            if MAP_OR_AWK_NAMES.contains(&name.as_str()) {
                return Err(error_with_span(MAP_OR_AWK_MESSAGE, span.clone(), source));
            }
        }
        Expr::Placeholder { .. } => {}
        Expr::FieldIndex { span, .. } => {
            return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
        }
        Expr::FString { parts, .. } => {
            for part in parts {
                validate_fstring_part(part, source)?;
            }
        }
        Expr::Unary { expr, .. } => {
            validate_expr(expr, source)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_expr(left, source)?;
            validate_expr(right, source)?;
        }
        Expr::AugAssign { target, value, .. } => {
            validate_assign_target(target, source)?;
            validate_expr(value, source)?;
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            validate_assign_target(target, source)?;
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            validate_expr(left, source)?;
            for expr in comparators {
                validate_expr(expr, source)?;
            }
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            validate_expr(test, source)?;
            validate_expr(body, source)?;
            validate_expr(orelse, source)?;
        }
        Expr::TryExpr { expr, fallback, .. } => {
            validate_expr(expr, source)?;
            if let Some(fallback) = fallback {
                validate_expr(fallback, source)?;
            }
        }
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                validate_expr(value, source)?;
            }
        }
        Expr::YieldFrom { expr, .. } => {
            validate_expr(expr, source)?;
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                validate_param(param, source)?;
            }
            validate_block(body, source)?;
        }
        Expr::Compound { expressions, .. } => {
            for expr in expressions {
                validate_expr(expr, source)?;
            }
        }
        Expr::Regex { pattern, .. } => {
            validate_regex_pattern(pattern, source)?;
        }
        Expr::RegexMatch { value, pattern, .. } => {
            validate_expr(value, source)?;
            validate_regex_pattern(pattern, source)?;
        }
        Expr::Subprocess { parts, .. } => {
            for part in parts {
                validate_fstring_part(part, source)?;
            }
        }
        Expr::StructuredAccessor { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. } => {}
        Expr::Call { func, args, .. } => {
            validate_expr(func, source)?;
            for arg in args {
                validate_argument(arg, source)?;
            }
        }
        Expr::Attribute { value, .. } => {
            validate_expr(value, source)?;
        }
        Expr::Index { value, index, .. } => {
            validate_expr(value, source)?;
            validate_expr(index, source)?;
        }
        Expr::Paren { expr, .. } => {
            validate_expr(expr, source)?;
        }
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                validate_expr(expr, source)?;
            }
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                validate_expr(key, source)?;
                validate_expr(value, source)?;
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                validate_expr(start, source)?;
            }
            if let Some(end) = end {
                validate_expr(end, source)?;
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            validate_expr(element, source)?;
            validate_expr(iter, source)?;
            for expr in ifs {
                validate_expr(expr, source)?;
            }
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            validate_expr(key, source)?;
            validate_expr(value, source)?;
            validate_expr(iter, source)?;
            for expr in ifs {
                validate_expr(expr, source)?;
            }
        }
    }
    Ok(())
}

fn validate_regex_pattern(pattern: &RegexPattern, source: &str) -> Result<(), ParseError> {
    if let RegexPattern::Interpolated(parts) = pattern {
        for part in parts {
            validate_fstring_part(part, source)?;
        }
    }
    Ok(())
}

fn validate_fstring_part(part: &FStringPart, source: &str) -> Result<(), ParseError> {
    if let FStringPart::Expr(expr) = part {
        validate_expr(&expr.expr, source)?;
        if let Some(spec) = &expr.format_spec {
            for spec_part in spec {
                validate_fstring_part(spec_part, source)?;
            }
        }
    }
    Ok(())
}
