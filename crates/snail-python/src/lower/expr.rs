use pyo3::prelude::*;
use pyo3::types::PyList;
use snail_ast::{DictEntry, *};
use snail_error::LowerError;

use super::constants::*;
use super::helpers::{
    assign_name, build_destructure_try, build_let_guard_test, byte_string_expr, name_expr,
    number_expr, regex_pattern_expr, string_expr,
};
use super::operators::{
    aug_op_to_string, lower_aug_assign_op, lower_binary_op, lower_bool_op, lower_compare_op,
    lower_unary_op,
};
use super::py_ast::{AstBuilder, py_err_to_lower};
use super::stmt::{
    TailBehavior, lower_block, lower_block_with_implicit_return, lower_block_with_tail,
    lower_except_handler, lower_parameters, lower_while_stmt, lower_with_item,
};

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
            let values = lower_fstring_parts(builder, parts, span, exception_name)?;
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
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<Vec<PyObject>, LowerError> {
    let mut lowered = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            FStringPart::Text(text) => {
                let const_node = builder
                    .call_node("Constant", vec![text.clone().into_py(builder.py())], span)
                    .map_err(py_err_to_lower)?;
                lowered.push(const_node);
            }
            FStringPart::Expr(expr) => {
                let value = lower_expr_with_exception(builder, &expr.expr, exception_name)?;
                let conversion = fstring_conversion(expr.conversion).into_py(builder.py());
                let format_spec = match &expr.format_spec {
                    Some(parts) => {
                        let values = lower_fstring_parts(builder, parts, span, exception_name)?;
                        builder
                            .call_node(
                                "JoinedStr",
                                vec![PyList::new_bound(builder.py(), values).into_py(builder.py())],
                                span,
                            )
                            .map_err(py_err_to_lower)?
                    }
                    None => builder.py().None().into_py(builder.py()),
                };
                let formatted = builder
                    .call_node("FormattedValue", vec![value, conversion, format_spec], span)
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
                // Wrap $text and $fd in __snail_force() to eagerly resolve
                // LazyProxy objects, so that compact try can catch I/O errors.
                if name == SNAIL_XARGS_TEXT || name == SNAIL_XARGS_FD {
                    let func = name_expr(
                        builder,
                        SNAIL_FORCE_HELPER,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?;
                    let inner = name_expr(
                        builder,
                        py_name,
                        span,
                        builder.load_ctx().map_err(py_err_to_lower)?,
                    )?;
                    return builder
                        .call_node(
                            "Call",
                            vec![
                                func,
                                PyList::new_bound(builder.py(), vec![inner]).into_py(builder.py()),
                                PyList::empty_bound(builder.py()).into_py(builder.py()),
                            ],
                            span,
                        )
                        .map_err(py_err_to_lower);
                }
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
            let values = lower_fstring_parts(builder, parts, span, exception_name)?;
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
            for entry in entries {
                match entry {
                    DictEntry::KeyValue { key, value, .. } => {
                        keys.push(lower_expr_with_exception(builder, key, exception_name)?);
                        values.push(lower_expr_with_exception(builder, value, exception_name)?);
                    }
                    DictEntry::Unpack { value, .. } => {
                        keys.push(builder.py().None().into_py(builder.py()));
                        values.push(lower_expr_with_exception(builder, value, exception_name)?);
                    }
                }
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
            let target = lower_assign_target(builder, target)?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                    span,
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
            let target = lower_assign_target(builder, target)?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                    span,
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
        Expr::GeneratorExpr {
            element,
            target,
            iter,
            ifs,
            span,
        } => {
            let element = lower_expr_with_exception(builder, element, exception_name)?;
            let target = lower_assign_target(builder, target)?;
            let iter = lower_expr_with_exception(builder, iter, exception_name)?;
            let mut lowered_ifs = Vec::with_capacity(ifs.len());
            for cond in ifs {
                lowered_ifs.push(lower_expr_with_exception(builder, cond, exception_name)?);
            }
            let comprehension = builder
                .call_node(
                    "comprehension",
                    vec![
                        target,
                        iter,
                        PyList::new_bound(builder.py(), lowered_ifs).into_py(builder.py()),
                        0u8.into_py(builder.py()),
                    ],
                    span,
                )
                .map_err(py_err_to_lower)?;
            builder
                .call_node(
                    "GeneratorExp",
                    vec![
                        element,
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
        Expr::Starred { value, span } => {
            let value = lower_expr_with_exception(builder, value, exception_name)?;
            builder
                .call_node(
                    "Starred",
                    vec![value, builder.load_ctx().map_err(py_err_to_lower)?],
                    span,
                )
                .map_err(py_err_to_lower)
        }
        Expr::Block { .. }
        | Expr::If { .. }
        | Expr::While { .. }
        | Expr::For { .. }
        | Expr::Def { .. }
        | Expr::Class { .. }
        | Expr::Try { .. }
        | Expr::With { .. }
        | Expr::Awk { .. }
        | Expr::Xargs { .. } => Err(LowerError::new(
            "compound expressions cannot be used in expression context yet",
        )),
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
    parts: &[FStringPart],
    span: &SourceSpan,
    exception_name: Option<&str>,
) -> Result<PyObject, LowerError> {
    let values = lower_fstring_parts(builder, parts, span, exception_name)?;
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

pub(super) fn lower_call_arguments(
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
            Argument::Keyword {
                name, value, span, ..
            } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node(
                        "keyword",
                        vec![name.to_string().into_py(builder.py()), value],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                keywords.push(keyword);
            }
            Argument::Star { value, span, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let starred = builder
                    .call_node(
                        "Starred",
                        vec![value, builder.load_ctx().map_err(py_err_to_lower)?],
                        span,
                    )
                    .map_err(py_err_to_lower)?;
                positional.push(starred);
            }
            Argument::KwStar { value, span, .. } => {
                let value = lower_expr_with_exception(builder, value, exception_name)?;
                let keyword = builder
                    .call_node(
                        "keyword",
                        vec![builder.py().None().into_py(builder.py()), value],
                        span,
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
        Expr::Yield { value, .. } => {
            if let Some(value) = value {
                count_placeholders(value, info);
            }
        }
        Expr::YieldFrom { expr, .. } => count_placeholders(expr, info),
        Expr::Regex { pattern, .. } => count_placeholders_in_regex(pattern, info),
        Expr::RegexMatch { value, pattern, .. } => {
            count_placeholders(value, info);
            count_placeholders_in_regex(pattern, info);
        }
        Expr::Subprocess { parts, .. } => {
            for part in parts {
                count_placeholders_in_fstring_part(part, info);
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
        Expr::Starred { value, .. } => count_placeholders(value, info),
        Expr::List { elements, .. } | Expr::Tuple { elements, .. } | Expr::Set { elements, .. } => {
            for expr in elements {
                count_placeholders(expr, info);
            }
        }
        Expr::Dict { entries, .. } => {
            for entry in entries {
                match entry {
                    DictEntry::KeyValue { key, value, .. } => {
                        count_placeholders(key, info);
                        count_placeholders(value, info);
                    }
                    DictEntry::Unpack { value, .. } => {
                        count_placeholders(value, info);
                    }
                }
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
        }
        | Expr::GeneratorExpr {
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
        // Compound expressions: placeholders are not expected inside these
        Expr::Block { .. }
        | Expr::If { .. }
        | Expr::While { .. }
        | Expr::For { .. }
        | Expr::Def { .. }
        | Expr::Class { .. }
        | Expr::With { .. }
        | Expr::Awk { .. }
        | Expr::Xargs { .. } => {}
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            ..
        } => {
            count_placeholders_in_stmt_block(body, info);
            for handler in handlers {
                if let Some(type_name) = &handler.type_name {
                    count_placeholders(type_name, info);
                }
                count_placeholders_in_stmt_block(&handler.body, info);
            }
            if let Some(else_body) = else_body {
                count_placeholders_in_stmt_block(else_body, info);
            }
            if let Some(finally_body) = finally_body {
                count_placeholders_in_stmt_block(finally_body, info);
            }
        }
    }
}

fn count_placeholders_in_stmt_block(stmts: &[Stmt], info: &mut PlaceholderInfo) {
    for stmt in stmts {
        if let Stmt::Expr { value, .. } = stmt {
            count_placeholders(value, info);
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
                .map(|part| substitute_placeholder_in_fstring_part(part, replacement))
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
        Expr::Starred { value, span } => Expr::Starred {
            value: Box::new(substitute_placeholder(value, replacement)),
            span: span.clone(),
        },
        Expr::Dict { entries, span } => Expr::Dict {
            entries: entries
                .iter()
                .map(|entry| match entry {
                    DictEntry::KeyValue { key, value, span } => DictEntry::KeyValue {
                        key: substitute_placeholder(key, replacement),
                        value: substitute_placeholder(value, replacement),
                        span: span.clone(),
                    },
                    DictEntry::Unpack { value, span } => DictEntry::Unpack {
                        value: substitute_placeholder(value, replacement),
                        span: span.clone(),
                    },
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
        Expr::GeneratorExpr {
            element,
            target,
            iter,
            ifs,
            span,
        } => Expr::GeneratorExpr {
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
        // Compound expressions: placeholders are not expected inside these,
        // but we clone them through for completeness.
        Expr::Block { stmts, span } => Expr::Block {
            stmts: stmts.clone(),
            span: span.clone(),
        },
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => Expr::If {
            cond: cond.clone(),
            body: body.clone(),
            elifs: elifs.clone(),
            else_body: else_body.clone(),
            span: span.clone(),
        },
        Expr::While {
            cond,
            body,
            else_body,
            span,
        } => Expr::While {
            cond: cond.clone(),
            body: body.clone(),
            else_body: else_body.clone(),
            span: span.clone(),
        },
        Expr::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => Expr::For {
            target: target.clone(),
            iter: iter.clone(),
            body: body.clone(),
            else_body: else_body.clone(),
            span: span.clone(),
        },
        Expr::Def {
            name,
            params,
            body,
            decorators,
            span,
        } => Expr::Def {
            name: name.clone(),
            params: params.clone(),
            body: body.clone(),
            decorators: decorators.clone(),
            span: span.clone(),
        },
        Expr::Class {
            name,
            bases,
            body,
            decorators,
            span,
        } => Expr::Class {
            name: name.clone(),
            bases: bases.clone(),
            body: body.clone(),
            decorators: decorators.clone(),
            span: span.clone(),
        },
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => Expr::Try {
            body: substitute_placeholder_in_stmt_block(body, replacement),
            handlers: handlers
                .iter()
                .map(|handler| ExceptHandler {
                    type_name: handler
                        .type_name
                        .as_ref()
                        .map(|expr| substitute_placeholder(expr, replacement)),
                    name: handler.name.clone(),
                    body: substitute_placeholder_in_stmt_block(&handler.body, replacement),
                    span: handler.span.clone(),
                })
                .collect(),
            else_body: else_body
                .as_ref()
                .map(|body| substitute_placeholder_in_stmt_block(body, replacement)),
            finally_body: finally_body
                .as_ref()
                .map(|body| substitute_placeholder_in_stmt_block(body, replacement)),
            span: span.clone(),
        },
        Expr::With {
            items, body, span, ..
        } => Expr::With {
            items: items.clone(),
            body: body.clone(),
            span: span.clone(),
        },
        Expr::Awk {
            sources,
            body,
            span,
        } => Expr::Awk {
            sources: sources.clone(),
            body: body.clone(),
            span: span.clone(),
        },
        Expr::Xargs {
            sources,
            body,
            span,
        } => Expr::Xargs {
            sources: sources.clone(),
            body: body.clone(),
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

fn substitute_placeholder_in_stmt_block(stmts: &[Stmt], replacement: &Expr) -> Vec<Stmt> {
    stmts
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
            other => other.clone(),
        })
        .collect()
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

fn lower_for_stmt(
    builder: &AstBuilder<'_>,
    target: &AssignTarget,
    iter: &Expr,
    body: &[Stmt],
    else_body: &Option<Vec<Stmt>>,
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    use super::break_rewrite::rewrite_breaks_in_block;
    let target = lower_assign_target(builder, target)?;
    let iter_expr = lower_expr(builder, iter)?;
    let body_tail = if tail != TailBehavior::None {
        TailBehavior::CaptureOnly
    } else {
        TailBehavior::None
    };
    let rewritten_body;
    let body_ref = if tail != TailBehavior::None {
        rewritten_body = {
            let mut b = body.to_vec();
            rewrite_breaks_in_block(&mut b, "__snail_last_result", span);
            b
        };
        &rewritten_body[..]
    } else {
        body
    };
    let body = lower_block_with_tail(builder, body_ref, body_tail, span)?;
    let orelse = else_body
        .as_ref()
        .map(|items| lower_block_with_tail(builder, items, body_tail, span))
        .transpose()?
        .unwrap_or_default();
    let for_node = builder
        .call_node(
            "For",
            vec![
                target,
                iter_expr,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                PyList::new_bound(builder.py(), orelse).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    if tail != TailBehavior::None {
        wrap_compound_with_tail(builder, vec![for_node], tail, span)
    } else {
        Ok(vec![for_node])
    }
}

fn lower_def_stmt(
    builder: &AstBuilder<'_>,
    name: &str,
    params: &[Parameter],
    body: &[Stmt],
    decorators: &[Expr],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let args = lower_parameters(builder, params, None, span)?;
    let body = lower_block_with_implicit_return(builder, body, span)?;
    let lowered_decorators: Vec<PyObject> = decorators
        .iter()
        .map(|d| lower_expr(builder, d))
        .collect::<Result<Vec<_>, _>>()?;
    let decorator_list = PyList::new_bound(builder.py(), &lowered_decorators).into_py(builder.py());
    let func_node = builder
        .call_node(
            "FunctionDef",
            vec![
                name.to_string().into_py(builder.py()),
                args,
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                decorator_list,
                builder.py().None().into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    Ok(vec![func_node])
}

fn lower_class_stmt(
    builder: &AstBuilder<'_>,
    name: &str,
    bases: &[Expr],
    body: &[Stmt],
    decorators: &[Expr],
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let lowered_bases: Vec<PyObject> = bases
        .iter()
        .map(|b| lower_expr(builder, b))
        .collect::<Result<Vec<_>, _>>()?;
    let lowered_decorators: Vec<PyObject> = decorators
        .iter()
        .map(|d| lower_expr(builder, d))
        .collect::<Result<Vec<_>, _>>()?;
    let body = lower_block(builder, body, span)?;
    let class_node = builder
        .call_node(
            "ClassDef",
            vec![
                name.to_string().into_py(builder.py()),
                PyList::new_bound(builder.py(), &lowered_bases).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
                PyList::new_bound(builder.py(), &lowered_decorators).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    Ok(vec![class_node])
}

fn lower_try_stmt(
    builder: &AstBuilder<'_>,
    body: &[Stmt],
    handlers: &[ExceptHandler],
    else_body: &Option<Vec<Stmt>>,
    finally_body: &Option<Vec<Stmt>>,
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let body_tail = if tail != TailBehavior::None {
        TailBehavior::CaptureOnly
    } else {
        TailBehavior::None
    };
    let body = lower_block_with_tail(builder, body, body_tail, span)?;
    let handlers = handlers
        .iter()
        .map(|handler| lower_except_handler(builder, handler, tail))
        .collect::<Result<Vec<_>, _>>()?;
    let orelse = else_body
        .as_ref()
        .map(|items| lower_block_with_tail(builder, items, body_tail, span))
        .transpose()?
        .unwrap_or_default();
    // Finally blocks are cleanup code — never capture
    let finalbody = finally_body
        .as_ref()
        .map(|items| lower_block(builder, items, span))
        .transpose()?
        .unwrap_or_default();
    let try_node = builder
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
        .map_err(py_err_to_lower)?;
    if tail != TailBehavior::None {
        wrap_compound_with_tail(builder, vec![try_node], tail, span)
    } else {
        Ok(vec![try_node])
    }
}

fn lower_with_stmt(
    builder: &AstBuilder<'_>,
    items: &[WithItem],
    body: &[Stmt],
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let items = items
        .iter()
        .map(|item| lower_with_item(builder, item))
        .collect::<Result<Vec<_>, _>>()?;
    let body_tail = if tail != TailBehavior::None {
        TailBehavior::CaptureOnly
    } else {
        TailBehavior::None
    };
    let body = lower_block_with_tail(builder, body, body_tail, span)?;
    let with_node = builder
        .call_node(
            "With",
            vec![
                PyList::new_bound(builder.py(), items).into_py(builder.py()),
                PyList::new_bound(builder.py(), body).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    if tail != TailBehavior::None {
        wrap_compound_with_tail(builder, vec![with_node], tail, span)
    } else {
        Ok(vec![with_node])
    }
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

pub(crate) fn lower_if_chain(
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

pub(crate) fn lower_if_block_with_tail(
    builder: &AstBuilder<'_>,
    cond: &Condition,
    body: &[Stmt],
    elifs: &[(Condition, Vec<Stmt>)],
    else_body: &Option<Vec<Stmt>>,
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    match cond {
        Condition::Expr(cond_expr) => {
            let test = lower_expr(builder, cond_expr)?;
            let body = lower_block_with_tail(builder, body, tail, span)?;
            let orelse = if let Some((elif_cond, elif_body)) = elifs.first() {
                lower_if_block_with_tail(
                    builder,
                    elif_cond,
                    elif_body,
                    &elifs[1..],
                    else_body,
                    tail,
                    span,
                )?
            } else if let Some(else_body) = else_body {
                lower_block_with_tail(builder, else_body, tail, span)?
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
            Ok(vec![if_node])
        }
        Condition::Let {
            target,
            value,
            guard,
            span: cond_span,
        } => {
            let mut stmts = Vec::new();
            let value_expr = lower_expr(builder, value)?;
            stmts.push(assign_name(
                builder,
                SNAIL_LET_VALUE,
                value_expr,
                cond_span,
            )?);
            let try_node = build_destructure_try(builder, target, cond_span)?;
            stmts.push(try_node);
            let test =
                build_let_guard_test(builder, guard.as_ref().map(|expr| expr.as_ref()), cond_span)?;
            let body = lower_block_with_tail(builder, body, tail, span)?;
            let orelse = if let Some((elif_cond, elif_body)) = elifs.first() {
                lower_if_block_with_tail(
                    builder,
                    elif_cond,
                    elif_body,
                    &elifs[1..],
                    else_body,
                    tail,
                    span,
                )?
            } else if let Some(else_body) = else_body {
                lower_block_with_tail(builder, else_body, tail, span)?
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
                    cond_span,
                )
                .map_err(py_err_to_lower)?;
            stmts.push(if_node);
            Ok(stmts)
        }
    }
}

/// Lower an expression used as a statement. Returns multiple Python statements
/// because some expressions (like If, While, For, etc.) lower to Python statement nodes.
pub(crate) fn lower_expr_as_stmt(
    builder: &AstBuilder<'_>,
    expr: &Expr,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    match expr {
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => lower_if_chain(builder, cond, body, elifs, else_body, span),
        Expr::While {
            cond,
            body,
            else_body,
            span,
        } => lower_while_stmt(builder, cond, body, else_body, TailBehavior::None, span),
        Expr::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => lower_for_stmt(
            builder,
            target,
            iter,
            body,
            else_body,
            TailBehavior::None,
            span,
        ),
        Expr::Def {
            name: Some(name),
            params,
            body,
            decorators,
            span,
        } => lower_def_stmt(builder, name, params, body, decorators, span),
        Expr::Def { name: None, .. } => Err(LowerError::new("anonymous def was not desugared")),
        Expr::Class {
            name,
            bases,
            body,
            decorators,
            span,
        } => lower_class_stmt(builder, name, bases, body, decorators, span),
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => lower_try_stmt(
            builder,
            body,
            handlers,
            else_body,
            finally_body,
            TailBehavior::None,
            span,
        ),
        Expr::With {
            items, body, span, ..
        } => lower_with_stmt(builder, items, body, TailBehavior::None, span),
        Expr::Awk {
            sources,
            body,
            span,
            ..
        } => super::awk::lower_awk_stmt(builder, sources, body, span, TailBehavior::None),
        Expr::Xargs {
            sources,
            body,
            span,
            ..
        } => super::xargs::lower_xargs_stmt(builder, sources, body, span, TailBehavior::None),
        Expr::Block { stmts, span } => lower_block(builder, stmts, span),
        _ => {
            let value = lower_expr(builder, expr)?;
            Ok(vec![
                builder
                    .call_node("Expr", vec![value], span)
                    .map_err(py_err_to_lower)?,
            ])
        }
    }
}

/// Lower an expression at tail position with the given tail behavior.
/// Handles tail propagation into compound expression branches (if, lines, xargs)
/// and generic tail wrapping (auto-print, capture, implicit return) for other expressions.
pub(crate) fn lower_tail_expr(
    builder: &AstBuilder<'_>,
    expr: &Expr,
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    // Propagate tail behavior into block expressions
    if let Expr::Block {
        stmts,
        span: block_span,
    } = expr
    {
        return lower_block_with_tail(builder, stmts, tail, block_span);
    }
    // Compound expressions that propagate tail behavior via CaptureOnly + wrap
    match expr {
        Expr::If {
            cond,
            body,
            elifs,
            else_body,
            span,
        } => {
            let if_stmts = lower_if_block_with_tail(
                builder,
                cond,
                body,
                elifs,
                else_body,
                TailBehavior::CaptureOnly,
                span,
            )?;
            return wrap_compound_with_tail(builder, if_stmts, tail, span);
        }
        Expr::While {
            cond,
            body,
            else_body,
            span,
        } => {
            return lower_while_stmt(builder, cond, body, else_body, tail, span);
        }
        Expr::For {
            target,
            iter,
            body,
            else_body,
            span,
        } => {
            return lower_for_stmt(builder, target, iter, body, else_body, tail, span);
        }
        Expr::Try {
            body,
            handlers,
            else_body,
            finally_body,
            span,
        } => {
            return lower_try_stmt(builder, body, handlers, else_body, finally_body, tail, span);
        }
        Expr::With {
            items, body, span, ..
        } => {
            return lower_with_stmt(builder, items, body, tail, span);
        }
        Expr::Awk {
            sources,
            body,
            span,
            ..
        } => {
            return super::awk::lower_awk_stmt(builder, sources, body, span, tail);
        }
        Expr::Xargs {
            sources,
            body,
            span,
            ..
        } => {
            return super::xargs::lower_xargs_stmt(builder, sources, body, span, tail);
        }
        // Def, Class still don't propagate tail
        Expr::Def { .. } | Expr::Class { .. } => {
            return lower_expr_as_stmt(builder, expr, span);
        }
        _ => {}
    }
    let value = lower_expr(builder, expr)?;
    match tail {
        TailBehavior::AutoPrint => build_auto_print_block(builder, value, span),
        TailBehavior::CaptureOnly => Ok(vec![assign_name(
            builder,
            "__snail_last_result",
            value,
            span,
        )?]),
        TailBehavior::ImplicitReturn => {
            let return_stmt = builder
                .call_node("Return", vec![value], span)
                .map_err(py_err_to_lower)?;
            Ok(vec![return_stmt])
        }
        TailBehavior::None => Ok(vec![
            builder
                .call_node("Expr", vec![value], span)
                .map_err(py_err_to_lower)?,
        ]),
    }
}

fn build_auto_print_block(
    builder: &AstBuilder<'_>,
    expr: PyObject,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let assign = assign_name(builder, "__snail_last_result", expr, span)?;
    let print_call = emit_auto_print_of_last_result(builder, span)?;
    Ok(vec![assign, print_call])
}

/// Emit `__snail_auto_print(__snail_last_result)` as a standalone statement.
/// Used after compound statements whose body already captured to `__snail_last_result`.
fn emit_auto_print_of_last_result(
    builder: &AstBuilder<'_>,
    span: &SourceSpan,
) -> Result<PyObject, LowerError> {
    let last_result = name_expr(
        builder,
        "__snail_last_result",
        span,
        builder.load_ctx().map_err(py_err_to_lower)?,
    )?;
    let call = builder
        .call_node(
            "Call",
            vec![
                name_expr(
                    builder,
                    SNAIL_AUTO_PRINT_HELPER,
                    span,
                    builder.load_ctx().map_err(py_err_to_lower)?,
                )?,
                PyList::new_bound(builder.py(), vec![last_result]).into_py(builder.py()),
                PyList::empty_bound(builder.py()).into_py(builder.py()),
            ],
            span,
        )
        .map_err(py_err_to_lower)?;
    builder
        .call_node("Expr", vec![call], span)
        .map_err(py_err_to_lower)
}

/// Wrap a compound statement with tail behavior:
/// 1. Prepend `__snail_last_result = None`
/// 2. Append the compound statement(s)
/// 3. Based on tail: append auto-print / return / nothing
pub(crate) fn wrap_compound_with_tail(
    builder: &AstBuilder<'_>,
    compound_stmts: Vec<PyObject>,
    tail: TailBehavior,
    span: &SourceSpan,
) -> Result<Vec<PyObject>, LowerError> {
    let mut result = Vec::new();

    // __snail_last_result = None
    let none_val = builder
        .call_node(
            "Constant",
            vec![builder.py().None().into_py(builder.py())],
            span,
        )
        .map_err(py_err_to_lower)?;
    result.push(assign_name(builder, "__snail_last_result", none_val, span)?);

    // The compound statement(s) themselves
    result.extend(compound_stmts);

    // Tail action after the compound
    match tail {
        TailBehavior::AutoPrint => {
            result.push(emit_auto_print_of_last_result(builder, span)?);
        }
        TailBehavior::ImplicitReturn => {
            let last_result = name_expr(
                builder,
                "__snail_last_result",
                span,
                builder.load_ctx().map_err(py_err_to_lower)?,
            )?;
            let return_stmt = builder
                .call_node("Return", vec![last_result], span)
                .map_err(py_err_to_lower)?;
            result.push(return_stmt);
        }
        TailBehavior::CaptureOnly => {
            // Already captured inside the body — nothing extra needed
        }
        TailBehavior::None => {
            unreachable!("wrap_compound_with_tail should not be called with TailBehavior::None")
        }
    }

    Ok(result)
}
