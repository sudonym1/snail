use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::{SNAIL_LET_KEEP, SNAIL_LET_OK, SNAIL_LET_VALUE};
use super::expr::{
    lower_assign_target, lower_delete_target, lower_expr, lower_expr_with_exception,
};
use super::helpers::{assign_name, name_expr};
use super::operators::lower_compare_op;
use super::py_ast::{AstBuilder, py_err_to_lower};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TailBehavior {
    None,
    AutoPrint,
    ImplicitReturn,
}

pub(crate) fn lower_stmt(builder: &AstBuilder<'_>, stmt: &Stmt) -> Result<PyObject, LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => match cond {
            Condition::Expr(cond) => {
                lower_if_expr(builder, cond.as_ref(), body, elifs, else_body, span)
            }
            Condition::Let { .. } => {
                Err(LowerError::new("if let should be lowered via lower_block"))
            }
        },
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => {
            let Condition::Expr(cond) = cond else {
                return Err(LowerError::new(
                    "while let should be lowered via lower_block",
                ));
            };
            let test = lower_expr(builder, cond.as_ref())?;
            let body = lower_block(builder, body, span)?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items, span))
                .transpose()?
                .unwrap_or_default();
            builder
                .call_node(
                    "While",
                    vec![
                        test,
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => {
            let target = lower_assign_target(builder, target)?;
            let iter = lower_expr(builder, iter)?;
            let body = lower_block(builder, body, span)?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items, span))
                .transpose()?
                .unwrap_or_default();
            builder
                .call_node(
                    "For",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Def {
            name,
            params,
            body,
            span,
        } => {
            let args = lower_parameters(builder, params, None)?;
            let body = lower_block_with_implicit_return(builder, body, span)?;
            builder
                .call_node(
                    "FunctionDef",
                    vec![
                        name.to_string().into_py(builder.py()),
                        args,
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                        builder.py().None().into_py(builder.py()),
                        builder.py().None().into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Class { name, body, span } => {
            let body = lower_block(builder, body, span)?;
            builder
                .call_node(
                    "ClassDef",
                    vec![
                        name.to_string().into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => {
            let body = lower_block(builder, body, span)?;
            let handlers = handlers
                .iter()
                .map(|handler| lower_except_handler(builder, handler))
                .collect::<Result<Vec<_>, _>>()?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items, span))
                .transpose()?
                .unwrap_or_default();
            let finalbody = finally_body
                .as_ref()
                .map(|items| lower_block(builder, items, span))
                .transpose()?
                .unwrap_or_default();
            builder
                .call_node(
                    "Try",
                    vec![
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::new_bound(builder.py(), handlers).into_py(builder.py()),
                        PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
                        PyList::new_bound(builder.py(), finalbody).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::With { items, body, span } => {
            let items = items
                .iter()
                .map(|item| lower_with_item(builder, item))
                .collect::<Result<Vec<_>, _>>()?;
            let body = lower_block(builder, body, span)?;
            builder
                .call_node(
                    "With",
                    vec![
                        PyList::new_bound(builder.py(), items).into_py(builder.py()),
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Return { value, span } => {
            let value = value
                .as_ref()
                .map(|expr| lower_expr(builder, expr))
                .transpose()?
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            builder
                .call_node("Return", vec![value], span)
                .map_err(py_err_to_lower)
        }
        Stmt::Raise { value, from, span } => {
            let value = value
                .as_ref()
                .map(|expr| lower_expr(builder, expr))
                .transpose()?
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            let from = from
                .as_ref()
                .map(|expr| lower_expr(builder, expr))
                .transpose()?
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            builder
                .call_node("Raise", vec![value, from], span)
                .map_err(py_err_to_lower)
        }
        Stmt::Assert {
            test,
            message,
            span,
        } => {
            let test = lower_expr(builder, test)?;
            let msg = message
                .as_ref()
                .map(|expr| lower_expr(builder, expr))
                .transpose()?
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            builder
                .call_node("Assert", vec![test, msg], span)
                .map_err(py_err_to_lower)
        }
        Stmt::Delete { targets, span } => {
            let targets = targets
                .iter()
                .map(|target| lower_delete_target(builder, target))
                .collect::<Result<Vec<_>, _>>()?;
            builder
                .call_node(
                    "Delete",
                    vec![PyList::new_bound(builder.py(), targets).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Break { span } => builder
            .call_node("Break", Vec::new(), span)
            .map_err(py_err_to_lower),
        Stmt::Continue { span } => builder
            .call_node("Continue", Vec::new(), span)
            .map_err(py_err_to_lower),
        Stmt::Pass { span } => builder
            .call_node("Pass", Vec::new(), span)
            .map_err(py_err_to_lower),
        Stmt::Import { items, span } => {
            let names = items
                .iter()
                .map(|item| lower_import_name(builder, item))
                .collect::<Result<Vec<_>, _>>()?;
            builder
                .call_node(
                    "Import",
                    vec![PyList::new_bound(builder.py(), names).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::ImportFrom {
            level,
            module,
            items,
            span,
        } => {
            let module_is_future = *level == 0
                && module
                    .as_ref()
                    .is_some_and(|module| module.len() == 1 && module[0] == "__future__");
            let names = match items {
                ImportFromItems::Names(items) => {
                    let filtered_items: Vec<&ImportItem> = if module_is_future {
                        items
                            .iter()
                            .filter(|item| !(item.name.len() == 1 && item.name[0] == "braces"))
                            .collect()
                    } else {
                        items.iter().collect()
                    };
                    if filtered_items.is_empty() {
                        return builder
                            .call_node("Pass", Vec::new(), span)
                            .map_err(py_err_to_lower);
                    }
                    filtered_items
                        .iter()
                        .map(|item| lower_import_name(builder, item))
                        .collect::<Result<Vec<_>, _>>()?
                }
                ImportFromItems::Star { .. } => vec![lower_import_star(builder)?],
            };
            let module_name = module
                .as_ref()
                .map(|module| module.join(".").into_py(builder.py()))
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            builder
                .call_node(
                    "ImportFrom",
                    vec![
                        module_name,
                        PyList::new_bound(builder.py(), names).into_py(builder.py()),
                        (*level as u32).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Assign {
            targets,
            value,
            span,
        } => {
            let targets = targets
                .iter()
                .map(|target| lower_assign_target(builder, target))
                .collect::<Result<Vec<_>, _>>()?;
            let value = lower_expr(builder, value)?;
            builder
                .call_node(
                    "Assign",
                    vec![
                        PyList::new_bound(builder.py(), targets).into_py(builder.py()),
                        value,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Stmt::Expr {
            value,
            semicolon_terminated: _,
            span,
        } => {
            let value = lower_expr(builder, value)?;
            builder
                .call_node("Expr", vec![value], span)
                .map_err(py_err_to_lower)
        }
    }
}

pub(crate) fn lower_block(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    lower_block_with_tail(builder, block, TailBehavior::None, span)
}

pub(crate) fn lower_block_with_auto_print(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    auto_print: bool,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let tail = if auto_print {
        TailBehavior::AutoPrint
    } else {
        TailBehavior::None
    };
    lower_block_with_tail(builder, block, tail, span)
}

pub(crate) fn lower_block_with_implicit_return(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    lower_block_with_tail(builder, block, TailBehavior::ImplicitReturn, span)
}

fn lower_block_with_tail(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for (idx, stmt) in block.iter().enumerate() {
        let is_last = idx == block.len().saturating_sub(1);
        if is_last {
            match (tail, stmt) {
                (
                    TailBehavior::AutoPrint,
                    Stmt::Expr {
                        value,
                        semicolon_terminated,
                        span,
                    },
                ) if !semicolon_terminated => {
                    let expr = lower_expr(builder, value)?;
                    stmts.extend(build_auto_print_block(builder, expr, span)?);
                    continue;
                }
                (
                    TailBehavior::ImplicitReturn,
                    Stmt::Expr {
                        value,
                        semicolon_terminated,
                        span,
                    },
                ) if !semicolon_terminated => {
                    let expr = lower_expr(builder, value)?;
                    let return_stmt = builder
                        .call_node("Return", vec![expr], span)
                        .map_err(py_err_to_lower)?;
                    stmts.push(return_stmt);
                    continue;
                }
                _ => {}
            }
        }
        match stmt {
            Stmt::If {
                cond,
                body,
                elifs,
                else_body,
                span,
            } => {
                stmts.extend(lower_if_chain(builder, cond, body, elifs, else_body, span)?);
            }
            Stmt::While {
                cond,
                body,
                else_body,
                span,
            } => {
                stmts.extend(lower_while_stmt(builder, cond, body, else_body, span)?);
            }
            _ => stmts.push(lower_stmt(builder, stmt)?),
        }
    }
    if stmts.is_empty() {
        stmts.push(
            builder
                .call_node("Pass", Vec::new(), span)
                .map_err(py_err_to_lower)?,
        );
    }
    Ok(stmts)
}

fn lower_if_expr(
    builder: &AstBuilder<'_>,
    cond: &Expr,
    body: &[Stmt],
    elifs: &[(Condition, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let test = lower_expr(builder, cond)?;
    let body = lower_block(builder, body, span)?;
    let orelse = if let Some((elif_cond, elif_body)) = elifs.first() {
        lower_if_chain(builder, elif_cond, elif_body, &elifs[1..], else_body, span)?
    } else if let Some(else_body) = else_body {
        lower_block(builder, else_body, span)?
    } else {
        Vec::new()
    };
    builder
        .call_node(
            "If",
            vec![
                test,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_if_chain(
    builder: &AstBuilder<'_>,
    cond: &Condition,
    body: &[Stmt],
    elifs: &[(Condition, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    match cond {
        Condition::Expr(cond) => Ok(vec![lower_if_expr(
            builder,
            cond.as_ref(),
            body,
            elifs,
            else_body,
            span,
        )?]),
        Condition::Let {
            target,
            value,
            guard,
            span: cond_span,
        } => lower_if_let(
            builder,
            target.as_ref(),
            value.as_ref(),
            guard.as_ref().map(|expr| expr.as_ref()),
            body,
            elifs,
            else_body,
            cond_span,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_if_let(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    value: &Expr,
    guard: Option<&Expr>,
    body: &[Stmt],
    elifs: &[(Condition, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    let value_expr = lower_expr(builder, value)?;
    stmts.push(assign_name(builder, SNAIL_LET_VALUE, value_expr, span)?);

    let try_node = build_destructure_try(builder, target, span)?;
    stmts.push(try_node);

    let test = build_let_guard_test(builder, guard, span)?;
    let body = lower_block(builder, body, span)?;
    let orelse = if let Some((elif_cond, elif_body)) = elifs.first() {
        lower_if_chain(builder, elif_cond, elif_body, &elifs[1..], else_body, span)?
    } else if let Some(else_body) = else_body {
        lower_block(builder, else_body, span)?
    } else {
        Vec::new()
    };
    let if_node = builder
        .call_node(
            "If",
            vec![
                test,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(if_node);
    Ok(stmts)
}

fn lower_while_stmt(
    builder: &AstBuilder<'_>,
    cond: &Condition,
    body: &[Stmt],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    match cond {
        Condition::Expr(cond) => {
            let test = lower_expr(builder, cond.as_ref())?;
            let body = lower_block(builder, body, span)?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items, span))
                .transpose()?
                .unwrap_or_default();
            let while_node = builder
                .call_node(
                    "While",
                    vec![
                        test,
                        PyList::new_bound(builder.py(), body).into_py(builder.py()),
                        PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            Ok(vec![while_node])
        }
        Condition::Let {
            target,
            value,
            guard,
            span: cond_span,
        } => lower_while_let(
            builder,
            target.as_ref(),
            value.as_ref(),
            guard.as_ref().map(|expr| expr.as_ref()),
            body,
            else_body,
            cond_span,
        ),
    }
}

fn lower_while_let(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    value: &Expr,
    guard: Option<&Expr>,
    body: &[Stmt],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    stmts.push(assign_name(
        builder,
        SNAIL_LET_KEEP,
        bool_constant(builder, true, span)?,
        span,
    )?);

    let mut loop_body = Vec::new();
    let value_expr = lower_expr(builder, value)?;
    loop_body.push(assign_name(builder, SNAIL_LET_VALUE, value_expr, span)?);
    loop_body.push(assign_name(
        builder,
        SNAIL_LET_OK,
        bool_constant(builder, false, span)?,
        span,
    )?);

    let try_node = build_destructure_try(builder, target, span)?;
    loop_body.push(try_node);

    let test = build_let_guard_test(builder, guard, span)?;
    let body = lower_block(builder, body, span)?;
    let keep_false = assign_name(
        builder,
        SNAIL_LET_KEEP,
        bool_constant(builder, false, span)?,
        span,
    )?;
    let if_node = builder
        .call_node(
            "If",
            vec![
                test,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![keep_false]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    loop_body.push(if_node);

    let test_expr = name_expr(
        builder,
        SNAIL_LET_KEEP,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let orelse = else_body
        .as_ref()
        .map(|items| lower_block(builder, items, span))
        .transpose()?
        .unwrap_or_default();
    let while_node = builder
        .call_node(
            "While",
            vec![
                test_expr,
                PyList::new_bound(builder.py(), loop_body).into_py(builder.py()),
                PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    stmts.push(while_node);
    Ok(stmts)
}

fn build_let_guard_test(
    builder: &AstBuilder<'_>,
    guard: Option<&Expr>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let ok_expr = Expr::Name {
        name: SNAIL_LET_OK.to_string(),
        span: span.clone(),
    };
    let test_expr = if let Some(guard) = guard {
        Expr::Binary {
            left: Box::new(ok_expr),
            op: BinaryOp::And,
            right: Box::new(guard.clone()),
            span: span.clone(),
        }
    } else {
        ok_expr
    };
    lower_expr(builder, &test_expr)
}

fn build_destructure_try(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let target = lower_assign_target(builder, target)?;
    let value_expr = name_expr(
        builder,
        SNAIL_LET_VALUE,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let assign = builder
        .call_node(
            "Assign",
            vec![
                PyList::new_bound(builder.py(), vec![target]).into_py(builder.py()),
                value_expr,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let ok_true = assign_name(
        builder,
        SNAIL_LET_OK,
        bool_constant(builder, true, span)?,
        span,
    )?;
    let ok_false = assign_name(
        builder,
        SNAIL_LET_OK,
        bool_constant(builder, false, span)?,
        span,
    )?;
    let handler = build_destructure_handler(builder, ok_false, span)?;
    builder
        .call_node(
            "Try",
            vec![
                PyList::new_bound(builder.py(), vec![assign]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![handler]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![ok_true]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn build_destructure_handler(
    builder: &AstBuilder<'_>,
    ok_false: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let exc_type = build_destructure_exception_tuple(builder, span)?;
    builder
        .call_node(
            "ExceptHandler",
            vec![
                exc_type,
                builder.py().None().into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![ok_false]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn build_destructure_exception_tuple(
    builder: &AstBuilder<'_>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let type_error = name_expr(
        builder,
        "TypeError",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let value_error = name_expr(
        builder,
        "ValueError",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Tuple",
            vec![
                PyList::new_bound(builder.py(), vec![type_error, value_error])
                    .into_py(builder.py()),
                builder.load_ctx().map_err(py_err_to_lower)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn bool_constant(
    builder: &AstBuilder<'_>,
    value: bool,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    builder
        .call_node("Constant", vec![value.into_py(builder.py())], span)
        .map_err(py_err_to_lower)
}

pub(crate) fn lower_parameters(
    builder: &AstBuilder<'_>,
    params: &[Parameter],
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let mut args = Vec::new();
    let mut defaults = Vec::new();
    let mut vararg = None;
    let mut kwarg = None;

    for param in params {
        match param {
            Parameter::Regular {
                name,
                default,
                span: _,
            } => {
                let arg = builder
                    .call_node_no_loc(
                        "arg",
                        vec![
                            name.to_string().into_py(builder.py()),
                            builder.py().None().into_py(builder.py()),
                        ],
                    )
                    .map_err(py_err_to_lower)?;
                args.push(arg);
                if let Some(default_expr) = default {
                    defaults.push(lower_expr_with_exception(
                        builder,
                        default_expr,
                        exception_name,
                    )?);
                }
            }
            Parameter::VarArgs { name, .. } => {
                let arg = builder
                    .call_node_no_loc(
                        "arg",
                        vec![
                            name.to_string().into_py(builder.py()),
                            builder.py().None().into_py(builder.py()),
                        ],
                    )
                    .map_err(py_err_to_lower)?;
                vararg = Some(arg);
            }
            Parameter::KwArgs { name, .. } => {
                let arg = builder
                    .call_node_no_loc(
                        "arg",
                        vec![
                            name.to_string().into_py(builder.py()),
                            builder.py().None().into_py(builder.py()),
                        ],
                    )
                    .map_err(py_err_to_lower)?;
                kwarg = Some(arg);
            }
        }
    }

    builder
        .call_node_no_loc(
            "arguments",
            vec![
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::new_bound(builder.py(), args).into_py(builder.py()),
                vararg.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                kwarg.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                PyList::new_bound(builder.py(), defaults).into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)
}

fn lower_except_handler(
    builder: &AstBuilder<'_>,
    handler: &ExceptHandler,
) -> Result<PyObject, LowerError> {
    let type_name = handler
        .type_name
        .as_ref()
        .map(|expr| lower_expr(builder, expr))
        .transpose()?
        .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
    let name = handler
        .name
        .as_ref()
        .map(|name| name.to_string().into_py(builder.py()))
        .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
    let body = lower_block(builder, &handler.body, &handler.span)?;
    builder
        .call_node(
            "ExceptHandler",
            vec![
                type_name,
                name,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
            ],
            &handler.span,
        )
        .map_err(py_err_to_lower)
}

fn lower_with_item(builder: &AstBuilder<'_>, item: &WithItem) -> Result<PyObject, LowerError> {
    let context_expr = lower_expr(builder, &item.context)?;
    let optional_vars = item
        .target
        .as_ref()
        .map(|target| lower_assign_target(builder, target))
        .transpose()?
        .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
    builder
        .call_node_no_loc("withitem", vec![context_expr, optional_vars])
        .map_err(py_err_to_lower)
}

fn lower_import_name(builder: &AstBuilder<'_>, item: &ImportItem) -> Result<PyObject, LowerError> {
    let name = item.name.join(".");
    let asname = item
        .alias
        .as_ref()
        .map(|alias| alias.to_string().into_py(builder.py()))
        .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
    builder
        .call_node_no_loc("alias", vec![name.into_py(builder.py()), asname])
        .map_err(py_err_to_lower)
}

fn lower_import_star(builder: &AstBuilder<'_>) -> Result<PyObject, LowerError> {
    let asname = builder.py().None().into_py(builder.py());
    builder
        .call_node_no_loc("alias", vec!["*".into_py(builder.py()), asname])
        .map_err(py_err_to_lower)
}

fn build_auto_print_block(
    builder: &AstBuilder<'_>,
    expr: PyObject,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();

    let assign = assign_name(builder, "__snail_last_result", expr, span)?;
    stmts.push(assign);

    let last_result = name_expr(
        builder,
        "__snail_last_result",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;

    let is_string = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    "isinstance",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(
                    builder.py(),
                    vec![
                        last_result.clone_ref(builder.py()),
                        name_expr(
                            builder,
                            "str",
                            span,
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        )?,
                    ],
                )
                .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let print_call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    "print",
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(builder.py(), vec![last_result.clone_ref(builder.py())])
                    .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let print_stmt = builder
        .call_node("Expr", vec![print_call], span)
        .map_err(py_err_to_lower)?;

    let is_not_none = builder
        .call_node(
            "Compare",
            vec![
                last_result.clone_ref(builder.py()),
                PyList::new_bound(
                    builder.py(),
                    vec![lower_compare_op(builder, CompareOp::IsNot)?],
                )
                .into_py(builder.py()),
                PyList::new_bound(
                    builder.py(),
                    vec![
                        builder
                            .call_node(
                                "Constant",
                                vec![builder.py().None().into_py(builder.py())],
                                span,
                            )
                            .map_err(py_err_to_lower)?,
                    ],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let import_pprint = builder
        .call_node(
            "Import",
            vec![
                PyList::new_bound(
                    builder.py(),
                    vec![
                        builder
                            .call_node_no_loc(
                                "alias",
                                vec![
                                    "pprint".to_string().into_py(builder.py()),
                                    builder.py().None().into_py(builder.py()),
                                ],
                            )
                            .map_err(py_err_to_lower)?,
                    ],
                )
                .into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let pprint_call = builder
        .call_node(
            "Call",
            vec![
                builder
                    .call_node(
                        "Attribute",
                        vec![
                            name_expr(
                                builder,
                                "pprint",
                                span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?,
                            "pprint".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?,
                PyList::new_bound(builder.py(), vec![last_result]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let pprint_stmt = builder
        .call_node("Expr", vec![pprint_call], span)
        .map_err(py_err_to_lower)?;

    let pprint_if = builder
        .call_node(
            "If",
            vec![
                is_not_none,
                PyList::new_bound(builder.py(), vec![import_pprint, pprint_stmt])
                    .into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    let top_if = builder
        .call_node(
            "If",
            vec![
                is_string,
                PyList::new_bound(builder.py(), vec![print_stmt]).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![pprint_if]).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;

    stmts.push(top_if);
    Ok(stmts)
}
