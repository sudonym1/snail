use snail_ast::*;
use snail_error::LowerError;

pub(crate) fn validate_yield_usage_program(program: &Program) -> Result<(), LowerError> {
    check_stmts(&program.stmts, false)
}

fn check_stmts(stmts: &[Stmt], in_function: bool) -> Result<(), LowerError> {
    for stmt in stmts {
        check_stmt(stmt, in_function)?;
    }
    Ok(())
}

fn check_stmt(stmt: &Stmt, in_function: bool) -> Result<(), LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            ..
        } => {
            check_condition(cond, in_function)?;
            check_stmts(body, in_function)?;
            for (elif_cond, elif_body) in elifs {
                check_condition(elif_cond, in_function)?;
                check_stmts(elif_body, in_function)?;
            }
            if let Some(else_body) = else_body {
                check_stmts(else_body, in_function)?;
            }
        }
        Stmt::While {
            cond,
            body,
            else_body,
            ..
        } => {
            check_condition(cond, in_function)?;
            check_stmts(body, in_function)?;
            if let Some(else_body) = else_body {
                check_stmts(else_body, in_function)?;
            }
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            check_assign_target(target, in_function)?;
            check_expr(iter, in_function)?;
            check_stmts(body, in_function)?;
            if let Some(else_body) = else_body {
                check_stmts(else_body, in_function)?;
            }
        }
        Stmt::Def { params, body, .. } => {
            for param in params {
                check_param(param)?;
            }
            check_stmts(body, true)?;
        }
        Stmt::Class { body, .. } => {
            check_stmts(body, false)?;
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            check_stmts(body, in_function)?;
            for handler in handlers {
                if let Some(type_name) = &handler.type_name {
                    check_expr(type_name, in_function)?;
                }
                check_stmts(&handler.body, in_function)?;
            }
            if let Some(else_body) = else_body {
                check_stmts(else_body, in_function)?;
            }
            if let Some(finally_body) = finally_body {
                check_stmts(finally_body, in_function)?;
            }
        }
        Stmt::With { items, body, .. } => {
            for item in items {
                check_expr(&item.context, in_function)?;
                if let Some(target) = &item.target {
                    check_assign_target(target, in_function)?;
                }
            }
            check_stmts(body, in_function)?;
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                check_expr(value, in_function)?;
            }
        }
        Stmt::Raise { value, from, .. } => {
            if let Some(value) = value {
                check_expr(value, in_function)?;
            }
            if let Some(from) = from {
                check_expr(from, in_function)?;
            }
        }
        Stmt::Assert { test, message, .. } => {
            check_expr(test, in_function)?;
            if let Some(message) = message {
                check_expr(message, in_function)?;
            }
        }
        Stmt::Delete { targets, .. } => {
            for target in targets {
                check_assign_target(target, in_function)?;
            }
        }
        Stmt::Assign { targets, value, .. } => {
            for target in targets {
                check_assign_target(target, in_function)?;
            }
            check_expr(value, in_function)?;
        }
        Stmt::Expr { value, .. } => {
            check_expr(value, in_function)?;
        }
        Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Pass { .. }
        | Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::SegmentBreak { .. } => {}
        Stmt::Lines { sources, body, .. } => {
            for source in sources {
                check_expr(source, in_function)?;
            }
            check_stmts(body, in_function)?;
        }
        Stmt::Files { source, body, .. } => {
            if let Some(source) = source {
                check_expr(source, in_function)?;
            }
            check_stmts(body, in_function)?;
        }
        Stmt::PatternAction {
            pattern, action, ..
        } => {
            if let Some(pattern) = pattern {
                check_expr(pattern, in_function)?;
            }
            if let Some(action) = action {
                check_stmts(action, in_function)?;
            }
        }
    }
    Ok(())
}

fn check_condition(cond: &Condition, in_function: bool) -> Result<(), LowerError> {
    match cond {
        Condition::Expr(expr) => check_expr(expr, in_function),
        Condition::Let {
            target,
            value,
            guard,
            ..
        } => {
            check_assign_target(target, in_function)?;
            check_expr(value, in_function)?;
            if let Some(guard) = guard {
                check_expr(guard, in_function)?;
            }
            Ok(())
        }
    }
}

fn check_param(param: &Parameter) -> Result<(), LowerError> {
    if let Parameter::Regular {
        default: Some(default),
        ..
    } = param
    {
        check_expr(default, false)?;
    }
    Ok(())
}

fn check_assign_target(target: &AssignTarget, in_function: bool) -> Result<(), LowerError> {
    match target {
        AssignTarget::Name { .. } => {}
        AssignTarget::Attribute { value, .. } => check_expr(value, in_function)?,
        AssignTarget::Index { value, index, .. } => {
            check_expr(value, in_function)?;
            check_expr(index, in_function)?;
        }
        AssignTarget::Starred { target, .. } => check_assign_target(target, in_function)?,
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                check_assign_target(element, in_function)?;
            }
        }
    }
    Ok(())
}

fn check_expr(expr: &Expr, in_function: bool) -> Result<(), LowerError> {
    match expr {
        Expr::Yield { value, .. } => {
            if !in_function {
                return Err(yield_error());
            }
            if let Some(value) = value {
                check_expr(value, in_function)?;
            }
        }
        Expr::YieldFrom { expr, .. } => {
            if !in_function {
                return Err(yield_error());
            }
            check_expr(expr, in_function)?;
        }
        Expr::Name { .. }
        | Expr::Placeholder { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => {}
        Expr::FString { parts, .. } => check_fstring_parts(parts, in_function)?,
        Expr::Unary { expr, .. } => check_expr(expr, in_function)?,
        Expr::Binary { left, right, .. } => {
            check_expr(left, in_function)?;
            check_expr(right, in_function)?;
        }
        Expr::AugAssign { target, value, .. } => {
            check_assign_target(target, in_function)?;
            check_expr(value, in_function)?;
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            check_assign_target(target, in_function)?;
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            check_expr(left, in_function)?;
            for expr in comparators {
                check_expr(expr, in_function)?;
            }
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            check_expr(test, in_function)?;
            check_expr(body, in_function)?;
            check_expr(orelse, in_function)?;
        }
        Expr::TryExpr { expr, fallback, .. } => {
            check_expr(expr, in_function)?;
            if let Some(fallback) = fallback {
                check_expr(fallback, in_function)?;
            }
        }
        Expr::Lambda { params, body, .. } => {
            for param in params {
                check_param(param)?;
            }
            // Anonymous defs are hoisted to defs; allow yield in their bodies.
            check_stmts(body, true)?;
        }
        Expr::Compound { expressions, .. } => {
            for expr in expressions {
                check_expr(expr, in_function)?;
            }
        }
        Expr::Regex { pattern, .. } => check_regex_pattern(pattern, in_function)?,
        Expr::RegexMatch { value, pattern, .. } => {
            check_expr(value, in_function)?;
            check_regex_pattern(pattern, in_function)?;
        }
        Expr::Subprocess { parts, .. } => {
            check_fstring_parts(parts, in_function)?;
        }
        Expr::Call { func, args, .. } => {
            check_expr(func, in_function)?;
            for arg in args {
                match arg {
                    Argument::Positional { value, .. }
                    | Argument::Keyword { value, .. }
                    | Argument::Star { value, .. }
                    | Argument::KwStar { value, .. } => check_expr(value, in_function)?,
                }
            }
        }
        Expr::Attribute { value, .. } => check_expr(value, in_function)?,
        Expr::Index { value, index, .. } => {
            check_expr(value, in_function)?;
            check_expr(index, in_function)?;
        }
        Expr::Paren { expr, .. } => check_expr(expr, in_function)?,
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                check_expr(expr, in_function)?;
            }
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                check_expr(key, in_function)?;
                check_expr(value, in_function)?;
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                check_expr(start, in_function)?;
            }
            if let Some(end) = end {
                check_expr(end, in_function)?;
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            check_expr(element, in_function)?;
            check_expr(iter, in_function)?;
            for expr in ifs {
                check_expr(expr, in_function)?;
            }
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            check_expr(key, in_function)?;
            check_expr(value, in_function)?;
            check_expr(iter, in_function)?;
            for expr in ifs {
                check_expr(expr, in_function)?;
            }
        }
    }
    Ok(())
}

fn check_regex_pattern(pattern: &RegexPattern, in_function: bool) -> Result<(), LowerError> {
    if let RegexPattern::Interpolated(parts) = pattern {
        check_fstring_parts(parts, in_function)?;
    }
    Ok(())
}

fn check_fstring_parts(parts: &[FStringPart], in_function: bool) -> Result<(), LowerError> {
    for part in parts {
        if let FStringPart::Expr(expr) = part {
            check_fstring_expr(expr, in_function)?;
        }
    }
    Ok(())
}

fn check_fstring_expr(expr: &FStringExpr, in_function: bool) -> Result<(), LowerError> {
    check_expr(&expr.expr, in_function)?;
    if let Some(format_spec) = &expr.format_spec {
        check_fstring_parts(format_spec, in_function)?;
    }
    Ok(())
}

fn yield_error() -> LowerError {
    LowerError::new("yield expressions are only allowed inside function bodies")
}
