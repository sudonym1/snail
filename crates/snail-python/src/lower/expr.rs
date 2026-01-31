use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::*;
use snail_error::LowerError;

use super::constants::*;
use super::helpers::{byte_string_expr, name_expr, number_expr, regex_pattern_expr, string_expr};
use super::operators::{
    aug_op_to_string, lower_aug_assign_op, lower_binary_op, lower_bool_op, lower_compare_op,
    lower_unary_op,
};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::lower_parameters;

pub(crate) fn lower_expr(builder: &AstBuilder<'_>, expr: &Expr) -> Result<PyObject, LowerError> {
    lower_expr_with_exception(builder, expr, None)
}

pub(crate) fn lower_assign_target(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
) -> Result<PyObject, LowerError> {
    let store_ctx = builder.store_ctx().map_err(py_err_to_lower)?;
    match target {
        AssignTarget::Name { name, span } => name_expr(builder, name, span, store_ctx),
        AssignTarget::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, None)?;
            builder
                .call_node(
                    "Attribute",
                    vec![value, attr.to_string().into_py(builder.py()), store_ctx],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::Index { value, index, span } => {
            let value_expr = lower_expr_with_exception(builder, value, None)?;
            let index_expr = lower_expr_with_exception(builder, index, None)?;
            subscript_expr(builder, value_expr, index_expr, store_ctx, span)
        }
        AssignTarget::Starred { target, span } => {
            let value = lower_assign_target(builder, target)?;
            builder
                .call_node("Starred", vec![value, store_ctx.clone()], span)
                .map_err(py_err_to_lower)
        }
        AssignTarget::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_assign_target(builder, element)?);
            }
            builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        store_ctx,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_assign_target(builder, element)?);
            }
            builder
                .call_node(
                    "List",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        store_ctx,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

pub(crate) fn lower_delete_target(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
) -> Result<PyObject, LowerError> {
    let del_ctx = builder.del_ctx().map_err(py_err_to_lower)?;
    match target {
        AssignTarget::Name { name, span } => name_expr(builder, name, span, del_ctx),
        AssignTarget::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, None)?;
            builder
                .call_node(
                    "Attribute",
                    vec![value, attr.to_string().into_py(builder.py()), del_ctx],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::Index { value, index, span } => {
            let value_expr = lower_expr_with_exception(builder, value, None)?;
            let index_expr = lower_expr_with_exception(builder, index, None)?;
            subscript_expr(builder, value_expr, index_expr, del_ctx, span)
        }
        AssignTarget::Starred { .. } => Err(LowerError::new(
            "starred targets are not valid in del statements",
        )),
        AssignTarget::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_delete_target(builder, element)?);
            }
            builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        del_ctx,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        AssignTarget::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_delete_target(builder, element)?);
            }
            builder
                .call_node(
                    "List",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        del_ctx,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn subscript_expr(
    builder: &AstBuilder<'_>,
    value: PyObject,
    index: PyObject,
    ctx: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let slice = builder.wrap_index(index, span).map_err(py_err_to_lower)?;
    builder
        .call_node("Subscript", vec![value, slice, ctx], span)
        .map_err(py_err_to_lower)
}

pub(crate) fn lower_regex_match(
    builder: &AstBuilder<'_>,
    value: &Expr,
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let func = name_expr(
        builder,
        SNAIL_REGEX_SEARCH,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let value_expr = lower_expr_with_exception(builder, value, exception_name)?;
    let pattern_expr = lower_regex_pattern_expr(builder, pattern, span, exception_name)?;
    let args = vec![value_expr, pattern_expr];
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), args).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_regex_pattern_expr(
    builder: &AstBuilder<'_>,
    pattern: &RegexPattern,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match pattern {
        RegexPattern::Literal(text) => regex_pattern_expr(builder, text, span),
        RegexPattern::Interpolated(parts) => {
            let values = lower_fstring_parts(builder, parts, exception_name)?;
            builder
                .call_node(
                    "JoinedStr",
                    vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn lower_contains_call(
    builder: &AstBuilder<'_>,
    helper_name: &str,
    left: PyObject,
    right: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let func = name_expr(
        builder,
        helper_name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let args = vec![left, right];
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), args).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_compare_pair(
    builder: &AstBuilder<'_>,
    left: PyObject,
    op: CompareOp,
    right: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    match op {
        CompareOp::In => lower_contains_call(builder, SNAIL_CONTAINS_HELPER, left, right, span),
        CompareOp::NotIn => {
            lower_contains_call(builder, SNAIL_CONTAINS_NOT_HELPER, left, right, span)
        }
        _ => {
            let op = lower_compare_op(builder, op)?;
            builder
                .call_node(
                    "Compare",
                    vec![
                        left,
                        PyList::new_bound(builder.py(), vec![op]).into_py(builder.py()),
                        PyList::new_bound(builder.py(), vec![right]).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn named_expr(
    builder: &AstBuilder<'_>,
    name: &str,
    value: PyObject,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let target = name_expr(
        builder,
        name,
        span,
        builder.store_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node("NamedExpr", vec![target, value], span)
        .map_err(py_err_to_lower)
}

fn lower_compare_chain(
    builder: &AstBuilder<'_>,
    left: &Expr,
    ops: &[CompareOp],
    comparators: &[Expr],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    if ops.len() != comparators.len() {
        return Err(LowerError::new(
            "comparison ops must match comparator count",
        ));
    }
    if ops.is_empty() {
        return Err(LowerError::new("comparison missing operator"));
    }

    let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
    let right_expr = lower_expr_with_exception(builder, &comparators[0], exception_name)?;
    let left_named = named_expr(builder, SNAIL_COMPARE_LEFT, left_expr, span)?;
    let right_named = named_expr(builder, SNAIL_COMPARE_RIGHT, right_expr, span)?;
    let mut comparisons = Vec::with_capacity(ops.len());
    comparisons.push(lower_compare_pair(
        builder,
        left_named,
        ops[0],
        right_named,
        span,
    )?);

    for (index, op) in ops.iter().enumerate().skip(1) {
        let prev_right = name_expr(
            builder,
            SNAIL_COMPARE_RIGHT,
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        )?;
        let left_named = named_expr(builder, SNAIL_COMPARE_LEFT, prev_right, span)?;
        let right_expr = lower_expr_with_exception(builder, &comparators[index], exception_name)?;
        let right_named = named_expr(builder, SNAIL_COMPARE_RIGHT, right_expr, span)?;
        comparisons.push(lower_compare_pair(
            builder,
            left_named,
            *op,
            right_named,
            span,
        )?);
    }

    if comparisons.len() == 1 {
        return Ok(comparisons.remove(0));
    }

    let op = lower_bool_op(builder, BinaryOp::And)?;
    builder
        .call_node(
            "BoolOp",
            vec![
                op,
                PyList::new_bound(builder.py(), comparisons).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_fstring_parts(
    builder: &AstBuilder<'_>,
    parts: &[FStringPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyObject>, LowerError> {
    let mut lowered = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => {
                let const_node = builder
                    .call_node(
                        "Constant",
                        vec![text.clone().into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered.push(const_node);
            }
            FStringPart::Expr(expr) => {
                let value = lower_expr_with_exception(builder, &expr.expr, exception_name)?;
                let conversion = fstring_conversion(expr.conversion).into_py(builder.py());
                let format_spec = match &expr.format_spec {
                    Some(parts) => {
                        let values = lower_fstring_parts(builder, parts, exception_name)?;
                        builder
                            .call_node(
                                "JoinedStr",
                                vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                                &dummy_span(),
                            )
                            .map_err(py_err_to_lower)?
                    }
                    None => builder.py().None().into_py(builder.py()),
                };
                let formatted = builder
                    .call_node(
                        "FormattedValue",
                        vec![value, conversion, format_spec],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered.push(formatted);
            }
        }
    }
    Ok(lowered)
}

fn fstring_conversion(conversion: FStringConversion) -> i32 {
    match conversion {
        FStringConversion::None => -1,
        FStringConversion::Str => 's' as i32,
        FStringConversion::Repr => 'r' as i32,
        FStringConversion::Ascii => 'a' as i32,
    }
}

pub(crate) fn lower_expr_with_exception(
    builder: &AstBuilder<'_>,
    expr: &Expr,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match expr {
        Expr::Name { name, span } => {
            if name == "$e" {
                if let Some(exception_name) = exception_name {
                    return name_expr(
                        builder,
                        exception_name,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    );
                }
                return Err(LowerError::new(
                    "`$e` is only available in compact exception fallbacks",
                ));
            }
            if let Some(py_name) = injected_py_name(name) {
                return name_expr(
                    builder,
                    py_name,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                );
            }
            name_expr(
                builder,
                name,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )
        }
        Expr::Placeholder { span } => name_expr(
            builder,
            "_",
            span,
            builder.load_ctx().map_err(py_err_to_lower)?,
        ),
        Expr::FieldIndex { index, span } => {
            if index == "0" {
                return name_expr(
                    builder,
                    SNAIL_AWK_LINE_PYVAR,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                );
            }
            let field_index = index
                .parse::<i32>()
                .map_err(|_| LowerError::new(format!("Invalid field index: ${}", index)))?;
            let python_index = field_index - 1;
            let value = name_expr(
                builder,
                SNAIL_AWK_FIELDS_PYVAR,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let index_expr = number_expr(builder, &python_index.to_string(), span)?;
            subscript_expr(
                builder,
                value,
                index_expr,
                builder.load_ctx().map_err(py_err_to_lower)?,
                span,
            )
        }
        Expr::Number { value, span } => number_expr(builder, value, span),
        Expr::String {
            value,
            raw,
            bytes,
            delimiter,
            span,
        } => {
            if *bytes {
                byte_string_expr(builder, value, *raw, *delimiter, span)
            } else {
                string_expr(builder, value, *raw, *delimiter, span)
            }
        }
        Expr::FString { parts, bytes, span } => {
            let values = lower_fstring_parts(builder, parts, exception_name)?;
            let joined = builder
                .call_node(
                    "JoinedStr",
                    vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)?;

            if *bytes {
                // Wrap in .encode() call: f"...".encode()
                let encode_attr = builder
                    .call_node(
                        "Attribute",
                        vec![
                            joined,
                            "encode".to_string().into_py(builder.py()),
                            builder.load_ctx().map_err(py_err_to_lower)?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                builder
                    .call_node(
                        "Call",
                        vec![
                            encode_attr,
                            PyList::empty_bound(builder.py()).into_py(builder.py()),
                            PyList::empty_bound(builder.py()).into_py(builder.py()),
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)
            } else {
                Ok(joined)
            }
        }
        Expr::Bool { value, span } => builder
            .call_node("Constant", vec![value.into_py(builder.py())], span)
            .map_err(py_err_to_lower),
        Expr::None { span } => builder
            .call_node(
                "Constant",
                vec![builder.py().None().into_py(builder.py())],
                span,
            )
            .map_err(py_err_to_lower),
        Expr::Unary { op, expr, span } => {
            let operand = lower_expr_with_exception(builder, expr, exception_name)?;
            let op = lower_unary_op(builder, *op)?;
            builder
                .call_node("UnaryOp", vec![op, operand], span)
                .map_err(py_err_to_lower)
        }
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => {
            if *op == BinaryOp::Pipeline {
                // Pipeline: a | b lowers to b(a) with placeholder support for call args.
                match right.as_ref() {
                    Expr::Call {
                        func,
                        args,
                        span: call_span,
                    } => {
                        let placeholder_info = placeholder_info_in_arguments(args);
                        if placeholder_info.count > 1 {
                            let span = placeholder_info
                                .first_span
                                .unwrap_or_else(|| call_span.clone());
                            return Err(LowerError::multiple_placeholders(span));
                        }
                        if placeholder_info.count == 1 {
                            let replaced_args = substitute_placeholder_in_arguments(args, left);
                            let func_expr =
                                lower_expr_with_exception(builder, func, exception_name)?;
                            let (args, keywords) =
                                lower_call_arguments(builder, &replaced_args, exception_name)?;
                            let mut helper_args = Vec::with_capacity(args.len() + 1);
                            helper_args.push(func_expr);
                            helper_args.extend(args);
                            let helper = name_expr(
                                builder,
                                SNAIL_PARTIAL_HELPER,
                                call_span,
                                builder.load_ctx().map_err(py_err_to_lower)?,
                            )?;
                            let partial_call = builder
                                .call_node(
                                    "Call",
                                    vec![
                                        helper,
                                        PyList::new_bound(builder.py(), helper_args)
                                            .into_py(builder.py()),
                                        PyList::new_bound(builder.py(), keywords)
                                            .into_py(builder.py()),
                                    ],
                                    call_span,
                                )
                                .map_err(py_err_to_lower)?;
                            return builder
                                .call_node(
                                    "Call",
                                    vec![
                                        partial_call,
                                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                                    ],
                                    span,
                                )
                                .map_err(py_err_to_lower);
                        }

                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj = lower_expr_with_exception(builder, right, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                    Expr::Subprocess {
                        kind,
                        parts,
                        span: s_span,
                    } => {
                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj =
                            lower_subprocess_object(builder, kind, parts, s_span, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                    _ => {
                        let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                        let right_obj = lower_expr_with_exception(builder, right, exception_name)?;
                        let args = vec![left_expr];
                        builder
                            .call_node(
                                "Call",
                                vec![
                                    right_obj,
                                    PyList::new_bound(builder.py(), args).into_py(builder.py()),
                                    PyList::empty_bound(builder.py()).into_py(builder.py()),
                                ],
                                span,
                            )
                            .map_err(py_err_to_lower)
                    }
                }
            } else if *op == BinaryOp::Or || *op == BinaryOp::And {
                let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                let right_expr = lower_expr_with_exception(builder, right, exception_name)?;
                let op = lower_bool_op(builder, *op)?;
                builder
                    .call_node(
                        "BoolOp",
                        vec![
                            op,
                            PyList::new_bound(builder.py(), vec![left_expr, right_expr])
                                .into_py(builder.py()),
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)
            } else {
                let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                let right_expr = lower_expr_with_exception(builder, right, exception_name)?;
                let op = lower_binary_op(builder, *op)?;
                builder
                    .call_node("BinOp", vec![left_expr, op, right_expr], span)
                    .map_err(py_err_to_lower)
            }
        }
        Expr::AugAssign {
            target,
            op,
            value,
            span,
        } => lower_aug_assign(builder, target, *op, value, span, exception_name),
        Expr::PrefixIncr { op, target, span } => {
            lower_incr(builder, target, *op, true, span, exception_name)
        }
        Expr::PostfixIncr { op, target, span } => {
            lower_incr(builder, target, *op, false, span, exception_name)
        }
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => {
            if ops.len() == 1 {
                let left_expr = lower_expr_with_exception(builder, left, exception_name)?;
                let right_expr =
                    lower_expr_with_exception(builder, &comparators[0], exception_name)?;
                lower_compare_pair(builder, left_expr, ops[0], right_expr, span)
            } else {
                lower_compare_chain(builder, left, ops, comparators, span, exception_name)
            }
        }
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => {
            let test = lower_expr_with_exception(builder, test, exception_name)?;
            let body = lower_expr_with_exception(builder, body, exception_name)?;
            let orelse = lower_expr_with_exception(builder, orelse, exception_name)?;
            builder
                .call_node("IfExp", vec![test, body, orelse], span)
                .map_err(py_err_to_lower)
        }
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => {
            let try_lambda = builder
                .call_node(
                    "Lambda",
                    vec![
                        empty_lambda_args(builder)?,
                        lower_expr_with_exception(builder, expr, exception_name)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let mut args = vec![try_lambda];
            if let Some(fallback_expr) = fallback {
                let fallback_lambda = builder
                    .call_node(
                        "Lambda",
                        vec![
                            lambda_args_with_param(builder, SNAIL_EXCEPTION_VAR)?,
                            lower_expr_with_exception(
                                builder,
                                fallback_expr,
                                Some(SNAIL_EXCEPTION_VAR),
                            )?,
                        ],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                args.push(fallback_lambda);
            }
            let func = name_expr(
                builder,
                SNAIL_TRY_HELPER,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), args).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Yield { value, span } => {
            let value = value
                .as_deref()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .transpose()?
                .unwrap_or_else(|| builder.py().None().into_py(builder.py()));
            builder
                .call_node("Yield", vec![value], span)
                .map_err(py_err_to_lower)
        }
        Expr::YieldFrom { expr, span } => {
            let value = lower_expr_with_exception(builder, expr, exception_name)?;
            builder
                .call_node("YieldFrom", vec![value], span)
                .map_err(py_err_to_lower)
        }
        Expr::Lambda { params, body, span } => {
            lower_lambda_expr(builder, params, body, span, exception_name)
        }
        Expr::Compound { expressions, span } => {
            let mut lowered = Vec::new();
            for expr in expressions {
                lowered.push(lower_expr_with_exception(builder, expr, exception_name)?);
            }
            let tuple_expr = builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let index_expr = builder
                .call_node(
                    "UnaryOp",
                    vec![
                        lower_unary_op(builder, UnaryOp::Minus)?,
                        number_expr(builder, "1", span)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            subscript_expr(
                builder,
                tuple_expr,
                index_expr,
                builder.load_ctx().map_err(py_err_to_lower)?,
                span,
            )
        }
        Expr::Regex { pattern, span } => {
            let func = name_expr(
                builder,
                SNAIL_REGEX_COMPILE,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let arg = lower_regex_pattern_expr(builder, pattern, span, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => lower_regex_match(builder, value, pattern, span, exception_name),
        Expr::Subprocess { kind, parts, span } => {
            // Standalone subprocess: $(cmd) lowers to SubprocessCapture(cmd)()
            let subprocess_obj =
                lower_subprocess_object(builder, kind, parts, span, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        subprocess_obj,
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::StructuredAccessor { query, span } => {
            // $[query] lowers to __snail_jmespath_query(query) which returns a callable
            let escaped_query = escape_for_python_string(query);
            let func = name_expr(
                builder,
                SNAIL_JMESPATH_QUERY,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let arg = string_expr(
                builder,
                &escaped_query,
                false,
                StringDelimiter::Double,
                span,
            )?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                        PyList::empty_bound(builder.py()).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Call { func, args, span } => {
            let func = lower_expr_with_exception(builder, func, exception_name)?;
            let (args, keywords) = lower_call_arguments(builder, args, exception_name)?;
            builder
                .call_node(
                    "Call",
                    vec![
                        func,
                        PyList::new_bound(builder.py(), args).into_py(builder.py()),
                        PyList::new_bound(builder.py(), keywords).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Attribute { value, attr, span } => {
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            if attr.chars().all(|ch| ch.is_ascii_digit()) {
                let index = attr
                    .parse::<i32>()
                    .map_err(|_| LowerError::new(format!("Invalid match group index: .{attr}")))?;
                let index_expr = number_expr(builder, &index.to_string(), span)?;
                return subscript_expr(
                    builder,
                    value,
                    index_expr,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                    span,
                );
            }
            builder
                .call_node(
                    "Attribute",
                    vec![
                        value,
                        attr.to_string().into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Index { value, index, span } => {
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            let index = lower_expr_with_exception(builder, index, exception_name)?;
            subscript_expr(
                builder,
                value,
                index,
                builder.load_ctx().map_err(py_err_to_lower)?,
                span,
            )
        }
        Expr::Paren { expr, .. } => lower_expr_with_exception(builder, expr, exception_name),
        Expr::List { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "List",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Tuple { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "Tuple",
                    vec![
                        PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Set { elements, span } => {
            let mut lowered = Vec::with_capacity(elements.len());
            for element in elements {
                lowered.push(lower_expr_with_exception(builder, element, exception_name)?);
            }
            builder
                .call_node(
                    "Set",
                    vec![PyList::new_bound(builder.py(), lowered).into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Dict { entries, span } => {
            let mut keys = Vec::with_capacity(entries.len());
            let mut values = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                keys.push(lower_expr_with_exception(builder, key, exception_name)?);
                values.push(lower_expr_with_exception(builder, value, exception_name)?);
            }
            builder
                .call_node(
                    "Dict",
                    vec![
                        PyList::new_bound(builder.py(), keys).into_py(builder.py()),
                        PyList::new_bound(builder.py(), values).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => {
            let element = lower_expr_with_exception(builder, element, exception_name)?;
            let target = name_expr(
                builder,
                target,
                span,
                builder.store_ctx().map_err(py_err_to_lower)?,
            )?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node_no_loc(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "ListComp",
                    vec![
                        element,
                        PyList::new_bound(builder.py(), vec![comprehension]).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => {
            let key = lower_expr_with_exception(builder, key, exception_name)?;
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            let target = name_expr(
                builder,
                target,
                span,
                builder.store_ctx().map_err(py_err_to_lower)?,
            )?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node_no_loc(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "DictComp",
                    vec![
                        key,
                        value,
                        PyList::new_bound(builder.py(), vec![comprehension]).into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Slice { start, end, span } => {
            let start = start
                .as_deref()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .transpose()?;
            let end = end
                .as_deref()
                .map(|expr| lower_expr_with_exception(builder, expr, exception_name))
                .transpose()?;
            builder
                .call_node(
                    "Slice",
                    vec![
                        start.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                        end.unwrap_or_else(|| builder.py().None().into_py(builder.py())),
                        builder.py().None().into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)
        }
    }
}

fn lower_aug_assign(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    op: AugAssignOp,
    value: &Expr,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match target {
        AssignTarget::Name { name, .. } => {
            let left = name_expr(
                builder,
                name,
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let right = lower_expr_with_exception(builder, value, exception_name)?;
            let op_node = lower_aug_assign_op(builder, op)?;
            let binop = builder
                .call_node("BinOp", vec![left, op_node, right], span)
                .map_err(py_err_to_lower)?;
            named_expr(builder, name, binop, span)
        }
        AssignTarget::Attribute {
            value: target_value,
            attr,
            ..
        } => {
            let object = lower_expr_with_exception(builder, target_value, exception_name)?;
            let attr_node = builder
                .call_node(
                    "Constant",
                    vec![attr.to_string().into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let rhs = lower_expr_with_exception(builder, value, exception_name)?;
            let op_node = builder
                .call_node(
                    "Constant",
                    vec![aug_op_to_string(op).to_string().into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)?;
            lower_runtime_call(
                builder,
                SNAIL_AUG_ATTR,
                vec![object, attr_node, rhs, op_node],
                span,
            )
        }
        AssignTarget::Index {
            value: target_value,
            index: target_index,
            ..
        } => {
            let object = lower_expr_with_exception(builder, target_value, exception_name)?;
            let index = lower_expr_with_exception(builder, target_index, exception_name)?;
            let rhs = lower_expr_with_exception(builder, value, exception_name)?;
            let op_node = builder
                .call_node(
                    "Constant",
                    vec![aug_op_to_string(op).to_string().into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)?;
            lower_runtime_call(
                builder,
                SNAIL_AUG_INDEX,
                vec![object, index, rhs, op_node],
                span,
            )
        }
        AssignTarget::Starred { .. } | AssignTarget::Tuple { .. } | AssignTarget::List { .. } => {
            Err(LowerError::new(
                "augmented assignment target must be a name, attribute, or index",
            ))
        }
    }
}

fn lower_incr(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    op: IncrOp,
    pre: bool,
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    match target {
        AssignTarget::Name { name, .. } => {
            if pre {
                return lower_named_incr(builder, name, op, span);
            }
            lower_postfix_name_incr(builder, name, op, span)
        }
        AssignTarget::Attribute { value, attr, .. } => {
            let object = lower_expr_with_exception(builder, value, exception_name)?;
            let attr_node = builder
                .call_node(
                    "Constant",
                    vec![attr.to_string().into_py(builder.py())],
                    span,
                )
                .map_err(py_err_to_lower)?;
            let delta = number_expr(builder, incr_delta(op), span)?;
            let pre_node = builder
                .call_node("Constant", vec![pre.into_py(builder.py())], span)
                .map_err(py_err_to_lower)?;
            lower_runtime_call(
                builder,
                SNAIL_INCR_ATTR,
                vec![object, attr_node, delta, pre_node],
                span,
            )
        }
        AssignTarget::Index { value, index, .. } => {
            let object = lower_expr_with_exception(builder, value, exception_name)?;
            let index = lower_expr_with_exception(builder, index, exception_name)?;
            let delta = number_expr(builder, incr_delta(op), span)?;
            let pre_node = builder
                .call_node("Constant", vec![pre.into_py(builder.py())], span)
                .map_err(py_err_to_lower)?;
            lower_runtime_call(
                builder,
                SNAIL_INCR_INDEX,
                vec![object, index, delta, pre_node],
                span,
            )
        }
        AssignTarget::Starred { .. } | AssignTarget::Tuple { .. } | AssignTarget::List { .. } => {
            Err(LowerError::new(
                "increment/decrement target must be a name, attribute, or index",
            ))
        }
    }
}

fn lower_runtime_call(
    builder: &AstBuilder<'_>,
    helper_name: &str,
    args: Vec<PyObject>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let func = name_expr(
        builder,
        helper_name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), args).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_named_incr(
    builder: &AstBuilder<'_>,
    name: &str,
    op: IncrOp,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let left = name_expr(
        builder,
        name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let delta = number_expr(builder, "1", span)?;
    let aug_op = match op {
        IncrOp::Increment => AugAssignOp::Add,
        IncrOp::Decrement => AugAssignOp::Sub,
    };
    let op_node = lower_aug_assign_op(builder, aug_op)?;
    let binop = builder
        .call_node("BinOp", vec![left, op_node, delta], span)
        .map_err(py_err_to_lower)?;
    named_expr(builder, name, binop, span)
}

fn lower_postfix_name_incr(
    builder: &AstBuilder<'_>,
    name: &str,
    op: IncrOp,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let current = name_expr(
        builder,
        name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let tmp_assign = named_expr(builder, SNAIL_INCR_TMP, current, span)?;
    let update = lower_named_incr(builder, name, op, span)?;
    let tmp_load = name_expr(
        builder,
        SNAIL_INCR_TMP,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let tuple_expr = builder
        .call_node(
            "Tuple",
            vec![
                PyList::new_bound(builder.py(), vec![tmp_assign, update, tmp_load])
                    .into_py(builder.py()),
                builder.load_ctx().map_err(py_err_to_lower)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let index_expr = builder
        .call_node(
            "UnaryOp",
            vec![
                lower_unary_op(builder, UnaryOp::Minus)?,
                number_expr(builder, "1", span)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    subscript_expr(
        builder,
        tuple_expr,
        index_expr,
        builder.load_ctx().map_err(py_err_to_lower)?,
        span,
    )
}

fn incr_delta(op: IncrOp) -> &'static str {
    match op {
        IncrOp::Increment => "1",
        IncrOp::Decrement => "-1",
    }
}

fn lower_subprocess_object(
    builder: &AstBuilder<'_>,
    kind: &SubprocessKind,
    parts: &[SubprocessPart],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let values = lower_subprocess_parts(builder, parts, exception_name)?;
    let command = builder
        .call_node(
            "JoinedStr",
            vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
            span,
        )
        .map_err(py_err_to_lower)?;
    let class_name = match kind {
        SubprocessKind::Capture => SNAIL_SUBPROCESS_CAPTURE_CLASS,
        SubprocessKind::Status => SNAIL_SUBPROCESS_STATUS_CLASS,
    };
    let func = name_expr(
        builder,
        class_name,
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    builder
        .call_node(
            "Call",
            vec![
                func,
                PyList::new_bound(builder.py(), vec![command]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)
}

fn lower_subprocess_parts(
    builder: &AstBuilder<'_>,
    parts: &[SubprocessPart],
    exception_name: Option<&str>,
) -> Result<Vec<PyObject>, LowerError> {
    let mut lowered_parts = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            SubprocessPart::Text(text) => {
                let const_node = builder
                    .call_node(
                        "Constant",
                        vec![text.clone().into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered_parts.push(const_node);
            }
            SubprocessPart::Expr(expr) => {
                let value = lower_expr_with_exception(builder, expr, exception_name)?;
                let conversion = (-1i32).into_py(builder.py());
                let format_spec = builder.py().None();
                let formatted = builder
                    .call_node(
                        "FormattedValue",
                        vec![value, conversion, format_spec.into_py(builder.py())],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                lowered_parts.push(formatted);
            }
        }
    }
    Ok(lowered_parts)
}

fn lower_lambda_expr(
    builder: &AstBuilder<'_>,
    params: &[Parameter],
    body: &[Stmt],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let args = lower_parameters(builder, params, exception_name)?;
    let body_expr = lower_lambda_body_expr(builder, body, span, exception_name)?;
    builder
        .call_node("Lambda", vec![args, body_expr], span)
        .map_err(py_err_to_lower)
}

fn lower_lambda_body_expr(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let mut lowered = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Expr { value, .. } => {
                lowered.push(lower_expr_with_exception(builder, value, exception_name)?);
            }
            _ => {
                return Err(LowerError::new(
                    "def expression bodies must contain only expression statements",
                ));
            }
        }
    }

    if lowered.is_empty() {
        return builder
            .call_node(
                "Constant",
                vec![builder.py().None().into_py(builder.py())],
                span,
            )
            .map_err(py_err_to_lower);
    }

    if lowered.len() == 1 {
        return Ok(lowered.pop().unwrap());
    }

    let tuple_expr = builder
        .call_node(
            "Tuple",
            vec![
                PyList::new_bound(builder.py(), lowered).into_py(builder.py()),
                builder.load_ctx().map_err(py_err_to_lower)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    let index_expr = builder
        .call_node(
            "UnaryOp",
            vec![
                lower_unary_op(builder, UnaryOp::Minus)?,
                number_expr(builder, "1", span)?,
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    subscript_expr(
        builder,
        tuple_expr,
        index_expr,
        builder.load_ctx().map_err(py_err_to_lower)?,
        span,
    )
}

fn lower_call_arguments(
    builder: &AstBuilder<'_>,
    args: &[Argument],
    exception_name: Option<&str>,
) -> Result<(Vec<PyObject>, Vec<PyObject>), LowerError> {
    let mut positional = Vec::new();
    let mut keywords = Vec::new();
    for arg in args {
        match arg {
            Argument::Positional { value, .. } => {
                positional.push(lower_expr_with_exception(builder, value, exception_name)?);
            }
            Argument::Keyword { name, value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node_no_loc(
                        "keyword",
                        vec![name.to_string().into_py(builder.py()), value],
                    )
                    .map_err(py_err_to_lower)?;
                keywords.push(keyword);
            }
            Argument::Star { value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let starred = builder
                    .call_node(
                        "Starred",
                        vec![value, builder.load_ctx().map_err(py_err_to_lower)?],
                        &dummy_span(),
                    )
                    .map_err(py_err_to_lower)?;
                positional.push(starred);
            }
            Argument::KwStar { value, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node_no_loc(
                        "keyword",
                        vec![builder.py().None().into_py(builder.py()), value],
                    )
                    .map_err(py_err_to_lower)?;
                keywords.push(keyword);
            }
        }
    }
    Ok((positional, keywords))
}

#[derive(Default)]
struct PlaceholderInfo {
    count: usize,
    first_span: Option<SourceSpan>,
}

fn placeholder_info_in_arguments(args: &[Argument]) -> PlaceholderInfo {
    let mut info = PlaceholderInfo::default();
    for arg in args {
        match arg {
            Argument::Positional { value, .. }
            | Argument::Keyword { value, .. }
            | Argument::Star { value, .. }
            | Argument::KwStar { value, .. } => count_placeholders(value, &mut info),
        }
    }
    info
}

fn count_placeholders(expr: &Expr, info: &mut PlaceholderInfo) {
    match expr {
        Expr::Placeholder { span } => {
            if info.first_span.is_none() {
                info.first_span = Some(span.clone());
            }
            info.count += 1;
        }
        Expr::Name { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => {}
        Expr::FString { parts, .. } => {
            for part in parts {
                count_placeholders_in_fstring_part(part, info);
            }
        }
        Expr::Unary { expr, .. } => count_placeholders(expr, info),
        Expr::Binary { left, right, .. } => {
            count_placeholders(left, info);
            count_placeholders(right, info);
        }
        Expr::AugAssign { target, value, .. } => {
            count_placeholders_in_assign_target(target, info);
            count_placeholders(value, info);
        }
        Expr::PrefixIncr { target, .. } | Expr::PostfixIncr { target, .. } => {
            count_placeholders_in_assign_target(target, info);
        }
        Expr::Compare {
            left, comparators, ..
        } => {
            count_placeholders(left, info);
            for expr in comparators {
                count_placeholders(expr, info);
            }
        }
        Expr::IfExpr {
            test, body, orelse, ..
        } => {
            count_placeholders(test, info);
            count_placeholders(body, info);
            count_placeholders(orelse, info);
        }
        Expr::TryExpr { expr, fallback, .. } => {
            count_placeholders(expr, info);
            if let Some(fallback) = fallback {
                count_placeholders(fallback, info);
            }
        }
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                count_placeholders(value, info);
            }
        }
        Expr::YieldFrom { expr, .. } => count_placeholders(expr, info),
        Expr::Lambda { params, body, .. } => {
            for param in params {
                if let Parameter::Regular { default, .. } = param
                    && let Some(default) = default
                {
                    count_placeholders(default, info);
                }
            }
            for stmt in body {
                if let Stmt::Expr { value, .. } = stmt {
                    count_placeholders(value, info);
                }
            }
        }
        Expr::Compound { expressions, .. } => {
            for expr in expressions {
                count_placeholders(expr, info);
            }
        }
        Expr::Regex { pattern, .. } => count_placeholders_in_regex(pattern, info),
        Expr::RegexMatch { value, pattern, .. } => {
            count_placeholders(value, info);
            count_placeholders_in_regex(pattern, info);
        }
        Expr::Subprocess { parts, .. } => {
            for part in parts {
                if let SubprocessPart::Expr(expr) = part {
                    count_placeholders(expr, info);
                }
            }
        }
        Expr::Call { func, args, .. } => {
            count_placeholders(func, info);
            for arg in args {
                match arg {
                    Argument::Positional { value, .. }
                    | Argument::Keyword { value, .. }
                    | Argument::Star { value, .. }
                    | Argument::KwStar { value, .. } => count_placeholders(value, info),
                }
            }
        }
        Expr::Attribute { value, .. } => count_placeholders(value, info),
        Expr::Index { value, index, .. } => {
            count_placeholders(value, info);
            count_placeholders(index, info);
        }
        Expr::Paren { expr, .. } => count_placeholders(expr, info),
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                count_placeholders(expr, info);
            }
        }
        Expr::Dict { entries, .. } => {
            for (key, value) in entries {
                count_placeholders(key, info);
                count_placeholders(value, info);
            }
        }
        Expr::Slice { start, end, .. } => {
            if let Some(start) = start {
                count_placeholders(start, info);
            }
            if let Some(end) = end {
                count_placeholders(end, info);
            }
        }
        Expr::ListComp {
            element, iter, ifs, ..
        } => {
            count_placeholders(element, info);
            count_placeholders(iter, info);
            for expr in ifs {
                count_placeholders(expr, info);
            }
        }
        Expr::DictComp {
            key,
            value,
            iter,
            ifs,
            ..
        } => {
            count_placeholders(key, info);
            count_placeholders(value, info);
            count_placeholders(iter, info);
            for expr in ifs {
                count_placeholders(expr, info);
            }
        }
    }
}

fn count_placeholders_in_assign_target(target: &AssignTarget, info: &mut PlaceholderInfo) {
    match target {
        AssignTarget::Name { .. } => {}
        AssignTarget::Attribute { value, .. } => count_placeholders(value, info),
        AssignTarget::Index { value, index, .. } => {
            count_placeholders(value, info);
            count_placeholders(index, info);
        }
        AssignTarget::Starred { target, .. } => count_placeholders_in_assign_target(target, info),
        AssignTarget::Tuple { elements, .. } | AssignTarget::List { elements, .. } => {
            for element in elements {
                count_placeholders_in_assign_target(element, info);
            }
        }
    }
}

fn count_placeholders_in_regex(pattern: &RegexPattern, info: &mut PlaceholderInfo) {
    if let RegexPattern::Interpolated(parts) = pattern {
        for part in parts {
            count_placeholders_in_fstring_part(part, info);
        }
    }
}

fn count_placeholders_in_fstring_part(part: &FStringPart, info: &mut PlaceholderInfo) {
    if let FStringPart::Expr(expr) = part {
        count_placeholders(&expr.expr, info);
        if let Some(spec) = &expr.format_spec {
            for spec_part in spec {
                count_placeholders_in_fstring_part(spec_part, info);
            }
        }
    }
}

fn substitute_placeholder_in_arguments(args: &[Argument], replacement: &Expr) -> Vec<Argument> {
    args.iter()
        .map(|arg| match arg {
            Argument::Positional { value, span } => Argument::Positional {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::Keyword { name, value, span } => Argument::Keyword {
                name: name.clone(),
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::Star { value, span } => Argument::Star {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
            Argument::KwStar { value, span } => Argument::KwStar {
                value: substitute_placeholder(value, replacement),
                span: span.clone(),
            },
        })
        .collect()
}

fn substitute_placeholder(expr: &Expr, replacement: &Expr) -> Expr {
    match expr {
        Expr::Placeholder { .. } => replacement.clone(),
        Expr::Name { .. }
        | Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Bool { .. }
        | Expr::None { .. }
        | Expr::StructuredAccessor { .. }
        | Expr::FieldIndex { .. } => expr.clone(),
        Expr::FString { parts, bytes, span } => Expr::FString {
            parts: parts
                .iter()
                .map(|part| substitute_placeholder_in_fstring_part(part, replacement))
                .collect(),
            bytes: *bytes,
            span: span.clone(),
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_placeholder(expr, replacement)),
            span: span.clone(),
        },
        Expr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(substitute_placeholder(left, replacement)),
            op: *op,
            right: Box::new(substitute_placeholder(right, replacement)),
            span: span.clone(),
        },
        Expr::AugAssign {
            target,
            op,
            value,
            span,
        } => Expr::AugAssign {
            target: Box::new(substitute_placeholder_in_assign_target(target, replacement)),
            op: *op,
            value: Box::new(substitute_placeholder(value, replacement)),
            span: span.clone(),
        },
        Expr::PrefixIncr { op, target, span } => Expr::PrefixIncr {
            op: *op,
            target: Box::new(substitute_placeholder_in_assign_target(target, replacement)),
            span: span.clone(),
        },
        Expr::PostfixIncr { op, target, span } => Expr::PostfixIncr {
            op: *op,
            target: Box::new(substitute_placeholder_in_assign_target(target, replacement)),
            span: span.clone(),
        },
        Expr::Compare {
            left,
            ops,
            comparators,
            span,
        } => Expr::Compare {
            left: Box::new(substitute_placeholder(left, replacement)),
            ops: ops.clone(),
            comparators: comparators
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::IfExpr {
            test,
            body,
            orelse,
            span,
        } => Expr::IfExpr {
            test: Box::new(substitute_placeholder(test, replacement)),
            body: Box::new(substitute_placeholder(body, replacement)),
            orelse: Box::new(substitute_placeholder(orelse, replacement)),
            span: span.clone(),
        },
        Expr::TryExpr {
            expr,
            fallback,
            span,
        } => Expr::TryExpr {
            expr: Box::new(substitute_placeholder(expr, replacement)),
            fallback: fallback
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            span: span.clone(),
        },
        Expr::Yield { value, span } => Expr::Yield {
            value: value
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            span: span.clone(),
        },
        Expr::YieldFrom { expr, span } => Expr::YieldFrom {
            expr: Box::new(substitute_placeholder(expr, replacement)),
            span: span.clone(),
        },
        Expr::Lambda { params, body, span } => Expr::Lambda {
            params: params
                .iter()
                .map(|param| match param {
                    Parameter::Regular {
                        name,
                        default,
                        span,
                    } => Parameter::Regular {
                        name: name.clone(),
                        default: default
                            .as_ref()
                            .map(|expr| substitute_placeholder(expr, replacement)),
                        span: span.clone(),
                    },
                    Parameter::VarArgs { name, span } => Parameter::VarArgs {
                        name: name.clone(),
                        span: span.clone(),
                    },
                    Parameter::KwArgs { name, span } => Parameter::KwArgs {
                        name: name.clone(),
                        span: span.clone(),
                    },
                })
                .collect(),
            body: body
                .iter()
                .map(|stmt| match stmt {
                    Stmt::Expr {
                        value,
                        semicolon_terminated,
                        span,
                    } => Stmt::Expr {
                        value: substitute_placeholder(value, replacement),
                        semicolon_terminated: *semicolon_terminated,
                        span: span.clone(),
                    },
                    _ => stmt.clone(),
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Compound { expressions, span } => Expr::Compound {
            expressions: expressions
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Regex { pattern, span } => Expr::Regex {
            pattern: substitute_placeholder_in_regex(pattern, replacement),
            span: span.clone(),
        },
        Expr::RegexMatch {
            value,
            pattern,
            span,
        } => Expr::RegexMatch {
            value: Box::new(substitute_placeholder(value, replacement)),
            pattern: substitute_placeholder_in_regex(pattern, replacement),
            span: span.clone(),
        },
        Expr::Subprocess { kind, parts, span } => Expr::Subprocess {
            kind: *kind,
            parts: parts
                .iter()
                .map(|part| match part {
                    SubprocessPart::Text(text) => SubprocessPart::Text(text.clone()),
                    SubprocessPart::Expr(expr) => {
                        SubprocessPart::Expr(Box::new(substitute_placeholder(expr, replacement)))
                    }
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Call { func, args, span } => Expr::Call {
            func: Box::new(substitute_placeholder(func, replacement)),
            args: substitute_placeholder_in_arguments(args, replacement),
            span: span.clone(),
        },
        Expr::Attribute { value, attr, span } => Expr::Attribute {
            value: Box::new(substitute_placeholder(value, replacement)),
            attr: attr.clone(),
            span: span.clone(),
        },
        Expr::Index { value, index, span } => Expr::Index {
            value: Box::new(substitute_placeholder(value, replacement)),
            index: Box::new(substitute_placeholder(index, replacement)),
            span: span.clone(),
        },
        Expr::Paren { expr, span } => Expr::Paren {
            expr: Box::new(substitute_placeholder(expr, replacement)),
            span: span.clone(),
        },
        Expr::List { elements, span } => Expr::List {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Tuple { elements, span } => Expr::Tuple {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Set { elements, span } => Expr::Set {
            elements: elements
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::Dict { entries, span } => Expr::Dict {
            entries: entries
                .iter()
                .map(|(key, value)| {
                    (
                        substitute_placeholder(key, replacement),
                        substitute_placeholder(value, replacement),
                    )
                })
                .collect(),
            span: span.clone(),
        },
        Expr::Slice { start, end, span } => Expr::Slice {
            start: start
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            end: end
                .as_ref()
                .map(|expr| Box::new(substitute_placeholder(expr, replacement))),
            span: span.clone(),
        },
        Expr::ListComp {
            element,
            target,
            iter,
            ifs,
            span,
        } => Expr::ListComp {
            element: Box::new(substitute_placeholder(element, replacement)),
            target: target.clone(),
            iter: Box::new(substitute_placeholder(iter, replacement)),
            ifs: ifs
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
        Expr::DictComp {
            key,
            value,
            target,
            iter,
            ifs,
            span,
        } => Expr::DictComp {
            key: Box::new(substitute_placeholder(key, replacement)),
            value: Box::new(substitute_placeholder(value, replacement)),
            target: target.clone(),
            iter: Box::new(substitute_placeholder(iter, replacement)),
            ifs: ifs
                .iter()
                .map(|expr| substitute_placeholder(expr, replacement))
                .collect(),
            span: span.clone(),
        },
    }
}

fn substitute_placeholder_in_assign_target(
    target: &AssignTarget,
    replacement: &Expr,
) -> AssignTarget {
    match target {
        AssignTarget::Name { name, span } => AssignTarget::Name {
            name: name.clone(),
            span: span.clone(),
        },
        AssignTarget::Attribute { value, attr, span } => AssignTarget::Attribute {
            value: Box::new(substitute_placeholder(value, replacement)),
            attr: attr.clone(),
            span: span.clone(),
        },
        AssignTarget::Index { value, index, span } => AssignTarget::Index {
            value: Box::new(substitute_placeholder(value, replacement)),
            index: Box::new(substitute_placeholder(index, replacement)),
            span: span.clone(),
        },
        AssignTarget::Starred { target, span } => AssignTarget::Starred {
            target: Box::new(substitute_placeholder_in_assign_target(target, replacement)),
            span: span.clone(),
        },
        AssignTarget::Tuple { elements, span } => AssignTarget::Tuple {
            elements: elements
                .iter()
                .map(|element| substitute_placeholder_in_assign_target(element, replacement))
                .collect(),
            span: span.clone(),
        },
        AssignTarget::List { elements, span } => AssignTarget::List {
            elements: elements
                .iter()
                .map(|element| substitute_placeholder_in_assign_target(element, replacement))
                .collect(),
            span: span.clone(),
        },
    }
}

fn substitute_placeholder_in_regex(pattern: &RegexPattern, replacement: &Expr) -> RegexPattern {
    match pattern {
        RegexPattern::Literal(text) => RegexPattern::Literal(text.clone()),
        RegexPattern::Interpolated(parts) => RegexPattern::Interpolated(
            parts
                .iter()
                .map(|part| substitute_placeholder_in_fstring_part(part, replacement))
                .collect(),
        ),
    }
}

fn substitute_placeholder_in_fstring_part(part: &FStringPart, replacement: &Expr) -> FStringPart {
    match part {
        FStringPart::Text(text) => FStringPart::Text(text.clone()),
        FStringPart::Expr(expr) => FStringPart::Expr(FStringExpr {
            expr: Box::new(substitute_placeholder(&expr.expr, replacement)),
            conversion: expr.conversion,
            format_spec: expr.format_spec.as_ref().map(|parts| {
                parts
                    .iter()
                    .map(|part| substitute_placeholder_in_fstring_part(part, replacement))
                    .collect()
            }),
        }),
    }
}

fn empty_lambda_args(builder: &AstBuilder<'_>) -> Result<PyObject, LowerError> {
    builder
        .call_node_no_loc(
            "arguments",
            vec![
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)
}

fn lambda_args_with_param(builder: &AstBuilder<'_>, name: &str) -> Result<PyObject, LowerError> {
    let arg = builder
        .call_node_no_loc(
            "arg",
            vec![
                name.to_string().into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)?;
    builder
        .call_node_no_loc(
            "arguments",
            vec![
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::new_bound(builder.py(), vec![arg]).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                builder.py().None().into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
        )
        .map_err(py_err_to_lower)
}

fn dummy_span() -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            offset: 0,
            line: 0,
            column: 0,
        },
        end: SourcePos {
            offset: 0,
            line: 0,
            column: 0,
        },
    }
}
