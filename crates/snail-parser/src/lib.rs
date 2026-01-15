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
use util::{error_with_span, full_span, parse_error_from_pest};

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
    let program = Program { stmts, span };
    validate_no_awk_syntax(&program, source)?;
    Ok(program)
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

const AWK_ONLY_NAMES: [&str; 6] = ["$l", "$f", "$n", "$fn", "$p", "$m"];
const AWK_ONLY_MESSAGE: &str = "awk variables are only valid in awk mode; use --awk";

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
            validate_expr(cond, source)?;
            validate_block(body, source)?;
            for (elif_cond, elif_body) in elifs {
                validate_expr(elif_cond, source)?;
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
            validate_expr(cond, source)?;
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
    }
}

fn validate_expr(expr: &Expr, source: &str) -> Result<(), ParseError> {
    match expr {
        Expr::Name { name, span } => {
            if AWK_ONLY_NAMES.contains(&name.as_str()) {
                return Err(error_with_span(AWK_ONLY_MESSAGE, span.clone(), source));
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
                if let SubprocessPart::Expr(expr) = part {
                    validate_expr(expr, source)?;
                }
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
        validate_expr(expr, source)?;
    }
    Ok(())
}
