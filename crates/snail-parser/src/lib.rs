use pest::Parser;
use pest_derive::Parser;

use snail_ast::{DictEntry, *};
use snail_error::ParseError;

mod expr;
mod literal;
pub mod preprocess;
mod stmt;
mod string;
mod util;

use stmt::parse_stmt_list;
use util::{LineIndex, error_with_span, full_span, parse_error_from_pest};

#[derive(Parser)]
#[grammar = "snail.pest"]
pub struct SnailParser;

/// Parses a regular Snail program.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let preprocessed = preprocess::preprocess(source)?;
    let lx = LineIndex::new(source);
    let mut pairs = SnailParser::parse(Rule::program, &preprocessed)
        .map_err(|err| parse_error_from_pest(err, &lx))?;
    let pair = pairs
        .next()
        .ok_or_else(|| ParseError::new("missing program root"))?;
    let span = full_span(&lx);
    let mut stmts = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::stmt_list {
            stmts = parse_stmt_list(inner, &lx)?;
        }
    }

    let program = Program { stmts, span };
    validate_program(&program, &lx, ValidationMode::Main)?;
    Ok(program)
}

const AWK_ONLY_NAMES: [&str; 4] = ["$n", "$fn", "$m", "$f"];
const AWK_ONLY_MESSAGE: &str = "awk variables are only valid inside awk { } blocks";

const XARGS_ONLY_NAMES: [&str; 2] = ["$fd", "$text"];
const XARGS_ONLY_MESSAGE: &str = "xargs variables are only valid inside xargs { } blocks";
const XARGS_OR_AWK_NAMES: [&str; 1] = ["$src"];
const XARGS_OR_AWK_MESSAGE: &str = "$src is only valid inside awk { } or xargs { } blocks";

#[derive(Clone, Copy, Eq, PartialEq)]
enum ValidationMode {
    Main,
    Awk,
    Xargs,
}

fn validate_program(
    program: &Program,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for stmt in &program.stmts {
        validate_stmt_mode(stmt, lx, mode)?;
    }
    Ok(())
}

fn validate_stmt_mode(
    stmt: &Stmt,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match stmt {
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, lx, mode)?;
            }
        }
        Stmt::Raise { value, from, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, lx, mode)?;
            }
            if let Some(from) = from {
                validate_expr_mode(from, lx, mode)?;
            }
        }
        Stmt::Assert { test, message, .. } => {
            validate_expr_mode(test, lx, mode)?;
            if let Some(message) = message {
                validate_expr_mode(message, lx, mode)?;
            }
        }
        Stmt::Delete { targets, .. } => {
            for target in targets {
                validate_assign_target_mode(target, lx, mode)?;
            }
        }
        Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Pass { .. }
        | Stmt::SegmentBreak { .. } => {}
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                validate_assign_target_mode(target, lx, mode)?;
            }
            validate_expr_mode(value, lx, mode)?;
        }
        Stmt::Expr { value, .. } => {
            validate_expr_mode(value, lx, mode)?;
        }
        Stmt::PatternAction {
            pattern,
            action,
            span,
            ..
        } => {
            if mode != ValidationMode::Awk {
                return Err(error_with_span(
                    "pattern/action rules are only valid inside awk { } blocks",
                    span.clone(),
                    lx,
                ));
            }
            if let Some(pattern) = pattern {
                validate_expr_mode(pattern, lx, mode)?;
            }
            if let Some(action) = action {
                validate_block_mode(action, lx, mode)?;
            }
        }
    }
    Ok(())
}

fn validate_block_mode(
    block: &[Stmt],
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for stmt in block {
        validate_stmt_mode(stmt, lx, mode)?;
    }
    Ok(())
}

fn validate_with_item_mode(
    item: &WithItem,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    validate_expr_mode(&item.context, lx, mode)?;
    if let Some(target) = &item.target {
        validate_assign_target_mode(target, lx, mode)?;
    }
    Ok(())
}

fn validate_except_handler_mode(
    handler: &ExceptHandler,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let Some(expr) = &handler.type_name {
        validate_expr_mode(expr, lx, mode)?;
    }
    validate_block_mode(&handler.body, lx, mode)?;
    Ok(())
}

fn validate_param_mode(
    param: &Parameter,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match param {
        Parameter::Regular { default, .. } => {
            if let Some(default) = default {
                validate_expr_mode(default, lx, mode)?;
            }
        }
        Parameter::VarArgs { .. }
        | Parameter::KwArgs { .. }
        | Parameter::PosonlySep { .. }
        | Parameter::KwonlySep { .. } => {}
    }
    Ok(())
}

fn validate_argument_mode(
    arg: &Argument,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match arg {
        Argument::Positional { value, .. }
        | Argument::Keyword { value, .. }
        | Argument::Star { value, .. }
        | Argument::KwStar { value, .. } => validate_expr_mode(value, lx, mode),
    }
}

fn validate_assign_target_mode(
    target: &AssignTarget,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Attribute { value, .. } => validate_expr_mode(value, lx, mode),
        AssignTarget::Index { value, index, .. } => {
            validate_expr_mode(value, lx, mode)?;
            validate_expr_mode(index, lx, mode)
        }
        AssignTarget::Starred { target, .. } => validate_assign_target_mode(target, lx, mode),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                validate_assign_target_mode(element, lx, mode)?;
            }
            Ok(())
        }
    }
}

fn validate_condition_mode(
    cond: &Condition,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match cond {
        Condition::Expr(expr) => validate_expr_mode(expr, lx, mode),
        Condition::Let {
            target,
            value,
            guard,
            ..
        } => {
            validate_assign_target_mode(target, lx, mode)?;
            validate_expr_mode(value, lx, mode)?;
            if let Some(guard) = guard {
                validate_expr_mode(guard, lx, mode)?;
            }
            Ok(())
        }
    }
}

fn validate_name_for_mode(
    name: &str,
    span: &SourceSpan,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match mode {
        ValidationMode::Awk => {
            // Awk mode allows awk vars ($n, $fn, $m, $f) and $src, but not $fd/$text
            if XARGS_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(
                    "xargs variables ($fd, $text) are not valid inside awk { } blocks",
                    span.clone(),
                    lx,
                ));
            }
        }
        ValidationMode::Xargs => {
            // Xargs mode allows xargs vars ($src, $fd, $text) but not awk-only vars
            if AWK_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(
                    "awk variables ($n, $fn, $m, $f) are not valid inside xargs { } blocks",
                    span.clone(),
                    lx,
                ));
            }
        }
        ValidationMode::Main => {
            // Main mode rejects all special vars
            if AWK_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), lx));
            }
            if XARGS_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(XARGS_ONLY_MESSAGE, span.clone(), lx));
            }
            if XARGS_OR_AWK_NAMES.contains(&name) {
                return Err(error_with_span(XARGS_OR_AWK_MESSAGE, span.clone(), lx));
            }
        }
    }
    Ok(())
}

fn validate_exprs_mode(
    exprs: &[Expr],
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for expr in exprs {
        validate_expr_mode(expr, lx, mode)?;
    }
    Ok(())
}

fn validate_fstring_parts_mode(
    parts: &[FStringPart],
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for part in parts {
        validate_fstring_part_mode(part, lx, mode)?;
    }
    Ok(())
}

fn validate_expr_mode(
    expr: &Expr,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match expr {
        Expr::Name { name, span } => {
            validate_name_for_mode(name, span, lx, mode)?;
        }
        Expr::FieldIndex { span, .. } => {
            if mode != ValidationMode::Awk {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), lx));
            }
        }
        Expr::Placeholder { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. } => {}
        Expr::FString { parts, .. } | Expr::Subprocess { parts, .. } => {
            validate_fstring_parts_mode(parts, lx, mode)?;
        }
        Expr::Unary { expr, .. } | Expr::YieldFrom { expr, .. } | Expr::Paren { expr, .. } => {
            validate_expr_mode(expr, lx, mode)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_expr_mode(left, lx, mode)?;
            validate_expr_mode(right, lx, mode)?;
        }
        Expr::AugAssign { target, value, .. } => {
            validate_assign_target_mode(target, lx, mode)?;
            validate_expr_mode(value, lx, mode)?;
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            validate_assign_target_mode(target, lx, mode)?;
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            validate_expr_mode(left, lx, mode)?;
            validate_exprs_mode(comparators, lx, mode)?;
        }
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, lx, mode)?;
            }
        }
        Expr::Starred { value, .. } => {
            validate_expr_mode(value, lx, mode)?;
        }
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            validate_exprs_mode(elements, lx, mode)?
        }
        Expr::Regex { pattern, .. } => {
            validate_regex_pattern_mode(pattern, lx, mode)?;
        }
        Expr::RegexMatch { value, pattern, .. } => {
            validate_expr_mode(value, lx, mode)?;
            validate_regex_pattern_mode(pattern, lx, mode)?;
        }
        Expr::Call { func, args, .. } => {
            validate_expr_mode(func, lx, mode)?;
            for arg in args {
                validate_argument_mode(arg, lx, mode)?;
            }
        }
        Expr::Attribute { value, .. } => {
            validate_expr_mode(value, lx, mode)?;
        }
        Expr::Index { value, index, .. } => {
            validate_expr_mode(value, lx, mode)?;
            validate_expr_mode(index, lx, mode)?;
        }
        Expr::Dict { entries, .. } => {
            for entry in entries {
                match entry {
                    DictEntry::KeyValue { key, value, .. } => {
                        validate_expr_mode(key, lx, mode)?;
                        validate_expr_mode(value, lx, mode)?;
                    }
                    DictEntry::Unpack { value, .. } => {
                        validate_expr_mode(value, lx, mode)?;
                    }
                }
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_mode(start, lx, mode)?;
            }
            if let Some(end) = end {
                validate_expr_mode(end, lx, mode)?;
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        }
        | Expr::GeneratorExpr {
            element, iter, ifs, ..
        } => {
            validate_expr_mode(element, lx, mode)?;
            validate_expr_mode(iter, lx, mode)?;
            validate_exprs_mode(ifs, lx, mode)?;
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            validate_expr_mode(key, lx, mode)?;
            validate_expr_mode(value, lx, mode)?;
            validate_expr_mode(iter, lx, mode)?;
            validate_exprs_mode(ifs, lx, mode)?;
        }
        Expr::Block { stmts, .. } => {
            validate_block_mode(stmts, lx, mode)?;
        }
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            validate_condition_mode(cond, lx, mode)?;
            validate_block_mode(body, lx, mode)?;
            for (elif_cond, elif_body) in elifs {
                validate_condition_mode(elif_cond, lx, mode)?;
                validate_block_mode(elif_body, lx, mode)?;
            }
            if let Some(body) = else_body {
                validate_block_mode(body, lx, mode)?;
            }
        }
        Expr::While {
            cond,
            body,
            else_body,
            ..
        } => {
            validate_condition_mode(cond, lx, mode)?;
            validate_block_mode(body, lx, mode)?;
            if let Some(body) = else_body {
                validate_block_mode(body, lx, mode)?;
            }
        }
        Expr::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            validate_assign_target_mode(target, lx, mode)?;
            validate_expr_mode(iter, lx, mode)?;
            validate_block_mode(body, lx, mode)?;
            if let Some(body) = else_body {
                validate_block_mode(body, lx, mode)?;
            }
        }
        Expr::Def { params, body, .. } => {
            for param in params {
                validate_param_mode(param, lx, mode)?;
            }
            validate_block_mode(body, lx, mode)?;
        }
        Expr::Class { body, .. } => {
            validate_block_mode(body, lx, mode)?;
        }
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            validate_block_mode(body, lx, mode)?;
            for handler in handlers {
                validate_except_handler_mode(handler, lx, mode)?;
            }
            if let Some(body) = else_body {
                validate_block_mode(body, lx, mode)?;
            }
            if let Some(body) = finally_body {
                validate_block_mode(body, lx, mode)?;
            }
        }
        Expr::With { items, body, .. } => {
            for item in items {
                validate_with_item_mode(item, lx, mode)?;
            }
            validate_block_mode(body, lx, mode)?;
        }
        Expr::Awk { sources, body, .. } => {
            for src in sources {
                validate_argument_mode(src, lx, mode)?;
            }
            validate_block_mode(body, lx, ValidationMode::Awk)?;
        }
        Expr::Xargs { sources, body, .. } => {
            for src in sources {
                validate_argument_mode(src, lx, mode)?;
            }
            validate_block_mode(body, lx, ValidationMode::Xargs)?;
        }
    }
    Ok(())
}

fn validate_regex_pattern_mode(
    pattern: &RegexPattern,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let RegexPattern::Interpolated(parts) = pattern {
        validate_fstring_parts_mode(parts, lx, mode)?;
    }
    Ok(())
}

fn validate_fstring_part_mode(
    part: &FStringPart,
    lx: &LineIndex<'_>,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let FStringPart::Expr(expr) = part {
        validate_expr_mode(&expr.expr, lx, mode)?;
        if let Some(spec) = &expr.format_spec {
            validate_fstring_parts_mode(spec, lx, mode)?;
        }
    }
    Ok(())
}
