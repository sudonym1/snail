use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use crate::expr::{lower_assign_target, lower_delete_target, lower_expr};
use crate::helpers::{assign_name, name_expr};
use crate::operators::lower_compare_op;
use crate::py_ast::{AstBuilder, py_err_to_lower};

pub(crate) fn lower_stmt(builder: &AstBuilder<'_>, stmt: &Stmt) -> Result<PyObject, LowerError> {
    match stmt {
        Stmt::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => lower_if(builder, cond, body, elifs, else_body, span),
        Stmt::While {
            cond,
            body,
            else_body,
            span,
        } => {
            let test = lower_expr(builder, cond)?;
            let body = lower_block(builder, body)?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items))
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
            let body = lower_block(builder, body)?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items))
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
            let args = lower_parameters(builder, params)?;
            let body = lower_block(builder, body)?;
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
            let body = lower_block(builder, body)?;
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
            let body = lower_block(builder, body)?;
            let handlers = handlers
                .iter()
                .map(|handler| lower_except_handler(builder, handler))
                .collect::<Result<Vec<_>, _>>()?;
            let orelse = else_body
                .as_ref()
                .map(|items| lower_block(builder, items))
                .transpose()?
                .unwrap_or_default();
            let finalbody = finally_body
                .as_ref()
                .map(|items| lower_block(builder, items))
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
            let body = lower_block(builder, body)?;
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
            module,
            items,
            span,
        } => {
            let filtered_items: Vec<&ImportItem> = if module.len() == 1 && module[0] == "__future__"
            {
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
            let names = filtered_items
                .iter()
                .map(|item| lower_import_name(builder, item))
                .collect::<Result<Vec<_>, _>>()?;
            let module_name = if module.is_empty() {
                builder.py().None().into_py(builder.py())
            } else {
                module.join(".").into_py(builder.py())
            };
            builder
                .call_node(
                    "ImportFrom",
                    vec![
                        module_name,
                        PyList::new_bound(builder.py(), names).into_py(builder.py()),
                        0u8.into_py(builder.py()),
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
) -> Result<Vec<PyObject>, LowerError> {
    lower_block_with_auto_print(builder, block, false)
}

pub(crate) fn lower_block_with_auto_print(
    builder: &AstBuilder<'_>,
    block: &[Stmt],
    auto_print: bool,
) -> Result<Vec<PyObject>, LowerError> {
    let mut stmts = Vec::new();
    for (idx, stmt) in block.iter().enumerate() {
        let is_last = idx == block.len().saturating_sub(1);
        if auto_print
            && is_last
            && let Stmt::Expr {
                value,
                semicolon_terminated,
                span,
            } = stmt
            && !semicolon_terminated
        {
            let expr = lower_expr(builder, value)?;
            stmts.extend(build_auto_print_block(builder, expr, span)?);
            continue;
        }
        stmts.push(lower_stmt(builder, stmt)?);
    }
    Ok(stmts)
}

fn lower_if(
    builder: &AstBuilder<'_>,
    cond: &Expr,
    body: &[Stmt],
    elifs: &[(Expr, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let test = lower_expr(builder, cond)?;
    let body = lower_block(builder, body)?;
    let orelse = if let Some((elif_cond, elif_body)) = elifs.first() {
        vec![lower_if(
            builder,
            elif_cond,
            elif_body,
            &elifs[1..],
            else_body,
            span,
        )?]
    } else if let Some(else_body) = else_body {
        lower_block(builder, else_body)?
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

fn lower_parameters(
    builder: &AstBuilder<'_>,
    params: &[Parameter],
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
                    defaults.push(lower_expr(builder, default_expr)?);
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
    let body = lower_block(builder, &handler.body)?;
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
