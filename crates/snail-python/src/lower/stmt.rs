use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::{SNAIL_LET_KEEP, SNAIL_LET_OK, SNAIL_LET_VALUE};
use super::expr::{
    lower_assign_target, lower_delete_target, lower_expr, lower_expr_as_stmt,
    lower_expr_with_exception, lower_tail_expr,
};
use super::helpers::{
    assign_name, bool_constant, build_destructure_try, build_let_guard_test, name_expr,
};
use super::py_ast::{AstBuilder, py_err_to_lower};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TailBehavior {
    None,
    AutoPrint,
    CaptureOnly,
    ImplicitReturn,
}

pub(crate) fn lower_stmt(builder: &AstBuilder<'_>, stmt: &Stmt) -> Result<PyObject, LowerError> {
    match stmt {
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
        Stmt::PatternAction { .. } => Err(LowerError::new(
            "pattern/action should be lowered via lower_awk_body",
        )),
        Stmt::SegmentBreak { .. } => Err(LowerError::new(
            "segment break should be handled by lower_block_with_tail",
        )),
    }
}

pub(crate) fn lower_block(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    lower_block_with_tail(builder, block, TailBehavior::None, span)
}

pub(crate) fn lower_block_auto(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    auto_print: bool,
    capture_last: bool,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let tail = if auto_print {
        TailBehavior::AutoPrint
    } else if capture_last {
        TailBehavior::CaptureOnly
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

pub(crate) fn lower_block_with_tail(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for (idx, stmt) in block.iter().enumerate() {
        // Skip SegmentBreak nodes — they are only markers for tail behavior
        if matches!(stmt, Stmt::SegmentBreak { .. }) {
            continue;
        }
        let is_last = idx == block.len().saturating_sub(1);
        let next_is_break = matches!(block.get(idx + 1), Some(Stmt::SegmentBreak { .. }));
        let at_tail = (is_last || next_is_break) && tail != TailBehavior::None;

        // Tail position: delegate tail behavior to the appropriate module
        if at_tail
            && let Stmt::Expr {
                value,
                semicolon_terminated,
                span,
            } = stmt
            && !semicolon_terminated
        {
            stmts.extend(lower_tail_expr(builder, value, tail, span)?);
            continue;
        }

        // General lowering
        stmts.extend(lower_stmt_to_stmts(builder, stmt)?);
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

/// Lower a statement to one or more Python AST statements. Dispatches to
/// expression lowering which may produce multiple Python statements from a
/// single Snail statement (compound expressions like if/while/for/etc.).
fn lower_stmt_to_stmts(builder: &AstBuilder<'_>, stmt: &Stmt) -> Result<Vec<PyObject>, LowerError> {
    match stmt {
        Stmt::Expr { value, span, .. } => lower_expr_as_stmt(builder, value, span),
        _ => Ok(vec![lower_stmt(builder, stmt)?]),
    }
}

pub(crate) fn lower_while_stmt(
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

pub(crate) fn lower_except_handler(
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

pub(crate) fn lower_with_item(
    builder: &AstBuilder<'_>,
    item: &WithItem,
) -> Result<PyObject, LowerError> {
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
