use snail_ast::*;
use snail_error::LowerError;
use snail_python_ast::*;

use crate::expr::{lower_assign_target, lower_expr};
use crate::span::{expr_span, span_from_block, stmt_span};

pub(crate) fn lower_stmt(stmt: &Stmt) -> Result<PyStmt, LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => lower_if(cond, body, elifs, else_body, span),
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => Ok(PyStmt::While {
            test: lower_expr(cond)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => Ok(PyStmt::For {
            target: lower_assign_target(target)?,
            iter: lower_expr(iter)?,
            body: lower_block(body)?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::Def {
            name,
            params,
            body,
            span,
        } => Ok(PyStmt::FunctionDef {
            name: name.clone(),
            args: params
                .iter()
                .map(lower_parameter)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Class { name, body, span } => Ok(PyStmt::ClassDef {
            name: name.clone(),
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => Ok(PyStmt::Try {
            body: lower_block(body)?,
            handlers: handlers
                .iter()
                .map(lower_except_handler)
                .collect::<Result<Vec<_>, _>>()?,
            orelse: else_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            finalbody: finally_body
                .as_ref()
                .map(|items| lower_block(items))
                .transpose()?
                .unwrap_or_default(),
            span: span.clone(),
        }),
        Stmt::With { items, body, span } => Ok(PyStmt::With {
            items: items
                .iter()
                .map(lower_with_item)
                .collect::<Result<Vec<_>, _>>()?,
            body: lower_block(body)?,
            span: span.clone(),
        }),
        Stmt::Return { value, span } => Ok(PyStmt::Return {
            value: value.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Raise { value, from, span } => Ok(PyStmt::Raise {
            value: value.as_ref().map(lower_expr).transpose()?,
            from: from.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Assert {
            test,
            message,
            span,
        } => Ok(PyStmt::Assert {
            test: lower_expr(test)?,
            message: message.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Stmt::Delete { targets, span } => Ok(PyStmt::Delete {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            span: span.clone(),
        }),
        Stmt::Break { span } => Ok(PyStmt::Break { span: span.clone() }),
        Stmt::Continue { span } => Ok(PyStmt::Continue { span: span.clone() }),
        Stmt::Pass { span } => Ok(PyStmt::Pass { span: span.clone() }),
        Stmt::Import { items, span } => Ok(PyStmt::Import {
            names: items.iter().map(lower_import_name).collect(),
            span: span.clone(),
        }),
        Stmt::ImportFrom {
            module,
            items,
            span,
        } => {
            if module.len() == 1 && module[0] == "__future__" {
                let filtered: Vec<&ImportItem> = items
                    .iter()
                    .filter(|item| !(item.name.len() == 1 && item.name[0] == "braces"))
                    .collect();
                if filtered.is_empty() {
                    return Ok(PyStmt::Pass { span: span.clone() });
                }
                return Ok(PyStmt::ImportFrom {
                    module: module.clone(),
                    names: filtered
                        .iter()
                        .map(|item| lower_import_name(item))
                        .collect(),
                    span: span.clone(),
                });
            }
            Ok(PyStmt::ImportFrom {
                module: module.clone(),
                names: items.iter().map(lower_import_name).collect(),
                span: span.clone(),
            })
        }
        Stmt::Assign {
            targets,
            value,
            span,
        } => Ok(PyStmt::Assign {
            targets: targets
                .iter()
                .map(lower_assign_target)
                .collect::<Result<Vec<_>, _>>()?,
            value: lower_expr(value)?,
            span: span.clone(),
        }),
        Stmt::Expr {
            value,
            semicolon_terminated,
            span,
        } => Ok(PyStmt::Expr {
            value: lower_expr(value)?,
            semicolon_terminated: *semicolon_terminated,
            span: span.clone(),
        }),
    }
}

fn lower_if(
    cond: &Expr,
    body: &[Stmt],
    elifs: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span_hint: &SourceSpan,
) -> Result<PyStmt, LowerError> {
    let test = lower_expr(cond)?;
    let body = lower_block(body)?;
    let mut span = span_from_block(&body).unwrap_or_else(|| span_hint.clone());
    span.start = expr_span(&test).start.clone();
    if let Some((elif_cond, elif_body)) = elifs.first() {
        let nested = lower_if(elif_cond, elif_body, &elifs[1..], else_body, span_hint)?;
        span.end = stmt_span(&nested).end.clone();
        Ok(PyStmt::If {
            test,
            body,
            orelse: vec![nested],
            span,
        })
    } else {
        let orelse = match else_body {
            Some(else_block) => lower_block(else_block)?,
            None => Vec::new(),
        };
        if let Some(last) = orelse.last() {
            span.end = stmt_span(last).end.clone();
        }
        Ok(PyStmt::If {
            test,
            body,
            orelse,
            span,
        })
    }
}

pub(crate) fn lower_block(block: &[Stmt]) -> Result<Vec<PyStmt>, LowerError> {
    block.iter().map(lower_stmt).collect()
}

fn lower_except_handler(handler: &ExceptHandler) -> Result<PyExceptHandler, LowerError> {
    Ok(PyExceptHandler {
        type_name: handler.type_name.as_ref().map(lower_expr).transpose()?,
        name: handler.name.clone(),
        body: lower_block(&handler.body)?,
        span: handler.span.clone(),
    })
}

fn lower_with_item(item: &WithItem) -> Result<PyWithItem, LowerError> {
    Ok(PyWithItem {
        context: lower_expr(&item.context)?,
        target: item.target.as_ref().map(lower_assign_target).transpose()?,
        span: item.span.clone(),
    })
}

fn lower_import_name(item: &ImportItem) -> PyImportName {
    PyImportName {
        name: item.name.clone(),
        asname: item.alias.clone(),
        span: item.span.clone(),
    }
}

fn lower_parameter(param: &Parameter) -> Result<PyParameter, LowerError> {
    match param {
        Parameter::Regular {
            name,
            default,
            span,
        } => Ok(PyParameter::Regular {
            name: name.clone(),
            default: default.as_ref().map(lower_expr).transpose()?,
            span: span.clone(),
        }),
        Parameter::VarArgs { name, span } => Ok(PyParameter::VarArgs {
            name: name.clone(),
            span: span.clone(),
        }),
        Parameter::KwArgs { name, span } => Ok(PyParameter::KwArgs {
            name: name.clone(),
            span: span.clone(),
        }),
    }
}
