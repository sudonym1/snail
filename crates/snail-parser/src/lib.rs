use pest::Parser;
use pest_derive::Parser;

use snail_ast::*;
use snail_error::ParseError;

mod expr;
mod literal;
pub mod preprocess;
mod stmt;
mod string;
mod util;

use stmt::parse_stmt_list;
use util::{error_with_span, full_span, parse_error_from_pest};

#[derive(Parser)]
#[grammar = "snail.pest"]
pub struct SnailParser;

/// Parses a regular Snail program.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let preprocessed = preprocess::preprocess(source)?;
    let mut pairs = SnailParser::parse(Rule::program, &preprocessed)
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

    let program = Program { stmts, span };
    validate_program(&program, source, ValidationMode::Main)?;
    Ok(program)
}

const AWK_ONLY_NAMES: [&str; 4] = ["$n", "$fn", "$m", "$f"];
const AWK_ONLY_MESSAGE: &str = "awk variables are only valid inside lines { } blocks";

const MAP_ONLY_NAMES: [&str; 2] = ["$fd", "$text"];
const MAP_ONLY_MESSAGE: &str = "map variables are only valid inside files { } blocks";
const MAP_OR_AWK_NAMES: [&str; 1] = ["$src"];
const MAP_OR_AWK_MESSAGE: &str = "$src is only valid inside lines { } or files { } blocks";

#[derive(Clone, Copy, Eq, PartialEq)]
enum ValidationMode {
    Main,
    Lines,
    Files,
}

fn validate_program(
    program: &Program,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for stmt in &program.stmts {
        validate_stmt_mode(stmt, source, mode)?;
    }
    Ok(())
}

fn validate_stmt_mode(stmt: &Stmt, source: &str, mode: ValidationMode) -> Result<(), ParseError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            validate_condition_mode(cond, source, mode)?;
            validate_block_mode(body, source, mode)?;
            for (elif_cond, elif_body) in elifs {
                validate_condition_mode(elif_cond, source, mode)?;
                validate_block_mode(elif_body, source, mode)?;
            }
            if let Some(body) = else_body {
                validate_block_mode(body, source, mode)?;
            }
        }
        Stmt::While {
            cond,
            body,
            else_body,
            ..
        } => {
            validate_condition_mode(cond, source, mode)?;
            validate_block_mode(body, source, mode)?;
            if let Some(body) = else_body {
                validate_block_mode(body, source, mode)?;
            }
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            validate_assign_target_mode(target, source, mode)?;
            validate_expr_mode(iter, source, mode)?;
            validate_block_mode(body, source, mode)?;
            if let Some(body) = else_body {
                validate_block_mode(body, source, mode)?;
            }
        }
        Stmt::Def { params, body, .. } => {
            for param in params {
                validate_param_mode(param, source, mode)?;
            }
            validate_block_mode(body, source, mode)?;
        }
        Stmt::Class { body, .. } => {
            validate_block_mode(body, source, mode)?;
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            validate_block_mode(body, source, mode)?;
            for handler in handlers {
                validate_except_handler_mode(handler, source, mode)?;
            }
            if let Some(body) = else_body {
                validate_block_mode(body, source, mode)?;
            }
            if let Some(body) = finally_body {
                validate_block_mode(body, source, mode)?;
            }
        }
        Stmt::With { items, body, .. } => {
            for item in items {
                validate_with_item_mode(item, source, mode)?;
            }
            validate_block_mode(body, source, mode)?;
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, source, mode)?;
            }
        }
        Stmt::Raise { value, from, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, source, mode)?;
            }
            if let Some(from) = from {
                validate_expr_mode(from, source, mode)?;
            }
        }
        Stmt::Assert { test, message, .. } => {
            validate_expr_mode(test, source, mode)?;
            if let Some(message) = message {
                validate_expr_mode(message, source, mode)?;
            }
        }
        Stmt::Delete { targets, .. } => {
            for target in targets {
                validate_assign_target_mode(target, source, mode)?;
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
                validate_assign_target_mode(target, source, mode)?;
            }
            validate_expr_mode(value, source, mode)?;
        }
        Stmt::Expr { value, .. } => {
            validate_expr_mode(value, source, mode)?;
        }
        Stmt::Lines {
            source: src, body, ..
        } => {
            if let Some(src) = src {
                validate_expr_mode(src, source, mode)?;
            }
            validate_block_mode(body, source, ValidationMode::Lines)?;
        }
        Stmt::Files {
            source: src, body, ..
        } => {
            if let Some(src) = src {
                validate_expr_mode(src, source, mode)?;
            }
            validate_block_mode(body, source, ValidationMode::Files)?;
        }
        Stmt::PatternAction {
            pattern,
            action,
            span,
            ..
        } => {
            if mode != ValidationMode::Lines {
                return Err(error_with_span(
                    "pattern/action rules are only valid inside lines { } blocks",
                    span.clone(),
                    source,
                ));
            }
            if let Some(pattern) = pattern {
                validate_expr_mode(pattern, source, mode)?;
            }
            if let Some(action) = action {
                validate_block_mode(action, source, mode)?;
            }
        }
    }
    Ok(())
}

fn validate_block_mode(
    block: &[Stmt],
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for stmt in block {
        validate_stmt_mode(stmt, source, mode)?;
    }
    Ok(())
}

fn validate_with_item_mode(
    item: &WithItem,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    validate_expr_mode(&item.context, source, mode)?;
    if let Some(target) = &item.target {
        validate_assign_target_mode(target, source, mode)?;
    }
    Ok(())
}

fn validate_except_handler_mode(
    handler: &ExceptHandler,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let Some(expr) = &handler.type_name {
        validate_expr_mode(expr, source, mode)?;
    }
    validate_block_mode(&handler.body, source, mode)?;
    Ok(())
}

fn validate_param_mode(
    param: &Parameter,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match param {
        Parameter::Regular { default, .. } => {
            if let Some(default) = default {
                validate_expr_mode(default, source, mode)?;
            }
        }
        Parameter::VarArgs { .. } | Parameter::KwArgs { .. } => {}
    }
    Ok(())
}

fn validate_argument_mode(
    arg: &Argument,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match arg {
        Argument::Positional { value, .. }
        | Argument::Keyword { value, .. }
        | Argument::Star { value, .. }
        | Argument::KwStar { value, .. } => validate_expr_mode(value, source, mode),
    }
}

fn validate_assign_target_mode(
    target: &AssignTarget,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Attribute { value, .. } => validate_expr_mode(value, source, mode),
        AssignTarget::Index { value, index, .. } => {
            validate_expr_mode(value, source, mode)?;
            validate_expr_mode(index, source, mode)
        }
        AssignTarget::Starred { target, .. } => validate_assign_target_mode(target, source, mode),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                validate_assign_target_mode(element, source, mode)?;
            }
            Ok(())
        }
    }
}

fn validate_condition_mode(
    cond: &Condition,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match cond {
        Condition::Expr(expr) => validate_expr_mode(expr, source, mode),
        Condition::Let {
            target,
            value,
            guard,
            ..
        } => {
            validate_assign_target_mode(target, source, mode)?;
            validate_expr_mode(value, source, mode)?;
            if let Some(guard) = guard {
                validate_expr_mode(guard, source, mode)?;
            }
            Ok(())
        }
    }
}

fn validate_name_for_mode(
    name: &str,
    span: &SourceSpan,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    match mode {
        ValidationMode::Lines => {
            // Lines mode allows awk vars ($n, $fn, $m, $f) and $src, but not $fd/$text
            if MAP_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(
                    "map variables ($fd, $text) are not valid inside lines { } blocks",
                    span.clone(),
                    source,
                ));
            }
        }
        ValidationMode::Files => {
            // Files mode allows map vars ($src, $fd, $text) but not awk-only vars
            if AWK_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(
                    "awk variables ($n, $fn, $m, $f) are not valid inside files { } blocks",
                    span.clone(),
                    source,
                ));
            }
        }
        ValidationMode::Main => {
            // Main mode rejects all special vars
            if AWK_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
            }
            if MAP_ONLY_NAMES.contains(&name) {
                return Err(error_with_span(MAP_ONLY_MESSAGE, span.clone(), source));
            }
            if MAP_OR_AWK_NAMES.contains(&name) {
                return Err(error_with_span(MAP_OR_AWK_MESSAGE, span.clone(), source));
            }
        }
    }
    Ok(())
}

fn validate_exprs_mode(
    exprs: &[Expr],
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for expr in exprs {
        validate_expr_mode(expr, source, mode)?;
    }
    Ok(())
}

fn validate_fstring_parts_mode(
    parts: &[FStringPart],
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    for part in parts {
        validate_fstring_part_mode(part, source, mode)?;
    }
    Ok(())
}

fn validate_expr_mode(expr: &Expr, source: &str, mode: ValidationMode) -> Result<(), ParseError> {
    match expr {
        Expr::Name { name, span } => {
            validate_name_for_mode(name, span, source, mode)?;
        }
        Expr::FieldIndex { span, .. } => {
            if mode != ValidationMode::Lines {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
            }
        }
        Expr::Placeholder { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. } => {}
        Expr::FString { parts, .. } | Expr::Subprocess { parts, .. } => {
            validate_fstring_parts_mode(parts, source, mode)?;
        }
        Expr::Unary { expr, .. } | Expr::YieldFrom { expr, .. } | Expr::Paren { expr, .. } => {
            validate_expr_mode(expr, source, mode)?;
        }
        Expr::Binary { left, right, .. } => {
            validate_expr_mode(left, source, mode)?;
            validate_expr_mode(right, source, mode)?;
        }
        Expr::AugAssign { target, value, .. } => {
            validate_assign_target_mode(target, source, mode)?;
            validate_expr_mode(value, source, mode)?;
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            validate_assign_target_mode(target, source, mode)?;
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            validate_expr_mode(left, source, mode)?;
            validate_exprs_mode(comparators, source, mode)?;
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            validate_expr_mode(test, source, mode)?;
            validate_expr_mode(body, source, mode)?;
            validate_expr_mode(orelse, source, mode)?;
        }
        Expr::TryExpr { expr, fallback, .. } => {
            validate_expr_mode(expr, source, mode)?;
            if let Some(fallback) = fallback {
                validate_expr_mode(fallback, source, mode)?;
            }
        }
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                validate_expr_mode(value, source, mode)?;
            }
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                validate_param_mode(param, source, mode)?;
            }
            validate_block_mode(body, source, mode)?;
        }
        Expr::Compound {
            expressions: elements,
            ..
        }
        | Expr::List { elements, .. }
        | Expr::Tuple { elements, .. }
        | Expr::Set { elements, .. } => validate_exprs_mode(elements, source, mode)?,
        Expr::Regex { pattern, .. } => {
            validate_regex_pattern_mode(pattern, source, mode)?;
        }
        Expr::RegexMatch { value, pattern, .. } => {
            validate_expr_mode(value, source, mode)?;
            validate_regex_pattern_mode(pattern, source, mode)?;
        }
        Expr::Call { func, args, .. } => {
            validate_expr_mode(func, source, mode)?;
            for arg in args {
                validate_argument_mode(arg, source, mode)?;
            }
        }
        Expr::Attribute { value, .. } => {
            validate_expr_mode(value, source, mode)?;
        }
        Expr::Index { value, index, .. } => {
            validate_expr_mode(value, source, mode)?;
            validate_expr_mode(index, source, mode)?;
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                validate_expr_mode(key, source, mode)?;
                validate_expr_mode(value, source, mode)?;
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_mode(start, source, mode)?;
            }
            if let Some(end) = end {
                validate_expr_mode(end, source, mode)?;
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            validate_expr_mode(element, source, mode)?;
            validate_expr_mode(iter, source, mode)?;
            validate_exprs_mode(ifs, source, mode)?;
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            validate_expr_mode(key, source, mode)?;
            validate_expr_mode(value, source, mode)?;
            validate_expr_mode(iter, source, mode)?;
            validate_exprs_mode(ifs, source, mode)?;
        }
    }
    Ok(())
}

fn validate_regex_pattern_mode(
    pattern: &RegexPattern,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let RegexPattern::Interpolated(parts) = pattern {
        validate_fstring_parts_mode(parts, source, mode)?;
    }
    Ok(())
}

fn validate_fstring_part_mode(
    part: &FStringPart,
    source: &str,
    mode: ValidationMode,
) -> Result<(), ParseError> {
    if let FStringPart::Expr(expr) = part {
        validate_expr_mode(&expr.expr, source, mode)?;
        if let Some(spec) = &expr.format_spec {
            validate_fstring_parts_mode(spec, source, mode)?;
        }
    }
    Ok(())
}
